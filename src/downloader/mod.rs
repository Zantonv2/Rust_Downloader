pub mod spotify;
pub mod itunes;
pub mod youtube;
pub mod soundcloud;
pub mod yt_dlp;
pub mod audio;
pub mod converter;
pub mod covers;
pub mod metadata;
pub mod api_wrapper;
pub mod async_manager;

pub use audio::AudioDownloader;
pub use async_manager::{AsyncDownloadManager, DownloadTaskResult};

use crate::config::{AudioFormat, Bitrate};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Track metadata from Spotify
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackMetadata {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub release_date: Option<String>,
    pub duration_ms: u32,
    pub genres: Vec<String>,
    pub spotify_url: String,
    pub preview_url: Option<String>,
    pub external_urls: std::collections::HashMap<String, String>,
    pub album_cover_url: Option<String>,
    // Additional fields for UI compatibility
    pub composer: Option<String>,
    pub comment: Option<String>,
}

/// Album metadata from Spotify
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumMetadata {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub release_date: String,
    pub total_tracks: u32,
    pub images: Vec<ImageInfo>,
    pub spotify_url: String,
    pub tracks: Vec<TrackMetadata>,
}

/// Playlist metadata from Spotify
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistMetadata {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub total_tracks: u32,
    pub images: Vec<ImageInfo>,
    pub spotify_url: String,
    pub tracks: Vec<TrackMetadata>,
}

/// Image information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub track_id: String,
    pub stage: DownloadStage,
    pub progress: f32, // 0.0 to 1.0
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum DownloadStage {
    Queued,
    FetchingMetadata,
    SearchingSource,
    DownloadingAudio,
    ConvertingAudio,
    DownloadingCover,
    DownloadingLyrics,
    EmbeddingMetadata,
    Completed,
    Error,
}

impl std::fmt::Display for DownloadStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadStage::Queued => write!(f, "Queued"),
            DownloadStage::FetchingMetadata => write!(f, "Fetching Metadata"),
            DownloadStage::SearchingSource => write!(f, "Searching Source"),
            DownloadStage::DownloadingAudio => write!(f, "Downloading Audio"),
            DownloadStage::ConvertingAudio => write!(f, "Converting Audio"),
            DownloadStage::DownloadingCover => write!(f, "Downloading Cover"),
            DownloadStage::DownloadingLyrics => write!(f, "Downloading Lyrics"),
            DownloadStage::EmbeddingMetadata => write!(f, "Embedding Metadata"),
            DownloadStage::Completed => write!(f, "Completed"),
            DownloadStage::Error => write!(f, "Error"),
        }
    }
}

/// Download options
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub format: AudioFormat,
    pub bitrate: Bitrate,
    pub output_dir: PathBuf,
    pub download_lyrics: bool,
    pub download_cover: bool,
    pub embed_metadata: bool,
    pub cover_width: u32,
    pub cover_height: u32,
    pub cover_format: String,
    // Individual Metadata Toggles (matching UI)
    pub embed_title: bool,
    pub embed_artist: bool,
    pub embed_album: bool,
    pub embed_year: bool,
    pub embed_genre: bool,
    pub embed_track_number: bool,
    pub embed_disc_number: bool,
    pub embed_album_artist: bool,
    pub embed_composer: bool,
    pub embed_comment: bool,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            format: AudioFormat::Mp3,
            bitrate: Bitrate::Kbps320,
            output_dir: dirs::audio_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap().join("Music"))
                .join("Spotify Downloads"),
            download_lyrics: true,
            download_cover: true,
            embed_metadata: true,
            cover_width: 500,
            cover_height: 500,
            cover_format: "jpeg".to_string(),
            // Individual Metadata Toggles (matching UI defaults)
            embed_title: true,
            embed_artist: true,
            embed_album: true,
            embed_year: true,
            embed_genre: true,
            embed_track_number: true,
            embed_disc_number: true,
            embed_album_artist: true,
            embed_composer: true,
            embed_comment: true,
        }
    }
}
