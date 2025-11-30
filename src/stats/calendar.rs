use anyhow::Result;
use git2::Repository;
use serde::Serialize;

use crate::stats::{StatsContext, path_filter};

#[derive(Debug, Serialize, Clone)]
pub struct CalendarHeatmap {
    // 7 x 24 matrix, rows: 0=Mon..6=Sun (ISO), cols: 0..23 hours UTC
    pub matrix: [[u64; 24]; 7],
}

pub fn compute_calendar(repo: &Repository, ctx: &StatsContext) -> Result<CalendarHeatmap> {
    let filtered_commits = path_filter::FilteredCommits::new(repo, ctx)?;
    let mut matrix = [[0u64; 24]; 7];

    for commit_result in filtered_commits {
        let commit = commit_result?;
        let ts = commit.time().seconds();

        let dt = time::OffsetDateTime::from_unix_timestamp(ts)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        let iso_weekday = dt.date().weekday().number_from_monday() as usize - 1; // 0..6
        let hour = dt.hour() as usize;
        matrix[iso_weekday][hour] += 1;
    }

    Ok(CalendarHeatmap { matrix })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn test_calendar_heatmap_creation() {
        // Test CalendarHeatmap struct creation
        let matrix = [[0u64; 24]; 7];
        let heatmap = CalendarHeatmap { matrix };

        // Verify dimensions (7 days x 24 hours)
        assert_eq!(heatmap.matrix.len(), 7);
        assert_eq!(heatmap.matrix[0].len(), 24);

        // All values should be zero initially
        for day in 0..7 {
            for hour in 0..24 {
                assert_eq!(heatmap.matrix[day][hour], 0);
            }
        }
    }

    #[test]
    fn test_calendar_heatmap_serialization() {
        let mut matrix = [[0u64; 24]; 7];
        // Add some sample data
        matrix[1][9] = 5; // Tuesday 9 AM
        matrix[4][14] = 3; // Friday 2 PM

        let heatmap = CalendarHeatmap { matrix };

        // Test JSON serialization
        let json_result = serde_json::to_string(&heatmap);
        assert!(
            json_result.is_ok(),
            "CalendarHeatmap should serialize to JSON"
        );

        let json_str = json_result.unwrap();
        assert!(
            json_str.contains("matrix"),
            "JSON should contain matrix field"
        );
    }

    #[test]
    fn test_calendar_matrix_indexing() {
        let mut matrix = [[0u64; 24]; 7];

        // Test weekday indexing (0=Monday, 6=Sunday)
        matrix[0][10] = 10; // Monday 10 AM
        matrix[6][22] = 20; // Sunday 10 PM

        let heatmap = CalendarHeatmap { matrix };

        assert_eq!(heatmap.matrix[0][10], 10); // Monday 10 AM
        assert_eq!(heatmap.matrix[6][22], 20); // Sunday 10 PM

        // All other values should be zero
        assert_eq!(heatmap.matrix[0][9], 0);
        assert_eq!(heatmap.matrix[6][21], 0);
    }

    #[test]
    fn test_calendar_matrix_boundaries() {
        let mut matrix = [[0u64; 24]; 7];

        // Test boundary values
        matrix[0][0] = 1; // Monday midnight
        matrix[0][23] = 2; // Monday 11 PM
        matrix[6][0] = 3; // Sunday midnight  
        matrix[6][23] = 4; // Sunday 11 PM

        let heatmap = CalendarHeatmap { matrix };

        assert_eq!(heatmap.matrix[0][0], 1);
        assert_eq!(heatmap.matrix[0][23], 2);
        assert_eq!(heatmap.matrix[6][0], 3);
        assert_eq!(heatmap.matrix[6][23], 4);
    }

    #[test]
    fn test_calendar_large_values() {
        let mut matrix = [[0u64; 24]; 7];

        // Test large commit counts
        matrix[3][15] = u64::MAX / 2; // Wednesday 3 PM - very busy

        let heatmap = CalendarHeatmap { matrix };

        assert_eq!(heatmap.matrix[3][15], u64::MAX / 2);

        // Should serialize correctly with large values
        let json_result = serde_json::to_string(&heatmap);
        assert!(
            json_result.is_ok(),
            "Should handle large values in serialization"
        );
    }

    #[test]
    fn test_calendar_weekday_mapping() {
        // Test ISO weekday mapping (Monday=1, Sunday=7 -> array indices 0-6)
        let test_cases = vec![
            (time::Weekday::Monday, 0),
            (time::Weekday::Tuesday, 1),
            (time::Weekday::Wednesday, 2),
            (time::Weekday::Thursday, 3),
            (time::Weekday::Friday, 4),
            (time::Weekday::Saturday, 5),
            (time::Weekday::Sunday, 6),
        ];

        for (weekday, expected_index) in test_cases {
            let array_index = weekday.number_from_monday() as usize - 1;
            assert_eq!(
                array_index, expected_index,
                "Weekday {:?} should map to array index {}",
                weekday, expected_index
            );
        }
    }

    #[test]
    fn test_calendar_time_consistency() {
        // Test that calendar uses consistent time parsing with other modules
        let test_timestamps = vec![
            1609459200, // 2021-01-01 00:00:00 UTC (Friday)
            1609545600, // 2021-01-02 00:00:00 UTC (Saturday)
            1609632000, // 2021-01-03 00:00:00 UTC (Sunday)
        ];

        for ts in test_timestamps {
            let dt = OffsetDateTime::from_unix_timestamp(ts).unwrap_or(OffsetDateTime::UNIX_EPOCH);
            let iso_weekday = dt.date().weekday().number_from_monday() as usize - 1;
            let hour = dt.hour() as usize;

            // Should be valid indices
            assert!(
                iso_weekday < 7,
                "Weekday index should be < 7, got {}",
                iso_weekday
            );
            assert!(hour < 24, "Hour should be < 24, got {}", hour);
        }
    }

    #[test]
    fn test_calendar_clone_and_debug() {
        let mut matrix = [[0u64; 24]; 7];
        matrix[2][12] = 42; // Wednesday noon

        let original = CalendarHeatmap { matrix };
        let cloned = original.clone();

        // Verify cloning works correctly
        assert_eq!(original.matrix[2][12], cloned.matrix[2][12]);

        // Test debug formatting
        let debug_str = format!("{:?}", original);
        assert!(debug_str.contains("CalendarHeatmap"));
        assert!(debug_str.contains("matrix"));
    }

    #[test]
    fn test_time_parsing_integration() {
        // Test time parsing consistency with activity module
        let test_cases = vec![
            "1d",
            "7d",
            "30d",
            "1w",
            "2w",
            "4w",
            "2023-01-01",
            "2024-06-15",
        ];

        for test_case in test_cases {
            let result = crate::stats::activity::parse_instant(Some(test_case));

            if let Some(timestamp) = result {
                // If parsing succeeds, timestamp should be reasonable
                assert!(
                    timestamp > 0,
                    "Parsed timestamp should be positive for: {}",
                    test_case
                );

                // Should be able to convert to weekday/hour
                let dt = OffsetDateTime::from_unix_timestamp(timestamp)
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH);
                let weekday_index = dt.date().weekday().number_from_monday() as usize - 1;
                let hour = dt.hour() as usize;

                assert!(weekday_index < 7);
                assert!(hour < 24);
            }
        }
    }
}
