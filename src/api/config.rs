use super::ApiConfig;
use crate::errors::{Result, SpotifyDownloaderError};
use std::env;

/// Load API configuration from environment variables and config files
#[allow(dead_code)]
pub struct ApiConfigLoader;

impl ApiConfigLoader {
    /// Load API configuration from environment variables
    pub fn from_env() -> ApiConfig {
        ApiConfig {
            spotify_client_id: env::var("SPOTIFY_CLIENT_ID").unwrap_or_default(),
            spotify_client_secret: env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default(),
            lastfm_api_key: env::var("LASTFM_API_KEY").ok(),
            musicbrainz_user_agent: env::var("MUSICBRAINZ_USER_AGENT")
                .unwrap_or_else(|_| "SpotifyDownloader/1.0".to_string()),
            request_timeout: std::time::Duration::from_secs(
                env::var("API_REQUEST_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30)
            ),
            max_retries: env::var("API_MAX_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            retry_delay: std::time::Duration::from_millis(
                env::var("API_RETRY_DELAY")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(500)
            ),
            proxy_config: None,
        }
    }

    /// Load API configuration from a TOML config file
    pub fn from_file(path: &str) -> Result<ApiConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to read config file: {}", e)))?;
        
        let config: toml::Value = toml::from_str(&content)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to parse config file: {}", e)))?;

        let api_section = config.get("api")
            .ok_or_else(|| SpotifyDownloaderError::Config("Missing [api] section in config file".to_string()))?;

        Ok(ApiConfig {
            spotify_client_id: api_section.get("spotify_client_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            spotify_client_secret: api_section.get("spotify_client_secret")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            lastfm_api_key: api_section.get("lastfm_api_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            musicbrainz_user_agent: api_section.get("musicbrainz_user_agent")
                .and_then(|v| v.as_str())
                .unwrap_or("SpotifyDownloader/1.0")
                .to_string(),
            request_timeout: std::time::Duration::from_secs(
                api_section.get("request_timeout")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(30) as u64
            ),
            max_retries: api_section.get("max_retries")
                .and_then(|v| v.as_integer())
                .unwrap_or(3) as u32,
            retry_delay: std::time::Duration::from_millis(
                api_section.get("retry_delay")
                    .and_then(|v| v.as_integer())
                    .unwrap_or(500) as u64
            ),
            proxy_config: None,
        })
    }

    /// Load API configuration with fallback priority:
    /// 1. Environment variables
    /// 2. Config file (if exists)
    /// 3. Default values
    pub fn load() -> Result<ApiConfig> {
        // First try environment variables
        let env_config = Self::from_env();
        
        // If we have at least one API key from env, use env config
        if !env_config.spotify_client_id.is_empty() 
            || !env_config.spotify_client_secret.is_empty() 
            || env_config.lastfm_api_key.is_some() {
            return Ok(env_config);
        }

        // Try to load from config file
        let config_paths = [
            "config.toml",
            "spotify-downloader.toml",
            &format!("{}/.config/spotify-downloader/config.toml", 
                env::var("HOME").unwrap_or_else(|_| ".".to_string())),
        ];

        for path in &config_paths {
            if std::path::Path::new(path).exists() {
                match Self::from_file(path) {
                    Ok(config) => return Ok(config),
                    Err(e) => {
                        eprintln!("Warning: Failed to load config from {}: {}", path, e);
                        continue;
                    }
                }
            }
        }

        // Fall back to environment variables (even if empty)
        Ok(env_config)
    }
}

/// Example TOML configuration file content
#[allow(dead_code)]
pub const EXAMPLE_CONFIG: &str = r#"
[api]
spotify_client_id = "your_spotify_client_id"
spotify_client_secret = "your_spotify_client_secret"
lastfm_api_key = "your_lastfm_api_key"
musicbrainz_user_agent = "SpotifyDownloader/1.0"
request_timeout = 30
max_retries = 3
retry_delay = 500
"#;
