use std::{env, sync::OnceLock};

use anyhow::Result;
use dotenv::dotenv;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Server host to listen on
    pub server_host: String,
    /// Server port to listen on
    pub server_port: u16,
    /// Database file path
    pub database_path: String,
    /// Access token for API authentication
    pub access_token: String,
    /// Font path for BMP generation
    pub font_path: String,
    /// Image refresh rate in seconds
    pub refresh_rate: u32,
}

// Global config instance
static CONFIG: OnceLock<Config> = OnceLock::new();

impl Config {
    /// Initialize configuration from environment variables
    pub fn init() -> Result<&'static Config> {
        // Load .env file if it exists
        let _ = dotenv();

        // Get configuration from environment or use defaults
        let config = Config {
            server_host: get_env_or_default("SERVER_HOST", "127.0.0.1".to_string()),
            server_port: get_env_or_default("SERVER_PORT", 8080),
            database_path: get_env_or_default("DATABASE_PATH", "devices.db".to_string()),
            access_token: get_env_or("ACCESS_TOKEN")
                .ok_or_else(|| anyhow::anyhow!("ACCESS_TOKEN environment variable is required"))?,
            font_path: get_env_or_default("FONT_PATH", "assets/fonts/BlockKie.ttf".to_string()),
            refresh_rate: get_env_or_default("REFRESH_RATE", 200),
        };

        // Store in global state
        CONFIG.get_or_init(|| config);
        Ok(CONFIG.get().unwrap())
    }

    /// Get global configuration, initializing if necessary
    pub fn get() -> Result<&'static Config> {
        if let Some(config) = CONFIG.get() {
            Ok(config)
        } else {
            // In test environment, provide a default config
            if cfg!(test) {
                let test_config = Config {
                    server_host: "127.0.0.1".to_string(),
                    server_port: 8080,
                    database_path: "test_devices.db".to_string(),
                    access_token: std::env::var("ACCESS_TOKEN")
                        .unwrap_or_else(|_| "your-secret-access-token".to_string()),
                    font_path: "assets/fonts/BlockKie.ttf".to_string(),
                    refresh_rate: 200,
                };
                CONFIG.get_or_init(|| test_config);
                Ok(CONFIG.get().unwrap())
            } else {
                Config::init()
            }
        }
    }
}

/// Get an environment variable or return a default value
fn get_env_or_default<T: std::str::FromStr>(key: &str, default: T) -> T {
    get_env_or(key).unwrap_or(default)
}

/// Get an environment variable parsed to the desired type
fn get_env_or<T: std::str::FromStr>(key: &str) -> Option<T> {
    env::var(key).ok().and_then(|val| val.parse().ok())
}
