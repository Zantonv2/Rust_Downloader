use crate::downloader::{TrackMetadata, AlbumMetadata, PlaylistMetadata, ImageInfo};
use crate::errors::{Result, SpotifyDownloaderError};
use std::path::PathBuf;

/// Wrapper that can use either existing downloader modules or centralized API
pub struct ApiWrapper;

impl ApiWrapper {
    /// Get Spotify track metadata - tries centralized API first, falls back to existing client
    pub async fn get_spotify_track_metadata(url: &str) -> Result<TrackMetadata> {
        // Try centralized API first
        match crate::api::integration::ApiIntegration::get_spotify_track_metadata(url).await {
            Ok(metadata) => Ok(metadata),
            Err(_) => {
                // Fall back to direct Spotify client
                let settings = crate::settings::Settings::load_from_local_json()
                    .unwrap_or_else(|_| crate::settings::Settings::default());
                let config = settings.config();
                
                let mut spotify_client = crate::downloader::spotify::SpotifyClient::new(
                    config.api_keys.spotify_client_id.clone().unwrap_or_default(),
                    config.api_keys.spotify_client_secret.clone().unwrap_or_default(),
                );
                
                spotify_client.get_track_metadata(url).await
            }
        }
    }

    /// Get Spotify album metadata - tries centralized API first, falls back to existing client
    pub async fn get_spotify_album_metadata(url: &str) -> Result<AlbumMetadata> {
        // Try centralized API first
        match crate::api::integration::ApiIntegration::get_spotify_album_metadata(url).await {
            Ok(metadata) => Ok(metadata),
            Err(_) => {
                // Fall back to direct Spotify client
                let settings = crate::settings::Settings::load_from_local_json()
                    .unwrap_or_else(|_| crate::settings::Settings::default());
                let config = settings.config();
                
                let mut spotify_client = crate::downloader::spotify::SpotifyClient::new(
                    config.api_keys.spotify_client_id.clone().unwrap_or_default(),
                    config.api_keys.spotify_client_secret.clone().unwrap_or_default(),
                );
                
                spotify_client.get_album_metadata(url).await
            }
        }
    }

    /// Get Spotify playlist metadata - tries centralized API first, falls back to existing client
    pub async fn get_spotify_playlist_metadata(url: &str) -> Result<PlaylistMetadata> {
        // Try centralized API first
        match crate::api::integration::ApiIntegration::get_spotify_playlist_metadata(url).await {
            Ok(metadata) => Ok(metadata),
            Err(_) => {
                // Fall back to direct Spotify client
                let settings = crate::settings::Settings::load_from_local_json()
                    .unwrap_or_else(|_| crate::settings::Settings::default());
                let config = settings.config();
                
                let mut spotify_client = crate::downloader::spotify::SpotifyClient::new(
                    config.api_keys.spotify_client_id.clone().unwrap_or_default(),
                    config.api_keys.spotify_client_secret.clone().unwrap_or_default(),
                );
                
                spotify_client.get_playlist_metadata(url).await
            }
        }
    }

    /// Download YouTube audio - uses existing YouTube downloader
    pub async fn download_youtube_audio(url: &str, output_path: &PathBuf) -> Result<()> {
        // Use existing YouTube downloader directly
        let youtube_downloader = crate::downloader::youtube::YoutubeDownloader::new();
        let config = crate::config::Config::default();
        youtube_downloader.download_audio(url, output_path, crate::config::AudioFormat::Mp3, crate::config::Bitrate::Kbps320, None, &config).await
    }

    /// Download SoundCloud audio - uses existing SoundCloud downloader
    pub async fn download_soundcloud_audio(url: &str, output_path: &PathBuf) -> Result<()> {
        // Use existing SoundCloud downloader directly
        let soundcloud_downloader = crate::downloader::soundcloud::SoundcloudDownloader::new();
        soundcloud_downloader.download_audio(url, output_path).await
    }

    /// Search for cover art - tries multiple sources in order of preference
    pub async fn search_cover_art(artist: &str, album: &str) -> Result<Option<ImageInfo>> {
        // Try iTunes first (usually fastest and most reliable)
        if let Ok(Some(cover)) = crate::api::integration::ApiIntegration::search_itunes_cover_art(artist, album).await {
            return Ok(Some(cover));
        }

        // Last.fm and MusicBrainz not implemented in centralized API yet

        // Fall back to existing cover downloader
        let cover_downloader = crate::downloader::covers::CoverDownloader::new();
        // Create a temporary track metadata for the cover downloader
        let track = TrackMetadata {
            id: "temp".to_string(),
            title: album.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            album_artist: None,
            track_number: None,
            disc_number: None,
            release_date: None,
            duration_ms: 0,
            genres: Vec::new(),
            spotify_url: String::new(),
            preview_url: None,
            external_urls: std::collections::HashMap::new(),
            album_cover_url: None,
            composer: None,
            comment: None,
        };
        
        match cover_downloader.find_cover_art(&track).await {
            Ok(url) => {
                // Download the image and return ImageInfo
                let response = reqwest::get(&url).await?;
                if response.status().is_success() {
                    Ok(Some(ImageInfo {
                        url,
                        width: 500, // Default size
                        height: 500,
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None)
        }
    }

    /// Check if centralized API is available
    pub fn is_centralized_api_available() -> bool {
        crate::api::get_api_manager().is_ok()
    }

    /// Get Last.fm track info - uses Last.fm API with client_id and client_secret
    pub async fn get_lastfm_track_info(artist: &str, track: &str) -> Result<LastfmTrackInfo> {
        let client_id = std::env::var("LASTFM_CLIENT_ID")
            .map_err(|_| SpotifyDownloaderError::Lastfm("LASTFM_CLIENT_ID not set".to_string()))?;
        let _client_secret = std::env::var("LASTFM_CLIENT_SECRET")
            .map_err(|_| SpotifyDownloaderError::Lastfm("LASTFM_CLIENT_SECRET not set".to_string()))?;

        let client = reqwest::Client::new();
        let url = format!(
            "https://ws.audioscrobbler.com/2.0/?method=track.getinfo&api_key={}&artist={}&track={}&format=json",
            client_id,
            urlencoding::encode(artist),
            urlencoding::encode(track)
        );

        let response = client.get(&url).send().await?;
        let json: serde_json::Value = response.json().await?;

        if let Some(track_data) = json.get("track") {
            let name = track_data.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(track)
                .to_string();
            
            let artist_name = track_data.get("artist")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or(artist)
                .to_string();

            let album = track_data.get("album")
                .and_then(|v| v.get("title"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let duration = track_data.get("duration")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u32>().ok())
                .map(|ms| ms / 1000); // Convert to seconds

            let playcount = track_data.get("playcount")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok());

            let listeners = track_data.get("listeners")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok());

            let tags = track_data.get("toptags")
                .and_then(|v| v.get("tag"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|tag| tag.get("name").and_then(|v| v.as_str()))
                        .take(5) // Limit to top 5 tags
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            Ok(LastfmTrackInfo {
                name,
                artist: artist_name,
                album,
                duration,
                playcount,
                listeners,
                tags,
            })
        } else {
            Err(SpotifyDownloaderError::Lastfm("Track not found on Last.fm".to_string()))
        }
    }

    /// Get Last.fm artist info
    pub async fn get_lastfm_artist_info(artist: &str) -> Result<LastfmArtistInfo> {
        let client_id = std::env::var("LASTFM_CLIENT_ID")
            .map_err(|_| SpotifyDownloaderError::Lastfm("LASTFM_CLIENT_ID not set".to_string()))?;

        let client = reqwest::Client::new();
        let url = format!(
            "https://ws.audioscrobbler.com/2.0/?method=artist.getinfo&api_key={}&artist={}&format=json",
            client_id,
            urlencoding::encode(artist)
        );

        let response = client.get(&url).send().await?;
        let json: serde_json::Value = response.json().await?;

        if let Some(artist_data) = json.get("artist") {
            let name = artist_data.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(artist)
                .to_string();

            let playcount = artist_data.get("stats")
                .and_then(|v| v.get("playcount"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok());

            let listeners = artist_data.get("stats")
                .and_then(|v| v.get("listeners"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok());

            let bio = artist_data.get("bio")
                .and_then(|v| v.get("summary"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let tags = artist_data.get("tags")
                .and_then(|v| v.get("tag"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|tag| tag.get("name").and_then(|v| v.as_str()))
                        .take(5)
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            Ok(LastfmArtistInfo {
                name,
                playcount,
                listeners,
                bio,
                tags,
            })
        } else {
            Err(SpotifyDownloaderError::Lastfm("Artist not found on Last.fm".to_string()))
        }
    }

    /// Get YouTube video metadata using YouTube Data API
    pub async fn get_youtube_metadata(video_id: &str) -> Result<YoutubeVideoInfo> {
        let api_key = std::env::var("YOUTUBE_API_KEY")
            .map_err(|_| SpotifyDownloaderError::Youtube("YOUTUBE_API_KEY not set".to_string()))?;

        let client = reqwest::Client::new();
        let url = format!(
            "https://www.googleapis.com/youtube/v3/videos?part=snippet,contentDetails,statistics&id={}&key={}",
            video_id, api_key
        );

        let response = client.get(&url).send().await?;
        let json: serde_json::Value = response.json().await?;

        if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
            if let Some(video) = items.first() {
                let snippet = video.get("snippet").ok_or_else(|| 
                    SpotifyDownloaderError::Youtube("Missing snippet data".to_string()))?;
                
                let title = snippet.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Title")
                    .to_string();
                
                let description = snippet.get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let channel_title = snippet.get("channelTitle")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Channel")
                    .to_string();
                
                let published_at = snippet.get("publishedAt")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let tags = snippet.get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|tag| tag.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                
                let duration = video.get("contentDetails")
                    .and_then(|v| v.get("duration"))
                    .and_then(|v| v.as_str())
                    .map(|s| parse_youtube_duration(s))
                    .flatten();
                
                let view_count = video.get("statistics")
                    .and_then(|v| v.get("viewCount"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok());
                
                let like_count = video.get("statistics")
                    .and_then(|v| v.get("likeCount"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok());
                
                Ok(YoutubeVideoInfo {
                    video_id: video_id.to_string(),
                    title,
                    description,
                    channel_title,
                    published_at,
                    duration,
                    view_count,
                    like_count,
                    tags,
                })
            } else {
                Err(SpotifyDownloaderError::Youtube("Video not found".to_string()))
            }
        } else {
            Err(SpotifyDownloaderError::Youtube("No video data found".to_string()))
        }
    }

    /// Get SoundCloud track info with client ID for enhanced features
    pub async fn get_soundcloud_track_info(track_id: &str) -> Result<SoundcloudTrackInfo> {
        let client_id = std::env::var("SOUNDCLOUD_CLIENT_ID")
            .map_err(|_| SpotifyDownloaderError::Soundcloud("SOUNDCLOUD_CLIENT_ID not set".to_string()))?;

        let client = reqwest::Client::new();
        let url = format!(
            "https://api-v2.soundcloud.com/tracks/{}?client_id={}",
            track_id, client_id
        );

        let response = client.get(&url).send().await?;
        let json: serde_json::Value = response.json().await?;

        let title = json.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Title")
            .to_string();
        
        let user = json.get("user")
            .and_then(|v| v.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Artist")
            .to_string();
        
        let description = json.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let duration = json.get("duration")
            .and_then(|v| v.as_u64())
            .map(|ms| (ms / 1000) as u32); // Convert to seconds
        
        let play_count = json.get("playback_count")
            .and_then(|v| v.as_u64());
        
        let like_count = json.get("likes_count")
            .and_then(|v| v.as_u64());
        
        let genre = json.get("genre")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let tags = json.get("tag_list")
            .and_then(|v| v.as_str())
            .map(|s| s.split(' ').map(|t| t.to_string()).collect())
            .unwrap_or_default();
        
        let artwork_url = json.get("artwork_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        let stream_url = json.get("stream_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(SoundcloudTrackInfo {
            track_id: track_id.to_string(),
            title,
            artist: user,
            description,
            duration,
            play_count,
            like_count,
            genre,
            tags,
            artwork_url,
            stream_url,
        })
    }

    /// Get API status information
    pub async fn get_api_status() -> Result<ApiStatus> {
        let api_manager = crate::api::get_api_manager()?;
        
        let lastfm_configured = std::env::var("LASTFM_CLIENT_ID").is_ok() && 
                               std::env::var("LASTFM_CLIENT_SECRET").is_ok();
        
        let youtube_configured = std::env::var("YOUTUBE_API_KEY").is_ok();
        let soundcloud_configured = std::env::var("SOUNDCLOUD_CLIENT_ID").is_ok();
        
        Ok(ApiStatus {
            centralized_api_available: true,
            spotify_configured: api_manager.spotify().await.read().await.is_configured(),
            lastfm_configured,
            youtube_configured,
            soundcloud_configured,
        })
    }
}

/// Last.fm track information
#[derive(Debug, Clone)]
pub struct LastfmTrackInfo {
    pub name: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<u32>, // in seconds
    pub playcount: Option<u64>,
    pub listeners: Option<u64>,
    pub tags: Vec<String>,
}

/// Last.fm artist information
#[derive(Debug, Clone)]
pub struct LastfmArtistInfo {
    pub name: String,
    pub playcount: Option<u64>,
    pub listeners: Option<u64>,
    pub bio: Option<String>,
    pub tags: Vec<String>,
}

/// YouTube video information
#[derive(Debug, Clone)]
pub struct YoutubeVideoInfo {
    pub video_id: String,
    pub title: String,
    pub description: Option<String>,
    pub channel_title: String,
    pub published_at: Option<String>,
    pub duration: Option<u32>, // in seconds
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub tags: Vec<String>,
}

/// SoundCloud track information
#[derive(Debug, Clone)]
pub struct SoundcloudTrackInfo {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub description: Option<String>,
    pub duration: Option<u32>, // in seconds
    pub play_count: Option<u64>,
    pub like_count: Option<u64>,
    pub genre: Option<String>,
    pub tags: Vec<String>,
    pub artwork_url: Option<String>,
    pub stream_url: Option<String>,
}

/// API status information
#[derive(Debug, Clone)]
pub struct ApiStatus {
    pub centralized_api_available: bool,
    pub spotify_configured: bool,
    pub lastfm_configured: bool,
    pub youtube_configured: bool,
    pub soundcloud_configured: bool,
}

/// Parse YouTube duration string (e.g., "PT4M13S" -> 253 seconds)
fn parse_youtube_duration(duration: &str) -> Option<u32> {
    // Remove "PT" prefix
    let duration = duration.strip_prefix("PT")?;
    
    let mut total_seconds = 0u32;
    let mut current_num = String::new();
    
    for ch in duration.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else {
            if let Ok(num) = current_num.parse::<u32>() {
                match ch {
                    'H' => total_seconds += num * 3600,
                    'M' => total_seconds += num * 60,
                    'S' => total_seconds += num,
                    _ => {}
                }
            }
            current_num.clear();
        }
    }
    
    Some(total_seconds)
}
