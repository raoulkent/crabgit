use anyhow::Result;
use crate::stats::StatsContext;
use crate::stats::activity::ActivityPoint;
use crate::stats::churn::ChurnPoint;
use time::OffsetDateTime;

fn fmt_bucket(ts: i64) -> String {
    let dt = OffsetDateTime::from_unix_timestamp(ts).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    dt.date().to_string() // ISO 8601 YYYY-MM-DD
}

pub fn print_repo_activity(ctx: &StatsContext, series: &[ActivityPoint]) -> Result<()> {
    println!("🦀 Repo activity (commits per {:?})", ctx.bucket);
    println!("=====================================\n");
    println!("path   : {}", ctx.repo_path);
    println!("since  : {}", ctx.since.as_deref().unwrap_or("-"));
    println!("until  : {}", ctx.until.as_deref().unwrap_or("-"));
    println!("\n bucket            commits");
    println!(" ----------------  -------");
    for p in series.iter().take(20) {
        println!(" {:<16}  {:>7}", fmt_bucket(p.bucket_start), p.commits);
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
    println!("\n bucket            adds     dels");
    println!(" ----------------  -------  -------");
    for p in series.iter().take(20) {
        println!(" {:<16}  {:>7}  {:>7}", fmt_bucket(p.bucket_start), p.adds, p.dels);
    }
    if series.len() > 20 {
        println!(" ... ({} more)", series.len() - 20);
    }
    Ok(())
}
