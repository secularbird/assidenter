mod services;

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use serde::{Deserialize, Serialize};
use base64::Engine;

use crate::services::{WhisperLiveKit, QwenLLM, VoxCPMTTS, ServiceMode};
use crate::services::asr::WhisperConfig;
use crate::services::llm::QwenConfig;
use crate::services::tts::VoxCPMConfig;

#[cfg(feature = "embedded-services")]
use crate::services::embedded::{ModelManager, ModelInfo};

/// Application state (thread-safe)
pub struct AppState {
    asr: Mutex<WhisperLiveKit>,
    llm: Mutex<QwenLLM>,
    tts: Mutex<VoxCPMTTS>,
    is_listening: AtomicBool,
    service_mode: ServiceMode,
    #[cfg(feature = "embedded-services")]
    model_manager: ModelManager,
}

impl AppState {
    fn new() -> Self {
        Self {
            asr: Mutex::new(WhisperLiveKit::new(WhisperConfig::default())),
            llm: Mutex::new(QwenLLM::new(QwenConfig::default())),
            tts: Mutex::new(VoxCPMTTS::new(VoxCPMConfig::default())),
            is_listening: AtomicBool::new(false),
            service_mode: ServiceMode::default(),
            #[cfg(feature = "embedded-services")]
            model_manager: ModelManager::new(),
        }
    }
}

/// Service configuration for the frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub asr_url: String,
    pub llm_url: String,
    pub tts_url: String,
}

/// Processing result sent to frontend
#[derive(Debug, Clone, Serialize)]
pub struct ProcessingResult {
    pub status: String,
    pub transcription: Option<String>,
    pub response: Option<String>,
    pub audio_ready: bool,
}

/// Service status for frontend
#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatus {
    pub mode: String,
    pub asr_ready: bool,
    pub llm_ready: bool,
    pub tts_ready: bool,
    #[cfg(feature = "embedded-services")]
    pub models_ready: bool,
}

/// Start listening for voice input (simplified - frontend handles audio)
#[tauri::command]
async fn start_listening(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state.is_listening.load(Ordering::SeqCst) {
        return Err("Already listening".to_string());
    }
    state.is_listening.store(true, Ordering::SeqCst);
    
    // Emit listening started event
    let _ = app.emit("listening-started", ());
    
    log::info!("Listening started");
    Ok(())
}

/// Stop listening for voice input
#[tauri::command]
async fn stop_listening(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.is_listening.store(false, Ordering::SeqCst);
    
    // Emit listening stopped event
    let _ = app.emit("listening-stopped", ());
    
    log::info!("Listening stopped");
    Ok(())
}

/// Check if currently listening
#[tauri::command]
async fn is_listening(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.is_listening.load(Ordering::SeqCst))
}

/// Get current service status
#[tauri::command]
async fn get_service_status(state: State<'_, AppState>) -> Result<ServiceStatus, String> {
    let mode = match state.service_mode {
        ServiceMode::Remote => "remote",
        ServiceMode::Embedded => "embedded",
    };

    Ok(ServiceStatus {
        mode: mode.to_string(),
        asr_ready: true, // Remote services are always "ready" (connectivity checked on use)
        llm_ready: true,
        tts_ready: true,
        #[cfg(feature = "embedded-services")]
        models_ready: state.model_manager.are_models_ready(),
    })
}

/// Process audio data (received from frontend as base64 WAV)
#[tauri::command]
async fn process_audio(
    audio_base64: String,
    app: AppHandle,
    state: State<'_, AppState>
) -> Result<ProcessingResult, String> {
    // Decode base64 audio
    let audio_data = base64::engine::general_purpose::STANDARD
        .decode(&audio_base64)
        .map_err(|e| format!("Failed to decode audio: {}", e))?;
    
    // Emit processing status
    let _ = app.emit("processing-status", "Transcribing...");
    
    // Step 1: ASR - Transcribe speech to text
    let asr = state.asr.lock().await;
    let transcription = asr.transcribe_wav(&audio_data).await?;
    drop(asr);
    
    let transcribed_text = transcription.text.clone();
    log::info!("Transcription: {}", transcribed_text);
    
    let _ = app.emit("transcription", &transcribed_text);
    
    if transcribed_text.trim().is_empty() {
        return Ok(ProcessingResult {
            status: "empty".to_string(),
            transcription: Some(transcribed_text),
            response: None,
            audio_ready: false,
        });
    }
    
    // Step 2: LLM - Generate response
    let _ = app.emit("processing-status", "Thinking...");
    
    let mut llm = state.llm.lock().await;
    let llm_response = llm.chat(&transcribed_text).await?;
    drop(llm);
    
    let response_text = llm_response.text.clone();
    log::info!("LLM Response: {}", response_text);
    
    let _ = app.emit("llm-response", &response_text);
    
    // Step 3: TTS - Synthesize speech
    let _ = app.emit("processing-status", "Generating audio...");
    
    let tts = state.tts.lock().await;
    let tts_result = tts.synthesize(&response_text).await?;
    drop(tts);
    
    // Emit TTS audio data as base64
    let audio_base64 = base64::engine::general_purpose::STANDARD.encode(&tts_result.audio_data);
    let _ = app.emit("tts-audio", audio_base64);
    
    Ok(ProcessingResult {
        status: "complete".to_string(),
        transcription: Some(transcribed_text),
        response: Some(response_text),
        audio_ready: true,
    })
}

/// Configure services
#[tauri::command]
async fn configure_services(config: ServiceConfig, state: State<'_, AppState>) -> Result<(), String> {
    // Update ASR config
    let mut asr = state.asr.lock().await;
    asr.set_server_url(config.asr_url);
    drop(asr);

    // Update LLM config
    let mut llm = state.llm.lock().await;
    llm.set_server_url(config.llm_url);
    drop(llm);

    // Update TTS config
    let mut tts = state.tts.lock().await;
    tts.set_server_url(config.tts_url);
    drop(tts);

    log::info!("Services configured");
    Ok(())
}

/// Clear LLM conversation history
#[tauri::command]
async fn clear_conversation(state: State<'_, AppState>) -> Result<(), String> {
    let mut llm = state.llm.lock().await;
    llm.clear_history();
    log::info!("Conversation cleared");
    Ok(())
}

/// Send a text message to the LLM (without speech)
#[tauri::command]
async fn send_text_message(
    message: String,
    app: AppHandle,
    state: State<'_, AppState>
) -> Result<ProcessingResult, String> {
    // LLM - Generate response
    let _ = app.emit("processing-status", "Thinking...");
    
    let mut llm = state.llm.lock().await;
    let llm_response = llm.chat(&message).await?;
    drop(llm);

    let response_text = llm_response.text.clone();
    let _ = app.emit("llm-response", &response_text);

    // TTS - Synthesize speech
    let _ = app.emit("processing-status", "Generating audio...");
    
    let tts = state.tts.lock().await;
    let tts_result = tts.synthesize(&response_text).await?;
    drop(tts);

    // Emit TTS audio data as base64
    let audio_base64 = base64::engine::general_purpose::STANDARD.encode(&tts_result.audio_data);
    let _ = app.emit("tts-audio", audio_base64);

    Ok(ProcessingResult {
        status: "complete".to_string(),
        transcription: Some(message),
        response: Some(response_text),
        audio_ready: true,
    })
}

// ============================================================================
// Model Management Commands (for embedded/Android mode)
// ============================================================================

/// Get information about required models
#[cfg(feature = "embedded-services")]
#[tauri::command]
async fn get_model_info(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, String> {
    Ok(state.model_manager.get_model_info())
}

/// Check if all models are ready
#[cfg(feature = "embedded-services")]
#[tauri::command]
async fn are_models_ready(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.model_manager.are_models_ready())
}

/// Get model download URL
#[cfg(feature = "embedded-services")]
#[tauri::command]
async fn get_model_download_url(file_name: String, state: State<'_, AppState>) -> Result<String, String> {
    state.model_manager.get_download_url(&file_name)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Unknown model: {}", file_name))
}

/// Get model directory path
#[cfg(feature = "embedded-services")]
#[tauri::command]
async fn get_model_dir(state: State<'_, AppState>) -> Result<String, String> {
    state.model_manager.ensure_model_dir()?;
    Ok(state.model_manager.model_dir().to_string_lossy().to_string())
}

// Placeholder commands for non-embedded builds
#[cfg(not(feature = "embedded-services"))]
#[tauri::command]
async fn get_model_info() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[cfg(not(feature = "embedded-services"))]
#[tauri::command]
async fn are_models_ready() -> Result<bool, String> {
    Ok(true) // Remote mode doesn't need local models
}

#[cfg(not(feature = "embedded-services"))]
#[tauri::command]
async fn get_model_download_url(_file_name: String) -> Result<String, String> {
    Err("Model downloads not available in remote mode".to_string())
}

#[cfg(not(feature = "embedded-services"))]
#[tauri::command]
async fn get_model_dir() -> Result<String, String> {
    Err("Model directory not available in remote mode".to_string())
}

/// Screenshot result sent to frontend
#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotResult {
    pub success: bool,
    pub image_base64: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub error: Option<String>,
}

/// Take a screenshot of a specific monitor
#[tauri::command]
async fn take_screenshot(monitor_index: Option<usize>) -> Result<ScreenshotResult, String> {
    use xcap::Monitor;
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;
    
    // Get all monitors
    let monitors = Monitor::all()
        .map_err(|e| format!("Failed to get monitors: {}", e))?;
    
    if monitors.is_empty() {
        return Ok(ScreenshotResult {
            success: false,
            image_base64: None,
            width: None,
            height: None,
            error: Some("No monitors found".to_string()),
        });
    }
    
    // Select monitor (default to primary/first monitor)
    let index = monitor_index.unwrap_or(0);
    let monitor = monitors.get(index)
        .ok_or_else(|| format!("Monitor index {} out of range (available: {})", index, monitors.len()))?;
    
    // Capture screenshot
    let image = monitor.capture_image()
        .map_err(|e| format!("Failed to capture screenshot: {}", e))?;
    
    // Convert to PNG and encode as base64
    let mut png_data = Vec::new();
    let encoder = PngEncoder::new(&mut png_data);
    encoder.write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgba8,
    ).map_err(|e| format!("Failed to encode image: {}", e))?;
    
    let base64_image = base64::engine::general_purpose::STANDARD.encode(&png_data);
    
    log::info!("Screenshot captured: {}x{}", image.width(), image.height());
    
    Ok(ScreenshotResult {
        success: true,
        image_base64: Some(base64_image),
        width: Some(image.width()),
        height: Some(image.height()),
        error: None,
    })
}

/// Get list of available monitors for screenshot
#[tauri::command]
async fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    use xcap::Monitor;
    
    let monitors = Monitor::all()
        .map_err(|e| format!("Failed to get monitors: {}", e))?;
    
    let monitor_infos: Vec<MonitorInfo> = monitors.iter().enumerate().map(|(index, monitor)| {
        MonitorInfo {
            index,
            name: monitor.name().to_string(),
            x: monitor.x(),
            y: monitor.y(),
            width: monitor.width(),
            height: monitor.height(),
            is_primary: monitor.is_primary(),
        }
    }).collect();
    
    Ok(monitor_infos)
}

/// Monitor information for frontend
#[derive(Debug, Clone, Serialize)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_listening,
            stop_listening,
            is_listening,
            get_service_status,
            process_audio,
            configure_services,
            clear_conversation,
            send_text_message,
            // Model management
            get_model_info,
            are_models_ready,
            get_model_download_url,
            get_model_dir,
            // Screenshot
            take_screenshot,
            get_monitors,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
