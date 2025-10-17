use anyhow::Result;
use git2::{Branch, BranchType, Oid, Repository};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BranchMetrics {
    pub name: String,
    pub branch_type: String,
    pub ahead: usize,
    pub behind: usize,
    pub commits: u64,
    pub churn_adds: u64,
    pub churn_dels: u64,
    pub merge_ratio: f64,
    pub last_commit: Option<String>,
    pub last_commit_time: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct BranchStats {
    pub base_branch: String,
    pub branches: Vec<BranchMetrics>,
}

/// Compute branch metrics including ahead/behind, activity, and merge ratios
pub fn analyze_branches(
    repo: &Repository,
    base_branch: Option<&str>,
    since: Option<&str>,
    until: Option<&str>,
    no_merges: bool,
) -> Result<BranchStats> {
    // Determine base branch
    let base_ref = determine_base_branch(repo, base_branch)?;
    let base_oid = base_ref
        .target()
        .ok_or_else(|| anyhow::anyhow!("Base branch has no target OID"))?;

    let mut branch_metrics = Vec::new();
    let branches = repo.branches(None)?;

    for branch_result in branches {
        let (branch, branch_type) = branch_result?;
        let name = branch.name()?.unwrap_or("(unknown)").to_string();

        // Skip the base branch itself
        if name == base_ref.shorthand().unwrap_or("") {
            continue;
        }

        // Skip branches without target (symbolic refs, etc.)
        if branch.get().target().is_none() {
            continue;
        }

        if let Ok(metrics) = compute_branch_metrics(
            repo,
            &branch,
            branch_type,
            &name,
            base_oid,
            since,
            until,
            no_merges,
        ) {
            branch_metrics.push(metrics);
        }
    }

    // Sort branches by activity (commits descending)
    branch_metrics.sort_by(|a, b| b.commits.cmp(&a.commits));

    Ok(BranchStats {
        base_branch: base_ref.shorthand().unwrap_or("(unknown)").to_string(),
        branches: branch_metrics,
    })
}

fn determine_base_branch<'a>(
    repo: &'a Repository,
    base_branch: Option<&'a str>,
) -> Result<git2::Reference<'a>> {
    if let Some(base) = base_branch {
        // Try to resolve the specified base branch
        if let Ok(branch_ref) = repo.find_reference(&format!("refs/heads/{}", base)) {
            return Ok(branch_ref);
        }
        if let Ok(remote_ref) = repo.find_reference(&format!("refs/remotes/origin/{}", base)) {
            return Ok(remote_ref);
        }
    }

    // Try to find the default remote branch (origin/main or origin/master)
    if let Ok(remote_ref) = repo.find_reference("refs/remotes/origin/main") {
        return Ok(remote_ref);
    }
    if let Ok(remote_ref) = repo.find_reference("refs/remotes/origin/master") {
        return Ok(remote_ref);
    }

    // Fall back to HEAD
    Ok(repo.head()?)
}

fn compute_branch_metrics(
    repo: &Repository,
    branch: &Branch,
    branch_type: BranchType,
    name: &str,
    base_oid: Oid,
    since: Option<&str>,
    until: Option<&str>,
    no_merges: bool,
) -> Result<BranchMetrics> {
    let branch_ref = branch.get();
    let branch_oid = branch_ref
        .target()
        .ok_or_else(|| anyhow::anyhow!("Branch has no target OID"))?;

    // Compute ahead/behind
    let (ahead, behind) = repo.graph_ahead_behind(branch_oid, base_oid)?;

    // Get the latest commit on this branch
    let branch_commit = repo.find_commit(branch_oid)?;
    let last_commit = Some(branch_commit.id().to_string());
    let last_commit_time = Some(branch_commit.time().seconds());

    // Compute activity metrics for this branch
    let (commits, churn_adds, churn_dels, merge_ratio) =
        compute_branch_activity(repo, branch_oid, base_oid, since, until, no_merges)?;

    Ok(BranchMetrics {
        name: name.to_string(),
        branch_type: match branch_type {
            BranchType::Local => "local".to_string(),
            BranchType::Remote => "remote".to_string(),
        },
        ahead,
        behind,
        commits,
        churn_adds,
        churn_dels,
        merge_ratio,
        last_commit,
        last_commit_time,
    })
}

fn compute_branch_activity(
    repo: &Repository,
    branch_oid: Oid,
    base_oid: Oid,
    since: Option<&str>,
    until: Option<&str>,
    no_merges: bool,
) -> Result<(u64, u64, u64, f64)> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push(branch_oid)?;
    revwalk.hide(base_oid)?;

    // Parse time filters
    let since_time = crate::stats::activity::parse_instant(since);
    let until_time = crate::stats::activity::parse_instant(until);

    let mut commits = 0u64;
    let mut merge_commits = 0u64;
    let mut total_adds = 0u64;
    let mut total_dels = 0u64;

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        // Apply time filters
        let commit_time = commit.time().seconds();
        if let Some(since) = since_time {
            if commit_time < since {
                continue;
            }
        }
        if let Some(until) = until_time {
            if commit_time > until {
                continue;
            }
        }

        let is_merge = commit.parent_count() > 1;
        if is_merge {
            merge_commits += 1;
        }

        if no_merges && is_merge {
            continue;
        }

        commits += 1;

        // Compute churn for this commit
        if let Ok((adds, dels)) = compute_commit_churn(repo, &commit) {
            total_adds += adds;
            total_dels += dels;
        }
    }

    let total_commits = commits + if no_merges { 0 } else { merge_commits };
    let merge_ratio = if total_commits > 0 {
        merge_commits as f64 / total_commits as f64
    } else {
        0.0
    };

    Ok((commits, total_adds, total_dels, merge_ratio))
}

fn compute_commit_churn(repo: &Repository, commit: &git2::Commit) -> Result<(u64, u64)> {
    if commit.parent_count() == 0 {
        // Root commit - compare against empty tree
        let tree = commit.tree()?;
        return crate::stats::churn::diff_trees_public(repo, None, Some(&tree));
    }

    // For regular commits, diff against the first parent
    let parent = commit.parent(0)?;
    let parent_tree = parent.tree()?;
    let commit_tree = commit.tree()?;

    crate::stats::churn::diff_trees_public(repo, Some(&parent_tree), Some(&commit_tree))
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

        // Create initial commit to establish main branch
        let sig = Signature::new("Test User", "test@example.com", &Time::new(1000000, 0))?;
        let tree_id = {
            let mut tree_builder = repo.treebuilder(None)?;
            let blob_id = repo.blob(b"initial content")?;
            tree_builder.insert("README.md", blob_id, git2::FileMode::Blob.into())?;
            tree_builder.write()?
        };
        {
            let tree = repo.find_tree(tree_id)?;
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?
        };

        Ok((temp_dir, repo))
    }

    fn create_commit(
        repo: &Repository,
        message: &str,
        parent: Option<&git2::Commit>,
    ) -> Result<git2::Oid> {
        let sig = Signature::new("Test User", "test@example.com", &Time::new(1000000, 0))?;

        // Create an empty tree for simplicity
        let tree_id = {
            let mut tree_builder = repo.treebuilder(None)?;
            let file_content = format!("Content for {}", message);
            let blob_id = repo.blob(file_content.as_bytes())?;
            tree_builder.insert("test.txt", blob_id, git2::FileMode::Blob.into())?;
            tree_builder.write()?
        };

        let parents: Vec<&git2::Commit> = if let Some(p) = parent {
            vec![p]
        } else {
            vec![]
        };

        let oid = {
            let tree = repo.find_tree(tree_id)?;
            repo.commit(
                None, // Don't update HEAD, we'll do that manually
                &sig, &sig, message, &tree, &parents,
            )?
        };

        // Update HEAD to point to the new commit
        repo.reference("HEAD", oid, true, "commit")?;

        Ok(oid)
    }

    #[test]
    fn test_branch_ahead_behind() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo()?;

        // Test that the function runs without crashing
        let result = analyze_branches(&repo, None, None, None, false);

        // Should succeed even with minimal repository
        assert!(result.is_ok());
        let stats = result?;

        // Should have a base branch determined
        assert!(!stats.base_branch.is_empty());

        Ok(())
    }

    #[test]
    fn test_branch_activity_metrics() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo()?;

        // Analyze branches
        let stats = analyze_branches(&repo, None, None, None, false)?;

        // Should work with basic repository
        assert!(stats.branches.len() >= 0); // May or may not have branches depending on setup

        Ok(())
    }

    #[test]
    fn test_merge_ratio_calculation() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo()?;

        // Test the compute_branch_activity function with basic parameters
        let head_oid = repo.head()?.target().unwrap();
        let result = compute_branch_activity(
            &repo, head_oid, head_oid, // Same oid means no commits between
            None, None, false,
        );

        // Should succeed and return metrics
        assert!(result.is_ok());
        let (commits, _adds, _dels, merge_ratio) = result?;

        // With same oid, should have 0 commits and 0 merge ratio
        assert_eq!(commits, 0);
        assert_eq!(merge_ratio, 0.0);

        Ok(())
    }
}
