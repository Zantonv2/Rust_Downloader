use crate::downloader::{TrackMetadata, AlbumMetadata, PlaylistMetadata, ImageInfo};
use crate::errors::{Result, SpotifyDownloaderError};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

/// Spotify API client
pub struct SpotifyClient {
    client: Client,
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpotifyTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u32,
}

#[derive(Debug, Deserialize)]
struct SpotifyTrackResponse {
    id: String,
    name: String,
    artists: Vec<SpotifyArtist>,
    album: SpotifyAlbum,
    track_number: u32,
    disc_number: u32,
    duration_ms: u32,
    external_urls: HashMap<String, String>,
    preview_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SpotifyArtist {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SpotifyAlbum {
    id: String,
    name: String,
    artists: Vec<SpotifyArtist>,
    release_date: String,
    total_tracks: u32,
    images: Vec<SpotifyImage>,
    external_urls: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct SpotifyImage {
    url: String,
    width: u32,
    height: u32,
}

impl SpotifyClient {
    /// Create a new Spotify client
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            access_token: None,
        }
    }

    /// Create a new Spotify client with a custom HTTP client (for proxy support)
    pub fn new_with_client(client_id: String, client_secret: String, client: Client) -> Self {
        Self {
            client,
            client_id,
            client_secret,
            access_token: None,
        }
    }

    /// Check if the client is configured (has client_id and client_secret)
    #[allow(dead_code)]
    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }

    /// Authenticate with Spotify API
    pub async fn authenticate(&mut self) -> Result<()> {
        println!("Authenticating with Spotify API...");
        println!("Client ID: {}", if self.client_id.is_empty() { "EMPTY" } else { "SET" });
        println!("Client Secret: {}", if self.client_secret.is_empty() { "EMPTY" } else { "SET" });
        
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        println!("Sending authentication request...");
        println!("Using proxy-configured client for Spotify API...");
        let response = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .form(&params)
            .send()
            .await?;

        let status = response.status();
        println!("Authentication response status: {}", status);
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SpotifyDownloaderError::Spotify(
                format!("Authentication failed: {} - {}", status, error_text)
            ));
        }

        let token_response: SpotifyTokenResponse = response.json().await?;
        self.access_token = Some(token_response.access_token);
        println!("Authentication successful!");

        Ok(())
    }

    /// Ensure we have a valid access token
    async fn ensure_authenticated(&mut self) -> Result<()> {
        if self.access_token.is_none() {
            self.authenticate().await?;
        }
        Ok(())
    }

    /// Get track metadata from Spotify URL
    pub async fn get_track_metadata(&mut self, url: &str) -> Result<TrackMetadata> {
        println!("Getting track metadata for URL: {}", url);
        self.ensure_authenticated().await?;

        let track_id = self.extract_track_id(url)?;
        println!("Extracted track ID: {}", track_id);
        let access_token = self.access_token.as_ref().unwrap();

        println!("Fetching track data from Spotify API...");
        println!("Using proxy-configured client for track API request...");
        let response = self
            .client
            .get(&format!("https://api.spotify.com/v1/tracks/{}", track_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        let status = response.status();
        println!("Track API response status: {}", status);
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SpotifyDownloaderError::Spotify(
                format!("Failed to fetch track: {} - {}", status, error_text)
            ));
        }

        println!("Parsing track data...");
        let spotify_track: SpotifyTrackResponse = response.json().await?;
        
        // Get the largest album cover image
        let album_cover_url = spotify_track.album.images
            .iter()
            .max_by_key(|img| img.width)
            .map(|img| img.url.clone());

        Ok(TrackMetadata {
            id: spotify_track.id,
            title: spotify_track.name,
            artist: spotify_track.artists.iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<&str>>()
                .join(", "),
            album: spotify_track.album.name,
            album_artist: Some(spotify_track.album.artists.iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<&str>>()
                .join(", ")),
            track_number: Some(spotify_track.track_number),
            disc_number: Some(spotify_track.disc_number),
            release_date: Some(spotify_track.album.release_date),
            duration_ms: spotify_track.duration_ms,
            genres: Vec::new(), // Would need additional API call
            spotify_url: url.to_string(),
            preview_url: spotify_track.preview_url,
            external_urls: spotify_track.external_urls,
            album_cover_url,
            composer: None,
            comment: None,
        })
    }

    /// Get album metadata from Spotify URL
    pub async fn get_album_metadata(&mut self, url: &str) -> Result<AlbumMetadata> {
        self.ensure_authenticated().await?;

        let album_id = self.extract_album_id(url)?;
        let access_token = self.access_token.as_ref().unwrap();

        let response = self
            .client
            .get(&format!("https://api.spotify.com/v1/albums/{}", album_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SpotifyDownloaderError::Spotify(
                format!("Failed to fetch album: {}", response.status())
            ));
        }

        let spotify_album: SpotifyAlbum = response.json().await?;
        
        // Fetch tracks for the album
        let tracks = self.fetch_album_tracks(&spotify_album.id).await?;

        Ok(AlbumMetadata {
            id: spotify_album.id,
            name: spotify_album.name,
            artist: spotify_album.artists.iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<&str>>()
                .join(", "),
            release_date: spotify_album.release_date,
            total_tracks: spotify_album.total_tracks,
            images: spotify_album.images.into_iter().map(|img| ImageInfo {
                url: img.url,
                width: img.width,
                height: img.height,
            }).collect(),
            spotify_url: url.to_string(),
            tracks,
        })
    }

    /// Get playlist metadata from Spotify URL
    pub async fn get_playlist_metadata(&mut self, url: &str) -> Result<PlaylistMetadata> {
        self.ensure_authenticated().await?;

        let playlist_id = self.extract_playlist_id(url)?;
        let access_token = self.access_token.as_ref().unwrap();

        let response = self
            .client
            .get(&format!("https://api.spotify.com/v1/playlists/{}", playlist_id))
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SpotifyDownloaderError::Spotify(
                format!("Failed to fetch playlist: {}", response.status())
            ));
        }

        let playlist_response: serde_json::Value = response.json().await?;
        
        // Parse playlist metadata
        let playlist_name = playlist_response.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown Playlist")
            .to_string();
        
        let playlist_description = playlist_response.get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        
        let total_tracks = playlist_response.get("tracks")
            .and_then(|t| t.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;
        
        // Fetch playlist tracks
        let tracks = self.fetch_playlist_tracks(&playlist_id).await?;
        
        Ok(PlaylistMetadata {
            id: playlist_id,
            name: playlist_name,
            description: Some(playlist_description),
            owner: "Unknown".to_string(), // Would need additional API call
            total_tracks,
            images: Vec::new(), // Would need additional API call
            spotify_url: url.to_string(),
            tracks,
        })
    }

    /// Extract track ID from Spotify URL
    fn extract_track_id(&self, url: &str) -> Result<String> {
        // Handle various Spotify URL formats
        if url.contains("/track/") {
            let parts: Vec<&str> = url.split("/track/").collect();
            if let Some(id_part) = parts.get(1) {
                let id = id_part.split('?').next().unwrap_or(id_part);
                return Ok(id.to_string());
            }
        }
        
        Err(SpotifyDownloaderError::InvalidUrl(format!("Invalid Spotify track URL: {}", url)))
    }

    /// Extract album ID from Spotify URL
    fn extract_album_id(&self, url: &str) -> Result<String> {
        if url.contains("/album/") {
            let parts: Vec<&str> = url.split("/album/").collect();
            if let Some(id_part) = parts.get(1) {
                let id = id_part.split('?').next().unwrap_or(id_part);
                return Ok(id.to_string());
            }
        }
        
        Err(SpotifyDownloaderError::InvalidUrl(format!("Invalid Spotify album URL: {}", url)))
    }

    /// Extract playlist ID from Spotify URL
    fn extract_playlist_id(&self, url: &str) -> Result<String> {
        if url.contains("/playlist/") {
            let parts: Vec<&str> = url.split("/playlist/").collect();
            if let Some(id_part) = parts.get(1) {
                let id = id_part.split('?').next().unwrap_or(id_part);
                return Ok(id.to_string());
            }
        }
        
        Err(SpotifyDownloaderError::InvalidUrl(format!("Invalid Spotify playlist URL: {}", url)))
    }
    
    /// Fetch tracks for an album
    async fn fetch_album_tracks(&mut self, album_id: &str) -> Result<Vec<TrackMetadata>> {
        self.ensure_authenticated().await?;
        let access_token = self.access_token.as_ref().unwrap();
        
        let mut all_tracks = Vec::new();
        let mut next_url = Some(format!("https://api.spotify.com/v1/albums/{}/tracks?limit=50", album_id));
        let mut page_count = 0;
        
        while let Some(url) = next_url {
            page_count += 1;
            println!("Fetching album tracks page {}...", page_count);
            
            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
            
            if !response.status().is_success() {
                return Err(SpotifyDownloaderError::Spotify(
                    format!("Failed to fetch album tracks page {}: {}", page_count, response.status())
                ));
            }
            
            let tracks_response: serde_json::Value = response.json().await?;
            
            // Parse tracks from this page
            if let Some(items) = tracks_response.get("items").and_then(|i| i.as_array()) {
                let page_track_count = items.len();
                println!("Found {} tracks on page {}", page_track_count, page_count);
                
                for item in items {
                    if let Ok(track) = self.parse_track_from_album_item(item) {
                        all_tracks.push(track);
                    }
                }
            }
            
            // Check for next page
            next_url = tracks_response.get("next")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
                
            if next_url.is_some() {
                println!("More tracks available, fetching next page...");
            }
        }
        
        println!("Successfully fetched {} tracks from {} pages", all_tracks.len(), page_count);
        Ok(all_tracks)
    }
    
    /// Fetch tracks for a playlist
    async fn fetch_playlist_tracks(&mut self, playlist_id: &str) -> Result<Vec<TrackMetadata>> {
        self.ensure_authenticated().await?;
        let access_token = self.access_token.as_ref().unwrap();
        
        let mut all_tracks = Vec::new();
        let mut next_url = Some(format!("https://api.spotify.com/v1/playlists/{}/tracks?limit=100", playlist_id));
        let mut page_count = 0;
        
        while let Some(url) = next_url {
            page_count += 1;
            println!("Fetching playlist tracks page {}...", page_count);
            
            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .send()
                .await?;
            
            if !response.status().is_success() {
                return Err(SpotifyDownloaderError::Spotify(
                    format!("Failed to fetch playlist tracks page {}: {}", page_count, response.status())
                ));
            }
            
            let tracks_response: serde_json::Value = response.json().await?;
            
            // Parse tracks from this page
            if let Some(items) = tracks_response.get("items").and_then(|i| i.as_array()) {
                let page_track_count = items.len();
                println!("Found {} tracks on page {}", page_track_count, page_count);
                
                for item in items {
                    if let Some(track_data) = item.get("track") {
                        // Skip null tracks (deleted tracks)
                        if !track_data.is_null() {
                            if let Ok(track) = self.parse_track_from_playlist_item(track_data) {
                                all_tracks.push(track);
                            }
                        }
                    }
                }
            }
            
            // Check for next page
            next_url = tracks_response.get("next")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
                
            if next_url.is_some() {
                println!("More tracks available, fetching next page...");
            }
        }
        
        println!("Successfully fetched {} tracks from {} pages", all_tracks.len(), page_count);
        Ok(all_tracks)
    }
    
    /// Parse track metadata from album tracks response
    fn parse_track_from_album_item(&self, item: &serde_json::Value) -> Result<TrackMetadata> {
        let id = item.get("id")
            .and_then(|i| i.as_str())
            .unwrap_or("")
            .to_string();
        
        let name = item.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        
        let artists: &[serde_json::Value] = item.get("artists")
            .and_then(|a| a.as_array())
            .map_or(&[], |v| v);
        
        // Join all artists with commas for multiple artists support
        let artist = artists.iter()
            .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
            .collect::<Vec<&str>>()
            .join(", ");
        
        let track_number = item.get("track_number")
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);
        
        let duration_ms = item.get("duration_ms")
            .and_then(|d| d.as_u64())
            .unwrap_or(0) as u32;
        
        Ok(TrackMetadata {
            id: id.clone(),
            title: name,
            artist,
            album: "Unknown Album".to_string(), // Would need album context
            album_artist: None,
            track_number,
            disc_number: None,
            release_date: None,
            duration_ms,
            genres: Vec::new(),
            spotify_url: format!("https://open.spotify.com/track/{}", id),
            preview_url: None,
            external_urls: std::collections::HashMap::new(),
            album_cover_url: None,
            composer: None,
            comment: None,
        })
    }
    
    /// Parse track metadata from playlist tracks response
    fn parse_track_from_playlist_item(&self, track_data: &serde_json::Value) -> Result<TrackMetadata> {
        let id = track_data.get("id")
            .and_then(|i| i.as_str())
            .unwrap_or("")
            .to_string();
        
        let name = track_data.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        
        let artists: &[serde_json::Value] = track_data.get("artists")
            .and_then(|a| a.as_array())
            .map_or(&[], |v| v);
        
        // Join all artists with commas for multiple artists support
        let artist = artists.iter()
            .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
            .collect::<Vec<&str>>()
            .join(", ");
        
        let album = track_data.get("album")
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Unknown Album")
            .to_string();
        
        let album_artists = track_data.get("album")
            .and_then(|a| a.get("artists"))
            .and_then(|a| a.as_array());
        
        // Join all album artists with commas for multiple artists support
        let album_artist = album_artists
            .map(|artists| {
                artists.iter()
                    .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                    .collect::<Vec<&str>>()
                    .join(", ")
            });
        
        let track_number = track_data.get("track_number")
            .and_then(|t| t.as_u64())
            .map(|t| t as u32);
        
        let disc_number = track_data.get("disc_number")
            .and_then(|d| d.as_u64())
            .map(|d| d as u32);
        
        let duration_ms = track_data.get("duration_ms")
            .and_then(|d| d.as_u64())
            .unwrap_or(0) as u32;
        
        let preview_url = track_data.get("preview_url")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());
        
        let external_urls = track_data.get("external_urls")
            .and_then(|e| e.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(TrackMetadata {
            id: id.clone(),
            title: name,
            artist,
            album,
            album_artist,
            track_number,
            disc_number,
            release_date: None,
            duration_ms,
            genres: Vec::new(),
            spotify_url: format!("https://open.spotify.com/track/{}", id),
            preview_url,
            external_urls,
            album_cover_url: None,
            composer: None,
            comment: None,
        })
    }
}
