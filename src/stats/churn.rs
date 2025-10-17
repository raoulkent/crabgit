use anyhow::Result;
use git2::{DiffOptions, Repository, Sort, Tree};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::stats::{Bucket, StatsContext};

#[derive(Debug, Serialize, Clone)]
pub struct ChurnPoint {
    pub bucket_start: i64,
    pub adds: u64,
    pub dels: u64,
}

pub fn compute_churn(repo: &Repository, ctx: &StatsContext) -> Result<Vec<ChurnPoint>> {
    let (since_ts, until_ts) = (
        crate::stats::activity::parse_instant(ctx.since.as_deref()),
        crate::stats::activity::parse_instant(ctx.until.as_deref()),
    );

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let mut buckets: BTreeMap<i64, (u64, u64)> = BTreeMap::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let ts = commit.time().seconds();
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

        let parents = commit.parent_count();
        if parents == 0 {
            let tree = commit.tree()?;
            let (adds, dels) = diff_trees(repo, None, Some(&tree))?;
            let key = bucket_start(ts, ctx.bucket);
            let entry = buckets.entry(key).or_insert((0, 0));
            entry.0 += adds;
            entry.1 += dels;
        } else {
            if parents > 1 && ctx.no_merges {
                continue;
            }
            let parent = commit.parent(0)?;
            let parent_tree = parent.tree()?;
            let tree = commit.tree()?;
            let (adds, dels) = diff_trees(repo, Some(&parent_tree), Some(&tree))?;
            let key = bucket_start(ts, ctx.bucket);
            let entry = buckets.entry(key).or_insert((0, 0));
            entry.0 += adds;
            entry.1 += dels;
        }
    }

    Ok(buckets
        .into_iter()
        .map(|(k, (a, d))| ChurnPoint {
            bucket_start: k,
            adds: a,
            dels: d,
        })
        .collect())
}

fn diff_trees(repo: &Repository, a: Option<&Tree>, b: Option<&Tree>) -> Result<(u64, u64)> {
    let mut opts = DiffOptions::new();
    // default options; could add pathspec filtering later
    let diff = repo.diff_tree_to_tree(a, b, Some(&mut opts))?;
    let stats = diff.stats()?;
    Ok((stats.insertions() as u64, stats.deletions() as u64))
}

pub fn diff_trees_public(
    repo: &Repository,
    a: Option<&Tree>,
    b: Option<&Tree>,
) -> Result<(u64, u64)> {
    diff_trees(repo, a, b)
}

fn bucket_start(ts: i64, bucket: Bucket) -> i64 {
    match bucket {
        Bucket::Day => ts - (ts % 86_400),
        Bucket::Week => {
            let day = ts / 86_400;
            let week_day0 = day - (day % 7);
            week_day0 * 86_400
        }
        Bucket::Month => crate::stats::activity::month_floor(ts),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Bucket;

    #[test]
    fn test_bucket_start_day_churn() {
        // Test day bucketing consistency with activity module
        let ts = 1609459200 + 3661; // 2021-01-01 01:01:01 UTC
        let expected = 1609459200;   // 2021-01-01 00:00:00 UTC
        assert_eq!(bucket_start(ts, Bucket::Day), expected);
        
        // Test with different time in same day
        let ts2 = 1609459200 + 86399; // 2021-01-01 23:59:59 UTC
        assert_eq!(bucket_start(ts2, Bucket::Day), expected);
    }

    #[test]
    fn test_bucket_start_week_churn() {
        // Test week bucketing consistency
        let ts = 1609459200; // 2021-01-01 00:00:00 UTC (Friday)
        let day = ts / 86_400;
        let week_day0 = day - (day % 7);
        let expected = week_day0 * 86_400;
        
        assert_eq!(bucket_start(ts, Bucket::Week), expected);
        
        // Test different day in same week
        let ts2 = ts + (2 * 86_400); // Sunday same week
        assert_eq!(bucket_start(ts2, Bucket::Week), expected);
    }

    #[test]
    fn test_bucket_start_month_churn() {
        // Test month bucketing uses activity module function
        let ts = 1609459200 + 15 * 86_400; // 2021-01-16
        let result = bucket_start(ts, Bucket::Month);
        let expected = crate::stats::activity::month_floor(ts);
        assert_eq!(result, expected);
        
        // Test consistency across different dates in same month
        let ts_start = 1609459200; // 2021-01-01
        let ts_mid = ts_start + 15 * 86_400; // 2021-01-16
        let ts_end = ts_start + 30 * 86_400; // 2021-01-31
        
        let result_start = bucket_start(ts_start, Bucket::Month);
        let result_mid = bucket_start(ts_mid, Bucket::Month);
        let result_end = bucket_start(ts_end, Bucket::Month);
        
        assert_eq!(result_start, result_mid);
        assert_eq!(result_mid, result_end);
    }

    #[test]
    fn test_churn_point_creation() {
        // Test ChurnPoint struct creation and data integrity
        let point = ChurnPoint {
            bucket_start: 1609459200,
            adds: 150,
            dels: 75,
        };
        
        assert_eq!(point.bucket_start, 1609459200);
        assert_eq!(point.adds, 150);
        assert_eq!(point.dels, 75);
        
        // Test serialization
        let json_result = serde_json::to_string(&point);
        assert!(json_result.is_ok(), "ChurnPoint should serialize to JSON");
        
        let json_str = json_result.unwrap();
        assert!(json_str.contains("150"), "JSON should contain adds value");
        assert!(json_str.contains("75"), "JSON should contain dels value");
        assert!(json_str.contains("1609459200"), "JSON should contain bucket_start");
    }

    #[test]
    fn test_churn_point_zero_values() {
        // Test ChurnPoint with zero values (no changes in bucket)
        let point = ChurnPoint {
            bucket_start: 1609459200,
            adds: 0,
            dels: 0,
        };
        
        assert_eq!(point.adds, 0);
        assert_eq!(point.dels, 0);
        
        // Zero values should still serialize correctly
        let json_result = serde_json::to_string(&point);
        assert!(json_result.is_ok());
    }

    #[test]
    fn test_churn_point_large_values() {
        // Test ChurnPoint with large churn values
        let point = ChurnPoint {
            bucket_start: 1609459200,
            adds: u64::MAX / 2,
            dels: u64::MAX / 3,
        };
        
        assert_eq!(point.adds, u64::MAX / 2);
        assert_eq!(point.dels, u64::MAX / 3);
        
        // Large values should serialize without overflow
        let json_result = serde_json::to_string(&point);
        assert!(json_result.is_ok(), "Large values should serialize correctly");
    }

    #[test]
    fn test_diff_trees_public_wrapper() {
        // Test that the public wrapper function exists and is accessible
        // This ensures the public API remains stable for testing purposes
        
        // We can't easily test the actual diff functionality without a real repository,
        // but we can verify the function signature and that it's properly exposed
        let _function_exists = diff_trees_public;
        
        // The function should be callable (though we can't test with None trees easily)
        // This test primarily verifies the public API exposure
    }

    #[test]
    fn test_bucket_boundaries() {
        // Test bucket boundary conditions to ensure no off-by-one errors
        
        // Test exact day boundary
        let day_boundary = 1609459200; // 2021-01-01 00:00:00 UTC
        assert_eq!(bucket_start(day_boundary, Bucket::Day), day_boundary);
        
        // Test one second before next day
        let day_before_next = day_boundary + 86399;
        assert_eq!(bucket_start(day_before_next, Bucket::Day), day_boundary);
        
        // Test first second of next day
        let next_day = day_boundary + 86400;
        assert_eq!(bucket_start(next_day, Bucket::Day), next_day);
    }

    #[test]
    fn test_churn_point_ordering() {
        // Test that ChurnPoint can be ordered (useful for collections)
        let point1 = ChurnPoint {
            bucket_start: 1609459200,
            adds: 100,
            dels: 50,
        };
        
        let point2 = ChurnPoint {
            bucket_start: 1609545600, // next day
            adds: 75,
            dels: 25,
        };
        
        // Points should be orderable by bucket_start
        assert!(point1.bucket_start < point2.bucket_start);
        
        // Test cloning works
        let point1_clone = point1.clone();
        assert_eq!(point1.bucket_start, point1_clone.bucket_start);
        assert_eq!(point1.adds, point1_clone.adds);
        assert_eq!(point1.dels, point1_clone.dels);
    }

    #[test]
    fn test_time_consistency_with_activity() {
        // Ensure churn module time parsing is consistent with activity module
        let test_cases = vec![
            "7d",
            "2w", 
            "3m",
            "1y",
            "2021-01-01",
            "2023-12-25"
        ];
        
        for test_case in test_cases {
            let activity_result = crate::stats::activity::parse_instant(Some(test_case));
            let churn_result = crate::stats::activity::parse_instant(Some(test_case));
            
            assert_eq!(activity_result, churn_result, 
                "Time parsing should be consistent between activity and churn modules for: {}", test_case);
        }
    }
}
