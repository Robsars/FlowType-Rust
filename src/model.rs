use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::copy;
use log::info;

pub struct ModelManager {
    model_dir: PathBuf,
}

impl ModelManager {
    pub fn new() -> Self {
        let model_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("models");
        
        Self { model_dir }
    }

    /// Returns the path to the requested model, downloading it if necessary.
    /// Default suggested: "distil-medium.en" or "base.en"
    pub fn get_or_download_model(&self, model_name: &str) -> Result<PathBuf> {
        if !self.model_dir.exists() {
            fs::create_dir_all(&self.model_dir).context("Failed to create models directory")?;
        }

        let file_name = format!("ggml-{}.bin", model_name);
        let model_path = self.model_dir.join(&file_name);

        if model_path.exists() {
            info!("Model located at: {:?}", model_path);
            return Ok(model_path);
        }

        info!("Model '{}' not found. Downloading...", model_name);
        self.download_model(model_name, &model_path)?;
        
        Ok(model_path)
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
