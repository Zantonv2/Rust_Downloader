use std::path::PathBuf;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use std::fs::File;
use std::collections::HashMap;

// For metadata writing
use id3::{Tag, TagLike, Version, frame::Picture as Id3Picture};
use lofty::{
    read_from_path, 
    file::{AudioFile, TaggedFileExt}, 
    tag::{Tag as LoftyTag, TagType, ItemKey}, 
    picture::Picture as LoftyPicture,
    config::WriteOptions
};

use crate::errors::SpotifyDownloaderError;
use crate::downloader::{DownloadOptions, TrackMetadata};
use crate::lyrics::LyricsResult;

type Result<T> = std::result::Result<T, SpotifyDownloaderError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsMetadataResult {
    pub success: bool,
    pub error: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub message: Option<String>,
}

pub struct MetadataEmbedder {
    // Symphonia doesn't need any state
}

impl MetadataEmbedder {
    /// Create a new metadata embedder
    pub fn new() -> Self {
        Self {}
    }

    /// Read metadata from an audio file using symphonia
    pub async fn read_metadata(&self, file_path: &PathBuf) -> Result<TrackMetadata> {
        let file = File::open(file_path)
            .map_err(|e| SpotifyDownloaderError::Metadata(format!("Failed to open file: {}", e)))?;
        
        let source = MediaSourceStream::new(Box::new(file), Default::default());
        let hint = Hint::new();
        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, source, &format_opts, &metadata_opts)
            .map_err(|e| SpotifyDownloaderError::Metadata(format!("Failed to probe file: {}", e)))?;

        let mut format = probed.format;
        let metadata = format.metadata();

        // Extract metadata from symphonia
        let mut title = "Unknown".to_string();
        let mut artist = "Unknown".to_string();
        let mut album = "Unknown".to_string();
        let mut album_artist = None;
        let mut track_number = None;
        let mut disc_number = None;
        let mut year = None;
        let mut genres = Vec::new();
        let mut composer = None;
        let mut comment = None;

        // Get the current metadata revision
        if let Some(current) = metadata.current() {
            for tag in current.tags() {
                match tag.std_key {
                    Some(symphonia::core::meta::StandardTagKey::TrackTitle) => {
                        title = tag.value.to_string();
                    }
                    Some(symphonia::core::meta::StandardTagKey::Artist) => {
                        artist = tag.value.to_string();
                    }
                    Some(symphonia::core::meta::StandardTagKey::Album) => {
                        album = tag.value.to_string();
                    }
                    Some(symphonia::core::meta::StandardTagKey::AlbumArtist) => {
                        album_artist = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::TrackNumber) => {
                        if let Ok(num) = tag.value.to_string().parse::<u32>() {
                            track_number = Some(num);
                        }
                    }
                    Some(symphonia::core::meta::StandardTagKey::DiscNumber) => {
                        if let Ok(num) = tag.value.to_string().parse::<u32>() {
                            disc_number = Some(num);
                        }
                    }
                    Some(symphonia::core::meta::StandardTagKey::Date) => {
                        year = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Genre) => {
                        genres.push(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Composer) => {
                        composer = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Comment) => {
                        comment = Some(tag.value.to_string());
                    }
                    _ => {}
                }
            }
        }

        // Get duration from the first track
        let mut duration_ms = 0u32;
        if let Some(track) = format.tracks().first() {
            if let Some(time_base) = track.codec_params.time_base {
                if let Some(n_frames) = track.codec_params.n_frames {
                    duration_ms = (n_frames as f64 * time_base.denom as f64 / time_base.numer as f64 * 1000.0) as u32;
                }
            }
        }

        Ok(TrackMetadata {
            id: String::new(),
            title,
            artist,
            album,
            album_artist,
            track_number,
            disc_number,
            release_date: year,
            duration_ms,
            genres,
            spotify_url: String::new(),
            preview_url: None,
            external_urls: HashMap::new(),
            album_cover_url: None,
            composer,
            comment,
        })
    }

    /// Main metadata embedding function
    pub async fn embed_metadata(
        &self,
        file_path: &PathBuf,
        track: &TrackMetadata,
        cover_art_data: Option<&Vec<u8>>,
        lyrics_data: Option<&LyricsResult>,
        options: &DownloadOptions,
    ) -> Result<()> {
        println!("🔧 Embedding metadata for: {} - {}", track.artist, track.title);
        println!("📊 Cover art data: {}", if cover_art_data.is_some() { "Available" } else { "None" });
        println!("📊 Lyrics data: {}", if lyrics_data.is_some() { "Available" } else { "None" });
        if let Some(lyrics) = lyrics_data {
            if let Some(synced) = &lyrics.synced {
                println!("📊 Synced lyrics: {} lines", synced.lines.len());
            } else if let Some(unsynced) = &lyrics.unsynced {
                println!("📊 Unsynced lyrics: {} characters", unsynced.text.len());
            }
        }
        
        // Detect file format and use appropriate library
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        match extension.as_str() {
            "mp3" => {
                self.embed_mp3_metadata(file_path, track, cover_art_data, lyrics_data, options).await?;
            }
            "m4a" | "mp4" => {
                // Use lofty for M4A/MP4 with separate lyrics folder (like FLAC/WAV)
                self.embed_with_lyrics_folder(file_path, track, cover_art_data, lyrics_data, options).await?;
            }
            "flac" | "wav" => {
                self.embed_with_lyrics_folder(file_path, track, cover_art_data, lyrics_data, options).await?;
            }
            _ => {
                self.embed_lofty_metadata(file_path, track, cover_art_data, lyrics_data, options).await?;
            }
        }
        
        println!("✅ Metadata embedding completed");
        Ok(())
    }

    /// Embed metadata for MP3 files using id3 crate
    async fn embed_mp3_metadata(
        &self,
        file_path: &PathBuf,
        track: &TrackMetadata,
        cover_art_data: Option<&Vec<u8>>,
        lyrics_data: Option<&LyricsResult>,
        _options: &DownloadOptions,
    ) -> Result<()> {
        println!("📝 Embedding MP3 metadata using id3");
        
        // Read existing tag or create new one
        let mut tag = match Tag::read_from_path(file_path) {
            Ok(tag) => tag,
            Err(_) => Tag::new(),
        };
        
        // Set basic metadata
        if !track.title.is_empty() {
            tag.set_title(&self.format_metadata_string(&track.title));
        }
        if !track.artist.is_empty() {
            tag.set_artist(&self.format_metadata_string(&track.artist));
        }
        if !track.album.is_empty() {
            tag.set_album(&self.format_metadata_string(&track.album));
        }
        if let Some(album_artist) = &track.album_artist {
            tag.set_album_artist(&self.format_metadata_string(album_artist));
        }
        if let Some(track_num) = track.track_number {
            tag.set_track(track_num);
        }
        if let Some(disc_num) = track.disc_number {
            tag.set_disc(disc_num);
        }
        if let Some(year) = &track.release_date {
            if let Ok(year_num) = year.parse::<i32>() {
                tag.set_year(year_num);
            }
        }
        if !track.genres.is_empty() {
            let formatted_genres: Vec<String> = track.genres.iter()
                .map(|genre| self.format_metadata_string(genre))
                .collect();
            tag.set_genre(&formatted_genres.join(", "));
        }
        if let Some(composer) = &track.composer {
            tag.set_text("TCOM", &self.format_metadata_string(composer));
        }
        if let Some(comment) = &track.comment {
            tag.add_frame(id3::frame::Frame::with_content("COMM", id3::frame::Content::Comment(id3::frame::Comment {
                lang: "eng".to_string(),
                description: "".to_string(),
                text: self.format_metadata_string(comment),
            })));
        }
        
        // Add cover art first
        if let Some(cover_data) = cover_art_data {
            let picture = Id3Picture {
                mime_type: "image/jpeg".to_string(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: "Cover".to_string(),
                data: cover_data.clone(),
            };
            tag.add_frame(picture);
        }
        
        // Add lyrics - prioritize synced over unsynced
        if let Some(lyrics) = lyrics_data {
            if let Some(synced) = &lyrics.synced {
                println!("📝 Adding synced lyrics to MP3: {} lines", synced.lines.len());
                // Add synced lyrics (SYLT) - proper ID3v2.4 SYLT frame
                let sync_lyrics = id3::frame::SynchronisedLyrics {
                    lang: "eng".to_string(),
                    timestamp_format: id3::frame::TimestampFormat::Ms,
                    content_type: id3::frame::SynchronisedLyricsType::Lyrics,
                    content: synced.lines.iter()
                        .map(|line| (line.timestamp, line.text.clone()))
                        .collect(),
                    description: "Synced Lyrics".to_string(),
                };
                tag.add_frame(id3::frame::Frame::with_content("SYLT", id3::frame::Content::SynchronisedLyrics(sync_lyrics)));
                println!("✅ Synced lyrics added to MP3");
            } else if let Some(unsynced) = &lyrics.unsynced {
                println!("📝 Adding unsynced lyrics to MP3: {} characters", unsynced.text.len());
                // Add unsynced lyrics (USLT) - proper ID3v2.4 USLT frame
                let lyrics_frame = id3::frame::Lyrics {
                    lang: "eng".to_string(),
                    description: "Lyrics".to_string(),
                    text: unsynced.text.clone(),
                };
                tag.add_frame(id3::frame::Frame::with_content("USLT", id3::frame::Content::Lyrics(lyrics_frame)));
                println!("✅ Unsynced lyrics added to MP3");
            }
        } else {
            println!("⚠️ No lyrics data provided for MP3 embedding");
        }
        
        // Write the tag
        tag.write_to_path(file_path, Version::Id3v24)
            .map_err(|e| SpotifyDownloaderError::Metadata(format!("Failed to write MP3 metadata: {}", e)))?;
        
        Ok(())
    }


    /// Embed metadata for FLAC/WAV/M4A/MP4 files with separate lyrics folder
    async fn embed_with_lyrics_folder(
        &self,
        file_path: &PathBuf,
        track: &TrackMetadata,
        cover_art_data: Option<&Vec<u8>>,
        lyrics_data: Option<&LyricsResult>,
        options: &DownloadOptions,
    ) -> Result<()> {
        println!("📝 Embedding FLAC/WAV/M4A/MP4 metadata with lyrics folder");
        
        // First embed basic metadata AND lyrics using lofty
        self.embed_lofty_metadata(file_path, track, cover_art_data, lyrics_data, options).await?;
        
        // Also create lyrics folder and save LRC/TXT file if we have lyrics
        if let Some(lyrics) = lyrics_data {
            if let Some(synced) = &lyrics.synced {
                self.create_lyrics_folder_and_lrc(file_path, track, synced).await?;
            } else if let Some(unsynced) = &lyrics.unsynced {
                self.create_lyrics_folder_and_txt(file_path, track, unsynced).await?;
            }
        }
        
        Ok(())
    }

    /// Create lyrics folder and save LRC file for synced lyrics
    async fn create_lyrics_folder_and_lrc(
        &self,
        file_path: &PathBuf,
        track: &TrackMetadata,
        synced_lyrics: &crate::lyrics::SyncedLyrics,
    ) -> Result<()> {
        // Create lyrics folder path: same directory as audio file + "/lyrics"
        let mut lyrics_dir = file_path.parent().unwrap().to_path_buf();
        lyrics_dir.push("lyrics");
        
        // Ensure lyrics directory exists
        std::fs::create_dir_all(&lyrics_dir)
            .map_err(|e| SpotifyDownloaderError::Metadata(format!("Failed to create lyrics directory: {}", e)))?;
        
        // Create LRC filename: same as audio file but with .lrc extension
        let mut lrc_path = lyrics_dir;
        if let Some(stem) = file_path.file_stem() {
            lrc_path.push(format!("{}.lrc", stem.to_string_lossy()));
        } else {
            let formatted_artist = self.format_artists_for_filename(&track.artist);
            lrc_path.push(format!("{} - {}.lrc", formatted_artist, track.title));
        }
        
        // Generate LRC content
        let lrc_content = self.convert_to_lrc(synced_lyrics);
        
        // Write LRC file
        std::fs::write(&lrc_path, lrc_content)
            .map_err(|e| SpotifyDownloaderError::Metadata(format!("Failed to write LRC file: {}", e)))?;
        
        println!("📝 Created LRC file: {}", lrc_path.display());
        Ok(())
    }

    /// Create lyrics folder and save TXT file for unsynced lyrics
    async fn create_lyrics_folder_and_txt(
        &self,
        file_path: &PathBuf,
        track: &TrackMetadata,
        unsynced_lyrics: &crate::lyrics::UnsyncedLyrics,
    ) -> Result<()> {
        // Create lyrics folder path: same directory as audio file + "/lyrics"
        let mut lyrics_dir = file_path.parent().unwrap().to_path_buf();
        lyrics_dir.push("lyrics");
        
        // Ensure lyrics directory exists
        std::fs::create_dir_all(&lyrics_dir)
            .map_err(|e| SpotifyDownloaderError::Metadata(format!("Failed to create lyrics directory: {}", e)))?;
        
        // Create TXT filename: same as audio file but with .txt extension
        let mut txt_path = lyrics_dir;
        if let Some(stem) = file_path.file_stem() {
            txt_path.push(format!("{}.txt", stem.to_string_lossy()));
        } else {
            let formatted_artist = self.format_artists_for_filename(&track.artist);
            txt_path.push(format!("{} - {}.txt", formatted_artist, track.title));
        }
        
        // Write TXT file
        std::fs::write(&txt_path, &unsynced_lyrics.text)
            .map_err(|e| SpotifyDownloaderError::Metadata(format!("Failed to write TXT file: {}", e)))?;
        
        println!("📝 Created TXT file: {}", txt_path.display());
        Ok(())
    }

    /// Convert synced lyrics to LRC format
    fn convert_to_lrc(&self, lyrics: &crate::lyrics::SyncedLyrics) -> String {
        let mut lrc = String::new();

        // Add offset if present
        if lyrics.offset != 0 {
            lrc.push_str(&format!("[offset:{}]\n", lyrics.offset));
        }

        // Add lyrics lines
        for line in &lyrics.lines {
            let minutes = line.timestamp / 60000;
            let seconds = (line.timestamp % 60000) / 1000;
            let milliseconds = line.timestamp % 1000;

            lrc.push_str(&format!(
                "[{:02}:{:02}.{:02}]{}\n",
                minutes, seconds, milliseconds / 10, line.text
            ));
        }

        lrc
    }

    /// Format metadata strings by replacing semicolons with commas
    fn format_metadata_string(&self, text: &str) -> String {
        text.replace("; ", ", ")
    }

    /// Format artists for filename with proper comma separation
    fn format_artists_for_filename(&self, artist: &str) -> String {
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

    /// Embed metadata for other formats using lofty crate
    async fn embed_lofty_metadata(
        &self,
        file_path: &PathBuf,
        track: &TrackMetadata,
        cover_art_data: Option<&Vec<u8>>,
        lyrics_data: Option<&LyricsResult>,
        _options: &DownloadOptions,
    ) -> Result<()> {
        println!("📝 Embedding metadata using lofty");
        
        // Read the file
        let mut tagged_file = read_from_path(file_path)
            .map_err(|e| SpotifyDownloaderError::Metadata(format!("Failed to read file with lofty: {}", e)))?;
        
        // Determine the best tag type for this format
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        let tag_type = match extension.as_str() {
            "flac" | "ogg" => TagType::VorbisComments,
            "mp4" | "m4a" => TagType::Mp4Ilst,
            "aac" | "wav" | "aiff" | "mpc" => TagType::Id3v2, // Formats that support ID3v2
            _ => TagType::VorbisComments, // Default fallback
        };
        
        // Get or create the appropriate tag
        let tag = if let Some(tag) = tagged_file.primary_tag_mut() {
            tag
        } else {
            let new_tag = LoftyTag::new(tag_type);
            tagged_file.insert_tag(new_tag);
            tagged_file.primary_tag_mut().unwrap()
        };
        
        // Set basic metadata
        if !track.title.is_empty() {
            tag.insert_text(ItemKey::TrackTitle, self.format_metadata_string(&track.title));
        }
        if !track.artist.is_empty() {
            tag.insert_text(ItemKey::TrackArtist, self.format_metadata_string(&track.artist));
        }
        if !track.album.is_empty() {
            tag.insert_text(ItemKey::AlbumTitle, self.format_metadata_string(&track.album));
        }
        if let Some(album_artist) = &track.album_artist {
            tag.insert_text(ItemKey::AlbumArtist, self.format_metadata_string(album_artist));
        }
        if let Some(track_num) = track.track_number {
            tag.insert_text(ItemKey::TrackNumber, track_num.to_string());
        }
        if let Some(disc_num) = track.disc_number {
            tag.insert_text(ItemKey::DiscNumber, disc_num.to_string());
        }
        if let Some(year) = &track.release_date {
            tag.insert_text(ItemKey::Year, year.clone());
        }
        if !track.genres.is_empty() {
            let formatted_genres: Vec<String> = track.genres.iter()
                .map(|genre| self.format_metadata_string(genre))
                .collect();
            tag.insert_text(ItemKey::Genre, formatted_genres.join(", "));
        }
        if let Some(composer) = &track.composer {
            tag.insert_text(ItemKey::Composer, self.format_metadata_string(composer));
        }
        if let Some(comment) = &track.comment {
            tag.insert_text(ItemKey::Comment, self.format_metadata_string(comment));
        }
        
        // Add cover art
        if let Some(cover_data) = cover_art_data {
            if let Ok(picture) = LoftyPicture::from_reader(&mut cover_data.as_slice()) {
                tag.set_picture(0, picture);
            }
        }
        
        // Add lyrics - for non-MP3 formats, we'll use the generic lyrics field
        // This provides the best compatibility across different formats
        if let Some(lyrics) = lyrics_data {
            if let Some(synced) = &lyrics.synced {
                // Store synced lyrics as LRC format in the lyrics field
                // This format is widely supported and can be parsed by most players
                let synced_text = synced.lines.iter()
                    .map(|line| {
                        let minutes = line.timestamp / 60000;
                        let seconds = (line.timestamp % 60000) / 1000;
                        let centiseconds = (line.timestamp % 1000) / 10;
                        format!("[{:02}:{:02}.{:02}]{}", minutes, seconds, centiseconds, line.text)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                tag.insert_text(ItemKey::Lyrics, synced_text);
            } else if let Some(unsynced) = &lyrics.unsynced {
                // Add unsynced lyrics
                tag.insert_text(ItemKey::Lyrics, unsynced.text.clone());
            }
        }
        
        // Save the changes
        tagged_file.save_to_path(file_path, WriteOptions::default())
            .map_err(|e| SpotifyDownloaderError::Metadata(format!("Failed to save metadata with lofty: {}", e)))?;
        
        Ok(())
    }

    /// Verify metadata embedding
    pub async fn verify_metadata_embedding(
        &self,
        file_path: &PathBuf,
        expected_track: &TrackMetadata,
    ) -> Result<bool> {
        println!("🔍 Verifying metadata for: {}", file_path.display());
        
        let actual_metadata = self.read_metadata(file_path).await?;
        
        let mut all_match = true;
        
        if actual_metadata.title != expected_track.title {
            println!("❌ Title mismatch: expected '{}', got '{}'", expected_track.title, actual_metadata.title);
            all_match = false;
        }
        
        if actual_metadata.artist != expected_track.artist {
            println!("❌ Artist mismatch: expected '{}', got '{}'", expected_track.artist, actual_metadata.artist);
            all_match = false;
        }
        
        if actual_metadata.album != expected_track.album {
            println!("❌ Album mismatch: expected '{}', got '{}'", expected_track.album, actual_metadata.album);
            all_match = false;
        }
        
        if let Some(expected_album_artist) = &expected_track.album_artist {
            if let Some(actual_album_artist) = &actual_metadata.album_artist {
                if actual_album_artist != expected_album_artist {
                    println!("❌ Album Artist mismatch: expected '{}', got '{}'", expected_album_artist, actual_album_artist);
                    all_match = false;
                }
            } else {
                println!("❌ Album Artist missing: expected '{}'", expected_album_artist);
                all_match = false;
            }
        }
        
        if let Some(expected_track_num) = expected_track.track_number {
            if let Some(actual_track_num) = actual_metadata.track_number {
                if actual_track_num != expected_track_num {
                    println!("❌ Track Number mismatch: expected {}, got {}", expected_track_num, actual_track_num);
                    all_match = false;
                }
            } else {
                println!("❌ Track Number missing: expected {}", expected_track_num);
                all_match = false;
            }
        }
        
        if all_match {
            println!("✅ All metadata fields match");
        } else {
            println!("❌ Some metadata fields don't match");
        }
        
        Ok(all_match)
    }
}