use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::cache::CacheConfig;

/// Global configuration for GitCrab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Default time window for analysis (e.g., "90d", "6m", "1y")
    #[serde(default = "default_window")]
    pub default_window: String,

    /// Default number of top results to show
    #[serde(default = "default_top")]
    pub default_top: usize,

    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfig,

    /// Parallel processing configuration
    #[serde(default)]
    pub parallel: ParallelConfig,

    /// Debug configuration
    #[serde(default)]
    pub debug: DebugConfig,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            default_window: default_window(),
            default_top: default_top(),
            cache: CacheConfig::default(),
            parallel: ParallelConfig::default(),
            debug: DebugConfig::default(),
        }
    }
}

/// Parallel processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Enable parallel processing
    #[serde(default = "default_parallel_enabled")]
    pub enabled: bool,

    /// Maximum number of threads (0 = use all available cores)
    #[serde(default)]
    pub max_threads: usize,

    /// Chunk size for parallel processing
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            enabled: default_parallel_enabled(),
            max_threads: 0, // Use all available cores
            chunk_size: default_chunk_size(),
        }
    }
}

/// Debug configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Enable debug mode
    #[serde(default)]
    pub enabled: bool,

    /// Enable timing logs
    #[serde(default)]
    pub timing: bool,

    /// Log level (error, warn, info, debug, trace)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timing: false,
            log_level: default_log_level(),
        }
    }
}

/// Default values
fn default_window() -> String {
    "90d".to_string()
}

fn default_top() -> usize {
    25
}

fn default_parallel_enabled() -> bool {
    true
}

fn default_chunk_size() -> usize {
    1000
}

fn default_log_level() -> String {
    "info".to_string()
}

impl GlobalConfig {
    /// Load configuration from file, falling back to defaults
    pub fn load() -> Self {
        match Self::load_from_file() {
            Ok(config) => config,
            Err(_) => {
                // If loading fails, create and save default config
                let default_config = Self::default();
                if let Err(e) = default_config.save() {
                    eprintln!("Warning: Failed to save default config: {}", e);
                }
                default_config
            }
        }
    }

    /// Load configuration from the config file
    pub fn load_from_file() -> Result<Self> {
        let config_path = get_config_file_path();
        let content = fs::read_to_string(&config_path)
            .context(format!("Failed to read config file: {:?}", config_path))?;

        let config: GlobalConfig =
            toml::from_str(&content).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = get_config_file_path();

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, content)
            .context(format!("Failed to write config file: {:?}", config_path))?;

        Ok(())
    }

    /// Get the path to the config file
    #[allow(dead_code)]
    pub fn config_file_path() -> PathBuf {
        get_config_file_path()
    }

    /// Apply command-line overrides to the configuration
    pub fn with_overrides(mut self, overrides: ConfigOverrides) -> Self {
        if let Some(debug) = overrides.debug {
            self.debug.enabled = debug;
            self.debug.timing = debug; // Enable timing when debug is enabled
        }

        if let Some(cache_enabled) = overrides.cache_enabled {
            self.cache.enabled = cache_enabled;
        }

        if let Some(parallel_enabled) = overrides.parallel_enabled {
            self.parallel.enabled = parallel_enabled;
        }

        if let Some(max_threads) = overrides.max_threads {
            self.parallel.max_threads = max_threads;
        }

        self
    }
}

/// Command-line configuration overrides
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    pub debug: Option<bool>,
    pub cache_enabled: Option<bool>,
    pub parallel_enabled: Option<bool>,
    pub max_threads: Option<usize>,
}

/// Get the path to the global config file
fn get_config_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("crabgit")
        .join("config.toml")
}

/// Parse duration string (e.g., "90d", "6m", "1y") into days
#[allow(dead_code)]
pub fn parse_duration_to_days(duration: &str) -> Result<u64> {
    if duration.is_empty() {
        return Err(anyhow::anyhow!("Duration cannot be empty"));
    }

    let (number_str, unit) = if let Some(last_char) = duration.chars().last() {
        if last_char.is_alphabetic() {
            (&duration[..duration.len() - 1], last_char)
        } else {
            (duration, 'd') // Default to days if no unit specified
        }
    } else {
        return Err(anyhow::anyhow!("Invalid duration format"));
    };

    let number: u64 = number_str
        .parse()
        .context("Failed to parse duration number")?;

    let days = match unit.to_ascii_lowercase() {
        'd' => number,
        'w' => number * 7,
        'm' => number * 30,  // Approximate month
        'y' => number * 365, // Approximate year
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid duration unit: {}. Use d, w, m, or y",
                unit
            ));
        }
    };

    Ok(days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert_eq!(config.default_window, "90d");
        assert_eq!(config.default_top, 25);
        assert!(config.cache.enabled);
        assert!(config.parallel.enabled);
        assert!(!config.debug.enabled);
    }

    #[test]
    fn test_config_serialization() -> Result<()> {
        let config = GlobalConfig::default();
        let serialized = toml::to_string(&config)?;
        let deserialized: GlobalConfig = toml::from_str(&serialized)?;

        assert_eq!(config.default_window, deserialized.default_window);
        assert_eq!(config.default_top, deserialized.default_top);

        Ok(())
    }

    #[test]
    fn test_config_overrides() {
        let config = GlobalConfig::default();

        let overrides = ConfigOverrides {
            debug: Some(true),
            cache_enabled: Some(false),
            parallel_enabled: Some(false),
            max_threads: Some(4),
        };

        let config_with_overrides = config.with_overrides(overrides);

        assert!(config_with_overrides.debug.enabled);
        assert!(config_with_overrides.debug.timing); // Should be enabled with debug
        assert!(!config_with_overrides.cache.enabled);
        assert!(!config_with_overrides.parallel.enabled);
        assert_eq!(config_with_overrides.parallel.max_threads, 4);
    }

    #[test]
    fn test_parse_duration_to_days() -> Result<()> {
        assert_eq!(parse_duration_to_days("7d")?, 7);
        assert_eq!(parse_duration_to_days("2w")?, 14);
        assert_eq!(parse_duration_to_days("3m")?, 90);
        assert_eq!(parse_duration_to_days("1y")?, 365);
        assert_eq!(parse_duration_to_days("30")?, 30); // Default to days

        // Case insensitive
        assert_eq!(parse_duration_to_days("7D")?, 7);
        assert_eq!(parse_duration_to_days("2W")?, 14);

        Ok(())
    }

    #[test]
    fn test_parse_duration_errors() {
        assert!(parse_duration_to_days("").is_err());
        assert!(parse_duration_to_days("abc").is_err());
        assert!(parse_duration_to_days("7x").is_err());
    }
}
