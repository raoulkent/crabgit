use anyhow::Result;
use git2::{Commit, DiffOptions, Oid, Patch, Repository, Sort, Tree};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::path::Path;

use crate::stats::StatsContext;

/// A filtered commit stream that handles all filtering logic in one place
pub struct FilteredCommits<'repo> {
    repo: &'repo Repository,
    revwalk: git2::Revwalk<'repo>,
    since_ts: Option<i64>,
    until_ts: Option<i64>,
    no_merges: bool,
    path_filters: CompiledPathFilters,
}

impl<'repo> FilteredCommits<'repo> {
    /// Create a new filtered commit stream from a repository and context
    pub fn new(repo: &'repo Repository, ctx: &StatsContext) -> Result<Self> {
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        Ok(Self {
            repo,
            revwalk,
            since_ts: crate::stats::activity::parse_instant(ctx.since.as_deref()),
            until_ts: crate::stats::activity::parse_instant(ctx.until.as_deref()),
            no_merges: ctx.no_merges,
            path_filters: CompiledPathFilters::from_context(ctx)?,
        })
    }

    /// Create from repository with custom starting point
    pub fn from_oid(
        repo: &'repo Repository,
        oid: Oid,
        ctx: &StatsContext,
    ) -> Result<Self> {
        let mut revwalk = repo.revwalk()?;
        revwalk.push(oid)?;
        revwalk.set_sorting(Sort::TIME)?;

        Ok(Self {
            repo,
            revwalk,
            since_ts: crate::stats::activity::parse_instant(ctx.since.as_deref()),
            until_ts: crate::stats::activity::parse_instant(ctx.until.as_deref()),
            no_merges: ctx.no_merges,
            path_filters: CompiledPathFilters::from_context(ctx)?,
        })
    }

    /// Hide commits reachable from this OID (for branch comparisons)
    pub fn hide(&mut self, oid: Oid) -> Result<()> {
        self.revwalk.hide(oid)?;
        Ok(())
    }

    /// Get the compiled path filters
    pub fn path_filters(&self) -> &CompiledPathFilters {
        &self.path_filters
    }

    /// Check if a commit passes all filters
    fn passes_filters(&self, commit: &Commit) -> Result<bool> {
        // Check merge filter
        if self.no_merges && commit.parent_count() > 1 {
            return Ok(false);
        }

        // Check time filters
        let ts = commit.time().seconds();
        if let Some(since) = self.since_ts {
            if ts < since {
                return Ok(false);
            }
        }
        if let Some(until) = self.until_ts {
            if ts > until {
                return Ok(false);
            }
        }

        // Check path filters
        if self.path_filters.is_active()
            && !commit_has_matching_paths(self.repo, commit, &self.path_filters)?
        {
            return Ok(false);
        }

        Ok(true)
    }
}

impl<'repo> Iterator for FilteredCommits<'repo> {
    type Item = Result<Commit<'repo>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let oid = match self.revwalk.next()? {
                Ok(oid) => oid,
                Err(e) => return Some(Err(e.into())),
            };

            let commit = match self.repo.find_commit(oid) {
                Ok(c) => c,
                Err(e) => return Some(Err(e.into())),
            };

            match self.passes_filters(&commit) {
                Ok(true) => return Some(Ok(commit)),
                Ok(false) => continue, // Skip this commit
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompiledPathFilters {
    include: Option<GlobSet>,
    exclude: Option<GlobSet>,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub adds: u64,
    pub dels: u64,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ScanOutcome {
    pub matched_files: usize,
    pub total_adds: u64,
    pub total_dels: u64,
}

impl CompiledPathFilters {
    pub fn from_context(ctx: &StatsContext) -> Result<Self> {
        Self::from_patterns(ctx.include.as_deref(), ctx.exclude.as_deref())
    }

    pub fn from_patterns(include: Option<&str>, exclude: Option<&str>) -> Result<Self> {
        Ok(Self {
            include: build_globset(include)?,
            exclude: build_globset(exclude)?,
        })
    }

    pub fn is_active(&self) -> bool {
        self.include.is_some() || self.exclude.is_some()
    }

    pub fn matches(&self, path: &str) -> bool {
        let p = Path::new(path);
        if let Some(ex) = &self.exclude {
            if ex.is_match(p) {
                return false;
            }
        }
        if let Some(inc) = &self.include {
            return inc.is_match(p);
        }
        true
    }
}

pub fn commit_has_matching_paths(
    repo: &Repository,
    commit: &Commit,
    filters: &CompiledPathFilters,
) -> Result<bool> {
    if !filters.is_active() {
        return Ok(true);
    }

    let mut has_match = false;
    scan_commit_changes(repo, commit, filters, |_| {
        has_match = true;
        false
    })?;
    Ok(has_match)
}

pub fn compute_commit_churn(
    repo: &Repository,
    commit: &Commit,
    filters: &CompiledPathFilters,
) -> Result<Option<(u64, u64)>> {
    if !filters.is_active() {
        let tree = commit.tree()?;
        if commit.parent_count() == 0 {
            let (adds, dels) = diff_stats(repo, None, Some(&tree))?;
            return Ok(Some((adds, dels)));
        }
        let parent_tree = commit.parent(0)?.tree()?;
        let (adds, dels) = diff_stats(repo, Some(&parent_tree), Some(&tree))?;
        return Ok(Some((adds, dels)));
    }

    let outcome = scan_commit_changes(repo, commit, filters, |_| true)?;
    if outcome.matched_files == 0 {
        Ok(None)
    } else {
        Ok(Some((outcome.total_adds, outcome.total_dels)))
    }
}

pub fn collect_changed_paths(
    repo: &Repository,
    commit: &Commit,
    filters: &CompiledPathFilters,
) -> Result<Vec<String>> {
    if filters.is_active() {
        let mut paths = Vec::new();
        scan_commit_changes(repo, commit, filters, |change| {
            paths.push(change.path.clone());
            true
        })?;
        return Ok(paths);
    }

    if commit.parent_count() == 0 {
        let tree = commit.tree()?;
        let mut paths = Vec::new();
        collect_tree_files(repo, &tree, "", &mut paths)?;
        return Ok(paths);
    }

    let parent = commit.parent(0)?;
    let parent_tree = parent.tree()?;
    let tree = commit.tree()?;
    let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?;
    let mut paths = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .and_then(|p| p.to_str())
            {
                paths.push(path.to_string());
            }
            true
        },
        None,
        None,
        None,
    )?;
    Ok(paths)
}

pub fn scan_commit_changes<F>(
    repo: &Repository,
    commit: &Commit,
    filters: &CompiledPathFilters,
    mut on_file: F,
) -> Result<ScanOutcome>
where
    F: FnMut(&FileChange) -> bool,
{
    let tree = commit.tree()?;
    let parent_tree: Option<Tree> = if commit.parent_count() == 0 {
        None
    } else {
        Some(commit.parent(0)?.tree()?)
    };

    let mut opts = DiffOptions::new();
    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))?;

    let mut outcome = ScanOutcome::default();
    for (idx, delta) in diff.deltas().enumerate() {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(|p| p.to_str())
            .map(|s| s.to_string());
        let Some(path) = path else { continue };

        if !filters.matches(&path) {
            continue;
        }

        let mut adds = 0;
        let mut dels = 0;
        if let Some(patch) = Patch::from_diff(&diff, idx)? {
            let (_, a, d) = patch.line_stats()?;
            adds = a as u64;
            dels = d as u64;
        }

        outcome.matched_files += 1;
        outcome.total_adds += adds;
        outcome.total_dels += dels;

        if !on_file(&FileChange { path, adds, dels }) {
            break;
        }
    }

    Ok(outcome)
}

fn build_globset(pattern: Option<&str>) -> Result<Option<GlobSet>> {
    let Some(raw) = pattern else { return Ok(None) };
    let mut builder = GlobSetBuilder::new();
    let glob = GlobBuilder::new(raw).literal_separator(true).build()?;
    builder.add(glob);
    Ok(Some(builder.build()?))
}

fn diff_stats(repo: &Repository, a: Option<&Tree>, b: Option<&Tree>) -> Result<(u64, u64)> {
    let mut opts = DiffOptions::new();
    let diff = repo.diff_tree_to_tree(a, b, Some(&mut opts))?;
    let stats = diff.stats()?;
    Ok((stats.insertions() as u64, stats.deletions() as u64))
}

fn collect_tree_files(
    repo: &Repository,
    tree: &Tree,
    prefix: &str,
    out: &mut Vec<String>,
) -> Result<()> {
    for entry in tree {
        let name = entry.name().unwrap_or("(unknown)");
        let path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", prefix, name)
        };

        match entry.kind() {
            Some(git2::ObjectType::Tree) => {
                if let Ok(object) = entry.to_object(repo) {
                    if let Ok(subtree) = object.peel_to_tree() {
                        collect_tree_files(repo, &subtree, &path, out)?;
                    }
                }
            }
            Some(git2::ObjectType::Blob) => out.push(path),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_respects_include_and_exclude() -> Result<()> {
        let filters = CompiledPathFilters::from_patterns(Some("src/**"), Some("src/tests/**"))?;
        assert!(filters.matches("src/lib.rs"));
        assert!(!filters.matches("README.md"));
        assert!(!filters.matches("src/tests/mod.rs"));
        Ok(())
    }

    #[test]
    fn inactive_filters_match_everything() -> Result<()> {
        let filters = CompiledPathFilters::from_patterns(None, None)?;
        assert!(filters.matches("any/path.rs"));
        assert!(!filters.is_active() || filters.matches("foo"));
        Ok(())
    }
}
