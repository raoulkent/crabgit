use anyhow::Result;
use crate::stats::StatsContext;

pub fn print_repo_placeholder(ctx: &StatsContext) -> Result<()> {
    println!("(chart) repo stats placeholder");
    println!("path: {}", ctx.repo_path);
    Ok(())
}
