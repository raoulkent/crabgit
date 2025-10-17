use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Timer for measuring operation durations
#[allow(dead_code)]
#[derive(Debug)]
pub struct Timer {
    start: Instant,
    name: String,
    enabled: bool,
}

#[allow(dead_code)]
impl Timer {
    /// Create a new timer
    pub fn new(name: &str, enabled: bool) -> Self {
        if enabled {
            println!("[TIMER] Starting: {}", name);
        }

        Self {
            start: Instant::now(),
            name: name.to_string(),
            enabled,
        }
    }

    /// Stop the timer and log the duration
    pub fn stop(self) -> Duration {
        let duration = self.start.elapsed();

        if self.enabled {
            println!("[TIMER] Finished: {} ({:?})", self.name, duration);
        }

        duration
    }

    /// Stop the timer with a custom message
    pub fn stop_with_message(self, message: &str) -> Duration {
        let duration = self.start.elapsed();

        if self.enabled {
            println!(
                "[TIMER] Finished: {} - {} ({:?})",
                self.name, message, duration
            );
        }

        duration
    }
}

/// Performance metrics collector
#[derive(Debug, Default)]
pub struct PerfMetrics {
    timings: HashMap<String, Vec<Duration>>,
    enabled: bool,
}

#[allow(dead_code)]
impl PerfMetrics {
    /// Create a new metrics collector
    pub fn new(enabled: bool) -> Self {
        Self {
            timings: HashMap::new(),
            enabled,
        }
    }

    /// Record a timing measurement
    #[allow(dead_code)]
    pub fn record(&mut self, operation: &str, duration: Duration) {
        if !self.enabled {
            return;
        }

        self.timings
            .entry(operation.to_string())
            .or_default()
            .push(duration);
    }

    /// Start a timer for an operation
    #[allow(dead_code)]
    pub fn timer(&self, operation: &str) -> Timer {
        Timer::new(operation, self.enabled)
    }

    /// Get summary statistics for all recorded operations
    pub fn summary(&self) -> PerformanceSummary {
        let mut operations = Vec::new();

        for (name, durations) in &self.timings {
            if durations.is_empty() {
                continue;
            }

            let total: Duration = durations.iter().sum();
            let count = durations.len();
            let avg = total / count as u32;
            let min = *durations.iter().min().unwrap();
            let max = *durations.iter().max().unwrap();

            operations.push(OperationStats {
                name: name.clone(),
                count,
                total,
                avg,
                min,
                max,
            });
        }

        // Sort by total time descending
        operations.sort_by(|a, b| b.total.cmp(&a.total));

        PerformanceSummary { operations }
    }

    /// Print performance summary
    pub fn print_summary(&self) {
        if !self.enabled || self.timings.is_empty() {
            return;
        }

        let summary = self.summary();

        println!("\n[PERF] Performance Summary");
        println!("[PERF] ===================");

        for stats in &summary.operations {
            println!(
                "[PERF] {}: {} calls, total: {:?}, avg: {:?}, min: {:?}, max: {:?}",
                stats.name, stats.count, stats.total, stats.avg, stats.min, stats.max
            );
        }

        println!();
    }

    /// Clear all recorded metrics
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.timings.clear();
    }
}

/// Performance summary
#[derive(Debug)]
pub struct PerformanceSummary {
    pub operations: Vec<OperationStats>,
}

/// Statistics for a single operation
#[derive(Debug)]
pub struct OperationStats {
    pub name: String,
    pub count: usize,
    pub total: Duration,
    pub avg: Duration,
    pub min: Duration,
    pub max: Duration,
}

/// Macro for timing code blocks
#[macro_export]
macro_rules! timed {
    ($metrics:expr, $operation:expr, $code:block) => {{
        let timer = $metrics.timer($operation);
        let result = $code;
        let duration = timer.stop();
        $metrics.record($operation, duration);
        result
    }};
}

/// Debug logging utilities
pub struct DebugLogger {
    enabled: bool,
}

#[allow(dead_code)]
impl DebugLogger {
    /// Create a new debug logger
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Log a debug message
    #[allow(dead_code)]
    pub fn debug(&self, message: &str) {
        if self.enabled {
            println!("[DEBUG] {}", message);
        }
    }

    /// Log a debug message with formatting
    #[allow(dead_code)]
    pub fn debugf(&self, args: std::fmt::Arguments) {
        if self.enabled {
            println!("[DEBUG] {}", args);
        }
    }

    /// Log an info message
    pub fn info(&self, message: &str) {
        if self.enabled {
            println!("[INFO] {}", message);
        }
    }

    /// Log a warning message
    #[allow(dead_code)]
    pub fn warn(&self, message: &str) {
        if self.enabled {
            println!("[WARN] {}", message);
        }
    }

    /// Log an error message
    #[allow(dead_code)]
    pub fn error(&self, message: &str) {
        if self.enabled {
            eprintln!("[ERROR] {}", message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_timer_enabled() {
        let timer = Timer::new("test_operation", true);
        thread::sleep(Duration::from_millis(1));
        let duration = timer.stop();

        assert!(duration >= Duration::from_millis(1));
    }

    #[test]
    fn test_timer_disabled() {
        let timer = Timer::new("test_operation", false);
        thread::sleep(Duration::from_millis(1));
        let duration = timer.stop();

        assert!(duration >= Duration::from_millis(1));
    }

    #[test]
    fn test_perf_metrics() {
        let mut metrics = PerfMetrics::new(true);

        // Record some test timings
        metrics.record("operation1", Duration::from_millis(100));
        metrics.record("operation1", Duration::from_millis(200));
        metrics.record("operation2", Duration::from_millis(50));

        let summary = metrics.summary();

        assert_eq!(summary.operations.len(), 2);

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
    }

    #[test]
    fn test_perf_metrics_disabled() {
        let mut metrics = PerfMetrics::new(false);

        metrics.record("operation1", Duration::from_millis(100));

        let summary = metrics.summary();
        assert_eq!(summary.operations.len(), 0);
    }

    #[test]
    fn test_debug_logger() {
        let logger = DebugLogger::new(true);

        // These should not panic
        logger.debug("Debug message");
        logger.info("Info message");
        logger.warn("Warning message");
        logger.error("Error message");

        let disabled_logger = DebugLogger::new(false);
        disabled_logger.debug("This should not print");
    }
}
