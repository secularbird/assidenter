pub mod asr;
pub mod llm;
pub mod tts;

#[cfg(feature = "embedded-services")]
pub mod embedded;

pub use asr::WhisperLiveKit;
pub use llm::QwenLLM;
pub use tts::VoxCPMTTS;

// Service mode configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceMode {
    /// Use remote HTTP services
    Remote,
    /// Use embedded on-device inference (for mobile)
    Embedded,
}

impl Default for ServiceMode {
    fn default() -> Self {
        #[cfg(feature = "embedded-services")]
        return ServiceMode::Embedded;
        
        #[cfg(not(feature = "embedded-services"))]
        return ServiceMode::Remote;
    }
}
