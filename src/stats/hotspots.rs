use anyhow::Result;
use git2::{DiffOptions, Patch, Repository, Sort, Tree};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use time::OffsetDateTime;

use crate::stats::StatsContext;

#[derive(Debug, Serialize, Clone)]
pub struct FileHotspot {
    pub path: String,
    pub adds: u64,
    pub dels: u64,
    pub churn: u64,
    pub weighted: f64,
}

pub fn compute_hotspots(
    repo: &Repository,
    ctx: &StatsContext,
    include_glob: Option<&str>,
    exclude_glob: Option<&str>,
    half_life_days: f64,
) -> Result<Vec<FileHotspot>> {
    let (since_ts, until_ts) = (
        crate::stats::activity::parse_instant(ctx.since.as_deref()),
        crate::stats::activity::parse_instant(ctx.until.as_deref()),
    );

    let include = build_globset(include_glob)?;
    let exclude = build_globset(exclude_glob)?;

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let mut by_file: HashMap<String, (u64, u64, f64)> = HashMap::new(); // adds, dels, weighted

    let reference_ts = until_ts.unwrap_or_else(|| OffsetDateTime::now_utc().unix_timestamp());
    let hl = if half_life_days > 0.0 { half_life_days } else { 90.0 };

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        if ctx.no_merges && commit.parent_count() > 1 { continue; }
        let ts = commit.time().seconds();
        if let Some(since) = since_ts && ts < since { continue; }
        if let Some(until) = until_ts && ts > until { continue; }

        let parent_tree_opt: Option<Tree> = if commit.parent_count() == 0 {
            None
        } else {
            Some(commit.parent(0)?.tree()?)
        };
        let tree = commit.tree()?;

        let mut opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(parent_tree_opt.as_ref(), Some(&tree), Some(&mut opts))?;

        // Recency weight
        let age_days = ((reference_ts - ts) as f64) / 86_400.0;
        let weight = 0.5f64.powf(age_days / hl);

        for (i, delta) in diff.deltas().enumerate() {
            // Determine path (prefer new path)
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .and_then(|p| p.to_str())
                .map(|s| s.to_string());
            let Some(path) = path else { continue };

            if !matches_globsets(&path, include.as_ref(), exclude.as_ref()) {
                continue;
            }

            // Compute per-file adds/dels via patch line stats
            if let Some(patch) = Patch::from_diff(&diff, i)? {
                let (_ctx, adds, dels) = patch.line_stats()?;
                let entry = by_file.entry(path).or_insert((0, 0, 0.0));
                entry.0 += adds as u64;
                entry.1 += dels as u64;
                entry.2 += weight * (adds as f64 + dels as f64);
            }
        }
    }

    let mut out: Vec<FileHotspot> = by_file
        .into_iter()
        .map(|(path, (adds, dels, weighted))| FileHotspot {
            path,
            adds,
            dels,
            churn: adds + dels,
            weighted,
        })
        .collect();

    // Sort by weighted churn desc, then raw churn
    out.sort_by(|a, b| b
        .weighted
        .partial_cmp(&a.weighted)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| b.churn.cmp(&a.churn))
        .then_with(|| a.path.cmp(&b.path))
    );

    Ok(out)
}

fn build_globset(pattern: Option<&str>) -> Result<Option<GlobSet>> {
    let Some(pat) = pattern else { return Ok(None) };
    let mut builder = GlobSetBuilder::new();
    // Use literal separators and case-sensitive matching by default
    let g = GlobBuilder::new(pat).literal_separator(true).build()?;
    builder.add(g);
    Ok(Some(builder.build()?))
}

fn matches_globsets(path: &str, include: Option<&GlobSet>, exclude: Option<&GlobSet>) -> bool {
    let p = Path::new(path);
    if let Some(ex) = exclude && ex.is_match(p) { return false; }
    if let Some(inc) = include { return inc.is_match(p); }
    true
}
