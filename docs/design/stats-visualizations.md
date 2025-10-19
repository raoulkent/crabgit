# GitCrab Statistics and Visualization Design

Status: Draft
Owner: TBD (propose: core maintainers)
Reviewed by: —

## 1. Purpose and scope
Design the statistics and visualization layer for GitCrab, a fast CLI for inspecting Git repositories. The goal is to provide actionable insights across repo entities (repository, branches, commits, authors, files/dirs, tags/releases, remotes) with clear, performant computations and terminal-friendly visualizations, plus machine-readable outputs (JSON) for downstream tools.

Out of scope for MVP: external platform data (PRs/issues) unless retrieved from the local clone (e.g., tags and annotated messages are in scope).

## 2. Core principles
- Accurate by default, fast enough for median repos; fall back to approximate/cached when needed for very large repos.
- Consistent dimensions: time window, branch/path filters, aggregation buckets (day/week/month).
- Export-first: every stat available in JSON; terminal views are summaries atop the same data.
- Progressive enhancement: compute cheap metrics first; optionally enable expensive metrics.

## 3. Entities and candidate statistics
Below, metrics are grouped by entity. Each item suggests: definition, why it’s useful, computation source, complexity, and recommended charts.

### 3.1 Repository
- Activity over time
  - Definition: commits per bucket (day/week/month) within [since, until].
  - Why: understand cadence and trends.
  - Source: revwalk by time.
  - Complexity: O(N commits).
  - Visuals: line/area timeseries; calendar heatmap; sparkline.
- Code churn over time
  - Definition: sum of additions/deletions per bucket.
  - Why: shows velocity and refactoring spikes.
  - Source: diff per commit (numstat).
  - Complexity: O(total touched files); can be heavy; cacheable.
  - Visuals: stacked area (adds/dels); dual line; bar per bucket.
- Top authors (counts/churn)
  - Definition: commits and added+deleted lines per author in window.
  - Why: contributor distribution, bus factor signal.
  - Source: revwalk + diff.
  - Complexity: O(commits) + diff cost.
  - Visuals: horizontal bar; pareto (80/20); box/violin for per-author commit sizes.
- Active days/hours (temporal distribution)
  - Definition: histogram by weekday/hour.
  - Why: activity patterns; working hours.
  - Source: commit timestamps.
  - Visuals: heatmap (weekday x hour); circular clock histogram.
- Releases/tags cadence
  - Definition: tags per month, delta between tags.
  - Why: release rhythm.
  - Source: tags list + annotated dates.
  - Visuals: line/bar timeseries; histogram of inter-release intervals.
- Hotspots (files with persistent churn)
  - Definition: files ranked by churn weighted by recency (decay).
  - Why: maintenance risk, refactor targets.
  - Source: file-level churn + exponential decay.
  - Visuals: treemap by directory; bar chart top-N; heatmap over time.
- File coupling (co-change)
  - Definition: pairs of files edited together frequently (lift/pointwise MI).
  - Why: implicit dependencies; module boundaries.
  - Source: per-commit file sets; pair co-occurrence counts.
  - Visuals: network graph; chord diagram; matrix heatmap (top pairs).
- Bus factor (ownership concentration)
  - Definition: per-file top author share by lines last touched.
  - Why: knowledge risk.
  - Source: blame or last-modified approximation.
  - Visuals: histogram of ownership %; treemap colored by ownership.

### 3.2 Branch
- Ahead/behind vs main
  - Definition: unique commits ahead/behind selected base.
  - Why: divergence and merge urgency.
  - Source: graph range diff.
  - Visuals: dual bar; spark bar.
- Branch activity
  - Definition: commits/churn per branch over window.
  - Why: active branch hotspots.
  - Visuals: stacked area by branch; small multiples lines.
- Merge ratio
  - Definition: merges / total commits.
  - Why: integration style.
  - Visuals: bar; donut.

### 3.3 Commit
- Commit size distribution
  - Definition: histogram of added+deleted lines per commit.
  - Why: reviewability; outliers.
  - Visuals: histogram; box/violin.
- Reverts and fixups
  - Definition: commits matching revert/fixup conventions.
  - Why: stability and hygiene signals.
  - Visuals: count over time; bar by type.

### 3.4 Author
- Activity timeseries
  - Definition: commits and churn per bucket per author.
  - Visuals: faceted small multiples; stacked area.
- Ownership spread
  - Definition: number of files with >X% lines last touched by author.
  - Visuals: bar; cumulative curve.
- Review surrogate
  - Definition: merges by author; co-change breadth (files/commit median).
  - Visuals: bar/histogram.

### 3.5 File/Directory
- Churn
  - Definition: adds/dels over time; lifetime churn.
  - Visuals: per-file sparkline; treemap by directory.
- Volatility
  - Definition: commits touching file per period.
  - Visuals: heatmap (file x time) for top-N files.
- Age and recency
  - Definition: time since last modification; file age.
  - Visuals: bar/histogram; color scale on treemap.

### 3.6 Tag/Release
- Release notes size
  - Definition: total changes (commit count/churn) between tags.
  - Visuals: bar per release; slope graph over releases.

### 3.7 Remote (optional)
- Remotes and default branches
  - Definition: configured remotes, default tracking branch.
  - Visuals: table; simple list.

## 4. Cross-cutting dimensions and filters
- Time: --since, --until, --last 90d, buckets: day/week/month.
- Branch scope: one/many branches; base branch for comparisons.
- Path filters: include/exclude globs; language filters (optional later).
- Author filters: include/exclude regex; bot detection heuristics.
- Merge handling: include/exclude merge commits; first-parent only.

## 5. Output formats
- json: full structured metrics (for scripting/CI).
- table: compact terminal tables.
- chart: ASCII charts (sparklines, bars, heatmaps) or emit JSON for external plotting.

## 6. Visualization mapping (recommendations)
- Timeseries (activity, churn): line/area; stacked area for grouped series.
- Distributions (commit sizes): histogram, box, violin.
- Rankings (top-N authors/files): horizontal bars; pareto curve.
- Calendar/time-of-day patterns: calendar heatmap; weekday x hour heatmap.
- Hierarchies (dirs/files, ownership): treemap; sunburst (future GUI export).
- Relationships (file coupling): matrix heatmap; network graph (JSON export).
- Divergence (ahead/behind): dual bars; slope charts.

## 7. CLI surface (proposed)
- Repo-level
  - crabgit stats repo [--since 90d] [--until] [--bucket week] [--format json|table|chart]
  - crabgit stats calendar [--since 1y]
  - crabgit stats churn [--since 90d] [--paths 'src/**'] [--top 20]
- Authors
  - crabgit stats authors [--since 90d] [--top 15] [--metric commits|churn]
- Files/Dirs
  - crabgit stats hotspots [--since 180d] [--top 50]
  - crabgit stats coupling [--since 180d] [--top 50] [--emit json]
- Branches
  - crabgit stats branches [--base main] [--since 90d]
- Tags/Releases
  - crabgit stats releases [--since 2y]

Examples
```bash
# Last year activity (weekly), table view
crabgit stats repo --since 1y --bucket week --format table

# Top churn files in src/, last 90d
crabgit stats hotspots --since 90d --paths 'src/**' --top 25

# File coupling data as JSON for external plotting
crabgit stats coupling --since 180d --emit json > coupling.json
```

## 8. Computation and performance
- Revwalk: sort by time/topo as needed; allow first-parent when requested.
- Diff strategy: use numstat where possible to avoid full patch parsing; skip binary.
- Caching: per-commit diff stats cache (e.g., LMDB or on-disk JSON) keyed by commit id; invalidate on repo change.
- Parallelism: Rayon-based parallel mapping per commit batch; cap concurrency.
- Approximations: for ownership, start with last-modified by commit author per line via blame only when requested (--expensive).
- Limits: default top-N and time windows to avoid O(all history) by default.

## 9. Data model (internal)
- StatsContext { repo_path, branches, since, until, bucket, paths, authors, merge_mode }
- Series<T> { name, points: Vec<(timestamp/bucket, value)> }
- RankedItem { key, value, aux }
- CouplingEdge { file_a, file_b, support, confidence, lift }
- Ownership { file, author, share, last_changed }

## 10. MVP slice
- Activity timeseries (commits), churn timeseries (adds/dels), top authors (commits/churn), hotspots (files by churn with decay), calendar heatmap.
- Outputs: table + JSON; simple ASCII charts (bars, sparks) in terminal.

## 11. Future extensions
- GUI/HTML export (vega-lite spec); sunburst/treemap; network graph export.
- Language-aware stats (loc by language, need file scanning).
- PR/issue integration via provider CLIs/APIs (GitHub, GitLab) behind flags.

## 12. Open questions
- Default base branch resolution (main vs master vs configured HEAD)?
- Acceptable cost for churn by default: compute for last 90d only?
- Ownership: include blame (slow) or keep as opt-in only?
- ASCII heatmap style vs external export for richer visuals?

## 13. Acceptance criteria (for MVP)
- Commands above exist and run on medium repos (<50k commits) under reasonable time (<5s for default windows).
- All outputs available as JSON; table view readable in 80-col terminals.
- Unit tests for aggregations and diff parsing; benchmark on sample repo.
