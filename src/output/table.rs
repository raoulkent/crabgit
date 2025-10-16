use anyhow::Result;
use crate::stats::StatsContext;

pub fn print_repo_placeholder(ctx: &StatsContext) -> Result<()> {
    println!("🦀 Repo stats (placeholder)");
    println!("===========================\n");
    println!("path   : {}", ctx.repo_path);
    println!("since  : {}", ctx.since.as_deref().unwrap_or("-"));
    println!("until  : {}", ctx.until.as_deref().unwrap_or("-"));
    println!("bucket : {:?}", ctx.bucket);
    Ok(())
}
