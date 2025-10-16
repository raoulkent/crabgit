# gitcrab

Cli tool for inspecting Git repos. Blazingly fast 🦀

## Features

- 📊 **Repository Status**: View current branch, HEAD commit, and repository info
- 🌿 **Branch Listing**: List all local and remote branches with current branch indicator
- 📜 **Commit Log**: View recent commits with customizable count
- 🎯 **Interactive Mode**: Explore repositories with an interactive menu
- 🚀 **Fast & Efficient**: Built with Rust for blazing performance

## Installation

### From Source

```bash
cargo build --release
```

The binary will be available at `target/release/gitcrab`.

## Usage

```bash
# Show repository status (default behavior)
gitcrab

# Show repository status explicitly
gitcrab status

# List all branches
gitcrab branches

# Show last 10 commits (default)
gitcrab log

# Show last N commits
gitcrab log --count 5

# Interactive mode
gitcrab interactive

# Inspect a different repository
gitcrab --repo /path/to/repo status
```

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
- `git2` - Git repository operations
- `anyhow` - Error handling
- `dialoguer` - Interactive prompts

## License

See LICENSE file for details.
