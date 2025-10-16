use anyhow::Result;
use git2::{Repository, Sort};
use serde::Serialize;
use std::collections::BTreeMap;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime};

use crate::stats::{Bucket, StatsContext};

#[derive(Debug, Serialize, Clone)]
pub struct ActivityPoint {
    pub bucket_start: i64,
    pub commits: u64,
}

pub fn compute_activity(repo: &Repository, ctx: &StatsContext) -> Result<Vec<ActivityPoint>> {
    let (since_ts, until_ts) = (parse_instant(ctx.since.as_deref()), parse_instant(ctx.until.as_deref()));

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let mut buckets: BTreeMap<i64, u64> = BTreeMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let t = commit.time();
        let ts = t.seconds();
        if let Some(since) = since_ts
            && ts < since
        {
            continue;
        }
        if let Some(until) = until_ts
            && ts > until
        {
            continue;
        }
        let key = bucket_start(ts, ctx.bucket);
        *buckets.entry(key).or_insert(0) += 1;
    }

    Ok(buckets
        .into_iter()
        .map(|(k, v)| ActivityPoint { bucket_start: k, commits: v })
        .collect())
}

fn bucket_start(ts: i64, bucket: Bucket) -> i64 {
    match bucket {
        Bucket::Day => ts - (ts % 86_400),
        Bucket::Week => {
            // Anchor weeks to 7-day windows from Unix epoch (not ISO week)
            let day = ts / 86_400;
            let week_day0 = day - (day % 7);
            week_day0 * 86_400
        }
        Bucket::Month => month_floor(ts),
    }
}

fn month_floor(ts: i64) -> i64 {
    let dt = OffsetDateTime::from_unix_timestamp(ts).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let date = dt.date();
    let y = date.year();
    let m: Month = date.month();
    let first = Date::from_calendar_date(y, m, 1).unwrap();
    let pdt = PrimitiveDateTime::new(first, time::Time::MIDNIGHT).assume_utc();
    pdt.unix_timestamp()
}

fn parse_instant(s: Option<&str>) -> Option<i64> {
    let s = s?;
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if let Some(num) = s.strip_suffix('d')
        && let Ok(n) = num.parse::<i64>()
    { return Some(now - n * 86_400); }

    if let Some(num) = s.strip_suffix('w')
        && let Ok(n) = num.parse::<i64>()
    { return Some(now - n * 7 * 86_400); }

    if let Some(num) = s.strip_suffix('m')
        && let Ok(n) = num.parse::<i64>()
    { return Some(now - n * 30 * 86_400); }

    if let Some(num) = s.strip_suffix('y')
        && let Ok(n) = num.parse::<i64>()
    { return Some(now - n * 365 * 86_400); }

    // Try YYYY-MM-DD
    if let Some((y, rest)) = s.split_once('-')
        && let Ok(year) = y.parse::<i32>()
        && let Some((mo, d)) = rest.split_once('-')
        && let (Ok(month_u8), Ok(day_u8)) = (mo.parse::<u8>(), d.parse::<u8>())
        && let Ok(month) = Month::try_from(month_u8)
        && let Ok(date) = Date::from_calendar_date(year, month, day_u8)
    {
        let pdt = PrimitiveDateTime::new(date, time::Time::MIDNIGHT).assume_utc();
        return Some(pdt.unix_timestamp());
    }
    None
}
