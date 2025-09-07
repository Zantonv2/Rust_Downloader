use crate::downloader::ImageInfo;
use crate::errors::{Result, SpotifyDownloaderError};
use reqwest::Client;
use serde::Deserialize;

/// iTunes API client for fetching cover art and metadata
pub struct ItunesClient {
    client: Client,
}

#[derive(Debug, Deserialize)]
struct ItunesSearchResponse {
    results: Vec<ItunesResult>,
}

#[derive(Debug, Deserialize)]
struct ItunesResult {
    #[serde(rename = "artworkUrl100")]
    artwork_url_100: Option<String>,
    #[serde(rename = "artworkUrl512")]
    artwork_url_512: Option<String>,
    #[serde(rename = "artworkUrl60")]
    artwork_url_60: Option<String>,
    #[serde(rename = "artworkUrl30")]
    artwork_url_30: Option<String>,
}

impl ItunesClient {
    /// Create a new iTunes client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Create a new iTunes client with a custom HTTP client (for proxy support)
    pub fn new_with_client(client: Client) -> Self {
        Self { client }
    }

    /// Search for cover art by artist and track name
    pub async fn search_cover_art(&self, artist: &str, track: &str) -> Result<Option<ImageInfo>> {
        let query = format!("{} {}", artist, track);
        let encoded_query = urlencoding::encode(&query);
        
        let url = format!(
            "https://itunes.apple.com/search?term={}&media=music&limit=1",
            encoded_query
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(SpotifyDownloaderError::Itunes(
                format!("iTunes API request failed: {}", response.status())
            ));
        }

        let search_response: ItunesSearchResponse = response.json().await?;

        if let Some(result) = search_response.results.first() {
            // Try to get the highest quality artwork available
            let artwork_url = result.artwork_url_512
                .as_ref()
                .or(result.artwork_url_100.as_ref())
                .or(result.artwork_url_60.as_ref())
                .or(result.artwork_url_30.as_ref());

            if let Some(url) = artwork_url {
                return Ok(Some(ImageInfo {
                    url: url.clone(),
                    width: 512, // iTunes doesn't provide exact dimensions, use reasonable defaults
                    height: 512,
                }));
            }
        }

        Ok(None)
    }

    /// Search for cover art by album name and artist
    pub async fn search_album_cover(&self, artist: &str, album: &str) -> Result<Option<ImageInfo>> {
        let query = format!("{} {}", artist, album);
        let encoded_query = urlencoding::encode(&query);
        
        let url = format!(
            "https://itunes.apple.com/search?term={}&media=music&entity=album&limit=1",
            encoded_query
        );

        println!("🌐 iTunes album search URL: {}", url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            println!("❌ iTunes album API request failed: {}", response.status());
            return Err(SpotifyDownloaderError::Itunes(
                format!("iTunes API request failed: {}", response.status())
            ));
        }

        let search_response: ItunesSearchResponse = response.json().await?;
        println!("📊 iTunes album search returned {} results", search_response.results.len());

        if let Some(result) = search_response.results.first() {
            let artwork_url = result.artwork_url_512
                .as_ref()
                .or(result.artwork_url_100.as_ref())
                .or(result.artwork_url_60.as_ref())
                .or(result.artwork_url_30.as_ref());

            if let Some(url) = artwork_url {
                return Ok(Some(ImageInfo {
                    url: url.clone(),
                    width: 512,
                    height: 512,
                }));
            }
        }

        Ok(None)
    }
}
