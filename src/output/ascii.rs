use anyhow::Result;
use crate::stats::StatsContext;
use crate::stats::activity::ActivityPoint;
use crate::stats::churn::ChurnPoint;
use crate::stats::authors::AuthorStats;
use crate::stats::AuthorMetric;

fn sparkline(values: impl Iterator<Item = u64>) -> String {
    let blocks: [char; 8] = ['▁','▂','▃','▄','▅','▆','▇','█'];
    let collected: Vec<u64> = values.collect();
    let max = collected.iter().copied().max().unwrap_or(1);
    collected
        .into_iter()
        .map(|v| {
            let level = if max == 0 { 0 } else { ((v as f64 / max as f64) * 7.0).round() as usize };
            blocks[level.min(7)]
        })
        .collect()
}

pub fn print_repo_activity(_ctx: &StatsContext, series: &[ActivityPoint]) -> Result<()> {
    if series.is_empty() { println!("(no data)"); return Ok(()); }
    let line = sparkline(series.iter().map(|p| p.commits));
    println!("{}", line);
    Ok(())
}

pub fn print_repo_churn(_ctx: &StatsContext, series: &[ChurnPoint]) -> Result<()> {
    if series.is_empty() { println!("(no data)"); return Ok(()); }
    let adds = sparkline(series.iter().map(|p| p.adds));
    let dels = sparkline(series.iter().map(|p| p.dels));
    println!("A:{}", adds);
    println!("D:{}", dels);
    Ok(())
}

pub fn print_authors_bars(authors: &[AuthorStats], metric: AuthorMetric) -> Result<()> {
    if authors.is_empty() { println!("(no data)"); return Ok(()); }
    let values: Vec<u64> = match metric {
        AuthorMetric::Commits => authors.iter().map(|a| a.commits).collect(),
        AuthorMetric::Churn => authors.iter().map(|a| a.adds + a.dels).collect(),
    };
    let max = *values.iter().max().unwrap_or(&1);
    for (a, v) in authors.iter().zip(values.iter()) {
        let width = if max == 0 { 0 } else { ((*v as f64 / max as f64) * 30.0).round() as usize };
        let bar = "█".repeat(width);
        println!("{:>6} | {:<30} {}", v, a.author, bar);
    }
    Ok(())
}

pub fn print_calendar_heatmap(matrix: &[[u64;24];7]) -> Result<()> {
    // Render 7x24 heatmap using 5 levels
    let shades: [char; 5] = [' ', '░', '▒', '▓', '█'];
    let mut max_val = 0u64;
    for row in matrix.iter().take(7) {
        for v in row.iter().take(24) { max_val = max_val.max(*v); }
    }
    let days = ["Mon","Tue","Wed","Thu","Fri","Sat","Sun"];
    println!("    00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22 23");
    for r in 0..7 {
        print!(" {} ", days[r]);
        for c in 0..24 {
            let v = matrix[r][c];
            let idx = if max_val == 0 { 0 } else { ((v as f64 / max_val as f64) * 4.0).round() as usize };
            print!(" {}", shades[idx.min(4)]);
        }
        println!();
    }
    Ok(())
}

pub fn print_hotspots_bars(hs: &[crate::stats::hotspots::FileHotspot]) -> Result<()> {
    if hs.is_empty() { println!("(no data)"); return Ok(()); }
    let max = hs.iter().map(|h| h.weighted).fold(0.0, f64::max);
    for h in hs.iter() {
        let ratio = if max == 0.0 { 0.0 } else { h.weighted / max };
        let width = (ratio * 30.0).round() as usize;
        let bar = "█".repeat(width);
        println!("{:>8.1} | {:<30} {}", h.weighted, truncate_path(&h.path, 30), bar);
    }
    Ok(())
}

fn truncate_path(s: &str, max: usize) -> String {
    if s.len() <= max { return s.to_string(); }
    let mut out = String::with_capacity(max);
    out.push('…');
    out.push_str(&s[s.len().saturating_sub(max-1)..]);
    out
}
