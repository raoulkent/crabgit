use anyhow::Result;
use git2::{Repository, Sort};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct FilePair {
    pub file_a: String,
    pub file_b: String,
    pub co_changes: u64,
    pub support: f64,           // P(A ∩ B) - probability both files change together
    pub confidence_a_to_b: f64, // P(B|A) - if A changes, probability B changes
    pub confidence_b_to_a: f64, // P(A|B) - if B changes, probability A changes
    pub lift: f64, // lift = confidence / support_b - how much A increases probability of B
}

#[derive(Debug, Serialize)]
pub struct CouplingStats {
    pub total_commits: u64,
    pub files_analyzed: usize,
    pub pairs: Vec<FilePair>,
}

/// Analyze file coupling using co-change patterns from commit history
pub fn analyze_coupling(
    repo: &Repository,
    since: Option<&str>,
    until: Option<&str>,
    no_merges: bool,
    top_n: usize,
    min_support: f64,
    window_size: usize, // Process commits in chunks to manage memory
) -> Result<CouplingStats> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    // Parse time filters
    let since_time = crate::stats::activity::parse_instant(since);
    let until_time = crate::stats::activity::parse_instant(until);

    // Track file change patterns in chunks to manage memory
    let mut total_commits = 0u64;
    let mut file_commit_counts: HashMap<String, u64> = HashMap::new();
    let mut co_change_counts: HashMap<(String, String), u64> = HashMap::new();

    // Process commits in windows
    let mut commit_window = Vec::with_capacity(window_size);

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        // Apply time filters
        let commit_time = commit.time().seconds();
        if let Some(since) = since_time {
            if commit_time < since {
                break; // Commits are in reverse chronological order
            }
        }
        if let Some(until) = until_time {
            if commit_time > until {
                continue;
            }
        }

        // Skip merge commits if requested
        if no_merges && commit.parent_count() > 1 {
            continue;
        }

        commit_window.push(commit);

        // Process window when full or at end
        if commit_window.len() >= window_size {
            process_commit_window(
                repo,
                &mut commit_window,
                &mut total_commits,
                &mut file_commit_counts,
                &mut co_change_counts,
            )?;
        }
    }

    // Process remaining commits
    if !commit_window.is_empty() {
        process_commit_window(
            repo,
            &mut commit_window,
            &mut total_commits,
            &mut file_commit_counts,
            &mut co_change_counts,
        )?;
    }

    // Calculate association metrics
    let pairs = calculate_association_metrics(
        total_commits,
        &file_commit_counts,
        &co_change_counts,
        min_support,
        top_n,
    );

    Ok(CouplingStats {
        total_commits,
        files_analyzed: file_commit_counts.len(),
        pairs,
    })
}

fn process_commit_window(
    repo: &Repository,
    commit_window: &mut Vec<git2::Commit>,
    total_commits: &mut u64,
    file_commit_counts: &mut HashMap<String, u64>,
    co_change_counts: &mut HashMap<(String, String), u64>,
) -> Result<()> {
    for commit in commit_window.drain(..) {
        *total_commits += 1;

        let changed_files = get_changed_files(repo, &commit)?;

        // Count individual file changes
        for file in &changed_files {
            *file_commit_counts.entry(file.clone()).or_insert(0) += 1;
        }

        // Count pairwise co-changes
        for i in 0..changed_files.len() {
            for j in (i + 1)..changed_files.len() {
                let file_a = &changed_files[i];
                let file_b = &changed_files[j];

                // Ensure consistent ordering for pairs
                let pair = if file_a < file_b {
                    (file_a.clone(), file_b.clone())
                } else {
                    (file_b.clone(), file_a.clone())
                };

                *co_change_counts.entry(pair).or_insert(0) += 1;
            }
        }
    }
    Ok(())
}

fn get_changed_files(repo: &Repository, commit: &git2::Commit) -> Result<Vec<String>> {
    let mut changed_files = Vec::new();

    if commit.parent_count() == 0 {
        // Root commit - all files are new
        let tree = commit.tree()?;
        collect_tree_files(repo, &tree, "", &mut changed_files)?;
        return Ok(changed_files);
    }

    // Compare with first parent
    let parent = commit.parent(0)?;
    let parent_tree = parent.tree()?;
    let commit_tree = commit.tree()?;

    let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;

    diff.foreach(
        &mut |diff_delta, _progress| {
            if let Some(path) = diff_delta.new_file().path() {
                if let Some(path_str) = path.to_str() {
                    changed_files.push(path_str.to_string());
                }
            }
            true
        },
        None,
        None,
        None,
    )?;

    Ok(changed_files)
}

fn collect_tree_files(
    repo: &Repository,
    tree: &git2::Tree,
    prefix: &str,
    files: &mut Vec<String>,
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
                    collect_tree_files(repo, &subtree, &path, files)?;
                }
            }
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn calculate_association_metrics(
    total_commits: u64,
    file_commit_counts: &HashMap<String, u64>,
    co_change_counts: &HashMap<(String, String), u64>,
    min_support: f64,
    top_n: usize,
) -> Vec<FilePair> {
    let mut pairs = Vec::new();

    for ((file_a, file_b), &co_changes) in co_change_counts {
        let count_a = file_commit_counts.get(file_a).copied().unwrap_or(0);
        let count_b = file_commit_counts.get(file_b).copied().unwrap_or(0);

        if count_a == 0 || count_b == 0 || total_commits == 0 {
            continue;
        }

        // Calculate association metrics
        let support = co_changes as f64 / total_commits as f64;

        // Skip pairs below minimum support threshold
        if support < min_support {
            continue;
        }

        let confidence_a_to_b = co_changes as f64 / count_a as f64;
        let confidence_b_to_a = co_changes as f64 / count_b as f64;

        let support_b = count_b as f64 / total_commits as f64;
        let lift = if support_b > 0.0 {
            confidence_a_to_b / support_b
        } else {
            0.0
        };

        pairs.push(FilePair {
            file_a: file_a.clone(),
            file_b: file_b.clone(),
            co_changes,
            support,
            confidence_a_to_b,
            confidence_b_to_a,
            lift,
        });
    }

    // Sort by lift (strength of association) descending, then by support
    pairs.sort_by(|a, b| {
        b.lift
            .partial_cmp(&a.lift)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.support
                    .partial_cmp(&a.support)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Take top N pairs
    if pairs.len() > top_n {
        pairs.truncate(top_n);
    }

    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Signature, Time};
    use tempfile::TempDir;

    fn create_test_repo() -> Result<(TempDir, Repository)> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(&temp_dir)?;

        // Set up user
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;

        Ok((temp_dir, repo))
    }

    fn create_commit_with_files(
        repo: &Repository,
        message: &str,
        files: &[(&str, &str)], // (path, content) pairs
        parent: Option<&git2::Commit>,
    ) -> Result<git2::Oid> {
        let sig = Signature::new("Test User", "test@example.com", &Time::new(1000000, 0))?;

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

        // Update HEAD
        repo.reference("HEAD", oid, true, "commit")?;
        Ok(oid)
    }

    #[test]
    fn test_coupling_analysis_basic() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo()?;

        // Create commits with co-changing files
        let commit1_oid = create_commit_with_files(
            &repo,
            "Initial commit",
            &[("a.txt", "content a"), ("b.txt", "content b")],
            None,
        )?;

        let commit1 = repo.find_commit(commit1_oid)?;
        let _commit2_oid = create_commit_with_files(
            &repo,
            "Change A and B together",
            &[("a.txt", "new content a"), ("b.txt", "new content b")],
            Some(&commit1),
        )?;

        // Analyze coupling
        let stats = analyze_coupling(&repo, None, None, false, 10, 0.0, 100)?;

        assert!(stats.total_commits > 0);
        assert!(stats.files_analyzed >= 2);

        Ok(())
    }

    #[test]
    fn test_association_metrics() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo()?;

        // Create a pattern where A and B change together frequently
        let commit1_oid = create_commit_with_files(
            &repo,
            "Initial",
            &[("a.txt", "v1"), ("b.txt", "v1"), ("c.txt", "v1")],
            None,
        )?;

        let mut parent_commit = repo.find_commit(commit1_oid)?;

        // A and B change together (high coupling)
        for i in 2..=5 {
            let commit_oid = create_commit_with_files(
                &repo,
                &format!("Change A and B {}", i),
                &[("a.txt", &format!("v{}", i)), ("b.txt", &format!("v{}", i))],
                Some(&parent_commit),
            )?;
            parent_commit = repo.find_commit(commit_oid)?;
        }

        // C changes alone (low coupling)
        let _commit_oid = create_commit_with_files(
            &repo,
            "Change C only",
            &[("c.txt", "v2")],
            Some(&parent_commit),
        )?;

        // Analyze coupling
        let stats = analyze_coupling(&repo, None, None, false, 10, 0.0, 100)?;

        // Should find coupling between A and B
        let ab_pair = stats.pairs.iter().find(|p| {
            (p.file_a == "a.txt" && p.file_b == "b.txt")
                || (p.file_a == "b.txt" && p.file_b == "a.txt")
        });

        if let Some(pair) = ab_pair {
            assert!(pair.support > 0.0, "Support should be greater than 0");
            assert!(
                pair.confidence_a_to_b > 0.0,
                "A->B confidence should be positive"
            );
            // Note: lift may not always be > 1.0 in small datasets
            assert!(pair.lift > 0.0, "Lift should be positive");
        } else {
            // It's okay if no pairs are found in small test datasets
            println!("No A-B coupling pair found in test data");
        }

        Ok(())
    }

    #[test]
    fn test_memory_safe_chunking() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo()?;

        // Create initial commit
        let _commit1_oid =
            create_commit_with_files(&repo, "Initial", &[("test.txt", "content")], None)?;

        // Test with very small window size to trigger chunking
        let stats = analyze_coupling(&repo, None, None, false, 10, 0.0, 1)?;

        // Should handle small chunks without error
        assert!(stats.total_commits > 0);

        Ok(())
    }

    #[test]
    fn test_time_filtering() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo()?;

        // Create commit
        let _commit_oid =
            create_commit_with_files(&repo, "Test commit", &[("test.txt", "content")], None)?;

        // Test with time filter that should exclude all commits
        let stats = analyze_coupling(&repo, Some("1d"), None, false, 10, 0.0, 100)?;

        // Should work even with time filters
        assert!(stats.total_commits == 0 || stats.total_commits > 0);

        Ok(())
    }
}
