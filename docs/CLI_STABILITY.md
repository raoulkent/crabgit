# GitCrab CLI Stability Guarantee

This document outlines the CLI stability guarantees for GitCrab and documents the evolution of command-line interfaces.

## Stability Policy

GitCrab follows semantic versioning for CLI stability:

- **Major version changes** (e.g., 1.x → 2.x): May include breaking changes to CLI flags, command structure, or output formats
- **Minor version changes** (e.g., 1.1 → 1.2): May add new commands/flags but maintain backward compatibility
- **Patch version changes** (e.g., 1.1.1 → 1.1.2): Bug fixes only, no CLI changes

## Stable CLI Elements (v1.0+)

The following CLI elements are considered stable and will not change without a major version bump:

### Core Commands
- `crabgit` (default status display)
- `crabgit status` - Repository status information
- `crabgit branches` - Branch listing
- `crabgit log` - Commit history
- `crabgit stats` - Statistical analysis (with subcommands)
- `crabgit tui` - Terminal UI mode  
- `crabgit interactive` - Interactive menu mode

### Global Options
- `--repo <PATH>` - Repository path specification
- `--debug` - Enable debug/timing output
- `--help` - Show help information
- `--version` - Show version information

### Stats Subcommands
- `stats repo` - Repository activity and churn analysis
- `stats authors` - Author contribution statistics  
- `stats calendar` - Activity heatmap analysis
- `stats hotspots` - File hotspot analysis with decay
- `stats branches` - Branch comparison and metrics
- `stats coupling` - File co-change analysis
- `stats ownership` - Code ownership analysis
- `stats stability` - Change stability analysis
- `stats releases` - Release/tag analysis

### Output Formats
- `--format table` - Human-readable table output
- `--format json` - Machine-readable JSON output  
- `--format chart` - ASCII chart/visualization output

### Time Window Specifications
- Relative formats: `30d`, `12w`, `6m`, `2y`
- Absolute formats: `2024-01-01`, `2024-12-31`
- `--since` and `--until` parameters

### Common Parameters
- `--top N` - Limit results to top N items
- `--no-merges` - Exclude merge commits
- `--since` / `--until` - Time window specification

## Performance Options (Stable)

- `--no-cache` - Disable result caching
- `--no-parallel` - Disable parallel processing  
- `--max-threads N` - Control thread pool size

## Experimental Features

Currently, there are no experimental CLI features. All documented commands and flags are considered stable.

## Deprecated Features

No features are currently deprecated. When features are deprecated:

1. They will be marked as deprecated in help text
2. A deprecation warning will be shown when used
3. Alternative approaches will be documented  
4. Deprecated features will be removed only in major version updates

## JSON Output Stability

JSON output schemas are versioned and documented in `/docs/schemas/`. The schema format follows these stability rules:

- **Additive changes** (new fields): Minor version updates
- **Breaking changes** (removed/renamed fields): Major version updates
- **Schema validation**: All outputs validate against published schemas

## CLI Evolution History

### v0.1.0 (Current)
- Initial CLI design implemented
- All core commands and stats subcommands available
- JSON schema documentation established
- CLI stability policy established

## Reporting CLI Issues

If you encounter CLI issues or have suggestions:

1. Check if the behavior is documented in this stability guide
2. Review the help text with `--help` for current behavior
3. Open an issue with:
   - GitCrab version (`crabgit --version`)
   - Full command line used
   - Expected vs. actual behavior
   - Sample output if relevant

## Testing CLI Compatibility

GitCrab includes comprehensive CLI integration tests to ensure stability:

- All stable commands are tested across different scenarios
- JSON output is validated against schemas
- Breaking changes trigger test failures
- Performance benchmarks track CLI response times

Run the test suite with:
```bash
cargo test
```

Run integration tests specifically:
```bash  
cargo test --test cli_stats
cargo test --test integration_comprehensive
```