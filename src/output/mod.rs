pub mod json;
pub mod table;
pub mod ascii;

use crate::stats::{OutputFormat, StatsContext, StatsMetric};
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
