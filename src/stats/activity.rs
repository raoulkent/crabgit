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
    let (since_ts, until_ts) = (
        parse_instant(ctx.since.as_deref()),
        parse_instant(ctx.until.as_deref()),
    );

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
        .map(|(k, v)| ActivityPoint {
            bucket_start: k,
            commits: v,
        })
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

pub fn month_floor(ts: i64) -> i64 {
    let dt = OffsetDateTime::from_unix_timestamp(ts).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let date = dt.date();
    let y = date.year();
    let m: Month = date.month();
    let first = Date::from_calendar_date(y, m, 1).unwrap();
    let pdt = PrimitiveDateTime::new(first, time::Time::MIDNIGHT).assume_utc();
    pdt.unix_timestamp()
}

pub fn parse_instant(s: Option<&str>) -> Option<i64> {
    let s = s?;
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if let Some(num) = s.strip_suffix('d')
        && let Ok(n) = num.parse::<i64>()
    {
        return Some(now - n * 86_400);
    }

    if let Some(num) = s.strip_suffix('w')
        && let Ok(n) = num.parse::<i64>()
    {
        return Some(now - n * 7 * 86_400);
    }

    if let Some(num) = s.strip_suffix('m')
        && let Ok(n) = num.parse::<i64>()
    {
        return Some(now - n * 30 * 86_400);
    }

    if let Some(num) = s.strip_suffix('y')
        && let Ok(n) = num.parse::<i64>()
    {
        return Some(now - n * 365 * 86_400);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use time::{Date, Month};

    #[test]
    fn test_bucket_start_day() {
        // Test day bucketing - should round down to start of day
        let ts = 1609459200 + 3661; // 2021-01-01 01:01:01 UTC
        let expected = 1609459200;   // 2021-01-01 00:00:00 UTC
        assert_eq!(bucket_start(ts, Bucket::Day), expected);
        
        // Test with different time in same day
        let ts2 = 1609459200 + 86399; // 2021-01-01 23:59:59 UTC
        assert_eq!(bucket_start(ts2, Bucket::Day), expected);
    }

    #[test]
    fn test_bucket_start_week() {
        // Test week bucketing - should round down to start of week
        let ts = 1609459200; // 2021-01-01 00:00:00 UTC (Friday)
        let day = ts / 86_400;        // days since epoch
        let week_day0 = day - (day % 7);
        let expected = week_day0 * 86_400;
        
        assert_eq!(bucket_start(ts, Bucket::Week), expected);
        
        // Test different day in same week
        let ts2 = ts + (2 * 86_400); // Sunday same week
        assert_eq!(bucket_start(ts2, Bucket::Week), expected);
    }

    #[test] 
    fn test_bucket_start_month() {
        // Test month bucketing
        let ts = 1609459200 + 15 * 86_400; // 2021-01-16
        let result = bucket_start(ts, Bucket::Month);
        
        // Should be start of January 2021
        let expected_date = Date::from_calendar_date(2021, Month::January, 1).unwrap();
        let expected = time::PrimitiveDateTime::new(expected_date, time::Time::MIDNIGHT)
            .assume_utc()
            .unix_timestamp();
            
        assert_eq!(result, expected);
    }

    #[test]
    fn test_month_floor() {
        // Test month flooring function
        let ts = 1609459200 + 15 * 86_400; // 2021-01-16
        let result = month_floor(ts);
        
        // Should return start of January 2021
        let expected_date = Date::from_calendar_date(2021, Month::January, 1).unwrap();
        let expected = time::PrimitiveDateTime::new(expected_date, time::Time::MIDNIGHT)
            .assume_utc()
            .unix_timestamp();
            
        assert_eq!(result, expected);
        
        // Test different month
        let feb_ts = 1612137600; // 2021-02-01
        let feb_result = month_floor(feb_ts + 10 * 86_400); // 2021-02-11
        let feb_expected = Date::from_calendar_date(2021, Month::February, 1).unwrap();
        let feb_expected_ts = time::PrimitiveDateTime::new(feb_expected, time::Time::MIDNIGHT)
            .assume_utc()
            .unix_timestamp();
            
        assert_eq!(feb_result, feb_expected_ts);
    }

    #[test]
    fn test_parse_instant_relative_formats() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        
        // Test days
        let result = parse_instant(Some("7d")).unwrap();
        let expected = now - (7 * 86_400);
        assert!((result - expected).abs() <= 1, "7d parsing should be accurate");
        
        // Test weeks
        let result = parse_instant(Some("2w")).unwrap();
        let expected = now - (2 * 7 * 86_400);
        assert!((result - expected).abs() <= 1, "2w parsing should be accurate");
        
        // Test months
        let result = parse_instant(Some("3m")).unwrap();
        let expected = now - (3 * 30 * 86_400);
        assert!((result - expected).abs() <= 1, "3m parsing should be accurate");
        
        // Test years
        let result = parse_instant(Some("1y")).unwrap();
        let expected = now - (365 * 86_400);
        assert!((result - expected).abs() <= 1, "1y parsing should be accurate");
        
        // Test zero values
        let result = parse_instant(Some("0d")).unwrap();
        assert!((result - now).abs() <= 1, "0d should be approximately now");
    }

    #[test] 
    fn test_parse_instant_absolute_dates() {
        // Test ISO date parsing
        let result = parse_instant(Some("2021-01-01")).unwrap();
        let expected = Date::from_calendar_date(2021, Month::January, 1).unwrap();
        let expected_ts = time::PrimitiveDateTime::new(expected, time::Time::MIDNIGHT)
            .assume_utc()
            .unix_timestamp();
        assert_eq!(result, expected_ts);
        
        // Test different date
        let result = parse_instant(Some("2023-12-25")).unwrap();
        let expected = Date::from_calendar_date(2023, Month::December, 25).unwrap();
        let expected_ts = time::PrimitiveDateTime::new(expected, time::Time::MIDNIGHT)
            .assume_utc()
            .unix_timestamp();
        assert_eq!(result, expected_ts);
        
        // Test leap year date
        let result = parse_instant(Some("2020-02-29")).unwrap();
        let expected = Date::from_calendar_date(2020, Month::February, 29).unwrap();
        let expected_ts = time::PrimitiveDateTime::new(expected, time::Time::MIDNIGHT)
            .assume_utc()
            .unix_timestamp();
        assert_eq!(result, expected_ts);
    }

    #[test]
    fn test_parse_instant_edge_cases() {
        // Test None input
        assert_eq!(parse_instant(None), None);
        
        // Test empty string
        assert_eq!(parse_instant(Some("")), None);
        
        // Test invalid formats
        assert_eq!(parse_instant(Some("invalid")), None);
        assert_eq!(parse_instant(Some("30x")), None);
        assert_eq!(parse_instant(Some("2021-13-01")), None); // Invalid month
        assert_eq!(parse_instant(Some("2021-02-30")), None); // Invalid day
        assert_eq!(parse_instant(Some("abc-def-ghi")), None); // Non-numeric
        
        // Test malformed relative dates
        assert_eq!(parse_instant(Some("d")), None);   // Missing number
        assert_eq!(parse_instant(Some("10")), None);  // Missing unit
        assert!(parse_instant(Some("-5d")).is_some()); // Negative values parse (future dates)
    }

    #[test]
    fn test_parse_instant_boundary_values() {
        // Test large values
        let result = parse_instant(Some("999d"));
        assert!(result.is_some(), "Should handle large day values");
        
        let result = parse_instant(Some("52w"));
        assert!(result.is_some(), "Should handle large week values");
        
        let result = parse_instant(Some("12m"));
        assert!(result.is_some(), "Should handle large month values");
        
        let result = parse_instant(Some("100y"));
        assert!(result.is_some(), "Should handle large year values");
    }

    #[test]
    fn test_activity_point_creation() {
        // Test ActivityPoint struct creation
        let point = ActivityPoint {
            bucket_start: 1609459200,
            commits: 42,
        };
        
        assert_eq!(point.bucket_start, 1609459200);
        assert_eq!(point.commits, 42);
        
        // Test serialization works (implicit through serde)
        let json_result = serde_json::to_string(&point);
        assert!(json_result.is_ok(), "ActivityPoint should serialize to JSON");
    }
}
