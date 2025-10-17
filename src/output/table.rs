use anyhow::Result;
use crate::stats::StatsContext;
use crate::stats::activity::ActivityPoint;
use crate::stats::churn::ChurnPoint;
use crate::stats::authors::AuthorStats;
use crate::stats::AuthorMetric;
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

pub fn print_authors_table(authors: &[AuthorStats], metric: AuthorMetric) -> Result<()> {
    println!("🦀 Top authors by {}", match metric { AuthorMetric::Commits => "commits", AuthorMetric::Churn => "churn"});
    println!("============================\n");
    println!(" author                       commits    adds    dels    total");
    println!(" --------------------------  -------  ------  ------  ------");
    for a in authors.iter() {
        let total = a.adds + a.dels;
        println!(" {:<26}  {:>7}  {:>6}  {:>6}  {:>6}", a.author, a.commits, a.adds, a.dels, total);
    }
    Ok(())
}

pub fn print_calendar_table(matrix: &[[u64;24];7]) -> Result<()> {
    println!("🦀 Weekday x Hour activity (UTC)");
    println!("===============================\n");
    println!("        00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22 23");
    let days = ["Mon","Tue","Wed","Thu","Fri","Sat","Sun"];
    for (r, day) in days.iter().enumerate() {
        print!(" {} ", day);
        for c in 0..24 {
            print!("{:>3}", matrix[r][c]);
        }
        println!();
    }
    Ok(())
}

pub fn print_hotspots_table(hs: &[crate::stats::hotspots::FileHotspot], half_life_days: f64, include: Option<&str>, exclude: Option<&str>) -> Result<()> {
    println!("🦀 Hotspots (half-life: {:.1}d)", half_life_days);
    if let Some(i) = include { println!("filter include: {}", i); }
    if let Some(e) = exclude { println!("filter exclude: {}", e); }
    println!("================================\n");
    println!(" weighted  churn   adds   dels  path");
    println!(" --------  ------  -----  ----- ----------------------------------------");
    for f in hs.iter() {
        println!(" {:>8.1}  {:>6}  {:>5}  {:>5} {}", f.weighted, f.churn, f.adds, f.dels, f.path);
    }
    Ok(())
}

pub fn print_branches_table(stats: &crate::stats::branches::BranchStats) -> Result<()> {
    println!("🦀 Branch metrics (base: {})", stats.base_branch);
    println!("===============================\n");
    println!(" branch                      type    ahead behind commits    adds    dels merge%");
    println!(" -------------------------- ------ ------ ------ ------- ------- ------- ------");
    for branch in &stats.branches {
        let merge_pct = (branch.merge_ratio * 100.0).round() as u32;
        println!(
            " {:<26} {:>6} {:>6} {:>6} {:>7} {:>7} {:>7} {:>5}%",
            branch.name,
            branch.branch_type,
            branch.ahead,
            branch.behind,
            branch.commits,
            branch.churn_adds,
            branch.churn_dels,
            merge_pct
        );
    }
    Ok(())
}
