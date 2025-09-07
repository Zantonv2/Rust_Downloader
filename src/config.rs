use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::errors::{Result, SpotifyDownloaderError};

/// Supported audio formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    M4a,
    Flac,
    Wav,
}

impl std::fmt::Display for AudioFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioFormat::Mp3 => write!(f, "mp3"),
            AudioFormat::M4a => write!(f, "m4a"),
            AudioFormat::Flac => write!(f, "flac"),
            AudioFormat::Wav => write!(f, "wav"),
        }
    }
}

impl std::str::FromStr for AudioFormat {
    type Err = SpotifyDownloaderError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "mp3" => Ok(AudioFormat::Mp3),
            "m4a" => Ok(AudioFormat::M4a),
            "flac" => Ok(AudioFormat::Flac),
            "wav" => Ok(AudioFormat::Wav),
            _ => Err(SpotifyDownloaderError::InvalidFormat(s.to_string())),
        }
    }
}

/// Supported bitrates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bitrate {
    Kbps128,
    Kbps192,
    Kbps256,
    Kbps320,
}

impl std::fmt::Display for Bitrate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bitrate::Kbps128 => write!(f, "128"),
            Bitrate::Kbps192 => write!(f, "192"),
            Bitrate::Kbps256 => write!(f, "256"),
            Bitrate::Kbps320 => write!(f, "320"),
        }
    }
}

impl std::str::FromStr for Bitrate {
    type Err = SpotifyDownloaderError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "128" => Ok(Bitrate::Kbps128),
            "192" => Ok(Bitrate::Kbps192),
            "256" => Ok(Bitrate::Kbps256),
            "320" => Ok(Bitrate::Kbps320),
            _ => Err(SpotifyDownloaderError::InvalidBitrate(s.to_string())),
        }
    }
}

impl Bitrate {
    pub fn as_u32(&self) -> u32 {
        match self {
            Bitrate::Kbps128 => 128,
            Bitrate::Kbps192 => 192,
            Bitrate::Kbps256 => 256,
            Bitrate::Kbps320 => 320,
        }
    }
}

/// Cover art configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverConfig {
    pub width: u32,
    pub height: u32,
    pub format: String, // "jpeg", "png", etc.
}

impl Default for CoverConfig {
    fn default() -> Self {
        Self {
            width: 500,
            height: 500,
            format: "jpeg".to_string(),
        }
    }
}

/// Metadata configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    pub embed_metadata: bool,
    pub embed_title: bool,
    pub embed_artist: bool,
    pub embed_album: bool,
    pub embed_album_artist: bool,
    pub embed_track_number: bool,
    pub embed_disc_number: bool,
    pub embed_year: bool,
    pub embed_genre: bool,
    pub embed_lyrics: bool,
    pub embed_cover: bool,
    pub embed_duration: bool,
    pub embed_bpm: bool,
    pub embed_isrc: bool,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            embed_metadata: true,
            embed_title: true,
            embed_artist: true,
            embed_album: true,
            embed_album_artist: true,
            embed_track_number: true,
            embed_disc_number: true,
            embed_year: true,
            embed_genre: true,
            embed_lyrics: true,
            embed_cover: true,
            embed_duration: true,
            embed_bpm: false,
            embed_isrc: false,
        }
    }
}

/// API Keys configuration for various services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeys {
    pub spotify_client_id: Option<String>,
    pub spotify_client_secret: Option<String>,
    pub musixmatch_api_key: Option<String>,
    pub genius_access_token: Option<String>,
    pub lastfm_api_key: Option<String>,
    pub lastfm_client_secret: Option<String>,
}

impl Default for ApiKeys {
    fn default() -> Self {
        Self {
            spotify_client_id: None,
            spotify_client_secret: None,
            musixmatch_api_key: None,
            genius_access_token: None,
            lastfm_api_key: None,
            lastfm_client_secret: None,
        }
    }
}

/// UI preferences and window state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPreferences {
    pub window_width: u32,
    pub window_height: u32,
    pub window_x: i32,
    pub window_y: i32,
    pub maximized: bool,
    pub theme: String, // "light", "dark", "auto"
    pub show_advanced_options: bool,
    pub auto_download_lyrics: bool,
    pub auto_download_covers: bool,
    pub preferred_lyrics_source: String, // "lrclib", "lyrics_ovh", "genius", "azlyrics"
    pub max_concurrent_downloads: u32, // Maximum number of concurrent downloads
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            window_width: 1200,
            window_height: 800,
            window_x: 100,
            window_y: 100,
            maximized: false,
            theme: "auto".to_string(),
            show_advanced_options: false,
            auto_download_lyrics: true,
            auto_download_covers: true,
            preferred_lyrics_source: "lrclib".to_string(),
            max_concurrent_downloads: 3,
        }
    }
}

/// SponsorBlock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SponsorBlockConfig {
    pub enabled: bool,
    pub remove_categories: Vec<String>,
}

impl Default for SponsorBlockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            remove_categories: vec![
                "sponsor".to_string(),
                "intro".to_string(),
                "outro".to_string(),
                "preview".to_string(),
                "interaction".to_string(),
                "selfpromo".to_string(),
                "music_offtopic".to_string(),
            ],
        }
    }
}

/// Cookies configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookiesConfig {
    pub enabled: bool,
    pub browsers: Vec<String>,
}

impl Default for CookiesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            browsers: vec![
                "firefox".to_string(),
                "chrome".to_string(),
                "chromium".to_string(),
                "edge".to_string(),
                "safari".to_string(),
            ],
        }
    }
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 1080,
            username: None,
            password: None,
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub download_directory: PathBuf,
    pub default_format: AudioFormat,
    pub default_bitrate: Bitrate,
    pub cover_config: CoverConfig,
    pub metadata_config: MetadataConfig,
    pub api_keys: ApiKeys,
    pub ui_preferences: UiPreferences,
    pub sponsorblock_config: SponsorBlockConfig,
    pub cookies_config: CookiesConfig,
    pub proxy_config: ProxyConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_directory: dirs::audio_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap().join("Music"))
                .join("SpotifyDownloads"),
            default_format: AudioFormat::Mp3,
            default_bitrate: Bitrate::Kbps320,
            cover_config: CoverConfig::default(),
            metadata_config: MetadataConfig::default(),
            api_keys: ApiKeys::default(),
            ui_preferences: UiPreferences::default(),
            sponsorblock_config: SponsorBlockConfig::default(),
            cookies_config: CookiesConfig::default(),
            proxy_config: ProxyConfig::default(),
        }
    }
}

impl Config {
    /// Get the configuration directory path
    pub fn config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .ok_or_else(|| SpotifyDownloaderError::Config("Could not find config directory".to_string()))
            .map(|dir| dir.join("spotify-downloader"))
    }

    /// Get the settings file path (TOML format)
    pub fn settings_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("settings.toml"))
    }

    /// Get the JSON settings file path
    pub fn json_settings_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("settings.json"))
    }

    /// Get the local JSON settings file path (in project root)
    pub fn local_json_settings_path() -> Result<PathBuf> {
        Ok(std::env::current_dir()?.join("settings.json"))
    }

    /// Load configuration from file (tries JSON first, then TOML)
    pub fn load() -> Result<Self> {
        // Try to load from local JSON file first
        if let Ok(local_json_path) = Self::local_json_settings_path() {
            if local_json_path.exists() {
                if let Ok(config) = Self::load_from_json(&local_json_path) {
                    return Ok(config);
                }
            }
        }

        // Try to load from config directory JSON file
        if let Ok(json_path) = Self::json_settings_path() {
            if json_path.exists() {
                if let Ok(config) = Self::load_from_json(&json_path) {
                    return Ok(config);
                }
            }
        }

        // Fall back to TOML
        let settings_path = Self::settings_path()?;
        
        if !settings_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let content = std::fs::read_to_string(&settings_path)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to read settings file: {}", e)))?;
        
        toml::from_str(&content)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to parse settings file: {}", e)))
    }

    /// Load configuration from JSON file
    fn load_from_json(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to read JSON settings file: {}", e)))?;
        
        serde_json::from_str(&content)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to parse JSON settings file: {}", e)))
    }

    /// Save configuration to file (saves both JSON and TOML)
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to create config directory: {}", e)))?;

        // Save TOML version
        let settings_path = Self::settings_path()?;
        let toml_content = toml::to_string_pretty(self)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to serialize TOML settings: {}", e)))?;
        
        std::fs::write(&settings_path, toml_content)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to write TOML settings file: {}", e)))?;

        // Save JSON version
        let json_path = Self::json_settings_path()?;
        let json_content = serde_json::to_string_pretty(self)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to serialize JSON settings: {}", e)))?;
        
        std::fs::write(&json_path, &json_content)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to write JSON settings file: {}", e)))?;

        // Also save a local JSON file for easy access
        if let Ok(local_json_path) = Self::local_json_settings_path() {
            let _ = std::fs::write(&local_json_path, &json_content);
        }

        Ok(())
    }

    /// Ensure download directory exists
    pub fn ensure_download_directory(&self) -> Result<()> {
        std::fs::create_dir_all(&self.download_directory)
            .map_err(|e| SpotifyDownloaderError::Config(format!("Failed to create download directory: {}", e)))?;
        Ok(())
    }
}
