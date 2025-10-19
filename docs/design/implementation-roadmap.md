# GitCrab Implementation Roadmap

Status: Draft
Owner: TBD

Goals
- Deliver stats and visualizations in small, shippable increments.
- Keep defaults fast; gate expensive analysis behind flags; provide JSON outputs for tooling.

Phase 0 — Scaffolding and foundations (1–2 days)
- Add module layout: stats/, output/{table,json,ascii}/, cli/stats.rs.
- Introduce StatsContext and bucketing helpers (time windows, bucket sizes).
- Add output selection: --format json|table|chart (ASCII). Implement minimal writers.
- Tests: unit tests for bucketing and format selection.
- Acceptance: crabgit stats repo runs and prints a placeholder JSON with context.

Phase 1 — Activity timeseries (commits) (0.5–1 day)
- Implement commit revwalk filtered by [since, until], optional branches/paths.
- Aggregate per bucket (day/week/month). Add --bucket with validation.
- Outputs: JSON series + table; ASCII sparkline for quick glance.
- Tests: synthetic commits fixture; verify counts per bucket.
- Acceptance: crabgit stats repo --since 90d prints commits/bucket in <2s on medium repos.

Phase 2 — Churn timeseries (adds/dels) (1–2 days)
- Compute per-commit numstat (adds, dels); exclude binary; respect path filters.
- Aggregate adds/dels per bucket; output stacked table and JSON series.
- Performance: early stop with time window; skip merges with --no-merges option.
- Tests: patch fixtures; validate aggregation math.
- Acceptance: churn last 90d runs in <5s on medium repos.

Phase 3 — Top authors (commits and churn) (0.5–1 day)
- Aggregate commits and churn per author; support --top N and --metric.
- ASCII horizontal bars; JSON with totals and ranks.
- Tests: author aggregation; tie-breaking; sorting.
- Acceptance: authors table matches JSON and sorts deterministically.

Phase 4 — Calendar and hour-of-day heatmaps (1 day)
- Bucket by weekday/hour; produce 7x24 matrix.
- ASCII heatmap renderer; JSON matrix for external plotting.
- Tests: mapping timestamps to buckets; rendering boundaries.
- Acceptance: heatmap visible in 80-col; JSON schema documented.

Phase 5 — Hotspots (files by churn with decay) (1–2 days)
- Compute file-level churn; apply recency decay (configurable half-life).
- Support path globs and language include/exclude (basic extension filter).
- Outputs: top-N table, JSON; optional directory aggregation.
- Tests: decay weighting; top-N correctness.
- Acceptance: hotspots top 25 returns in <5s for last 180d.

Phase 6 — Branch metrics (ahead/behind, activity, merge ratio) (1 day)
- Add ahead/behind vs --base (default resolves from remote HEAD if not set).
- Branch activity (commits/churn per branch) within window; merge ratio per branch.
- Outputs: small tables; JSON.
- Tests: graph range diffs; merge commit detection.
- Acceptance: ahead/behind matches git output on test repos.

Phase 7 — File coupling (co-change) (2–3 days)
- Build per-commit file sets; count co-occurrences; compute support/confidence/lift.
- Memory-safe chunking for large histories; limit by window and top-N.
- Outputs: JSON edges; ASCII top pairs table.
- Tests: synthetic commits ensure expected pairs and metrics.
- Acceptance: coupling for last 180d on medium repos in <10s (configurable limits).

Phase 8 — Ownership and bus factor (opt-in) (2–3 days)
- Fast approximation: last-modified-by per file from history (no blame).
- Optional: --expensive uses blame to compute per-line shares for top-N files.
- Outputs: ownership histogram; JSON per-file shares.
- Tests: approximation vs blame on small fixtures; performance guardrails.
- Acceptance: approximation <5s; blame mode gated and cancelable.

Phase 9 — Tags/Releases metrics (0.5–1 day)
- Enumerate tags with dates; compute commits/churn between tags.
- Outputs: per-release table; JSON series.
- Tests: tag range computations; annotated vs lightweight tags.
- Acceptance: release diffing matches git log ranges.

Phase 10 — Performance, caching, and ergonomics (ongoing)
- Add optional on-disk cache for per-commit numstat keyed by OID.
- Parallelize per-commit processing (Rayon) with concurrency caps.
- Config defaults (window=90d, top=25) and global config file.
- Telemetry: simple timing logs under --debug.

Phase 11 — Polish, docs, and stability (0.5–1 day)
- Man pages/help text examples; README updates; docs for JSON schemas.
- Integration tests on sample repos; benchmark script.
- Stabilize CLI flags; deprecate experimental ones.

Testing strategy
- Unit tests for aggregations and bucketing; golden tests for ASCII renderers.
- Integration tests against small fixture repos (generated locally during tests).
- Performance checks via benchmark script (optional CI job).

Deliverables per phase
- Code, unit tests, and docs snippets.
- JSON schema examples added under docs/schemas/.

Risk and mitigations
- Large histories: default windows + caches + streaming.
- Blame cost: opt-in and scoped to top-N paths.
- Output width: ASCII renderers adapt to terminal width; fall back to table.
