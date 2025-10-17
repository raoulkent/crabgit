use anyhow::Result;
use git2::{DiffOptions, Repository, Sort, Tree};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::stats::{Bucket, StatsContext};

#[derive(Debug, Serialize, Clone)]
pub struct ChurnPoint {
    pub bucket_start: i64,
    pub adds: u64,
    pub dels: u64,
}

pub fn compute_churn(repo: &Repository, ctx: &StatsContext) -> Result<Vec<ChurnPoint>> {
    let (since_ts, until_ts) = (
        crate::stats::activity::parse_instant(ctx.since.as_deref()),
        crate::stats::activity::parse_instant(ctx.until.as_deref()),
    );

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let mut buckets: BTreeMap<i64, (u64, u64)> = BTreeMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let ts = commit.time().seconds();
        if let Some(since) = since_ts
            && ts < since
        {
            continue;
        }
        if let Some(until) = until_ts
            && ts > until
        {
            continue;
        }

        let parents = commit.parent_count();
        if parents == 0 {
            let tree = commit.tree()?;
            let (adds, dels) = diff_trees(repo, None, Some(&tree))?;
            let key = bucket_start(ts, ctx.bucket);
            let entry = buckets.entry(key).or_insert((0, 0));
            entry.0 += adds;
            entry.1 += dels;
        } else {
            if parents > 1 && ctx.no_merges {
                continue;
            }
            let parent = commit.parent(0)?;
            let parent_tree = parent.tree()?;
            let tree = commit.tree()?;
            let (adds, dels) = diff_trees(repo, Some(&parent_tree), Some(&tree))?;
            let key = bucket_start(ts, ctx.bucket);
            let entry = buckets.entry(key).or_insert((0, 0));
            entry.0 += adds;
            entry.1 += dels;
        }
    }

    Ok(buckets
        .into_iter()
        .map(|(k, (a, d))| ChurnPoint {
            bucket_start: k,
            adds: a,
            dels: d,
        })
        .collect())
}

fn diff_trees(repo: &Repository, a: Option<&Tree>, b: Option<&Tree>) -> Result<(u64, u64)> {
    let mut opts = DiffOptions::new();
    // default options; could add pathspec filtering later
    let diff = repo.diff_tree_to_tree(a, b, Some(&mut opts))?;
    let stats = diff.stats()?;
    Ok((stats.insertions() as u64, stats.deletions() as u64))
}

pub fn diff_trees_public(
    repo: &Repository,
    a: Option<&Tree>,
    b: Option<&Tree>,
) -> Result<(u64, u64)> {
    diff_trees(repo, a, b)
}

fn bucket_start(ts: i64, bucket: Bucket) -> i64 {
    match bucket {
        Bucket::Day => ts - (ts % 86_400),
        Bucket::Week => {
            let day = ts / 86_400;
            let week_day0 = day - (day % 7);
            week_day0 * 86_400
        }
        Bucket::Month => crate::stats::activity::month_floor(ts),
    }
}
