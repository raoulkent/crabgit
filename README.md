# gitcrab

A CLI tool for data-driven Git repository analysis. Make informed decisions about your codebase through statistical insights and interactive exploration. Blazingly fast 🦀

## Features

- 📊 **Repository Statistics**: Analyze commit patterns, contributor activity, and repository health metrics
- 📈 **Contributor Insights**: Track changes by author, identify top contributors, and analyze collaboration patterns
- 📁 **File & Directory Analysis**: Inspect change frequency and stability across different parts of your codebase
- 🕰️ **Time-based Analytics**: Examine development trends over different time periods
- 🖥️ **Interactive TUI**: Explore repository data through an intuitive terminal interface with tabbed navigation
- 🎯 **Interactive Mode**: Navigate through repository statistics with menu-driven exploration
- 🚀 **Fast & Efficient**: Built with Rust for blazing performance on large repositories

## Installation

### From Source

```bash
cargo build --release
```

The binary will be available at `target/release/gitcrab`.

## Usage

### Core Analytics Commands

```bash
# Display comprehensive repository statistics
gitcrab stats

# Launch interactive TUI for detailed exploration
gitcrab tui

# Interactive menu-driven mode
gitcrab interactive
```

### Basic Repository Information

```bash
# Show repository status (default behavior)
gitcrab

# List all branches
gitcrab branches

# Show recent commits
gitcrab log --count 20
```

### Advanced Analytics

#### Change Stability Analysis

Analyze the stability of file changes to identify areas that may need attention:

```bash
# Analyze stability of all files (shows least stable first)
gitcrab stats stability

# Filter by author to assess consultant/new developer impact
gitcrab stats stability --author "John Doe"

# Focus on specific directory
gitcrab stats stability --directory src/

# Analyze recent changes only
gitcrab stats stability --since 30d

# Show top 10 most unstable files
gitcrab stats stability --top 10

# Get JSON output for further processing
gitcrab stats stability --format json

# Visual chart of stability scores
gitcrab stats stability --format chart
```

**Stability Metrics Explained:**
- **Stability Score**: Lower values indicate more stable files (fewer changes, reverts, fixes)
- **Changes**: Total number of modifications to the file
- **Authors**: Number of different contributors who modified the file
- **Reverts**: Number of commits that reverted previous changes
- **Fixes**: Number of commits identified as bug fixes
- **Avg Days**: Average time between modifications
- **Primary Author**: Developer who made the most changes to the file

Use this analysis to:
- Evaluate code quality from consultants or new team members
- Identify files that may need refactoring or better testing
- Spot patterns in problematic areas of the codebase
- Make data-driven decisions about code review focus

### Analyzing Different Repositories

```bash
# Analyze a specific repository
gitcrab --repo /path/to/repo stats

# Launch TUI for a different repository
gitcrab --repo /path/to/repo tui
```

## What GitCrab Reveals

GitCrab helps you understand your repository through data-driven insights:

- **Repository Health**: Total commits, branch count, repository age, and overall activity metrics
- **Contributor Analysis**: Top contributors by commit count, collaboration patterns, and team dynamics
- **Development Trends**: Commit frequency over time, peak development periods, and project evolution
- **Code Stability**: Areas of high change frequency vs. stable components
- **Team Insights**: Individual contributor patterns and code ownership analysis

Use these insights to make informed decisions about code reviews, refactoring priorities, team responsibilities, and technical debt management.

### Help

```bash
gitcrab --help
```

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Lint

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Format

```bash
cargo fmt
```

## Dependencies

- `clap` - Command line argument parsing
- `git2` - Git repository operations for data extraction
- `anyhow` - Error handling
- `dialoguer` - Interactive prompts and menus
- `ratatui` - Terminal user interface framework
- `crossterm` - Cross-platform terminal manipulation

## License

See LICENSE file for details.
