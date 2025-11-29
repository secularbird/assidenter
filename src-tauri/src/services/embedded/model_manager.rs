//! Model Manager for downloading and managing AI models
//! 
//! This module handles downloading, verifying, and managing the AI models
//! required for embedded inference.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::{MODEL_DIR, WHISPER_MODEL_FILE, LLM_MODEL_FILE, WHISPER_MODEL_URL, LLM_MODEL_URL};

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub file_name: String,
    pub download_url: String,
    pub size_bytes: u64,
    pub is_downloaded: bool,
}

/// Download progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f32,
}

/// Model manager for handling model downloads and storage
pub struct ModelManager {
    model_dir: PathBuf,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            model_dir: MODEL_DIR.clone(),
        }
    }

    pub fn with_model_dir(model_dir: PathBuf) -> Self {
        Self { model_dir }
    }

    /// Get the model directory path
    pub fn model_dir(&self) -> &PathBuf {
        &self.model_dir
    }

    /// Ensure the model directory exists
    pub fn ensure_model_dir(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.model_dir)
            .map_err(|e| format!("Failed to create model directory: {}", e))
    }

    /// Get information about all required models
    pub fn get_model_info(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                name: "Whisper Tiny (ASR)".to_string(),
                file_name: WHISPER_MODEL_FILE.to_string(),
                download_url: WHISPER_MODEL_URL.to_string(),
                size_bytes: 75_000_000, // ~75MB
                is_downloaded: self.model_dir.join(WHISPER_MODEL_FILE).exists(),
            },
            ModelInfo {
                name: "Qwen 0.5B Q4 (LLM)".to_string(),
                file_name: LLM_MODEL_FILE.to_string(),
                download_url: LLM_MODEL_URL.to_string(),
                size_bytes: 400_000_000, // ~400MB
                is_downloaded: self.model_dir.join(LLM_MODEL_FILE).exists(),
            },
        ]
    }

    /// Check if all required models are downloaded
    pub fn are_models_ready(&self) -> bool {
        self.model_dir.join(WHISPER_MODEL_FILE).exists() &&
        self.model_dir.join(LLM_MODEL_FILE).exists()
    }

    /// Check if a specific model is downloaded
    pub fn is_model_downloaded(&self, file_name: &str) -> bool {
        self.model_dir.join(file_name).exists()
    }

    /// Get the path to a model file
    pub fn get_model_path(&self, file_name: &str) -> PathBuf {
        self.model_dir.join(file_name)
    }

    /// Get download URL for a model
    pub fn get_download_url(&self, file_name: &str) -> Option<&'static str> {
        match file_name {
            f if f == WHISPER_MODEL_FILE => Some(WHISPER_MODEL_URL),
            f if f == LLM_MODEL_FILE => Some(LLM_MODEL_URL),
            _ => None,
        }
    }

    /// Delete a model file
    pub fn delete_model(&self, file_name: &str) -> Result<(), String> {
        let path = self.model_dir.join(file_name);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete model: {}", e))?;
        }
        Ok(())
    }

    /// Get total size of downloaded models
    pub fn get_downloaded_size(&self) -> u64 {
        let mut total = 0;
        
        for info in self.get_model_info() {
            if info.is_downloaded {
                if let Ok(metadata) = std::fs::metadata(self.model_dir.join(&info.file_name)) {
                    total += metadata.len();
                }
            }
        }
        
        total
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}
