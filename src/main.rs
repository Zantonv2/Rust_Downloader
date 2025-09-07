mod cli;
mod config;
mod settings;
mod errors;
mod downloader;
mod lyrics;
mod utils;
mod ui;
mod csv_import;
mod api;

use cli::Cli;
use errors::Result;
use utils::logger::Logger;
use api::init_api_manager;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    Logger::init()?;

    // Initialize API manager with configuration from settings
    let settings = crate::settings::Settings::load_from_local_json().unwrap_or_else(|_| {
        let settings = crate::settings::Settings::default();
        settings.save_to_local_json().ok();
        settings
    });
    
    let config = settings.config();
    let api_config = api::ApiConfig {
        spotify_client_id: config.api_keys.spotify_client_id.clone().unwrap_or_default(),
        spotify_client_secret: config.api_keys.spotify_client_secret.clone().unwrap_or_default(),
        lastfm_api_key: config.api_keys.lastfm_api_key.clone(),
        musicbrainz_user_agent: "SpotifyDownloader/1.0".to_string(),
        request_timeout: std::time::Duration::from_secs(30),
        max_retries: 3,
        retry_delay: std::time::Duration::from_millis(500),
        proxy_config: Some(config.proxy_config.clone()),
    };
    init_api_manager(api_config);

    // Parse command line arguments
    let cli = Cli::parse();

    // Execute the command
    cli.execute().await?;

    Ok(())
}
