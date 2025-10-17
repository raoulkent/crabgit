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

#### Repository Activity Analysis

Track development patterns and commit frequency over time:

```bash
# Repository activity over last 90 days (weekly buckets)
gitcrab stats repo --since 90d --bucket week

# Daily commit activity for current month
gitcrab stats repo --since 30d --bucket day --format table

# Monthly activity trend for the past year
gitcrab stats repo --since 365d --bucket month --format chart

# Code churn (lines added/deleted) analysis
gitcrab stats repo --metric churn --since 180d
```

#### Author and Contributor Analysis

Understand team contributions and collaboration patterns:

```bash
# Top 15 contributors by commit count
gitcrab stats authors --top 15 --metric commits

# Top contributors by code churn (lines changed)
gitcrab stats authors --metric churn --format table

# Author activity excluding merge commits
gitcrab stats authors --no-merges --since 90d

# JSON output for data processing
gitcrab stats authors --format json --top 20
```

#### Activity Heatmaps

Visualize when development activity happens:

```bash
# Weekday/hour activity heatmap
gitcrab stats calendar --since 365d

# Activity patterns for recent months
gitcrab stats calendar --since 90d --format chart

# JSON format for external visualization
gitcrab stats calendar --format json > heatmap.json
```

#### File Hotspot Analysis

Identify files with high change frequency using recency decay:

```bash
# Top 25 hotspots with 90-day decay
gitcrab stats hotspots --since 180d --top 25

# Focus on source code files only
gitcrab stats hotspots --include 'src/**' --half-life-days 60

# Exclude test files from analysis
gitcrab stats hotspots --exclude 'tests/**' --format table

# Chart visualization of hotspots
gitcrab stats hotspots --format chart --top 15
```

#### Branch Analysis

Compare branch activity and divergence:

```bash
# Branch analysis vs main branch
gitcrab stats branches --base origin/main

# Activity across all branches (last 30 days)
gitcrab stats branches --since 30d --no-merges

# JSON output for CI/CD integration
gitcrab stats branches --format json
```

#### Code Coupling Analysis

Find files that change together frequently:

```bash
# Top 20 coupled file pairs
gitcrab stats coupling --top 20 --min-support 0.05

# Coupling analysis for recent changes
gitcrab stats coupling --since 90d --window-size 1000

# High-confidence coupling relationships
gitcrab stats coupling --min-support 0.1 --format json
```

#### Code Ownership Analysis

Understand who owns what parts of the codebase:

```bash
# Fast ownership approximation (last-modified)
gitcrab stats ownership --top 30

# Expensive but accurate blame-based analysis
gitcrab stats ownership --expensive --top 20

# Focus on specific file patterns
gitcrab stats ownership --include '*.rs' --exclude 'target/*'
```

#### Change Stability Analysis

Analyze the stability of file changes to identify areas that may need attention:

```bash
# Analyze stability of all files (shows least stable first)
gitcrab stats stability --top 25

# Filter by author to assess consultant/new developer impact
gitcrab stats stability --author "John Doe" --since 90d

# Focus on specific directory
gitcrab stats stability --directory src/ --no-merges

# Get JSON output for further processing
gitcrab stats stability --format json --top 15
```

**Stability Metrics Explained:**
- **Stability Score**: Lower values indicate more stable files (fewer changes, reverts, fixes)
- **Changes**: Total number of modifications to the file
- **Authors**: Number of different contributors who modified the file
- **Reverts**: Number of commits that reverted previous changes
- **Fixes**: Number of commits identified as bug fixes
- **Avg Days**: Average time between modifications
- **Primary Author**: Developer who made the most changes to the file

#### Release Analysis

Analyze release patterns and tag-based metrics:

```bash
# Analyze last 10 releases
gitcrab stats releases --limit 10

# All releases with detailed metrics
gitcrab stats releases --format json
```

### Output Formats

GitCrab supports multiple output formats for different use cases:

| Format | Use Case | Example |
|--------|----------|---------|
| `table` | Human-readable terminal output | `--format table` |
| `json` | Programmatic processing, APIs | `--format json` |
| `chart` | ASCII visualizations, dashboards | `--format chart` |

### Integration Examples

#### CI/CD Pipeline Integration

```bash
#!/bin/bash
# Generate release metrics for CI
gitcrab stats releases --format json > metrics/releases.json
gitcrab stats hotspots --format json > metrics/hotspots.json
gitcrab stats authors --format json > metrics/contributors.json
```

#### Data Analysis Workflows

```bash
# Extract commit counts for analysis
gitcrab stats repo --format json | jq '.series[].commits'

# Get top author commit counts
gitcrab stats authors --format json | jq '.authors[] | {name: .author, commits: .commits}'

# Find files with high churn
gitcrab stats hotspots --format json | jq '.hotspots[0:5] | .[] | .path'
```

### Performance Optimization

For large repositories, GitCrab provides several optimization options:

```bash
# Use caching for repeated analyses
gitcrab --no-cache stats repo  # Disable cache

# Control parallel processing
gitcrab --max-threads 4 stats coupling

# Debug performance timing
gitcrab --debug stats hotspots --since 180d
```

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
