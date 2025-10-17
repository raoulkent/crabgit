use anyhow::Result;
use git2::{Oid, Repository, Time};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct ReleaseStats {
    pub releases: Vec<Release>,
    pub total_releases: usize,
    pub date_range_days: u32,
    pub summary: ReleaseSummary,
}

#[derive(Debug, Serialize, Clone)]
pub struct Release {
    pub name: String,
    pub tag_type: TagType,
    pub date: i64,
    pub commits: u32,
    pub commits_since_previous: u32,
    pub churn_adds: u32,
    pub churn_dels: u32,
    pub churn_total: u32,
    pub days_since_previous: u32,
    pub tagger: Option<String>,
    pub message: Option<String>,
    pub commit_oid: String,
}

#[derive(Debug, Serialize, Clone)]
pub enum TagType {
    Lightweight,
    Annotated,
}

#[derive(Debug, Serialize, Clone)]
pub struct ReleaseSummary {
    pub avg_commits_per_release: f64,
    pub avg_churn_per_release: f64,
    pub avg_days_between_releases: f64,
    pub most_active_release: Option<String>,
    pub largest_churn_release: Option<String>,
}

#[derive(Debug)]
struct TagInfo {
    name: String,
    tag_type: TagType,
    date: i64,
    tagger: Option<String>,
    message: Option<String>,
    target_oid: Oid,
}

pub fn analyze_releases(repo: &Repository, limit: Option<usize>) -> Result<ReleaseStats> {
    // Get all tags sorted by date (newest first)
    let mut tags = collect_tags(repo)?;
    tags.sort_by(|a, b| b.date.cmp(&a.date));

    // Apply limit if specified
    if let Some(limit) = limit {
        tags.truncate(limit);
    }

    let mut releases = Vec::new();
    let mut previous_tag_oid: Option<Oid> = None;

    for (i, tag_info) in tags.iter().enumerate() {
        let commits_total = count_commits_to_tag(repo, tag_info.target_oid)?;

        let (commits_since_previous, churn_adds, churn_dels, days_since_previous) =
            if let Some(prev_oid) = previous_tag_oid {
                let commits = count_commits_between_tags(repo, prev_oid, tag_info.target_oid)?;
                let (adds, dels) = compute_churn_between_tags(repo, prev_oid, tag_info.target_oid)?;

                // Calculate days between this tag and the previous one
                let days = if i < tags.len() - 1 {
                    ((tags[i - 1].date - tag_info.date) / (24 * 3600)) as u32
                } else {
                    0
                };

                (commits, adds, dels, days)
            } else {
                // First tag - compare against initial commit
                let (adds, dels) = compute_churn_to_tag(repo, tag_info.target_oid)?;
                (commits_total, adds, dels, 0)
            };

        let release = Release {
            name: tag_info.name.clone(),
            tag_type: tag_info.tag_type.clone(),
            date: tag_info.date,
            commits: commits_total,
            commits_since_previous,
            churn_adds,
            churn_dels,
            churn_total: churn_adds + churn_dels,
            days_since_previous,
            tagger: tag_info.tagger.clone(),
            message: tag_info.message.clone(),
            commit_oid: tag_info.target_oid.to_string(),
        };

        releases.push(release);
        previous_tag_oid = Some(tag_info.target_oid);
    }

    let summary = calculate_summary(&releases);
    let date_range_days = if releases.len() >= 2 {
        let first_date = releases.last().unwrap().date;
        let last_date = releases.first().unwrap().date;
        ((last_date - first_date) / (24 * 3600)) as u32
    } else {
        0
    };

    Ok(ReleaseStats {
        total_releases: releases.len(),
        releases,
        date_range_days,
        summary,
    })
}

fn collect_tags(repo: &Repository) -> Result<Vec<TagInfo>> {
    let mut tags = Vec::new();

    // Iterate over all references and find tags
    repo.tag_foreach(|oid, name| {
        // Convert name bytes to string
        if let Ok(name_str) = std::str::from_utf8(name) {
            // Remove refs/tags/ prefix
            let tag_name = name_str.strip_prefix("refs/tags/").unwrap_or(name_str);

            if let Ok(obj) = repo.find_object(oid, None) {
                let (tag_type, date, tagger, message, target_oid) = match obj.kind() {
                    Some(git2::ObjectType::Tag) => {
                        // Annotated tag
                        if let Some(tag) = obj.as_tag() {
                            let tagger_info = tag
                                .tagger()
                                .map(|sig| sig.name().unwrap_or("Unknown").to_string());
                            let message_info = tag.message().map(|msg| msg.to_string());
                            let target_oid = tag.target_id();
                            (
                                TagType::Annotated,
                                tag.tagger()
                                    .map(|t| t.when())
                                    .unwrap_or(Time::new(0, 0))
                                    .seconds(),
                                tagger_info,
                                message_info,
                                target_oid,
                            )
                        } else {
                            (TagType::Annotated, 0, None, None, oid)
                        }
                    }
                    _ => {
                        // Lightweight tag - points directly to a commit
                        if let Ok(commit) = repo.find_commit(oid) {
                            (
                                TagType::Lightweight,
                                commit.time().seconds(),
                                None,
                                None,
                                oid,
                            )
                        } else {
                            (TagType::Lightweight, 0, None, None, oid)
                        }
                    }
                };

                tags.push(TagInfo {
                    name: tag_name.to_string(),
                    tag_type,
                    date,
                    tagger,
                    message,
                    target_oid,
                });
            }
        }
        true // Continue iteration
    })?;

    Ok(tags)
}

fn count_commits_to_tag(repo: &Repository, tag_oid: Oid) -> Result<u32> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push(tag_oid)?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    Ok(revwalk.count() as u32)
}

fn count_commits_between_tags(repo: &Repository, from_oid: Oid, to_oid: Oid) -> Result<u32> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push(to_oid)?;
    revwalk.hide(from_oid)?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    Ok(revwalk.count() as u32)
}

fn compute_churn_to_tag(repo: &Repository, tag_oid: Oid) -> Result<(u32, u32)> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push(tag_oid)?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut total_adds = 0;
    let mut total_dels = 0;

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        // Skip merge commits to avoid double counting
        if commit.parent_count() > 1 {
            continue;
        }

        let (adds, dels) = compute_commit_churn(repo, &commit)?;
        total_adds += adds;
        total_dels += dels;
    }

    Ok((total_adds, total_dels))
}

fn compute_churn_between_tags(repo: &Repository, from_oid: Oid, to_oid: Oid) -> Result<(u32, u32)> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push(to_oid)?;
    revwalk.hide(from_oid)?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut total_adds = 0;
    let mut total_dels = 0;

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        // Skip merge commits to avoid double counting
        if commit.parent_count() > 1 {
            continue;
        }

        let (adds, dels) = compute_commit_churn(repo, &commit)?;
        total_adds += adds;
        total_dels += dels;
    }

    Ok((total_adds, total_dels))
}

fn compute_commit_churn(repo: &Repository, commit: &git2::Commit) -> Result<(u32, u32)> {
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;
    let stats = diff.stats()?;

    Ok((stats.insertions() as u32, stats.deletions() as u32))
}

fn calculate_summary(releases: &[Release]) -> ReleaseSummary {
    if releases.is_empty() {
        return ReleaseSummary {
            avg_commits_per_release: 0.0,
            avg_churn_per_release: 0.0,
            avg_days_between_releases: 0.0,
            most_active_release: None,
            largest_churn_release: None,
        };
    }

    let total_commits: u32 = releases.iter().map(|r| r.commits_since_previous).sum();
    let total_churn: u32 = releases.iter().map(|r| r.churn_total).sum();
    let total_days: u32 = releases.iter().map(|r| r.days_since_previous).sum();

    let avg_commits_per_release = total_commits as f64 / releases.len() as f64;
    let avg_churn_per_release = total_churn as f64 / releases.len() as f64;
    let avg_days_between_releases = if releases.len() > 1 {
        total_days as f64 / (releases.len() - 1) as f64
    } else {
        0.0
    };

    let most_active_release = releases
        .iter()
        .max_by_key(|r| r.commits_since_previous)
        .map(|r| r.name.clone());

    let largest_churn_release = releases
        .iter()
        .max_by_key(|r| r.churn_total)
        .map(|r| r.name.clone());

    ReleaseSummary {
        avg_commits_per_release,
        avg_churn_per_release,
        avg_days_between_releases,
        most_active_release,
        largest_churn_release,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_repo_with_tags() -> Result<(TempDir, Repository)> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;

        // Configure user for commits
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;

        let sig = Signature::now("Test User", "test@example.com")?;

        // Create initial commit
        let initial_commit = {
            let tree_id = {
                let mut index = repo.index()?;
                let file_path = temp_dir.path().join("README.md");
                fs::write(&file_path, "# Test Repo")?;
                index.add_path(Path::new("README.md"))?;
                index.write()?;
                index.write_tree()?
            };

            let tree = repo.find_tree(tree_id)?;
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?
        };

        // Create tag v1.0.0 (lightweight)
        repo.tag_lightweight("v1.0.0", &repo.find_object(initial_commit, None)?, false)?;

        // Add another commit
        let second_commit = {
            let tree_id = {
                let mut index = repo.index()?;
                let file_path = temp_dir.path().join("feature.txt");
                fs::write(&file_path, "New feature")?;
                index.add_path(Path::new("feature.txt"))?;
                index.write()?;
                index.write_tree()?
            };

            let tree = repo.find_tree(tree_id)?;
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Add feature",
                &tree,
                &[&repo.find_commit(initial_commit)?],
            )?
        };

        // Create annotated tag v2.0.0
        {
            let target = repo.find_object(second_commit, None)?;
            repo.tag("v2.0.0", &target, &sig, "Release v2.0.0", false)?;
        }

        Ok((temp_dir, repo))
    }

    #[test]
    fn test_collect_tags() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo_with_tags()?;

        let tags = collect_tags(&repo)?;
        assert_eq!(tags.len(), 2);

        // Should have both v1.0.0 and v2.0.0
        let tag_names: Vec<&String> = tags.iter().map(|t| &t.name).collect();
        assert!(tag_names.contains(&&"v1.0.0".to_string()));
        assert!(tag_names.contains(&&"v2.0.0".to_string()));

        Ok(())
    }

    #[test]
    fn test_tag_types() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo_with_tags()?;

        let tags = collect_tags(&repo)?;

        let v1_tag = tags.iter().find(|t| t.name == "v1.0.0").unwrap();
        let v2_tag = tags.iter().find(|t| t.name == "v2.0.0").unwrap();

        assert!(matches!(v1_tag.tag_type, TagType::Lightweight));
        assert!(matches!(v2_tag.tag_type, TagType::Annotated));

        Ok(())
    }

    #[test]
    fn test_analyze_releases() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo_with_tags()?;

        let stats = analyze_releases(&repo, None)?;

        assert_eq!(stats.total_releases, 2);
        assert_eq!(stats.releases.len(), 2);

        // Check that we have the right releases (order may vary if timestamps are identical)
        let release_names: Vec<&String> = stats.releases.iter().map(|r| &r.name).collect();
        assert!(release_names.contains(&&"v1.0.0".to_string()));
        assert!(release_names.contains(&&"v2.0.0".to_string()));

        Ok(())
    }

    #[test]
    fn test_analyze_releases_with_limit() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo_with_tags()?;

        let stats = analyze_releases(&repo, Some(1))?;

        assert_eq!(stats.total_releases, 1);
        assert_eq!(stats.releases.len(), 1);
        // Should get one of the tags (order may vary with identical timestamps)
        assert!(stats.releases[0].name == "v1.0.0" || stats.releases[0].name == "v2.0.0");

        Ok(())
    }

    #[test]
    fn test_commit_counting() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo_with_tags()?;

        let stats = analyze_releases(&repo, None)?;

        // v2.0.0 should have 2 total commits, 1 since previous
        let v2_release = stats.releases.iter().find(|r| r.name == "v2.0.0").unwrap();
        assert_eq!(v2_release.commits, 2);
        assert_eq!(v2_release.commits_since_previous, 1);

        // v1.0.0 should have 1 total commit, 1 since previous (initial)
        let v1_release = stats.releases.iter().find(|r| r.name == "v1.0.0").unwrap();
        assert_eq!(v1_release.commits, 1);
        assert_eq!(v1_release.commits_since_previous, 1);

        Ok(())
    }

    #[test]
    fn test_churn_calculation() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo_with_tags()?;

        let stats = analyze_releases(&repo, None)?;

        // All releases should have some churn (files were added)
        for release in &stats.releases {
            assert!(release.churn_total > 0);
            assert!(release.churn_adds > 0);
            // Deletions might be 0 for new files
        }

        Ok(())
    }

    #[test]
    fn test_summary_calculation() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo_with_tags()?;

        let stats = analyze_releases(&repo, None)?;

        // Should have meaningful summary stats
        assert!(stats.summary.avg_commits_per_release > 0.0);
        assert!(stats.summary.avg_churn_per_release > 0.0);
        assert!(stats.summary.most_active_release.is_some());
        assert!(stats.summary.largest_churn_release.is_some());

        Ok(())
    }

    #[test]
    fn test_empty_repository() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;

        let stats = analyze_releases(&repo, None)?;

        assert_eq!(stats.total_releases, 0);
        assert!(stats.releases.is_empty());
        assert_eq!(stats.summary.avg_commits_per_release, 0.0);
        assert!(stats.summary.most_active_release.is_none());

        Ok(())
    }

    #[test]
    fn test_repository_with_no_tags() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;

        // Configure user and create a commit
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;

        let sig = Signature::now("Test User", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            let file_path = temp_dir.path().join("README.md");
            fs::write(&file_path, "# Test")?;
            index.add_path(Path::new("README.md"))?;
            index.write()?;
            index.write_tree()?
        };

        let tree = repo.find_tree(tree_id)?;
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

        let stats = analyze_releases(&repo, None)?;

        assert_eq!(stats.total_releases, 0);
        assert!(stats.releases.is_empty());

        Ok(())
    }

    #[test]
    fn test_single_tag() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;

        // Configure and create commit
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;

        let sig = Signature::now("Test User", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            let file_path = temp_dir.path().join("README.md");
            fs::write(&file_path, "# Test")?;
            index.add_path(Path::new("README.md"))?;
            index.write()?;
            index.write_tree()?
        };

        let tree = repo.find_tree(tree_id)?;
        let commit_oid = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

        // Create single tag
        repo.tag_lightweight("v1.0.0", &repo.find_object(commit_oid, None)?, false)?;

        let stats = analyze_releases(&repo, None)?;

        assert_eq!(stats.total_releases, 1);
        assert_eq!(stats.releases[0].name, "v1.0.0");
        assert_eq!(stats.releases[0].commits, 1);
        assert_eq!(stats.releases[0].days_since_previous, 0); // No previous release

        Ok(())
    }

    #[test]
    fn test_tag_message_and_tagger() -> Result<()> {
        let (_temp_dir, repo) = create_test_repo_with_tags()?;

        let stats = analyze_releases(&repo, None)?;

        let v2_release = stats.releases.iter().find(|r| r.name == "v2.0.0").unwrap();
        assert!(matches!(v2_release.tag_type, TagType::Annotated));
        assert!(v2_release.tagger.is_some());
        assert!(v2_release.message.is_some());
        assert_eq!(v2_release.message.as_ref().unwrap(), "Release v2.0.0");

        let v1_release = stats.releases.iter().find(|r| r.name == "v1.0.0").unwrap();
        assert!(matches!(v1_release.tag_type, TagType::Lightweight));
        assert!(v1_release.tagger.is_none());
        assert!(v1_release.message.is_none());

        Ok(())
    }
}
