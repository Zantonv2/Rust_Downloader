use crate::downloader::{DownloadOptions, DownloadProgress, TrackMetadata};
use crate::errors::{Result, SpotifyDownloaderError};
use std::path::PathBuf;
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;
use std::sync::Arc;

/// Async download manager that handles concurrent downloads
pub struct AsyncDownloadManager {
    audio_downloader: crate::downloader::audio::AudioDownloader,
    semaphore: Arc<Semaphore>,
    progress_sender: Option<mpsc::UnboundedSender<DownloadProgress>>,
}

/// Download task result
#[derive(Debug, Clone)]
pub struct DownloadTaskResult {
    pub track: TrackMetadata,
    pub success: bool,
    pub output_path: Option<PathBuf>,
    pub error: Option<String>,
}

impl AsyncDownloadManager {
    /// Create a new async download manager
    pub fn new(max_concurrent: usize) -> Self {
        // Get proxy-configured client
        let client = crate::api::get_api_manager()
            .map(|api_manager| api_manager.client().clone())
            .unwrap_or_else(|_| reqwest::Client::new());
        
        Self {
            audio_downloader: crate::downloader::audio::AudioDownloader::new_with_client(client),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            progress_sender: None,
        }
    }

    /// Set the progress sender for reporting download progress
    pub fn set_progress_sender(&mut self, sender: mpsc::UnboundedSender<DownloadProgress>) {
        self.progress_sender = Some(sender);
    }

    /// Download multiple tracks concurrently
    pub async fn download_tracks(
        &self,
        tracks: Vec<TrackMetadata>,
        options: &DownloadOptions,
        config: &crate::config::Config,
    ) -> Result<Vec<DownloadTaskResult>> {
        let mut handles: Vec<JoinHandle<DownloadTaskResult>> = Vec::new();

        for track in tracks {
            let semaphore = Arc::clone(&self.semaphore);
            // Get proxy-configured client for each task
            let client = crate::api::get_api_manager()
                .map(|api_manager| api_manager.client().clone())
                .unwrap_or_else(|_| reqwest::Client::new());
            let audio_downloader = crate::downloader::audio::AudioDownloader::new_with_client(client);
            let options = options.clone();
            let progress_sender = self.progress_sender.clone();
            let config = config.clone();

            let handle = tokio::spawn(async move {
                // Send "Queued" status when track is waiting for semaphore
                if let Some(sender) = &progress_sender {
                    let _ = sender.send(DownloadProgress {
                        track_id: track.id.clone(),
                        stage: crate::downloader::DownloadStage::Queued,
                        progress: 0.0,
                        message: "Queued for download...".to_string(),
                    });
                }

                let _permit = match semaphore.acquire().await {
                    Ok(permit) => {
                        // Send "Starting" status when track acquires semaphore and starts downloading
                        if let Some(sender) = &progress_sender {
                            let _ = sender.send(DownloadProgress {
                                track_id: track.id.clone(),
                                stage: crate::downloader::DownloadStage::SearchingSource,
                                progress: 0.1,
                                message: "Starting download...".to_string(),
                            });
                        }
                        permit
                    },
                    Err(e) => {
                        return DownloadTaskResult {
                            track: track.clone(),
                            success: false,
                            output_path: None,
                            error: Some(format!("Failed to acquire semaphore: {}", e)),
                        };
                    }
                };

                // Create a progress channel for this specific track
                let (track_progress_tx, mut track_progress_rx) = mpsc::unbounded_channel::<DownloadProgress>();
                let track_id = track.id.clone();
                let progress_sender_clone = progress_sender.clone();

                // Spawn progress monitoring task for this track
                if let Some(global_sender) = progress_sender_clone {
                    tokio::spawn(async move {
                        while let Some(progress) = track_progress_rx.recv().await {
                            let _ = global_sender.send(DownloadProgress {
                                track_id: track_id.clone(),
                                stage: progress.stage,
                                progress: progress.progress,
                                message: progress.message,
                            });
                        }
                    });
                }

                let result = {
                    let mut audio_downloader = audio_downloader;
                    audio_downloader
                        .download_track(&track, &options, Some(track_progress_tx), &config)
                        .await
                };

                match result {
                    Ok(output_path) => DownloadTaskResult {
                        track: track.clone(),
                        success: true,
                        output_path: Some(output_path),
                        error: None,
                    },
                    Err(e) => DownloadTaskResult {
                        track: track.clone(),
                        success: false,
                        output_path: None,
                        error: Some(e.to_string()),
                    },
                }
            });

            handles.push(handle);
        }

        // Wait for all downloads to complete
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    // Task was cancelled or panicked
                    results.push(DownloadTaskResult {
                        track: TrackMetadata {
                            title: "Unknown".to_string(),
                            artist: "Unknown".to_string(),
                            album: "Unknown".to_string(),
                            ..Default::default()
                        },
                        success: false,
                        output_path: None,
                        error: Some(format!("Task failed: {}", e)),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Download a single track (for backward compatibility)
    pub async fn download_single_track(
        &self,
        track: &TrackMetadata,
        options: &DownloadOptions,
        config: &crate::config::Config,
    ) -> Result<PathBuf> {
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            SpotifyDownloaderError::Download(format!("Failed to acquire semaphore: {}", e))
        })?;

        {
            let mut audio_downloader = self.audio_downloader.clone();
            audio_downloader
                .download_track(track, options, self.progress_sender.clone(), config)
                .await
        }
    }
}

impl Clone for AsyncDownloadManager {
    fn clone(&self) -> Self {
        // Get proxy-configured client for cloned instance
        let client = crate::api::get_api_manager()
            .map(|api_manager| api_manager.client().clone())
            .unwrap_or_else(|_| reqwest::Client::new());
        
        Self {
            audio_downloader: crate::downloader::audio::AudioDownloader::new_with_client(client),
            semaphore: Arc::clone(&self.semaphore),
            progress_sender: self.progress_sender.clone(),
        }
    }
}
