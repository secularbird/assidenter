use serde::{Deserialize, Serialize};
use reqwest::Client;
use base64::{Engine as _, engine::general_purpose::STANDARD};

/// WhisperLiveKit ASR service configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperConfig {
    pub server_url: String,
    pub language: String,
    pub model: String,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:9090".to_string(),
            language: "auto".to_string(),
            model: "whisper-large-v3".to_string(),
        }
    }
}

/// ASR transcription result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration: Option<f64>,
    pub is_final: bool,
}

/// WhisperLiveKit ASR service client
pub struct WhisperLiveKit {
    config: WhisperConfig,
    client: Client,
}

impl WhisperLiveKit {
    pub fn new(config: WhisperConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Transcribe WAV audio data to text
    pub async fn transcribe_wav(&self, wav_data: &[u8]) -> Result<TranscriptionResult, String> {
        // Encode as base64
        let audio_base64 = STANDARD.encode(wav_data);
        
        // Create the request payload
        let payload = serde_json::json!({
            "audio": audio_base64,
            "language": self.config.language,
            "model": self.config.model,
            "format": "wav"
        });

        // Send request to WhisperLiveKit server
        let response = self.client
            .post(format!("{}/transcribe", self.config.server_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to send transcription request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Transcription failed with status: {}", response.status()));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse transcription response: {}", e))?;

        Ok(TranscriptionResult {
            text: result["text"].as_str().unwrap_or("").to_string(),
            language: result["language"].as_str().map(|s| s.to_string()),
            duration: result["duration"].as_f64(),
            is_final: true,
        })
    }

    /// Transcribe audio samples to text
    pub async fn transcribe(&self, samples: &[i16], sample_rate: u32) -> Result<TranscriptionResult, String> {
        // Convert samples to WAV format
        let wav_data = self.samples_to_wav(samples, sample_rate)?;
        self.transcribe_wav(&wav_data).await
    }

    /// Convert i16 samples to WAV format bytes
    fn samples_to_wav(&self, samples: &[i16], sample_rate: u32) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::new();
        
        // WAV header
        let data_size = (samples.len() * 2) as u32;
        let file_size = data_size + 36;
        
        // RIFF header
        buffer.extend_from_slice(b"RIFF");
        buffer.extend_from_slice(&file_size.to_le_bytes());
        buffer.extend_from_slice(b"WAVE");
        
        // fmt subchunk
        buffer.extend_from_slice(b"fmt ");
        buffer.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size for PCM
        buffer.extend_from_slice(&1u16.to_le_bytes());   // AudioFormat (1 = PCM)
        buffer.extend_from_slice(&1u16.to_le_bytes());   // NumChannels
        buffer.extend_from_slice(&sample_rate.to_le_bytes()); // SampleRate
        buffer.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // ByteRate
        buffer.extend_from_slice(&2u16.to_le_bytes());   // BlockAlign
        buffer.extend_from_slice(&16u16.to_le_bytes());  // BitsPerSample
        
        // data subchunk
        buffer.extend_from_slice(b"data");
        buffer.extend_from_slice(&data_size.to_le_bytes());
        
        // Audio data
        for sample in samples {
            buffer.extend_from_slice(&sample.to_le_bytes());
        }
        
        Ok(buffer)
    }

    /// Get current configuration
    pub fn config(&self) -> &WhisperConfig {
        &self.config
    }

    /// Update server URL
    pub fn set_server_url(&mut self, url: String) {
        self.config.server_url = url;
    }
}
