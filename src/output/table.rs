use anyhow::Result;
use crate::stats::StatsContext;
use crate::stats::activity::ActivityPoint;
use crate::stats::churn::ChurnPoint;

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

pub fn print_repo_churn(ctx: &StatsContext, series: &[ChurnPoint]) -> Result<()> {
    println!("🦀 Repo churn (adds/dels per {:?})", ctx.bucket);
    println!("===================================\n");
    println!("path   : {}", ctx.repo_path);
    println!("since  : {}", ctx.since.as_deref().unwrap_or("-"));
    println!("until  : {}", ctx.until.as_deref().unwrap_or("-"));
    println!("\n bucket_start        adds     dels");
    println!(" ------------------  -------  -------");
    for p in series.iter().take(20) {
        println!(" {:<18}  {:>7}  {:>7}", p.bucket_start, p.adds, p.dels);
    }
    if series.len() > 20 {
        println!(" ... ({} more)", series.len() - 20);
    }
    Ok(())
}
