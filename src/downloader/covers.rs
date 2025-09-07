use crate::downloader::{ImageInfo, TrackMetadata};
use crate::errors::{Result, SpotifyDownloaderError};
use image::{ImageFormat, DynamicImage};
use reqwest::Client;
use std::path::PathBuf;

/// Cover art downloader and processor
pub struct CoverDownloader {
    client: Client,
    itunes_client: crate::downloader::itunes::ItunesClient,
}

impl CoverDownloader {
    /// Create a new cover downloader
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            itunes_client: crate::downloader::itunes::ItunesClient::new(),
        }
    }

    /// Create a new cover downloader with a custom HTTP client (for proxy support)
    pub fn new_with_client(client: Client) -> Self {
        Self {
            client: client.clone(),
            itunes_client: crate::downloader::itunes::ItunesClient::new_with_client(client),
        }
    }

    /// Download and process cover art for a track (legacy method - use download_cover_art_data instead)
    pub async fn download_cover_art(
        &self,
        track: &TrackMetadata,
        output_path: &PathBuf,
        width: u32,
        height: u32,
        format: &str,
    ) -> Result<()> {
        // Get cover art data
        let cover_data = self.download_cover_art_data(track, width, height, format).await?;
        
        // Save the data to file
        std::fs::write(output_path, cover_data)
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to save cover art: {}", e)))?;

        Ok(())
    }

    /// Download and process cover art data (returns raw bytes)
    pub async fn download_cover_art_data(
        &self,
        track: &TrackMetadata,
        width: u32,
        height: u32,
        format: &str,
    ) -> Result<Vec<u8>> {
        println!("🔍 Searching for cover art for: {} - {}", track.artist, track.title);
        
        // Try to get cover art from multiple sources
        let cover_url = self.find_cover_art(track).await?;
        println!("✅ Found cover art URL: {}", cover_url);

        // Download the cover art
        println!("📥 Downloading cover art from: {}", cover_url);
        let image_data = self.download_image(&cover_url).await?;
        println!("✅ Cover art downloaded: {} bytes", image_data.len());

        // Process and resize the image
        println!("🖼️ Processing cover art: {}x{} -> {}x{}", 
                image_data.len(), image_data.len(), width, height);
        let processed_image = self.process_image(&image_data, width, height, format)?;

        // Convert to bytes
        println!("🔄 Converting cover art to {} format", format);
        let result = self.image_to_bytes(&processed_image, format)?;
        println!("✅ Cover art processed successfully: {} bytes", result.len());
        
        Ok(result)
    }

    /// Download cover art and save it to covers/ folder
    pub async fn download_cover_art_to_folder(
        &self,
        track: &TrackMetadata,
        output_dir: &PathBuf,
        width: u32,
        height: u32,
        format: &str,
    ) -> Result<PathBuf> {
        println!("🖼️ Downloading cover art to folder for: {} - {}", track.artist, track.title);
        
        // Create covers directory
        let mut covers_dir = output_dir.clone();
        covers_dir.push("covers");
        
        // Ensure covers directory exists
        std::fs::create_dir_all(&covers_dir)
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to create covers directory: {}", e)))?;
        
        // Generate filename using the same format as audio files
        let formatted_artist = self.format_artists_for_filename(&track.artist);
        let filename = format!("{} - {}", formatted_artist, track.title);
        let sanitized_filename = self.sanitize_filename(&filename);
        let extension = match format.to_lowercase().as_str() {
            "jpeg" | "jpg" => "jpg",
            "png" => "png",
            "webp" => "webp",
            _ => "jpg", // Default to jpg
        };
        
        let mut cover_path = covers_dir;
        cover_path.push(format!("{}.{}", sanitized_filename, extension));
        
        // Download and process cover art
        let cover_data = self.download_cover_art_data(track, width, height, format).await?;
        
        // Save to file
        std::fs::write(&cover_path, cover_data)
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to save cover art: {}", e)))?;
        
        println!("✅ Cover art saved to: {}", cover_path.display());
        Ok(cover_path)
    }

    /// Find cover art from multiple sources (optimized for speed)
    pub async fn find_cover_art(&self, track: &TrackMetadata) -> Result<String> {
        println!("🔍 Searching for cover art for: {} - {} (Album: {})", track.artist, track.title, track.album);
        
        // Try Spotify first (if available from track metadata) - fastest
        if let Some(cover_url) = &track.album_cover_url {
            if !cover_url.is_empty() {
                println!("✅ Using Spotify cover art from metadata: {}", cover_url);
                return Ok(cover_url.clone());
            }
        }

        // Try to fetch cover art from Spotify API if we have a Spotify URL
        if let Some(spotify_url) = self.extract_spotify_track_id(&track.spotify_url) {
            println!("🔍 Fetching cover art from Spotify API for track ID: {}", spotify_url);
            match self.fetch_spotify_cover_art(&spotify_url).await {
                Ok(Some(cover_url)) => {
                    println!("✅ Using Spotify cover art from API: {}", cover_url);
                    return Ok(cover_url);
                }
                Ok(None) => println!("❌ Spotify API returned no cover art"),
                Err(e) => println!("❌ Spotify API failed: {}", e),
            }
        }

        // Try iTunes album cover (usually faster than track cover)
        println!("🔍 Trying iTunes album search for: {} - {}", track.artist, track.album);
        match self.itunes_client.search_album_cover(&track.artist, &track.album).await {
            Ok(Some(image_info)) => {
                println!("✅ Using iTunes album cover: {}", image_info.url);
                return Ok(image_info.url);
            }
            Ok(None) => println!("❌ iTunes album search returned no results"),
            Err(e) => println!("❌ iTunes album search failed: {}", e),
        }

        // Try iTunes track cover as fallback
        println!("🔍 Trying iTunes track search for: {} - {}", track.artist, track.title);
        match self.itunes_client.search_cover_art(&track.artist, &track.title).await {
            Ok(Some(image_info)) => {
                println!("✅ Using iTunes track cover: {}", image_info.url);
                return Ok(image_info.url);
            }
            Ok(None) => println!("❌ iTunes track search returned no results"),
            Err(e) => println!("❌ iTunes track search failed: {}", e),
        }

        // Skip slower sources for now - we can add them back if needed
        Err(SpotifyDownloaderError::CoverArt("No cover art found".to_string()))
    }

    /// Extract Spotify track ID from Spotify URL
    fn extract_spotify_track_id(&self, spotify_url: &str) -> Option<String> {
        if spotify_url.starts_with("https://open.spotify.com/track/") {
            let parts: Vec<&str> = spotify_url.split('/').collect();
            if let Some(track_id) = parts.last() {
                return Some(track_id.to_string());
            }
        }
        None
    }

    /// Fetch cover art from Spotify API using track ID
    async fn fetch_spotify_cover_art(&self, track_id: &str) -> Result<Option<String>> {
        // Get API manager to access Spotify client
        let api_manager = crate::api::get_api_manager()?;
        let spotify_api = api_manager.spotify().await;
        let mut spotify = spotify_api.write().await;
        
        // Get track metadata from Spotify (this includes cover art)
        let track_url = format!("https://open.spotify.com/track/{}", track_id);
        match spotify.get_track_metadata(&track_url).await {
            Ok(metadata) => Ok(metadata.album_cover_url),
            Err(e) => {
                println!("❌ Failed to fetch track metadata from Spotify: {}", e);
                Err(e)
            }
        }
    }

    /// Download image from URL
    async fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(SpotifyDownloaderError::CoverArt(
                format!("Failed to download image: {}", response.status())
            ));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Process and resize image
    fn process_image(
        &self,
        image_data: &[u8],
        width: u32,
        height: u32,
        _format: &str,
    ) -> Result<DynamicImage> {
        // Load the image
        let img = image::load_from_memory(image_data)
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to load image: {}", e)))?;

        // Resize the image
        let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);

        Ok(resized)
    }

    /// Save processed image to file
    fn save_image(
        &self,
        image: &DynamicImage,
        output_path: &PathBuf,
        format: &str,
    ) -> Result<()> {
        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to create directory: {}", e)))?;
        }

        // Determine image format
        let img_format = match format.to_lowercase().as_str() {
            "jpeg" | "jpg" => ImageFormat::Jpeg,
            "png" => ImageFormat::Png,
            "webp" => ImageFormat::WebP,
            _ => ImageFormat::Jpeg, // Default to JPEG
        };

        // Save the image
        image.save_with_format(output_path, img_format)
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to save image: {}", e)))?;

        Ok(())
    }

    /// Get cover art output path for a track
    pub fn get_cover_path(&self, track: &TrackMetadata, base_path: &PathBuf, format: &str) -> PathBuf {
        let mut path = base_path.clone();
        path.push(sanitize_filename(&track.artist));
        path.push(sanitize_filename(&track.album));
        path.push(format!("cover.{}", format));
        path
    }

    /// Check if cover art already exists
    pub fn cover_exists(&self, path: &PathBuf) -> bool {
        path.exists()
    }

    /// Get image dimensions from file
    pub fn get_image_dimensions(&self, path: &PathBuf) -> Result<(u32, u32)> {
        let img = image::open(path)
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to open image: {}", e)))?;

        Ok((img.width(), img.height()))
    }
    
    /// Search for cover art on Last.fm
    async fn search_lastfm_cover_art(&self, track: &TrackMetadata) -> Result<Option<ImageInfo>> {
        let search_query = format!("{} {}", track.artist, track.album);
        let search_url = format!(
            "https://ws.audioscrobbler.com/2.0/?method=album.search&album={}&api_key=your_api_key&format=json",
            urlencoding::encode(&search_query)
        );
        
        let response = self.client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Last.fm search failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Ok(None);
        }
        
        let json: serde_json::Value = response.json().await
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to parse Last.fm response: {}", e)))?;
        
        // Extract image URL from Last.fm response
        if let Some(albums) = json.get("results").and_then(|r| r.get("albummatches")).and_then(|a| a.get("album")).and_then(|a| a.as_array()) {
            if let Some(album) = albums.first() {
                if let Some(images) = album.get("image").and_then(|i| i.as_array()) {
                    // Get the largest image (usually the last one)
                    if let Some(large_image) = images.last() {
                        if let Some(url) = large_image.get("#text").and_then(|t| t.as_str()) {
                            if !url.is_empty() {
                                return Ok(Some(ImageInfo {
                                    url: url.to_string(),
                                    width: 500, // Last.fm doesn't provide dimensions
                                    height: 500,
                                }));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Search for cover art on MusicBrainz
    async fn search_musicbrainz_cover_art(&self, track: &TrackMetadata) -> Result<Option<ImageInfo>> {
        // First, search for the release
        let search_query = format!("{} AND artist:{}", track.album, track.artist);
        let search_url = format!(
            "https://musicbrainz.org/ws/2/release?query={}&fmt=json",
            urlencoding::encode(&search_query)
        );
        
        let response = self.client
            .get(&search_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("MusicBrainz search failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Ok(None);
        }
        
        let json: serde_json::Value = response.json().await
            .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to parse MusicBrainz response: {}", e)))?;
        
        // Extract release ID
        if let Some(releases) = json.get("releases").and_then(|r| r.as_array()) {
            if let Some(release) = releases.first() {
                if let Some(release_id) = release.get("id").and_then(|i| i.as_str()) {
                    // Get cover art from Cover Art Archive
                    let cover_url = format!("https://coverartarchive.org/release/{}/front", release_id);
                    
                    // Check if cover art exists
                    let cover_response = self.client
                        .head(&cover_url)
                        .send()
                        .await;
                    
                    if let Ok(cover_response) = cover_response {
                        if cover_response.status().is_success() {
                            return Ok(Some(ImageInfo {
                                url: cover_url,
                                width: 500, // Cover Art Archive doesn't provide dimensions in HEAD request
                                height: 500,
                            }));
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Search for cover art on Spotify (placeholder - would need API access)
    async fn search_spotify_cover_art(&self, _track: &TrackMetadata) -> Result<Option<ImageInfo>> {
        // This would require Spotify API access and proper authentication
        // For now, return None as a placeholder
        println!("Spotify cover art search not implemented (requires API access)");
        Ok(None)
    }

    /// Convert image to bytes in the specified format
    fn image_to_bytes(&self, image: &DynamicImage, format: &str) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        
        match format.to_lowercase().as_str() {
            "jpeg" | "jpg" => {
                image.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Jpeg)
                    .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to encode JPEG: {}", e)))?;
            }
            "png" => {
                image.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
                    .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to encode PNG: {}", e)))?;
            }
            "webp" => {
                image.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::WebP)
                    .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to encode WebP: {}", e)))?;
            }
            _ => {
                // Default to JPEG
                image.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Jpeg)
                    .map_err(|e| SpotifyDownloaderError::CoverArt(format!("Failed to encode image: {}", e)))?;
            }
        }
        
        Ok(bytes)
    }

    /// Format artists for filename with proper comma separation
    fn format_artists_for_filename(&self, artist: &str) -> String {
        // Common separators that indicate multiple artists
        let separators = ["feat.", "featuring", "ft.", "ft", "&", "x", "X", "vs", "vs.", "feat"];
        
        let mut formatted = artist.to_string();
        
        // Replace common separators with comma and space
        for separator in &separators {
            let pattern = format!(r"\b{}\b", regex::escape(separator));
            if let Ok(regex) = regex::Regex::new(&pattern) {
                formatted = regex.replace_all(&formatted, ", ").to_string();
            }
        }
        
        // Clean up multiple spaces and trim
        formatted = regex::Regex::new(r"\s+")
            .unwrap()
            .replace_all(&formatted, " ")
            .to_string();
        
        formatted.trim().to_string()
    }

    /// Sanitize filename by removing invalid characters and replacing semicolons with commas
    fn sanitize_filename(&self, filename: &str) -> String {
        filename
            .chars()
            .map(|c| match c {
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
                ';' => ',', // Replace semicolon with comma
                _ => c,
            })
            .collect::<String>()
            .trim()
            .to_string()
    }
}

/// Sanitize filename by removing invalid characters and replacing semicolons with commas
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ';' => ',', // Replace semicolon with comma
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
