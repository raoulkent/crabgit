use anyhow::Result;
use git2::{Repository, Sort};
use serde::Serialize;
use std::collections::HashMap;

use crate::stats::StatsContext;

#[derive(Debug, Serialize, Clone)]
pub struct AuthorStats {
    pub author: String,
    pub commits: u64,
    pub adds: u64,
    pub dels: u64,
}

pub fn compute_authors(repo: &Repository, ctx: &StatsContext) -> Result<Vec<AuthorStats>> {
    let (since_ts, until_ts) = (
        crate::stats::activity::parse_instant(ctx.since.as_deref()),
        crate::stats::activity::parse_instant(ctx.until.as_deref()),
    );

    let mut by_author: HashMap<String, AuthorStats> = HashMap::new();

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let ts = commit.time().seconds();
        if let Some(since) = since_ts && ts < since { continue; }
        if let Some(until) = until_ts && ts > until { continue; }

        // Skip merges if requested
        if ctx.no_merges && commit.parent_count() > 1 {
            continue;
        }

        let author = commit.author().name().unwrap_or("Unknown").to_string();
        let entry = by_author.entry(author.clone()).or_insert(AuthorStats { author, commits: 0, adds: 0, dels: 0 });
        entry.commits += 1;

        // Churn via diff to first parent (or empty tree for root)
        let (adds, dels) = if commit.parent_count() == 0 {
            let tree = commit.tree()?;
            crate::stats::churn::diff_trees_public(repo, None, Some(&tree))?
        } else {
            let parent = commit.parent(0)?;
            let parent_tree = parent.tree()?;
            let tree = commit.tree()?;
            crate::stats::churn::diff_trees_public(repo, Some(&parent_tree), Some(&tree))?
        };
        entry.adds += adds;
        entry.dels += dels;
    }

    let mut v: Vec<AuthorStats> = by_author.into_values().collect();
    // sort by commits desc as default; callers can re-sort
    v.sort_by(|a, b| b.commits.cmp(&a.commits).then_with(|| (b.adds + b.dels).cmp(&(a.adds + a.dels))));
    Ok(v)
}
