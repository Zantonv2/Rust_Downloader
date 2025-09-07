use crate::downloader::{TrackMetadata, AlbumMetadata, PlaylistMetadata, ImageInfo};
use crate::errors::Result;
use super::get_api_manager;
use std::path::PathBuf;

/// Integration layer between centralized API manager and existing downloader modules
pub struct ApiIntegration;

impl ApiIntegration {
    /// Get Spotify track metadata using centralized API
    pub async fn get_spotify_track_metadata(url: &str) -> Result<TrackMetadata> {
        let api_manager = get_api_manager()?;
        let spotify_api = api_manager.spotify().await;
        let mut spotify = spotify_api.write().await;
        
        // Direct call instead of using make_request to avoid lifetime issues
        spotify.get_track_metadata(url).await
    }

    /// Get Spotify album metadata using centralized API
    pub async fn get_spotify_album_metadata(url: &str) -> Result<AlbumMetadata> {
        let api_manager = get_api_manager()?;
        let spotify_api = api_manager.spotify().await;
        let mut spotify = spotify_api.write().await;
        
        // Direct call instead of using make_request to avoid lifetime issues
        spotify.get_album_metadata(url).await
    }

    /// Get Spotify playlist metadata using centralized API
    pub async fn get_spotify_playlist_metadata(url: &str) -> Result<PlaylistMetadata> {
        let api_manager = get_api_manager()?;
        let spotify_api = api_manager.spotify().await;
        let mut spotify = spotify_api.write().await;
        
        // Direct call instead of using make_request to avoid lifetime issues
        spotify.get_playlist_metadata(url).await
    }

    /// Download YouTube audio using existing YouTube downloader
    pub async fn download_youtube_audio(url: &str, output_path: &PathBuf) -> Result<()> {
        // Use existing YouTube downloader directly
        let youtube_downloader = crate::downloader::youtube::YoutubeDownloader::new();
        let config = crate::config::Config::default();
        youtube_downloader.download_audio(url, output_path, crate::config::AudioFormat::Mp3, crate::config::Bitrate::Kbps320, None, &config).await
    }

    /// Download SoundCloud audio using existing SoundCloud downloader
    pub async fn download_soundcloud_audio(url: &str, output_path: &PathBuf) -> Result<()> {
        // Use existing SoundCloud downloader directly
        let soundcloud_downloader = crate::downloader::soundcloud::SoundcloudDownloader::new();
        soundcloud_downloader.download_audio(url, output_path).await
    }

    /// Search iTunes cover art using centralized API
    pub async fn search_itunes_cover_art(artist: &str, album: &str) -> Result<Option<ImageInfo>> {
        let api_manager = get_api_manager()?;
        let itunes_api = api_manager.itunes().await;
        let itunes = itunes_api.read().await;
        
        // Direct call instead of using make_request to avoid lifetime issues
        itunes.search_cover_art(artist, album).await
    }

    /// Get HTTP client for direct use
    pub fn get_http_client() -> Result<&'static reqwest::Client> {
        let api_manager = get_api_manager()?;
        Ok(api_manager.client())
    }
}
