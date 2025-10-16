use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dialoguer::{Select, theme::ColorfulTheme};
use git2::Repository;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gitcrab")]
#[command(about = "CLI tool for inspecting Git repos. Blazingly fast 🦀", long_about = None)]
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

    /// Interactive mode for exploring the repository
    Interactive,
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

fn interactive_mode(repo: &Repository) -> Result<()> {
    println!("🦀 Interactive Mode");
    println!("===================\n");

    loop {
        let options = vec![
            "Show repository status",
            "List branches",
            "View recent commits",
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
            3 => {
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
