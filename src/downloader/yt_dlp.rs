use crate::errors::{Result, SpotifyDownloaderError};
use std::path::PathBuf;
use tokio::process::Command as AsyncCommand;

/// yt-dlp subprocess wrapper for fallback downloads
pub struct YtDlpDownloader {
    executable_path: String,
}

impl YtDlpDownloader {
    /// Create a new yt-dlp downloader
    pub fn new() -> Self {
        Self {
            executable_path: "yt-dlp".to_string(),
        }
    }

    /// Create a new yt-dlp downloader with custom executable path
    pub fn with_path(executable_path: String) -> Self {
        Self { executable_path }
    }

    /// Check if yt-dlp is available
    pub async fn is_available(&self) -> bool {
        let output = AsyncCommand::new(&self.executable_path)
            .arg("--version")
            .output()
            .await;

        output.is_ok()
    }

    /// Download audio from URL using yt-dlp
    pub async fn download_audio(
        &self,
        url: &str,
        output_path: &PathBuf,
        format: &str,
        bitrate: u32,
    ) -> Result<()> {
        let output_dir = output_path.parent()
            .ok_or_else(|| SpotifyDownloaderError::Download("Invalid output path".to_string()))?;

        let _output_template = output_path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SpotifyDownloaderError::Download("Invalid output filename".to_string()))?;

        let mut cmd = AsyncCommand::new(&self.executable_path);
        cmd.arg(url)
            .arg("--extract-audio")
            .arg("--audio-format").arg(format)
            .arg("--audio-quality").arg(&format!("{}", bitrate))
            .arg("--output").arg(format!("{}/%(title)s.%(ext)s", output_dir.display()))
            .arg("--no-playlist")
            .arg("--quiet");

        let output = cmd.output().await
            .map_err(|e| SpotifyDownloaderError::Download(format!("Failed to execute yt-dlp: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpotifyDownloaderError::Download(format!("yt-dlp failed: {}", stderr)));
        }

        Ok(())
    }

    /// Get video/audio information using yt-dlp
    pub async fn get_info(&self, url: &str) -> Result<YtDlpInfo> {
        let mut cmd = AsyncCommand::new(&self.executable_path);
        cmd.arg(url)
            .arg("--dump-json")
            .arg("--no-playlist")
            .arg("--quiet");

        let output = cmd.output().await
            .map_err(|e| SpotifyDownloaderError::Download(format!("Failed to execute yt-dlp: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpotifyDownloaderError::Download(format!("yt-dlp failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let info: YtDlpInfo = serde_json::from_str(&stdout)
            .map_err(|e| SpotifyDownloaderError::Download(format!("Failed to parse yt-dlp output: {}", e)))?;

        Ok(info)
    }

    /// Search for videos using yt-dlp
    pub async fn search(&self, query: &str, max_results: u32) -> Result<Vec<YtDlpSearchResult>> {
        let search_url = format!("ytsearch{}:{}", max_results, query);

        let mut cmd = AsyncCommand::new(&self.executable_path);
        cmd.arg(&search_url)
            .arg("--dump-json")
            .arg("--quiet");

        let output = cmd.output().await
            .map_err(|e| SpotifyDownloaderError::Download(format!("Failed to execute yt-dlp: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpotifyDownloaderError::Download(format!("yt-dlp search failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        
        let mut results = Vec::new();
        for line in lines {
            if let Ok(info) = serde_json::from_str::<YtDlpInfo>(line) {
                results.push(YtDlpSearchResult {
                    title: info.title,
                    url: info.webpage_url,
                    duration: info.duration,
                    uploader: info.uploader,
                    view_count: info.view_count,
                });
            }
        }

        Ok(results)
    }
}

/// yt-dlp information structure
#[derive(Debug, serde::Deserialize)]
pub struct YtDlpInfo {
    pub id: String,
    pub title: String,
    pub duration: Option<u32>,
    pub uploader: Option<String>,
    pub view_count: Option<u64>,
    pub webpage_url: String,
    pub formats: Option<Vec<YtDlpFormat>>,
}

/// yt-dlp format information
#[derive(Debug, serde::Deserialize)]
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

/// yt-dlp search result
#[derive(Debug, Clone)]
pub struct YtDlpSearchResult {
    pub title: String,
    pub url: String,
    pub duration: Option<u32>,
    pub uploader: Option<String>,
    pub view_count: Option<u64>,
}
