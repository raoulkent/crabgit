use anyhow::Result;
use std::fs;
use tempfile::TempDir;

use crabgit::cache::{CacheConfig, CachedCommitStats, CommitStatsCache};
use crabgit::config::{
    ConfigOverrides, DebugConfig, GlobalConfig, ParallelConfig, parse_duration_to_days,
};
use crabgit::parallel::{BatchProcessor, MemorySafeIterator, ParallelProcessor};
use crabgit::telemetry::{DebugLogger, PerfMetrics, Timer};

#[test]
fn test_cache_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        enabled: true,
        cache_dir: temp_dir.path().to_path_buf(),
        max_size_mb: 10,
    };

    let mut cache = CommitStatsCache::new(cache_config)?;

    // Test multiple commit stats
    let commits = vec![
        CachedCommitStats {
            oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
            timestamp: 1234567890,
            additions: 100,
            deletions: 50,
            files_changed: 5,
        },
        CachedCommitStats {
            oid: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
            timestamp: 1234567900,
            additions: 200,
            deletions: 75,
            files_changed: 8,
        },
    ];

    // Store commits in cache
    for commit in &commits {
        cache.put(commit.clone())?;
    }

    // Verify retrieval
    for commit in &commits {
        let oid = git2::Oid::from_str(&commit.oid)?;
        let retrieved = cache.get(&oid);
        assert!(retrieved.is_some());
        let retrieved_commit = retrieved.unwrap();
        assert_eq!(retrieved_commit.oid, commit.oid);
        assert_eq!(retrieved_commit.additions, commit.additions);
        assert_eq!(retrieved_commit.deletions, commit.deletions);
    }

    // Test cache stats
    let stats = cache.stats();
    assert!(stats.enabled);
    assert_eq!(stats.memory_entries, 2);
    assert!(stats.disk_entries >= 2);
    assert!(stats.disk_size_bytes > 0);

    // Test cache clear
    cache.clear()?;
    let stats_after_clear = cache.stats();
    assert_eq!(stats_after_clear.memory_entries, 0);

    Ok(())
}

#[test]
fn test_config_system() -> Result<()> {
    // Test default configuration
    let default_config = GlobalConfig::default();
    assert_eq!(default_config.default_window, "90d");
    assert_eq!(default_config.default_top, 25);
    assert!(default_config.cache.enabled);
    assert!(default_config.parallel.enabled);
    assert!(!default_config.debug.enabled);

    // Test configuration with overrides
    let overrides = ConfigOverrides {
        debug: Some(true),
        cache_enabled: Some(false),
        parallel_enabled: Some(false),
        max_threads: Some(8),
    };

    let config_with_overrides = default_config.with_overrides(overrides);
    assert!(config_with_overrides.debug.enabled);
    assert!(config_with_overrides.debug.timing); // Should be enabled with debug
    assert!(!config_with_overrides.cache.enabled);
    assert!(!config_with_overrides.parallel.enabled);
    assert_eq!(config_with_overrides.parallel.max_threads, 8);

    // Test duration parsing
    assert_eq!(parse_duration_to_days("7d")?, 7);
    assert_eq!(parse_duration_to_days("2w")?, 14);
    assert_eq!(parse_duration_to_days("3m")?, 90);
    assert_eq!(parse_duration_to_days("1y")?, 365);
    assert_eq!(parse_duration_to_days("30")?, 30); // Default to days

    // Test case insensitivity
    assert_eq!(parse_duration_to_days("7D")?, 7);
    assert_eq!(parse_duration_to_days("2W")?, 14);

    // Test invalid durations
    assert!(parse_duration_to_days("").is_err());
    assert!(parse_duration_to_days("abc").is_err());
    assert!(parse_duration_to_days("7x").is_err());

    Ok(())
}

#[test]
fn test_config_file_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("config.toml");

    // Create a custom config
    let config = GlobalConfig {
        default_window: "180d".to_string(),
        default_top: 50,
        cache: CacheConfig {
            enabled: false,
            ..Default::default()
        },
        debug: DebugConfig {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    };

    // Manually save to temp location for testing
    let config_content = toml::to_string_pretty(&config)?;
    fs::write(&config_path, config_content)?;

    // Load and verify
    let loaded_content = fs::read_to_string(&config_path)?;
    let loaded_config: GlobalConfig = toml::from_str(&loaded_content)?;

    assert_eq!(loaded_config.default_window, "180d");
    assert_eq!(loaded_config.default_top, 50);
    assert!(!loaded_config.cache.enabled);
    assert!(loaded_config.debug.enabled);

    Ok(())
}

#[test]
fn test_telemetry_system() -> Result<()> {
    use std::thread;
    use std::time::Duration;

    // Test timer functionality
    let timer = Timer::new("test_operation", true);
    thread::sleep(Duration::from_millis(10));
    let duration = timer.stop();
    assert!(duration >= Duration::from_millis(10));

    // Test performance metrics
    let mut metrics = PerfMetrics::new(true);

    // Record various operations
    metrics.record("operation1", Duration::from_millis(100));
    metrics.record("operation1", Duration::from_millis(200));
    metrics.record("operation2", Duration::from_millis(50));
    metrics.record("operation3", Duration::from_millis(300));

    let summary = metrics.summary();
    assert_eq!(summary.operations.len(), 3);

    // Find operation1 stats
    let op1_stats = summary
        .operations
        .iter()
        .find(|s| s.name == "operation1")
        .unwrap();

    assert_eq!(op1_stats.count, 2);
    assert_eq!(op1_stats.total, Duration::from_millis(300));
    assert_eq!(op1_stats.avg, Duration::from_millis(150));
    assert_eq!(op1_stats.min, Duration::from_millis(100));
    assert_eq!(op1_stats.max, Duration::from_millis(200));

    // Test operations are sorted by total time (descending)
    assert!(summary.operations[0].total >= summary.operations[1].total);
    assert!(summary.operations[1].total >= summary.operations[2].total);

    // Test debug logger
    let logger = DebugLogger::new(true);
    logger.debug("Test debug message");
    logger.info("Test info message");
    logger.warn("Test warning message");
    logger.error("Test error message");

    // Test disabled logger (should not panic)
    let disabled_logger = DebugLogger::new(false);
    disabled_logger.debug("This should not print");

    Ok(())
}

#[test]
fn test_parallel_processing() -> Result<()> {
    // Test with parallel enabled
    let parallel_config = ParallelConfig {
        enabled: true,
        max_threads: 0, // Use default to avoid thread pool reinitialization
        chunk_size: 100,
    };

    let processor = ParallelProcessor::new(parallel_config.clone())?;

    // Test parallel map
    let numbers: Vec<i32> = (1..=1000).collect();
    let results = processor.par_map(numbers.clone(), |x| x * 2);
    let expected: Vec<i32> = numbers.iter().map(|&x| x * 2).collect();
    assert_eq!(results, expected);

    // Test parallel filter_map
    let filtered_results =
        processor.par_filter_map(
            numbers.clone(),
            |x| {
                if x % 2 == 0 { Some(x * 3) } else { None }
            },
        );
    let expected_filtered: Vec<i32> = numbers
        .iter()
        .filter(|&&x| x % 2 == 0)
        .map(|&x| x * 3)
        .collect();
    assert_eq!(filtered_results, expected_filtered);

    // Test parallel reduce
    let sum = processor.par_reduce(numbers.clone(), || 0, |a, b| a + b);
    let expected_sum: i32 = numbers.iter().sum();
    assert_eq!(sum, expected_sum);

    // Test with parallel disabled
    let sequential_config = ParallelConfig {
        enabled: false,
        max_threads: 1,
        chunk_size: 100,
    };

    let sequential_processor = ParallelProcessor::new(sequential_config)?;
    let sequential_results = sequential_processor.par_map(numbers.clone(), |x| x * 2);
    assert_eq!(sequential_results, expected);

    Ok(())
}

#[test]
fn test_batch_processing() {
    use std::sync::{Arc, Mutex};

    let config = ParallelConfig {
        enabled: true,
        max_threads: 0, // Use default to avoid thread pool reinitialization
        chunk_size: 100,
    };

    let numbers: Vec<i32> = (1..=1000).collect();
    let processor = BatchProcessor::new(numbers, config);

    // Test batch processing
    let results = processor.process(|batch| batch.iter().map(|&x| x * 2).collect());

    assert_eq!(results.len(), 1000);
    for (i, &result) in results.iter().enumerate() {
        assert_eq!(result, (i as i32 + 1) * 2);
    }

    // Test batch processing with progress tracking
    let numbers: Vec<i32> = (1..=300).collect(); // 3 batches of 100
    let processor = BatchProcessor::new(
        numbers,
        ParallelConfig {
            enabled: false, // Use sequential for predictable progress
            max_threads: 1,
            chunk_size: 100,
        },
    );

    let progress_calls = Arc::new(Mutex::new(Vec::new()));
    let progress_calls_clone = progress_calls.clone();

    let results = processor.process_with_progress(
        |batch| batch.iter().map(|&x| x * 2).collect(),
        move |current, total| {
            progress_calls_clone.lock().unwrap().push((current, total));
        },
    );

    let progress = progress_calls.lock().unwrap();
    assert_eq!(progress.len(), 3);
    assert_eq!(progress[0], (1, 3));
    assert_eq!(progress[1], (2, 3));
    assert_eq!(progress[2], (3, 3));

    assert_eq!(results.len(), 300);
}

#[test]
fn test_memory_safe_iterator() {
    let data: Vec<i32> = (1..=1000).collect();
    let iterator = MemorySafeIterator::new(data, 100, true);

    let window_sums = iterator.process_windows(|window| window.iter().sum::<i32>());

    // Should have 10 windows of 100 items each
    assert_eq!(window_sums.len(), 10);

    // First window: 1+2+...+100 = 5050
    assert_eq!(window_sums[0], 5050);
    // Second window: 101+102+...+200 = 15050
    assert_eq!(window_sums[1], 15050);

    // Test sequential processing
    let sequential_iterator = MemorySafeIterator::new((1..=100).collect(), 25, false);
    let sequential_sums = sequential_iterator.process_windows(|window| window.iter().sum::<i32>());

    assert_eq!(sequential_sums.len(), 4);
    // First window: 1+2+...+25 = 325
    assert_eq!(sequential_sums[0], 325);
}

#[test]
fn test_integration_cache_and_config() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create config with custom cache directory
    let mut config = GlobalConfig::default();
    config.cache.cache_dir = temp_dir.path().join("custom_cache");
    config.cache.enabled = true;
    config.cache.max_size_mb = 50;

    // Test that cache uses the custom directory
    let mut cache = CommitStatsCache::new(config.cache.clone())?;

    let test_stats = CachedCommitStats {
        oid: "abcdef1234567890abcdef1234567890abcdef12".to_string(), // Valid 40-char git OID
        timestamp: 1234567890,
        additions: 42,
        deletions: 24,
        files_changed: 3,
    };

    cache.put(test_stats.clone())?;

    // Verify the cache directory was created in the custom location
    assert!(config.cache.cache_dir.exists());

    let stats = cache.stats();
    assert!(stats.enabled);
    assert_eq!(stats.memory_entries, 1);
    assert!(stats.disk_entries >= 1);

    Ok(())
}

#[test]
fn test_performance_under_load() -> Result<()> {
    use std::time::Instant;

    // Test parallel processing performance with larger dataset
    let config = ParallelConfig {
        enabled: true,
        max_threads: 0, // Use default to avoid thread pool reinitialization
        chunk_size: 1000,
    };

    let processor = ParallelProcessor::new(config)?;
    let large_dataset: Vec<i32> = (1..=100_000).collect();

    let start = Instant::now();
    let results = processor.par_map(large_dataset.clone(), |x| {
        // Simulate some computational work (avoid overflow)
        (x % 1000) * 2 + 1
    });
    let parallel_duration = start.elapsed();

    assert_eq!(results.len(), 100_000);

    // Test sequential processing for comparison
    let sequential_config = ParallelConfig {
        enabled: false,
        max_threads: 1,
        chunk_size: 1000,
    };

    let sequential_processor = ParallelProcessor::new(sequential_config)?;

    let start = Instant::now();
    let sequential_results = sequential_processor.par_map(large_dataset, |x| (x % 1000) * 2 + 1);
    let sequential_duration = start.elapsed();

    assert_eq!(sequential_results.len(), 100_000);
    assert_eq!(results, sequential_results);

    // On multi-core systems, parallel should generally be faster for this workload
    // But we don't assert this as it depends on the test environment
    println!("Parallel duration: {:?}", parallel_duration);
    println!("Sequential duration: {:?}", sequential_duration);

    Ok(())
}

#[test]
fn test_error_handling() -> Result<()> {
    // Test cache with invalid OID
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        enabled: true,
        cache_dir: temp_dir.path().to_path_buf(),
        max_size_mb: 10,
    };

    let mut cache = CommitStatsCache::new(cache_config)?;

    let invalid_stats = CachedCommitStats {
        oid: "invalid_oid".to_string(),
        timestamp: 1234567890,
        additions: 100,
        deletions: 50,
        files_changed: 5,
    };

    // This should fail with invalid OID
    assert!(cache.put(invalid_stats).is_err());

    // Test config parsing errors
    assert!(parse_duration_to_days("").is_err());
    assert!(parse_duration_to_days("invalid").is_err());
    assert!(parse_duration_to_days("7z").is_err()); // Invalid unit

    Ok(())
}

#[test]
fn test_cache_disabled_behavior() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        enabled: false,
        cache_dir: temp_dir.path().to_path_buf(),
        max_size_mb: 10,
    };

    let mut cache = CommitStatsCache::new(cache_config)?;

    let test_stats = CachedCommitStats {
        oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
        timestamp: 1234567890,
        additions: 100,
        deletions: 50,
        files_changed: 5,
    };

    // Should succeed but not actually cache anything
    cache.put(test_stats.clone())?;

    let oid = git2::Oid::from_str(&test_stats.oid)?;
    let retrieved = cache.get(&oid);
    assert!(retrieved.is_none());

    let stats = cache.stats();
    assert!(!stats.enabled);
    assert_eq!(stats.memory_entries, 0);
    assert_eq!(stats.disk_entries, 0);

    Ok(())
}
