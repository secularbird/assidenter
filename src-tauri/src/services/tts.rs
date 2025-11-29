use serde::{Deserialize, Serialize};
use reqwest::Client;
use base64::{Engine as _, engine::general_purpose::STANDARD};

/// VoxCPM TTS configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoxCPMConfig {
    pub server_url: String,
    pub voice: String,
    pub speed: f32,
    pub sample_rate: u32,
}

impl Default for VoxCPMConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:5500".to_string(),
            voice: "default".to_string(),
            speed: 1.0,
            sample_rate: 22050,
        }
    }
}

/// TTS synthesis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSResult {
    pub audio_data: Vec<u8>,
    pub sample_rate: u32,
    pub duration: f64,
}

/// VoxCPM TTS service client
pub struct VoxCPMTTS {
    config: VoxCPMConfig,
    client: Client,
}

impl VoxCPMTTS {
    pub fn new(config: VoxCPMConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Synthesize text to speech
    pub async fn synthesize(&self, text: &str) -> Result<TTSResult, String> {
        // Create the request payload
        let payload = serde_json::json!({
            "text": text,
            "voice": self.config.voice,
            "speed": self.config.speed,
            "sample_rate": self.config.sample_rate,
            "format": "wav"
        });

        // Send request to VoxCPM server
        let response = self.client
            .post(format!("{}/tts", self.config.server_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to send TTS request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("TTS request failed with status: {}", response.status()));
        }

        // Check if response is JSON with base64 audio or raw audio bytes
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let audio_data = if content_type.contains("application/json") {
            // JSON response with base64 encoded audio
            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse TTS response: {}", e))?;

            let audio_base64 = result["audio"]
                .as_str()
                .ok_or("Missing audio data in response")?;

            STANDARD
                .decode(audio_base64)
                .map_err(|e| format!("Failed to decode audio data: {}", e))?
        } else {
            // Raw audio bytes
            response
                .bytes()
                .await
                .map_err(|e| format!("Failed to read audio bytes: {}", e))?
                .to_vec()
        };

        // Calculate approximate duration assuming 16-bit mono PCM audio
        // Duration = total_bytes / (sample_rate * bytes_per_sample * channels)
        // For 16-bit mono: bytes_per_sample = 2, channels = 1
        let bytes_per_sample: f64 = 2.0;
        let duration = audio_data.len() as f64 / (self.config.sample_rate as f64 * bytes_per_sample);

        Ok(TTSResult {
            audio_data,
            sample_rate: self.config.sample_rate,
            duration,
        })
    }

    /// Get current configuration
    pub fn config(&self) -> &VoxCPMConfig {
        &self.config
    }

    /// Update server URL
    pub fn set_server_url(&mut self, url: String) {
        self.config.server_url = url;
    }

    /// Update voice
    pub fn set_voice(&mut self, voice: String) {
        self.config.voice = voice;
    }

    /// Update speech speed
    pub fn set_speed(&mut self, speed: f32) {
        self.config.speed = speed;
    }
}
