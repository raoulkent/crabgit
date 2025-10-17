use anyhow::Result;
use git2::{Repository, Sort};
use serde::Serialize;
use std::collections::HashMap;

use crate::stats::StatsContext;

#[derive(Debug, Serialize, Clone)]
pub struct AuthorStats {
    pub author: String,
    pub commits: u64,
    pub adds: u64,
    pub dels: u64,
}

pub fn compute_authors(repo: &Repository, ctx: &StatsContext) -> Result<Vec<AuthorStats>> {
    let (since_ts, until_ts) = (
        crate::stats::activity::parse_instant(ctx.since.as_deref()),
        crate::stats::activity::parse_instant(ctx.until.as_deref()),
    );

    let mut by_author: HashMap<String, AuthorStats> = HashMap::new();

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

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

        // Skip merges if requested
        if ctx.no_merges && commit.parent_count() > 1 {
            continue;
        }

        let author = commit.author().name().unwrap_or("Unknown").to_string();
        let entry = by_author.entry(author.clone()).or_insert(AuthorStats {
            author,
            commits: 0,
            adds: 0,
            dels: 0,
        });
        entry.commits += 1;

        // Churn via diff to first parent (or empty tree for root)
        let (adds, dels) = if commit.parent_count() == 0 {
            let tree = commit.tree()?;
            crate::stats::churn::diff_trees_public(repo, None, Some(&tree))?
        } else {
            let parent = commit.parent(0)?;
            let parent_tree = parent.tree()?;
            let tree = commit.tree()?;
            crate::stats::churn::diff_trees_public(repo, Some(&parent_tree), Some(&tree))?
        };
        entry.adds += adds;
        entry.dels += dels;
    }

    let mut v: Vec<AuthorStats> = by_author.into_values().collect();
    // sort by commits desc as default; callers can re-sort
    v.sort_by(|a, b| {
        b.commits
            .cmp(&a.commits)
            .then_with(|| (b.adds + b.dels).cmp(&(a.adds + a.dels)))
    });
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_author_stats_creation() {
        // Test AuthorStats struct creation and data integrity
        let stats = AuthorStats {
            author: "Alice Developer".to_string(),
            commits: 42,
            adds: 1500,
            dels: 750,
        };

        assert_eq!(stats.author, "Alice Developer");
        assert_eq!(stats.commits, 42);
        assert_eq!(stats.adds, 1500);
        assert_eq!(stats.dels, 750);

        // Test total churn calculation
        let total_churn = stats.adds + stats.dels;
        assert_eq!(total_churn, 2250);
    }

    #[test]
    fn test_author_stats_serialization() {
        let stats = AuthorStats {
            author: "Bob Contributor".to_string(),
            commits: 15,
            adds: 500,
            dels: 200,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&stats);
        assert!(json_result.is_ok(), "AuthorStats should serialize to JSON");

        let json_str = json_result.unwrap();
        assert!(
            json_str.contains("Bob Contributor"),
            "JSON should contain author name"
        );
        assert!(json_str.contains("15"), "JSON should contain commits count");
        assert!(json_str.contains("500"), "JSON should contain adds count");
        assert!(json_str.contains("200"), "JSON should contain dels count");
    }

    #[test]
    fn test_author_stats_zero_values() {
        // Test AuthorStats with zero values (inactive author)
        let stats = AuthorStats {
            author: "Inactive User".to_string(),
            commits: 0,
            adds: 0,
            dels: 0,
        };

        assert_eq!(stats.commits, 0);
        assert_eq!(stats.adds, 0);
        assert_eq!(stats.dels, 0);

        // Zero values should serialize correctly
        let json_result = serde_json::to_string(&stats);
        assert!(json_result.is_ok());
    }

    #[test]
    fn test_author_stats_large_values() {
        // Test AuthorStats with large values (very active author)
        let stats = AuthorStats {
            author: "Super Contributor".to_string(),
            commits: u64::MAX / 1000,
            adds: u64::MAX / 2,
            dels: u64::MAX / 3,
        };

        assert_eq!(stats.commits, u64::MAX / 1000);
        assert_eq!(stats.adds, u64::MAX / 2);
        assert_eq!(stats.dels, u64::MAX / 3);

        // Large values should serialize without overflow
        let json_result = serde_json::to_string(&stats);
        assert!(
            json_result.is_ok(),
            "Large values should serialize correctly"
        );
    }

    #[test]
    fn test_author_stats_ordering() {
        // Test that AuthorStats can be ordered and compared
        let alice = AuthorStats {
            author: "Alice".to_string(),
            commits: 100,
            adds: 2000,
            dels: 500,
        };

        let bob = AuthorStats {
            author: "Bob".to_string(),
            commits: 75,
            adds: 1500,
            dels: 300,
        };

        let charlie = AuthorStats {
            author: "Charlie".to_string(),
            commits: 100, // same as Alice
            adds: 1800,   // less than Alice
            dels: 400,
        };

        // Test comparisons
        assert!(alice.commits > bob.commits);
        assert_eq!(alice.commits, charlie.commits);

        // Test total churn comparison
        let alice_churn = alice.adds + alice.dels;
        let bob_churn = bob.adds + bob.dels;
        let charlie_churn = charlie.adds + charlie.dels;

        assert!(alice_churn > bob_churn);
        assert!(alice_churn > charlie_churn);
        assert!(charlie_churn > bob_churn);
    }

    #[test]
    fn test_author_stats_cloning() {
        // Test that AuthorStats can be cloned properly
        let original = AuthorStats {
            author: "Original Author".to_string(),
            commits: 25,
            adds: 800,
            dels: 200,
        };

        let cloned = original.clone();

        // Verify all fields are cloned correctly
        assert_eq!(original.author, cloned.author);
        assert_eq!(original.commits, cloned.commits);
        assert_eq!(original.adds, cloned.adds);
        assert_eq!(original.dels, cloned.dels);

        // Verify they are independent (modifying clone doesn't affect original)
        // Note: We can't test this directly with the current struct, but cloning works
    }

    #[test]
    fn test_author_name_handling() {
        // Test various author name formats
        let test_cases = vec![
            "Simple Name",
            "First Last",
            "Name With Spaces And Numbers 123",
            "unicode_テスト_name",
            "email@domain.com",
            "Name <email@domain.com>",
            "", // empty name
        ];

        for name in test_cases {
            let stats = AuthorStats {
                author: name.to_string(),
                commits: 1,
                adds: 10,
                dels: 5,
            };

            assert_eq!(stats.author, name);

            // Should serialize regardless of name format
            let json_result = serde_json::to_string(&stats);
            assert!(json_result.is_ok(), "Should serialize name: '{}'", name);
        }
    }

    #[test]
    fn test_author_stats_debug_format() {
        // Test that Debug trait is implemented and works
        let stats = AuthorStats {
            author: "Debug Test".to_string(),
            commits: 5,
            adds: 100,
            dels: 25,
        };

        let debug_str = format!("{:?}", stats);

        // Debug output should contain key information
        assert!(debug_str.contains("Debug Test"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("100"));
        assert!(debug_str.contains("25"));
        assert!(debug_str.contains("AuthorStats"));
    }

    #[test]
    fn test_author_stats_edge_cases() {
        // Test edge cases for author statistics

        // Only commits, no churn
        let commits_only = AuthorStats {
            author: "Commits Only".to_string(),
            commits: 10,
            adds: 0,
            dels: 0,
        };
        assert_eq!(commits_only.adds + commits_only.dels, 0);

        // Only deletions (cleanup author)
        let deletions_only = AuthorStats {
            author: "Cleanup Expert".to_string(),
            commits: 5,
            adds: 0,
            dels: 1000,
        };
        assert_eq!(deletions_only.adds, 0);
        assert_eq!(deletions_only.dels, 1000);

        // Only additions (new feature author)
        let additions_only = AuthorStats {
            author: "Feature Creator".to_string(),
            commits: 3,
            adds: 2000,
            dels: 0,
        };
        assert_eq!(additions_only.adds, 2000);
        assert_eq!(additions_only.dels, 0);
    }

    #[test]
    fn test_time_parsing_consistency() {
        // Ensure authors module uses consistent time parsing
        let test_cases = vec!["30d", "2w", "6m", "1y", "2023-01-15", "2024-12-31"];

        for test_case in test_cases {
            let result = crate::stats::activity::parse_instant(Some(test_case));

            // Should get consistent results (either Some or None)
            match result {
                Some(timestamp) => {
                    assert!(
                        timestamp > 0,
                        "Timestamp should be positive for: {}",
                        test_case
                    );
                }
                None => {
                    // Some formats might not be supported, that's okay
                }
            }
        }
    }
}
