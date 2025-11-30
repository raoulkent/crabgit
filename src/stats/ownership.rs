use anyhow::Result;
use git2::Repository;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

use crate::stats::{StatsContext, path_filter};

#[derive(Debug, Clone, Serialize)]
pub struct FileOwnership {
    pub path: String,
    pub primary_owner: String,
    pub ownership_percentage: f64,
    pub contributors: Vec<ContributorShare>,
    pub last_modified_by: String,
    pub last_modified_time: i64,
    pub total_lines: usize,
    pub analysis_mode: OwnershipMode,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContributorShare {
    pub author: String,
    pub lines: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum OwnershipMode {
    LastModified,
    BlameAnalysis,
}

#[derive(Debug, Serialize)]
pub struct OwnershipStats {
    pub analysis_mode: OwnershipMode,
    pub files_analyzed: usize,
    pub total_files_in_repo: usize,
    pub file_ownership: Vec<FileOwnership>,
    pub overall_ownership: Vec<AuthorOwnership>,
    pub bus_factor: BusFactor,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthorOwnership {
    pub author: String,
    pub files_owned: usize,
    pub total_lines_owned: usize,
    pub ownership_percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct BusFactor {
    pub score: f64,
    pub description: String,
    pub top_contributors_coverage: f64,
    pub critical_files: usize,
}

/// Analyze code ownership using fast last-modified-by approximation
pub fn analyze_ownership_fast(
    repo: &Repository,
    ctx: &StatsContext,
    top_n: usize,
) -> Result<OwnershipStats> {
    let filtered_commits = path_filter::FilteredCommits::new(repo, ctx)?;
    let path_filters = filtered_commits.path_filters().clone();

    let mut file_last_modified: HashMap<String, (String, i64)> = HashMap::new();
    let mut total_files_seen = std::collections::HashSet::new();

    // Walk commits to find last modifier for each file
    for commit_result in filtered_commits {
        let commit = commit_result?;
        let commit_time = commit.time().seconds();

        let author = commit.author().name().unwrap_or("(unknown)").to_string();
        let changed_files = path_filter::collect_changed_paths(repo, &commit, &path_filters)?;

        if changed_files.is_empty() {
            continue;
        }

        for file_path in changed_files {
            total_files_seen.insert(file_path.clone());

            if !file_last_modified.contains_key(&file_path) {
                file_last_modified.insert(file_path, (author.clone(), commit_time));
            }
        }
    }

    // Build file ownership data (fast approximation)
    let mut file_ownership = Vec::new();
    for (file_path, (last_author, last_time)) in &file_last_modified {
        // For fast mode, we approximate that the last modifier owns 100% of the file
        let contributors = vec![ContributorShare {
            author: last_author.clone(),
            lines: 1, // Placeholder since we don't count actual lines in fast mode
            percentage: 100.0,
        }];

        file_ownership.push(FileOwnership {
            path: file_path.clone(),
            primary_owner: last_author.clone(),
            ownership_percentage: 100.0,
            contributors,
            last_modified_by: last_author.clone(),
            last_modified_time: *last_time,
            total_lines: 1, // Placeholder
            analysis_mode: OwnershipMode::LastModified,
        });
    }

    // Sort by last modified time (most recent first)
    file_ownership.sort_by(|a, b| b.last_modified_time.cmp(&a.last_modified_time));

    // Limit to top N files
    if file_ownership.len() > top_n {
        file_ownership.truncate(top_n);
    }

    // Calculate overall ownership statistics
    let overall_ownership = calculate_overall_ownership(&file_ownership);
    let bus_factor = calculate_bus_factor(&overall_ownership, file_ownership.len());

    Ok(OwnershipStats {
        analysis_mode: OwnershipMode::LastModified,
        files_analyzed: file_ownership.len(),
        total_files_in_repo: total_files_seen.len(),
        file_ownership,
        overall_ownership,
        bus_factor,
    })
}

/// Analyze code ownership using expensive blame analysis (opt-in)
pub fn analyze_ownership_blame(
    repo: &Repository,
    ctx: &StatsContext,
    top_n: usize,
) -> Result<OwnershipStats> {
    let mut file_ownership = Vec::new();
    let mut processed_files = 0;

    // Get current HEAD tree
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let tree = commit.tree()?;

    // Walk the tree to find files
    let mut files_to_analyze = Vec::new();
    let filters = path_filter::CompiledPathFilters::from_context(ctx)?;
    collect_files_for_blame(repo, &tree, "", &mut files_to_analyze, &filters)?;

    // Limit files to analyze for performance
    if files_to_analyze.len() > top_n {
        files_to_analyze.truncate(top_n);
    }

    for file_path in files_to_analyze {
        if let Ok(ownership) = analyze_file_blame(repo, &file_path) {
            file_ownership.push(ownership);
            processed_files += 1;
        }
    }

    // Sort by ownership percentage (highest first)
    file_ownership.sort_by(|a, b| {
        b.ownership_percentage
            .partial_cmp(&a.ownership_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Calculate overall ownership statistics
    let overall_ownership = calculate_overall_ownership(&file_ownership);
    let bus_factor = calculate_bus_factor(&overall_ownership, file_ownership.len());

    Ok(OwnershipStats {
        analysis_mode: OwnershipMode::BlameAnalysis,
        files_analyzed: processed_files,
        total_files_in_repo: file_ownership.len(),
        file_ownership,
        overall_ownership,
        bus_factor,
    })
}

fn analyze_file_blame(repo: &Repository, file_path: &str) -> Result<FileOwnership> {
    let blame = repo.blame_file(Path::new(file_path), None)?;
    let mut author_lines: HashMap<String, usize> = HashMap::new();
    let mut last_modified_by = String::from("(unknown)");
    let mut last_modified_time = 0i64;

    let total_lines = blame.len();

    // Count lines per author
    for i in 0..total_lines {
        if let Some(hunk) = blame.get_line(i) {
            let commit = repo.find_commit(hunk.final_commit_id())?;
            let author = commit.author().name().unwrap_or("(unknown)").to_string();
            let commit_time = commit.time().seconds();

            *author_lines.entry(author.clone()).or_insert(0) += 1;

            // Track most recent modification
            if commit_time > last_modified_time {
                last_modified_time = commit_time;
                last_modified_by = author;
            }
        }
    }

    // Find primary owner and create contributor shares
    let mut contributors: Vec<ContributorShare> = author_lines
        .iter()
        .map(|(author, &lines)| ContributorShare {
            author: author.clone(),
            lines,
            percentage: (lines as f64 / total_lines as f64) * 100.0,
        })
        .collect();

    // Sort by percentage (highest first)
    contributors.sort_by(|a, b| {
        b.percentage
            .partial_cmp(&a.percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let primary_owner = contributors
        .first()
        .map(|c| c.author.clone())
        .unwrap_or_else(|| "(unknown)".to_string());

    let ownership_percentage = contributors.first().map(|c| c.percentage).unwrap_or(0.0);

    Ok(FileOwnership {
        path: file_path.to_string(),
        primary_owner,
        ownership_percentage,
        contributors,
        last_modified_by,
        last_modified_time,
        total_lines,
        analysis_mode: OwnershipMode::BlameAnalysis,
    })
}

fn collect_files_for_blame(
    repo: &Repository,
    tree: &git2::Tree,
    prefix: &str,
    files: &mut Vec<String>,
    filters: &path_filter::CompiledPathFilters,
) -> Result<()> {
    for entry in tree {
        let name = entry.name().unwrap_or("(unknown)");
        let path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", prefix, name)
        };

        if entry.kind() == Some(git2::ObjectType::Tree) {
            if let Ok(object) = entry.to_object(repo) {
                if let Ok(subtree) = object.peel_to_tree() {
                    collect_files_for_blame(repo, &subtree, &path, files, filters)?;
                }
            }
        } else if entry.kind() == Some(git2::ObjectType::Blob) {
            if filters.matches(&path) {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn calculate_overall_ownership(file_ownership: &[FileOwnership]) -> Vec<AuthorOwnership> {
    let mut author_stats: HashMap<String, (usize, usize)> = HashMap::new(); // (files_owned, total_lines)

    for file in file_ownership {
        let entry = author_stats
            .entry(file.primary_owner.clone())
            .or_insert((0, 0));
        entry.0 += 1; // files_owned
        entry.1 += file.total_lines; // total_lines
    }

    let total_lines: usize = author_stats.values().map(|(_, lines)| *lines).sum();

    let mut overall_ownership: Vec<AuthorOwnership> = author_stats
        .iter()
        .map(
            |(author, &(files_owned, total_lines_owned))| AuthorOwnership {
                author: author.clone(),
                files_owned,
                total_lines_owned,
                ownership_percentage: if total_lines > 0 {
                    (total_lines_owned as f64 / total_lines as f64) * 100.0
                } else {
                    0.0
                },
            },
        )
        .collect();

    overall_ownership.sort_by(|a, b| {
        b.ownership_percentage
            .partial_cmp(&a.ownership_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    overall_ownership
}

fn calculate_bus_factor(overall_ownership: &[AuthorOwnership], _total_files: usize) -> BusFactor {
    if overall_ownership.is_empty() {
        return BusFactor {
            score: 1.0,
            description: "No ownership data available".to_string(),
            top_contributors_coverage: 0.0,
            critical_files: 0,
        };
    }

    // Calculate how much of the codebase is covered by top contributors
    let top_3_coverage: f64 = overall_ownership
        .iter()
        .take(3)
        .map(|a| a.ownership_percentage)
        .sum();

    // Bus factor calculation (simplified):
    // Higher concentration = lower bus factor (more risky)
    // Lower concentration = higher bus factor (less risky)
    let concentration_factor = top_3_coverage / 100.0;
    let bus_factor_score = if concentration_factor > 0.8 {
        1.0 // Very risky - top 3 own >80%
    } else if concentration_factor > 0.6 {
        2.0 // Risky - top 3 own 60-80%
    } else if concentration_factor > 0.4 {
        3.0 // Moderate - top 3 own 40-60%
    } else {
        4.0 // Good - distributed ownership
    };

    let description = match bus_factor_score as u32 {
        1 => "Critical: Very high concentration of ownership".to_string(),
        2 => "High risk: Significant ownership concentration".to_string(),
        3 => "Moderate risk: Some ownership concentration".to_string(),
        _ => "Good: Well-distributed ownership".to_string(),
    };

    // Count files with >80% single-author ownership as critical
    let critical_files = overall_ownership
        .iter()
        .filter(|a| a.ownership_percentage > 80.0)
        .map(|a| a.files_owned)
        .sum();

    BusFactor {
        score: bus_factor_score,
        description,
        top_contributors_coverage: top_3_coverage,
        critical_files,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature, Time};
    use tempfile::TempDir;

    fn default_ctx() -> StatsContext {
        StatsContext {
            repo_path: "test".into(),
            since: None,
            until: None,
            bucket: crate::stats::Bucket::Week,
            no_merges: false,
            include: None,
            exclude: None,
        }
    }

    fn create_test_repo() -> Result<(TempDir, Repository)> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;

        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;

        Ok((temp_dir, repo))
    }

    fn create_commit_with_files(
        repo: &Repository,
        message: &str,
        files: &[(&str, &str)],
        author: &str,
        parent: Option<&git2::Commit>,
    ) -> Result<git2::Oid> {
        let sig = Signature::new(author, "test@example.com", &Time::new(1000000, 0))?;

        let tree_id = {
            let mut tree_builder = repo.treebuilder(None)?;
            for (path, content) in files {
                let blob_id = repo.blob(content.as_bytes())?;
                tree_builder.insert(path, blob_id, git2::FileMode::Blob.into())?;
            }
            tree_builder.write()?
        };

        let parents: Vec<&git2::Commit> = if let Some(p) = parent {
            vec![p]
        } else {
            vec![]
        };

        let oid = {
            let tree = repo.find_tree(tree_id)?;
            repo.commit(None, &sig, &sig, message, &tree, &parents)?
        };

        repo.reference("HEAD", oid, true, "commit")?;
        Ok(oid)
    }

    #[test]
    fn test_ownership_fast_analysis() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo()?;

        let commit1_oid = create_commit_with_files(
            &repo,
            "Initial commit",
            &[("file1.txt", "content1"), ("file2.txt", "content2")],
            "Alice",
            None,
        )?;

        let commit1 = repo.find_commit(commit1_oid)?;
        create_commit_with_files(
            &repo,
            "Update file1",
            &[("file1.txt", "updated content1")],
            "Bob",
            Some(&commit1),
        )?;

        let ctx = default_ctx();
        let stats = analyze_ownership_fast(&repo, &ctx, 10)?;

        assert_eq!(stats.analysis_mode, OwnershipMode::LastModified);
        assert!(stats.files_analyzed > 0);
        assert!(!stats.overall_ownership.is_empty());

        Ok(())
    }

    #[test]
    fn test_bus_factor_calculation() -> Result<()> {
        let ownership = vec![
            AuthorOwnership {
                author: "Alice".to_string(),
                files_owned: 8,
                total_lines_owned: 800,
                ownership_percentage: 80.0,
            },
            AuthorOwnership {
                author: "Bob".to_string(),
                files_owned: 2,
                total_lines_owned: 200,
                ownership_percentage: 20.0,
            },
        ];

        let bus_factor = calculate_bus_factor(&ownership, 10);

        assert_eq!(bus_factor.score, 1.0);
        assert!(bus_factor.description.contains("Critical"));
        assert!(bus_factor.top_contributors_coverage > 50.0);

        Ok(())
    }

    #[test]
    fn test_overall_ownership_calculation() -> Result<()> {
        let file_ownership = vec![
            FileOwnership {
                path: "file1.txt".to_string(),
                primary_owner: "Alice".to_string(),
                ownership_percentage: 100.0,
                contributors: vec![],
                last_modified_by: "Alice".to_string(),
                last_modified_time: 1000000,
                total_lines: 100,
                analysis_mode: OwnershipMode::LastModified,
            },
            FileOwnership {
                path: "file2.txt".to_string(),
                primary_owner: "Bob".to_string(),
                ownership_percentage: 100.0,
                contributors: vec![],
                last_modified_by: "Bob".to_string(),
                last_modified_time: 1000000,
                total_lines: 50,
                analysis_mode: OwnershipMode::LastModified,
            },
        ];

        let overall = calculate_overall_ownership(&file_ownership);

        assert_eq!(overall.len(), 2);
        let alice = overall.iter().find(|a| a.author == "Alice").unwrap();
        let bob = overall.iter().find(|a| a.author == "Bob").unwrap();

        assert!(alice.ownership_percentage > bob.ownership_percentage);
        assert_eq!(alice.files_owned, 1);
        assert_eq!(bob.files_owned, 1);

        Ok(())
    }
}
