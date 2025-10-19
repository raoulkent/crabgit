# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Scope

GitCrab is a CLI tool designed to help developers and teams make data-driven decisions about Git repositories through statistical analysis and intuitive interfaces. The tool focuses on providing actionable insights about:

- Repository health metrics (commits, branches, age, activity)
- Contributor analysis and collaboration patterns
- Development trends and commit frequency over time
- Code stability and change frequency analysis
- Team insights and code ownership patterns

The goal is to enable informed decisions about code reviews, refactoring priorities, team responsibilities, and technical debt management through data-driven repository analysis.

## Commands

### Build and Development
```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Lint code
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt

# Run the tool (after building)
./target/release/crabgit

# Run TUI mode
./target/release/crabgit tui

# Show repository statistics
./target/release/crabgit stats
```

### Testing Individual Components
```bash
# Run specific test
cargo test test_show_status

# Run tests with output
cargo test -- --nocapture
```

## Architecture

This is a single-file Rust CLI application (`src/main.rs`) that provides Git repository inspection tools. The application is structured around these core components:

### Command Structure
- Uses `clap` with derive macros for CLI parsing
- Main `Cli` struct handles global options (like `--repo`)
- `Commands` enum defines subcommands: Status, Branches, Log, Stats, TUI, Interactive
- Default behavior shows repository status when no subcommand is provided

### Core Functions
- `show_status()`: Displays repository path, current branch, HEAD commit info
- `list_branches()`: Lists all local and remote branches with current branch indicator
- `show_log()`: Shows recent commits with configurable count
- `show_stats()`: Calculates and displays repository statistics (commits, contributors, age)
- `run_tui()`: Launches full-screen TUI with tabbed interface
- `interactive_mode()`: Provides menu-driven interface using `dialoguer`

### Dependencies & Their Roles
- `git2`: Core Git operations (repository access, branch listing, commit walking)
- `clap`: Command-line argument parsing with derive macros
- `anyhow`: Error handling with context
- `dialoguer`: Interactive prompts and menus
- `ratatui`: Terminal UI framework for full-screen interfaces
- `crossterm`: Cross-platform terminal manipulation

### Key Design Patterns
- Single repository instance passed to all functions
- Consistent error handling with `anyhow::Result`
- Uses Git2 library's safe Rust bindings rather than shell commands
- Interactive mode loops until user chooses to exit

## Development Notes

- All Git operations are performed through the `git2` crate, not shell commands
- Repository path defaults to current directory if not specified
- Tests assume they run within a valid Git repository
- Uses Rust 2024 edition