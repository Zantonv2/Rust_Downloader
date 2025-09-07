use crate::errors::Result;
use std::path::PathBuf;

/// File system utilities
#[allow(dead_code)]
pub struct FileUtils;

impl FileUtils {
    /// Get file size in bytes
    pub fn get_file_size(path: &PathBuf) -> Result<u64> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        Ok(metadata.len())
    }

    /// Check if file exists
    pub fn file_exists(path: &PathBuf) -> bool {
        path.exists() && path.is_file()
    }

    /// Check if directory exists
    pub fn directory_exists(path: &PathBuf) -> bool {
        path.exists() && path.is_dir()
    }

    /// Create directory recursively
    pub fn create_directory(path: &PathBuf) -> Result<()> {
        std::fs::create_dir_all(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        Ok(())
    }

    /// Delete file
    pub fn delete_file(path: &PathBuf) -> Result<()> {
        std::fs::remove_file(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        Ok(())
    }

    /// Delete directory recursively
    pub fn delete_directory(path: &PathBuf) -> Result<()> {
        std::fs::remove_dir_all(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        Ok(())
    }

    /// Copy file
    pub fn copy_file(from: &PathBuf, to: &PathBuf) -> Result<()> {
        // Ensure destination directory exists
        if let Some(parent) = to.parent() {
            Self::create_directory(&parent.to_path_buf())?;
        }

        std::fs::copy(from, to)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        Ok(())
    }

    /// Move file
    pub fn move_file(from: &PathBuf, to: &PathBuf) -> Result<()> {
        // Ensure destination directory exists
        if let Some(parent) = to.parent() {
            Self::create_directory(&parent.to_path_buf())?;
        }

        std::fs::rename(from, to)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        Ok(())
    }

    /// Read file contents as string
    pub fn read_file_to_string(path: &PathBuf) -> Result<String> {
        std::fs::read_to_string(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))
    }

    /// Write string to file
    pub fn write_string_to_file(path: &PathBuf, content: &str) -> Result<()> {
        // Ensure destination directory exists
        if let Some(parent) = path.parent() {
            Self::create_directory(&parent.to_path_buf())?;
        }

        std::fs::write(path, content)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        Ok(())
    }

    /// Read file contents as bytes
    pub fn read_file_to_bytes(path: &PathBuf) -> Result<Vec<u8>> {
        std::fs::read(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))
    }

    /// Write bytes to file
    pub fn write_bytes_to_file(path: &PathBuf, content: &[u8]) -> Result<()> {
        // Ensure destination directory exists
        if let Some(parent) = path.parent() {
            Self::create_directory(&parent.to_path_buf())?;
        }

        std::fs::write(path, content)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        Ok(())
    }

    /// List files in directory
    pub fn list_files(path: &PathBuf) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        let entries = std::fs::read_dir(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() {
                files.push(path);
            }
        }

        Ok(files)
    }

    /// List directories in directory
    pub fn list_directories(path: &PathBuf) -> Result<Vec<PathBuf>> {
        let mut directories = Vec::new();
        
        let entries = std::fs::read_dir(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
            let path = entry.path();
            
            if path.is_dir() {
                directories.push(path);
            }
        }

        Ok(directories)
    }

    /// Get file modification time
    pub fn get_file_modified_time(path: &PathBuf) -> Result<std::time::SystemTime> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        
        metadata.modified()
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))
    }

    /// Check if file is older than specified duration
    pub fn is_file_older_than(path: &PathBuf, duration: std::time::Duration) -> Result<bool> {
        let modified_time = Self::get_file_modified_time(path)?;
        let now = std::time::SystemTime::now();
        
        if let Ok(elapsed) = now.duration_since(modified_time) {
            Ok(elapsed > duration)
        } else {
            Ok(false)
        }
    }

    /// Get temporary file path
    pub fn get_temp_file_path(prefix: &str, suffix: &str) -> Result<PathBuf> {
        let temp_dir = std::env::temp_dir();
        let filename = format!("{}_{}.{}", prefix, uuid::Uuid::new_v4(), suffix);
        Ok(temp_dir.join(filename))
    }

    /// Clean up temporary files older than specified duration
    pub fn cleanup_temp_files(prefix: &str, max_age: std::time::Duration) -> Result<()> {
        let temp_dir = std::env::temp_dir();
        
        if !Self::directory_exists(&temp_dir) {
            return Ok(());
        }

        let files = Self::list_files(&temp_dir)?;
        
        for file in files {
            if let Some(filename) = file.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with(prefix) && Self::is_file_older_than(&file, max_age)? {
                    let _ = Self::delete_file(&file); // Ignore errors for cleanup
                }
            }
        }

        Ok(())
    }
}
