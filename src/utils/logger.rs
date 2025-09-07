use crate::errors::Result;
use tracing::{info, warn, error, debug, Level};
use tracing_subscriber::{fmt, EnvFilter};

/// Logger utility for the application
pub struct Logger;

impl Logger {
    /// Initialize the logger with default configuration
    pub fn init() -> Result<()> {
        Self::init_with_level(Level::INFO)
    }

    /// Initialize the logger with specified level
    pub fn init_with_level(level: Level) -> Result<()> {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(level.to_string()));

        fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_thread_ids(true)
            .with_thread_names(true)
            .init();

        Ok(())
    }

    /// Initialize the logger with custom filter
    pub fn init_with_filter(filter: &str) -> Result<()> {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(filter));

        fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .with_thread_ids(true)
            .with_thread_names(true)
            .init();

        Ok(())
    }

    /// Log info message
    pub fn info(message: &str) {
        info!("{}", message);
    }

    /// Log warning message
    pub fn warn(message: &str) {
        warn!("{}", message);
    }

    /// Log error message
    pub fn error(message: &str) {
        error!("{}", message);
    }

    /// Log debug message
    pub fn debug(message: &str) {
        debug!("{}", message);
    }

    /// Log info message with context
    pub fn info_with_context(message: &str, context: &str) {
        info!("[{}] {}", context, message);
    }

    /// Log warning message with context
    pub fn warn_with_context(message: &str, context: &str) {
        warn!("[{}] {}", context, message);
    }

    /// Log error message with context
    pub fn error_with_context(message: &str, context: &str) {
        error!("[{}] {}", context, message);
    }

    /// Log debug message with context
    pub fn debug_with_context(message: &str, context: &str) {
        debug!("[{}] {}", context, message);
    }

    /// Log download progress
    pub fn log_download_progress(current: u64, total: u64, filename: &str) {
        let percentage = if total > 0 {
            (current as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        info!(
            "Downloading {}: {}/{} bytes ({:.1}%)",
            filename, current, total, percentage
        );
    }

    /// Log operation start
    pub fn log_operation_start(operation: &str) {
        info!("Starting operation: {}", operation);
    }

    /// Log operation completion
    pub fn log_operation_complete(operation: &str) {
        info!("Completed operation: {}", operation);
    }

    /// Log operation failure
    pub fn log_operation_failed(operation: &str, error: &str) {
        error!("Failed operation: {} - Error: {}", operation, error);
    }

    /// Log configuration loaded
    pub fn log_config_loaded(path: &str) {
        info!("Configuration loaded from: {}", path);
    }

    /// Log configuration saved
    pub fn log_config_saved(path: &str) {
        info!("Configuration saved to: {}", path);
    }

    /// Log track download start
    pub fn log_track_download_start(artist: &str, title: &str) {
        info!("Starting download: {} - {}", artist, title);
    }

    /// Log track download complete
    pub fn log_track_download_complete(artist: &str, title: &str, path: &str) {
        info!("Download complete: {} - {} -> {}", artist, title, path);
    }

    /// Log track download failed
    pub fn log_track_download_failed(artist: &str, title: &str, error: &str) {
        error!("Download failed: {} - {} - Error: {}", artist, title, error);
    }

    /// Log API request
    pub fn log_api_request(method: &str, url: &str) {
        debug!("API Request: {} {}", method, url);
    }

    /// Log API response
    pub fn log_api_response(status: u16, url: &str) {
        debug!("API Response: {} {}", status, url);
    }

    /// Log API error
    pub fn log_api_error(error: &str, url: &str) {
        error!("API Error: {} - URL: {}", error, url);
    }

    /// Log file operation
    pub fn log_file_operation(operation: &str, path: &str) {
        debug!("File operation: {} -> {}", operation, path);
    }

    /// Log file operation error
    pub fn log_file_operation_error(operation: &str, path: &str, error: &str) {
        error!("File operation error: {} -> {} - Error: {}", operation, path, error);
    }

    /// Log network operation
    pub fn log_network_operation(operation: &str, url: &str) {
        debug!("Network operation: {} -> {}", operation, url);
    }

    /// Log network operation error
    pub fn log_network_operation_error(operation: &str, url: &str, error: &str) {
        error!("Network operation error: {} -> {} - Error: {}", operation, url, error);
    }

    /// Log conversion operation
    pub fn log_conversion_start(input_format: &str, output_format: &str, input_path: &str) {
        info!("Starting conversion: {} -> {} ({})", input_format, output_format, input_path);
    }

    /// Log conversion complete
    pub fn log_conversion_complete(input_format: &str, output_format: &str, output_path: &str) {
        info!("Conversion complete: {} -> {} ({})", input_format, output_format, output_path);
    }

    /// Log conversion failed
    pub fn log_conversion_failed(input_format: &str, output_format: &str, error: &str) {
        error!("Conversion failed: {} -> {} - Error: {}", input_format, output_format, error);
    }

    /// Log metadata embedding
    pub fn log_metadata_embedding(file_path: &str) {
        debug!("Embedding metadata: {}", file_path);
    }

    /// Log metadata embedding complete
    pub fn log_metadata_embedding_complete(file_path: &str) {
        debug!("Metadata embedding complete: {}", file_path);
    }

    /// Log metadata embedding failed
    pub fn log_metadata_embedding_failed(file_path: &str, error: &str) {
        error!("Metadata embedding failed: {} - Error: {}", file_path, error);
    }
}
