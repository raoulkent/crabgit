use anyhow::Result;
use git2::{Commit, Oid, Repository};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use super::StatsContext;

#[derive(Debug, Serialize, Clone)]
pub struct FileStability {
    pub path: String,
    pub changes: u32,
    pub authors: u32,
    pub reverts: u32,
    pub fixes: u32,
    pub avg_days_between_changes: f64,
    pub last_change_days_ago: u32,
    pub stability_score: f64,
    pub primary_author: String,
    pub primary_author_changes: u32,
}

#[derive(Debug, Serialize)]
pub struct StabilityStats {
    pub files: Vec<FileStability>,
    pub total_files_analyzed: usize,
    pub date_range_days: u32,
    pub filters: StabilityFilters,
}

#[derive(Debug, Serialize)]
pub struct StabilityFilters {
    pub author: Option<String>,
    pub directory: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

#[derive(Debug)]
struct FileChangeInfo {
    commits: Vec<CommitInfo>,
    authors: HashSet<String>,
}

#[derive(Debug)]
struct CommitInfo {
    _oid: Oid,
    timestamp: i64,
    author: String,
    message: String,
}

pub fn analyze_stability(
    repo: &Repository,
    ctx: &StatsContext,
    author_filter: Option<&str>,
    directory_filter: Option<&str>,
    top: usize,
) -> Result<StabilityStats> {
    let mut file_changes: HashMap<String, FileChangeInfo> = HashMap::new();

    // Parse time filters
    let (since_time, until_time) = parse_time_range(ctx.since.as_deref(), ctx.until.as_deref())?;

    // Walk through commits
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        // Filter by time range
        let commit_time = commit.time().seconds();
        if let Some(since) = since_time
            && commit_time < since
        {
            break;
        } // commits are in chronological order
        if let Some(until) = until_time
            && commit_time > until
        {
            continue;
        }

        // Filter by author if specified
        if let Some(author_filter) = author_filter {
            if let Some(author_name) = commit.author().name() {
                if !author_name.contains(author_filter) {
                    continue;
                }
            } else {
                continue;
            }
        }

        // Skip merge commits if requested
        if ctx.no_merges && commit.parent_count() > 1 {
            continue;
        }

        // Analyze the commit's changes
        analyze_commit_changes(repo, &commit, directory_filter, &mut file_changes)?;
    }

    // Calculate stability metrics for each file
    let mut file_stats: Vec<FileStability> = file_changes
        .into_iter()
        .map(|(path, info)| calculate_file_stability(path, info, since_time, until_time))
        .collect();

    // Sort by stability score (lower is less stable)
    file_stats.sort_by(|a, b| {
        a.stability_score
            .partial_cmp(&b.stability_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Take top N results
    if file_stats.len() > top {
        file_stats.truncate(top);
    }

    let total_files = file_stats.len();
    let date_range_days = if let (Some(since), Some(until)) = (since_time, until_time) {
        ((until - since) / (24 * 3600)) as u32
    } else {
        365 // default to 1 year if not specified
    };

    Ok(StabilityStats {
        files: file_stats,
        total_files_analyzed: total_files,
        date_range_days,
        filters: StabilityFilters {
            author: author_filter.map(|s| s.to_string()),
            directory: directory_filter.map(|s| s.to_string()),
            since: ctx.since.clone(),
            until: ctx.until.clone(),
        },
    })
}

fn analyze_commit_changes(
    repo: &Repository,
    commit: &Commit,
    directory_filter: Option<&str>,
    file_changes: &mut HashMap<String, FileChangeInfo>,
) -> Result<()> {
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

    diff.foreach(
        &mut |delta, _progress| {
            let file_path = match delta.new_file().path() {
                Some(path) => path.to_string_lossy().to_string(),
                None => return true, // Skip if no path
            };

            // Apply directory filter
            if let Some(dir_filter) = directory_filter
                && !file_path.starts_with(dir_filter)
            {
                return true;
            }

            // Skip binary files and non-code files
            if is_likely_binary_or_non_code(&file_path) {
                return true;
            }

            let author = commit.author().name().unwrap_or("Unknown").to_string();
            let commit_info = CommitInfo {
                _oid: commit.id(),
                timestamp: commit.time().seconds(),
                author: author.clone(),
                message: commit.message().unwrap_or("").to_string(),
            };

            let file_info = file_changes
                .entry(file_path)
                .or_insert_with(|| FileChangeInfo {
                    commits: Vec::new(),
                    authors: HashSet::new(),
                });

            file_info.commits.push(commit_info);
            file_info.authors.insert(author);

            true
        },
        None,
        None,
        None,
    )?;

    Ok(())
}

fn calculate_file_stability(
    path: String,
    mut info: FileChangeInfo,
    _since_time: Option<i64>,
    _until_time: Option<i64>,
) -> FileStability {
    // Sort commits by timestamp (oldest first)
    info.commits.sort_by_key(|c| c.timestamp);

    let changes = info.commits.len() as u32;
    let authors = info.authors.len() as u32;

    // Count reverts and fixes by analyzing commit messages
    let reverts = info
        .commits
        .iter()
        .filter(|c| is_revert_commit(&c.message))
        .count() as u32;

    let fixes = info
        .commits
        .iter()
        .filter(|c| is_fix_commit(&c.message))
        .count() as u32;

    // Calculate average days between changes
    let avg_days_between_changes = if changes > 1 {
        let first_timestamp = info.commits.first().unwrap().timestamp;
        let last_timestamp = info.commits.last().unwrap().timestamp;
        let total_days = (last_timestamp - first_timestamp) / (24 * 3600);
        total_days as f64 / (changes - 1) as f64
    } else {
        0.0
    };

    // Calculate days since last change
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let last_change_days_ago = if let Some(last_commit) = info.commits.last() {
        ((now - last_commit.timestamp) / (24 * 3600)) as u32
    } else {
        0
    };

    // Find primary author
    let mut author_commit_counts: HashMap<String, u32> = HashMap::new();
    for commit in &info.commits {
        *author_commit_counts
            .entry(commit.author.clone())
            .or_insert(0) += 1;
    }

    let (primary_author, primary_author_changes) = author_commit_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .unwrap_or(("Unknown".to_string(), 0));

    // Calculate stability score (lower = less stable)
    // Factors: high change frequency, multiple authors, reverts, fixes
    let change_frequency_factor = if avg_days_between_changes > 0.0 {
        1.0 / (avg_days_between_changes / 30.0).max(0.1) // Changes per month
    } else {
        changes as f64 // If only one change, use change count
    };

    let author_factor = authors as f64 * 0.5; // More authors = less stable
    let revert_factor = reverts as f64 * 2.0; // Reverts are bad for stability
    let fix_factor = fixes as f64 * 1.5; // Fixes indicate previous issues

    let stability_score = change_frequency_factor + author_factor + revert_factor + fix_factor;

    FileStability {
        path,
        changes,
        authors,
        reverts,
        fixes,
        avg_days_between_changes,
        last_change_days_ago,
        stability_score,
        primary_author,
        primary_author_changes,
    }
}

fn is_revert_commit(message: &str) -> bool {
    let message_lower = message.to_lowercase();
    message_lower.starts_with("revert")
        || message_lower.contains("revert")
        || message_lower.starts_with("rollback")
}

fn is_fix_commit(message: &str) -> bool {
    let message_lower = message.to_lowercase();
    message_lower.starts_with("fix")
        || message_lower.contains("hotfix")
        || message_lower.contains("bugfix")
        || message_lower.contains("patch")
        || message_lower.contains("urgent")
        || message_lower.starts_with("bug")
}

fn is_likely_binary_or_non_code(path: &str) -> bool {
    let path_lower = path.to_lowercase();

    // Skip common non-code extensions
    let non_code_extensions = [
        ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico", ".pdf", ".doc", ".docx", ".xls", ".xlsx",
        ".zip", ".tar", ".gz", ".7z", ".rar", ".exe", ".dll", ".so", ".dylib", ".lock", ".log",
        ".tmp", ".cache", ".min.js", ".min.css", // minified files
    ];

    for ext in &non_code_extensions {
        if path_lower.ends_with(ext) {
            return true;
        }
    }

    // Skip common directories that contain generated/vendor code
    let non_code_dirs = [
        "node_modules/",
        "target/",
        "build/",
        "dist/",
        "out/",
        ".git/",
        ".svn/",
        ".hg/",
        "vendor/",
        "third_party/",
        "external/",
        "coverage/",
        ".coverage/",
        "__pycache__/",
    ];

    for dir in &non_code_dirs {
        if path.contains(dir) {
            return true;
        }
    }

    false
}

fn parse_time_range(
    since: Option<&str>,
    until: Option<&str>,
) -> Result<(Option<i64>, Option<i64>)> {
    let since_time = if let Some(since_str) = since {
        Some(parse_time_spec(since_str)?)
    } else {
        None
    };

    let until_time = if let Some(until_str) = until {
        Some(parse_time_spec(until_str)?)
    } else {
        None
    };

    Ok((since_time, until_time))
}

fn parse_time_spec(spec: &str) -> Result<i64> {
    // Handle relative time specs like "30d", "3m", "1y"
    if let Some(spec_without_d) = spec.strip_suffix('d') {
        let days: u32 = spec_without_d.parse()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        return Ok(now - (days as i64 * 24 * 3600));
    } else if let Some(spec_without_w) = spec.strip_suffix('w') {
        let weeks: u32 = spec_without_w.parse()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        return Ok(now - (weeks as i64 * 7 * 24 * 3600));
    } else if let Some(spec_without_m) = spec.strip_suffix('m') {
        let months: u32 = spec_without_m.parse()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        return Ok(now - (months as i64 * 30 * 24 * 3600));
    } else if let Some(spec_without_y) = spec.strip_suffix('y') {
        let years: u32 = spec_without_y.parse()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        return Ok(now - (years as i64 * 365 * 24 * 3600));
    }

    // Handle absolute date formats (ISO 8601)
    // For now, return an error for unsupported formats
    anyhow::bail!("Unsupported time format: {}. Use formats like '30d', '3m', '1y'", spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_is_revert_commit() {
        assert!(is_revert_commit("Revert 'Add new feature'"));
        assert!(is_revert_commit("revert: broken change"));
        assert!(is_revert_commit("This reverts commit abc123"));
        assert!(is_revert_commit("Rollback previous changes"));
        
        assert!(!is_revert_commit("Add new feature"));
        assert!(!is_revert_commit("Fix bug in parser"));
        assert!(!is_revert_commit("Update documentation"));
    }

    #[test]
    fn test_is_fix_commit() {
        assert!(is_fix_commit("Fix parser bug"));
        assert!(is_fix_commit("fix: memory leak"));
        assert!(is_fix_commit("Hotfix for production issue"));
        assert!(is_fix_commit("Bugfix: handle edge case"));
        assert!(is_fix_commit("Patch for security vulnerability"));
        assert!(is_fix_commit("Urgent fix for crash"));
        assert!(is_fix_commit("Bug: null pointer exception"));
        
        assert!(!is_fix_commit("Add new feature"));
        assert!(!is_fix_commit("Refactor code"));
        assert!(!is_fix_commit("Update documentation"));
    }

    #[test]
    fn test_is_likely_binary_or_non_code() {
        // Binary files
        assert!(is_likely_binary_or_non_code("image.png"));
        assert!(is_likely_binary_or_non_code("document.pdf"));
        assert!(is_likely_binary_or_non_code("archive.zip"));
        assert!(is_likely_binary_or_non_code("app.exe"));
        assert!(is_likely_binary_or_non_code("library.dll"));
        
        // Minified files
        assert!(is_likely_binary_or_non_code("app.min.js"));
        assert!(is_likely_binary_or_non_code("style.min.css"));
        
        // Generated/vendor directories
        assert!(is_likely_binary_or_non_code("node_modules/package/index.js"));
        assert!(is_likely_binary_or_non_code("target/debug/app.exe"));
        assert!(is_likely_binary_or_non_code("vendor/library/src.rs"));
        
        // Code files (should return false)
        assert!(!is_likely_binary_or_non_code("src/main.rs"));
        assert!(!is_likely_binary_or_non_code("app.js"));
        assert!(!is_likely_binary_or_non_code("style.css"));
        assert!(!is_likely_binary_or_non_code("README.md"));
        assert!(!is_likely_binary_or_non_code("Cargo.toml"));
    }

    #[test]
    fn test_parse_time_spec() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Test days
        let result = parse_time_spec("7d").unwrap();
        let expected = now - (7 * 24 * 3600);
        assert!((result - expected).abs() <= 1, "7d should parse to 7 days ago");
        
        // Test weeks
        let result = parse_time_spec("2w").unwrap();
        let expected = now - (2 * 7 * 24 * 3600);
        assert!((result - expected).abs() <= 1, "2w should parse to 2 weeks ago");
        
        // Test months
        let result = parse_time_spec("3m").unwrap();
        let expected = now - (3 * 30 * 24 * 3600);
        assert!((result - expected).abs() <= 1, "3m should parse to 3 months ago");
        
        // Test years
        let result = parse_time_spec("1y").unwrap();
        let expected = now - (365 * 24 * 3600);
        assert!((result - expected).abs() <= 1, "1y should parse to 1 year ago");
        
        // Test invalid format
        assert!(parse_time_spec("invalid").is_err());
        assert!(parse_time_spec("30x").is_err());
    }

    #[test]
    fn test_calculate_file_stability_single_change() {
        let file_info = FileChangeInfo {
            commits: vec![CommitInfo {
                _oid: git2::Oid::zero(),
                timestamp: 1000000,
                author: "Alice".to_string(),
                message: "Add feature".to_string(),
            }],
            authors: HashSet::from(["Alice".to_string()]),
        };
        
        let result = calculate_file_stability(
            "src/main.rs".to_string(),
            file_info,
            None,
            None,
        );
        
        assert_eq!(result.path, "src/main.rs");
        assert_eq!(result.changes, 1);
        assert_eq!(result.authors, 1);
        assert_eq!(result.reverts, 0);
        assert_eq!(result.fixes, 0);
        assert_eq!(result.avg_days_between_changes, 0.0);
        assert_eq!(result.primary_author, "Alice");
        assert_eq!(result.primary_author_changes, 1);
        // Stability score should be low for single change by single author
        assert!(result.stability_score > 0.0 && result.stability_score < 5.0);
    }

    #[test]
    fn test_calculate_file_stability_multiple_changes() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let file_info = FileChangeInfo {
            commits: vec![
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: now - (30 * 24 * 3600), // 30 days ago
                    author: "Alice".to_string(),
                    message: "Add feature".to_string(),
                },
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: now - (20 * 24 * 3600), // 20 days ago
                    author: "Bob".to_string(),
                    message: "Fix bug in feature".to_string(),
                },
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: now - (10 * 24 * 3600), // 10 days ago
                    author: "Alice".to_string(),
                    message: "Revert previous fix".to_string(),
                },
            ],
            authors: HashSet::from(["Alice".to_string(), "Bob".to_string()]),
        };
        
        let result = calculate_file_stability(
            "src/unstable.rs".to_string(),
            file_info,
            None,
            None,
        );
        
        assert_eq!(result.path, "src/unstable.rs");
        assert_eq!(result.changes, 3);
        assert_eq!(result.authors, 2);
        assert_eq!(result.reverts, 1); // "Revert previous fix"
        assert_eq!(result.fixes, 1); // "Fix bug in feature"
        assert!(result.avg_days_between_changes > 0.0);
        assert_eq!(result.primary_author, "Alice"); // 2 commits vs Bob's 1
        assert_eq!(result.primary_author_changes, 2);
        
        // Should have higher instability due to multiple authors, reverts, and fixes
        assert!(result.stability_score > 5.0);
    }

    #[test]
    fn test_calculate_file_stability_scoring() {
        // Test that files with more issues get higher (worse) stability scores
        
        // Stable file: single author, no reverts/fixes
        let stable_info = FileChangeInfo {
            commits: vec![CommitInfo {
                _oid: git2::Oid::zero(),
                timestamp: 1000000,
                author: "Alice".to_string(),
                message: "Add feature".to_string(),
            }],
            authors: HashSet::from(["Alice".to_string()]),
        };
        
        let stable_result = calculate_file_stability(
            "stable.rs".to_string(),
            stable_info,
            None,
            None,
        );
        
        // Unstable file: multiple authors, reverts, fixes
        let unstable_info = FileChangeInfo {
            commits: vec![
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: 1000000,
                    author: "Alice".to_string(),
                    message: "Add feature".to_string(),
                },
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: 1001000,
                    author: "Bob".to_string(),
                    message: "Fix critical bug".to_string(),
                },
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: 1002000,
                    author: "Charlie".to_string(),
                    message: "Revert broken change".to_string(),
                },
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: 1003000,
                    author: "Dave".to_string(),
                    message: "Hotfix for production".to_string(),
                },
            ],
            authors: HashSet::from([
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Dave".to_string(),
            ]),
        };
        
        let unstable_result = calculate_file_stability(
            "unstable.rs".to_string(),
            unstable_info,
            None,
            None,
        );
        
        // Unstable file should have significantly higher stability score
        assert!(unstable_result.stability_score > stable_result.stability_score * 2.0,
            "Unstable file should have much higher instability score. Stable: {}, Unstable: {}",
            stable_result.stability_score, unstable_result.stability_score);
        
        // Verify specific metrics
        assert_eq!(unstable_result.authors, 4);
        assert_eq!(unstable_result.reverts, 1);
        assert_eq!(unstable_result.fixes, 2); // "Fix critical bug" + "Hotfix for production"
    }

    #[test]
    fn test_author_commit_counting() {
        let file_info = FileChangeInfo {
            commits: vec![
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: 1000000,
                    author: "Alice".to_string(),
                    message: "Commit 1".to_string(),
                },
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: 1001000,
                    author: "Alice".to_string(),
                    message: "Commit 2".to_string(),
                },
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: 1002000,
                    author: "Bob".to_string(),
                    message: "Commit 3".to_string(),
                },
                CommitInfo {
                    _oid: git2::Oid::zero(),
                    timestamp: 1003000,
                    author: "Alice".to_string(),
                    message: "Commit 4".to_string(),
                },
            ],
            authors: HashSet::from(["Alice".to_string(), "Bob".to_string()]),
        };
        
        let result = calculate_file_stability(
            "test.rs".to_string(),
            file_info,
            None,
            None,
        );
        
        // Alice has 3 commits, Bob has 1, so Alice should be primary author
        assert_eq!(result.primary_author, "Alice");
        assert_eq!(result.primary_author_changes, 3);
        assert_eq!(result.authors, 2);
        assert_eq!(result.changes, 4);
    }
    
    #[test]
    fn test_stability_edge_cases() {
        // Test with empty commits (should handle gracefully)
        let empty_info = FileChangeInfo {
            commits: vec![],
            authors: HashSet::new(),
        };
        
        let result = calculate_file_stability(
            "empty.rs".to_string(),
            empty_info,
            None,
            None,
        );
        
        assert_eq!(result.path, "empty.rs");
        assert_eq!(result.changes, 0);
        assert_eq!(result.authors, 0);
        assert_eq!(result.primary_author, "Unknown");
        assert_eq!(result.primary_author_changes, 0);
        
        // Test commit message edge cases
        assert!(is_fix_commit("FIX: uppercase"));
        assert!(is_fix_commit("emergency hotfix"));
        assert!(is_revert_commit("REVERT: uppercase"));
        assert!(!is_fix_commit(""));
        assert!(!is_revert_commit(""));
        
        // Test binary file detection edge cases
        assert!(is_likely_binary_or_non_code("path/to/node_modules/lib.js"));
        assert!(is_likely_binary_or_non_code("src/target/release/app"));
        assert!(!is_likely_binary_or_non_code("package.json")); // JSON files are code
        assert!(!is_likely_binary_or_non_code("Makefile")); // Build files are code
    }

    #[test]
    fn test_time_parsing_edge_cases() {
        // Test zero values
        let result = parse_time_spec("0d").unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        assert!((result - now).abs() <= 1);
        
        // Test large values
        assert!(parse_time_spec("999d").is_ok());
        assert!(parse_time_spec("52w").is_ok());
        assert!(parse_time_spec("12m").is_ok());
        assert!(parse_time_spec("10y").is_ok());
        
        // Test malformed inputs
        assert!(parse_time_spec("d").is_err());
        assert!(parse_time_spec("10").is_err());
        assert!(parse_time_spec("-5d").is_err());
        assert!(parse_time_spec("5.5d").is_err());
    }
}
