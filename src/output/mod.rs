pub mod json;
pub mod table;
pub mod ascii;

use crate::stats::{AuthorMetric, OutputFormat, StatsContext, StatsMetric};
use anyhow::Result;
use git2::Repository;

#[derive(serde::Serialize)]
#[serde(tag = "metric")]
enum RepoSeries {
    #[serde(rename = "activity")]
    Activity { bucket: crate::stats::Bucket, series: Vec<crate::stats::activity::ActivityPoint> },
    #[serde(rename = "churn")]
    Churn { bucket: crate::stats::Bucket, series: Vec<crate::stats::churn::ChurnPoint> },
}

#[derive(serde::Serialize)]
struct RepoOutput {
    context: StatsContext,
    #[serde(flatten)]
    data: RepoSeries,
}

pub fn render_repo(repo: &Repository, metric: StatsMetric, ctx: &StatsContext, fmt: OutputFormat) -> Result<()> {
    match (fmt, metric) {
        (OutputFormat::Json, StatsMetric::Activity) => {
            let series = crate::stats::activity::compute_activity(repo, ctx)?;
            let out = RepoOutput { context: ctx.clone(), data: RepoSeries::Activity { bucket: ctx.bucket, series } };
            json::print(&out)
        }
        (OutputFormat::Json, StatsMetric::Churn) => {
            let series = crate::stats::churn::compute_churn(repo, ctx)?;
            let out = RepoOutput { context: ctx.clone(), data: RepoSeries::Churn { bucket: ctx.bucket, series } };
            json::print(&out)
        }
        (OutputFormat::Table, StatsMetric::Activity) => {
            let series = crate::stats::activity::compute_activity(repo, ctx)?;
            table::print_repo_activity(ctx, &series)
        }
        (OutputFormat::Table, StatsMetric::Churn) => {
            let series = crate::stats::churn::compute_churn(repo, ctx)?;
            table::print_repo_churn(ctx, &series)
        }
        (OutputFormat::Chart, StatsMetric::Activity) => {
            let series = crate::stats::activity::compute_activity(repo, ctx)?;
            ascii::print_repo_activity(ctx, &series)
        }
        (OutputFormat::Chart, StatsMetric::Churn) => {
            let series = crate::stats::churn::compute_churn(repo, ctx)?;
            ascii::print_repo_churn(ctx, &series)
        }
    }
}

#[derive(serde::Serialize)]
struct AuthorsOutput {
    context: StatsContext,
    metric: AuthorMetric,
    top: usize,
    authors: Vec<crate::stats::authors::AuthorStats>,
}

pub fn render_authors(repo: &Repository, ctx: &StatsContext, metric: AuthorMetric, top: usize, fmt: OutputFormat) -> Result<()> {
    let mut authors = crate::stats::authors::compute_authors(repo, ctx)?;
    match metric {
        AuthorMetric::Commits => authors.sort_by(|a,b| b.commits.cmp(&a.commits)),
        AuthorMetric::Churn => authors.sort_by(|a,b| (b.adds+b.dels).cmp(&(a.adds+a.dels))),
    }
    let authors = if authors.len() > top { authors.into_iter().take(top).collect() } else { authors };
    match fmt {
        OutputFormat::Json => {
            let out = AuthorsOutput { context: ctx.clone(), metric, top, authors };
            json::print(&out)
        }
        OutputFormat::Table => table::print_authors_table(&authors, metric),
        OutputFormat::Chart => ascii::print_authors_bars(&authors, metric),
    }
}

#[derive(serde::Serialize)]
struct CalendarOutput {
    context: StatsContext,
    kind: &'static str,
    matrix: [[u64;24];7],
}

pub fn render_calendar(repo: &Repository, ctx: &StatsContext, fmt: OutputFormat) -> Result<()> {
    let mat = crate::stats::calendar::compute_calendar(repo, ctx.since.as_deref(), ctx.until.as_deref())?;
    match fmt {
        OutputFormat::Json => {
            let out = CalendarOutput { context: ctx.clone(), kind: "weekday_hour", matrix: mat.matrix };
            json::print(&out)
        }
        OutputFormat::Table => table::print_calendar_table(&mat.matrix),
        OutputFormat::Chart => ascii::print_calendar_heatmap(&mat.matrix),
    }
}

#[derive(serde::Serialize)]
struct HotspotsOutput<'a> {
    context: StatsContext,
    half_life_days: f64,
    include: Option<&'a str>,
    exclude: Option<&'a str>,
    top: usize,
    hotspots: Vec<crate::stats::hotspots::FileHotspot>,
}

pub fn render_hotspots(
    repo: &Repository,
    ctx: &StatsContext,
    include: Option<&str>,
    exclude: Option<&str>,
    half_life_days: f64,
    top: usize,
    fmt: OutputFormat,
) -> Result<()> {
    let mut hs = crate::stats::hotspots::compute_hotspots(repo, ctx, include, exclude, half_life_days)?;
    if hs.len() > top { hs.truncate(top); }
    match fmt {
        OutputFormat::Json => {
            let out = HotspotsOutput { context: ctx.clone(), half_life_days, include, exclude, top, hotspots: hs };
            json::print(&out)
        }
        OutputFormat::Table => table::print_hotspots_table(&hs, half_life_days, include, exclude),
        OutputFormat::Chart => ascii::print_hotspots_bars(&hs),
    }
}

#[derive(serde::Serialize)]
struct BranchesOutput<'a> {
    context: StatsContext,
    base: Option<&'a str>,
    branch_stats: crate::stats::branches::BranchStats,
}

pub fn render_branches(
    repo: &Repository,
    ctx: &StatsContext,
    base: Option<&str>,
    fmt: OutputFormat,
) -> Result<()> {
    let stats = crate::stats::branches::analyze_branches(
        repo,
        base,
        ctx.since.as_deref(),
        ctx.until.as_deref(),
        ctx.no_merges,
    )?;
    match fmt {
        OutputFormat::Json => {
            let out = BranchesOutput { context: ctx.clone(), base, branch_stats: stats };
            json::print(&out)
        }
        OutputFormat::Table => table::print_branches_table(&stats),
        OutputFormat::Chart => ascii::print_branches_bars(&stats),
    }
}

#[derive(serde::Serialize)]
struct CouplingOutput {
    context: StatsContext,
    top: usize,
    min_support: f64,
    window_size: usize,
    coupling_stats: crate::stats::coupling::CouplingStats,
}

pub fn render_coupling(
    repo: &Repository,
    ctx: &StatsContext,
    top: usize,
    min_support: f64,
    window_size: usize,
    fmt: OutputFormat,
) -> Result<()> {
    let stats = crate::stats::coupling::analyze_coupling(
        repo,
        ctx.since.as_deref(),
        ctx.until.as_deref(),
        ctx.no_merges,
        top,
        min_support,
        window_size,
    )?;
    match fmt {
        OutputFormat::Json => {
            let out = CouplingOutput { 
                context: ctx.clone(), 
                top, 
                min_support, 
                window_size, 
                coupling_stats: stats 
            };
            json::print(&out)
        }
        OutputFormat::Table => table::print_coupling_table(&stats, min_support),
        OutputFormat::Chart => ascii::print_coupling_edges(&stats),
    }
}
