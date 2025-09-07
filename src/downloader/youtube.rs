use crate::errors::{Result, SpotifyDownloaderError};
use crate::config::{AudioFormat, Bitrate};
use std::path::PathBuf;
use tokio::process::Command as AsyncCommand;
use serde::{Deserialize, Serialize};

/// YouTube and SoundCloud downloader using yt-dlp
pub struct YoutubeDownloader {
    executable_path: String,
}

impl YoutubeDownloader {
    /// Create a new YouTube/SoundCloud downloader
    pub fn new() -> Self {
        Self {
            executable_path: "yt-dlp".to_string(),
        }
    }

    /// Create a new downloader with custom executable path
    #[allow(dead_code)]
    pub fn with_path(executable_path: String) -> Self {
        Self { executable_path }
    }

    /// Check if yt-dlp is available
    #[allow(dead_code)]
    pub async fn is_available(&self) -> bool {
        let output = AsyncCommand::new(&self.executable_path)
            .arg("--version")
            .output()
            .await;

        output.is_ok()
    }

    /// Optimized search strategy: try ytsearch1 -> ytsearch5 -> scsearch1 -> scsearch5
    pub async fn search_optimized(&self, query: &str, config: &crate::config::Config) -> Result<Vec<SearchResult>> {
        let mut all_results = Vec::new();

        // Try ytsearch1 first (fastest)
        if let Ok(mut results) = self.search_youtube(query, 1, config).await {
            all_results.append(&mut results);
        }

        // Try ytsearch5 if we need more results
        if all_results.is_empty() {
            if let Ok(mut results) = self.search_youtube(query, 5, config).await {
                all_results.append(&mut results);
            }
        }

        // Try scsearch1 if YouTube failed
        if all_results.is_empty() {
            if let Ok(mut results) = self.search_soundcloud(query, 1, config).await {
                all_results.append(&mut results);
            }
        }

        // Try scsearch5 as last resort
        if all_results.is_empty() {
            if let Ok(mut results) = self.search_soundcloud(query, 5, config).await {
                all_results.append(&mut results);
            }
        }

        // Filter results to only include valid tracks
        let filtered_results: Vec<SearchResult> = all_results.into_iter()
            .filter(|result| {
                let is_track = self.is_track(&result.title);
                let is_valid_duration = self.is_valid_duration(result.duration);
                is_track && is_valid_duration
            })
            .collect();

        // Sort by relevance (view count as proxy for popularity)
        let mut sorted_results = filtered_results;
        sorted_results.sort_by(|a, b| b.view_count.cmp(&a.view_count));

        if !sorted_results.is_empty() {
            println!("✅ Found {} filtered results", sorted_results.len());
        }

        Ok(sorted_results)
    }

    /// Search for tracks on YouTube and SoundCloud (legacy method)
    #[allow(dead_code)]
    pub async fn search(&self, query: &str, max_results: u32, config: &crate::config::Config) -> Result<Vec<SearchResult>> {
        let mut all_results = Vec::new();

        // Search YouTube first
        if let Ok(mut youtube_results) = self.search_youtube(query, max_results, config).await {
            all_results.append(&mut youtube_results);
        }

        // Search SoundCloud
        if let Ok(mut soundcloud_results) = self.search_soundcloud(query, max_results, config).await {
            all_results.append(&mut soundcloud_results);
        }

        // Filter results to only include valid tracks
        let filtered_results: Vec<SearchResult> = all_results.into_iter()
            .filter(|result| {
                let is_track = self.is_track(&result.title);
                let is_valid_duration = self.is_valid_duration(result.duration);
                is_track && is_valid_duration
            })
            .collect();

        // Sort by relevance (view count as proxy for popularity)
        let mut sorted_results = filtered_results;
        sorted_results.sort_by(|a, b| b.view_count.cmp(&a.view_count));

        Ok(sorted_results)
    }

    /// Search YouTube specifically using ytsearch5 for better results
    pub async fn search_youtube(&self, query: &str, max_results: u32, config: &crate::config::Config) -> Result<Vec<SearchResult>> {
        let search_limit = std::cmp::min(max_results, 5);
        let search_url = format!("ytsearch{}:{}", search_limit, query);
        self.search_with_yt_dlp(&search_url, "YouTube", config).await
    }

    /// Search SoundCloud specifically using scsearch3
    pub async fn search_soundcloud(&self, query: &str, max_results: u32, config: &crate::config::Config) -> Result<Vec<SearchResult>> {
        let search_url = format!("scsearch{}:{}", max_results, query);
        self.search_with_yt_dlp(&search_url, "SoundCloud", config).await
    }

    /// Generic search using yt-dlp with optimized settings
    async fn search_with_yt_dlp(&self, search_url: &str, platform: &str, config: &crate::config::Config) -> Result<Vec<SearchResult>> {
        let mut cmd = AsyncCommand::new(&self.executable_path);
        cmd.arg(search_url)
            .arg("--dump-json")
            .arg("--no-playlist")
            .arg("--quiet")
            .arg("--socket-timeout")
            .arg("30") // Increased timeout for better reliability
            .arg("--retries")
            .arg("3") // More retries for better success rate
            .arg("--no-check-certificate") // Skip SSL verification for speed
            .arg("--prefer-free-formats") // Prefer free formats
            .arg("--format") // Prefer audio-only formats
            .arg("bestaudio[ext=m4a]/bestaudio[ext=mp3]/bestaudio")
            .arg("--user-agent").arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36") // Modern user agent
            .arg("--extractor-retries").arg("3") // Retry extractor operations
            .arg("--fragment-retries").arg("3"); // Retry fragment downloads

        // Add proxy support if enabled
        if config.proxy_config.enabled {
            let proxy_url = if let (Some(username), Some(password)) = (&config.proxy_config.username, &config.proxy_config.password) {
                format!("http://{}:{}@{}:{}", username, password, config.proxy_config.host, config.proxy_config.port)
            } else {
                format!("http://{}:{}", config.proxy_config.host, config.proxy_config.port)
            };
            cmd.arg("--proxy").arg(proxy_url);
        }

        // Cookies disabled - no cookie support
        
        let output = cmd.output().await
            .map_err(|e| SpotifyDownloaderError::Youtube(format!("Failed to execute yt-dlp search: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpotifyDownloaderError::Youtube(format!("yt-dlp search failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();
        
        // Parse JSON flexibly to handle both YouTube and SoundCloud formats
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
            let title = json_value.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown Title")
                .to_string();
            
            let id = json_value.get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let duration = json_value.get("duration")
                .and_then(|v| v.as_u64())
                .map(|d| d as u32);
            
            let uploader = json_value.get("uploader")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let view_count = json_value.get("view_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            let thumbnail = json_value.get("thumbnail")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            // Construct URL based on platform
            let url = if platform == "SoundCloud" {
                // SoundCloud format
                if let Some(uploader_name) = &uploader {
                    format!("https://soundcloud.com/{}/{}", 
                        uploader_name.to_lowercase().replace(" ", "-"),
                        id)
                } else {
                    format!("https://soundcloud.com/track/{}", id)
                }
            } else {
                // YouTube format
                json_value.get("webpage_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("https://youtube.com/watch?v={}", id))
            };
            
            results.push(SearchResult {
                title,
                url,
                duration,
                uploader,
                view_count,
                platform: platform.to_string(),
                thumbnail,
            });
        }

        Ok(results)
    }

    /// Download audio from URL using yt-dlp
    pub async fn download_audio(
        &self,
        url: &str,
        output_path: &PathBuf,
        format: AudioFormat,
        bitrate: Bitrate,
        progress_callback: Option<Box<dyn Fn(f32) + Send + Sync>>,
        config: &crate::config::Config,
    ) -> Result<()> {
        self.download_with_yt_dlp(url, output_path, &format, &bitrate, &progress_callback, config).await
    }
    
    
    /// Download using yt-dlp
    async fn download_with_yt_dlp(
        &self,
        url: &str,
        output_path: &PathBuf,
        format: &AudioFormat,
        bitrate: &Bitrate,
        progress_callback: &Option<Box<dyn Fn(f32) + Send + Sync>>,
        config: &crate::config::Config,
    ) -> Result<()> {
        // Create output directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| SpotifyDownloaderError::Youtube(format!("Failed to create output directory: {}", e)))?;
        }
        
        let output_dir = output_path.parent()
            .ok_or_else(|| SpotifyDownloaderError::Youtube("Invalid output path".to_string()))?
            .to_path_buf();

        let output_template = output_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SpotifyDownloaderError::Youtube("Invalid output filename".to_string()))?;
        
        // Create temp directory for yt-dlp part files
        let mut temp_dir = output_dir.clone();
        temp_dir.push("temp");
        tokio::fs::create_dir_all(&temp_dir).await
            .map_err(|e| SpotifyDownloaderError::Youtube(format!("Failed to create temp directory: {}", e)))?;
        
        // Build yt-dlp command with optimized settings
        let mut cmd = AsyncCommand::new(&self.executable_path);
        cmd.arg(url)
            .arg("--extract-audio")
            .arg("--audio-format").arg(format.to_string())
            .arg("--audio-quality").arg(&format!("{}", bitrate.as_u32()))
            .arg("--output").arg(format!("{}/{}", output_dir.display(), output_template))
            .arg("--paths").arg(format!("temp:{}", temp_dir.display())) // Use temp directory for part files
            .arg("--no-playlist")
            .arg("--progress")
            .arg("--newline")
            .arg("--no-check-certificate") // Skip SSL verification for speed
            .arg("--prefer-free-formats") // Prefer free formats
            .arg("--socket-timeout").arg("30") // Reasonable timeout
            .arg("--retries").arg("3") // Retry on failure
            .arg("--fragment-retries").arg("3") // Retry fragments
            .arg("--concurrent-fragments").arg("4") // Download fragments concurrently
            .arg("--buffer-size").arg("16K") // Smaller buffer for faster startup
            .arg("--http-chunk-size").arg("1M") // Larger chunks for better performance
            .arg("--user-agent").arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"); // Modern user agent

        // Add proxy support if enabled
        if config.proxy_config.enabled {
            let proxy_url = if let (Some(username), Some(password)) = (&config.proxy_config.username, &config.proxy_config.password) {
                format!("http://{}:{}@{}:{}", username, password, config.proxy_config.host, config.proxy_config.port)
            } else {
                format!("http://{}:{}", config.proxy_config.host, config.proxy_config.port)
            };
            cmd.arg("--proxy").arg(proxy_url);
        }

        // Add SponsorBlock support if enabled
        if config.sponsorblock_config.enabled {
            let categories = config.sponsorblock_config.remove_categories.join(",");
            cmd.arg("--sponsorblock-remove").arg(categories);
        }

        // Cookies disabled - no cookie support

        // Execute command with progress monitoring
        let mut child = cmd.spawn()
            .map_err(|e| SpotifyDownloaderError::Youtube(format!("Failed to spawn yt-dlp: {}", e)))?;

        // For now, we'll use a simpler approach without progress monitoring
        // TODO: Implement proper progress monitoring with yt-dlp
        if progress_callback.is_some() {
            // Just call the callback with 100% progress when done
            // This is a temporary solution until we implement proper progress parsing
            if let Some(callback) = progress_callback {
                callback(1.0);
            }
        }
        
        // Wait for command to complete
        let status = child.wait().await
            .map_err(|e| SpotifyDownloaderError::Youtube(format!("Failed to wait for yt-dlp: {}", e)))?;
        
        if !status.success() {
            return Err(SpotifyDownloaderError::Youtube(format!("yt-dlp failed with status: {}", status)));
        }
        
        // Find the downloaded file and rename it to the expected output path
        self.rename_downloaded_file(&output_dir, &output_template, &format.to_string(), output_path).await?;

        // Clean up any remaining temp files
        self.cleanup_temp_files(&temp_dir).await?;

        Ok(())
    }

    /// Parse progress line from yt-dlp output
    #[allow(dead_code)]
    fn parse_progress_line(&self, line: &str) -> Option<f32> {
        Self::parse_progress_line_static(line)
    }

    /// Parse progress line from yt-dlp output (static version)
    #[allow(dead_code)]
    fn parse_progress_line_static(line: &str) -> Option<f32> {
        // yt-dlp progress format: [download] 45.2% of 12.34MiB at 1.23MiB/s ETA 00:05
        if let Some(start) = line.find("[download]") {
            let progress_part = &line[start + 10..];
            if let Some(percent_start) = progress_part.find('%') {
                let percent_str = &progress_part[..percent_start].trim();
                if let Ok(percent) = percent_str.parse::<f32>() {
                    return Some(percent / 100.0);
                }
            }
        }
        None
    }

    /// Rename downloaded file to expected output path
    async fn rename_downloaded_file(
        &self,
        output_dir: &PathBuf,
        base_name: &str,
        format: &str,
        expected_path: &PathBuf,
    ) -> Result<()> {
        if let Ok(entries) = tokio::fs::read_dir(output_dir).await {
            let mut found_file = None;
            let mut entries = entries;
            
            while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.starts_with(base_name) && file_name.ends_with(&format!(".{}", format)) {
                        found_file = Some(entry.path());
                        break;
                    }
                }
            }
            
            if let Some(downloaded_file) = found_file {
                tokio::fs::rename(&downloaded_file, expected_path).await
                    .map_err(|e| SpotifyDownloaderError::Youtube(format!("Failed to rename downloaded file: {}", e)))?;
            } else {
                return Err(SpotifyDownloaderError::Youtube("Downloaded file not found".to_string()));
            }
        }
        
        Ok(())
    }

    /// Clean up temporary files from temp directory
    async fn cleanup_temp_files(&self, temp_dir: &PathBuf) -> Result<()> {
        if let Ok(mut entries) = tokio::fs::read_dir(temp_dir).await {
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    // Only delete files that look like temp files (contain "temp" or are part files)
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.contains("temp") || file_name.contains(".part") || file_name.contains(".ytdl") {
                            if let Err(e) = tokio::fs::remove_file(&path).await {
                                println!("Warning: Failed to clean up temp file {}: {}", path.display(), e);
                            } else {
                                println!("Cleaned up temp file: {}", path.display());
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Check if the title represents a track (not album, mixtape, EP, etc.)
    fn is_track(&self, title: &str) -> bool {
        let title_lower = title.to_lowercase();
        
        // Keywords that indicate non-track content
        let non_track_keywords = [
            "album", "mixtape", "ep", "compilation", "collection", "playlist",
            "full album", "complete album", "deluxe", "extended", "remastered",
            "live album", "studio album", "best of", "greatest hits",
            "soundtrack", "score", "instrumental", "acoustic", "unplugged",
            "remix album", "dubstep", "house", "techno", "trance", "drum and bass",
            "mix", "mashup", "mash-up", "bootleg", "unofficial", "full mix",
            "continuous mix", "dj mix", "radio show", "podcast", "interview"
        ];

        // Check if title contains non-track keywords
        for keyword in &non_track_keywords {
            if title_lower.contains(keyword) {
                return false;
            }
        }

        // Check for patterns that indicate albums
        if title_lower.contains(" - ") && title_lower.split(" - ").count() > 2 {
            return false; // Likely "Artist - Album - Track" format
        }

        // Check for duration patterns in title (e.g., "1:23:45" indicates long content)
        if regex::Regex::new(r"\d{1,2}:\d{2}:\d{2}").unwrap().is_match(&title_lower) {
            return false; // Likely a long-form content
        }

        true
    }

    /// Check if duration is valid (between 1:01 and 16:00 minutes)
    fn is_valid_duration(&self, duration: Option<u32>) -> bool {
        match duration {
            Some(dur) => dur >= 61 && dur <= 960, // 1:01 to 16:00 minutes
            None => true, // If duration is unknown, allow it
        }
    }

    /// Get video/audio information
    #[allow(dead_code)]
    pub async fn get_info(&self, url: &str) -> Result<YtDlpInfo> {
        let mut cmd = AsyncCommand::new(&self.executable_path);
        cmd.arg(url)
            .arg("--dump-json")
            .arg("--no-playlist")
            .arg("--quiet");

        let output = cmd.output().await
            .map_err(|e| SpotifyDownloaderError::Youtube(format!("Failed to execute yt-dlp: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpotifyDownloaderError::Youtube(format!("yt-dlp failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let info: YtDlpInfo = serde_json::from_str(&stdout)
            .map_err(|e| SpotifyDownloaderError::Youtube(format!("Failed to parse yt-dlp output: {}", e)))?;

        Ok(info)
    }

    /// Check if URL is supported by yt-dlp
    #[allow(dead_code)]
    pub async fn is_url_supported(&self, url: &str) -> bool {
        let mut cmd = AsyncCommand::new(&self.executable_path);
        cmd.arg("--dump-json")
            .arg("--no-playlist")
            .arg("--quiet")
            .arg(url);

        let output = cmd.output().await;
        output.is_ok() && output.unwrap().status.success()
    }
}

/// Search result from YouTube or SoundCloud
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub duration: Option<u32>,
    pub uploader: Option<String>,
    pub view_count: u64,
    pub platform: String,
    pub thumbnail: Option<String>,
}

/// yt-dlp information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtDlpInfo {
    pub id: String,
    pub title: String,
    pub duration: Option<u32>,
    pub uploader: Option<String>,
    pub view_count: Option<u64>,
    pub webpage_url: String,
    pub thumbnail: Option<String>,
    pub description: Option<String>,
    pub upload_date: Option<String>,
    pub formats: Option<Vec<YtDlpFormat>>,
}

/// yt-dlp format information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtDlpFormat {
    pub format_id: String,
    pub ext: String,
    pub acodec: Option<String>,
    pub vcodec: Option<String>,
    pub abr: Option<f32>, // audio bitrate
    pub vbr: Option<f32>, // video bitrate
    pub filesize: Option<u64>,
    pub url: String,
}

impl Default for YoutubeDownloader {
    fn default() -> Self {
        Self::new()
    }
}