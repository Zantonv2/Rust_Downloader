use crate::config::{AudioFormat, Bitrate};
use crate::errors::{Result, SpotifyDownloaderError};
use std::path::PathBuf;

/// Audio converter (placeholder - would use FFmpeg in production)
pub struct AudioConverter {
    // Configuration for audio conversion
}

impl AudioConverter {
    /// Create a new audio converter
    pub fn new() -> Self {
        Self {}
    }

    /// Convert audio file to specified format and bitrate
    pub async fn convert_audio(
        &self,
        input_path: &PathBuf,
        output_path: &PathBuf,
        format: AudioFormat,
        bitrate: Bitrate,
    ) -> Result<()> {
        println!("Converting audio from {} to {}", input_path.display(), output_path.display());
        println!("Format: {:?}, Bitrate: {} kbps", format, bitrate.as_u32());

        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| SpotifyDownloaderError::Conversion(format!("Failed to create output directory: {}", e)))?;
        }

        // First, ensure FFmpeg is available in PATH
        self.check_ffmpeg_availability()?;

        // Build FFmpeg command with optimized settings
        let mut cmd = tokio::process::Command::new("ffmpeg");
        
        // Input file
        cmd.arg("-i").arg(input_path);
        
        // Audio codec and bitrate
        let codec = match format {
            AudioFormat::Mp3 => "libmp3lame",
            AudioFormat::M4a => "aac",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "pcm_s16le",
        };
        
        cmd.arg("-acodec").arg(codec);
        cmd.arg("-b:a").arg(&format!("{}k", bitrate.as_u32()));
        
        // Optimized quality settings for speed
        match format {
            AudioFormat::Mp3 => {
                cmd.arg("-q:a").arg("2"); // High quality but faster than 0
                cmd.arg("-compression_level").arg("2"); // Faster compression
            },
            AudioFormat::M4a => {
                cmd.arg("-profile:a").arg("aac_low"); // Faster encoding
            },
            AudioFormat::Flac => {
                cmd.arg("-compression_level").arg("5"); // Balanced speed/compression
            },
            AudioFormat::Wav => {
                // No additional settings needed for WAV
            }
        }
        
        // Performance optimizations
        cmd.arg("-threads").arg("0"); // Use all available CPU cores
        cmd.arg("-loglevel").arg("error"); // Reduce logging overhead
        cmd.arg("-stats"); // Show progress
        
        // Overwrite output file
        cmd.arg("-y");
        
        // Output file
        cmd.arg(output_path);

        // Execute the conversion asynchronously
        let output = cmd.output().await
            .map_err(|e| SpotifyDownloaderError::Conversion(format!("Failed to execute ffmpeg: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(SpotifyDownloaderError::Conversion(format!("FFmpeg conversion failed: {}", error_msg)));
        }

        Ok(())
    }

    /// Get supported input formats
    pub fn get_supported_input_formats(&self) -> Vec<String> {
        vec![
            "mp3".to_string(),
            "m4a".to_string(),
            "flac".to_string(),
            "wav".to_string(),
            "aac".to_string(),
            "ogg".to_string(),
            "wma".to_string(),
        ]
    }

    /// Get supported output formats
    pub fn get_supported_output_formats(&self) -> Vec<AudioFormat> {
        vec![
            AudioFormat::Mp3,
            AudioFormat::M4a,
            AudioFormat::Flac,
            AudioFormat::Wav,
        ]
    }

    /// Get supported bitrates for a format
    pub fn get_supported_bitrates(&self, format: &AudioFormat) -> Vec<Bitrate> {
        match format {
            AudioFormat::Mp3 => vec![
                Bitrate::Kbps128,
                Bitrate::Kbps192,
                Bitrate::Kbps256,
                Bitrate::Kbps320,
            ],
            AudioFormat::M4a => vec![
                Bitrate::Kbps128,
                Bitrate::Kbps192,
                Bitrate::Kbps256,
                Bitrate::Kbps320,
            ],
            AudioFormat::Flac => vec![
                Bitrate::Kbps320, // FLAC is lossless, but we'll use this as a placeholder
            ],
            AudioFormat::Wav => vec![
                Bitrate::Kbps320, // WAV is lossless, but we'll use this as a placeholder
            ],
        }
    }

    /// Check if conversion is needed
    pub fn needs_conversion(&self, input_path: &PathBuf, format: AudioFormat, _bitrate: Bitrate) -> bool {
        // Check if input file exists
        if !input_path.exists() {
            return false;
        }

        // Get file extension
        let input_ext = input_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check if format matches
        let target_ext = match format {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::M4a => "m4a",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "wav",
        };

        // If format doesn't match, conversion is needed
        if input_ext != target_ext {
            return true;
        }

        // For now, only convert if format doesn't match
        // Bitrate conversion would require reading file metadata
        // which is complex and not always necessary
        false
    }

    /// Get estimated output file size
    pub fn estimate_output_size(&self, _input_path: &PathBuf, _bitrate: Bitrate) -> Result<u64> {
        // TODO: Implement size estimation based on bitrate and duration
        // This would involve:
        // 1. Getting the duration of the input file
        // 2. Calculating size based on bitrate
        // 3. Adding some overhead for metadata

        // For now, return a placeholder
        Ok(5 * 1024 * 1024) // 5MB placeholder
    }
    
    /// Check if FFmpeg is available in PATH
    fn check_ffmpeg_availability(&self) -> Result<()> {
        let output = std::process::Command::new("ffmpeg")
            .arg("-version")
            .output();
            
        match output {
            Ok(output) => {
                if output.status.success() {
                    // Log FFmpeg version for debugging
                    if let Ok(version_output) = String::from_utf8(output.stdout) {
                        if let Some(first_line) = version_output.lines().next() {
                            println!("Using FFmpeg: {}", first_line);
                        }
                    }
                    Ok(())
                } else {
                    Err(SpotifyDownloaderError::Conversion(
                        "FFmpeg is not working properly".to_string()
                    ))
                }
            }
            Err(_) => {
                Err(SpotifyDownloaderError::Conversion(
                    "FFmpeg not found in PATH. Please install FFmpeg and ensure it's in your PATH".to_string()
                ))
            }
        }
    }
    
    /// Get FFmpeg version information
    pub fn get_ffmpeg_version(&self) -> Result<String> {
        let output = std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map_err(|_| SpotifyDownloaderError::Conversion("FFmpeg not found".to_string()))?;
            
        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|_| SpotifyDownloaderError::Conversion("Invalid UTF-8 in FFmpeg output".to_string()))
        } else {
            Err(SpotifyDownloaderError::Conversion("FFmpeg version check failed".to_string()))
        }
    }
}
