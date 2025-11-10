use anyhow::Result;
use colored::*;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn download_prebuilt_file(
        &self,
        platform: &str,
        frida_version: &str,
    ) -> Result<Vec<u8>> {
        // First, get the list of files from the release
        let files = self.get_release_files(frida_version).await?;

        // Find the best matching file based on platform and version
        let matched_file = self.find_matching_file(&files, platform, frida_version)?;

        let url = matched_file.download_url;
        let filename = matched_file.name;

        println!(
            "{} {}",
            "→".blue(),
            format!("Downloading prebuilt file: {}", filename).blue()
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to download file: HTTP {}: {}",
                response.status(),
                url
            );
        }

        let total_size = response.content_length().unwrap_or(0);
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );

        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();
        let mut data = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message("Download complete!");

        Ok(data)
    }

    pub async fn download_to_file(&self, url: &str, path: &Path) -> Result<()> {
        println!("{} {}", "→".blue(), format!("Downloading: {}", url).blue());

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to download file: HTTP {}: {}",
                response.status(),
                url
            );
        }

        let total_size = response.content_length().unwrap_or(0);
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );

        let mut file = File::create(path).await?;
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        file.flush().await?;
        pb.finish_with_message("Download complete!");

        println!(
            "{} {}",
            "✓".green(),
            format!("Saved to: {}", path.display()).green()
        );

        Ok(())
    }

    pub async fn get_available_releases(&self) -> Result<Vec<String>> {
        let url = "https://api.github.com/repos/FriRebuild/fripack-inject/releases";
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to fetch releases: HTTP {}: {}",
                response.status(),
                url
            );
        }

        let releases: Vec<serde_json::Value> = response.json().await?;
        let mut versions = Vec::new();

        for release in releases {
            if let Some(tag_name) = release.get("tag_name").and_then(|v| v.as_str()) {
                if let Some(version) = tag_name.strip_prefix('v') {
                    versions.push(version.to_string());
                }
            }
        }

        versions.sort_by(|a, b| b.cmp(a)); // Sort in descending order

        Ok(versions)
    }

    /// Get the list of files for a specific release
    pub async fn get_release_files(&self, frida_version: &str) -> Result<Vec<ReleaseAsset>> {
        let url = format!(
            "https://api.github.com/repos/FriRebuild/fripack-inject/releases/tags/{}",
            frida_version
        );
        let response = self
            .client
            .get(&url)
            .header("User-Agent", "fripack-downloader")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to fetch release: HTTP {}: {}",
                response.status(),
                url
            );
        }

        let release: serde_json::Value = response.json().await?;
        let assets = release
            .get("assets")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("No assets found in release"))?;

        let mut files = Vec::new();
        for asset in assets {
            if let (Some(name), Some(download_url)) = (
                asset.get("name").and_then(|v| v.as_str()),
                asset.get("browser_download_url").and_then(|v| v.as_str()),
            ) {
                files.push(ReleaseAsset {
                    name: name.to_string(),
                    download_url: download_url.to_string(),
                });
            }
        }

        Ok(files)
    }

    /// Find the best matching file based on platform and version keywords
    fn find_matching_file(
        &self,
        files: &[ReleaseAsset],
        platform: &str,
        frida_version: &str,
    ) -> Result<ReleaseAsset> {
        // Platform mapping for better matching
        let platform_mappings = std::collections::HashMap::from([
            ("arm64-v8a", vec!["android-arm64", "arm64"]),
            ("armeabi-v7a", vec!["android-arm", "arm"]),
            ("x86", vec!["android-x86", "x86"]),
            ("x86_64", vec!["android-x86_64", "x86_64"]),
            ("linux-x86_64", vec!["linux-x86_64"]),
        ]);

        let platform_keywords = platform_mappings
            .get(platform)
            .unwrap_or(&vec![platform])
            .clone();

        // First try to find exact matches with version
        for file in files {
            let filename = file.name.to_lowercase();
            let version_lower = frida_version.to_lowercase();

            // Check if file contains version and platform keywords
            if filename.contains(&version_lower) {
                for keyword in &platform_keywords {
                    if filename.contains(&keyword.to_lowercase()) {
                        return Ok(file.clone());
                    }
                }
            }
        }

        // If no exact match, try platform-only matching
        for file in files {
            let filename = file.name.to_lowercase();

            for keyword in &platform_keywords {
                if filename.contains(&keyword.to_lowercase()) {
                    println!(
                        "{} {}",
                        "⚠".yellow(),
                        format!(
                            "Warning: Found platform match but version may not match exactly: {}",
                            file.name
                        )
                        .yellow()
                    );
                    return Ok(file.clone());
                }
            }
        }

        // If still no match, try to find any .so file as fallback
        for file in files {
            if file.name.ends_with(".so") {
                println!(
                    "{} {}",
                    "⚠".yellow(),
                    format!(
                        "Warning: Using fallback file (no platform match): {}",
                        file.name
                    )
                    .yellow()
                );
                return Ok(file.clone());
            }
        }

        anyhow::bail!(
            "No matching file found for platform: {} and version: {}",
            platform,
            frida_version
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}
