use crate::config::{Config, MetadataConfig};
use crate::errors::Result;

/// Settings management module
/// This module provides a simple interface for managing application settings
#[derive(Debug, Clone)]
pub struct Settings {
    config: Config,
}

impl Settings {
    /// Create a new settings instance by loading from file
    pub fn load() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self { config })
    }

    /// Load settings from local JSON file
    pub fn load_from_local_json() -> Result<Self> {
        let local_json_path = crate::config::Config::local_json_settings_path()?;
        
        if !local_json_path.exists() {
            // Create default settings if local JSON doesn't exist
            let settings = Self::default();
            settings.save_to_local_json()?;
            return Ok(settings);
        }

        let content = std::fs::read_to_string(&local_json_path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Config(format!("Failed to read local JSON settings: {}", e)))?;
        
        let config: Config = serde_json::from_str(&content)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Config(format!("Failed to parse local JSON settings: {}", e)))?;
        
        Ok(Self { config })
    }

    /// Create default settings
    pub fn default() -> Self {
        Self { config: Config::default() }
    }

    /// Get a reference to the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a mutable reference to the current configuration
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Save the current configuration to file
    pub fn save(&self) -> Result<()> {
        // Save to both config directory and local JSON file
        self.config.save()?;
        
        // Also save directly to local JSON file for easy access
        self.save_to_local_json()
    }

    /// Save settings directly to local JSON file
    pub fn save_to_local_json(&self) -> Result<()> {
        let local_json_path = crate::config::Config::local_json_settings_path()?;
        
        // Create the file if it doesn't exist
        if !local_json_path.exists() {
            if let Some(parent) = local_json_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| crate::errors::SpotifyDownloaderError::Config(format!("Failed to create directory: {}", e)))?;
            }
        }

        let json_content = serde_json::to_string_pretty(&self.config)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Config(format!("Failed to serialize settings: {}", e)))?;
        
        std::fs::write(&local_json_path, json_content)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Config(format!("Failed to write local JSON settings: {}", e)))?;

        Ok(())
    }

    /// Update the download directory
    pub fn set_download_directory(&mut self, path: std::path::PathBuf) -> Result<()> {
        self.config.download_directory = path;
        self.config.ensure_download_directory()?;
        self.save()
    }

    /// Update the default audio format
    pub fn set_default_format(&mut self, format: crate::config::AudioFormat) -> Result<()> {
        self.config.default_format = format;
        self.save()
    }

    /// Update the default bitrate
    pub fn set_default_bitrate(&mut self, bitrate: crate::config::Bitrate) -> Result<()> {
        self.config.default_bitrate = bitrate;
        self.save()
    }

    /// Update cover art configuration
    pub fn set_cover_config(&mut self, width: u32, height: u32, format: String) -> Result<()> {
        self.config.cover_config.width = width;
        self.config.cover_config.height = height;
        self.config.cover_config.format = format;
        self.save()
    }

    /// Set Spotify API credentials
    pub fn set_spotify_credentials(&mut self, client_id: String, client_secret: String) -> Result<()> {
        self.config.api_keys.spotify_client_id = Some(client_id);
        self.config.api_keys.spotify_client_secret = Some(client_secret);
        self.save()
    }

    /// Set Musixmatch API key
    pub fn set_musixmatch_api_key(&mut self, api_key: String) -> Result<()> {
        self.config.api_keys.musixmatch_api_key = Some(api_key);
        self.save()
    }

    /// Set Genius access token
    pub fn set_genius_access_token(&mut self, access_token: String) -> Result<()> {
        self.config.api_keys.genius_access_token = Some(access_token);
        self.save()
    }

    /// Set Last.fm API credentials
    pub fn set_lastfm_credentials(&mut self, api_key: String, client_secret: String) -> Result<()> {
        self.config.api_keys.lastfm_api_key = Some(api_key);
        self.config.api_keys.lastfm_client_secret = Some(client_secret);
        self.save()
    }

    /// Get API keys
    pub fn api_keys(&self) -> &crate::config::ApiKeys {
        &self.config.api_keys
    }

    /// Get mutable API keys
    pub fn api_keys_mut(&mut self) -> &mut crate::config::ApiKeys {
        &mut self.config.api_keys
    }

    /// Update UI preferences
    pub fn set_ui_preferences(&mut self, preferences: crate::config::UiPreferences) -> Result<()> {
        self.config.ui_preferences = preferences;
        self.save()
    }

    /// Get UI preferences
    pub fn ui_preferences(&self) -> &crate::config::UiPreferences {
        &self.config.ui_preferences
    }

    /// Get mutable UI preferences
    pub fn ui_preferences_mut(&mut self) -> &mut crate::config::UiPreferences {
        &mut self.config.ui_preferences
    }

    /// Update window state
    pub fn set_window_state(&mut self, width: u32, height: u32, x: i32, y: i32, maximized: bool) -> Result<()> {
        self.config.ui_preferences.window_width = width;
        self.config.ui_preferences.window_height = height;
        self.config.ui_preferences.window_x = x;
        self.config.ui_preferences.window_y = y;
        self.config.ui_preferences.maximized = maximized;
        self.save()
    }

    /// Set theme
    pub fn set_theme(&mut self, theme: String) -> Result<()> {
        self.config.ui_preferences.theme = theme;
        self.save()
    }

    /// Toggle advanced options visibility
    pub fn toggle_advanced_options(&mut self) -> Result<()> {
        self.config.ui_preferences.show_advanced_options = !self.config.ui_preferences.show_advanced_options;
        self.save()
    }

    /// Set preferred lyrics source
    pub fn set_preferred_lyrics_source(&mut self, source: String) -> Result<()> {
        self.config.ui_preferences.preferred_lyrics_source = source;
        self.save()
    }

    /// Toggle metadata embedding
    pub fn toggle_metadata_embedding(&mut self) -> Result<()> {
        self.config.metadata_config.embed_metadata = !self.config.metadata_config.embed_metadata;
        self.save()
    }

    /// Toggle specific metadata field embedding
    pub fn toggle_metadata_field(&mut self, field: &str) -> Result<()> {
        match field {
            "title" => self.config.metadata_config.embed_title = !self.config.metadata_config.embed_title,
            "artist" => self.config.metadata_config.embed_artist = !self.config.metadata_config.embed_artist,
            "album" => self.config.metadata_config.embed_album = !self.config.metadata_config.embed_album,
            "album_artist" => self.config.metadata_config.embed_album_artist = !self.config.metadata_config.embed_album_artist,
            "track_number" => self.config.metadata_config.embed_track_number = !self.config.metadata_config.embed_track_number,
            "disc_number" => self.config.metadata_config.embed_disc_number = !self.config.metadata_config.embed_disc_number,
            "year" => self.config.metadata_config.embed_year = !self.config.metadata_config.embed_year,
            "genre" => self.config.metadata_config.embed_genre = !self.config.metadata_config.embed_genre,
            "lyrics" => self.config.metadata_config.embed_lyrics = !self.config.metadata_config.embed_lyrics,
            "cover" => self.config.metadata_config.embed_cover = !self.config.metadata_config.embed_cover,
            "duration" => self.config.metadata_config.embed_duration = !self.config.metadata_config.embed_duration,
            "bpm" => self.config.metadata_config.embed_bpm = !self.config.metadata_config.embed_bpm,
            "isrc" => self.config.metadata_config.embed_isrc = !self.config.metadata_config.embed_isrc,
            _ => return Err(crate::errors::SpotifyDownloaderError::Config(format!("Unknown metadata field: {}", field))),
        }
        self.save()
    }

    /// Get metadata configuration
    pub fn metadata_config(&self) -> &MetadataConfig {
        &self.config.metadata_config
    }

    /// Get mutable metadata configuration
    pub fn metadata_config_mut(&mut self) -> &mut MetadataConfig {
        &mut self.config.metadata_config
    }

    /// Get SponsorBlock configuration
    pub fn sponsorblock_config(&self) -> &crate::config::SponsorBlockConfig {
        &self.config.sponsorblock_config
    }

    /// Get mutable SponsorBlock configuration
    pub fn sponsorblock_config_mut(&mut self) -> &mut crate::config::SponsorBlockConfig {
        &mut self.config.sponsorblock_config
    }

    /// Get cookies configuration
    pub fn cookies_config(&self) -> &crate::config::CookiesConfig {
        &self.config.cookies_config
    }

    /// Get mutable cookies configuration
    pub fn cookies_config_mut(&mut self) -> &mut crate::config::CookiesConfig {
        &mut self.config.cookies_config
    }

    /// Toggle SponsorBlock
    pub fn toggle_sponsorblock(&mut self) -> Result<()> {
        self.config.sponsorblock_config.enabled = !self.config.sponsorblock_config.enabled;
        self.save()
    }

    /// Toggle SponsorBlock category
    pub fn toggle_sponsorblock_category(&mut self, category: String) -> Result<()> {
        if self.config.sponsorblock_config.remove_categories.contains(&category) {
            self.config.sponsorblock_config.remove_categories.retain(|c| c != &category);
        } else {
            self.config.sponsorblock_config.remove_categories.push(category);
        }
        self.save()
    }

    /// Toggle cookies usage
    pub fn toggle_cookies(&mut self) -> Result<()> {
        self.config.cookies_config.enabled = !self.config.cookies_config.enabled;
        self.save()
    }

    /// Set selected browser
    pub fn set_selected_browser(&mut self, browser: String) -> Result<()> {
        // For now, we'll just store the selected browser in UI preferences
        // In a full implementation, this would be stored in cookies_config
        self.config.ui_preferences.preferred_lyrics_source = browser; // Reusing field for now
        self.save()
    }

    /// Get selected browser
    pub fn get_selected_browser(&self) -> String {
        self.config.ui_preferences.preferred_lyrics_source.clone()
    }
}
