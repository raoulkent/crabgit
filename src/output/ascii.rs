use anyhow::Result;
use crate::stats::StatsContext;
use crate::stats::activity::ActivityPoint;

pub fn print_repo_activity(_ctx: &StatsContext, series: &[ActivityPoint]) -> Result<()> {
    // Simple sparkline using Unicode block characters
    let blocks: [char; 8] = ['▁','▂','▃','▄','▅','▆','▇','█'];
    if series.is_empty() {
        println!("(no data)");
        return Ok(());
    }
    let max = series.iter().map(|p| p.commits).max().unwrap_or(1);
    let line: String = series.iter().map(|p| {
        let level = if max == 0 { 0 } else { ((p.commits as f64 / max as f64) * 7.0).round() as usize };
        blocks[level.min(7)]
    }).collect();
    println!("{}", line);
    Ok(())
}
