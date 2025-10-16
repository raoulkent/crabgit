use anyhow::Result;
use crate::stats::StatsContext;
use crate::stats::activity::ActivityPoint;

pub fn print_repo_activity(ctx: &StatsContext, series: &[ActivityPoint]) -> Result<()> {
    println!("🦀 Repo activity (commits per {:?})", ctx.bucket);
    println!("=====================================\n");
    println!("path   : {}", ctx.repo_path);
    println!("since  : {}", ctx.since.as_deref().unwrap_or("-"));
    println!("until  : {}", ctx.until.as_deref().unwrap_or("-"));
    println!("\n bucket_start        commits");
    println!(" ------------------  -------");
    for p in series.iter().take(20) {
        println!(" {:<18}  {:>7}", p.bucket_start, p.commits);
    }
    if series.len() > 20 {
        println!(" ... ({} more)", series.len() - 20);
    }
    Ok(())
}
