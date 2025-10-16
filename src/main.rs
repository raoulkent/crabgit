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

mod stats;
mod output;
#[derive(Parser)]
#[command(name = "gitcrab")]
#[command(about = "CLI tool for inspecting Git repos. Blazingly fast 🦀", long_about = None)]
#[command(version)]
struct Cli {
    /// Path to the Git repository (defaults to current directory)
    #[arg(short, long, value_name = "PATH")]
    repo: Option<PathBuf>,

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
    Stats {
        #[command(subcommand)]
        command: Option<StatsCommand>,
    },

    /// Launch TUI (Terminal User Interface) mode
    Tui,

    /// Interactive mode for exploring the repository
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

fn main() -> Result<()> {
    let cli = Cli::parse();

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
                output::render_repo(&repo, args.metric, &ctx, args.format)?;
            }
            Some(StatsCommand::Authors(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else { String::from(".") };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: stats::Bucket::Week, // unused for authors
                    no_merges: args.no_merges,
                };
                output::render_authors(&repo, &ctx, args.metric, args.top, args.format)?;
            }
            Some(StatsCommand::Calendar(args)) => {
                let repo_display = if let Some(p) = repo.workdir() {
                    p.display().to_string()
                } else if let Some(parent) = repo.path().parent() {
                    parent.display().to_string()
                } else { String::from(".") };
                let ctx = stats::StatsContext {
                    repo_path: repo_display,
                    since: args.since.clone(),
                    until: args.until.clone(),
                    bucket: stats::Bucket::Week, // unused for calendar
                    no_merges: false,
                };
                output::render_calendar(&repo, &ctx, args.format)?;
            }
            None => show_stats(&repo)?,
        },
        Some(Commands::Tui) => run_tui(&repo)?,
        Some(Commands::Interactive) => interactive_mode(&repo)?,
        None => {
            // Default behavior: show status
            show_status(&repo)?;
        }
    }

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
}
