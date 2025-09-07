use crate::downloader::TrackMetadata;
use crate::errors::{Result, SpotifyDownloaderError};
use std::path::PathBuf;
use std::collections::HashMap;

/// CSV importer for Exportify exports
pub struct CsvImporter {
    // Configuration for CSV import
}

/// Spotify CSV record structure (from your export)
#[derive(Debug, Clone)]
pub struct SpotifyRecord {
    pub track_uri: String,
    pub track_name: String,
    pub album_name: String,
    pub artist_name: String,
    pub release_date: String,
    pub duration_ms: u32,
    pub popularity: u32,
    pub explicit: bool,
    pub added_by: String,
    pub added_at: String,
    pub genres: String,
    pub record_label: String,
    pub danceability: f32,
    pub energy: f32,
    pub key: u32,
    pub loudness: f32,
    pub mode: u32,
    pub speechiness: f32,
    pub acousticness: f32,
    pub instrumentalness: f32,
    pub liveness: f32,
    pub valence: f32,
    pub tempo: f32,
    pub time_signature: u32,
}

impl CsvImporter {
    /// Create a new CSV importer
    pub fn new() -> Self {
        Self {}
    }

    /// Import tracks from Spotify CSV file
    pub async fn import_from_csv(&self, csv_path: &PathBuf) -> Result<Vec<TrackMetadata>> {
        println!("Importing tracks from CSV: {}", csv_path.display());

        if !csv_path.exists() {
            return Err(SpotifyDownloaderError::CsvImport(
                format!("CSV file not found: {}", csv_path.display())
            ));
        }

        let mut tracks = Vec::new();
        let mut reader = csv::Reader::from_path(csv_path)
            .map_err(|e| SpotifyDownloaderError::CsvImport(format!("Failed to read CSV: {}", e)))?;

        for (row_number, result) in reader.records().enumerate() {
            let record = result
                .map_err(|e| SpotifyDownloaderError::CsvImport(format!("Failed to parse CSV row {}: {}", row_number + 1, e)))?;

            if let Ok(spotify_record) = self.parse_spotify_record(&record) {
                let track_metadata = self.convert_to_track_metadata(spotify_record, row_number + 1);
                tracks.push(track_metadata);
            } else {
                println!("Warning: Failed to parse row {}: {:?}", row_number + 1, record);
            }
        }

        println!("Successfully imported {} tracks from CSV", tracks.len());
        Ok(tracks)
    }

    /// Parse a CSV record into SpotifyRecord
    fn parse_spotify_record(&self, record: &csv::StringRecord) -> Result<SpotifyRecord> {
        // Your CSV format has these columns (in order):
        // Track URI, Track Name, Album Name, Artist Name(s), Release Date, Duration (ms), 
        // Popularity, Explicit, Added By, Added At, Genres, Record Label, Danceability, 
        // Energy, Key, Loudness, Mode, Speechiness, Acousticness, Instrumentalness, 
        // Liveness, Valence, Tempo, Time Signature

        if record.len() < 24 {
            return Err(SpotifyDownloaderError::CsvImport(
                format!("Invalid CSV record: expected at least 24 columns, got {}", record.len())
            ));
        }

        let track_uri = record.get(0).unwrap_or("").to_string();
        let track_name = record.get(1).unwrap_or("").to_string();
        let album_name = record.get(2).unwrap_or("").to_string();
        let artist_name = record.get(3).unwrap_or("").to_string();
        let release_date = record.get(4).unwrap_or("").to_string();
        
        let duration_ms = record.get(5)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        
        let popularity = record.get(6)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        
        let explicit = record.get(7)
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(false);
        
        let added_by = record.get(8).unwrap_or("").to_string();
        let added_at = record.get(9).unwrap_or("").to_string();
        let genres = record.get(10).unwrap_or("").to_string();
        let record_label = record.get(11).unwrap_or("").to_string();
        
        let danceability = record.get(12)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        
        let energy = record.get(13)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        
        let key = record.get(14)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        
        let loudness = record.get(15)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        
        let mode = record.get(16)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        
        let speechiness = record.get(17)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        
        let acousticness = record.get(18)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        
        let instrumentalness = record.get(19)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        
        let liveness = record.get(20)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        
        let valence = record.get(21)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        
        let tempo = record.get(22)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.0);
        
        let time_signature = record.get(23)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        Ok(SpotifyRecord {
            track_uri,
            track_name,
            album_name,
            artist_name,
            release_date,
            duration_ms,
            popularity,
            explicit,
            added_by,
            added_at,
            genres,
            record_label,
            danceability,
            energy,
            key,
            loudness,
            mode,
            speechiness,
            acousticness,
            instrumentalness,
            liveness,
            valence,
            tempo,
            time_signature,
        })
    }

    /// Convert ExportifyRecord to TrackMetadata
    fn convert_to_track_metadata(&self, record: SpotifyRecord, track_number: usize) -> TrackMetadata {
        // Generate a unique ID for the track
        let hash = md5::compute(format!("{}{}", record.track_name, record.artist_name));
        let id = format!("csv_{}_{:x}", track_number, hash);

        // Parse release date
        let release_date = if record.release_date.is_empty() {
            None
        } else {
            Some(record.release_date)
        };

        // Create genres vector from comma-separated string
        let genres = if record.genres.is_empty() {
            Vec::new()
        } else {
            record.genres.split(',').map(|s| s.trim().to_string()).collect()
        };

        // Extract track ID from Spotify URI if available
        let spotify_track_id = if record.track_uri.starts_with("spotify:track:") {
            record.track_uri.replace("spotify:track:", "")
        } else {
            id.clone()
        };

        // Cover art will be fetched by CoverDownloader during download

        TrackMetadata {
            id: id.clone(),
            title: record.track_name,
            artist: record.artist_name.clone(),
            album: record.album_name,
            album_artist: Some(record.artist_name),
            track_number: Some(track_number as u32),
            disc_number: Some(1),
            release_date,
            duration_ms: record.duration_ms,
            genres,
            spotify_url: format!("https://open.spotify.com/track/{}", spotify_track_id),
            preview_url: None,
            external_urls: HashMap::new(),
            album_cover_url: None, // Will be fetched by CoverDownloader during download
            composer: None, // Not available in CSV
            comment: Some("Imported from Spotify CSV".to_string()),
        }
    }

    /// Validate CSV file format
    pub fn validate_csv_format(&self, csv_path: &PathBuf) -> Result<()> {
        if !csv_path.exists() {
            return Err(SpotifyDownloaderError::CsvImport(
                format!("CSV file not found: {}", csv_path.display())
            ));
        }

        let mut reader = csv::Reader::from_path(csv_path)
            .map_err(|e| SpotifyDownloaderError::CsvImport(format!("Failed to read CSV: {}", e)))?;

        // Check if we have headers
        let headers = reader.headers()
            .map_err(|e| SpotifyDownloaderError::CsvImport(format!("Failed to read CSV headers: {}", e)))?;

        println!("CSV headers found: {:?}", headers.iter().collect::<Vec<_>>());

        // Check if we have the minimum required columns
        if headers.len() < 24 {
            return Err(SpotifyDownloaderError::CsvImport(
                format!("Invalid CSV format: expected at least 24 columns, got {}", headers.len())
            ));
        }

        // Check for required column names (case-insensitive) - your CSV format
        let required_columns = [
            "track uri", "track name", "album name", "artist name(s)", "release date", 
            "duration (ms)", "popularity", "explicit", "added by", "added at", "genres", 
            "record label", "danceability", "energy", "key", "loudness", "mode", 
            "speechiness", "acousticness", "instrumentalness", "liveness", "valence", 
            "tempo", "time signature"
        ];

        let header_lower: Vec<String> = headers.iter().map(|h| h.to_lowercase()).collect();
        
        for required in &required_columns {
            if !header_lower.contains(&required.to_string()) {
                println!("Warning: Missing column '{}' in CSV", required);
            }
        }

        Ok(())
    }

    /// Get CSV file information
    pub fn get_csv_info(&self, csv_path: &PathBuf) -> Result<CsvInfo> {
        if !csv_path.exists() {
            return Err(SpotifyDownloaderError::CsvImport(
                format!("CSV file not found: {}", csv_path.display())
            ));
        }

        let mut reader = csv::Reader::from_path(csv_path)
            .map_err(|e| SpotifyDownloaderError::CsvImport(format!("Failed to read CSV: {}", e)))?;

        let headers = reader.headers()
            .map_err(|e| SpotifyDownloaderError::CsvImport(format!("Failed to read CSV headers: {}", e)))?;

        let header_count = headers.len();
        let header_names: Vec<String> = headers.iter().map(|h| h.to_string()).collect();

        let mut record_count = 0;
        for result in reader.records() {
            match result {
                Ok(_) => record_count += 1,
                Err(e) => {
                    println!("Warning: Failed to parse record: {}", e);
                }
            }
        }

        Ok(CsvInfo {
            file_path: csv_path.clone(),
            column_count: header_count,
            record_count,
            headers: header_names,
        })
    }

}

/// Information about a CSV file
#[derive(Debug, Clone)]
pub struct CsvInfo {
    pub file_path: PathBuf,
    pub column_count: usize,
    pub record_count: usize,
    pub headers: Vec<String>,
}

/// Batch download tracks from CSV
pub struct CsvBatchDownloader {
    csv_importer: CsvImporter,
    audio_downloader: crate::downloader::audio::AudioDownloader,
}

impl CsvBatchDownloader {
    /// Create a new CSV batch downloader
    pub fn new() -> Self {
        // Get proxy-configured client
        let client = crate::api::get_api_manager()
            .map(|api_manager| api_manager.client().clone())
            .unwrap_or_else(|_| reqwest::Client::new());
        
        Self {
            csv_importer: CsvImporter::new(),
            audio_downloader: crate::downloader::audio::AudioDownloader::new_with_client(client),
        }
    }

    /// Download all tracks from a CSV file
    pub async fn download_from_csv(
        &mut self,
        csv_path: &PathBuf,
        output_dir: &PathBuf,
        format: crate::config::AudioFormat,
        bitrate: crate::config::Bitrate,
        progress_callback: Option<Box<dyn Fn(usize, usize, String) + Send + Sync>>,
        config: &crate::config::Config,
    ) -> Result<CsvDownloadResult> {
        println!("Starting batch download from CSV: {}", csv_path.display());

        // Import tracks from CSV
        let tracks = self.csv_importer.import_from_csv(csv_path).await?;
        let total_tracks = tracks.len();

        if total_tracks == 0 {
            return Ok(CsvDownloadResult {
                total_tracks: 0,
                successful_downloads: 0,
                failed_downloads: 0,
                failed_tracks: Vec::new(),
            });
        }

        // Create download options
        let download_options = crate::downloader::DownloadOptions {
            format,
            bitrate,
            output_dir: output_dir.clone(),
            download_lyrics: true,
            download_cover: true,
            embed_metadata: true,
            cover_width: 500,
            cover_height: 500,
            cover_format: "jpeg".to_string(),
            // Individual Metadata Toggles (CSV import defaults to all enabled)
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
        };

        let mut successful_downloads = 0;
        let mut failed_downloads = 0;
        let mut failed_tracks = Vec::new();

        // Download each track
        for (index, track) in tracks.iter().enumerate() {
            if let Some(ref callback) = progress_callback {
                callback(index + 1, total_tracks, format!("Downloading: {} - {}", track.artist, track.title));
            }

            match self.audio_downloader.download_track(track, &download_options, None, config).await {
                Ok(_) => {
                    successful_downloads += 1;
                    println!("✓ Downloaded: {} - {}", track.artist, track.title);
                }
                Err(e) => {
                    failed_downloads += 1;
                    failed_tracks.push((track.clone(), e.to_string()));
                    println!("✗ Failed: {} - {} - Error: {}", track.artist, track.title, e);
                }
            }
        }

        let result = CsvDownloadResult {
            total_tracks,
            successful_downloads,
            failed_downloads,
            failed_tracks,
        };

        println!("Batch download completed:");
        println!("  Total tracks: {}", result.total_tracks);
        println!("  Successful: {}", result.successful_downloads);
        println!("  Failed: {}", result.failed_downloads);

        Ok(result)
    }
}

/// Result of CSV batch download
#[derive(Debug, Clone)]
pub struct CsvDownloadResult {
    pub total_tracks: usize,
    pub successful_downloads: usize,
    pub failed_downloads: usize,
    pub failed_tracks: Vec<(TrackMetadata, String)>,
}
