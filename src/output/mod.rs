pub mod json;
pub mod table;
pub mod ascii;

use crate::stats::{OutputFormat, StatsContext};
use anyhow::Result;
use git2::Repository;

#[derive(serde::Serialize)]
struct RepoActivityOutput {
    context: StatsContext,
    metric: &'static str,
    bucket: crate::stats::Bucket,
    series: Vec<crate::stats::activity::ActivityPoint>,
}

pub fn render_repo_placeholder(repo: &Repository, ctx: &StatsContext, fmt: OutputFormat) -> Result<()> {
    let series = crate::stats::activity::compute_activity(repo, ctx)?;
    match fmt {
        OutputFormat::Json => {
            let out = RepoActivityOutput {
                context: ctx.clone(),
                metric: "activity",
                bucket: ctx.bucket,
                series,
            };
            json::print(&out)
        }
        OutputFormat::Table => table::print_repo_activity(ctx, &series),
        OutputFormat::Chart => ascii::print_repo_activity(ctx, &series),
    }
}
