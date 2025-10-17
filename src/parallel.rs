use anyhow::Result;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::sync::Arc;

use crate::config::ParallelConfig;

/// Parallel processing manager
pub struct ParallelProcessor {
    config: ParallelConfig,
}

impl ParallelProcessor {
    /// Create a new parallel processor
    pub fn new(config: ParallelConfig) -> Result<Self> {
        if config.enabled && config.max_threads > 0 {
            // Configure Rayon thread pool
            ThreadPoolBuilder::new()
                .num_threads(config.max_threads)
                .build_global()
                .map_err(|e| anyhow::anyhow!("Failed to configure thread pool: {}", e))?;
        }
        
        Ok(Self { config })
    }

    /// Process items in parallel using a map function
    pub fn par_map<T, R, F>(&self, items: Vec<T>, f: F) -> Vec<R>
    where
        T: Send + Sync,
        R: Send,
        F: Fn(T) -> R + Send + Sync,
    {
        if self.config.enabled {
            items.into_par_iter().map(f).collect()
        } else {
            items.into_iter().map(f).collect()
        }
    }

    /// Process items in parallel with chunking
    pub fn par_map_chunked<T, R, F>(&self, items: Vec<T>, f: F) -> Vec<R>
    where
        T: Send + Sync,
        R: Send,
        F: Fn(T) -> R + Send + Sync,
    {
        if self.config.enabled {
            items
                .into_par_iter()
                .with_min_len(self.config.chunk_size)
                .map(f)
                .collect()
        } else {
            items.into_iter().map(f).collect()
        }
    }

    /// Process items in parallel and filter results
    pub fn par_filter_map<T, R, F>(&self, items: Vec<T>, f: F) -> Vec<R>
    where
        T: Send + Sync,
        R: Send,
        F: Fn(T) -> Option<R> + Send + Sync,
    {
        if self.config.enabled {
            items.into_par_iter().filter_map(f).collect()
        } else {
            items.into_iter().filter_map(f).collect()
        }
    }

    /// Reduce items in parallel
    pub fn par_reduce<T, F, I>(&self, items: Vec<T>, identity: I, reduce_fn: F) -> T
    where
        T: Send + Sync + Clone,
        F: Fn(T, T) -> T + Send + Sync,
        I: Fn() -> T + Send + Sync,
    {
        if self.config.enabled {
            items.into_par_iter().reduce(&identity, &reduce_fn)
        } else {
            items.into_iter().fold(identity(), reduce_fn)
        }
    }

    /// Check if parallel processing is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the configured number of threads
    pub fn thread_count(&self) -> usize {
        if self.config.max_threads == 0 {
            rayon::current_num_threads()
        } else {
            self.config.max_threads
        }
    }
}

/// Batch processor for handling large datasets
pub struct BatchProcessor<T> {
    items: Vec<T>,
    batch_size: usize,
    parallel_config: ParallelConfig,
}

impl<T> BatchProcessor<T>
where
    T: Send + Sync,
{
    /// Create a new batch processor
    pub fn new(items: Vec<T>, parallel_config: ParallelConfig) -> Self {
        Self {
            items,
            batch_size: parallel_config.chunk_size,
            parallel_config,
        }
    }

    /// Process all batches with a function
    pub fn process<R, F>(&self, processor_fn: F) -> Vec<R>
    where
        R: Send,
        F: Fn(&[T]) -> Vec<R> + Send + Sync + Clone,
    {
        let batches: Vec<_> = self
            .items
            .chunks(self.batch_size)
            .collect();

        if self.parallel_config.enabled {
            batches
                .into_par_iter()
                .flat_map(&processor_fn)
                .collect()
        } else {
            batches
                .into_iter()
                .flat_map(&processor_fn)
                .collect()
        }
    }

    /// Process batches with progress tracking
    pub fn process_with_progress<R, F, P>(
        &self, 
        processor_fn: F, 
        progress_fn: P
    ) -> Vec<R>
    where
        R: Send,
        F: Fn(&[T]) -> Vec<R> + Send + Sync,
        P: Fn(usize, usize) + Send + Sync,
    {
        let batches: Vec<_> = self
            .items
            .chunks(self.batch_size)
            .collect();
        
        let total_batches = batches.len();
        let progress_fn = Arc::new(progress_fn);

        if self.parallel_config.enabled {
            batches
                .into_par_iter()
                .enumerate()
                .flat_map(|(i, batch)| {
                    let result = processor_fn(batch);
                    progress_fn(i + 1, total_batches);
                    result
                })
                .collect()
        } else {
            batches
                .into_iter()
                .enumerate()
                .flat_map(|(i, batch)| {
                    let result = processor_fn(batch);
                    progress_fn(i + 1, total_batches);
                    result
                })
                .collect()
        }
    }
}

/// Memory-safe parallel iterator for large datasets
pub struct MemorySafeIterator<T> {
    items: Vec<T>,
    window_size: usize,
    parallel_enabled: bool,
}

impl<T> MemorySafeIterator<T>
where
    T: Send + Sync,
{
    /// Create a new memory-safe iterator
    pub fn new(items: Vec<T>, window_size: usize, parallel_enabled: bool) -> Self {
        Self {
            items,
            window_size,
            parallel_enabled,
        }
    }

    /// Process windows of data with a function
    pub fn process_windows<R, F>(&self, process_fn: F) -> Vec<R>
    where
        R: Send,
        F: Fn(&[T]) -> R + Send + Sync,
    {
        let windows: Vec<_> = self
            .items
            .chunks(self.window_size)
            .collect();

        if self.parallel_enabled {
            windows.into_par_iter().map(process_fn).collect()
        } else {
            windows.into_iter().map(process_fn).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_parallel_processor_enabled() -> Result<()> {
        let config = ParallelConfig {
            enabled: true,
            max_threads: 0, // Use default threads to avoid pool reinitialization
            chunk_size: 100,
        };
        
        let processor = ParallelProcessor::new(config)?;
        let numbers: Vec<i32> = (1..=10).collect();
        
        let results = processor.par_map(numbers, |x| x * 2);
        let expected: Vec<i32> = (1..=10).map(|x| x * 2).collect();
        
        assert_eq!(results, expected);
        Ok(())
    }

    #[test]
    fn test_parallel_processor_disabled() -> Result<()> {
        let config = ParallelConfig {
            enabled: false,
            max_threads: 0,
            chunk_size: 100,
        };
        
        let processor = ParallelProcessor::new(config)?;
        let numbers: Vec<i32> = (1..=10).collect();
        
        let results = processor.par_map(numbers, |x| x * 2);
        let expected: Vec<i32> = (1..=10).map(|x| x * 2).collect();
        
        assert_eq!(results, expected);
        Ok(())
    }

    #[test]
    fn test_par_filter_map() -> Result<()> {
        let config = ParallelConfig {
            enabled: true,
            max_threads: 0, // Use default threads to avoid pool reinitialization
            chunk_size: 100,
        };
        
        let processor = ParallelProcessor::new(config)?;
        let numbers: Vec<i32> = (1..=10).collect();
        
        let results = processor.par_filter_map(numbers, |x| {
            if x % 2 == 0 {
                Some(x * 2)
            } else {
                None
            }
        });
        
        let expected = vec![4, 8, 12, 16, 20];
        assert_eq!(results, expected);
        Ok(())
    }

    #[test]
    fn test_par_reduce() -> Result<()> {
        let config = ParallelConfig {
            enabled: true,
            max_threads: 0, // Use default threads to avoid pool reinitialization
            chunk_size: 100,
        };
        
        let processor = ParallelProcessor::new(config)?;
        let numbers: Vec<i32> = (1..=10).collect();
        
        let sum = processor.par_reduce(numbers, || 0, |a, b| a + b);
        assert_eq!(sum, 55); // 1+2+...+10 = 55
        Ok(())
    }

    #[test]
    fn test_batch_processor() {
        let config = ParallelConfig {
            enabled: true,
            max_threads: 2,
            chunk_size: 3,
        };
        
        let numbers: Vec<i32> = (1..=10).collect();
        let processor = BatchProcessor::new(numbers, config);
        
        let results = processor.process(|batch| {
            batch.iter().map(|&x| x * 2).collect()
        });
        
        let expected: Vec<i32> = (1..=10).map(|x| x * 2).collect();
        assert_eq!(results.len(), expected.len());
    }

    #[test]
    fn test_batch_processor_with_progress() {
        let config = ParallelConfig {
            enabled: false, // Use sequential for predictable test
            max_threads: 1,
            chunk_size: 3,
        };
        
        let numbers: Vec<i32> = (1..=9).collect(); // 3 batches of 3
        let processor = BatchProcessor::new(numbers, config);
        
        let progress_calls = Arc::new(Mutex::new(Vec::new()));
        let progress_calls_clone = progress_calls.clone();
        
        let results = processor.process_with_progress(
            |batch| batch.iter().map(|&x| x * 2).collect(),
            move |current, total| {
                progress_calls_clone.lock().unwrap().push((current, total));
            }
        );
        
        let progress = progress_calls.lock().unwrap();
        assert_eq!(progress.len(), 3);
        assert_eq!(progress[0], (1, 3));
        assert_eq!(progress[1], (2, 3));
        assert_eq!(progress[2], (3, 3));
        
        assert_eq!(results.len(), 9);
    }

    #[test]
    fn test_memory_safe_iterator() {
        let numbers: Vec<i32> = (1..=10).collect();
        let iterator = MemorySafeIterator::new(numbers, 3, true);
        
        let results = iterator.process_windows(|window| {
            window.iter().sum::<i32>()
        });
        
        // Windows: [1,2,3], [4,5,6], [7,8,9], [10]
        // Sums: 6, 15, 24, 10
        let expected = vec![6, 15, 24, 10];
        assert_eq!(results, expected);
    }
}