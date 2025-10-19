use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use dialoguer::{Select, theme::ColorfulTheme};
use git2::Repository;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};
use std::{
    collections::HashMap,
    io::{Stdout, stdout},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

mod cache;
mod config;
mod output;
mod parallel;
mod stats;
mod telemetry;
#[derive(Parser)]
#[command(name = "crabgit")]
#[command(about = "CLI tool for inspecting Git repos. Blazingly fast 🦀")]
#[command(
    long_about = "GitCrab provides data-driven insights into Git repositories through statistical analysis and interactive exploration.

EXAMPLES:
    # Show repository status (default)
    crabgit
    
    # Comprehensive repository statistics
    crabgit stats
    
    # Analyze commit activity over last 90 days
    crabgit stats repo --since 90d --format table
    
    # Find top contributors by commits
    crabgit stats authors --top 10 --metric commits
    
    # Identify file hotspots with high churn
    crabgit stats hotspots --since 180d --top 25
    
    # Analyze code stability and change patterns
    crabgit stats stability --author \"John Doe\" --top 15
    
    # Interactive Terminal UI for exploration
    crabgit tui
    
    # Analyze a different repository
    crabgit --repo /path/to/repo stats authors"
)]
#[command(version)]
struct Cli {
    /// Path to the Git repository (defaults to current directory)
    #[arg(short, long, value_name = "PATH")]
    repo: Option<PathBuf>,

    /// Enable debug mode with timing logs
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Disable caching
    #[arg(long, default_value_t = false)]
    no_cache: bool,

    /// Disable parallel processing
    #[arg(long, default_value_t = false)]
    no_parallel: bool,

    /// Maximum number of threads for parallel processing
    #[arg(long)]
    max_threads: Option<usize>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show repository status and information
    Status,

    /// List all branches
    Branches,

    /// List recent commits
    Log {
        /// Number of commits to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },

    /// Show repository statistics for data-driven insights
    #[command(
        long_about = "Analyze repository data through various statistical lenses.

EXAMPLES:
    # Basic repository statistics (default)
    crabgit stats
    
    # Repository activity over time
    crabgit stats repo --since 90d --bucket week
    
    # Top authors by commits or churn
    crabgit stats authors --top 15 --metric commits
    crabgit stats authors --metric churn --format json
    
    # Activity heatmap by weekday/hour
    crabgit stats calendar --since 180d
    
    # File hotspots with recency decay
    crabgit stats hotspots --half-life-days 60 --include 'src/**'
    
    # Branch analysis and comparison
    crabgit stats branches --base origin/main
    
    # File coupling and co-change analysis
    crabgit stats coupling --min-support 0.05 --top 20
    
    # Code ownership patterns
    crabgit stats ownership --expensive --top 30
    
    # Change stability analysis
    crabgit stats stability --directory src/ --no-merges
    
    # Release and tag metrics
    crabgit stats releases --limit 10"
    )]
    Stats {
        #[command(subcommand)]
        command: Option<StatsCommand>,
    },

    /// Launch TUI (Terminal User Interface) mode
    #[command(
        long_about = "Launch a full-screen terminal interface for interactive repository exploration.

Features tabbed navigation through:
- Repository status and basic information
- Branch listing with current branch indicator  
- Recent commit history with short IDs
- Statistical summaries and metrics

Navigation:
- Left/Right arrow keys to switch tabs
- 'q' to quit

EXAMPLE:
    crabgit tui"
    )]
    Tui,

    /// Interactive mode for exploring the repository
    #[command(
        long_about = "Menu-driven interface for step-by-step repository analysis.

Provides a guided experience through:
- Repository status inspection
- Branch management
- Commit history browsing
- Statistical analysis
- TUI mode launching

Ideal for:
- First-time users learning GitCrab features
- Exploratory analysis workflows
- When you're unsure which analysis to run

EXAMPLE:
    crabgit interactive"
    )]
    Interactive,
}

#[derive(Subcommand)]
enum StatsCommand {
    /// Repository-level statistics
    Repo(RepoArgs),
    /// Per-author statistics (top N)
    Authors(AuthorsArgs),
    /// Calendar/weekday-hour activity heatmap
    Calendar(CalendarArgs),
    /// File hotspots (churn with recency decay)
    Hotspots(HotspotsArgs),
    /// Branch metrics (ahead/behind, activity, merge ratio)
    Branches(BranchesArgs),
    /// File coupling (co-change analysis)
    Coupling(CouplingArgs),
    /// Code ownership and bus factor analysis
    Ownership(OwnershipArgs),
    /// Change stability analysis for files and authors
    Stability(StabilityArgs),
    /// Tags/releases metrics and analysis
    Releases(ReleasesArgs),
}

#[derive(Args, Debug, Clone)]
struct RepoArgs {
    /// Start of time window (e.g., "90d", "2024-01-01")
    #[arg(long)]
    since: Option<String>,

    /// End of time window (e.g., "2024-12-31")
    #[arg(long)]
    until: Option<String>,

    /// Aggregation bucket
    #[arg(long, value_enum, default_value = "week")]
    bucket: stats::Bucket,

    /// Metric to compute
    #[arg(long, value_enum, default_value = "activity")]
    metric: stats::StatsMetric,

    /// Exclude merge commits from metrics that traverse history
    #[arg(long, default_value_t = false)]
    no_merges: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    format: stats::OutputFormat,
}

#[derive(Args, Debug, Clone)]
struct AuthorsArgs {
    /// Start of time window
    #[arg(long)]
    since: Option<String>,
    /// End of time window
    #[arg(long)]
    until: Option<String>,
    /// Exclude merge commits
    #[arg(long, default_value_t = false)]
    no_merges: bool,
    /// Top N authors to show
    #[arg(long, default_value_t = 15)]
    top: usize,
    /// Metric to rank by
    #[arg(long, value_enum, default_value = "commits")]
    metric: stats::AuthorMetric,
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    format: stats::OutputFormat,
}

#[derive(Args, Debug, Clone)]
struct CalendarArgs {
    /// Start of time window
    #[arg(long)]
    since: Option<String>,
    /// End of time window
    #[arg(long)]
    until: Option<String>,
    /// Output format
    #[arg(long, value_enum, default_value = "chart")]
    format: stats::OutputFormat,
}

#[derive(Args, Debug, Clone)]
struct HotspotsArgs {
    /// Start of time window
    #[arg(long)]
    since: Option<String>,
    /// End of time window
    #[arg(long)]
    until: Option<String>,
    /// Exclude merge commits
    #[arg(long, default_value_t = true)]
    no_merges: bool,
    /// Include glob (e.g., 'src/**')
    #[arg(long)]
    include: Option<String>,
    /// Exclude glob (e.g., 'tests/**')
    #[arg(long)]
    exclude: Option<String>,
    /// Half-life for recency decay in days
    #[arg(long, default_value_t = 90.0)]
    half_life_days: f64,
    /// Top N results
    #[arg(long, default_value_t = 25)]
    top: usize,
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    format: stats::OutputFormat,
}

#[derive(Args, Debug, Clone)]
struct BranchesArgs {
    /// Start of time window
    #[arg(long)]
    since: Option<String>,
    /// End of time window
    #[arg(long)]
    until: Option<String>,
    /// Base branch for comparison (defaults to origin/main or origin/master)
    #[arg(long)]
    base: Option<String>,
    /// Exclude merge commits
    #[arg(long, default_value_t = false)]
    no_merges: bool,
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    format: stats::OutputFormat,
}

#[derive(Args, Debug, Clone)]
struct CouplingArgs {
    /// Start of time window
    #[arg(long)]
    since: Option<String>,
    /// End of time window
    #[arg(long)]
    until: Option<String>,
    /// Exclude merge commits
    #[arg(long, default_value_t = true)]
    no_merges: bool,
    /// Top N file pairs to show
    #[arg(long, default_value_t = 25)]
    top: usize,
    /// Minimum support threshold (0.0-1.0)
    #[arg(long, default_value_t = 0.01)]
    min_support: f64,
    /// Window size for memory-safe processing
    #[arg(long, default_value_t = 1000)]
    window_size: usize,
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    format: stats::OutputFormat,
}

#[derive(Args, Debug, Clone)]
struct OwnershipArgs {
    /// Start of time window (only applies to fast mode)
    #[arg(long)]
    since: Option<String>,
    /// End of time window (only applies to fast mode)
    #[arg(long)]
    until: Option<String>,
    /// Top N files to analyze
    #[arg(long, default_value_t = 25)]
    top: usize,
    /// Include file pattern (e.g., '*.rs')
    #[arg(long)]
    include: Option<String>,
    /// Exclude file pattern (e.g., 'target/*')
    #[arg(long)]
    exclude: Option<String>,
    /// Use expensive blame analysis (opt-in) instead of fast last-modified approximation
    #[arg(long, default_value_t = false)]
    expensive: bool,
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    format: stats::OutputFormat,
}

#[derive(Args, Debug, Clone)]
struct StabilityArgs {
    /// Start of time window
    #[arg(long)]
    since: Option<String>,
    /// End of time window
    #[arg(long)]
    until: Option<String>,
    /// Filter by author (partial name match)
    #[arg(long)]
    author: Option<String>,
    /// Filter by directory path
    #[arg(long)]
    directory: Option<String>,
    /// Exclude merge commits
    #[arg(long, default_value_t = true)]
    no_merges: bool,
    /// Top N files to show
    #[arg(long, default_value_t = 25)]
    top: usize,
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    format: stats::OutputFormat,
}

#[derive(Args, Debug, Clone)]
struct ReleasesArgs {
    /// Number of releases to analyze (defaults to all)
    #[arg(long, short = 'n')]
    limit: Option<usize>,
    /// Output format
    #[arg(long, value_enum, default_value = "table")]
    format: stats::OutputFormat,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration with CLI overrides
    let config_overrides = config::ConfigOverrides {
        debug: Some(cli.debug),
        cache_enabled: Some(!cli.no_cache),
        parallel_enabled: Some(!cli.no_parallel),
        max_threads: cli.max_threads,
    };
    let global_config = config::GlobalConfig::load().with_overrides(config_overrides);

    // Initialize performance metrics
    let perf_metrics = telemetry::PerfMetrics::new(global_config.debug.timing);
    let debug_logger = telemetry::DebugLogger::new(global_config.debug.enabled);

    debug_logger.info("GitCrab starting with configuration loaded");

    let repo_path = cli.repo.unwrap_or_else(|| PathBuf::from("."));
    let repo = Repository::open(&repo_path)
        .context(format!("Failed to open repository at {:?}", repo_path))?;

    match cli.command {
        Some(Commands::Status) => show_status(&repo)?,
        Some(Commands::Branches) => list_branches(&repo)?,
        Some(Commands::Log { count }) => show_log(&repo, count)?,
        Some(Commands::Stats { command }) => match command {
            Some(StatsCommand::Repo(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else {
                    String::from(".")
                };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: args.bucket,
                    no_merges: args.no_merges,
                };
                output::render_repo(&repo, args.metric, &ctx, args.format)?
            }
            Some(StatsCommand::Authors(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else {
                    String::from(".")
                };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: stats::Bucket::Week, // unused for authors
                    no_merges: args.no_merges,
                };
                output::render_authors(&repo, &ctx, args.metric, args.top, args.format)?
            }
            Some(StatsCommand::Calendar(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else {
                    String::from(".")
                };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: stats::Bucket::Week, // unused for calendar
                    no_merges: false,
                };
                output::render_calendar(&repo, &ctx, args.format)?
            }
            Some(StatsCommand::Hotspots(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else {
                    String::from(".")
                };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: stats::Bucket::Week,
                    no_merges: args.no_merges,
                };
                output::render_hotspots(
                    &repo,
                    &ctx,
                    args.include.as_deref(),
                    args.exclude.as_deref(),
                    args.half_life_days,
                    args.top,
                    args.format,
                )?
            }
            Some(StatsCommand::Branches(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else {
                    String::from(".")
                };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: stats::Bucket::Week,
                    no_merges: args.no_merges,
                };
                output::render_branches(&repo, &ctx, args.base.as_deref(), args.format)?
            }
            Some(StatsCommand::Coupling(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else {
                    String::from(".")
                };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: stats::Bucket::Week,
                    no_merges: args.no_merges,
                };
                output::render_coupling(
                    &repo,
                    &ctx,
                    args.top,
                    args.min_support,
                    args.window_size,
                    args.format,
                )?
            }
            Some(StatsCommand::Ownership(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else {
                    String::from(".")
                };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: stats::Bucket::Week,
                    no_merges: false, // Not relevant for ownership analysis
                };
                output::render_ownership(
                    &repo,
                    &ctx,
                    args.top,
                    args.include.as_deref(),
                    args.exclude.as_deref(),
                    args.expensive,
                    args.format,
                )?
            }
            Some(StatsCommand::Stability(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else {
                    String::from(".")
                };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: stats::Bucket::Week, // unused for stability
                    no_merges: args.no_merges,
                };
                output::render_stability(
                    &repo,
                    &ctx,
                    args.author.as_deref(),
                    args.directory.as_deref(),
                    args.top,
                    args.format,
                )?
            }
            Some(StatsCommand::Releases(args)) => {
                output::render_releases(&repo, args.limit, args.format)?
            }
            None => show_stats(&repo)?,
        },
        Some(Commands::Tui) => run_tui(&repo)?,
        Some(Commands::Interactive) => interactive_mode(&repo)?,
        None => {
            // Default behavior: show status
            show_status(&repo)?
        }
    }

    // Print performance summary if debug is enabled
    perf_metrics.print_summary();

    Ok(())
}

fn show_status(repo: &Repository) -> Result<()> {
    println!("🦀 Repository Information");
    println!("=========================\n");

    // Show repository path
    if let Some(path) = repo.path().parent() {
        println!("Path: {}", path.display());
    }

    // Show current branch
    let head = repo.head()?;
    if let Some(name) = head.shorthand() {
        println!("Current branch: {}", name);
    }

    // Show HEAD commit
    let head_commit = head.peel_to_commit()?;
    println!("HEAD commit: {}", head_commit.id());

    if let Some(message) = head_commit.message() {
        println!("Message: {}", message.lines().next().unwrap_or(""));
    }

    // Check if the repository is bare
    println!("Is bare: {}", repo.is_bare());

    Ok(())
}

fn list_branches(repo: &Repository) -> Result<()> {
    println!("🦀 Branches");
    println!("===========\n");

    let branches = repo.branches(None)?;

    for branch in branches {
        let (branch, branch_type) = branch?;
        let name = branch.name()?.unwrap_or("(unknown)");
        let type_str = match branch_type {
            git2::BranchType::Local => "local",
            git2::BranchType::Remote => "remote",
        };

        let is_head = branch.is_head();
        let marker = if is_head { "* " } else { "  " };

        println!("{}{} ({})", marker, name, type_str);
    }

    Ok(())
}

fn show_log(repo: &Repository, count: usize) -> Result<()> {
    println!("🦀 Recent Commits (showing last {})", count);
    println!("====================================\n");

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    for (i, oid) in revwalk.enumerate() {
        if i >= count {
            break;
        }

        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        println!("Commit: {}", commit.id());
        println!("Author: {}", commit.author());

        if let Some(message) = commit.message() {
            println!("Message: {}\n", message.trim());
        }
    }

    Ok(())
}

fn show_stats(repo: &Repository) -> Result<()> {
    println!("🦀 Repository Statistics");
    println!("========================\n");

    // Count total commits
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    let total_commits = revwalk.count();
    println!("Total commits: {}", total_commits);

    // Count branches
    let branches = repo.branches(None)?;
    let mut local_branches = 0;
    let mut remote_branches = 0;

    for branch in branches {
        let (_, branch_type) = branch?;
        match branch_type {
            git2::BranchType::Local => local_branches += 1,
            git2::BranchType::Remote => remote_branches += 1,
        }
    }

    println!("Local branches: {}", local_branches);
    println!("Remote branches: {}", remote_branches);

    // Analyze commit authors
    let mut author_stats: HashMap<String, usize> = HashMap::new();
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let author_name = commit.author().name().unwrap_or("Unknown").to_string();
        *author_stats.entry(author_name).or_insert(0) += 1;
    }

    println!("\nTop contributors:");
    let mut sorted_authors: Vec<_> = author_stats.iter().collect();
    sorted_authors.sort_by(|a, b| b.1.cmp(a.1));

    for (i, (author, count)) in sorted_authors.iter().take(5).enumerate() {
        println!("{}. {} ({} commits)", i + 1, author, count);
    }

    // Calculate repository age
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    let _ = revwalk.set_sorting(git2::Sort::TIME | git2::Sort::REVERSE);

    if let Some(first_oid) = revwalk.next() {
        let first_commit = repo.find_commit(first_oid?)?;
        let first_time = first_commit.time();
        let age_seconds =
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64 - first_time.seconds();
        let age_days = age_seconds / (24 * 60 * 60);
        println!("\nRepository age: {} days", age_days);
    }

    Ok(())
}

fn run_tui(repo: &Repository) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_tui_app(&mut terminal, repo);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_tui_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, repo: &Repository) -> Result<()> {
    let mut selected_tab = 0;
    let tabs = ["Status", "Branches", "Stats", "Log"];

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(f.area());

            let tab_titles: Vec<Line> = tabs
                .iter()
                .map(|t| {
                    let content = format!(" {} ", t);
                    Line::from(content)
                })
                .collect();

            let tabs_widget = Tabs::new(tab_titles)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("🦀 GitCrab TUI"),
                )
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .select(selected_tab);

            f.render_widget(tabs_widget, chunks[0]);

            match selected_tab {
                0 => render_status_tab(f, chunks[1], repo),
                1 => render_branches_tab(f, chunks[1], repo),
                2 => render_stats_tab(f, chunks[1], repo),
                3 => render_log_tab(f, chunks[1], repo),
                _ => {}
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Right => {
                    selected_tab = (selected_tab + 1) % tabs.len();
                }
                KeyCode::Left => {
                    selected_tab = if selected_tab > 0 {
                        selected_tab - 1
                    } else {
                        tabs.len() - 1
                    };
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn render_status_tab(f: &mut Frame, area: Rect, repo: &Repository) {
    let mut content = vec![];

    if let Some(path) = repo.path().parent() {
        content.push(format!("Path: {}", path.display()));
    }

    if let Ok(head) = repo.head() {
        if let Some(name) = head.shorthand() {
            content.push(format!("Current branch: {}", name));
        }

        if let Ok(commit) = head.peel_to_commit() {
            content.push(format!("HEAD commit: {}", commit.id()));
            if let Some(message) = commit.message() {
                content.push(format!("Message: {}", message.lines().next().unwrap_or("")));
            }
        }
    }

    content.push(format!("Is bare: {}", repo.is_bare()));

    let text = content.join("\n");
    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Repository Status"),
    );
    f.render_widget(paragraph, area);
}

fn render_branches_tab(f: &mut Frame, area: Rect, repo: &Repository) {
    let mut items = vec![];

    if let Ok(branches) = repo.branches(None) {
        for (branch, branch_type) in branches.flatten() {
            if let Ok(Some(name)) = branch.name() {
                let type_str = match branch_type {
                    git2::BranchType::Local => "local",
                    git2::BranchType::Remote => "remote",
                };
                let marker = if branch.is_head() { "*" } else { " " };
                let item_text = format!("{} {} ({})", marker, name, type_str);
                items.push(ListItem::new(item_text));
            }
        }
    }

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Branches"));
    f.render_widget(list, area);
}

fn render_stats_tab(f: &mut Frame, area: Rect, repo: &Repository) {
    let mut content = vec![];

    // Count commits
    if let Ok(mut revwalk) = repo.revwalk()
        && revwalk.push_head().is_ok()
    {
        let total_commits = revwalk.count();
        content.push(format!("Total commits: {}", total_commits));
    }

    // Count branches
    if let Ok(branches) = repo.branches(None) {
        let mut local = 0;
        let mut remote = 0;
        for (_, branch_type) in branches.flatten() {
            match branch_type {
                git2::BranchType::Local => local += 1,
                git2::BranchType::Remote => remote += 1,
            }
        }
        content.push(format!("Local branches: {}", local));
        content.push(format!("Remote branches: {}", remote));
    }

    let text = content.join("\n");
    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Repository Statistics"),
    );
    f.render_widget(paragraph, area);
}

fn render_log_tab(f: &mut Frame, area: Rect, repo: &Repository) {
    let mut items = vec![];

    if let Ok(mut revwalk) = repo.revwalk()
        && revwalk.push_head().is_ok()
        && revwalk.set_sorting(git2::Sort::TIME).is_ok()
    {
        for (i, oid) in revwalk.enumerate() {
            if i >= 10 {
                break;
            }
            if let Ok(oid) = oid
                && let Ok(commit) = repo.find_commit(oid)
            {
                let short_id = format!("{:.7}", commit.id());
                let message = commit
                    .message()
                    .and_then(|m| m.lines().next())
                    .unwrap_or("No message")
                    .to_string();
                let item_text = format!("{} {}", short_id, message);
                items.push(ListItem::new(item_text));
            }
        }
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Recent Commits"),
    );
    f.render_widget(list, area);
}

fn interactive_mode(repo: &Repository) -> Result<()> {
    println!("🦀 Interactive Mode");
    println!("===================\n");

    loop {
        let options = vec![
            "Show repository status",
            "List branches",
            "View recent commits",
            "Show repository statistics",
            "Launch TUI mode",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;

        println!(); // Add spacing

        match selection {
            0 => show_status(repo)?,
            1 => list_branches(repo)?,
            2 => {
                // Ask for number of commits
                let count = 10; // Default, could be made interactive too
                show_log(repo, count)?;
            }
            3 => show_stats(repo)?,
            4 => run_tui(repo)?,
            5 => {
                println!("Goodbye! 🦀");
                break;
            }
            _ => unreachable!(),
        }

        println!(); // Add spacing between operations
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_open_repository() {
        // Test opening the current repository
        let repo_path = env::current_dir().expect("Failed to get current directory");
        let result = Repository::open(&repo_path);
        assert!(result.is_ok(), "Should be able to open current repository");
    }

    #[test]
    fn test_show_status() {
        // Test that show_status doesn't panic on current repository
        let repo_path = env::current_dir().expect("Failed to get current directory");
        let repo = Repository::open(&repo_path).expect("Failed to open repository");
        let result = show_status(&repo);
        assert!(result.is_ok(), "show_status should succeed");
    }

    #[test]
    fn test_list_branches() {
        // Test that list_branches doesn't panic on current repository
        let repo_path = env::current_dir().expect("Failed to get current directory");
        let repo = Repository::open(&repo_path).expect("Failed to open repository");
        let result = list_branches(&repo);
        assert!(result.is_ok(), "list_branches should succeed");
    }

    #[test]
    fn test_show_log() {
        // Test that show_log doesn't panic on current repository
        let repo_path = env::current_dir().expect("Failed to get current directory");
        let repo = Repository::open(&repo_path).expect("Failed to open repository");
        let result = show_log(&repo, 5);
        assert!(result.is_ok(), "show_log should succeed");
    }

    #[test]
    fn test_stability_analysis() {
        // Test that stability analysis doesn't panic on current repository
        let repo_path = env::current_dir().expect("Failed to get current directory");
        let repo = Repository::open(&repo_path).expect("Failed to open repository");

        let ctx = stats::StatsContext {
            repo_path: repo_path.display().to_string(),
            since: Some("30d".to_string()),
            until: None,
            bucket: stats::Bucket::Week,
            no_merges: true,
        };

        let result = stats::stability::analyze_stability(&repo, &ctx, None, None, 10);
        assert!(result.is_ok(), "stability analysis should succeed");

        let stats = result.unwrap();
        assert!(
            stats.total_files_analyzed <= 100,
            "should analyze reasonable number of files"
        );
        assert_eq!(
            stats.date_range_days, 365,
            "default date range should be 365 days"
        );
    }
}
