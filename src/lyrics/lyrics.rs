use crate::downloader::TrackMetadata;
use crate::errors::Result;
use crate::config::ApiKeys;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Unified lyrics downloader supporting multiple sources
pub struct LyricsDownloader {
    client: reqwest::Client,
    api_keys: Option<ApiKeys>,
}

/// Lyrics line with timestamp for synced lyrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsLine {
    pub timestamp: u32, // milliseconds
    pub text: String,
}

/// Synced lyrics data (LRC format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncedLyrics {
    pub lines: Vec<LyricsLine>,
    pub offset: i32, // offset in milliseconds
    pub source: String,
}

/// Unsynced lyrics data (plain text)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsyncedLyrics {
    pub text: String,
    pub source: String,
}

/// Combined lyrics result
#[derive(Debug, Clone)]
pub struct LyricsResult {
    pub synced: Option<SyncedLyrics>,
    pub unsynced: Option<UnsyncedLyrics>,
    pub synced_path: Option<PathBuf>,
    pub unsynced_path: Option<PathBuf>,
}

/// LRClib API response structure
#[derive(Debug, Deserialize)]
struct LRClibResponse {
    #[serde(rename = "trackName")]
    track_name: String,
    #[serde(rename = "artistName")]
    artist_name: String,
    #[serde(rename = "albumName")]
    album_name: Option<String>,
    #[serde(rename = "duration")]
    duration: Option<f64>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
}

/// Lyrics.ovh API response structure
#[derive(Debug, Deserialize)]
struct LyricsOvhResponse {
    lyrics: Option<String>,
}

/// Musixmatch API response structure
#[derive(Debug, Deserialize)]
struct MusixmatchResponse {
    message: MusixmatchMessage,
}

#[derive(Debug, Deserialize)]
struct MusixmatchMessage {
    body: Option<MusixmatchBody>,
}

#[derive(Debug, Deserialize)]
struct MusixmatchBody {
    lyrics: Option<MusixmatchLyrics>,
}

#[derive(Debug, Deserialize)]
struct MusixmatchLyrics {
    #[serde(rename = "lyrics_body")]
    lyrics_body: Option<String>,
}

impl LyricsDownloader {
    /// Create a new lyrics downloader
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
            api_keys: None,
        }
    }

    /// Create a new lyrics downloader with API keys
    pub fn new_with_api_keys(api_keys: ApiKeys) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
            api_keys: Some(api_keys),
        }
    }

    /// Create a new lyrics downloader with a custom HTTP client (for proxy support)
    pub fn new_with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            api_keys: None,
        }
    }

    /// Create a new lyrics downloader with custom client and API keys
    pub fn new_with_client_and_api_keys(client: reqwest::Client, api_keys: ApiKeys) -> Self {
        Self {
            client,
            api_keys: Some(api_keys),
        }
    }

    /// Download lyrics for embedding only (no file saving)
    pub async fn download_lyrics_for_embedding(
        &self,
        track: &TrackMetadata,
    ) -> Result<LyricsResult> {
        let mut result = LyricsResult {
            synced: None,
            unsynced: None,
            synced_path: None,
            unsynced_path: None,
        };

        // Try to get synced lyrics first (prioritize LRClib)
        if let Ok(synced) = self.download_synced_lyrics(track).await {
            result.synced = Some(synced.clone());
            println!("Downloaded synced lyrics from: {}", synced.source);
        } else {
            println!("No synced lyrics found for: {} - {}", track.artist, track.title);
            
            // Fall back to unsynced lyrics
            if let Ok(unsynced) = self.download_unsynced_lyrics(track).await {
                result.unsynced = Some(unsynced.clone());
                println!("Downloaded unsynced lyrics from: {}", unsynced.source);
            } else {
                println!("No lyrics found for: {} - {}", track.artist, track.title);
            }
        }

        Ok(result)
    }

    /// Download lyrics for a track - synced lyrics first, then unsynced as fallback
    pub async fn download_lyrics(
        &self,
        track: &TrackMetadata,
        output_dir: &PathBuf,
    ) -> Result<LyricsResult> {
        let mut result = LyricsResult {
            synced: None,
            unsynced: None,
            synced_path: None,
            unsynced_path: None,
        };

        // Try to get synced lyrics first (prioritize LRClib)
        if let Ok(synced) = self.download_synced_lyrics(track).await {
            result.synced = Some(synced.clone());
            result.synced_path = Some(self.save_synced_lyrics(&synced, track, output_dir).await?);
            println!("Downloaded synced lyrics from: {}", synced.source);
        } else {
            println!("No synced lyrics found for: {} - {}", track.artist, track.title);
            
            // Fall back to unsynced lyrics
            if let Ok(unsynced) = self.download_unsynced_lyrics(track).await {
                result.unsynced = Some(unsynced.clone());
                result.unsynced_path = Some(self.save_unsynced_lyrics(&unsynced, track, output_dir).await?);
                println!("Downloaded unsynced lyrics from: {}", unsynced.source);
            } else {
                println!("No lyrics found for: {} - {}", track.artist, track.title);
            }
        }

        Ok(result)
    }

    /// Download synced lyrics with fallback chain
    async fn download_synced_lyrics(&self, track: &TrackMetadata) -> Result<SyncedLyrics> {
        println!("Downloading synced lyrics for: {} - {}", track.artist, track.title);

        // Try LRClib first (best source for synced lyrics)
        if let Ok(lyrics) = self.try_lrclib_synced(track).await {
            return Ok(lyrics);
        }

        // Try lyrics.ovh for synced lyrics
        if let Ok(lyrics) = self.try_lyrics_ovh_synced(track).await {
            return Ok(lyrics);
        }

        // If no synced lyrics found, return error
        Err(crate::errors::SpotifyDownloaderError::Lyrics(
            "No synced lyrics found from any source".to_string()
        ))
    }

    /// Download unsynced lyrics with fallback chain
    async fn download_unsynced_lyrics(&self, track: &TrackMetadata) -> Result<UnsyncedLyrics> {
        println!("Downloading unsynced lyrics for: {} - {}", track.artist, track.title);

        // Generate multiple search query variations
        let search_queries = self.generate_search_queries(track);

        // Try each search query variation with each source
        for query in &search_queries {
            println!("🔍 Trying search query: '{}'", query);
            
            // Try LRClib first (might have plain lyrics even if no synced)
            if let Ok(lyrics) = self.try_lrclib_unsynced_with_query(track, query).await {
                println!("✅ Found lyrics on LRClib with query: '{}'", query);
                return Ok(lyrics);
            }

            // Try lyrics.ovh
            if let Ok(lyrics) = self.try_lyrics_ovh_unsynced_with_query(track, query).await {
                println!("✅ Found lyrics on Lyrics.ovh with query: '{}'", query);
                return Ok(lyrics);
            }

            // Try Musixmatch
            if let Ok(lyrics) = self.try_musixmatch_with_query(track, query).await {
                println!("✅ Found lyrics on Musixmatch with query: '{}'", query);
                return Ok(lyrics);
            }

            // Try AZLyrics
            if let Ok(lyrics) = self.try_azlyrics_with_query(track, query).await {
                println!("✅ Found lyrics on AZLyrics with query: '{}'", query);
                return Ok(lyrics);
            }

            // Try Genius
            if let Ok(lyrics) = self.try_genius_with_query(track, query).await {
                println!("✅ Found lyrics on Genius with query: '{}'", query);
                return Ok(lyrics);
            }
        }

        // If no lyrics found, return error
        Err(crate::errors::SpotifyDownloaderError::Lyrics(
            "No lyrics found from any source with any query variation".to_string()
        ))
    }

    /// Try LRClib for synced lyrics
    async fn try_lrclib_synced(&self, track: &TrackMetadata) -> Result<SyncedLyrics> {
        let search_query = format!("{} {}", track.artist, track.title);
        let url = format!("https://lrclib.net/api/search?q={}", urlencoding::encode(&search_query));
        
        println!("🔍 LRClib synced search: {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("LRClib request failed: {}", e)))?;

        let status = response.status();
        println!("📡 LRClib response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            println!("❌ LRClib request failed: {} - {}", status, error_text);
            return Err(crate::errors::SpotifyDownloaderError::Lyrics(format!("LRClib request failed: {} - {}", status, error_text)));
        }

        let response_text = response.text().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to get response text: {}", e)))?;
        
        println!("📄 LRClib raw response: {}", response_text);

        let lrclib_responses: Vec<LRClibResponse> = serde_json::from_str(&response_text)
            .map_err(|e| {
                println!("❌ Failed to parse LRClib JSON: {}", e);
                crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to parse LRClib response: {}", e))
            })?;

        println!("🎵 LRClib found {} results", lrclib_responses.len());

        // Try to find the best match
        for (i, lrclib_response) in lrclib_responses.iter().enumerate() {
            println!("🎵 LRClib result {}: track='{}', artist='{}', has_synced={}, has_plain={}", 
                     i + 1,
                     lrclib_response.track_name, 
                     lrclib_response.artist_name,
                     lrclib_response.synced_lyrics.is_some(),
                     lrclib_response.plain_lyrics.is_some());

            if let Some(synced_lyrics_text) = &lrclib_response.synced_lyrics {
                if !synced_lyrics_text.trim().is_empty() {
                    println!("✅ Found synced lyrics on LRClib (result {}), length: {}", i + 1, synced_lyrics_text.len());
                    let lines = self.parse_lrc_content(synced_lyrics_text)?;
                    return Ok(SyncedLyrics {
                        lines,
                        offset: 0,
                        source: "LRClib".to_string(),
                    });
                } else {
                    println!("⚠️ LRClib result {} synced lyrics field is empty", i + 1);
                }
            } else {
                println!("⚠️ LRClib result {} has no synced_lyrics field", i + 1);
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No synced lyrics found on LRClib".to_string()))
    }

    /// Try LRClib for unsynced lyrics
    async fn try_lrclib_unsynced(&self, track: &TrackMetadata) -> Result<UnsyncedLyrics> {
        let search_query = format!("{} {}", track.artist, track.title);
        let url = format!("https://lrclib.net/api/search?q={}", urlencoding::encode(&search_query));
        
        println!("🔍 LRClib unsynced search: {}", url);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("LRClib request failed: {}", e)))?;

        let status = response.status();
        println!("📡 LRClib response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            println!("❌ LRClib request failed: {} - {}", status, error_text);
            return Err(crate::errors::SpotifyDownloaderError::Lyrics(format!("LRClib request failed: {} - {}", status, error_text)));
        }

        let response_text = response.text().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to get response text: {}", e)))?;
        
        println!("📄 LRClib raw response: {}", response_text);

        let lrclib_responses: Vec<LRClibResponse> = serde_json::from_str(&response_text)
            .map_err(|e| {
                println!("❌ Failed to parse LRClib JSON: {}", e);
                crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to parse LRClib response: {}", e))
            })?;

        println!("🎵 LRClib found {} results", lrclib_responses.len());

        // Try to find the best match
        for (i, lrclib_response) in lrclib_responses.iter().enumerate() {
            println!("🎵 LRClib result {}: track='{}', artist='{}', has_synced={}, has_plain={}", 
                     i + 1,
                     lrclib_response.track_name, 
                     lrclib_response.artist_name,
                     lrclib_response.synced_lyrics.is_some(),
                     lrclib_response.plain_lyrics.is_some());

            if let Some(plain_lyrics) = &lrclib_response.plain_lyrics {
                if !plain_lyrics.trim().is_empty() {
                    println!("✅ Found plain lyrics on LRClib (result {}), length: {}", i + 1, plain_lyrics.len());
                    return Ok(UnsyncedLyrics {
                        text: self.clean_lyrics_text(plain_lyrics),
                        source: "LRClib".to_string(),
                    });
                } else {
                    println!("⚠️ LRClib result {} plain lyrics field is empty", i + 1);
                }
            } else {
                println!("⚠️ LRClib result {} has no plain_lyrics field", i + 1);
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No unsynced lyrics found on LRClib".to_string()))
    }

    /// Try lyrics.ovh for synced lyrics
    async fn try_lyrics_ovh_synced(&self, track: &TrackMetadata) -> Result<SyncedLyrics> {
        let artist = urlencoding::encode(&track.artist);
        let title = urlencoding::encode(&track.title);
        let url = format!("https://lyrics.ovh/sync/{}/{}", artist, title);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Lyrics.ovh request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("Lyrics.ovh request failed".to_string()));
        }

        let text = response.text().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to read lyrics.ovh response: {}", e)))?;

        if !text.trim().is_empty() && text.contains('[') && text.contains(']') {
            let lines = self.parse_lrc_content(&text)?;
            return Ok(SyncedLyrics {
                lines,
                offset: 0,
                source: "Lyrics.ovh".to_string(),
            });
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No synced lyrics found on lyrics.ovh".to_string()))
    }

    /// Try lyrics.ovh for unsynced lyrics
    async fn try_lyrics_ovh_unsynced(&self, track: &TrackMetadata) -> Result<UnsyncedLyrics> {
        let artist = urlencoding::encode(&track.artist);
        let title = urlencoding::encode(&track.title);
        let url = format!("https://api.lyrics.ovh/v1/{}/{}", artist, title);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Lyrics.ovh API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("Lyrics.ovh API request failed".to_string()));
        }

        let lyrics_response: LyricsOvhResponse = response.json().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to parse lyrics.ovh response: {}", e)))?;

        if let Some(lyrics_text) = lyrics_response.lyrics {
            if !lyrics_text.trim().is_empty() {
                return Ok(UnsyncedLyrics {
                    text: self.clean_lyrics_text(&lyrics_text),
                    source: "Lyrics.ovh".to_string(),
                });
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No lyrics found on lyrics.ovh".to_string()))
    }

    /// Try Musixmatch for unsynced lyrics
    async fn try_musixmatch(&self, track: &TrackMetadata) -> Result<UnsyncedLyrics> {
        let api_key = self.api_keys
            .as_ref()
            .and_then(|keys| keys.musixmatch_api_key.as_ref())
            .ok_or_else(|| crate::errors::SpotifyDownloaderError::Lyrics("Musixmatch API key not configured".to_string()))?;

        let url = format!("https://api.musixmatch.com/ws/1.1/matcher.lyrics.get?q_track={}&q_artist={}&apikey={}", 
                         urlencoding::encode(&track.title), 
                         urlencoding::encode(&track.artist),
                         urlencoding::encode(api_key));

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Musixmatch request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("Musixmatch request failed".to_string()));
        }

        let musixmatch_response: MusixmatchResponse = response.json().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to parse Musixmatch response: {}", e)))?;

        if let Some(body) = musixmatch_response.message.body {
            if let Some(lyrics) = body.lyrics {
                if let Some(lyrics_text) = lyrics.lyrics_body {
                    if !lyrics_text.trim().is_empty() {
                        return Ok(UnsyncedLyrics {
                            text: self.clean_lyrics_text(&lyrics_text),
                            source: "Musixmatch".to_string(),
                        });
                    }
                }
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No lyrics found on Musixmatch".to_string()))
    }

    /// Try AZLyrics for unsynced lyrics
    async fn try_azlyrics(&self, track: &TrackMetadata) -> Result<UnsyncedLyrics> {
        let search_query = format!("{} {} site:azlyrics.com", track.artist, track.title);
        let search_url = format!("https://www.google.com/search?q={}", urlencoding::encode(&search_query));

        let response = self.client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("AZLyrics search failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("AZLyrics search failed".to_string()));
        }

        let html = response.text().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to read AZLyrics response: {}", e)))?;

        if let Some(azlyrics_url) = self.extract_azlyrics_url(&html) {
            if let Ok(lyrics_text) = self.fetch_azlyrics_content(&azlyrics_url).await {
                return Ok(UnsyncedLyrics {
                    text: self.clean_lyrics_text(&lyrics_text),
                    source: "AZLyrics".to_string(),
                });
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No lyrics found on AZLyrics".to_string()))
    }

    /// Try Genius for unsynced lyrics
    async fn try_genius(&self, track: &TrackMetadata) -> Result<UnsyncedLyrics> {
        // Try API first if access token is available
        if let Some(api_keys) = &self.api_keys {
            if let Some(access_token) = &api_keys.genius_access_token {
                if let Ok(lyrics) = self.try_genius_api(track, access_token).await {
                    return Ok(lyrics);
                }
            }
        }

        // Fall back to web scraping
        let search_query = format!("{} {}", track.artist, track.title);
        let search_url = format!("https://genius.com/search?q={}", urlencoding::encode(&search_query));

        let response = self.client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Genius search failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("Genius search failed".to_string()));
        }

        let html = response.text().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to read Genius response: {}", e)))?;

        if let Some(lyrics_text) = self.extract_genius_lyrics(&html) {
            return Ok(UnsyncedLyrics {
                text: self.clean_lyrics_text(&lyrics_text),
                source: "Genius".to_string(),
            });
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No lyrics found on Genius".to_string()))
    }

    /// Try Genius API with access token
    async fn try_genius_api(&self, track: &TrackMetadata, access_token: &str) -> Result<UnsyncedLyrics> {
        let search_query = format!("{} {}", track.artist, track.title);
        let url = format!("https://api.genius.com/search?q={}", urlencoding::encode(&search_query));

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Genius API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("Genius API request failed".to_string()));
        }

        // For now, fall back to web scraping as Genius API structure is complex
        // In a real implementation, you'd parse the API response
        Err(crate::errors::SpotifyDownloaderError::Lyrics("Genius API not fully implemented".to_string()))
    }

    /// Parse LRC content into lyrics lines
    fn parse_lrc_content(&self, content: &str) -> Result<Vec<LyricsLine>> {
        let mut lines = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse timestamp and text
            if let Some(bracket_end) = line.find(']') {
                let timestamp_str = &line[1..bracket_end];
                let text = &line[bracket_end + 1..];

                if let Some(timestamp) = self.parse_timestamp(timestamp_str) {
                    lines.push(LyricsLine {
                        timestamp,
                        text: text.to_string(),
                    });
                }
            }
        }

        if lines.is_empty() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("No valid LRC lines found".to_string()));
        }

        Ok(lines)
    }

    /// Parse timestamp from LRC format
    fn parse_timestamp(&self, timestamp_str: &str) -> Option<u32> {
        let parts: Vec<&str> = timestamp_str.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        let minutes: u32 = parts[0].parse().ok()?;
        let seconds_parts: Vec<&str> = parts[1].split('.').collect();
        let seconds: u32 = seconds_parts[0].parse().ok()?;
        let centiseconds: u32 = seconds_parts.get(1).unwrap_or(&"0").parse().ok()?;

        Some(minutes * 60000 + seconds * 1000 + centiseconds * 10)
    }

    /// Clean lyrics text by removing HTML and normalizing
    fn clean_lyrics_text(&self, text: &str) -> String {
        let mut cleaned = text.to_string();

        // Remove HTML tags
        cleaned = self.remove_html_tags(&cleaned);

        // Normalize whitespace
        cleaned = self.normalize_whitespace(&cleaned);

        // Remove common prefixes/suffixes
        cleaned = self.remove_common_prefixes(&cleaned);

        cleaned
    }

    /// Remove HTML tags from text
    fn remove_html_tags(&self, text: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;

        for ch in text.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ => {
                    if !in_tag {
                        result.push(ch);
                    }
                }
            }
        }

        result
    }

    /// Normalize whitespace
    fn normalize_whitespace(&self, text: &str) -> String {
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<&str>>()
            .join("\n")
    }

    /// Remove common prefixes and suffixes
    fn remove_common_prefixes(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Remove common prefixes
        let prefixes = [
            "Lyrics:",
            "Lyrics",
            "Song:",
            "Song",
            "Track:",
            "Track",
        ];

        for prefix in &prefixes {
            if result.starts_with(prefix) {
                result = result[prefix.len()..].trim_start().to_string();
                break;
            }
        }

        // Remove common suffixes
        let suffixes = [
            "More on Genius",
            "Genius",
            "AZLyrics.com",
            "Lyrics provided by",
        ];

        for suffix in &suffixes {
            if result.ends_with(suffix) {
                result = result[..result.len() - suffix.len()].trim_end().to_string();
                break;
            }
        }

        result
    }

    /// Extract AZLyrics URL from search results
    fn extract_azlyrics_url(&self, html: &str) -> Option<String> {
        let pattern = r#"href="(https://www\.azlyrics\.com/lyrics/[^"]+)""#;
        if let Ok(regex) = regex::Regex::new(pattern) {
            if let Some(captures) = regex.captures(html) {
                if let Some(url) = captures.get(1) {
                    return Some(url.as_str().to_string());
                }
            }
        }
        None
    }

    /// Fetch content from AZLyrics
    async fn fetch_azlyrics_content(&self, url: &str) -> Result<String> {
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to fetch AZLyrics: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("AZLyrics fetch failed".to_string()));
        }

        let html = response.text().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to read AZLyrics content: {}", e)))?;

        // Extract lyrics from AZLyrics HTML
        let pattern = r#"<!-- Usage of azlyrics\.com content by any third-party lyrics provider is prohibited by our licensing agreement\. Sorry about that\. -->(.*?)</div>"#;
        if let Ok(regex) = regex::Regex::new(pattern) {
            if let Some(captures) = regex.captures(&html) {
                if let Some(lyrics_html) = captures.get(1) {
                    return Ok(self.clean_lyrics_text(lyrics_html.as_str()));
                }
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("Could not extract lyrics from AZLyrics".to_string()))
    }

    /// Extract lyrics from Genius HTML
    fn extract_genius_lyrics(&self, html: &str) -> Option<String> {
        let patterns = [
            r#"<div[^>]*class="[^"]*lyrics[^"]*"[^>]*>(.*?)</div>"#,
            r#"<div[^>]*data-lyrics-container[^>]*>(.*?)</div>"#,
        ];

        for pattern in &patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(html) {
                    if let Some(lyrics_html) = captures.get(1) {
                        let lyrics = self.clean_lyrics_text(lyrics_html.as_str());
                        if !lyrics.trim().is_empty() {
                            return Some(lyrics);
                        }
                    }
                }
            }
        }

        None
    }

    /// Save synced lyrics to LRC file
    async fn save_synced_lyrics(
        &self,
        lyrics: &SyncedLyrics,
        track: &TrackMetadata,
        output_dir: &PathBuf,
    ) -> Result<PathBuf> {
        let mut path = output_dir.clone();
        path.push(sanitize_filename(&track.artist));
        path.push(sanitize_filename(&track.album));

        // Ensure directory exists
        std::fs::create_dir_all(&path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to create directory: {}", e)))?;

        let formatted_artist = format_artists_for_filename(&track.artist);
        let filename = format!("{} - {}.lrc", track.track_number.unwrap_or(1), sanitize_filename(&format!("{} - {}", formatted_artist, track.title)));
        path.push(filename);

        // Convert to LRC format
        let lrc_content = self.convert_to_lrc(lyrics);

        // Write to file
        std::fs::write(&path, lrc_content)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to write LRC file: {}", e)))?;

        Ok(path)
    }

    /// Save unsynced lyrics to text file
    async fn save_unsynced_lyrics(
        &self,
        lyrics: &UnsyncedLyrics,
        track: &TrackMetadata,
        output_dir: &PathBuf,
    ) -> Result<PathBuf> {
        let mut path = output_dir.clone();
        path.push(sanitize_filename(&track.artist));
        path.push(sanitize_filename(&track.album));

        // Ensure directory exists
        std::fs::create_dir_all(&path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to create directory: {}", e)))?;

        let formatted_artist = format_artists_for_filename(&track.artist);
        let filename = format!("{} - {}.txt", track.track_number.unwrap_or(1), sanitize_filename(&format!("{} - {}", formatted_artist, track.title)));
        path.push(filename);

        // Write to file
        std::fs::write(&path, &lyrics.text)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to write lyrics file: {}", e)))?;

        Ok(path)
    }

    /// Convert synced lyrics to LRC format
    fn convert_to_lrc(&self, lyrics: &SyncedLyrics) -> String {
        let mut lrc = String::new();

        // Add offset if present
        if lyrics.offset != 0 {
            lrc.push_str(&format!("[offset:{}]\n", lyrics.offset));
        }

        // Add lyrics lines
        for line in &lyrics.lines {
            let minutes = line.timestamp / 60000;
            let seconds = (line.timestamp % 60000) / 1000;
            let milliseconds = line.timestamp % 1000;

            lrc.push_str(&format!(
                "[{:02}:{:02}.{:02}]{}\n",
                minutes, seconds, milliseconds / 10, line.text
            ));
        }

        lrc
    }

    /// Generate optimized search query variations for better lyrics matching
    fn generate_search_queries(&self, track: &TrackMetadata) -> Vec<String> {
        let mut queries = Vec::new();
        
        // Only try the most effective queries to reduce search time
        queries.push(format!("{} {}", track.artist, track.title));
        queries.push(format!("{} - {}", track.artist, track.title));
        queries.push(format!("\"{}\" \"{}\"", track.artist, track.title));
        
        // Remove duplicates while preserving order
        let mut unique_queries = Vec::new();
        for query in queries {
            if !unique_queries.contains(&query) {
                unique_queries.push(query);
            }
        }
        
        unique_queries
    }

    /// Clean text for better search matching
    fn clean_for_search(&self, text: &str) -> String {
        let mut cleaned = text.to_string();
        
        // Remove common words that might interfere with search
        let common_words = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
            "feat", "featuring", "ft", "ft.", "feat.", "featuring.", "&", "vs", "vs.", "x", "X"
        ];
        
        for word in &common_words {
            let pattern = format!(r"\b{}\b", regex::escape(word));
            if let Ok(regex) = regex::Regex::new(&pattern) {
                cleaned = regex.replace_all(&cleaned, "").to_string();
            }
        }
        
        // Normalize whitespace
        cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        cleaned.trim().to_string()
    }

    /// Try LRClib for unsynced lyrics with specific query
    async fn try_lrclib_unsynced_with_query(&self, _track: &TrackMetadata, query: &str) -> Result<UnsyncedLyrics> {
        let url = format!("https://lrclib.net/api/search?q={}", urlencoding::encode(query));

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("LRClib request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("LRClib request failed".to_string()));
        }

        let lrclib_response: LRClibResponse = response.json().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to parse LRClib response: {}", e)))?;

        if let Some(plain_lyrics) = lrclib_response.plain_lyrics {
            if !plain_lyrics.trim().is_empty() {
                return Ok(UnsyncedLyrics {
                    text: self.clean_lyrics_text(&plain_lyrics),
                    source: "LRClib".to_string(),
                });
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No unsynced lyrics found on LRClib".to_string()))
    }

    /// Try lyrics.ovh for unsynced lyrics with specific query
    async fn try_lyrics_ovh_unsynced_with_query(&self, track: &TrackMetadata, query: &str) -> Result<UnsyncedLyrics> {
        // Extract artist and title from query for URL encoding
        let parts: Vec<&str> = query.splitn(2, ' ').collect();
        let artist = if parts.len() >= 1 { parts[0] } else { &track.artist };
        let title = if parts.len() >= 2 { parts[1] } else { &track.title };
        
        let artist_encoded = urlencoding::encode(artist);
        let title_encoded = urlencoding::encode(title);
        let url = format!("https://api.lyrics.ovh/v1/{}/{}", artist_encoded, title_encoded);

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Lyrics.ovh API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("Lyrics.ovh API request failed".to_string()));
        }

        let lyrics_response: LyricsOvhResponse = response.json().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to parse lyrics.ovh response: {}", e)))?;

        if let Some(lyrics_text) = lyrics_response.lyrics {
            if !lyrics_text.trim().is_empty() {
                return Ok(UnsyncedLyrics {
                    text: self.clean_lyrics_text(&lyrics_text),
                    source: "Lyrics.ovh".to_string(),
                });
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No lyrics found on lyrics.ovh".to_string()))
    }

    /// Try Musixmatch for unsynced lyrics with specific query
    async fn try_musixmatch_with_query(&self, track: &TrackMetadata, query: &str) -> Result<UnsyncedLyrics> {
        let api_key = self.api_keys
            .as_ref()
            .and_then(|keys| keys.musixmatch_api_key.as_ref())
            .ok_or_else(|| crate::errors::SpotifyDownloaderError::Lyrics("Musixmatch API key not configured".to_string()))?;

        // Extract artist and title from query
        let parts: Vec<&str> = query.splitn(2, ' ').collect();
        let title = if parts.len() >= 2 { parts[1] } else { &track.title };
        let artist = if parts.len() >= 1 { parts[0] } else { &track.artist };

        let url = format!("https://api.musixmatch.com/ws/1.1/matcher.lyrics.get?q_track={}&q_artist={}&apikey={}", 
                         urlencoding::encode(title), 
                         urlencoding::encode(artist),
                         urlencoding::encode(api_key));

        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Musixmatch request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("Musixmatch request failed".to_string()));
        }

        let musixmatch_response: MusixmatchResponse = response.json().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to parse Musixmatch response: {}", e)))?;

        if let Some(body) = musixmatch_response.message.body {
            if let Some(lyrics) = body.lyrics {
                if let Some(lyrics_text) = lyrics.lyrics_body {
                    if !lyrics_text.trim().is_empty() {
                        return Ok(UnsyncedLyrics {
                            text: self.clean_lyrics_text(&lyrics_text),
                            source: "Musixmatch".to_string(),
                        });
                    }
                }
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No lyrics found on Musixmatch".to_string()))
    }

    /// Try AZLyrics for unsynced lyrics with specific query
    async fn try_azlyrics_with_query(&self, _track: &TrackMetadata, query: &str) -> Result<UnsyncedLyrics> {
        let search_query = format!("{} site:azlyrics.com", query);
        let search_url = format!("https://www.google.com/search?q={}", urlencoding::encode(&search_query));

        let response = self.client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("AZLyrics search failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("AZLyrics search failed".to_string()));
        }

        let html = response.text().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to read AZLyrics response: {}", e)))?;

        if let Some(azlyrics_url) = self.extract_azlyrics_url(&html) {
            if let Ok(lyrics_text) = self.fetch_azlyrics_content(&azlyrics_url).await {
                return Ok(UnsyncedLyrics {
                    text: self.clean_lyrics_text(&lyrics_text),
                    source: "AZLyrics".to_string(),
                });
            }
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No lyrics found on AZLyrics".to_string()))
    }

    /// Try Genius for unsynced lyrics with specific query
    async fn try_genius_with_query(&self, track: &TrackMetadata, query: &str) -> Result<UnsyncedLyrics> {
        // Try API first if access token is available
        if let Some(api_keys) = &self.api_keys {
            if let Some(access_token) = &api_keys.genius_access_token {
                if let Ok(lyrics) = self.try_genius_api_with_query(track, query, access_token).await {
                    return Ok(lyrics);
                }
            }
        }

        // Fall back to web scraping
        let search_url = format!("https://genius.com/search?q={}", urlencoding::encode(query));

        let response = self.client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Genius search failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("Genius search failed".to_string()));
        }

        let html = response.text().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Failed to read Genius response: {}", e)))?;

        if let Some(lyrics_text) = self.extract_genius_lyrics(&html) {
            return Ok(UnsyncedLyrics {
                text: self.clean_lyrics_text(&lyrics_text),
                source: "Genius".to_string(),
            });
        }

        Err(crate::errors::SpotifyDownloaderError::Lyrics("No lyrics found on Genius".to_string()))
    }

    /// Try Genius API with access token and specific query
    async fn try_genius_api_with_query(&self, _track: &TrackMetadata, query: &str, access_token: &str) -> Result<UnsyncedLyrics> {
        let url = format!("https://api.genius.com/search?q={}", urlencoding::encode(query));

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Lyrics(format!("Genius API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Lyrics("Genius API request failed".to_string()));
        }

        // For now, fall back to web scraping as Genius API structure is complex
        // In a real implementation, you'd parse the API response
        Err(crate::errors::SpotifyDownloaderError::Lyrics("Genius API not fully implemented".to_string()))
    }
}

/// Format artists for filename with proper comma separation
fn format_artists_for_filename(artist: &str) -> String {
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
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '<' | '>' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ';' => ',', // Replace semicolon with comma
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
