
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::copy;
use log::info;
use tauri::{AppHandle, Manager};

pub struct ModelManager {
    app: AppHandle,
}

impl ModelManager {
    pub fn new(app: &AppHandle) -> Self {
        Self { app: app.clone() }
    }

    /// Returns the path to the requested model.
    /// Priority: 1. Bundled Resource, 2. Local File, 3. Download
    pub fn get_or_download_model(&self, model_name: &str) -> Result<PathBuf> {
        let file_name = format!("ggml-{}.bin", model_name);

        // 1. Check Bundled Resources
        if let Ok(resource_dir) = self.app.path().resource_dir() {
            let bundled_path = resource_dir.join("models").join(&file_name);
            if bundled_path.exists() {
                info!("Using bundled model: {:?}", bundled_path);
                return Ok(bundled_path);
            }
        }

        // 2. Check Local Directory (typical for dev environment)
        let local_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("models");
        
        if !local_dir.exists() {
            fs::create_dir_all(&local_dir).context("Failed to create models directory")?;
        }

        let local_path = local_dir.join(&file_name);
        if local_path.exists() {
            info!("Using local model: {:?}", local_path);
            return Ok(local_path);
        }

        // 3. Download
        info!("Model '{}' not found. Downloading...", model_name);
        self.download_model(model_name, &local_path)?;
        
        Ok(local_path)
    }

    fn download_model(&self, name: &str, dest: &Path) -> Result<()> {
        // Construct URL for HuggingFace (ggerganov/whisper.cpp)
        // Note: distil models might be in a different repo, but let's stick to standard for now or provide full URL logic
        let url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
            name
        );

        info!("Downloading from: {}", url);

        let mut response = reqwest::blocking::get(&url)
            .context("Failed to send request to model URL")?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to download model: Status {}", response.status());
        }

        let mut dest_file = fs::File::create(dest)
            .context("Failed to create model file")?;

        copy(&mut response, &mut dest_file)
            .context("Failed to write model content to file")?;

        info!("Download complete: {:?}", dest);
        Ok(())
    }
}
