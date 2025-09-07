use clap::{Parser, Subcommand};
use crate::config::{AudioFormat, Bitrate};
use crate::errors::Result;
use std::path::PathBuf;

/// Spotify Downloader - Download music from Spotify using YouTube/SoundCloud as source
#[derive(Parser)]
#[command(name = "spotify-downloader")]
#[command(about = "A Spotify music downloader with CLI and GUI support")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download a track from Spotify
    Download {
        /// Spotify URL (track, album, or playlist)
        url: String,
        
        /// Output format
        #[arg(short, long, value_enum, default_value = "mp3")]
        format: AudioFormat,
        
        /// Audio bitrate
        #[arg(short, long, value_enum, default_value = "320")]
        bitrate: Bitrate,
        
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Download lyrics
        #[arg(long, default_value = "true")]
        lyrics: bool,
        
        /// Download cover art
        #[arg(long, default_value = "true")]
        cover: bool,
        
        /// Embed metadata
        #[arg(long, default_value = "true")]
        metadata: bool,
    },
    
    /// Download only lyrics for a track
    Lyrics {
        /// Spotify URL
        url: String,
        
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Download synced lyrics (.lrc)
        #[arg(long, default_value = "true")]
        synced: bool,
        
        /// Download unsynced lyrics (.txt)
        #[arg(long, default_value = "true")]
        unsynced: bool,
    },
    
    /// Download only cover art for a track
    Cover {
        /// Spotify URL
        url: String,
        
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Cover art width
        #[arg(long, default_value = "500")]
        width: u32,
        
        /// Cover art height
        #[arg(long, default_value = "500")]
        height: u32,
        
        /// Cover art format
        #[arg(long, default_value = "jpeg")]
        format: String,
    },
    
    /// Configure application settings
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    
    /// Launch GUI mode
    Gui,
    

    
    /// Import and download tracks from CSV file (Exportify export)
    ImportCsv {
        /// Path to CSV file
        csv_path: PathBuf,
        
        /// Output format
        #[arg(short, long, value_enum, default_value = "mp3")]
        format: AudioFormat,
        
        /// Audio bitrate
        #[arg(short, long, value_enum, default_value = "320")]
        bitrate: Bitrate,
        
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Download lyrics
        #[arg(long, default_value = "true")]
        lyrics: bool,
        
        /// Download cover art
        #[arg(long, default_value = "true")]
        cover: bool,
        
        /// Embed metadata
        #[arg(long, default_value = "true")]
        metadata: bool,
    },
}

#[derive(Subcommand, Clone)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    
    /// Set download directory
    SetDir {
        /// Directory path
        path: PathBuf,
    },
    
    /// Set default audio format
    SetFormat {
        /// Audio format
        format: AudioFormat,
    },
    
    /// Set default bitrate
    SetBitrate {
        /// Bitrate
        bitrate: Bitrate,
    },
    
    /// Set Spotify API credentials
    SetSpotify {
        /// Client ID
        client_id: String,
        /// Client secret
        client_secret: String,
    },
    
    /// Reset to default settings
    Reset,
}

impl Cli {
    /// Parse command line arguments
    pub fn parse() -> Self {
        let matches = <Cli as clap::Parser>::parse();
        Self { command: matches.command }
    }
    
    /// Execute the CLI command
    pub async fn execute(self) -> Result<()> {
        match self.command {
            Commands::Download { 
                ref url, 
                format, 
                bitrate, 
                ref output, 
                lyrics, 
                cover, 
                metadata 
            } => {
                self.handle_download(url.clone(), format, bitrate, output.clone(), lyrics, cover, metadata).await
            }
            Commands::Lyrics { ref url, ref output, synced, unsynced } => {
                self.handle_lyrics(url.clone(), output.clone(), synced, unsynced).await
            }
            Commands::Cover { ref url, ref output, width, height, ref format } => {
                self.handle_cover(url.clone(), output.clone(), width, height, format.clone()).await
            }
            Commands::Config { ref command } => {
                self.handle_config(command.clone()).await
            }
            Commands::Gui => {
                self.handle_gui().await
            }
            Commands::ImportCsv { 
                ref csv_path, 
                format, 
                bitrate, 
                ref output, 
                lyrics, 
                cover, 
                metadata 
            } => {
                self.handle_csv_import(csv_path.clone(), format, bitrate, output.clone(), lyrics, cover, metadata).await
            }
        }
    }
    
    async fn handle_download(
        &self,
        url: String,
        format: AudioFormat,
        bitrate: Bitrate,
        output: Option<PathBuf>,
        lyrics: bool,
        cover: bool,
        metadata: bool,
    ) -> Result<()> {
        println!("Downloading from: {}", url);
        println!("Format: {}", format);
        println!("Bitrate: {} kbps", bitrate.as_u32());
        
        // Get output directory
        let output_dir = if let Some(output_dir) = output {
            output_dir
        } else {
            use crate::settings::Settings;
            let settings = Settings::load()?;
            settings.config().download_directory.clone()
        };
        
        println!("Output directory: {}", output_dir.display());
        println!("Download lyrics: {}", lyrics);
        println!("Download cover: {}", cover);
        println!("Embed metadata: {}", metadata);
        
        // Create download options
        let download_options = crate::downloader::DownloadOptions {
            format,
            bitrate,
            output_dir: output_dir.clone(),
            download_lyrics: lyrics,
            download_cover: cover,
            embed_metadata: metadata,
            cover_width: 500,
            cover_height: 500,
            cover_format: "jpeg".to_string(),
            // Individual Metadata Toggles (CLI defaults to all enabled)
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
        
        // Get track metadata based on URL type
        let track = if url.contains("spotify.com") {
            println!("Fetching track metadata from Spotify...");
            crate::downloader::api_wrapper::ApiWrapper::get_spotify_track_metadata(&url).await?
        } else if url.contains("youtube.com") || url.contains("youtu.be") {
            println!("Fetching track metadata from YouTube...");
            // For YouTube URLs, we'll create a basic track metadata and let the downloader handle the rest
            crate::downloader::TrackMetadata {
                id: uuid::Uuid::new_v4().to_string(),
                title: "YouTube Video".to_string(),
                artist: "Unknown Artist".to_string(),
                album: "Unknown Album".to_string(),
                album_artist: None,
                track_number: None,
                disc_number: None,
                release_date: None,
                duration_ms: 0,
                genres: Vec::new(),
                spotify_url: url.clone(),
                preview_url: None,
                external_urls: std::collections::HashMap::new(),
                album_cover_url: None,
                composer: None,
                comment: None,
            }
        } else if url.contains("soundcloud.com") {
            println!("Fetching track metadata from SoundCloud...");
            // For SoundCloud URLs, we'll create a basic track metadata and let the downloader handle the rest
            crate::downloader::TrackMetadata {
                id: uuid::Uuid::new_v4().to_string(),
                title: "SoundCloud Track".to_string(),
                artist: "Unknown Artist".to_string(),
                album: "Unknown Album".to_string(),
                album_artist: None,
                track_number: None,
                disc_number: None,
                release_date: None,
                duration_ms: 0,
                genres: Vec::new(),
                spotify_url: url.clone(),
                preview_url: None,
                external_urls: std::collections::HashMap::new(),
                album_cover_url: None,
                composer: None,
                comment: None,
            }
        } else {
            return Err(crate::errors::SpotifyDownloaderError::InvalidUrl(format!("Unsupported URL type: {}", url)));
        };
        
        println!("Found track: {} - {}", track.artist, track.title);
        
        // Create audio downloader with proxy-configured client
        let api_manager = crate::api::get_api_manager()?;
        let client = api_manager.client().clone();
        let audio_downloader = crate::downloader::audio::AudioDownloader::new_with_client(client);
        
        // Create progress channel
        let (progress_sender, mut progress_receiver) = tokio::sync::mpsc::unbounded_channel();
        
        // Start download in background
        let download_handle = {
            let track = track.clone();
            let download_options = download_options.clone();
            let mut audio_downloader = audio_downloader;
            tokio::spawn(async move {
                let config = crate::config::Config::default(); // Create config inside spawn
                audio_downloader.download_track(&track, &download_options, Some(progress_sender), &config).await
            })
        };
        
        // Monitor progress
        while let Some(progress) = progress_receiver.recv().await {
            println!("[{}] {} - {:.1}%", 
                progress.stage, 
                progress.message, 
                progress.progress * 100.0
            );
            
            if matches!(progress.stage, crate::downloader::DownloadStage::Completed | crate::downloader::DownloadStage::Error) {
                break;
            }
        }
        
        // Wait for download to complete
        match download_handle.await {
            Ok(Ok(output_path)) => {
                println!("Download completed successfully!");
                println!("File saved to: {}", output_path.display());
                
                // Download additional content if requested
                if lyrics {
                    self.download_lyrics_for_track(&track, &output_dir).await?;
                }
                
                if cover {
                    self.download_cover_for_track(&track, &output_dir).await?;
                }
            }
            Ok(Err(e)) => {
                eprintln!("Download failed: {}", e);
                return Err(e);
            }
            Err(e) => {
                eprintln!("Download task failed: {}", e);
                return Err(crate::errors::SpotifyDownloaderError::Unknown(format!("Download task failed: {}", e)));
            }
        }
        
        Ok(())
    }
    
    async fn download_lyrics_for_track(&self, track: &crate::downloader::TrackMetadata, output_dir: &PathBuf) -> Result<()> {
        println!("Downloading lyrics for: {} - {}", track.artist, track.title);
        
        // Get proxy-configured client for lyrics downloader
        let client = crate::api::get_api_manager()
            .map(|api_manager| api_manager.client().clone())
            .unwrap_or_else(|_| reqwest::Client::new());
        let lyrics_downloader = crate::lyrics::LyricsDownloader::new_with_client(client);
        let result = lyrics_downloader.download_lyrics(track, output_dir).await?;
        
        if let Some(synced_path) = result.synced_path {
            println!("Synced lyrics saved to: {}", synced_path.display());
        }
        
        if let Some(unsynced_path) = result.unsynced_path {
            println!("Unsynced lyrics saved to: {}", unsynced_path.display());
        }
        
        Ok(())
    }
    
    async fn download_cover_for_track(&self, track: &crate::downloader::TrackMetadata, output_dir: &PathBuf) -> Result<()> {
        println!("Downloading cover art for: {} - {}", track.artist, track.title);
        
        let cover_downloader = crate::downloader::covers::CoverDownloader::new();
        let cover_path = cover_downloader.get_cover_path(track, output_dir, "jpg");
        
        if !cover_downloader.cover_exists(&cover_path) {
            cover_downloader.download_cover_art(track, &cover_path, 500, 500, "jpg").await?;
            println!("Cover art saved to: {}", cover_path.display());
        } else {
            println!("Cover art already exists: {}", cover_path.display());
        }
        
        Ok(())
    }
    
    async fn handle_lyrics(
        &self,
        url: String,
        output: Option<PathBuf>,
        synced: bool,
        unsynced: bool,
    ) -> Result<()> {
        println!("Downloading lyrics from: {}", url);
        println!("Synced lyrics: {}", synced);
        println!("Unsynced lyrics: {}", unsynced);
        
        if let Some(output_dir) = output {
            println!("Output directory: {}", output_dir.display());
        }
        
        // TODO: Implement lyrics download logic
        println!("Lyrics download functionality will be implemented in the lyrics module");
        
        Ok(())
    }
    
    async fn handle_cover(
        &self,
        url: String,
        output: Option<PathBuf>,
        width: u32,
        height: u32,
        format: String,
    ) -> Result<()> {
        println!("Downloading cover art from: {}", url);
        println!("Size: {}x{}", width, height);
        println!("Format: {}", format);
        
        if let Some(output_dir) = output {
            println!("Output directory: {}", output_dir.display());
        }
        
        // TODO: Implement cover art download logic
        println!("Cover art download functionality will be implemented in the downloader module");
        
        Ok(())
    }
    
    async fn handle_config(&self, command: ConfigCommands) -> Result<()> {
        use crate::settings::Settings;
        
        match command {
            ConfigCommands::Show => {
                let settings = Settings::load()?;
                let config = settings.config();
                
                println!("Current configuration:");
                println!("  Download directory: {}", config.download_directory.display());
                println!("  Default format: {}", config.default_format);
                println!("  Default bitrate: {} kbps", config.default_bitrate.as_u32());
                println!("  Cover size: {}x{}", config.cover_config.width, config.cover_config.height);
                println!("  Cover format: {}", config.cover_config.format);
                println!("  Download lyrics: {}", config.metadata_config.embed_lyrics);
                println!("  Download cover: {}", config.metadata_config.embed_cover);
                println!("  Embed metadata: {}", config.metadata_config.embed_metadata);
                
                if config.api_keys.spotify_client_id.is_some() {
                    println!("  Spotify credentials: Set");
                } else {
                    println!("  Spotify credentials: Not set");
                }
            }
            ConfigCommands::SetDir { path } => {
                let mut settings = Settings::load()?;
                settings.set_download_directory(path)?;
                println!("Download directory updated");
            }
            ConfigCommands::SetFormat { format } => {
                let mut settings = Settings::load()?;
                settings.set_default_format(format)?;
                println!("Default format updated to: {}", format);
            }
            ConfigCommands::SetBitrate { bitrate } => {
                let mut settings = Settings::load()?;
                settings.set_default_bitrate(bitrate)?;
                println!("Default bitrate updated to: {} kbps", bitrate.as_u32());
            }
            ConfigCommands::SetSpotify { client_id, client_secret } => {
                let mut settings = Settings::load()?;
                settings.set_spotify_credentials(client_id, client_secret)?;
                println!("Spotify credentials updated");
            }
            ConfigCommands::Reset => {
                let config = crate::config::Config::default();
                config.save()?;
                println!("Configuration reset to defaults");
            }
        }
        
        Ok(())
    }
    
    async fn handle_gui(&self) -> Result<()> {
        println!("Launching GUI mode...");
        
        // Launch the iced GUI
        use crate::ui::run_iced;
        run_iced()?;
        
        Ok(())
    }
    

    
    async fn handle_csv_import(
        &self,
        csv_path: PathBuf,
        format: AudioFormat,
        bitrate: Bitrate,
        output: Option<PathBuf>,
        lyrics: bool,
        cover: bool,
        metadata: bool,
    ) -> Result<()> {
        println!("Importing tracks from CSV: {}", csv_path.display());
        println!("Format: {}", format);
        println!("Bitrate: {} kbps", bitrate.as_u32());
        
        // Get output directory
        let output_dir = if let Some(output_dir) = output {
            output_dir
        } else {
            use crate::settings::Settings;
            let settings = Settings::load()?;
            settings.config().download_directory.clone()
        };
        
        println!("Output directory: {}", output_dir.display());
        println!("Download lyrics: {}", lyrics);
        println!("Download cover: {}", cover);
        println!("Embed metadata: {}", metadata);
        
        // Validate CSV format first
        let csv_importer = crate::csv_import::CsvImporter::new();
        csv_importer.validate_csv_format(&csv_path)?;
        
        // Get CSV info
        let csv_info = csv_importer.get_csv_info(&csv_path)?;
        println!("CSV Info:");
        println!("  File: {}", csv_info.file_path.display());
        println!("  Columns: {}", csv_info.column_count);
        println!("  Records: {}", csv_info.record_count);
        
        // Create batch downloader
        let mut batch_downloader = crate::csv_import::CsvBatchDownloader::new();
        
        // Progress callback
        let progress_callback = Box::new(|current: usize, total: usize, message: String| {
            let percentage = (current as f32 / total as f32) * 100.0;
            println!("[{}/{}] ({:.1}%) {}", current, total, percentage, message);
        });
        
        // Start batch download
        let config = crate::config::Config::default();
        let result = batch_downloader.download_from_csv(
            &csv_path,
            &output_dir,
            format,
            bitrate,
            Some(progress_callback),
            &config,
        ).await?;
        
        // Print results
        println!("\n=== CSV Import Results ===");
        println!("Total tracks: {}", result.total_tracks);
        println!("Successful downloads: {}", result.successful_downloads);
        println!("Failed downloads: {}", result.failed_downloads);
        
        if !result.failed_tracks.is_empty() {
            println!("\nFailed tracks:");
            for (track, error) in &result.failed_tracks {
                println!("  ✗ {} - {}: {}", track.artist, track.title, error);
            }
        }
        
        if result.successful_downloads > 0 {
            println!("\n✓ Successfully downloaded {} tracks to: {}", 
                     result.successful_downloads, output_dir.display());
        }
        
        Ok(())
    }
}
