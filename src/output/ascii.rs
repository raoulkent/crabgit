use crate::stats::AuthorMetric;
use crate::stats::StatsContext;
use crate::stats::activity::ActivityPoint;
use crate::stats::authors::AuthorStats;
use crate::stats::churn::ChurnPoint;
use anyhow::Result;

fn sparkline(values: impl Iterator<Item = u64>) -> String {
    let blocks: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let collected: Vec<u64> = values.collect();
    let max = collected.iter().copied().max().unwrap_or(1);
    collected
        .into_iter()
        .map(|v| {
            let level = if max == 0 {
                0
            } else {
                ((v as f64 / max as f64) * 7.0).round() as usize
            };
            blocks[level.min(7)]
        })
        .collect()
}

pub fn print_repo_activity(_ctx: &StatsContext, series: &[ActivityPoint]) -> Result<()> {
    if series.is_empty() {
        println!("(no data)");
        return Ok(());
    }
    let line = sparkline(series.iter().map(|p| p.commits));
    println!("{}", line);
    Ok(())
}

pub fn print_repo_churn(_ctx: &StatsContext, series: &[ChurnPoint]) -> Result<()> {
    if series.is_empty() {
        println!("(no data)");
        return Ok(());
    }
    let adds = sparkline(series.iter().map(|p| p.adds));
    let dels = sparkline(series.iter().map(|p| p.dels));
    println!("A:{}", adds);
    println!("D:{}", dels);
    Ok(())
}

pub fn print_authors_bars(authors: &[AuthorStats], metric: AuthorMetric) -> Result<()> {
    if authors.is_empty() {
        println!("(no data)");
        return Ok(());
    }
    let values: Vec<u64> = match metric {
        AuthorMetric::Commits => authors.iter().map(|a| a.commits).collect(),
        AuthorMetric::Churn => authors.iter().map(|a| a.adds + a.dels).collect(),
    };
    let max = *values.iter().max().unwrap_or(&1);
    for (a, v) in authors.iter().zip(values.iter()) {
        let width = if max == 0 {
            0
        } else {
            ((*v as f64 / max as f64) * 30.0).round() as usize
        };
        let bar = "█".repeat(width);
        println!("{:>6} | {:<30} {}", v, a.author, bar);
    }
    Ok(())
}

pub fn print_calendar_heatmap(matrix: &[[u64; 24]; 7]) -> Result<()> {
    // Render 7x24 heatmap using 5 levels
    let shades: [char; 5] = [' ', '░', '▒', '▓', '█'];
    let mut max_val = 0u64;
    for row in matrix.iter().take(7) {
        for v in row.iter().take(24) {
            max_val = max_val.max(*v);
        }
    }
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    println!("    00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22 23");
    for r in 0..7 {
        print!(" {} ", days[r]);
        for c in 0..24 {
            let v = matrix[r][c];
            let idx = if max_val == 0 {
                0
            } else {
                ((v as f64 / max_val as f64) * 4.0).round() as usize
            };
            print!(" {}", shades[idx.min(4)]);
        }
        println!();
    }
    Ok(())
}

pub fn print_hotspots_bars(hs: &[crate::stats::hotspots::FileHotspot]) -> Result<()> {
    if hs.is_empty() {
        println!("(no data)");
        return Ok(());
    }
    let max = hs.iter().map(|h| h.weighted).fold(0.0, f64::max);
    for h in hs.iter() {
        let ratio = if max == 0.0 { 0.0 } else { h.weighted / max };
        let width = (ratio * 30.0).round() as usize;
        let bar = "█".repeat(width);
        println!(
            "{:>8.1} | {:<30} {}",
            h.weighted,
            truncate_path(&h.path, 30),
            bar
        );
    }
    Ok(())
}

pub fn print_branches_bars(stats: &crate::stats::branches::BranchStats) -> Result<()> {
    if stats.branches.is_empty() {
        println!("(no data)");
        return Ok(());
    }

    println!("🦀 Branch activity (base: {})", stats.base_branch);
    println!("==============================\n");

    let max_commits = stats.branches.iter().map(|b| b.commits).max().unwrap_or(1);

    for branch in &stats.branches {
        let ratio = branch.commits as f64 / max_commits as f64;
        let width = (ratio * 30.0).round() as usize;
        let bar = "█".repeat(width);

        let ahead_behind = if branch.ahead > 0 || branch.behind > 0 {
            format!(" (+{} -{}) ", branch.ahead, branch.behind)
        } else {
            String::from(" ")
        };

        println!(
            "{:>6} | {:<20}{} {}",
            branch.commits,
            truncate_branch_name(&branch.name, 20),
            ahead_behind,
            bar
        );
    }
    Ok(())
}

fn truncate_path(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut out = String::with_capacity(max);
    out.push('…');
    out.push_str(&s[s.len().saturating_sub(max - 1)..]);
    out
}

fn truncate_branch_name(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut out = String::with_capacity(max);
    out.push_str(&s[..max.saturating_sub(1)]);
    out.push('…');
    out
}

pub fn print_coupling_edges(stats: &crate::stats::coupling::CouplingStats) -> Result<()> {
    if stats.pairs.is_empty() {
        println!("(no coupling found)");
        return Ok(());
    }

    println!("🦀 File coupling edges (sorted by lift)");
    println!("========================================\n");

    let max_lift = stats.pairs.iter().map(|p| p.lift).fold(0.0, f64::max);

    for pair in &stats.pairs {
        let lift_ratio = if max_lift > 0.0 {
            pair.lift / max_lift
        } else {
            0.0
        };
        let width = (lift_ratio * 30.0).round() as usize;
        let bar = "█".repeat(width);

        println!(
            "{:>6.2} | {:<15} → {:<15} {}",
            pair.lift,
            truncate_file_path(&pair.file_a, 15),
            truncate_file_path(&pair.file_b, 15),
            bar
        );
    }

    println!("\nMetrics: lift = confidence/support, higher = stronger association");
    Ok(())
}

fn truncate_file_path(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    format!("…{}", &s[s.len().saturating_sub(max - 1)..])
}

pub fn print_ownership_bars(stats: &crate::stats::ownership::OwnershipStats) -> Result<()> {
    let mode_name = match stats.analysis_mode {
        crate::stats::ownership::OwnershipMode::LastModified => "fast",
        crate::stats::ownership::OwnershipMode::BlameAnalysis => "blame",
    };

    println!("🦀 Code ownership ({} analysis)", mode_name);
    println!("==================================\n");

    // Bus factor visualization
    let bus_factor_width = ((stats.bus_factor.score / 4.0) * 20.0).round() as usize;
    let risk_bar = match stats.bus_factor.score as u32 {
        1 => "█".repeat(bus_factor_width), // Red equivalent
        2 => "░".repeat(bus_factor_width), // Orange equivalent
        3 => "▒".repeat(bus_factor_width), // Yellow equivalent
        _ => "▓".repeat(bus_factor_width), // Green equivalent
    };

    println!("Bus Factor: {:.1} {}", stats.bus_factor.score, risk_bar);
    println!("{}", stats.bus_factor.description);
    println!();

    // Overall ownership bars
    if !stats.overall_ownership.is_empty() {
        println!("Overall Ownership:");
        let max_percentage = stats
            .overall_ownership
            .iter()
            .map(|a| a.ownership_percentage)
            .fold(0.0, f64::max);

        for author in stats.overall_ownership.iter().take(8) {
            let ratio = if max_percentage > 0.0 {
                author.ownership_percentage / max_percentage
            } else {
                0.0
            };
            let width = (ratio * 25.0).round() as usize;
            let bar = "█".repeat(width);

            println!(
                "{:>5.1}% | {:<20} {}",
                author.ownership_percentage,
                truncate_author(&author.author, 20),
                bar
            );
        }
        println!();
    }

    // Top files ownership
    if !stats.file_ownership.is_empty() {
        println!("File Ownership (top 10):");
        for (i, file) in stats.file_ownership.iter().take(10).enumerate() {
            let width = (file.ownership_percentage / 100.0 * 20.0).round() as usize;
            let bar = "█".repeat(width);

            println!(
                "{:>2}. {:>3.0}% | {:<15} {}",
                i + 1,
                file.ownership_percentage,
                truncate_file_path(&file.path, 15),
                bar
            );
        }
    }

    println!(
        "\nAnalyzed {} of {} files",
        stats.files_analyzed, stats.total_files_in_repo
    );
    Ok(())
}

fn truncate_author(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    format!("{}…", &s[..max.saturating_sub(1)])
}

pub fn print_stability_bars(stats: &crate::stats::stability::StabilityStats) -> Result<()> {
    println!("🦀 Change stability analysis");
    println!("============================\n");

    if stats.filters.author.is_some() || stats.filters.directory.is_some() {
        if let Some(author) = &stats.filters.author {
            println!("Author filter: {}", author);
        }
        if let Some(dir) = &stats.filters.directory {
            println!("Directory filter: {}", dir);
        }
        println!();
    }

    if stats.files.is_empty() {
        println!("(no files found)");
        return Ok(());
    }

    // Sort by stability score in descending order for display (worst first)
    let mut sorted_files = stats.files.clone();
    sorted_files.sort_by(|a, b| {
        b.stability_score
            .partial_cmp(&a.stability_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max_score = sorted_files
        .iter()
        .map(|f| f.stability_score)
        .fold(0.0, f64::max);

    println!("Stability Score (higher = less stable):");
    for (i, file) in sorted_files.iter().take(15).enumerate() {
        let ratio = if max_score > 0.0 {
            file.stability_score / max_score
        } else {
            0.0
        };
        let width = (ratio * 25.0).round() as usize;

        // Use different characters to indicate severity
        let bar = if ratio > 0.8 {
            "█".repeat(width) // █ for high instability
        } else if ratio > 0.5 {
            "▓".repeat(width) // ▓ for medium instability  
        } else {
            "░".repeat(width) // ░ for low instability
        };

        let change_info = if file.reverts > 0 || file.fixes > 0 {
            format!(" ({}r {}f)", file.reverts, file.fixes)
        } else {
            String::new()
        };

        println!(
            "{:>2}. {:>6.1} | {:<20}{} {}",
            i + 1,
            file.stability_score,
            truncate_file_path(&file.path, 20),
            change_info,
            bar
        );
    }

    if sorted_files.len() > 15 {
        println!("    ... ({} more files)", sorted_files.len() - 15);
    }

    println!("\nLegend: r=reverts, f=fixes. Higher scores = less stable files.");
    Ok(())
}

pub fn print_releases_bars(stats: &crate::stats::releases::ReleaseStats) -> anyhow::Result<()> {
    println!("🦀 Releases and Tags Analysis");
    println!("=============================\n");

    if stats.total_releases > 0 {
        println!(
            "Found {} releases over {} days",
            stats.total_releases, stats.date_range_days
        );
        if let Some(ref active) = stats.summary.most_active_release {
            println!("Most active: {}", active);
        }
        println!();
    }

    if stats.releases.is_empty() {
        println!("(no releases found)");
        return Ok(());
    }

    // Create bars showing commits per release
    println!("Commits per Release (since previous):");
    let max_commits = stats
        .releases
        .iter()
        .map(|r| r.commits_since_previous)
        .max()
        .unwrap_or(1);

    for release in &stats.releases {
        let ratio = release.commits_since_previous as f64 / max_commits as f64;
        let width = (ratio * 30.0).round() as usize;
        let bar = "█".repeat(width);

        let tag_type_symbol = match release.tag_type {
            crate::stats::releases::TagType::Annotated => "A",
            crate::stats::releases::TagType::Lightweight => "L",
        };

        println!(
            "{:>6} | {:<20} {} {} ({})",
            release.commits_since_previous,
            truncate_release_name(&release.name, 20),
            tag_type_symbol,
            bar,
            if release.days_since_previous > 0 {
                format!("{}d", release.days_since_previous)
            } else {
                "-".to_string()
            }
        );
    }

    // Create bars showing churn per release
    println!("\nChurn per Release (lines changed):");
    let max_churn = stats
        .releases
        .iter()
        .map(|r| r.churn_total)
        .max()
        .unwrap_or(1);

    for release in &stats.releases {
        let ratio = release.churn_total as f64 / max_churn as f64;
        let width = (ratio * 30.0).round() as usize;
        let bar = "█".repeat(width);

        println!(
            "{:>6} | {:<20} {}",
            release.churn_total,
            truncate_release_name(&release.name, 20),
            bar
        );
    }

    println!("\nLegend: A=Annotated tag, L=Lightweight tag, d=days since previous release");

    Ok(())
}

fn truncate_release_name(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    format!("{}…", &s[..max.saturating_sub(1)])
}
