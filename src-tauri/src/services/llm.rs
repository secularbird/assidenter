use serde::{Deserialize, Serialize};
use reqwest::Client;
use futures::StreamExt;

/// Qwen LLM configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QwenConfig {
    pub server_url: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: String,
}

impl Default for QwenConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".to_string(),
            model: "qwen-0.5b".to_string(),
            temperature: 0.7,
            max_tokens: 512,
            system_prompt: "You are a helpful AI assistant. Respond concisely and helpfully.".to_string(),
        }
    }
}

/// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub text: String,
    pub finish_reason: Option<String>,
}

/// Qwen 0.5 LLM service client
pub struct QwenLLM {
    config: QwenConfig,
    client: Client,
    conversation_history: Vec<ChatMessage>,
}

impl QwenLLM {
    pub fn new(config: QwenConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            conversation_history: Vec::new(),
        }
    }

    /// Send a message to the LLM and get a response
    pub async fn chat(&mut self, user_message: &str) -> Result<LLMResponse, String> {
        // Add user message to history
        self.conversation_history.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        // Build messages array with system prompt
        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: self.config.system_prompt.clone(),
        }];
        messages.extend(self.conversation_history.clone());

        // Create the request payload (OpenAI-compatible format)
        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "stream": false
        });

        // Send request to Qwen server
        let response = self.client
            .post(format!("{}/v1/chat/completions", self.config.server_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to send LLM request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("LLM request failed with status: {}", response.status()));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        let assistant_message = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let finish_reason = result["choices"][0]["finish_reason"]
            .as_str()
            .map(|s| s.to_string());

        // Add assistant response to history
        self.conversation_history.push(ChatMessage {
            role: "assistant".to_string(),
            content: assistant_message.clone(),
        });

        Ok(LLMResponse {
            text: assistant_message,
            finish_reason,
        })
    }

    /// Stream a response from the LLM
    pub async fn chat_stream<F>(&mut self, user_message: &str, mut on_chunk: F) -> Result<LLMResponse, String>
    where
        F: FnMut(&str),
    {
        // Add user message to history
        self.conversation_history.push(ChatMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        // Build messages array with system prompt
        let mut messages = vec![ChatMessage {
            role: "system".to_string(),
            content: self.config.system_prompt.clone(),
        }];
        messages.extend(self.conversation_history.clone());

        // Create the request payload
        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "stream": true
        });

        // Send streaming request
        let response = self.client
            .post(format!("{}/v1/chat/completions", self.config.server_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to send streaming LLM request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Streaming LLM request failed with status: {}", response.status()));
        }

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            
            // Parse SSE data
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        break;
                    }
                    
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            full_response.push_str(content);
                            on_chunk(content);
                        }
                    }
                }
            }
        }

        // Add assistant response to history
        self.conversation_history.push(ChatMessage {
            role: "assistant".to_string(),
            content: full_response.clone(),
        });

        Ok(LLMResponse {
            text: full_response,
            finish_reason: Some("stop".to_string()),
        })
    }

    /// Clear conversation history
    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }

    /// Get current configuration
    pub fn config(&self) -> &QwenConfig {
        &self.config
    }

    /// Update server URL
    pub fn set_server_url(&mut self, url: String) {
        self.config.server_url = url;
    }

    /// Update system prompt
    pub fn set_system_prompt(&mut self, prompt: String) {
        self.config.system_prompt = prompt;
    }
}
