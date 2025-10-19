use anyhow::{Context, Result};
use git2::Oid;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,
    /// Cache directory path
    pub cache_dir: PathBuf,
    /// Maximum cache size in MB
    pub max_size_mb: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_dir: get_default_cache_dir(),
            max_size_mb: 100, // 100MB default
        }
    }
}

/// Cached commit statistics
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCommitStats {
    pub oid: String,
    pub timestamp: u64,
    pub additions: usize,
    pub deletions: usize,
    pub files_changed: usize,
}

/// Cache manager for commit statistics
#[allow(dead_code)]
pub struct CommitStatsCache {
    config: CacheConfig,
    memory_cache: HashMap<Oid, CachedCommitStats>,
}

#[allow(dead_code)]
impl CommitStatsCache {
    pub fn new(config: CacheConfig) -> Result<Self> {
        if config.enabled {
            fs::create_dir_all(&config.cache_dir).context("Failed to create cache directory")?;
        }

        Ok(Self {
            config,
            memory_cache: HashMap::new(),
        })
    }

    /// Get commit stats from cache
    pub fn get(&self, oid: &Oid) -> Option<CachedCommitStats> {
        if !self.config.enabled {
            return None;
        }

        // Check memory cache first
        if let Some(stats) = self.memory_cache.get(oid) {
            return Some(stats.clone());
        }

        // Check disk cache
        self.load_from_disk(oid).ok()
    }

    /// Store commit stats in cache
    pub fn put(&mut self, stats: CachedCommitStats) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let oid = git2::Oid::from_str(&stats.oid).context("Invalid OID in cached stats")?;

        // Store in memory cache
        self.memory_cache.insert(oid, stats.clone());

        // Store on disk
        self.save_to_disk(&stats)?;

        Ok(())
    }

    /// Clear all cached data
    pub fn clear(&mut self) -> Result<()> {
        self.memory_cache.clear();

        if self.config.cache_dir.exists() {
            fs::remove_dir_all(&self.config.cache_dir)?;
            fs::create_dir_all(&self.config.cache_dir)?;
        }

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let disk_entries = self.count_disk_entries();
        let disk_size_bytes = self.calculate_disk_size();

        CacheStats {
            memory_entries: self.memory_cache.len(),
            disk_entries,
            disk_size_bytes,
            enabled: self.config.enabled,
        }
    }

    fn load_from_disk(&self, oid: &Oid) -> Result<CachedCommitStats> {
        let cache_file = self.get_cache_file_path(oid);
        let data = fs::read_to_string(&cache_file).context("Failed to read cache file")?;

        let stats: CachedCommitStats =
            serde_json::from_str(&data).context("Failed to deserialize cache data")?;

        Ok(stats)
    }

    fn save_to_disk(&self, stats: &CachedCommitStats) -> Result<()> {
        let oid = git2::Oid::from_str(&stats.oid)?;
        let cache_file = self.get_cache_file_path(&oid);

        // Create parent directory if it doesn't exist
        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let data = serde_json::to_string(stats).context("Failed to serialize cache data")?;

        fs::write(&cache_file, data).context("Failed to write cache file")?;

        Ok(())
    }

    fn get_cache_file_path(&self, oid: &Oid) -> PathBuf {
        let oid_str = oid.to_string();
        let prefix = &oid_str[0..2];
        let suffix = &oid_str[2..];

        self.config
            .cache_dir
            .join(prefix)
            .join(format!("{}.json", suffix))
    }

    fn count_disk_entries(&self) -> usize {
        if !self.config.cache_dir.exists() {
            return 0;
        }

        walkdir::WalkDir::new(&self.config.cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "json")
            })
            .count()
    }

    fn calculate_disk_size(&self) -> u64 {
        if !self.config.cache_dir.exists() {
            return 0;
        }

        walkdir::WalkDir::new(&self.config.cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    }
}

/// Cache statistics
#[allow(dead_code)]
#[derive(Debug)]
pub struct CacheStats {
    pub memory_entries: usize,
    pub disk_entries: usize,
    pub disk_size_bytes: u64,
    pub enabled: bool,
}

/// Get default cache directory
pub fn get_default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("crabgit")
        .join("commit_stats")
}

/// Generate cache key from repository path and configuration
#[allow(dead_code)]
pub fn generate_cache_key(repo_path: &str, config_hash: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(repo_path.as_bytes());
    hasher.update(config_hash.to_le_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_size_mb, 100);
        assert!(config.cache_dir.to_string_lossy().contains("crabgit"));
    }

    #[test]
    fn test_cache_put_get() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = CacheConfig {
            enabled: true,
            cache_dir: temp_dir.path().to_path_buf(),
            max_size_mb: 10,
        };

        let mut cache = CommitStatsCache::new(config)?;
        let oid = git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678")?;

        let stats = CachedCommitStats {
            oid: oid.to_string(),
            timestamp: 1234567890,
            additions: 100,
            deletions: 50,
            files_changed: 5,
        };

        cache.put(stats.clone())?;

        let retrieved = cache.get(&oid);
        assert!(retrieved.is_some());

        let retrieved_stats = retrieved.unwrap();
        assert_eq!(retrieved_stats.oid, stats.oid);
        assert_eq!(retrieved_stats.additions, stats.additions);
        assert_eq!(retrieved_stats.deletions, stats.deletions);

        Ok(())
    }

    #[test]
    fn test_cache_disabled() -> Result<()> {
        let config = CacheConfig {
            enabled: false,
            cache_dir: PathBuf::from("/tmp/test"),
            max_size_mb: 10,
        };

        let mut cache = CommitStatsCache::new(config)?;
        let oid = git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678")?;

        let stats = CachedCommitStats {
            oid: oid.to_string(),
            timestamp: 1234567890,
            additions: 100,
            deletions: 50,
            files_changed: 5,
        };

        cache.put(stats)?;
        let retrieved = cache.get(&oid);
        assert!(retrieved.is_none());

        Ok(())
    }

    #[test]
    fn test_cache_stats() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = CacheConfig {
            enabled: true,
            cache_dir: temp_dir.path().to_path_buf(),
            max_size_mb: 10,
        };

        let cache = CommitStatsCache::new(config)?;
        let stats = cache.stats();

        assert!(stats.enabled);
        assert_eq!(stats.memory_entries, 0);
        assert_eq!(stats.disk_entries, 0);
        assert_eq!(stats.disk_size_bytes, 0);

        Ok(())
    }

    #[test]
    fn test_generate_cache_key() {
        let key1 = generate_cache_key("/path/to/repo1", 12345);
        let key2 = generate_cache_key("/path/to/repo2", 12345);
        let key3 = generate_cache_key("/path/to/repo1", 54321);

        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key2, key3);

        // Same inputs should produce same key
        let key1_again = generate_cache_key("/path/to/repo1", 12345);
        assert_eq!(key1, key1_again);
    }
}
