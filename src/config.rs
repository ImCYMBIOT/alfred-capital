use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use crate::error::ConfigError;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub rpc: RpcConfig,
    pub database: DatabaseConfig,
    pub processing: ProcessingConfig,
    pub api: ApiConfig,
    pub logging: LoggingConfig,
}

/// RPC client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Polygon RPC endpoint URL
    pub endpoint: String,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial retry delay in seconds
    pub retry_delay_seconds: u64,
    /// Maximum retry delay in seconds
    pub max_retry_delay_seconds: u64,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// SQLite database file path
    pub path: String,
    /// Connection pool size
    pub connection_pool_size: u32,
    /// Enable WAL mode for better concurrency
    pub enable_wal_mode: bool,
    /// Database busy timeout in milliseconds
    pub busy_timeout_ms: u32,
}

/// Block processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Block polling interval in seconds
    pub poll_interval_seconds: u64,
    /// Batch size for processing multiple blocks
    pub batch_size: u32,
    /// POL token contract address on Polygon
    pub pol_token_address: String,
    /// Maximum blocks to process in a single batch
    pub max_blocks_per_batch: u32,
}

/// API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Enable HTTP API server
    pub enabled: bool,
    /// Server port
    pub port: u16,
    /// Server host/bind address
    pub host: String,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Maximum concurrent connections
    pub max_connections: u32,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    pub level: String,
    /// Log format (json, pretty)
    pub format: String,
    /// Enable file logging
    pub file_enabled: bool,
    /// Log file path (if file logging enabled)
    pub file_path: Option<String>,
    /// Maximum log file size in MB
    pub max_file_size_mb: u64,
    /// Number of log files to keep
    pub max_files: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            rpc: RpcConfig::default(),
            database: DatabaseConfig::default(),
            processing: ProcessingConfig::default(),
            api: ApiConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://polygon-rpc.com/".to_string(),
            timeout_seconds: 30,
            max_retries: 5,
            retry_delay_seconds: 2,
            max_retry_delay_seconds: 60,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "./blockchain.db".to_string(),
            connection_pool_size: 10,
            enable_wal_mode: true,
            busy_timeout_ms: 5000,
        }
    }
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            poll_interval_seconds: 2,
            batch_size: 100,
            // This is a placeholder - needs to be updated with actual POL token address
            pol_token_address: "0x455e53bd25bfb4ed405b8b8c2db7ab87cd0a7e9f".to_string(),
            max_blocks_per_batch: 10,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 8080,
            host: "127.0.0.1".to_string(),
            request_timeout_seconds: 30,
            max_connections: 100,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file_enabled: false,
            file_path: None,
            max_file_size_mb: 100,
            max_files: 5,
        }
    }
}

impl AppConfig {
    /// Load configuration from file and environment variables
    /// Environment variables take precedence over file values
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Self::load_from_file().unwrap_or_default();
        config.apply_env_overrides()?;
        config.validate()?;
        Ok(config)
    }
    
    /// Load configuration from TOML file
    pub fn load_from_file() -> Result<Self, ConfigError> {
        let config_path = env::var("CONFIG_FILE").unwrap_or_else(|_| "config.toml".to_string());
        
        if !Path::new(&config_path).exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&config_path)
            .map_err(|_| ConfigError::FileNotFound(config_path.clone()))?;
        let config: AppConfig = toml::from_str(&content)
            .map_err(|e| ConfigError::Parsing(e.to_string()))?;
        Ok(config)
    }
    
    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        // RPC configuration
        if let Ok(endpoint) = env::var("POLYGON_RPC_URL") {
            self.rpc.endpoint = endpoint;
        }
        if let Ok(timeout) = env::var("RPC_TIMEOUT_SECONDS") {
            self.rpc.timeout_seconds = timeout.parse()
                .map_err(|_| ConfigError::InvalidValue {
                    key: "RPC_TIMEOUT_SECONDS".to_string(),
                    value: timeout,
                })?;
        }
        if let Ok(retries) = env::var("RPC_MAX_RETRIES") {
            self.rpc.max_retries = retries.parse()
                .map_err(|_| ConfigError::InvalidValue {
                    key: "RPC_MAX_RETRIES".to_string(),
                    value: retries,
                })?;
        }
        
        // Database configuration
        if let Ok(path) = env::var("DATABASE_PATH") {
            self.database.path = path;
        }
        if let Ok(pool_size) = env::var("DATABASE_POOL_SIZE") {
            self.database.connection_pool_size = pool_size.parse()
                .map_err(|_| ConfigError::InvalidValue {
                    key: "DATABASE_POOL_SIZE".to_string(),
                    value: pool_size,
                })?;
        }
        if let Ok(wal_mode) = env::var("DATABASE_WAL_MODE") {
            self.database.enable_wal_mode = wal_mode.parse()
                .map_err(|_| ConfigError::InvalidValue {
                    key: "DATABASE_WAL_MODE".to_string(),
                    value: wal_mode,
                })?;
        }
        
        // Processing configuration
        if let Ok(interval) = env::var("BLOCK_POLL_INTERVAL") {
            self.processing.poll_interval_seconds = interval.parse()
                .map_err(|_| ConfigError::InvalidValue {
                    key: "BLOCK_POLL_INTERVAL".to_string(),
                    value: interval,
                })?;
        }
        if let Ok(batch_size) = env::var("PROCESSING_BATCH_SIZE") {
            self.processing.batch_size = batch_size.parse()
                .map_err(|_| ConfigError::InvalidValue {
                    key: "PROCESSING_BATCH_SIZE".to_string(),
                    value: batch_size,
                })?;
        }
        if let Ok(token_address) = env::var("POL_TOKEN_ADDRESS") {
            self.processing.pol_token_address = token_address;
        }
        
        // API configuration
        if let Ok(enabled) = env::var("API_ENABLED") {
            self.api.enabled = enabled.parse()
                .map_err(|_| ConfigError::InvalidValue {
                    key: "API_ENABLED".to_string(),
                    value: enabled,
                })?;
        }
        if let Ok(port) = env::var("API_PORT") {
            self.api.port = port.parse()
                .map_err(|_| ConfigError::InvalidValue {
                    key: "API_PORT".to_string(),
                    value: port,
                })?;
        }
        if let Ok(host) = env::var("API_HOST") {
            self.api.host = host;
        }
        
        // Logging configuration
        if let Ok(level) = env::var("LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(format) = env::var("LOG_FORMAT") {
            self.logging.format = format;
        }
        if let Ok(file_enabled) = env::var("LOG_FILE_ENABLED") {
            self.logging.file_enabled = file_enabled.parse()
                .map_err(|_| ConfigError::InvalidValue {
                    key: "LOG_FILE_ENABLED".to_string(),
                    value: file_enabled,
                })?;
        }
        if let Ok(file_path) = env::var("LOG_FILE_PATH") {
            self.logging.file_path = Some(file_path);
        }
        
        Ok(())
    }
    
    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate RPC endpoint URL
        if !self.rpc.endpoint.starts_with("http://") && !self.rpc.endpoint.starts_with("https://") {
            return Err(ConfigError::InvalidUrl(self.rpc.endpoint.clone()));
        }
        
        // Validate timeout values
        if self.rpc.timeout_seconds == 0 || self.rpc.timeout_seconds > 300 {
            return Err(ConfigError::InvalidValue {
                key: "rpc.timeout_seconds".to_string(),
                value: self.rpc.timeout_seconds.to_string(),
            });
        }
        
        // Validate retry configuration
        if self.rpc.max_retries == 0 || self.rpc.max_retries > 20 {
            return Err(ConfigError::InvalidValue {
                key: "rpc.max_retries".to_string(),
                value: self.rpc.max_retries.to_string(),
            });
        }
        
        // Validate poll interval
        if self.processing.poll_interval_seconds == 0 || self.processing.poll_interval_seconds > 300 {
            return Err(ConfigError::InvalidValue {
                key: "processing.poll_interval_seconds".to_string(),
                value: self.processing.poll_interval_seconds.to_string(),
            });
        }
        
        // Validate batch size
        if self.processing.batch_size == 0 || self.processing.batch_size > 1000 {
            return Err(ConfigError::InvalidValue {
                key: "processing.batch_size".to_string(),
                value: self.processing.batch_size.to_string(),
            });
        }
        
        // Validate POL token address format (basic hex check)
        if !self.processing.pol_token_address.starts_with("0x") || 
           self.processing.pol_token_address.len() != 42 {
            return Err(ConfigError::InvalidValue {
                key: "processing.pol_token_address".to_string(),
                value: self.processing.pol_token_address.clone(),
            });
        }
        
        // Validate API port
        if self.api.port == 0 {
            return Err(ConfigError::InvalidValue {
                key: "api.port".to_string(),
                value: self.api.port.to_string(),
            });
        }
        
        // Validate log level
        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(ConfigError::InvalidValue {
                key: "logging.level".to_string(),
                value: self.logging.level.clone(),
            });
        }
        
        // Validate log format
        let valid_formats = ["json", "pretty"];
        if !valid_formats.contains(&self.logging.format.as_str()) {
            return Err(ConfigError::InvalidValue {
                key: "logging.format".to_string(),
                value: self.logging.format.clone(),
            });
        }
        
        // Validate database path is not empty
        if self.database.path.trim().is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "database.path".to_string(),
                value: self.database.path.clone(),
            });
        }
        
        Ok(())
    }
    
    /// Generate a sample configuration file
    pub fn generate_sample_config() -> Result<String, ConfigError> {
        let config = Self::default();
        toml::to_string_pretty(&config)
            .map_err(|e| ConfigError::Parsing(e.to_string()))
    }
    
    /// Save configuration to file
    pub fn save_to_file(&self, path: &str) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Parsing(e.to_string()))?;
        fs::write(path, content)
            .map_err(|_| ConfigError::FileNotFound(path.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.rpc.endpoint, "https://polygon-rpc.com/");
        assert_eq!(config.rpc.timeout_seconds, 30);
        assert_eq!(config.database.path, "./blockchain.db");
        assert_eq!(config.processing.poll_interval_seconds, 2);
        assert_eq!(config.api.port, 8080);
        assert_eq!(config.logging.level, "info");
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid RPC endpoint
        config.rpc.endpoint = "invalid-url".to_string();
        assert!(config.validate().is_err());
        
        // Reset and test invalid timeout
        config = AppConfig::default();
        config.rpc.timeout_seconds = 0;
        assert!(config.validate().is_err());
        
        // Reset and test invalid poll interval
        config = AppConfig::default();
        config.processing.poll_interval_seconds = 0;
        assert!(config.validate().is_err());
        
        // Reset and test invalid token address
        config = AppConfig::default();
        config.processing.pol_token_address = "invalid".to_string();
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_env_overrides() {
        // Set environment variables
        env::set_var("POLYGON_RPC_URL", "https://test-rpc.com/");
        env::set_var("DATABASE_PATH", "/tmp/test.db");
        env::set_var("BLOCK_POLL_INTERVAL", "5");
        env::set_var("API_PORT", "9090");
        env::set_var("LOG_LEVEL", "debug");
        
        let mut config = AppConfig::default();
        config.apply_env_overrides().unwrap();
        
        assert_eq!(config.rpc.endpoint, "https://test-rpc.com/");
        assert_eq!(config.database.path, "/tmp/test.db");
        assert_eq!(config.processing.poll_interval_seconds, 5);
        assert_eq!(config.api.port, 9090);
        assert_eq!(config.logging.level, "debug");
        
        // Clean up
        env::remove_var("POLYGON_RPC_URL");
        env::remove_var("DATABASE_PATH");
        env::remove_var("BLOCK_POLL_INTERVAL");
        env::remove_var("API_PORT");
        env::remove_var("LOG_LEVEL");
    }
    
    #[test]
    fn test_invalid_env_values() {
        env::set_var("RPC_TIMEOUT_SECONDS", "invalid");
        
        let mut config = AppConfig::default();
        let result = config.apply_env_overrides();
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ConfigError::InvalidValue { .. }));
        
        env::remove_var("RPC_TIMEOUT_SECONDS");
    }
    
    #[test]
    fn test_config_file_loading() {
        let config_content = r#"
[rpc]
endpoint = "https://custom-rpc.com/"
timeout_seconds = 45
max_retries = 3
retry_delay_seconds = 1
max_retry_delay_seconds = 30

[database]
path = "/custom/path/db.sqlite"
connection_pool_size = 5
enable_wal_mode = false
busy_timeout_ms = 3000

[processing]
poll_interval_seconds = 3
batch_size = 50
pol_token_address = "0x1234567890123456789012345678901234567890"
max_blocks_per_batch = 5

[api]
enabled = false
port = 3000
host = "0.0.0.0"
request_timeout_seconds = 15
max_connections = 50

[logging]
level = "warn"
format = "json"
file_enabled = true
file_path = "/tmp/test.log"
max_file_size_mb = 50
max_files = 3
"#;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp_file, config_content.as_bytes()).unwrap();
        
        env::set_var("CONFIG_FILE", temp_file.path().to_str().unwrap());
        
        let config = AppConfig::load_from_file().unwrap();
        
        assert_eq!(config.rpc.endpoint, "https://custom-rpc.com/");
        assert_eq!(config.rpc.timeout_seconds, 45);
        assert_eq!(config.rpc.max_retries, 3);
        assert_eq!(config.database.path, "/custom/path/db.sqlite");
        assert_eq!(config.database.connection_pool_size, 5);
        assert!(!config.database.enable_wal_mode);
        assert_eq!(config.processing.poll_interval_seconds, 3);
        assert_eq!(config.processing.batch_size, 50);
        assert_eq!(config.processing.pol_token_address, "0x1234567890123456789012345678901234567890");
        assert!(!config.api.enabled);
        assert_eq!(config.api.port, 3000);
        assert_eq!(config.api.host, "0.0.0.0");
        assert_eq!(config.logging.level, "warn");
        assert_eq!(config.logging.format, "json");
        assert!(config.logging.file_enabled);
        assert_eq!(config.logging.file_path, Some("/tmp/test.log".to_string()));
        
        env::remove_var("CONFIG_FILE");
    }
    
    #[test]
    fn test_generate_sample_config() {
        let sample = AppConfig::generate_sample_config().unwrap();
        assert!(sample.contains("[rpc]"));
        assert!(sample.contains("[database]"));
        assert!(sample.contains("[processing]"));
        assert!(sample.contains("[api]"));
        assert!(sample.contains("[logging]"));
    }
    
    #[test]
    fn test_config_roundtrip() {
        let original_config = AppConfig::default();
        let toml_string = toml::to_string_pretty(&original_config).unwrap();
        let parsed_config: AppConfig = toml::from_str(&toml_string).unwrap();
        
        // Compare key fields to ensure roundtrip works
        assert_eq!(original_config.rpc.endpoint, parsed_config.rpc.endpoint);
        assert_eq!(original_config.database.path, parsed_config.database.path);
        assert_eq!(original_config.processing.poll_interval_seconds, parsed_config.processing.poll_interval_seconds);
    }
}