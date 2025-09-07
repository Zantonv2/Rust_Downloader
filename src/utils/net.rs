use crate::errors::Result;
use reqwest::Client;
use std::time::Duration;

/// Network utilities
#[allow(dead_code)]
pub struct NetworkUtils {
    client: Client,
}

impl NetworkUtils {
    /// Create a new network utils instance
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("SpotifyDownloader/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Create a new network utils instance with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("SpotifyDownloader/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Download file from URL
    pub async fn download_file(&self, url: &str, output_path: &std::path::PathBuf) -> Result<()> {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Network(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }

        let bytes = response.bytes().await?;
        
        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        }

        std::fs::write(output_path, bytes)
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;

        Ok(())
    }

    /// Download file with progress callback
    pub async fn download_file_with_progress<F>(
        &self,
        url: &str,
        output_path: &std::path::PathBuf,
        progress_callback: F,
    ) -> Result<()>
    where
        F: Fn(u64, u64), // (downloaded, total)
    {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Network(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0u64;
        // Ensure output directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
        }

        let mut file = tokio::fs::File::create(output_path)
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;

        // Download the response body
        let bytes = response.bytes().await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Network(e))?;
        
        tokio::io::AsyncWriteExt::write_all(&mut file, &bytes)
            .await
            .map_err(|e| crate::errors::SpotifyDownloaderError::Io(e))?;
            
        downloaded += bytes.len() as u64;
        progress_callback(downloaded, total_size);

        Ok(())
    }

    /// Make HTTP GET request
    pub async fn get(&self, url: &str) -> Result<String> {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Network(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }

        let text = response.text().await?;
        Ok(text)
    }

    /// Make HTTP POST request
    pub async fn post(&self, url: &str, body: &str) -> Result<String> {
        let response = self.client
            .post(url)
            .body(body.to_string())
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Network(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }

        let text = response.text().await?;
        Ok(text)
    }

    /// Make HTTP POST request with form data
    pub async fn post_form(&self, url: &str, form_data: &[(&str, &str)]) -> Result<String> {
        let response = self.client
            .post(url)
            .form(form_data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Network(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }

        let text = response.text().await?;
        Ok(text)
    }

    /// Check if URL is reachable
    pub async fn is_url_reachable(&self, url: &str) -> bool {
        match self.client.head(url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// Get content type of URL
    pub async fn get_content_type(&self, url: &str) -> Result<String> {
        let response = self.client.head(url).send().await?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Network(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        Ok(content_type)
    }

    /// Get file size from URL without downloading
    pub async fn get_content_length(&self, url: &str) -> Result<Option<u64>> {
        let response = self.client.head(url).send().await?;

        if !response.status().is_success() {
            return Err(crate::errors::SpotifyDownloaderError::Network(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }

        Ok(response.content_length())
    }

    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        url::Url::parse(url).is_ok()
    }

    /// Extract domain from URL
    pub fn extract_domain(url: &str) -> Option<String> {
        url::Url::parse(url)
            .ok()
            .and_then(|url| url.host_str().map(|s| s.to_string()))
    }

    /// Check if URL is HTTPS
    pub fn is_https(url: &str) -> bool {
        url::Url::parse(url)
            .map(|url| url.scheme() == "https")
            .unwrap_or(false)
    }

    /// Build query string from parameters
    pub fn build_query_string(params: &[(&str, &str)]) -> String {
        let mut query_parts = Vec::new();
        
        for (key, value) in params {
            let encoded_key = urlencoding::encode(key);
            let encoded_value = urlencoding::encode(value);
            query_parts.push(format!("{}={}", encoded_key, encoded_value));
        }
        
        query_parts.join("&")
    }

    /// Parse query string into parameters
    pub fn parse_query_string(query: &str) -> Vec<(String, String)> {
        let mut params = Vec::new();
        
        for pair in query.split('&') {
            if let Some(equal_pos) = pair.find('=') {
                let key = urlencoding::decode(&pair[..equal_pos])
                    .unwrap_or_else(|e| std::borrow::Cow::Owned(e.to_string()));
                let value = urlencoding::decode(&pair[equal_pos + 1..])
                    .unwrap_or_else(|e| std::borrow::Cow::Owned(e.to_string()));
                params.push((key.to_string(), value.to_string()));
            }
        }
        
        params
    }
}
