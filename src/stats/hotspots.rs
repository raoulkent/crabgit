use anyhow::Result;
use git2::Repository;
use serde::Serialize;
use std::collections::HashMap;
use time::OffsetDateTime;

use crate::stats::{StatsContext, path_filter};

#[derive(Debug, Serialize, Clone)]
pub struct FileHotspot {
    pub path: String,
    pub adds: u64,
    pub dels: u64,
    pub churn: u64,
    pub weighted: f64,
}

pub fn compute_hotspots(
    repo: &Repository,
    ctx: &StatsContext,
    half_life_days: f64,
) -> Result<Vec<FileHotspot>> {
    let filtered_commits = path_filter::FilteredCommits::new(repo, ctx)?;
    let path_filters = filtered_commits.path_filters().clone();

    let until_ts = crate::stats::activity::parse_instant(ctx.until.as_deref());
    let reference_ts = until_ts.unwrap_or_else(|| OffsetDateTime::now_utc().unix_timestamp());
    let hl = if half_life_days > 0.0 {
        half_life_days
    } else {
        90.0
    };

    let mut by_file: HashMap<String, (u64, u64, f64)> = HashMap::new();

    for commit_result in filtered_commits {
        let commit = commit_result?;
        let ts = commit.time().seconds();

        let age_days = ((reference_ts - ts) as f64) / 86_400.0;
        let weight = 0.5_f64.powf(age_days / hl);

        path_filter::scan_commit_changes(repo, &commit, &path_filters, |change| {
            let entry = by_file
                .entry(change.path.clone())
                .or_insert((0u64, 0u64, 0.0f64));
            entry.0 += change.adds;
            entry.1 += change.dels;
            entry.2 += weight * (change.adds + change.dels) as f64;
            true
        })?;
    }

    let mut hotspots: Vec<FileHotspot> = by_file
        .into_iter()
        .map(|(path, (adds, dels, weighted))| FileHotspot {
            path,
            adds,
            dels,
            churn: adds + dels,
            weighted,
        })
        .collect();

    hotspots.sort_by(|a, b| {
        b.weighted
            .partial_cmp(&a.weighted)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.churn.cmp(&a.churn))
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(hotspots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::path_filter;

    #[test]
    fn test_file_hotspot_creation() {
        let hotspot = FileHotspot {
            path: "src/main.rs".to_string(),
            adds: 1500,
            dels: 750,
            churn: 2250,
            weighted: 1800.5,
        };

        assert_eq!(hotspot.path, "src/main.rs");
        assert_eq!(hotspot.adds, 1500);
        assert_eq!(hotspot.dels, 750);
        assert_eq!(hotspot.churn, 2250);
        assert_eq!(hotspot.weighted, 1800.5);
    }

    #[test]
    fn test_file_hotspot_serialization() {
        let hotspot = FileHotspot {
            path: "lib/utils.rs".to_string(),
            adds: 500,
            dels: 200,
            churn: 700,
            weighted: 642.3,
        };

        let json_result = serde_json::to_string(&hotspot);
        assert!(json_result.is_ok(), "FileHotspot should serialize to JSON");

        let json_str = json_result.unwrap();
        assert!(json_str.contains("lib/utils.rs"));
        assert!(json_str.contains("500"));
        assert!(json_str.contains("200"));
        assert!(json_str.contains("700"));
        assert!(json_str.contains("642.3"));
    }

    #[test]
    fn test_file_hotspot_zero_values() {
        let hotspot = FileHotspot {
            path: "README.md".to_string(),
            adds: 0,
            dels: 0,
            churn: 0,
            weighted: 0.0,
        };

        assert_eq!(hotspot.adds, 0);
        assert_eq!(hotspot.dels, 0);
        assert_eq!(hotspot.churn, 0);
        assert_eq!(hotspot.weighted, 0.0);

        let json_result = serde_json::to_string(&hotspot);
        assert!(json_result.is_ok());
    }

    #[test]
    fn test_file_hotspot_large_values() {
        let hotspot = FileHotspot {
            path: "generated/large_file.rs".to_string(),
            adds: u64::MAX / 3,
            dels: u64::MAX / 4,
            churn: u64::MAX / 3 + u64::MAX / 4,
            weighted: f64::MAX / 2.0,
        };

        assert_eq!(hotspot.adds, u64::MAX / 3);
        assert_eq!(hotspot.dels, u64::MAX / 4);
        assert_eq!(hotspot.churn, u64::MAX / 3 + u64::MAX / 4);
        assert_eq!(hotspot.weighted, f64::MAX / 2.0);

        let json_result = serde_json::to_string(&hotspot);
        assert!(json_result.is_ok());
    }

    #[test]
    fn test_compiled_filter_patterns() {
        let patterns = vec!["*.rs", "src/**/*.rs", "lib/**", "test_*.py", "**/*.{js,ts}"];
        for pattern in patterns {
            let result = path_filter::CompiledPathFilters::from_patterns(Some(pattern), None);
            assert!(result.is_ok(), "pattern should compile: {}", pattern);
        }
    }

    #[test]
    fn test_compiled_filter_none() {
        let result = path_filter::CompiledPathFilters::from_patterns(None, None);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_active());
    }

    #[test]
    fn test_filters_include_exclude() {
        let filters = path_filter::CompiledPathFilters::from_patterns(Some("*.rs"), Some("test_*"))
            .expect("filters build");
        assert!(filters.matches("main.rs"));
        assert!(!filters.matches("lib.py"));
        assert!(!filters.matches("test_main.rs"));

        let exclude_only = path_filter::CompiledPathFilters::from_patterns(None, Some("test_*"))
            .expect("filters build");
        assert!(exclude_only.matches("main.rs"));
        assert!(!exclude_only.matches("test_lib.py"));
    }

    #[test]
    fn test_filters_no_rules_match_everything() {
        let filters = path_filter::CompiledPathFilters::from_patterns(None, None).unwrap();
        for path in ["main.rs", "lib.py", "test.js", "README.md", "src/utils.rs"] {
            assert!(filters.matches(path), "{} should match", path);
        }
    }

    #[test]
    fn test_file_hotspot_cloning() {
        let original = FileHotspot {
            path: "src/lib.rs".to_string(),
            adds: 100,
            dels: 50,
            churn: 150,
            weighted: 125.7,
        };

        let cloned = original.clone();
        assert_eq!(original.path, cloned.path);
        assert_eq!(original.adds, cloned.adds);
        assert_eq!(original.dels, cloned.dels);
        assert_eq!(original.churn, cloned.churn);
        assert_eq!(original.weighted, cloned.weighted);
    }

    #[test]
    fn test_file_hotspot_debug_format() {
        let hotspot = FileHotspot {
            path: "debug_test.rs".to_string(),
            adds: 42,
            dels: 21,
            churn: 63,
            weighted: 58.5,
        };

        let debug_str = format!("{:?}", hotspot);
        assert!(debug_str.contains("debug_test.rs"));
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("21"));
        assert!(debug_str.contains("63"));
        assert!(debug_str.contains("58.5"));
    }

    #[test]
    fn test_hotspot_path_formats() {
        let path_cases = vec![
            "main.rs",
            "src/lib.rs",
            "tests/integration/test_main.rs",
            "../parent/file.py",
            "./current/file.js",
            "C:\\Windows\\System32\\file.dll",
            "/usr/local/bin/script.sh",
            "unicode_测试_file.rs",
        ];

        for path in path_cases {
            let hotspot = FileHotspot {
                path: path.to_string(),
                adds: 10,
                dels: 5,
                churn: 15,
                weighted: 12.5,
            };

            assert_eq!(hotspot.path, path);
            let json_result = serde_json::to_string(&hotspot);
            assert!(json_result.is_ok(), "Should serialize path: '{}'", path);
        }
    }

    #[test]
    fn test_recency_weight_calculation() {
        let half_life_days = 90.0;
        let reference_ts = 1_640_995_200; // 2022-01-01 00:00:00 UTC

        let test_cases = vec![
            (reference_ts, 1.0),
            (reference_ts - 90 * 86_400, 0.5),
            (reference_ts - 180 * 86_400, 0.25),
            (reference_ts - 270 * 86_400, 0.125),
        ];

        for (commit_ts, expected_weight) in test_cases {
            let age_days = ((reference_ts - commit_ts) as f64) / 86_400.0;
            let weight = 0.5f64.powf(age_days / half_life_days);
            assert!(
                (weight - expected_weight).abs() < 0.001,
                "Weight for age {} days should be approximately {}",
                age_days,
                expected_weight
            );
        }
    }
}
