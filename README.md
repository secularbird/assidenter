# Assidenter

A Tauri 2 voice assistant application built with Vue.js that integrates:

- **WhisperLiveKit** for ASR (Automatic Speech Recognition)
- **Qwen 0.5** as the LLM (Large Language Model)
- **VoxCPM** for TTS (Text-to-Speech)
- **WebRTC VAD** for Voice Activity Detection

## Features

- 🎤 Voice-based conversation with VAD (Voice Activity Detection)
- 🧠 AI-powered responses using Qwen 0.5B
- 🔊 Text-to-speech output using VoxCPM
- ⌨️ Text input support for hybrid interaction
- ⚙️ Configurable service endpoints
- 🌙 Modern dark theme UI

## Prerequisites

Before running the application, ensure you have the following services running:

### 1. WhisperLiveKit (ASR Server)
Default URL: `http://localhost:9090`

```bash
# Install and run WhisperLiveKit
# See: https://github.com/collabora/WhisperLiveKit
```

### 2. Qwen 0.5B (LLM Server)
Default URL: `http://localhost:8080`

```bash
# Run Qwen 0.5B with OpenAI-compatible API
# Can use llama.cpp, vLLM, or similar serving frameworks
```

### 3. VoxCPM (TTS Server)
Default URL: `http://localhost:5500`

```bash
# Install and run VoxCPM TTS server
# See: https://github.com/OpenBMB/VoxCPM
```

## Installation

### Install dependencies

```bash
npm install
```

### Development

```bash
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

## Project Structure

```
assidenter/
├── src/                    # Vue frontend source
│   ├── App.vue            # Main app component
│   ├── components/        # Vue components
│   │   └── VoiceAssistant.vue
│   ├── main.js            # Vue entry point
│   └── style.css          # Global styles
├── src-tauri/             # Tauri/Rust backend
│   ├── src/
│   │   ├── lib.rs         # Main Tauri app logic
│   │   ├── main.rs        # Entry point
│   │   ├── audio/         # Audio capture & VAD modules
│   │   │   ├── capture.rs
│   │   │   ├── vad.rs
│   │   │   └── mod.rs
│   │   └── services/      # AI service integrations
│   │       ├── asr.rs     # WhisperLiveKit client
│   │       ├── llm.rs     # Qwen LLM client
│   │       ├── tts.rs     # VoxCPM TTS client
│   │       └── mod.rs
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── package.json           # Node.js dependencies
└── vite.config.js         # Vite configuration
```

## Usage

1. Start the required backend services (WhisperLiveKit, Qwen, VoxCPM)
2. Run the application with `npm run tauri dev`
3. Click the microphone button to start voice interaction
4. Speak your question - the VAD will detect when you start/stop speaking
5. Wait for the AI to process your speech and respond
6. Alternatively, type messages in the text input field

## Configuration

Click the ⚙️ button in the UI to configure service endpoints:

- **ASR Server**: WhisperLiveKit endpoint
- **LLM Server**: Qwen 0.5B API endpoint
- **TTS Server**: VoxCPM TTS endpoint

## Technical Details

### Voice Activity Detection (VAD)

The application uses WebRTC VAD for detecting speech in the audio stream:
- Analyzes audio in 30ms frames at 16kHz
- Requires 3 consecutive speech frames to trigger start
- Requires 10 consecutive silence frames to trigger end

### Audio Pipeline

1. **Capture**: Microphone input at 16kHz mono
2. **VAD**: Detect speech segments
3. **ASR**: Transcribe speech to text using WhisperLiveKit
4. **LLM**: Generate response using Qwen 0.5B
5. **TTS**: Synthesize speech using VoxCPM
6. **Playback**: Play audio response

## License

MIT
