use anyhow::Result;
use crate::stats::StatsContext;
use crate::stats::activity::ActivityPoint;
use crate::stats::churn::ChurnPoint;

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
