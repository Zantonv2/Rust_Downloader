use crate::config::AudioFormat;
use crate::downloader::{
    DownloadOptions, DownloadProgress, DownloadStage, TrackMetadata,
    youtube::YoutubeDownloader, soundcloud::SoundcloudDownloader, yt_dlp::YtDlpDownloader,
    converter::AudioConverter, covers::CoverDownloader, metadata::MetadataEmbedder,
};
use crate::errors::{Result, SpotifyDownloaderError};
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::mpsc;
use reqwest::Client;

/// Main audio downloader that orchestrates different download strategies
pub struct AudioDownloader {
    youtube_downloader: YoutubeDownloader,
    soundcloud_downloader: SoundcloudDownloader,
    ytdlp_downloader: YtDlpDownloader,
    converter: AudioConverter,
    cover_downloader: CoverDownloader,
    metadata_embedder: MetadataEmbedder,
    search_cache: HashMap<String, Vec<crate::downloader::youtube::SearchResult>>,
}

impl AudioDownloader {
    /// Create a new audio downloader
    pub fn new() -> Self {
        Self {
            youtube_downloader: YoutubeDownloader::new(),
            soundcloud_downloader: SoundcloudDownloader::new(),
            ytdlp_downloader: YtDlpDownloader::new(),
            converter: AudioConverter::new(),
            cover_downloader: CoverDownloader::new(),
            metadata_embedder: MetadataEmbedder::new(),
            search_cache: HashMap::new(),
        }
    }

    /// Create a new audio downloader with a custom HTTP client (for proxy support)
    pub fn new_with_client(client: Client) -> Self {
        Self {
            youtube_downloader: YoutubeDownloader::new(),
            soundcloud_downloader: SoundcloudDownloader::new(),
            ytdlp_downloader: YtDlpDownloader::new(),
            converter: AudioConverter::new(),
            cover_downloader: CoverDownloader::new_with_client(client),
            metadata_embedder: MetadataEmbedder::new(),
            search_cache: HashMap::new(),
        }
    }

    /// Download audio for a track with progress reporting
    pub async fn download_track(
        &mut self,
        track: &TrackMetadata,
        options: &DownloadOptions,
        progress_sender: Option<mpsc::UnboundedSender<DownloadProgress>>,
        config: &crate::config::Config,
    ) -> Result<PathBuf> {
        println!("🎵 Starting download for: {} - {}", track.artist, track.title);
        
        self.send_progress(
            &progress_sender,
            &track.id,
            DownloadStage::SearchingSource,
            0.1,
            "Searching for audio source...".to_string(),
        );

        // Search for the track on different platforms (without album name for better results)
        let search_query = format!("{} {}", track.artist, track.title);
        println!("🔍 Searching for: {}", search_query);
        
        // Check cache first
        let search_results = if let Some(cached_results) = self.search_cache.get(&search_query) {
            println!("📋 Using cached search results");
            cached_results.clone()
        } else {
            // Try optimized search strategy: ytsearch1 -> ytsearch5 -> scsearch1 -> scsearch5
            let search_results = self.youtube_downloader.search_optimized(&search_query, config).await;
            
            match search_results {
                Ok(results) if !results.is_empty() => {
                    // Cache the results for future use
                    self.search_cache.insert(search_query.clone(), results.clone());
                    results
                },
                Ok(_) => {
                    // Search succeeded but returned no results
                    println!("❌ No results found for: {} - {}", track.artist, track.title);
                    self.send_progress(
                        &progress_sender,
                        &track.id,
                        DownloadStage::Error,
                        0.0,
                        "No results found for this track".to_string(),
                    );
                    return Err(SpotifyDownloaderError::Download("No results found".to_string()));
                }
                Err(e) => {
                    // Search failed
                    println!("❌ Search failed for: {} - {}: {}", track.artist, track.title, e);
                    self.send_progress(
                        &progress_sender,
                        &track.id,
                        DownloadStage::Error,
                        0.0,
                        format!("Search failed: {}", e),
                    );
                    return Err(SpotifyDownloaderError::Download(format!("Search failed: {}", e)));
                }
            }
        };
        
        if !search_results.is_empty() {
            let best_match = &search_results[0]; // Take the first (best) result
            println!("✅ Found YouTube source: {}", best_match.platform);
            
            self.send_progress(
                &progress_sender,
                &track.id,
                DownloadStage::DownloadingAudio,
                0.3,
                format!("Found {} source, downloading...", best_match.platform),
            );

            let output_path = self.get_output_path(track, options);
            
            // Create progress callback for download
            let progress_sender_clone = progress_sender.clone();
            let track_id_clone = track.id.clone();
            let progress_callback = Box::new(move |progress: f32| {
                // Send progress updates during download
                if let Some(sender) = &progress_sender_clone {
                    let _ = sender.send(DownloadProgress {
                        track_id: track_id_clone.clone(),
                        stage: DownloadStage::DownloadingAudio,
                        progress: 0.3 + (progress * 0.3), // 30% to 60%
                        message: format!("Downloading... {:.1}%", progress * 100.0),
                    });
                }
            });

            self.youtube_downloader.download_audio(
                &best_match.url,
                &output_path,
                options.format,
                options.bitrate,
                Some(progress_callback),
                config,
            ).await?;

            self.send_progress(
                &progress_sender,
                &track.id,
                DownloadStage::ConvertingAudio,
                0.6,
                "Converting audio format...".to_string(),
            );

            // Convert to desired format and bitrate
            let converted_path = self.convert_audio(&output_path, options).await?;

            // Download cover art and lyrics in parallel if requested (for embedding only)
            let (cover_art_data, lyrics_data) = if options.download_cover || options.download_lyrics {
                self.send_progress(
                    &progress_sender,
                    &track.id,
                    DownloadStage::DownloadingCover,
                    0.8,
                    "Downloading cover art and lyrics for embedding...".to_string(),
                );
                
                // Create futures for parallel execution
                let cover_future = if options.download_cover {
                    println!("🖼️ Downloading cover art for: {} - {}", track.artist, track.title);
                    Some(self.cover_downloader.download_cover_art_data(
                        track,
                        options.cover_width,
                        options.cover_height,
                        &options.cover_format,
                    ))
                } else {
                    println!("⏭️ Cover art download disabled in settings");
                    None
                };
                
                let lyrics_future = if options.download_lyrics {
                    // Get proxy-configured client for lyrics downloader
                    let client = crate::api::get_api_manager()
                        .map(|api_manager| api_manager.client().clone())
                        .unwrap_or_else(|_| reqwest::Client::new());
                    let track_clone = track.clone();
                    Some(async move {
                        let lyrics_downloader = crate::lyrics::LyricsDownloader::new_with_client(client);
                        lyrics_downloader.download_lyrics_for_embedding(&track_clone).await
                    })
                } else {
                    None
                };
                
                // Execute both futures in parallel
                let (cover_result, lyrics_result) = tokio::join!(
                    async {
                        if let Some(future) = cover_future {
                            match future.await {
                                Ok(data) => {
                                    println!("✅ Cover art downloaded successfully: {} bytes", data.len());
                                    Some(data)
                                },
                                Err(e) => {
                                    println!("❌ Failed to download cover art: {}", e);
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    },
                    async {
                        if let Some(future) = lyrics_future {
                            match future.await {
                                Ok(result) => {
                                    println!("✅ Lyrics downloaded successfully for embedding");
                                    Some(result)
                                }
                                Err(e) => {
                                    println!("❌ Failed to download lyrics: {}", e);
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    }
                );
                
                (cover_result, lyrics_result)
            } else {
                (None, None)
            };

            // Embed metadata if requested
            if options.embed_metadata {
                self.send_progress(
                    &progress_sender,
                    &track.id,
                    DownloadStage::EmbeddingMetadata,
                    0.9,
                    "Embedding metadata...".to_string(),
                );
                
                self.metadata_embedder.embed_metadata(
                    &converted_path,
                    track,
                    cover_art_data.as_ref(),
                    lyrics_data.as_ref(),
                    options,
                ).await?;
            }

            // Save cover art to covers/ folder if we have cover art data
            if options.download_cover && cover_art_data.is_some() {
                self.send_progress(
                    &progress_sender,
                    &track.id,
                    DownloadStage::DownloadingCover,
                    0.95,
                    "Saving cover art to covers folder...".to_string(),
                );
                
                // Save the cover art data we already downloaded
                match self.save_cover_art_to_folder(
                    track,
                    &options.output_dir,
                    cover_art_data.as_ref().unwrap(),
                    &options.cover_format,
                ).await {
                    Ok(cover_path) => {
                        println!("✅ Cover art saved to: {}", cover_path.display());
                    }
                    Err(e) => {
                        println!("⚠️ Failed to save cover art to folder: {}", e);
                        // Don't fail the entire download for this
                    }
                }
            }

            println!("🎉 Download completed successfully: {} - {}", track.artist, track.title);
            
            self.send_progress(
                &progress_sender,
                &track.id,
                DownloadStage::Completed,
                1.0,
                "Download completed successfully!".to_string(),
            );

            return Ok(converted_path);
        } else {
            // No search results found
            println!("❌ No search results found for: {} - {}", track.artist, track.title);
            self.send_progress(
                &progress_sender,
                &track.id,
                DownloadStage::Error,
                0.0,
                "No search results found for this track".to_string(),
            );
        }

        // Try SoundCloud as fallback
        if let Ok(soundcloud_results) = self.soundcloud_downloader.search_tracks(&search_query).await {
            if !soundcloud_results.is_empty() {
                self.send_progress(
                    &progress_sender,
                    &track.id,
                    DownloadStage::DownloadingAudio,
                    0.3,
                    "Found SoundCloud source, downloading...".to_string(),
                );

                let output_path = self.get_output_path(track, options);
                // TODO: Implement SoundCloud download
                
                self.send_progress(
                    &progress_sender,
                    &track.id,
                    DownloadStage::Completed,
                    1.0,
                    "Download completed successfully!".to_string(),
                );

                return Ok(output_path);
            }
        }

        // Try yt-dlp as final fallback
        if self.ytdlp_downloader.is_available().await {
            self.send_progress(
                &progress_sender,
                &track.id,
                DownloadStage::DownloadingAudio,
                0.3,
                "Using yt-dlp fallback, downloading...".to_string(),
            );

            let output_path = self.get_output_path(track, options);
            let format_str = match options.format {
                AudioFormat::Mp3 => "mp3",
                AudioFormat::M4a => "m4a",
                AudioFormat::Flac => "flac",
                AudioFormat::Wav => "wav",
            };

            self.ytdlp_downloader.download_audio(
                &search_query,
                &output_path,
                format_str,
                options.bitrate.as_u32(),
            ).await?;

            self.send_progress(
                &progress_sender,
                &track.id,
                DownloadStage::Completed,
                1.0,
                "Download completed successfully!".to_string(),
            );

            return Ok(output_path);
        }

        Err(SpotifyDownloaderError::Download("No audio source found".to_string()))
    }

    /// Get the output path for a track
    fn get_output_path(&self, track: &TrackMetadata, options: &DownloadOptions) -> PathBuf {
        let mut path = options.output_dir.clone();
        
        // Create tracks directory
        path.push("tracks");
        
        // Create filename with artist and song name
        let formatted_artist = format_artists_for_filename(&track.artist);
        let filename = format!("{} - {}", 
            formatted_artist, 
            track.title
        );
        let sanitized_filename = sanitize_filename(&filename);
        let extension = match options.format {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "wav",
        };
        
        path.push(format!("{}.{}", sanitized_filename, extension));
        path
    }

    // Removed separate folder creation functions - everything is now embedded in metadata

    /// Convert audio to desired format and bitrate
    async fn convert_audio(&self, input_path: &PathBuf, options: &DownloadOptions) -> Result<PathBuf> {
        println!("Converting audio: {} to format {:?} at {} kbps", 
                 input_path.display(), options.format, options.bitrate.as_u32());
        
        // Create converter
        let converter = crate::downloader::converter::AudioConverter::new();
        
        // Check if conversion is needed
        if !converter.needs_conversion(input_path, options.format, options.bitrate) {
            println!("No conversion needed, using original file");
            return Ok(input_path.clone());
        }
        
        // Create output path with correct extension
        let mut output_path = input_path.clone();
        let extension = match options.format {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "wav",
        };
        
        // Replace extension
        if let Some(stem) = output_path.file_stem() {
            output_path.set_file_name(format!("{}.{}", stem.to_string_lossy(), extension));
        }
        
        // If input and output are the same, use a temporary file
        if input_path == &output_path {
            let mut temp_dir = options.output_dir.clone();
            temp_dir.push("temp");
            std::fs::create_dir_all(&temp_dir).ok(); // Create temp directory if it doesn't exist
            
            let temp_filename = format!("temp_convert_{}.{}", 
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                extension);
            output_path = temp_dir.join(temp_filename);
            println!("Using temporary file for conversion: {}", output_path.display());
        }
        
        // Perform conversion
        converter.convert_audio(input_path, &output_path, options.format, options.bitrate).await?;
        
        // If we used a temporary file, replace the original
        if input_path != &output_path {
            let mut temp_dir = options.output_dir.clone();
            temp_dir.push("temp");
            if output_path.parent() == Some(temp_dir.as_path()) {
                println!("Replacing original file with converted version");
                std::fs::rename(&output_path, input_path)
                    .map_err(|e| SpotifyDownloaderError::Conversion(format!("Failed to replace original file: {}", e)))?;
                return Ok(input_path.clone());
            }
        }
        
        Ok(output_path)
    }

    /// Convert synced lyrics to plain text
    fn convert_lyrics_to_text(&self, synced_lyrics: &crate::lyrics::SyncedLyrics) -> String {
        synced_lyrics.lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Save cover art data to covers/ folder
    async fn save_cover_art_to_folder(
        &self,
        track: &TrackMetadata,
        output_dir: &PathBuf,
        cover_art_data: &Vec<u8>,
        format: &str,
    ) -> Result<PathBuf> {
        // Create covers directory
        let mut covers_dir = output_dir.clone();
        covers_dir.push("covers");
        std::fs::create_dir_all(&covers_dir)
            .map_err(|e| SpotifyDownloaderError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to create covers directory: {}", e))))?;

        // Create filename
        let formatted_artist = format_artists_for_filename(&track.artist);
        let filename = format!("{} - {}", formatted_artist, track.title);
        let sanitized_filename = sanitize_filename(&filename);
        
        // Determine file extension
        let extension = match format.to_lowercase().as_str() {
            "jpg" | "jpeg" => "jpg",
            "png" => "png",
            "webp" => "webp",
            _ => "jpg", // Default to JPG
        };
        
        let cover_path = covers_dir.join(format!("{}.{}", sanitized_filename, extension));
        
        // Write cover art data to file
        std::fs::write(&cover_path, cover_art_data)
            .map_err(|e| SpotifyDownloaderError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to write cover art: {}", e))))?;
        
        Ok(cover_path)
    }

    /// Send progress update
    fn send_progress(
        &self,
        sender: &Option<mpsc::UnboundedSender<DownloadProgress>>,
        track_id: &str,
        stage: DownloadStage,
        progress: f32,
        message: String,
    ) {
        if let Some(sender) = sender {
            let _ = sender.send(DownloadProgress {
                track_id: track_id.to_string(),
                stage,
                progress,
                message,
            });
        }
    }
}

impl Clone for AudioDownloader {
    fn clone(&self) -> Self {
        Self {
            youtube_downloader: YoutubeDownloader::new(),
            soundcloud_downloader: SoundcloudDownloader::new(),
            ytdlp_downloader: YtDlpDownloader::new(),
            converter: AudioConverter::new(),
            cover_downloader: CoverDownloader::new(),
            metadata_embedder: MetadataEmbedder::new(),
            search_cache: HashMap::new(), // Start with empty cache for each instance
        }
    }
}

/// Format artists for filename with proper comma separation
fn format_artists_for_filename(artist: &str) -> String {
    // Common separators that indicate multiple artists
    let separators = ["feat.", "featuring", "ft.", "ft", "&", "x", "X", "vs", "vs.", "feat"];
    
    let mut formatted = artist.to_string();
    
    // Replace common separators with comma and space
    for separator in &separators {
        let pattern = format!(r"\b{}\b", regex::escape(separator));
        if let Ok(regex) = regex::Regex::new(&pattern) {
            formatted = regex.replace_all(&formatted, ", ").to_string();
        }
    }
    
    // Clean up multiple spaces and trim
    formatted = regex::Regex::new(r"\s+")
        .unwrap()
        .replace_all(&formatted, " ")
        .to_string();
    
    formatted.trim().to_string()
}

/// Sanitize filename by removing invalid characters and replacing semicolons with commas
fn sanitize_filename(filename: &str) -> String {
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
