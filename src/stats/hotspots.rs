use anyhow::Result;
use git2::{DiffOptions, Patch, Repository, Sort, Tree};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use time::OffsetDateTime;

use crate::stats::StatsContext;

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
    include_glob: Option<&str>,
    exclude_glob: Option<&str>,
    half_life_days: f64,
) -> Result<Vec<FileHotspot>> {
    let (since_ts, until_ts) = (
        crate::stats::activity::parse_instant(ctx.since.as_deref()),
        crate::stats::activity::parse_instant(ctx.until.as_deref()),
    );

    let include = build_globset(include_glob)?;
    let exclude = build_globset(exclude_glob)?;

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let mut by_file: HashMap<String, (u64, u64, f64)> = HashMap::new(); // adds, dels, weighted

    let reference_ts = until_ts.unwrap_or_else(|| OffsetDateTime::now_utc().unix_timestamp());
    let hl = if half_life_days > 0.0 {
        half_life_days
    } else {
        90.0
    };

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        if ctx.no_merges && commit.parent_count() > 1 {
            continue;
        }
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

        let parent_tree_opt: Option<Tree> = if commit.parent_count() == 0 {
            None
        } else {
            Some(commit.parent(0)?.tree()?)
        };
        let tree = commit.tree()?;

        let mut opts = DiffOptions::new();
        let diff =
            repo.diff_tree_to_tree(parent_tree_opt.as_ref(), Some(&tree), Some(&mut opts))?;

        // Recency weight
        let age_days = ((reference_ts - ts) as f64) / 86_400.0;
        let weight = 0.5f64.powf(age_days / hl);

        for (i, delta) in diff.deltas().enumerate() {
            // Determine path (prefer new path)
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .and_then(|p| p.to_str())
                .map(|s| s.to_string());
            let Some(path) = path else { continue };

            if !matches_globsets(&path, include.as_ref(), exclude.as_ref()) {
                continue;
            }

            // Compute per-file adds/dels via patch line stats
            if let Some(patch) = Patch::from_diff(&diff, i)? {
                let (_ctx, adds, dels) = patch.line_stats()?;
                let entry = by_file.entry(path).or_insert((0, 0, 0.0));
                entry.0 += adds as u64;
                entry.1 += dels as u64;
                entry.2 += weight * (adds as f64 + dels as f64);
            }
        }
    }

    let mut out: Vec<FileHotspot> = by_file
        .into_iter()
        .map(|(path, (adds, dels, weighted))| FileHotspot {
            path,
            adds,
            dels,
            churn: adds + dels,
            weighted,
        })
        .collect();

    // Sort by weighted churn desc, then raw churn
    out.sort_by(|a, b| {
        b.weighted
            .partial_cmp(&a.weighted)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.churn.cmp(&a.churn))
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(out)
}

fn build_globset(pattern: Option<&str>) -> Result<Option<GlobSet>> {
    let Some(pat) = pattern else { return Ok(None) };
    let mut builder = GlobSetBuilder::new();
    // Use literal separators and case-sensitive matching by default
    let g = GlobBuilder::new(pat).literal_separator(true).build()?;
    builder.add(g);
    Ok(Some(builder.build()?))
}

fn matches_globsets(path: &str, include: Option<&GlobSet>, exclude: Option<&GlobSet>) -> bool {
    let p = Path::new(path);
    if let Some(ex) = exclude
        && ex.is_match(p)
    {
        return false;
    }
    if let Some(inc) = include {
        return inc.is_match(p);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_hotspot_creation() {
        // Test FileHotspot struct creation and data integrity
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
        
        // Verify churn calculation consistency
        assert_eq!(hotspot.churn, hotspot.adds + hotspot.dels);
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
        
        // Test JSON serialization
        let json_result = serde_json::to_string(&hotspot);
        assert!(json_result.is_ok(), "FileHotspot should serialize to JSON");
        
        let json_str = json_result.unwrap();
        assert!(json_str.contains("lib/utils.rs"), "JSON should contain path");
        assert!(json_str.contains("500"), "JSON should contain adds");
        assert!(json_str.contains("200"), "JSON should contain dels");
        assert!(json_str.contains("700"), "JSON should contain churn");
        assert!(json_str.contains("642.3"), "JSON should contain weighted");
    }

    #[test]
    fn test_file_hotspot_zero_values() {
        // Test FileHotspot with zero churn (unchanged file)
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
        
        // Should serialize correctly
        let json_result = serde_json::to_string(&hotspot);
        assert!(json_result.is_ok());
    }

    #[test]
    fn test_file_hotspot_large_values() {
        // Test FileHotspot with large churn values
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
        
        // Should serialize without overflow
        let json_result = serde_json::to_string(&hotspot);
        assert!(json_result.is_ok(), "Large values should serialize correctly");
    }

    #[test]
    fn test_build_globset_valid_patterns() {
        // Test valid glob patterns
        let test_cases = vec![
            "*.rs",
            "src/**/*.rs",
            "lib/**",
            "test_*.py",
            "**/*.{js,ts}",
        ];
        
        for pattern in test_cases {
            let result = build_globset(Some(pattern));
            assert!(result.is_ok(), "Should build globset for pattern: {}", pattern);
            
            if let Ok(Some(globset)) = result {
                // Should be usable for matching
                // We can't easily test matching without creating Path objects
                let _globset = globset; // Just verify it's created
            }
        }
    }

    #[test]
    fn test_build_globset_none() {
        // Test None input
        let result = build_globset(None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test] 
    fn test_matches_globsets_include_exclude() {
        // Test glob matching logic with various scenarios
        let include_rs = build_globset(Some("*.rs")).unwrap();
        let exclude_test = build_globset(Some("test_*")).unwrap();
        
        // Test cases: (path, include_glob, exclude_glob, expected_result)
        let test_cases = vec![
            ("main.rs", include_rs.as_ref(), None, true),           // matches include
            ("lib.py", include_rs.as_ref(), None, false),          // doesn't match include
            ("test_main.rs", include_rs.as_ref(), exclude_test.as_ref(), false), // matches exclude
            ("main.rs", None, exclude_test.as_ref(), true),        // no include, doesn't match exclude
            ("test_lib.py", None, exclude_test.as_ref(), false),   // no include, matches exclude
        ];
        
        for (path, include, exclude, expected) in test_cases {
            let result = matches_globsets(path, include, exclude);
            assert_eq!(result, expected, 
                "Path '{}' should match: {}, got: {}", path, expected, result);
        }
    }

    #[test]
    fn test_matches_globsets_no_filters() {
        // Test with no include/exclude filters (should match all)
        let paths = vec![
            "main.rs",
            "lib.py", 
            "test.js",
            "README.md",
            "src/utils.rs",
        ];
        
        for path in paths {
            let result = matches_globsets(path, None, None);
            assert!(result, "Path '{}' should match when no filters are applied", path);
        }
    }

    #[test]
    fn test_file_hotspot_cloning() {
        // Test FileHotspot cloning
        let original = FileHotspot {
            path: "src/lib.rs".to_string(),
            adds: 100,
            dels: 50,
            churn: 150,
            weighted: 125.7,
        };
        
        let cloned = original.clone();
        
        // Verify all fields are cloned correctly
        assert_eq!(original.path, cloned.path);
        assert_eq!(original.adds, cloned.adds);
        assert_eq!(original.dels, cloned.dels);
        assert_eq!(original.churn, cloned.churn);
        assert_eq!(original.weighted, cloned.weighted);
    }

    #[test]
    fn test_file_hotspot_debug_format() {
        // Test Debug trait implementation
        let hotspot = FileHotspot {
            path: "debug_test.rs".to_string(),
            adds: 42,
            dels: 21,
            churn: 63,
            weighted: 58.5,
        };
        
        let debug_str = format!("{:?}", hotspot);
        
        // Debug output should contain key information
        assert!(debug_str.contains("debug_test.rs"));
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("21"));
        assert!(debug_str.contains("63"));
        assert!(debug_str.contains("58.5"));
        assert!(debug_str.contains("FileHotspot"));
    }

    #[test]
    fn test_hotspot_path_formats() {
        // Test various path formats
        let path_cases = vec![
            "main.rs",
            "src/lib.rs",
            "tests/integration/test_main.rs",
            "../parent/file.py",
            "./current/file.js",
            "C:\\Windows\\System32\\file.dll", // Windows path
            "/usr/local/bin/script.sh",        // Unix absolute path
            "unicode_测试_file.rs",             // Unicode filename
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
            
            // Should serialize regardless of path format
            let json_result = serde_json::to_string(&hotspot);
            assert!(json_result.is_ok(), "Should serialize path: '{}'", path);
        }
    }

    #[test]
    fn test_recency_weight_calculation() {
        // Test recency weight calculation logic
        let half_life_days = 90.0;
        let reference_ts = 1640995200; // 2022-01-01 00:00:00 UTC
        
        // Test age-based weight decay
        let test_cases = vec![
            (reference_ts, 1.0),                              // current time = full weight
            (reference_ts - 90 * 86_400, 0.5),               // 90 days ago = half weight
            (reference_ts - 180 * 86_400, 0.25),             // 180 days ago = quarter weight
            (reference_ts - 270 * 86_400, 0.125),            // 270 days ago = eighth weight
        ];
        
        for (commit_ts, expected_weight) in test_cases {
            let age_days = ((reference_ts - commit_ts) as f64) / 86_400.0;
            let weight = 0.5f64.powf(age_days / half_life_days);
            
            assert!((weight - expected_weight).abs() < 0.001, 
                "Weight for age {} days should be approximately {}, got {}", 
                age_days, expected_weight, weight);
        }
    }
}
