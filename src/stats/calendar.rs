use anyhow::Result;
use git2::{Repository, Sort};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct CalendarHeatmap {
    // 7 x 24 matrix, rows: 0=Mon..6=Sun (ISO), cols: 0..23 hours UTC
    pub matrix: [[u64; 24]; 7],
}

pub fn compute_calendar(repo: &Repository, since: Option<&str>, until: Option<&str>) -> Result<CalendarHeatmap> {
    let (since_ts, until_ts) = (
        crate::stats::activity::parse_instant(since),
        crate::stats::activity::parse_instant(until),
    );

    let mut matrix = [[0u64; 24]; 7];

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let ts = commit.time().seconds();
        if let Some(since) = since_ts && ts < since { continue; }
        if let Some(until) = until_ts && ts > until { continue; }

        let dt = time::OffsetDateTime::from_unix_timestamp(ts).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        let iso_weekday = dt.date().weekday().number_from_monday() as usize - 1; // 0..6
        let hour = dt.hour() as usize;
        matrix[iso_weekday][hour] += 1;
    }

    Ok(CalendarHeatmap { matrix })
}
