mod services;

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter, State};
use serde::{Deserialize, Serialize};
use base64::Engine;

use crate::services::{WhisperLiveKit, QwenLLM, VoxCPMTTS};
use crate::services::asr::WhisperConfig;
use crate::services::llm::QwenConfig;
use crate::services::tts::VoxCPMConfig;

/// Application state (thread-safe)
pub struct AppState {
    asr: Mutex<WhisperLiveKit>,
    llm: Mutex<QwenLLM>,
    tts: Mutex<VoxCPMTTS>,
    is_listening: AtomicBool,
}

impl AppState {
    fn new() -> Self {
        Self {
            asr: Mutex::new(WhisperLiveKit::new(WhisperConfig::default())),
            llm: Mutex::new(QwenLLM::new(QwenConfig::default())),
            tts: Mutex::new(VoxCPMTTS::new(VoxCPMConfig::default())),
            is_listening: AtomicBool::new(false),
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
            process_audio,
            configure_services,
            clear_conversation,
            send_text_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
