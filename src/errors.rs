use thiserror::Error;

/// Main error type for the Spotify downloader application
#[derive(Error, Debug)]
pub enum SpotifyDownloaderError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),

    #[error("ID3 tagging error: {0}")]
    Id3(#[from] id3::Error),

    #[error("Lofty tagging error: {0}")]
    Lofty(String),

    #[error("Image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("FFmpeg error: {0}")]
    Ffmpeg(String),

    #[error("YouTube download error: {0}")]
    Youtube(String),

    #[error("SoundCloud download error: {0}")]
    Soundcloud(String),

    #[error("Spotify API error: {0}")]
    Spotify(String),

    #[error("iTunes API error: {0}")]
    Itunes(String),

    #[error("Last.fm API error: {0}")]
    Lastfm(String),

    #[error("MusicBrainz API error: {0}")]
    Musicbrainz(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Settings error: {0}")]
    Settings(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Metadata error: {0}")]
    Metadata(String),

    #[error("Lyrics error: {0}")]
    Lyrics(String),

    #[error("Cover art error: {0}")]
    CoverArt(String),

    #[error("Conversion error: {0}")]
    Conversion(String),

    #[error("CSV import error: {0}")]
    CsvImport(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Invalid bitrate: {0}")]
    InvalidBitrate(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type alias for the application
pub type Result<T> = std::result::Result<T, SpotifyDownloaderError>;

/// Helper trait for converting various error types to our custom error
pub trait IntoSpotifyDownloaderError<T> {
    fn into_spotify_error(self) -> Result<T>;
}

impl<T, E> IntoSpotifyDownloaderError<T> for std::result::Result<T, E>
where
    E: std::fmt::Display,
{
    fn into_spotify_error(self) -> Result<T> {
        self.map_err(|e| SpotifyDownloaderError::Unknown(e.to_string()))
    }
}
