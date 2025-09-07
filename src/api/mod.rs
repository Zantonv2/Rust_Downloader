pub mod integration;
pub mod config;

use crate::errors::{Result, SpotifyDownloaderError};
use crate::downloader::{spotify::SpotifyClient, itunes::ItunesClient};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Centralized API manager for all external services
pub struct ApiManager {
    client: Client,
    spotify: Arc<RwLock<SpotifyClient>>,
    itunes: Arc<RwLock<ItunesClient>>,
    rate_limits: Arc<RwLock<HashMap<String, RateLimit>>>,
}

/// Rate limiting information for APIs
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
    pub current_minute_requests: u32,
    pub current_hour_requests: u32,
    pub current_day_requests: u32,
    pub last_reset_minute: std::time::SystemTime,
    pub last_reset_hour: std::time::SystemTime,
    pub last_reset_day: std::time::SystemTime,
}

/// API configuration for all services
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub spotify_client_id: String,
    pub spotify_client_secret: String,
    pub lastfm_api_key: Option<String>,
    pub musicbrainz_user_agent: String,
    pub request_timeout: std::time::Duration,
    pub max_retries: u32,
    pub retry_delay: std::time::Duration,
    pub proxy_config: Option<crate::config::ProxyConfig>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            spotify_client_id: String::new(),
            spotify_client_secret: String::new(),
            lastfm_api_key: None,
            musicbrainz_user_agent: "SpotifyDownloader/1.0".to_string(),
            request_timeout: std::time::Duration::from_secs(30),
            max_retries: 3,
            retry_delay: std::time::Duration::from_millis(500),
            proxy_config: None,
        }
    }
}

impl ApiManager {
    /// Create a new API manager with configuration
    pub fn new(config: ApiConfig) -> Self {
        let mut client_builder = Client::builder()
            .timeout(config.request_timeout)
            .user_agent(&config.musicbrainz_user_agent);

        // Add proxy support if configured
        if let Some(proxy_config) = &config.proxy_config {
            if proxy_config.enabled {
                let proxy_url = if let (Some(username), Some(password)) = (&proxy_config.username, &proxy_config.password) {
                    format!("http://{}:{}@{}:{}", username, password, proxy_config.host, proxy_config.port)
                } else {
                    format!("http://{}:{}", proxy_config.host, proxy_config.port)
                };
                
                if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
                    client_builder = client_builder.proxy(proxy);
                }
            }
        }

        let client = client_builder
            .build()
            .expect("Failed to create HTTP client");

        let spotify = Arc::new(RwLock::new(SpotifyClient::new_with_client(
            config.spotify_client_id.clone(),
            config.spotify_client_secret.clone(),
            client.clone(),
        )));

        let itunes = Arc::new(RwLock::new(ItunesClient::new_with_client(client.clone())));

        let mut rate_limits = HashMap::new();
        rate_limits.insert("spotify".to_string(), RateLimit::new(100, 1000, 10000));
        rate_limits.insert("youtube".to_string(), RateLimit::new(60, 1000, 10000));
        rate_limits.insert("soundcloud".to_string(), RateLimit::new(200, 2000, 20000));
        rate_limits.insert("itunes".to_string(), RateLimit::new(20, 1000, 10000));
        rate_limits.insert("lastfm".to_string(), RateLimit::new(5, 1000, 10000));
        rate_limits.insert("musicbrainz".to_string(), RateLimit::new(1, 100, 1000));

        Self {
            client,
            spotify,
            itunes,
            rate_limits: Arc::new(RwLock::new(rate_limits)),
        }
    }

    /// Get Spotify API instance
    pub async fn spotify(&self) -> Arc<RwLock<SpotifyClient>> {
        self.spotify.clone()
    }

    /// Get iTunes API instance
    pub async fn itunes(&self) -> Arc<RwLock<ItunesClient>> {
        self.itunes.clone()
    }

    /// Check if we can make a request to the specified API
    pub async fn can_make_request(&self, api_name: &str) -> Result<bool> {
        let mut limits = self.rate_limits.write().await;
        if let Some(limit) = limits.get_mut(api_name) {
            limit.update_limits();
            Ok(limit.can_make_request())
        } else {
            Err(SpotifyDownloaderError::Api(format!("Unknown API: {}", api_name)))
        }
    }

    /// Record a request for rate limiting
    pub async fn record_request(&self, api_name: &str) -> Result<()> {
        let mut limits = self.rate_limits.write().await;
        if let Some(limit) = limits.get_mut(api_name) {
            limit.record_request();
            Ok(())
        } else {
            Err(SpotifyDownloaderError::Api(format!("Unknown API: {}", api_name)))
        }
    }

    /// Make a rate-limited request
    pub async fn make_request<F, T>(&self, api_name: &str, request: F) -> Result<T>
    where
        F: FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        // Check rate limits
        if !self.can_make_request(api_name).await? {
            return Err(SpotifyDownloaderError::Api(format!(
                "Rate limit exceeded for {}",
                api_name
            )));
        }

        // Make the request
        let result = request().await;

        // Record the request regardless of success/failure
        let _ = self.record_request(api_name).await;

        result
    }

    /// Get HTTP client for direct use
    pub fn client(&self) -> &Client {
        &self.client
    }
}

impl RateLimit {
    pub fn new(per_minute: u32, per_hour: u32, per_day: u32) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            requests_per_minute: per_minute,
            requests_per_hour: per_hour,
            requests_per_day: per_day,
            current_minute_requests: 0,
            current_hour_requests: 0,
            current_day_requests: 0,
            last_reset_minute: now,
            last_reset_hour: now,
            last_reset_day: now,
        }
    }

    pub fn update_limits(&mut self) {
        let now = std::time::SystemTime::now();

        // Reset minute counter if needed
        if now.duration_since(self.last_reset_minute).unwrap_or_default() >= std::time::Duration::from_secs(60) {
            self.current_minute_requests = 0;
            self.last_reset_minute = now;
        }

        // Reset hour counter if needed
        if now.duration_since(self.last_reset_hour).unwrap_or_default() >= std::time::Duration::from_secs(3600) {
            self.current_hour_requests = 0;
            self.last_reset_hour = now;
        }

        // Reset day counter if needed
        if now.duration_since(self.last_reset_day).unwrap_or_default() >= std::time::Duration::from_secs(86400) {
            self.current_day_requests = 0;
            self.last_reset_day = now;
        }
    }

    pub fn can_make_request(&self) -> bool {
        self.current_minute_requests < self.requests_per_minute
            && self.current_hour_requests < self.requests_per_hour
            && self.current_day_requests < self.requests_per_day
    }

    pub fn record_request(&mut self) {
        self.current_minute_requests += 1;
        self.current_hour_requests += 1;
        self.current_day_requests += 1;
    }
}

/// Global API manager instance
static mut API_MANAGER: Option<ApiManager> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Initialize the global API manager
pub fn init_api_manager(config: ApiConfig) {
    unsafe {
        INIT.call_once(|| {
            API_MANAGER = Some(ApiManager::new(config));
        });
    }
}

/// Get the global API manager instance
pub fn get_api_manager() -> Result<&'static ApiManager> {
    unsafe {
        API_MANAGER.as_ref().ok_or_else(|| {
            SpotifyDownloaderError::Api("API manager not initialized. Call init_api_manager() first.".to_string())
        })
    }
}
