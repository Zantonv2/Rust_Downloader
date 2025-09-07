use crate::errors::{Result, SpotifyDownloaderError};
use reqwest::Client;
use serde::Deserialize;
use std::path::PathBuf;

/// SoundCloud downloader
pub struct SoundcloudDownloader {
    client: Client,
}

#[derive(Debug, Deserialize)]
pub struct SoundcloudTrack {
    id: u64,
    title: String,
    user: SoundcloudUser,
    duration: u32,
    stream_url: Option<String>,
    download_url: Option<String>,
    artwork_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SoundcloudUser {
    username: String,
}

impl SoundcloudDownloader {
    /// Create a new SoundCloud downloader
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Search for tracks on SoundCloud
    pub async fn search_tracks(&self, query: &str) -> Result<Vec<SoundcloudTrack>> {
        println!("Searching SoundCloud for: {}", query);
        
        // Use SoundCloud's search endpoint
        let search_url = format!(
            "https://api-v2.soundcloud.com/search/tracks?q={}&limit=10",
            urlencoding::encode(query)
        );
        
        let response = self.client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| SpotifyDownloaderError::Soundcloud(format!("Failed to search SoundCloud: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(SpotifyDownloaderError::Soundcloud(
                format!("SoundCloud search failed with status: {}", response.status())
            ));
        }
        
        let search_response: serde_json::Value = response.json().await
            .map_err(|e| SpotifyDownloaderError::Soundcloud(format!("Failed to parse response: {}", e)))?;
        
        let mut tracks = Vec::new();
        
        if let Some(collection) = search_response.get("collection").and_then(|c| c.as_array()) {
            for item in collection {
                if let Ok(track) = serde_json::from_value::<SoundcloudTrack>(item.clone()) {
                    tracks.push(track);
                }
            }
        }
        
        Ok(tracks)
    }

    /// Get track information from SoundCloud URL
    pub async fn get_track_info(&self, url: &str) -> Result<SoundcloudTrack> {
        println!("Getting SoundCloud track info for: {}", url);
        
        let track_id = self.extract_track_id(url)?;
        
        // Use SoundCloud's resolve endpoint
        let resolve_url = format!(
            "https://api-v2.soundcloud.com/tracks/{}",
            track_id
        );
        
        let response = self.client
            .get(&resolve_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| SpotifyDownloaderError::Soundcloud(format!("Failed to get track info: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(SpotifyDownloaderError::Soundcloud(
                format!("Failed to get track info with status: {}", response.status())
            ));
        }
        
        let track: SoundcloudTrack = response.json().await
            .map_err(|e| SpotifyDownloaderError::Soundcloud(format!("Failed to parse track info: {}", e)))?;
        
        Ok(track)
    }

    /// Download audio from SoundCloud
    pub async fn download_audio(&self, url: &str, output_path: &PathBuf) -> Result<()> {
        println!("Downloading SoundCloud audio from: {}", url);
        
        let track = self.get_track_info(url).await?;
        
        // Get the stream URL
        let stream_url = track.stream_url
            .ok_or_else(|| SpotifyDownloaderError::Soundcloud("No stream URL available".to_string()))?;
        
        // Download the audio stream
        let response = self.client
            .get(&stream_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| SpotifyDownloaderError::Soundcloud(format!("Failed to download audio: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(SpotifyDownloaderError::Soundcloud(
                format!("Failed to download audio with status: {}", response.status())
            ));
        }
        
        // Create output file
        let mut file = tokio::fs::File::create(output_path)
            .await
            .map_err(|e| SpotifyDownloaderError::Soundcloud(format!("Failed to create output file: {}", e)))?;
        
        // Get the response bytes and write to file
        use tokio::io::AsyncWriteExt;
        
        let bytes = response.bytes().await
            .map_err(|e| SpotifyDownloaderError::Soundcloud(format!("Failed to get response bytes: {}", e)))?;
        
        file.write_all(&bytes).await
            .map_err(|e| SpotifyDownloaderError::Soundcloud(format!("Failed to write to file: {}", e)))?;
        
        file.flush().await
            .map_err(|e| SpotifyDownloaderError::Soundcloud(format!("Failed to flush file: {}", e)))?;
        
        Ok(())
    }

    /// Check if URL is a valid SoundCloud URL
    pub fn is_soundcloud_url(&self, url: &str) -> bool {
        url.contains("soundcloud.com")
    }

    /// Extract track ID from SoundCloud URL
    fn extract_track_id(&self, url: &str) -> Result<String> {
        // SoundCloud URLs can be in various formats:
        // https://soundcloud.com/user/track-name
        // https://soundcloud.com/track-name
        // https://api.soundcloud.com/tracks/123456
        
        if url.contains("/tracks/") {
            // Direct track ID format
            let parts: Vec<&str> = url.split("/tracks/").collect();
            if let Some(id_part) = parts.get(1) {
                let id = id_part.split('?').next().unwrap_or(id_part);
                return Ok(id.to_string());
            }
        } else if url.contains("soundcloud.com/") {
            // User/track format - we'll need to resolve this
            // For now, extract the path and use it as a search term
            let path = url.split("soundcloud.com/").nth(1)
                .ok_or_else(|| SpotifyDownloaderError::InvalidUrl(format!("Invalid SoundCloud URL: {}", url)))?;
            
            // Remove query parameters
            let clean_path = path.split('?').next().unwrap_or(path);
            return Ok(clean_path.to_string());
        }
        
        Err(SpotifyDownloaderError::InvalidUrl(format!("Invalid SoundCloud URL: {}", url)))
    }
}
