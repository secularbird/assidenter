//! Embedded on-device inference services for mobile platforms
//! 
//! This module provides local inference capabilities that run directly on the device
//! without requiring external servers. This is essential for Android/iOS where
//! running Docker containers is not practical.
//!
//! ## Architecture
//! 
//! On mobile devices, the AI services run embedded within the app:
//! - ASR: Uses a lightweight Whisper model (tiny/base) for speech recognition
//! - LLM: Uses a quantized model (Qwen 0.5B Q4) via the app's inference engine
//! - TTS: Uses a simple on-device TTS solution
//!
//! ## Model Management
//!
//! Models are downloaded on first launch and stored in the app's data directory.
//! The app provides a model download UI to manage this process.

pub mod asr;
pub mod llm;
pub mod tts;
pub mod model_manager;

pub use asr::EmbeddedASR;
pub use llm::EmbeddedLLM;
pub use tts::EmbeddedTTS;
pub use model_manager::ModelManager;

use std::path::PathBuf;
use once_cell::sync::Lazy;

/// Default model directory path
pub static MODEL_DIR: Lazy<PathBuf> = Lazy::new(|| {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("assidenter")
        .join("models")
});

/// Model file names
pub const WHISPER_MODEL_FILE: &str = "whisper-tiny.bin";
pub const LLM_MODEL_FILE: &str = "qwen2-0.5b-q4.gguf";

/// Model download URLs (from Hugging Face)
pub const WHISPER_MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin";
pub const LLM_MODEL_URL: &str = "https://huggingface.co/Qwen/Qwen2-0.5B-Instruct-GGUF/resolve/main/qwen2-0_5b-instruct-q4_k_m.gguf";
