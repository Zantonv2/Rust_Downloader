use iced::{
    widget::{
        button, column, container, horizontal_space, pick_list, progress_bar,
        row, scrollable, text, text_input, vertical_space,
    },
    window::settings::PlatformSpecific,
    Alignment, Application, Command, Element, Length, Settings, Theme, Color,
    Background, Border, Shadow, executor, Subscription,
};
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::config::{AudioFormat, Bitrate};
use crate::downloader::{TrackMetadata, DownloadStage};
use crate::settings::Settings as AppSettings;
use crate::csv_import::{CsvImporter, CsvInfo};
use crate::errors::Result;


/// Main application state
#[derive(Debug, Clone)]
pub struct SpotifyDownloaderApp {
    // UI State
    current_view: View,
    theme: Theme,
    accent_color: Color,
    
    // Track Management
    tracks: Vec<TrackItem>,
    selected_tracks: Vec<usize>,
    
    // Import State
    url_input: String,
    csv_path: Option<PathBuf>,
    csv_info: Option<CsvInfo>,
    csv_import_progress: Option<f32>,
    csv_import_status: Option<String>,
    
    // Settings State
    settings: AppSettings,
    settings_open: bool,
    current_settings_tab: SettingsTab,
    
    // API Settings
    spotify_client_id: String,
    spotify_client_secret: String,
    lastfm_api_key: String,
    lastfm_client_secret: String,
    youtube_api_key: String,
    soundcloud_client_id: String,
    genius_api_key: String,
    musixmatch_api_key: String,
    
    // Download State
    download_progress: HashMap<String, f32>,
    download_status: HashMap<String, DownloadStage>,
    
    // UI State
    url_validation_error: Option<String>,
    settings_message: Option<String>,
    output_directory: String,
    selected_format: AudioFormat,
    selected_bitrate: Bitrate,
    download_lyrics: bool,
    download_cover: bool,
    embed_metadata: bool,
    max_concurrent_downloads: u32,
    
    // Individual Metadata Toggles
    embed_title: bool,
    embed_artist: bool,
    embed_album: bool,
    embed_year: bool,
    embed_genre: bool,
    embed_track_number: bool,
    embed_disc_number: bool,
    embed_album_artist: bool,
    embed_composer: bool,
    embed_comment: bool,
    
    // Async Communication
    command_sender: Option<mpsc::UnboundedSender<AppCommand>>,
    progress_sender: Option<mpsc::UnboundedSender<crate::downloader::DownloadProgress>>,
    
    // Drag & Drop State
    is_drag_over: bool,
    
    // Cookies Settings
    use_cookies: bool,
    selected_browser: String,
    cookie_import_status: Option<String>,
    enable_sponsorblock: bool,
    sponsorblock_categories: Vec<String>,
    
}

#[derive(Debug, Clone)]
pub enum View {
    Import,
    TrackList,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsTab {
    General,
    Audio,
    Metadata,
    API,
    Advanced,
}

#[derive(Debug, Clone)]
pub struct TrackItem {
    pub metadata: TrackMetadata,
    pub status: TrackStatus,
    pub progress: f32,
    pub error_message: Option<String>,
    pub current_stage: Option<DownloadStage>,
    pub stage_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
    Paused,
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    AddTrack(TrackMetadata),
    AddMultipleTracks(Vec<TrackMetadata>),
    UpdateProgress(String, f32),
    UpdateStatus(String, DownloadStage),
    SetError(String, String),
    CompleteDownload(String),
    BatchDownloadComplete(Vec<crate::downloader::DownloadTaskResult>),
}

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    SwitchView(View),
    
    // URL Import
    UrlInputChanged(String),
    ImportUrl,
    ValidateUrl,
    
    // CSV Import
    SelectCsvFile,
    CsvFileSelected(Option<PathBuf>),
    ImportCsv,
    CsvImportProgress(f32),
    CsvImportStatus(String),
    
    // Track Management
    SelectTrack(usize),
    SelectAllTracks,
    ClearSelection,
    InvertSelection,
    SelectByStatus(TrackStatus),
    DeleteTrack(usize),
    DeleteSelectedTracks,
    ClearAllTracks,
    RetryTrack(usize),
    RetrySelectedTracks,
    ClearError(usize),
    DownloadTrack(usize),
    DownloadSelectedTracks,
    
    // Download
    StartDownload,
    PauseDownload,
    StopDownload,
    
    // Settings
    ToggleSettings,
    UpdateOutputDirectory(String),
    SelectOutputDirectory,
    OpenOutputFolder,
    UpdateFormat(AudioFormat),
    UpdateBitrate(Bitrate),
    ToggleLyrics(bool),
    ToggleCover(bool),
    ToggleMetadata(bool),
    UpdateConcurrency(u32),
    
    // Individual Metadata Toggles
    ToggleTitle(bool),
    ToggleArtist(bool),
    ToggleAlbum(bool),
    ToggleYear(bool),
    ToggleGenre(bool),
    ToggleTrackNumber(bool),
    ToggleDiscNumber(bool),
    ToggleAlbumArtist(bool),
    ToggleComposer(bool),
    ToggleComment(bool),
    SaveSettings,
    ResetSettings,
    
    // Settings Tabs
    SwitchSettingsTab(SettingsTab),
    
    // API Settings
    UpdateSpotifyClientId(String),
    UpdateSpotifyClientSecret(String),
    UpdateLastfmApiKey(String),
    UpdateLastfmClientSecret(String),
    UpdateYoutubeApiKey(String),
    UpdateSoundcloudClientId(String),
    UpdateGeniusApiKey(String),
    UpdateMusixmatchApiKey(String),
    
    // Theme
    ToggleTheme,
    SetTheme(Theme),
    SetAccentColor(Color),
    
    
    // Async Commands
    CommandReceived(AppCommand),
    
    // Keyboard shortcuts
    KeyPressed(String),
    
    // Drag & Drop
    FileDropped(Vec<PathBuf>),
    DragEntered,
    DragExited,
    
    // Cookies Settings
    ToggleUseCookies(bool),
    SelectBrowser(String),
    TestCookieImport,
    ToggleSponsorBlock(bool),
    ToggleSponsorBlockCategory(String),
    ToggleCookies,
    ToggleBrowserSelection(String),
    CookieTestResult(std::result::Result<String, String>),
    
}

impl Application for SpotifyDownloaderApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let settings = AppSettings::load_from_local_json().unwrap_or_else(|_| {
            let settings = AppSettings::default();
            settings.save_to_local_json().ok();
            settings
        });

        let config = settings.config().clone();
        let ui_prefs = &config.ui_preferences;
        
        let app = Self {
            current_view: View::Import,
            theme: match ui_prefs.theme.as_str() {
                "dark" => Theme::Dark,
                "light" => Theme::Light,
                _ => Theme::Light,
            },
            accent_color: Color::from_rgb(0.2, 0.7, 0.2),
            tracks: Vec::new(),
            selected_tracks: Vec::new(),
            url_input: String::new(),
            csv_path: None,
            csv_info: None,
            settings,
            settings_open: false,
            current_settings_tab: SettingsTab::General,
            spotify_client_id: config.api_keys.spotify_client_id.clone().unwrap_or_default(),
            spotify_client_secret: config.api_keys.spotify_client_secret.clone().unwrap_or_default(),
            lastfm_api_key: config.api_keys.lastfm_api_key.clone().unwrap_or_default(),
            lastfm_client_secret: config.api_keys.lastfm_client_secret.clone().unwrap_or_default(),
            youtube_api_key: String::new(), // Not in new config yet
            soundcloud_client_id: String::new(), // Not in new config yet
            genius_api_key: config.api_keys.genius_access_token.clone().unwrap_or_default(),
            musixmatch_api_key: config.api_keys.musixmatch_api_key.clone().unwrap_or_default(),
            download_progress: HashMap::new(),
            download_status: HashMap::new(),
            url_validation_error: None,
            settings_message: None,
            output_directory: config.download_directory.to_string_lossy().to_string(),
            selected_format: config.default_format,
            selected_bitrate: config.default_bitrate,
            download_lyrics: ui_prefs.auto_download_lyrics,
            download_cover: ui_prefs.auto_download_covers,
            embed_metadata: config.metadata_config.embed_metadata,
            max_concurrent_downloads: ui_prefs.max_concurrent_downloads,
            embed_title: config.metadata_config.embed_title,
            embed_artist: config.metadata_config.embed_artist,
            embed_album: config.metadata_config.embed_album,
            embed_year: config.metadata_config.embed_year,
            embed_genre: config.metadata_config.embed_genre,
            embed_track_number: config.metadata_config.embed_track_number,
            embed_disc_number: config.metadata_config.embed_disc_number,
            embed_album_artist: config.metadata_config.embed_album_artist,
            embed_composer: true, // Not in metadata config yet
            embed_comment: true, // Not in metadata config yet
            command_sender: None,
            progress_sender: None,
            is_drag_over: false,
            csv_import_progress: None,
            csv_import_status: None,
            use_cookies: false,
            selected_browser: "Chrome".to_string(),
            cookie_import_status: None,
            enable_sponsorblock: false,
            sponsorblock_categories: vec!["sponsor".to_string(), "intro".to_string(), "outro".to_string()],
        };

        (app, Command::none())
    }

    fn title(&self) -> String {
        "Spotify Downloader".to_string()
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchView(view) => {
                self.current_view = view;
                // Clear messages when switching views
                self.url_validation_error = None;
                self.settings_message = None;
            }
            
            Message::UrlInputChanged(input) => {
                self.url_input = input;
                self.url_validation_error = None;
            }
            
            Message::ImportUrl => {
                println!("ImportUrl message received");
                if self.validate_url() {
                    let url = self.url_input.clone();
                    println!("URL validated, starting import for: {}", url);
                    self.url_input.clear();
                    
                    // Spawn async task to fetch metadata
                    return Command::perform(
                        async move {
                            println!("Starting async task to fetch metadata for: {}", url);
                            
                            // Determine URL type and call appropriate function
                            if url.contains("/playlist/") {
                                println!("Detected playlist URL, fetching playlist metadata");
                                match crate::downloader::api_wrapper::ApiWrapper::get_spotify_playlist_metadata(&url).await {
                                    Ok(playlist) => {
                                        println!("Successfully fetched playlist: {} with {} tracks", playlist.name, playlist.tracks.len());
                                        Ok(playlist.tracks)
                                    },
                                    Err(e) => {
                                        println!("Failed to fetch playlist metadata: {}", e);
                                        Err(format!("Failed to fetch playlist metadata: {}", e))
                                    },
                                }
                            } else if url.contains("/album/") {
                                println!("Detected album URL, fetching album metadata");
                                match crate::downloader::api_wrapper::ApiWrapper::get_spotify_album_metadata(&url).await {
                                    Ok(album) => {
                                        println!("Successfully fetched album: {} with {} tracks", album.name, album.tracks.len());
                                        Ok(album.tracks)
                                    },
                                    Err(e) => {
                                        println!("Failed to fetch album metadata: {}", e);
                                        Err(format!("Failed to fetch album metadata: {}", e))
                                    },
                                }
                            } else {
                                // Assume it's a track URL
                                println!("Detected track URL, fetching track metadata");
                                match crate::downloader::api_wrapper::ApiWrapper::get_spotify_track_metadata(&url).await {
                                    Ok(metadata) => {
                                        println!("Successfully fetched metadata: {} - {}", metadata.artist, metadata.title);
                                        Ok(vec![metadata])
                                    },
                                    Err(e) => {
                                        println!("Failed to fetch track metadata: {}", e);
                                        Err(format!("Failed to fetch track metadata: {}", e))
                                    },
                                }
                            }
                        },
                        |result| {
                            println!("Async task completed, processing result");
                            match result {
                                Ok(tracks) => {
                                    if tracks.len() == 1 {
                                        println!("Sending AddTrack command for: {} - {}", tracks[0].artist, tracks[0].title);
                                        Message::CommandReceived(AppCommand::AddTrack(tracks[0].clone()))
                                    } else {
                                        println!("Sending AddMultipleTracks command for {} tracks", tracks.len());
                                        Message::CommandReceived(AppCommand::AddMultipleTracks(tracks))
                                    }
                                },
                                Err(error) => {
                                    println!("Sending SetError command: {}", error);
                                    Message::CommandReceived(AppCommand::SetError("url_import".to_string(), error))
                                },
                            }
                        }
                    );
                } else {
                    println!("URL validation failed");
                }
            }
            
            Message::ValidateUrl => {
                self.validate_url();
            }
            
            Message::SelectCsvFile => {
                return Command::perform(select_file(), |path| {
                    Message::CsvFileSelected(path)
                });
            }
            
            Message::CsvFileSelected(path) => {
                self.csv_path = path;
                if let Some(ref csv_path) = self.csv_path {
                    let csv_importer = CsvImporter::new();
                    match csv_importer.get_csv_info(csv_path) {
                        Ok(info) => {
                            self.csv_info = Some(info);
                        }
                        Err(e) => {
                            self.url_validation_error = Some(format!("CSV Error: {}", e));
                        }
                    }
                }
            }
            
            Message::ImportCsv => {
                if let Some(ref csv_path) = self.csv_path {
                    let csv_path = csv_path.clone();
                    
                    // Spawn async task to import tracks from CSV
                    return Command::perform(
                        async move {
                            let importer = CsvImporter::new();
                            match importer.import_from_csv(&csv_path).await {
                                Ok(tracks) => Ok(tracks),
                                Err(e) => Err(format!("Failed to import CSV: {}", e)),
                            }
                        },
                        |result| match result {
                            Ok(tracks) => Message::CommandReceived(AppCommand::AddMultipleTracks(tracks)),
                            Err(error) => Message::CommandReceived(AppCommand::SetError("csv_import".to_string(), error)),
                        }
                    );
                }
            }
            
            Message::SelectTrack(index) => {
                println!("SelectTrack button pressed for index: {}", index);
                if self.selected_tracks.contains(&index) {
                    self.selected_tracks.retain(|&i| i != index);
                    println!("Track {} deselected", index);
                } else {
                    self.selected_tracks.push(index);
                    println!("Track {} selected", index);
                }
            }
            
            Message::SelectAllTracks => {
                println!("SelectAllTracks button pressed");
                self.selected_tracks = (0..self.tracks.len()).collect();
                println!("Selected {} tracks", self.selected_tracks.len());
            }
            
            Message::ClearSelection => {
                println!("ClearSelection button pressed");
                self.selected_tracks.clear();
                println!("Selection cleared");
            }
            
            Message::DeleteTrack(index) => {
                println!("DeleteTrack button pressed for index: {}", index);
                if index < self.tracks.len() {
                    let track_title = self.tracks[index].metadata.title.clone();
                    self.tracks.remove(index);
                    println!("Deleted track: {}", track_title);
                    // Update selected indices
                    self.selected_tracks.retain(|&i| i != index);
                    let mut new_selected = self.selected_tracks.clone();
                    new_selected = new_selected
                        .into_iter()
                        .map(|i| if i > index { i - 1 } else { i })
                        .collect();
                    self.selected_tracks = new_selected;
                }
            }
            
            Message::DeleteSelectedTracks => {
                println!("DeleteSelectedTracks button pressed");
                // Sort indices in descending order to avoid index shifting
                let mut indices = self.selected_tracks.clone();
                indices.sort_by(|a, b| b.cmp(a));
                println!("Deleting {} selected tracks", indices.len());
                for &index in &indices {
                    if index < self.tracks.len() {
                        self.tracks.remove(index);
                    }
                }
                self.selected_tracks.clear();
                println!("Selected tracks deleted");
            }
            
            Message::ClearAllTracks => {
                println!("ClearAllTracks button pressed");
                let track_count = self.tracks.len();
                self.tracks.clear();
                self.selected_tracks.clear();
                println!("Cleared all {} tracks", track_count);
            }
            
            Message::RetryTrack(index) => {
                println!("RetryTrack button pressed for index: {}", index);
                if index < self.tracks.len() {
                    let track_title = self.tracks[index].metadata.title.clone();
                    self.tracks[index].status = TrackStatus::Pending;
                    self.tracks[index].progress = 0.0;
                    self.tracks[index].error_message = None;
                    println!("Retrying track: {}", track_title);
                }
            }
            
            Message::RetrySelectedTracks => {
                println!("RetrySelectedTracks button pressed");
                for &index in &self.selected_tracks {
                    if index < self.tracks.len() {
                        self.tracks[index].status = TrackStatus::Pending;
                        self.tracks[index].progress = 0.0;
                        self.tracks[index].error_message = None;
                    }
                }
                println!("Retrying {} selected tracks", self.selected_tracks.len());
            }
            
            Message::ClearError(index) => {
                if let Some(track) = self.tracks.get_mut(index) {
                    track.error_message = None;
                    if matches!(track.status, TrackStatus::Failed) {
                        track.status = TrackStatus::Pending;
                    }
                }
            }
            
            Message::InvertSelection => {
                let all_indices: Vec<usize> = (0..self.tracks.len()).collect();
                let mut new_selection = Vec::new();
                
                for &index in &all_indices {
                    if !self.selected_tracks.contains(&index) {
                        new_selection.push(index);
                    }
                }
                
                self.selected_tracks = new_selection;
            }
            
            Message::SelectByStatus(status) => {
                self.selected_tracks.clear();
                for (index, track) in self.tracks.iter().enumerate() {
                    if track.status == status {
                        self.selected_tracks.push(index);
                    }
                }
            }
            
            Message::DownloadSelectedTracks => {
                println!("DownloadSelectedTracks button pressed");
                println!("Downloading {} selected tracks", self.selected_tracks.len());
                // Download all selected tracks
                for &index in &self.selected_tracks {
                    if let Some(track) = self.tracks.get_mut(index) {
                        if matches!(track.status, TrackStatus::Pending | TrackStatus::Failed) {
                            println!("Starting download for selected track: {} - {}", track.metadata.artist, track.metadata.title);
                            track.status = TrackStatus::Downloading;
                            track.progress = 0.0;
                            track.error_message = None;
                            
                            // Start download for this track
                            let metadata = track.metadata.clone();
                            let track_id = metadata.id.clone();
                            let output_dir = PathBuf::from(&self.output_directory);
                            let format = self.selected_format;
                            let bitrate = self.selected_bitrate;
                            
                            // Create progress channel
                            let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<crate::downloader::DownloadProgress>();
                            
                            // Spawn progress monitoring task
                            let track_id_clone = track_id.clone();
                            let command_sender = self.command_sender.clone();
                            tokio::spawn(async move {
                                while let Some(progress) = progress_rx.recv().await {
                                    if let Some(sender) = &command_sender {
                                        let _ = sender.send(AppCommand::UpdateProgress(track_id_clone.clone(), progress.progress));
                                        let _ = sender.send(AppCommand::UpdateStatus(track_id_clone.clone(), progress.stage));
                                    }
                                }
                            });
                            
                            // Clone metadata toggles for the async closure
                            let embed_title = self.embed_title;
                            let embed_artist = self.embed_artist;
                            let embed_album = self.embed_album;
                            let embed_year = self.embed_year;
                            let embed_genre = self.embed_genre;
                            let embed_track_number = self.embed_track_number;
                            let embed_disc_number = self.embed_disc_number;
                            let embed_album_artist = self.embed_album_artist;
                            let embed_composer = self.embed_composer;
                            let embed_comment = self.embed_comment;
                            
                            // Clone config for the async closure
                            let config = self.settings.config().clone();
                            
                            // Spawn download task
                            return Command::perform(
                                async move {
                                    use crate::downloader::{AudioDownloader, DownloadOptions};
                                    
                                    // Get proxy-configured client
                                    let api_manager = crate::api::get_api_manager().unwrap_or_else(|_| {
                                        panic!("API manager not initialized");
                                    });
                                    let client = api_manager.client().clone();
                                    let mut downloader = AudioDownloader::new_with_client(client);
                                    let options = DownloadOptions {
                                        format,
                                        bitrate,
                                        output_dir,
                                        download_lyrics: true,
                                        download_cover: true,
                                        embed_metadata: true,
                                        cover_width: 800,
                                        cover_height: 800,
                                        cover_format: "jpg".to_string(),
                                        // Individual Metadata Toggles (matching UI state)
                                        embed_title,
                                        embed_artist,
                                        embed_album,
                                        embed_year,
                                        embed_genre,
                                        embed_track_number,
                                        embed_disc_number,
                                        embed_album_artist,
                                        embed_composer,
                                        embed_comment,
                                    };
                                    
                                    match downloader.download_track(&metadata, &options, Some(progress_tx), &config).await {
                                        Ok(_) => Ok(track_id),
                                        Err(e) => Err(format!("Download failed: {}", e)),
                                    }
                                },
                                |result| match result {
                                    Ok(track_id) => Message::CommandReceived(AppCommand::CompleteDownload(track_id)),
                                    Err(error) => Message::CommandReceived(AppCommand::SetError("download".to_string(), error)),
                                }
                            );
                        }
                    }
                }
            }
            
            
            Message::DownloadTrack(index) => {
                println!("DownloadTrack button pressed for index: {}", index);
                if let Some(track) = self.tracks.get_mut(index) {
                    if matches!(track.status, TrackStatus::Pending | TrackStatus::Failed) {
                        println!("Starting download for track: {} - {}", track.metadata.artist, track.metadata.title);
                        track.status = TrackStatus::Downloading;
                        track.progress = 0.0;
                        track.error_message = None;
                        
                        // Start download for this specific track
                        let metadata = track.metadata.clone();
                        let track_id = metadata.id.clone();
                        let output_dir = PathBuf::from(&self.output_directory);
                        let format = self.selected_format;
                        let bitrate = self.selected_bitrate;
                        
                        // Create progress channel
                        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<crate::downloader::DownloadProgress>();
                        
                        // Spawn progress monitoring task
                        let track_id_clone = track_id.clone();
                        let command_sender = self.command_sender.clone();
                        tokio::spawn(async move {
                            while let Some(progress) = progress_rx.recv().await {
                                if let Some(sender) = &command_sender {
                                    let _ = sender.send(AppCommand::UpdateProgress(track_id_clone.clone(), progress.progress));
                                    let _ = sender.send(AppCommand::UpdateStatus(track_id_clone.clone(), progress.stage));
                                }
                            }
                        });
                        
                        // Clone metadata toggles for the async closure
                        let embed_title = self.embed_title;
                        let embed_artist = self.embed_artist;
                        let embed_album = self.embed_album;
                        let embed_year = self.embed_year;
                        let embed_genre = self.embed_genre;
                        let embed_track_number = self.embed_track_number;
                        let embed_disc_number = self.embed_disc_number;
                        let embed_album_artist = self.embed_album_artist;
                        let embed_composer = self.embed_composer;
                        let embed_comment = self.embed_comment;
                        
                        // Clone config for the async closure
                        let config = self.settings.config().clone();
                        
                        return Command::perform(
                            async move {
                                use crate::downloader::{AudioDownloader, DownloadOptions};
                                
                                // Get proxy-configured client
                                let api_manager = crate::api::get_api_manager().unwrap_or_else(|_| {
                                    panic!("API manager not initialized");
                                });
                                let client = api_manager.client().clone();
                                let mut downloader = AudioDownloader::new_with_client(client);
                                let options = DownloadOptions {
                                    format,
                                    bitrate,
                                    output_dir,
                                    download_lyrics: true,
                                    download_cover: true,
                                    embed_metadata: true,
                                    cover_width: 800,
                                    cover_height: 800,
                                    cover_format: "jpg".to_string(),
                                    // Individual Metadata Toggles (matching UI state)
                                    embed_title,
                                    embed_artist,
                                    embed_album,
                                    embed_year,
                                    embed_genre,
                                    embed_track_number,
                                    embed_disc_number,
                                    embed_album_artist,
                                    embed_composer,
                                    embed_comment,
                                };
                                
                                match downloader.download_track(&metadata, &options, Some(progress_tx), &config).await {
                                    Ok(_) => Ok(track_id),
                                    Err(e) => Err(format!("Download failed: {}", e)),
                                }
                            },
                            |result| match result {
                                Ok(track_id) => Message::CommandReceived(AppCommand::CompleteDownload(track_id)),
                                Err(error) => Message::CommandReceived(AppCommand::SetError("download".to_string(), error)),
                            }
                        );
                    }
                }
            }
            
            Message::StartDownload => {
                println!("StartDownload button pressed");
                // Start download process for pending tracks using AsyncDownloadManager
                let pending_tracks: Vec<TrackMetadata> = self.tracks
                    .iter()
                    .filter(|track| matches!(track.status, TrackStatus::Pending))
                    .map(|track| track.metadata.clone())
                    .collect();

                println!("Found {} pending tracks to download", pending_tracks.len());
                if !pending_tracks.is_empty() {
                    // Update all pending tracks to downloading status
                    for track in &mut self.tracks {
                        if matches!(track.status, TrackStatus::Pending) {
                            track.status = TrackStatus::Downloading;
                        }
                    }

                    let output_dir = PathBuf::from(&self.output_directory);
                    let format = self.selected_format;
                    let bitrate = self.selected_bitrate;
                    let max_concurrent = self.max_concurrent_downloads as usize;
                    
                    // Clone metadata toggles for the async closure
                    let embed_title = self.embed_title;
                    let embed_artist = self.embed_artist;
                    let embed_album = self.embed_album;
                    let embed_year = self.embed_year;
                    let embed_genre = self.embed_genre;
                    let embed_track_number = self.embed_track_number;
                    let embed_disc_number = self.embed_disc_number;
                    let embed_album_artist = self.embed_album_artist;
                    let embed_composer = self.embed_composer;
                    let embed_comment = self.embed_comment;
                    
                    // Clone config for the async closure
                    let config = self.settings.config().clone();
                    
                    // Create progress channel for monitoring
                    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<crate::downloader::DownloadProgress>();
                    let (command_tx, _command_rx) = mpsc::unbounded_channel::<AppCommand>();
                    
                    // Store the command sender for progress updates
                    self.command_sender = Some(command_tx.clone());
                    
                    // Spawn progress monitoring task
                    let command_tx_clone = command_tx.clone();
                    tokio::spawn(async move {
                        while let Some(progress) = progress_rx.recv().await {
                            // Use the track_id from the progress message
                            let _ = command_tx_clone.send(AppCommand::UpdateProgress(progress.track_id.clone(), progress.progress));
                            let _ = command_tx_clone.send(AppCommand::UpdateStatus(progress.track_id, progress.stage));
                        }
                    });
                    
                    return Command::perform(
                        async move {
                            use crate::downloader::{AsyncDownloadManager, DownloadOptions};
                            
                            let mut download_manager = AsyncDownloadManager::new(max_concurrent);
                            download_manager.set_progress_sender(progress_tx);
                            
                            let options = DownloadOptions {
                                format,
                                bitrate,
                                output_dir,
                                download_lyrics: true,
                                download_cover: true,
                                embed_metadata: true,
                                cover_width: 800,
                                cover_height: 800,
                                cover_format: "jpg".to_string(),
                                // Individual Metadata Toggles (matching UI state)
                                embed_title,
                                embed_artist,
                                embed_album,
                                embed_year,
                                embed_genre,
                                embed_track_number,
                                embed_disc_number,
                                embed_album_artist,
                                embed_composer,
                                embed_comment,
                            };
                            
                            match download_manager.download_tracks(pending_tracks, &options, &config).await {
                                Ok(results) => Ok(results),
                                Err(e) => Err(format!("Download manager failed: {}", e)),
                            }
                        },
                        |result| match result {
                            Ok(results) => Message::CommandReceived(AppCommand::BatchDownloadComplete(results)),
                            Err(error) => Message::CommandReceived(AppCommand::SetError("batch_download".to_string(), error)),
                        }
                    );
                }
            }
            
            Message::PauseDownload => {
                println!("PauseDownload button pressed");
                let mut paused_count = 0;
                for track in &mut self.tracks {
                    if matches!(track.status, TrackStatus::Downloading) {
                        track.status = TrackStatus::Paused;
                        paused_count += 1;
                    }
                }
                println!("Paused {} downloading tracks", paused_count);
            }
            
            Message::StopDownload => {
                println!("StopDownload button pressed");
                let mut stopped_count = 0;
                for track in &mut self.tracks {
                    if matches!(track.status, TrackStatus::Downloading | TrackStatus::Paused) {
                        track.status = TrackStatus::Pending;
                        track.progress = 0.0;
                        stopped_count += 1;
                    }
                }
                println!("Stopped {} tracks", stopped_count);
            }
            
            Message::ToggleSettings => {
                println!("[UI] ToggleSettings: {}", !self.settings_open);
                self.settings_open = !self.settings_open;
            }
            
            Message::UpdateOutputDirectory(path) => {
                self.output_directory = path;
            }
            
            Message::SelectOutputDirectory => {
                println!("[UI] SelectOutputDirectory pressed");
                return Command::perform(select_directory(), |path| {
                    Message::UpdateOutputDirectory(path.unwrap_or_default().to_string_lossy().to_string())
                });
            }
            
            Message::OpenOutputFolder => {
                println!("[UI] OpenOutputFolder pressed");
                let output_dir = self.output_directory.clone();
                // Simple implementation - just log the action
                println!("Opening folder: {}", output_dir);
                return Command::none();
            }
            
            Message::UpdateFormat(format) => {
                self.selected_format = format;
            }
            
            Message::UpdateBitrate(bitrate) => {
                self.selected_bitrate = bitrate;
            }
            
            Message::UpdateConcurrency(concurrency) => {
                self.max_concurrent_downloads = concurrency;
            }
            
            Message::ToggleLyrics(enabled) => {
                self.download_lyrics = enabled;
            }
            
            Message::ToggleCover(enabled) => {
                self.download_cover = enabled;
            }
            
            Message::ToggleMetadata(enabled) => {
                self.embed_metadata = enabled;
            }
            
            // Individual Metadata Toggles
            Message::ToggleTitle(enabled) => {
                self.embed_title = enabled;
            }
            
            Message::ToggleArtist(enabled) => {
                self.embed_artist = enabled;
            }
            
            Message::ToggleAlbum(enabled) => {
                self.embed_album = enabled;
            }
            
            Message::ToggleYear(enabled) => {
                self.embed_year = enabled;
            }
            
            Message::ToggleGenre(enabled) => {
                self.embed_genre = enabled;
            }
            
            Message::ToggleTrackNumber(enabled) => {
                self.embed_track_number = enabled;
            }
            
            Message::ToggleDiscNumber(enabled) => {
                self.embed_disc_number = enabled;
            }
            
            Message::ToggleAlbumArtist(enabled) => {
                self.embed_album_artist = enabled;
            }
            
            Message::ToggleComposer(enabled) => {
                self.embed_composer = enabled;
            }
            
            Message::ToggleComment(enabled) => {
                self.embed_comment = enabled;
            }
            
            Message::SaveSettings => {
                println!("SaveSettings button pressed");
                // Update all settings from UI state
                if let Err(e) = self.settings.set_download_directory(PathBuf::from(&self.output_directory)) {
                    self.settings_message = Some(format!("Settings Error: {}", e));
                    return Command::none();
                }
                
                // Update audio format and bitrate
                if let Err(e) = self.settings.set_default_format(self.selected_format) {
                    self.settings_message = Some(format!("Settings Error: {}", e));
                    return Command::none();
                }
                
                if let Err(e) = self.settings.set_default_bitrate(self.selected_bitrate) {
                    self.settings_message = Some(format!("Settings Error: {}", e));
                    return Command::none();
                }
                
                // Update API keys
                if let Err(e) = self.settings.set_spotify_credentials(
                    self.spotify_client_id.clone(),
                    self.spotify_client_secret.clone()
                ) {
                    self.settings_message = Some(format!("Settings Error: {}", e));
                    return Command::none();
                }
                
                if let Err(e) = self.settings.set_lastfm_credentials(
                    self.lastfm_api_key.clone(),
                    self.lastfm_client_secret.clone()
                ) {
                    self.settings_message = Some(format!("Settings Error: {}", e));
                    return Command::none();
                }
                
                if let Err(e) = self.settings.set_genius_access_token(self.genius_api_key.clone()) {
                    self.settings_message = Some(format!("Settings Error: {}", e));
                    return Command::none();
                }
                
                if let Err(e) = self.settings.set_musixmatch_api_key(self.musixmatch_api_key.clone()) {
                    self.settings_message = Some(format!("Settings Error: {}", e));
                    return Command::none();
                }
                
                // Update UI preferences
                let ui_prefs = self.settings.ui_preferences_mut();
                ui_prefs.auto_download_lyrics = self.download_lyrics;
                ui_prefs.auto_download_covers = self.download_cover;
                ui_prefs.max_concurrent_downloads = self.max_concurrent_downloads;
                ui_prefs.theme = match self.theme {
                    Theme::Light => "light".to_string(),
                    Theme::Dark => "dark".to_string(),
                    _ => "light".to_string(),
                };
                
                // Update metadata settings
                let metadata_config = self.settings.metadata_config_mut();
                metadata_config.embed_metadata = self.embed_metadata;
                metadata_config.embed_title = self.embed_title;
                metadata_config.embed_artist = self.embed_artist;
                metadata_config.embed_album = self.embed_album;
                metadata_config.embed_year = self.embed_year;
                metadata_config.embed_genre = self.embed_genre;
                metadata_config.embed_track_number = self.embed_track_number;
                metadata_config.embed_disc_number = self.embed_disc_number;
                metadata_config.embed_album_artist = self.embed_album_artist;
                metadata_config.embed_lyrics = self.download_lyrics;
                metadata_config.embed_cover = self.download_cover;
                
                // Save settings
                if let Err(e) = self.settings.save() {
                    self.settings_message = Some(format!("Failed to save settings: {}", e));
                    return Command::none();
                }
                
                self.settings_open = false;
                self.settings_message = Some("Settings saved successfully!".to_string());
            }
            
            Message::ResetSettings => {
                // Reset to default settings
                self.settings = AppSettings::default();
                
                // Update UI state from default settings
                let config = self.settings.config();
                let ui_prefs = &config.ui_preferences;
                
                self.output_directory = config.download_directory.to_string_lossy().to_string();
                self.selected_format = config.default_format;
                self.selected_bitrate = config.default_bitrate;
                self.download_lyrics = ui_prefs.auto_download_lyrics;
                self.download_cover = ui_prefs.auto_download_covers;
                self.embed_metadata = config.metadata_config.embed_metadata;
                self.embed_title = config.metadata_config.embed_title;
                self.embed_artist = config.metadata_config.embed_artist;
                self.embed_album = config.metadata_config.embed_album;
                self.embed_year = config.metadata_config.embed_year;
                self.embed_genre = config.metadata_config.embed_genre;
                self.embed_track_number = config.metadata_config.embed_track_number;
                self.embed_disc_number = config.metadata_config.embed_disc_number;
                self.embed_album_artist = config.metadata_config.embed_album_artist;
                self.embed_composer = true; // Not in metadata config yet
                self.embed_comment = true; // Not in metadata config yet
                
                // Clear API keys
                self.spotify_client_id = config.api_keys.spotify_client_id.clone().unwrap_or_default();
                self.spotify_client_secret = config.api_keys.spotify_client_secret.clone().unwrap_or_default();
                self.lastfm_api_key = config.api_keys.lastfm_api_key.clone().unwrap_or_default();
                self.lastfm_client_secret = config.api_keys.lastfm_client_secret.clone().unwrap_or_default();
                self.youtube_api_key.clear(); // Not in new config yet
                self.soundcloud_client_id.clear(); // Not in new config yet
                self.genius_api_key = config.api_keys.genius_access_token.clone().unwrap_or_default();
                self.musixmatch_api_key = config.api_keys.musixmatch_api_key.clone().unwrap_or_default();
                
                // Update theme
                self.theme = match ui_prefs.theme.as_str() {
                    "dark" => Theme::Dark,
                    "light" => Theme::Light,
                    _ => Theme::Light,
                };
                
                // Save the reset settings
                if let Err(e) = self.settings.save() {
                    self.settings_message = Some(format!("Failed to save reset settings: {}", e));
                } else {
                    self.settings_message = Some("Settings reset to defaults and saved".to_string());
                }
            }
            
            Message::SwitchSettingsTab(tab) => {
                self.current_settings_tab = tab;
            }
            
            Message::UpdateSpotifyClientId(id) => {
                self.spotify_client_id = id;
            }
            
            Message::UpdateSpotifyClientSecret(secret) => {
                self.spotify_client_secret = secret;
            }
            
            Message::UpdateLastfmApiKey(key) => {
                self.lastfm_api_key = key;
            }
            
            Message::UpdateLastfmClientSecret(secret) => {
                self.lastfm_client_secret = secret;
            }
            
            Message::UpdateYoutubeApiKey(key) => {
                self.youtube_api_key = key;
            }
            
            Message::UpdateSoundcloudClientId(id) => {
                self.soundcloud_client_id = id;
            }
            
            Message::UpdateGeniusApiKey(key) => {
                self.genius_api_key = key;
            }
            
            Message::UpdateMusixmatchApiKey(key) => {
                self.musixmatch_api_key = key;
            }
            
            Message::ToggleTheme => {
                self.theme = match self.theme {
                    Theme::Light => Theme::Dark,
                    Theme::Dark => Theme::Light,
                    _ => Theme::Light,
                };
            }
            
            Message::SetTheme(theme) => {
                self.theme = theme;
            }
            
            Message::SetAccentColor(color) => {
                self.accent_color = color;
            }
            
            // Cookies Settings
            Message::ToggleUseCookies(enabled) => {
                println!("[UI] ToggleUseCookies: {}", enabled);
                self.use_cookies = enabled;
                if let Err(e) = self.settings.toggle_cookies() {
                    eprintln!("Failed to save cookie settings: {}", e);
                }
            }
            
            Message::SelectBrowser(browser) => {
                println!("[UI] SelectBrowser: {}", browser);
                self.selected_browser = browser.clone();
                if let Err(e) = self.settings.set_selected_browser(browser) {
                    eprintln!("Failed to save browser selection: {}", e);
                }
            }
            
            Message::TestCookieImport => {
                println!("[UI] TestCookieImport pressed");
                self.cookie_import_status = Some("Testing cookie import...".to_string());
                
                // Test actual cookie import
                let config = self.settings.config().clone();
                let browsers = config.cookies_config.browsers.clone();
                
                return Command::perform(
                    async move {
                        Self::test_cookie_import_static(&browsers).await
                    },
                    |result| {
                        match result {
                            Ok(found_cookies) => {
                                if found_cookies > 0 {
                                    Message::CookieTestResult(Ok(format!("✅ Found YouTube cookies from {} browser(s)!", found_cookies)))
                                } else {
                                    Message::CookieTestResult(Ok("⚠️ No YouTube cookies found in configured browsers".to_string()))
                                }
                            }
                            Err(e) => {
                                Message::CookieTestResult(Err(format!("❌ YouTube cookie test failed: {}", e)))
                            }
                        }
                    }
                );
            }
            
            Message::ToggleSponsorBlock(enabled) => {
                println!("[UI] ToggleSponsorBlock: {}", enabled);
                self.enable_sponsorblock = enabled;
                if let Err(e) = self.settings.toggle_sponsorblock() {
                    eprintln!("Failed to save SponsorBlock settings: {}", e);
                }
            }
            
            Message::ToggleSponsorBlockCategory(category) => {
                println!("[UI] ToggleSponsorBlockCategory: {}", category);
                if self.sponsorblock_categories.contains(&category) {
                    self.sponsorblock_categories.retain(|c| c != &category);
                } else {
                    self.sponsorblock_categories.push(category.clone());
                }
                if let Err(e) = self.settings.toggle_sponsorblock_category(category) {
                    eprintln!("Failed to save SponsorBlock category: {}", e);
                }
            }
            
            Message::ToggleCookies => {
                println!("[UI] ToggleCookies pressed");
                self.use_cookies = !self.use_cookies;
                if let Err(e) = self.settings.toggle_cookies() {
                    eprintln!("Failed to save cookie settings: {}", e);
                }
            }
            
            Message::ToggleBrowserSelection(browser) => {
                self.selected_browser = browser;
            }
            
            Message::CookieTestResult(result) => {
                match result {
                    Ok(message) => {
                        self.cookie_import_status = Some(message);
                        println!("[UI] Cookie test completed successfully");
                    }
                    Err(e) => {
                        self.cookie_import_status = Some(e);
                        println!("[UI] Cookie test failed");
                    }
                }
            }
            
            Message::CsvImportProgress(progress) => {
                self.csv_import_progress = Some(progress);
            }
            
            Message::CsvImportStatus(status) => {
                self.csv_import_status = Some(status);
            }
            
            
            Message::CommandReceived(command) => {
                println!("CommandReceived message: {:?}", command);
                match command {
                    AppCommand::AddTrack(metadata) => {
                        println!("Adding track to list: {} - {}", metadata.artist, metadata.title);
                        self.tracks.push(TrackItem {
                            metadata,
                            status: TrackStatus::Pending,
                            progress: 0.0,
                            error_message: None,
                            current_stage: None,
                            stage_message: None,
                        });
                        println!("Track added successfully. Total tracks: {}", self.tracks.len());
                    }
                    AppCommand::AddMultipleTracks(tracks) => {
                        for metadata in tracks {
                            self.tracks.push(TrackItem {
                                metadata,
                                status: TrackStatus::Pending,
                                progress: 0.0,
                                error_message: None,
                                current_stage: None,
                                stage_message: None,
                            });
                        }
                    }
                    AppCommand::UpdateProgress(track_id, progress) => {
                        if let Some(track) = self.tracks.iter_mut().find(|t| t.metadata.id == track_id) {
                            track.progress = progress;
                        }
                    }
                    AppCommand::UpdateStatus(track_id, status) => {
                        if let Some(track) = self.tracks.iter_mut().find(|t| t.metadata.id == track_id) {
                            track.current_stage = Some(status.clone());
                            track.status = match status {
                                DownloadStage::Completed => TrackStatus::Completed,
                                DownloadStage::Error => TrackStatus::Failed,
                                _ => TrackStatus::Downloading,
                            };
                            // Update stage message based on the stage
                            track.stage_message = Some(match status {
                                DownloadStage::SearchingSource => "Searching for audio source...".to_string(),
                                DownloadStage::DownloadingAudio => "Downloading audio...".to_string(),
                                DownloadStage::ConvertingAudio => "Converting audio format...".to_string(),
                                DownloadStage::DownloadingCover => "Downloading cover art and lyrics...".to_string(),
                                DownloadStage::EmbeddingMetadata => "Embedding metadata...".to_string(),
                                DownloadStage::Completed => "Download completed successfully!".to_string(),
                                DownloadStage::Error => "Download failed".to_string(),
                                DownloadStage::Queued => "Queued for download...".to_string(),
                                DownloadStage::FetchingMetadata => "Fetching metadata...".to_string(),
                                DownloadStage::DownloadingLyrics => "Downloading lyrics...".to_string(),
                            });
                        }
                    }
                    AppCommand::SetError(track_id, error) => {
                        if let Some(track) = self.tracks.iter_mut().find(|t| t.metadata.id == track_id) {
                            track.error_message = Some(error);
                            track.status = TrackStatus::Failed;
                        }
                    }
                    AppCommand::CompleteDownload(track_id) => {
                        if let Some(track) = self.tracks.iter_mut().find(|t| t.metadata.id == track_id) {
                            track.status = TrackStatus::Completed;
                            track.progress = 1.0;
                            track.current_stage = Some(DownloadStage::Completed);
                            track.stage_message = Some("Download completed successfully!".to_string());
                        }
                    }
                    AppCommand::BatchDownloadComplete(results) => {
                        for result in results {
                            if let Some(track) = self.tracks.iter_mut().find(|t| t.metadata.id == result.track.id) {
                                if result.success {
                                    track.status = TrackStatus::Completed;
                                    track.progress = 1.0;
                                    track.current_stage = Some(DownloadStage::Completed);
                                    track.stage_message = Some("Download completed successfully!".to_string());
                                } else {
                                    track.status = TrackStatus::Failed;
                                    track.error_message = result.error;
                                    track.current_stage = Some(DownloadStage::Error);
                                    track.stage_message = Some("Download failed".to_string());
                                }
                            }
                        }
                    }
                }
            }
            
            Message::KeyPressed(key) => {
                match key.as_str() {
                    "Ctrl+a" | "Cmd+a" => {
                        self.selected_tracks = (0..self.tracks.len()).collect();
                    }
                    "Escape" => {
                        self.selected_tracks.clear();
                    }
                    "Delete" | "Backspace" => {
                        if !self.selected_tracks.is_empty() {
                            // Delete selected tracks
                            let mut indices = self.selected_tracks.clone();
                            indices.sort_by(|a, b| b.cmp(a));
                            for &index in &indices {
                                if index < self.tracks.len() {
                                    self.tracks.remove(index);
                                }
                            }
                            self.selected_tracks.clear();
                        }
                    }
                    "Space" => {
                        if !self.tracks.is_empty() {
                            // Toggle play/pause for first selected track or first track
                            let track_index = if !self.selected_tracks.is_empty() {
                                self.selected_tracks[0]
                            } else {
                                0
                            };
                            if track_index < self.tracks.len() {
                                match self.tracks[track_index].status {
                                    TrackStatus::Paused => {
                                        self.tracks[track_index].status = TrackStatus::Downloading;
                                    }
                                    TrackStatus::Downloading => {
                                        self.tracks[track_index].status = TrackStatus::Paused;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            Message::FileDropped(files) => {
                // Handle dropped files - for now, just process CSV files
                for file in files {
                    if let Some(extension) = file.extension() {
                        if extension == "csv" {
                            self.csv_path = Some(file);
                            // Process the CSV file
                            if let Some(ref csv_path) = self.csv_path {
                                let csv_importer = CsvImporter::new();
                                match csv_importer.get_csv_info(csv_path) {
                                    Ok(info) => {
                                        self.csv_info = Some(info);
                                    }
                                    Err(e) => {
                                        self.url_validation_error = Some(format!("CSV Error: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            Message::DragEntered => {
                self.is_drag_over = true;
            }
            
            Message::DragExited => {
                self.is_drag_over = false;
            }
            
            
            
        }
        
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match self.current_view {
            View::Import => self.import_view(),
            View::TrackList => self.track_list_view(),
            View::Settings => self.settings_view(),
        };

        let sidebar = self.sidebar();

        row![
            sidebar,
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(16)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

impl SpotifyDownloaderApp {

    fn sidebar(&self) -> Element<'_, Message> {
        let import_button = button("Import")
            .on_press(Message::SwitchView(View::Import))
            .padding([12, 20])
            .width(Length::Fill)
            .style(if matches!(self.current_view, View::Import) {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            });

        let tracks_button = button("Tracks")
            .on_press(Message::SwitchView(View::TrackList))
            .padding([12, 20])
            .width(Length::Fill)
            .style(if matches!(self.current_view, View::TrackList) {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            });

        let settings_button = button("Settings")
            .on_press(Message::SwitchView(View::Settings))
            .padding([12, 20])
            .width(Length::Fill)
            .style(if matches!(self.current_view, View::Settings) {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            });

        column![
            text("Navigation")
                .size(18)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(16),
            import_button,
            vertical_space().height(12),
            tracks_button,
            vertical_space().height(12),
            settings_button,
        ]
        .padding(24)
        .width(Length::Fixed(200.0))
        .spacing(8)
        .into()
    }

    fn import_view(&self) -> Element<'_, Message> {
        let url_section = self.url_import_section();
        let csv_section = self.csv_import_section();

        let content = column![
            text("Import Music")
                .size(32)
                .style(Color::from_rgb(0.2, 0.2, 0.2)),
            vertical_space().height(24),
            url_section,
            vertical_space().height(32),
            csv_section,
        ]
        .spacing(24)
        .padding(32);

        // Add drag & drop visual feedback
        if self.is_drag_over {
            scrollable(
                container(content)
                    .padding(24)
                    .style(iced::theme::Container::Custom(Box::new(DragOverStyle)))
            )
            .height(Length::Fill)
            .into()
        } else {
            scrollable(content)
                .height(Length::Fill)
                .into()
        }
    }

    fn url_import_section(&self) -> Element<'_, Message> {
        let url_input = text_input("Enter Spotify URL (track, album, or playlist)", &self.url_input)
            .on_input(Message::UrlInputChanged)
            .on_submit(Message::ImportUrl)
            .width(Length::Fill);

        let import_button = button("Import URL")
            .on_press(Message::ImportUrl);

        let error_text = if let Some(ref error) = self.url_validation_error {
            text(error)
                .size(14)
                .style(Color::from_rgb(0.8, 0.2, 0.2))
        } else {
            text("")
        };

        column![
            text("Single URL Import")
                .size(22)
                .style(Color::from_rgb(0.2, 0.2, 0.2)),
            vertical_space().height(16),
            row![
                url_input,
                horizontal_space().width(16),
                import_button,
            ]
            .align_items(Alignment::Center),
            vertical_space().height(12),
            error_text,
        ]
        .spacing(5)
        .into()
    }

    fn csv_import_section(&self) -> Element<'_, Message> {
        let file_button = button("Select CSV File")
            .on_press(Message::SelectCsvFile);

        let import_button = button("Import CSV")
            .on_press(Message::ImportCsv);

        let csv_info = if let Some(ref info) = self.csv_info {
            column![
                text(format!("File: {}", info.file_path.file_name().unwrap_or_default().to_string_lossy()))
                    .size(14),
                text(format!("Records: {}", info.record_count))
                    .size(14),
                text(format!("Columns: {}", info.column_count))
                    .size(14),
            ]
        } else {
            column![text("No file selected").size(14)]
        };

        column![
            text("CSV Import")
                .size(20)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            row![
                file_button,
                horizontal_space().width(10),
                import_button,
            ],
            vertical_space().height(10),
            csv_info,
        ]
        .spacing(5)
        .into()
    }


    fn track_list_view(&self) -> Element<'_, Message> {
        let header = self.track_list_header();
        let track_list = self.track_list();

        scrollable(
            column![
                header,
                vertical_space().height(20),
                track_list,
            ]
            .spacing(10)
        )
        .height(Length::Fill)
        .into()
    }

    fn track_list_header(&self) -> Element<'_, Message> {
        // Main action buttons - only essential ones
        let download_all_button = button("Download All")
            .on_press(Message::StartDownload)
            .padding([10, 16])
            .style(iced::theme::Button::Primary);

        let download_selected_button = button("Download Selected")
            .on_press(Message::DownloadSelectedTracks)
            .padding([10, 16])
            .style(iced::theme::Button::Primary);

        let pause_button = button("Pause/Resume")
            .on_press(Message::PauseDownload)
            .padding([10, 16])
            .style(iced::theme::Button::Secondary);

        let stop_button = button("Stop")
            .on_press(Message::StopDownload)
            .padding([10, 16])
            .style(iced::theme::Button::Destructive);

        let clear_list_button = button("Clear List")
            .on_press(Message::ClearAllTracks)
            .padding([10, 16])
            .style(iced::theme::Button::Destructive);

        row![
            text(format!("Tracks ({})", self.tracks.len()))
                .size(24)
                .style(Color::from_rgb(0.2, 0.2, 0.2)),
            horizontal_space(),
            download_all_button,
            horizontal_space().width(8),
            download_selected_button,
            horizontal_space().width(8),
            pause_button,
            horizontal_space().width(8),
            stop_button,
            horizontal_space().width(8),
            clear_list_button,
        ]
        .align_items(Alignment::Center)
        .into()
    }

    fn track_list(&self) -> Element<'_, Message> {
        if self.tracks.is_empty() {
            return column![
                text("No tracks imported yet")
                    .size(18)
                    .style(Color::from_rgb(0.5, 0.5, 0.5)),
                text("Go to the Import tab to add tracks")
                    .size(14)
                    .style(Color::from_rgb(0.6, 0.6, 0.6)),
            ]
            .align_items(Alignment::Center)
            .spacing(10)
            .into();
        }

        let track_items: Vec<Element<Message>> = self.tracks
            .iter()
            .enumerate()
            .map(|(index, track)| self.track_item(index, track))
            .collect();

        column(track_items)
            .spacing(10)
            .into()
    }

    fn track_item(&self, index: usize, track: &TrackItem) -> Element<'_, Message> {
        let is_selected = self.selected_tracks.contains(&index);
        
        let status_color = match track.status {
            TrackStatus::Pending => Color::from_rgb(0.5, 0.5, 0.5),
            TrackStatus::Downloading => {
                if let Some(stage) = &track.current_stage {
                    match stage {
                        DownloadStage::Queued => Color::from_rgb(0.9, 0.6, 0.2), // Orange for queued
                        _ => Color::from_rgb(0.2, 0.6, 0.9), // Blue for active downloading
                    }
                } else {
                    Color::from_rgb(0.2, 0.6, 0.9)
                }
            },
            TrackStatus::Completed => Color::from_rgb(0.2, 0.8, 0.2),
            TrackStatus::Failed => Color::from_rgb(0.8, 0.2, 0.2),
            TrackStatus::Paused => Color::from_rgb(0.9, 0.6, 0.2),
        };

        let status_text = match track.status {
            TrackStatus::Pending => "Pending",
            TrackStatus::Downloading => {
                if let Some(stage) = &track.current_stage {
                    match stage {
                        DownloadStage::Queued => "Queued",
                        DownloadStage::FetchingMetadata => "Fetching Metadata",
                        DownloadStage::SearchingSource => "Searching Source",
                        DownloadStage::DownloadingAudio => "Downloading Audio",
                        DownloadStage::ConvertingAudio => "Converting Audio",
                        DownloadStage::DownloadingCover => "Downloading Cover",
                        DownloadStage::DownloadingLyrics => "Downloading Lyrics",
                        DownloadStage::EmbeddingMetadata => "Embedding Metadata",
                        DownloadStage::Completed => "Completed",
                        DownloadStage::Error => "Error",
                    }
                } else {
                    "Downloading"
                }
            },
            TrackStatus::Completed => "Completed",
            TrackStatus::Failed => "Failed",
            TrackStatus::Paused => "Paused",
        };

        let download_button = if matches!(track.status, TrackStatus::Downloading) {
            button("Downloading...")
                .on_press(Message::DownloadTrack(index))
                .style(iced::theme::Button::Custom(Box::new(HoverButtonStyle)))
        } else if matches!(track.status, TrackStatus::Completed) {
            button("Download")
                .on_press(Message::DownloadTrack(index))
                .style(iced::theme::Button::Custom(Box::new(CompletedButtonStyle)))
        } else {
            button("Download")
                .on_press(Message::DownloadTrack(index))
                .style(iced::theme::Button::Custom(Box::new(HoverButtonStyle)))
        };

        let retry_button = button("Retry")
            .on_press(Message::RetryTrack(index))
            .style(iced::theme::Button::Secondary);

        let delete_button = button("Delete")
            .on_press(Message::DeleteTrack(index))
            .style(iced::theme::Button::Destructive);

        let progress_bar = if matches!(track.status, TrackStatus::Downloading) {
            progress_bar(0.0..=1.0, track.progress)
                .width(Length::Fill)
                .style(iced::theme::ProgressBar::Custom(Box::new(ProgressBarStyle)))
        } else if matches!(track.status, TrackStatus::Completed) {
            progress_bar(0.0..=1.0, 1.0)
                .width(Length::Fill)
                .style(iced::theme::ProgressBar::Custom(Box::new(CompletedProgressBarStyle)))
        } else {
            progress_bar(0.0..=1.0, 0.0)
                .width(Length::Fill)
        };

        let error_text = if let Some(ref error) = track.error_message {
            column![
                text("Error:")
                    .size(12)
                    .style(Color::from_rgb(0.8, 0.2, 0.2)),
                text(error)
                    .size(11)
                    .style(Color::from_rgb(0.8, 0.2, 0.2))
                    .width(Length::Fill),
                text("Click 'Retry' to try again or 'Delete' to remove")
                    .size(10)
                    .style(Color::from_rgb(0.6, 0.6, 0.6)),
            ]
            .spacing(2)
        } else {
            column![text("").size(12)]
        };

        button(
            container(
                column![
                    row![
                        text(format!("#{}", index + 1))
                            .size(14)
                            .style(Color::from_rgb(0.6, 0.6, 0.6)),
                        horizontal_space().width(10),
                        text(&track.metadata.title)
                            .size(16)
                            .style(Color::from_rgb(0.2, 0.2, 0.2)),
                        horizontal_space(),
                        text(status_text)
                            .size(12)
                            .style(status_color),
                    ],
                    vertical_space().height(5),
                    text(&track.metadata.artist)
                        .size(14)
                        .style(Color::from_rgb(0.4, 0.4, 0.4)),
                    vertical_space().height(5),
                    progress_bar,
                    vertical_space().height(5),
                    // Show stage message if available
                    if let Some(ref message) = track.stage_message {
                        text(message)
                            .size(11)
                            .style(Color::from_rgb(0.4, 0.4, 0.4))
                    } else {
                        text("")
                            .size(11)
                    },
                    vertical_space().height(5),
                    row![
                        download_button,
                        horizontal_space().width(8),
                        retry_button,
                        horizontal_space().width(8),
                        delete_button,
                        horizontal_space(),
                        text(format!("{:.1}%", track.progress * 100.0))
                            .size(12)
                            .style(Color::from_rgb(0.5, 0.5, 0.5)),
                    ],
                    error_text,
                ]
                .spacing(5)
            )
            .padding(15)
            .style(if is_selected {
                iced::theme::Container::Custom(Box::new(SelectedTrackStyle))
            } else {
                iced::theme::Container::Custom(Box::new(TrackStyle))
            })
        )
        .on_press(Message::SelectTrack(index))
        .into()
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let tabs = self.settings_tabs();
        let content = self.settings_content();

        let settings_message = if let Some(ref message) = self.settings_message {
            text(message)
                .size(14)
                .style(if message.contains("Error") {
                    Color::from_rgb(0.8, 0.2, 0.2)
                } else {
                    Color::from_rgb(0.2, 0.8, 0.2)
                })
        } else {
            text("")
        };

        scrollable(
            column![
                text("Settings")
                    .size(28)
                    .style(Color::from_rgb(0.2, 0.2, 0.2)),
                vertical_space().height(20),
                tabs,
                vertical_space().height(20),
                content,
                vertical_space().height(30),
                settings_message,
                vertical_space().height(10),
                row![
                    button("Save Settings")
                        .on_press(Message::SaveSettings),
                    horizontal_space(),
                    button("Reset to Defaults")
                        .on_press(Message::ResetSettings),
                ],
            ]
            .spacing(10)
        )
        .height(Length::Fill)
        .into()
    }

    fn output_settings_section(&self) -> Element<'_, Message> {
        let output_input = text_input("Output Directory", &self.output_directory)
            .on_input(Message::UpdateOutputDirectory)
            .width(Length::Fill);

        let browse_button = button("Browse")
            .on_press(Message::SelectOutputDirectory);

        column![
            text("Output Settings")
                .size(20)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            row![
                output_input,
                horizontal_space().width(10),
                browse_button,
            ],
        ]
        .spacing(5)
        .into()
    }

    fn format_settings_section(&self) -> Element<'_, Message> {
        let formats = [AudioFormat::Mp3, AudioFormat::M4a, AudioFormat::Flac, AudioFormat::Wav];
        let format_picklist = pick_list(
            formats,
            Some(self.selected_format),
            Message::UpdateFormat,
        )
        .width(Length::Fixed(150.0));

        let is_lossless = matches!(self.selected_format, AudioFormat::Flac | AudioFormat::Wav);
        
        let bitrate_section = if is_lossless {
            column![
                text("Bitrate: Not applicable for lossless formats")
                    .size(14)
                    .style(Color::from_rgb(0.6, 0.6, 0.6)),
            ]
        } else {
            let bitrates = [Bitrate::Kbps128, Bitrate::Kbps192, Bitrate::Kbps256, Bitrate::Kbps320];
            let bitrate_picklist = pick_list(
                bitrates,
                Some(self.selected_bitrate),
                Message::UpdateBitrate,
            )
            .width(Length::Fixed(150.0));
            
            column![
                text("Bitrate:").size(14),
                horizontal_space().width(10),
                bitrate_picklist,
            ]
            .align_items(Alignment::Center)
        };

        // Concurrency options
        let concurrency_options = [1, 2, 3, 4, 5, 6, 8, 10, 12, 16];
        let concurrency_picklist = pick_list(
            concurrency_options,
            Some(self.max_concurrent_downloads),
            Message::UpdateConcurrency,
        )
        .width(Length::Fixed(100.0));

        column![
            text("Audio Format")
                .size(20)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            row![
                text("Format:").size(14),
                horizontal_space().width(10),
                format_picklist,
                horizontal_space().width(20),
                bitrate_section,
            ]
            .align_items(Alignment::Center),
            vertical_space().height(15),
            row![
                text("Max Concurrent Downloads:").size(14),
                horizontal_space().width(10),
                concurrency_picklist,
            ]
            .align_items(Alignment::Center),
        ]
        .spacing(5)
        .into()
    }

    fn metadata_settings_section(&self) -> Element<'_, Message> {
        let lyrics_toggle = button(if self.download_lyrics { "ON" } else { "OFF" })
            .on_press(Message::ToggleLyrics(!self.download_lyrics));

        let cover_toggle = button(if self.download_cover { "ON" } else { "OFF" })
            .on_press(Message::ToggleCover(!self.download_cover));

        let metadata_toggle = button(if self.embed_metadata { "ON" } else { "OFF" })
            .on_press(Message::ToggleMetadata(!self.embed_metadata));

        let basic_metadata = column![
            text("Basic Metadata")
                .size(18)
                .style(Color::from_rgb(0.2, 0.2, 0.2)),
            vertical_space().height(10),
            self.create_metadata_toggle("Title", self.embed_title, Message::ToggleTitle),
            self.create_metadata_toggle("Artist", self.embed_artist, Message::ToggleArtist),
            self.create_metadata_toggle("Album", self.embed_album, Message::ToggleAlbum),
            self.create_metadata_toggle("Year", self.embed_year, Message::ToggleYear),
            self.create_metadata_toggle("Genre", self.embed_genre, Message::ToggleGenre),
        ]
        .spacing(8);

        let advanced_metadata = column![
            text("Advanced Metadata")
                .size(18)
                .style(Color::from_rgb(0.2, 0.2, 0.2)),
            vertical_space().height(10),
            self.create_metadata_toggle("Track Number", self.embed_track_number, Message::ToggleTrackNumber),
            self.create_metadata_toggle("Disc Number", self.embed_disc_number, Message::ToggleDiscNumber),
            self.create_metadata_toggle("Album Artist", self.embed_album_artist, Message::ToggleAlbumArtist),
            self.create_metadata_toggle("Composer", self.embed_composer, Message::ToggleComposer),
            self.create_metadata_toggle("Comment", self.embed_comment, Message::ToggleComment),
        ]
        .spacing(8);

        column![
            text("Metadata Settings")
                .size(24)
                .style(Color::from_rgb(0.2, 0.2, 0.2)),
            vertical_space().height(20),
            row![
                text("Download Lyrics:").size(14),
                horizontal_space().width(10),
                lyrics_toggle,
                horizontal_space().width(20),
                text("Download Cover:").size(14),
                horizontal_space().width(10),
                cover_toggle,
                horizontal_space().width(20),
                text("Embed Metadata:").size(14),
                horizontal_space().width(10),
                metadata_toggle,
            ]
            .align_items(Alignment::Center),
            vertical_space().height(20),
            basic_metadata,
            vertical_space().height(20),
            advanced_metadata,
        ]
        .spacing(10)
        .into()
    }

    fn api_settings_section(&self) -> Element<'_, Message> {
        let spotify_section = self.spotify_api_section();
        let lastfm_section = self.lastfm_api_section();
        let youtube_section = self.youtube_api_section();
        let soundcloud_section = self.soundcloud_api_section();
        let genius_section = self.genius_api_section();
        let musixmatch_section = self.musixmatch_api_section();

        column![
            text("API Settings")
                .size(24)
                .style(Color::from_rgb(0.2, 0.2, 0.2)),
            vertical_space().height(20),
            spotify_section,
            vertical_space().height(20),
            lastfm_section,
            vertical_space().height(20),
            youtube_section,
            vertical_space().height(20),
            soundcloud_section,
            vertical_space().height(20),
            genius_section,
            vertical_space().height(20),
            musixmatch_section,
        ]
        .spacing(10)
        .into()
    }

    fn advanced_settings_section(&self) -> Element<'_, Message> {
        let cookies_section = self.cookies_settings_section();
        let sponsorblock_section = self.sponsorblock_settings_section();

        column![
            text("Advanced Settings")
                .size(24)
                .style(Color::from_rgb(0.2, 0.2, 0.2)),
            vertical_space().height(20),
            cookies_section,
            vertical_space().height(20),
            sponsorblock_section,
        ]
        .spacing(10)
        .into()
    }

    fn cookies_settings_section(&self) -> Element<'_, Message> {
        let config = self.settings.config();
        let cookies_enabled = config.cookies_config.enabled;
        let available_browsers = vec![
            "firefox", "chrome", "chromium", "edge", "safari"
        ];
        
        let browser_buttons: Vec<Element<Message>> = available_browsers
            .iter()
            .map(|browser| {
                let is_selected = config.cookies_config.browsers.contains(&browser.to_string());
                let button_style = if is_selected {
                    iced::theme::Button::Primary
                } else {
                    iced::theme::Button::Secondary
                };
                
                button(text(browser))
                    .style(button_style)
                    .on_press(Message::ToggleBrowserSelection(browser.to_string()))
                    .into()
            })
            .collect();

        let test_button = button(text("Test Cookie Import"))
            .style(iced::theme::Button::Secondary)
            .on_press(Message::TestCookieImport);

        column![
            text("YouTube Cookie Settings")
                .size(18)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            row![
                button(if cookies_enabled { "[ENABLED] Enable YouTube Cookies" } else { "[DISABLED] Enable YouTube Cookies" })
                    .on_press(Message::ToggleCookies)
                    .style(if cookies_enabled { iced::theme::Button::Primary } else { iced::theme::Button::Secondary }),
            ],
            vertical_space().height(10),
            text("Select Browsers (YouTube cookies only):")
                .size(14)
                .style(Color::from_rgb(0.4, 0.4, 0.4)),
            vertical_space().height(5),
            row(browser_buttons).spacing(5),
            vertical_space().height(10),
            test_button,
        ]
        .spacing(5)
        .into()
    }

    fn sponsorblock_settings_section(&self) -> Element<'_, Message> {
        let config = self.settings.config();
        let sponsorblock_enabled = config.sponsorblock_config.enabled;
        let available_categories = vec![
            "sponsor", "intro", "outro", "preview", "interaction", 
            "selfpromo", "music_offtopic"
        ];
        
        let category_buttons: Vec<Element<Message>> = available_categories
            .iter()
            .map(|category| {
                let is_selected = config.sponsorblock_config.remove_categories.contains(&category.to_string());
                let button_style = if is_selected {
                    iced::theme::Button::Destructive
                } else {
                    iced::theme::Button::Secondary
                };
                
                button(text(category))
                    .style(button_style)
                    .on_press(Message::ToggleSponsorBlockCategory(category.to_string()))
                    .into()
            })
            .collect();

        column![
            text("SponsorBlock Settings")
                .size(18)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            row![
                button(if sponsorblock_enabled { "[ENABLED] Enable SponsorBlock" } else { "[DISABLED] Enable SponsorBlock" })
                    .on_press(Message::ToggleSponsorBlock(!sponsorblock_enabled))
                    .style(if sponsorblock_enabled { iced::theme::Button::Primary } else { iced::theme::Button::Secondary }),
            ],
            vertical_space().height(10),
            text("Remove Categories:")
                .size(14)
                .style(Color::from_rgb(0.4, 0.4, 0.4)),
            vertical_space().height(5),
            row(category_buttons).spacing(5),
        ]
        .spacing(5)
        .into()
    }

    fn settings_tabs(&self) -> Element<'_, Message> {
        let tabs = [
            ("General", SettingsTab::General),
            ("Audio", SettingsTab::Audio),
            ("Metadata", SettingsTab::Metadata),
            ("API", SettingsTab::API),
            ("Advanced", SettingsTab::Advanced),
        ];

        let tab_buttons: Vec<Element<Message>> = tabs
            .iter()
            .map(|(label, tab)| {
                let is_active = self.current_settings_tab == *tab;
                let button_style = if is_active {
                    Color::from_rgb(0.2, 0.5, 0.8)
                } else {
                    Color::from_rgb(0.7, 0.7, 0.7)
                };
                
                button(text(label).style(button_style))
                    .on_press(Message::SwitchSettingsTab(tab.clone()))
                    .padding(8)
                    .into()
            })
            .collect();

        row(tab_buttons)
        .spacing(5)
        .into()
    }

    fn settings_content(&self) -> Element<'_, Message> {
        match self.current_settings_tab {
            SettingsTab::General => self.general_settings_section(),
            SettingsTab::Audio => self.audio_settings_section(),
            SettingsTab::Metadata => self.metadata_settings_section(),
            SettingsTab::API => self.api_settings_section(),
            SettingsTab::Advanced => self.advanced_settings_section(),
        }
    }

    fn general_settings_section(&self) -> Element<'_, Message> {
        let output_section = self.output_settings_section();
        let theme_section = self.theme_settings_section();

        column![
            output_section,
            vertical_space().height(24),
            theme_section,
        ]
        .spacing(16)
        .into()
    }

    fn audio_settings_section(&self) -> Element<'_, Message> {
        self.format_settings_section()
    }

    fn spotify_settings_section(&self) -> Element<'_, Message> {
        column![
            text("Spotify API Settings")
                .size(20)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            text_input("Client ID", &self.spotify_client_id)
                .on_input(Message::UpdateSpotifyClientId)
                .width(Length::Fill),
            vertical_space().height(10),
            text_input("Client Secret", &self.spotify_client_secret)
                .on_input(Message::UpdateSpotifyClientSecret)
                .width(Length::Fill),
            vertical_space().height(10),
            text("Get your API credentials from https://developer.spotify.com/dashboard")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn lastfm_settings_section(&self) -> Element<'_, Message> {
        column![
            text("Last.fm API Settings")
                .size(20)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            text_input("API Key", &self.lastfm_api_key)
                .on_input(Message::UpdateLastfmApiKey)
                .width(Length::Fill),
            vertical_space().height(10),
            text("Get your API key from https://www.last.fm/api/account/create")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn youtube_settings_section(&self) -> Element<'_, Message> {
        column![
            text("YouTube API Settings")
                .size(20)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            text_input("API Key", &self.youtube_api_key)
                .on_input(Message::UpdateYoutubeApiKey)
                .width(Length::Fill),
            vertical_space().height(10),
            text("Get your API key from https://console.developers.google.com/")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn soundcloud_settings_section(&self) -> Element<'_, Message> {
        column![
            text("SoundCloud API Settings")
                .size(20)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            text_input("Client ID", &self.soundcloud_client_id)
                .on_input(Message::UpdateSoundcloudClientId)
                .width(Length::Fill),
            vertical_space().height(10),
            text("Get your client ID from https://developers.soundcloud.com/")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn genius_settings_section(&self) -> Element<'_, Message> {
        column![
            text("Genius API Settings")
                .size(20)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            text_input("API Key", &self.genius_api_key)
                .on_input(Message::UpdateGeniusApiKey)
                .width(Length::Fill),
            vertical_space().height(10),
            text("Get your API key from https://genius.com/api-clients")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn musixmatch_settings_section(&self) -> Element<'_, Message> {
        column![
            text("Musixmatch API Settings")
                .size(20)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(10),
            text_input("API Key", &self.musixmatch_api_key)
                .on_input(Message::UpdateMusixmatchApiKey)
                .width(Length::Fill),
            vertical_space().height(10),
            text("Get your API key from https://developer.musixmatch.com/")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn spotify_api_section(&self) -> Element<'_, Message> {
        column![
            text("Spotify API")
                .size(18)
                .style(Color::from_rgb(0.2, 0.6, 0.2)),
            vertical_space().height(8),
            text_input("Client ID", &self.spotify_client_id)
                .on_input(Message::UpdateSpotifyClientId)
                .width(Length::Fill),
            vertical_space().height(8),
            text_input("Client Secret", &self.spotify_client_secret)
                .on_input(Message::UpdateSpotifyClientSecret)
                .width(Length::Fill),
            vertical_space().height(5),
            text("Get your API credentials from https://developer.spotify.com/dashboard")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn lastfm_api_section(&self) -> Element<'_, Message> {
        column![
            text("Last.fm API")
                .size(18)
                .style(Color::from_rgb(0.8, 0.2, 0.2)),
            vertical_space().height(8),
            text_input("API Key", &self.lastfm_api_key)
                .on_input(Message::UpdateLastfmApiKey)
                .width(Length::Fill),
            vertical_space().height(8),
            text_input("Client Secret", &self.lastfm_client_secret)
                .on_input(Message::UpdateLastfmClientSecret)
                .width(Length::Fill),
            vertical_space().height(5),
            text("Get your API credentials from https://www.last.fm/api/account/create")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn youtube_api_section(&self) -> Element<'_, Message> {
        column![
            text("YouTube API")
                .size(18)
                .style(Color::from_rgb(0.8, 0.1, 0.1)),
            vertical_space().height(8),
            text_input("API Key", &self.youtube_api_key)
                .on_input(Message::UpdateYoutubeApiKey)
                .width(Length::Fill),
            vertical_space().height(5),
            text("Get your API key from https://console.developers.google.com/")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn soundcloud_api_section(&self) -> Element<'_, Message> {
        column![
            text("SoundCloud API")
                .size(18)
                .style(Color::from_rgb(1.0, 0.4, 0.0)),
            vertical_space().height(8),
            text_input("Client ID", &self.soundcloud_client_id)
                .on_input(Message::UpdateSoundcloudClientId)
                .width(Length::Fill),
            vertical_space().height(5),
            text("Get your client ID from https://developers.soundcloud.com/")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn genius_api_section(&self) -> Element<'_, Message> {
        column![
            text("Genius API")
                .size(18)
                .style(Color::from_rgb(1.0, 0.8, 0.0)),
            vertical_space().height(8),
            text_input("API Key", &self.genius_api_key)
                .on_input(Message::UpdateGeniusApiKey)
                .width(Length::Fill),
            vertical_space().height(5),
            text("Get your API key from https://genius.com/api-clients")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn musixmatch_api_section(&self) -> Element<'_, Message> {
        column![
            text("Musixmatch API")
                .size(18)
                .style(Color::from_rgb(0.2, 0.4, 0.8)),
            vertical_space().height(8),
            text_input("API Key", &self.musixmatch_api_key)
                .on_input(Message::UpdateMusixmatchApiKey)
                .width(Length::Fill),
            vertical_space().height(5),
            text("Get your API key from https://developer.musixmatch.com/")
                .size(12)
                .style(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(5)
        .into()
    }

    fn create_metadata_toggle<F>(&self, label: &str, enabled: bool, message: F) -> Element<'_, Message>
    where
        F: Fn(bool) -> Message + 'static,
    {
        row![
            text(label).size(14).style(Color::from_rgb(0.3, 0.3, 0.3)),
            horizontal_space(),
            button(if enabled { "ON" } else { "OFF" })
                .on_press(message(!enabled))
                .style(if enabled {
                    iced::theme::Button::Primary
                } else {
                    iced::theme::Button::Secondary
                }),
        ]
        .align_items(Alignment::Center)
        .into()
    }

    fn theme_settings_section(&self) -> Element<'_, Message> {
        let light_theme_button = button("Light")
            .on_press(Message::SetTheme(Theme::Light))
            .style(if self.theme == Theme::Light {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            })
            .padding([10, 16]);

        let dark_theme_button = button("Dark")
            .on_press(Message::SetTheme(Theme::Dark))
            .style(if self.theme == Theme::Dark {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            })
            .padding([10, 16]);

        let color_row = row![
            text("Color buttons will be implemented later")
                .size(14)
                .style(Color::from_rgb(0.5, 0.5, 0.5))
        ];

        column![
            text("Theme Settings")
                .size(22)
                .style(Color::from_rgb(0.2, 0.2, 0.2)),
            vertical_space().height(16),
            text("Theme")
                .size(16)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(8),
            row![
                light_theme_button,
                horizontal_space().width(12),
                dark_theme_button,
            ],
            vertical_space().height(16),
            text("Accent Color")
                .size(16)
                .style(Color::from_rgb(0.3, 0.3, 0.3)),
            vertical_space().height(8),
            color_row,
        ]
        .spacing(8)
        .into()
    }

    fn validate_url(&mut self) -> bool {
        if self.url_input.is_empty() {
            self.url_validation_error = Some("URL cannot be empty".to_string());
            return false;
        }

        if !crate::utils::Utils::is_spotify_url(&self.url_input) {
            self.url_validation_error = Some("Please enter a valid Spotify URL".to_string());
            return false;
        }

        true
    }

    /// Test YouTube-specific cookie import from configured browsers (static method for async command)
    async fn test_cookie_import_static(browsers: &[String]) -> Result<usize> {
        use tokio::process::Command;
        
        let mut found_cookies = 0;
        
        for browser in browsers {
            // Test if yt-dlp can find YouTube-specific cookies from this browser
            let mut cmd = Command::new("yt-dlp");
            cmd.arg("--cookies-from-browser").arg(browser)
               .arg("--cookies-from-browser-domain").arg("youtube.com") // Only YouTube cookies
               .arg("--dump-json")
               .arg("--quiet")
               .arg("https://www.youtube.com/watch?v=dQw4w9WgXcQ"); // Test with a known video
            
            match cmd.output().await {
                Ok(output) => {
                    if output.status.success() {
                        // Check if we got actual data (not just an error)
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if !stdout.is_empty() && stdout.contains("title") {
                            found_cookies += 1;
                            println!("[Cookie Test] Found YouTube cookies in {}", browser);
                        }
                    }
                }
                Err(_) => {
                    // yt-dlp not found or browser not supported
                    continue;
                }
            }
        }
        
        Ok(found_cookies)
    }
}

// Custom styles for track items
struct TrackStyle;
struct SelectedTrackStyle;
struct ProgressBarStyle;
struct CompletedProgressBarStyle;
struct DragOverStyle;
struct CompletedButtonStyle;
struct HoverButtonStyle;

impl container::StyleSheet for TrackStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.95, 0.95, 0.95))),
            border: Border {
                width: 1.0,
                radius: 8.0.into(),
                color: Color::from_rgb(0.8, 0.8, 0.8),
            },
            shadow: Shadow::default(),
            text_color: Some(Color::from_rgb(0.2, 0.2, 0.2)),
        }
    }
}

impl container::StyleSheet for SelectedTrackStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.9, 0.95, 1.0))),
            border: Border {
                width: 2.0,
                radius: 8.0.into(),
                color: Color::from_rgb(0.2, 0.6, 0.9),
            },
            shadow: Shadow::default(),
            text_color: Some(Color::from_rgb(0.2, 0.2, 0.2)),
        }
    }
}

impl container::StyleSheet for DragOverStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.9, 0.95, 1.0))),
            border: Border {
                width: 3.0,
                radius: 12.0.into(),
                color: Color::from_rgb(0.2, 0.7, 0.2),
            },
            shadow: Shadow::default(),
            text_color: Some(Color::from_rgb(0.2, 0.2, 0.2)),
        }
    }
}


impl iced::widget::progress_bar::StyleSheet for ProgressBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::progress_bar::Appearance {
        iced::widget::progress_bar::Appearance {
            background: Background::Color(Color::from_rgb(0.9, 0.9, 0.9)),
            bar: Background::Color(Color::from_rgb(0.2, 0.7, 0.2)),
            border_radius: 8.0.into(),
        }
    }
}


impl iced::widget::progress_bar::StyleSheet for CompletedProgressBarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::progress_bar::Appearance {
        iced::widget::progress_bar::Appearance {
            background: Background::Color(Color::from_rgb(0.9, 0.9, 0.9)),
            bar: Background::Color(Color::from_rgb(0.2, 0.8, 0.2)),
            border_radius: 8.0.into(),
        }
    }
}


impl iced::widget::button::StyleSheet for CompletedButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.2, 0.8, 0.2))),
            border: Border {
                width: 1.0,
                radius: 6.0.into(),
                color: Color::from_rgb(0.1, 0.6, 0.1),
            },
            text_color: Color::from_rgb(1.0, 1.0, 1.0),
            shadow: Shadow::default(),
            shadow_offset: [0.0, 0.0].into(),
        }
    }
}

impl iced::widget::button::StyleSheet for HoverButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.2, 0.7, 0.2))),
            border: Border {
                width: 1.0,
                radius: 6.0.into(),
                color: Color::from_rgb(0.1, 0.5, 0.1),
            },
            text_color: Color::from_rgb(1.0, 1.0, 1.0),
            shadow: Shadow::default(),
            shadow_offset: [0.0, 0.0].into(),
        }
    }

    fn hovered(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(Background::Color(Color::from_rgb(0.3, 0.8, 0.3))),
            border: Border {
                width: 2.0,
                radius: 6.0.into(),
                color: Color::from_rgb(0.2, 0.6, 0.2),
            },
            text_color: Color::from_rgb(1.0, 1.0, 1.0),
            shadow: Shadow {
                color: Color::from_rgb(0.0, 0.0, 0.0),
                offset: [0.0, 2.0].into(),
                blur_radius: 4.0,
            },
            shadow_offset: [0.0, 2.0].into(),
        }
    }

}



// Async helper functions
async fn select_file() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("CSV files", &["csv"])
        .add_filter("All files", &["*"])
        .pick_file()
        .await
        .map(|file| file.path().to_path_buf())
}

async fn select_directory() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .pick_folder()
        .await
        .map(|folder| folder.path().to_path_buf())
}


/// Run the GUI application
pub fn run() -> Result<()> {
    let settings = Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1200.0, 800.0),
            min_size: Some(iced::Size::new(800.0, 600.0)),
            max_size: Some(iced::Size::new(1920.0, 1080.0)),
            resizable: true,
            decorations: true, // Enable default Windows title bar
            transparent: false,
            icon: None,
            position: iced::window::Position::default(),
            visible: true,
            level: iced::window::Level::Normal,
            platform_specific: PlatformSpecific::default(),
            exit_on_close_request: true,
        },
        id: Some("spotify-downloader".to_string()),
        flags: (),
        fonts: Vec::new(),
        default_font: iced::Font::default(),
        default_text_size: 16.0.into(),
        antialiasing: true,
    };
    
    if let Err(e) = SpotifyDownloaderApp::run(settings) {
        return Err(crate::errors::SpotifyDownloaderError::Unknown(format!("GUI Error: {}", e)));
    }

    Ok(())
}
