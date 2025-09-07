pub mod fs;
pub mod net;
pub mod logger;


use crate::errors::Result;
use std::path::PathBuf;

/// Utility functions for the application
pub struct Utils;

impl Utils {
    /// Sanitize filename by removing invalid characters and replacing semicolons with commas
    pub fn sanitize_filename(filename: &str) -> String {
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

    /// Format file size in human readable format
    pub fn format_file_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Format duration in human readable format
    pub fn format_duration(seconds: u32) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes, secs)
        } else {
            format!("{}:{:02}", minutes, secs)
        }
    }

    /// Format duration in milliseconds
    pub fn format_duration_ms(milliseconds: u32) -> String {
        Self::format_duration(milliseconds / 1000)
    }

    /// Get file extension from path
    pub fn get_file_extension(path: &PathBuf) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }

    /// Check if file exists and is readable
    pub fn is_file_readable(path: &PathBuf) -> bool {
        path.exists() && path.is_file() && std::fs::metadata(path).map(|m| m.permissions().readonly()).unwrap_or(false)
    }

    /// Check if directory exists and is writable
    pub fn is_directory_writable(path: &PathBuf) -> bool {
        path.exists() && path.is_dir() && std::fs::metadata(path).map(|m| !m.permissions().readonly()).unwrap_or(false)
    }

    /// Create directory if it doesn't exist
    pub fn ensure_directory_exists(path: &PathBuf) -> Result<()> {
        if !path.exists() {
            std::fs::create_dir_all(path)
                .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        }
        Ok(())
    }

    /// Get relative path from base
    pub fn get_relative_path(path: &PathBuf, base: &PathBuf) -> Result<PathBuf> {
        path.strip_prefix(base)
            .map(|p| p.to_path_buf())
            .map_err(|_| crate::errors::SpotifyDownloaderError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Path is not relative to base"
            )))
    }

    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        url::Url::parse(url).is_ok()
    }

    /// Extract domain from URL
    pub fn extract_domain(url: &str) -> Option<String> {
        url::Url::parse(url)
            .ok()
            .and_then(|url| url.host_str().map(|s| s.to_string()))
    }

    /// Check if URL is a Spotify URL
    pub fn is_spotify_url(url: &str) -> bool {
        Self::extract_domain(url)
            .map(|domain| domain.contains("spotify.com"))
            .unwrap_or(false)
    }

    /// Check if URL is a YouTube URL
    pub fn is_youtube_url(url: &str) -> bool {
        Self::extract_domain(url)
            .map(|domain| domain.contains("youtube.com") || domain.contains("youtu.be"))
            .unwrap_or(false)
    }

    /// Check if URL is a SoundCloud URL
    pub fn is_soundcloud_url(url: &str) -> bool {
        Self::extract_domain(url)
            .map(|domain| domain.contains("soundcloud.com"))
            .unwrap_or(false)
    }

    /// Generate unique filename to avoid conflicts
    pub fn generate_unique_filename(base_path: &PathBuf) -> PathBuf {
        if !base_path.exists() {
            return base_path.clone();
        }

        let parent = base_path.parent().unwrap_or(base_path);
        let stem = base_path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let extension = base_path.extension().and_then(|s| s.to_str()).unwrap_or("");

        let mut counter = 1;
        loop {
            let new_name = if extension.is_empty() {
                format!("{} ({})", stem, counter)
            } else {
                format!("{} ({}).{}", stem, counter, extension)
            };
            
            let new_path = parent.join(new_name);
            if !new_path.exists() {
                return new_path;
            }
            counter += 1;
        }
    }

    /// Calculate file hash (MD5)
    pub fn calculate_file_hash(path: &PathBuf) -> Result<String> {
        use std::fs::File;
        use std::io::Read;
        
        let mut file = File::open(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;

        // Simple hash implementation (in production, use a proper crypto library)
        let hash = format!("{:x}", md5::compute(&buffer));
        Ok(hash)
    }

    /// Retry operation with exponential backoff
    pub async fn retry_with_backoff<F, T, E>(
        operation: F,
        max_retries: u32,
        initial_delay_ms: u64,
    ) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send>>,
        E: std::fmt::Display,
    {
        let mut delay = initial_delay_ms;
        
        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempt == max_retries {
                        return Err(crate::errors::SpotifyDownloaderError::Unknown(
                            format!("Operation failed after {} retries: {}", max_retries + 1, e)
                        ));
                    }
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    delay *= 2; // Exponential backoff
                }
            }
        }
        
        unreachable!()
    }
}
