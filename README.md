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

## Quick Start

### 1. Start Backend Services

The backend AI services are included in the `services/` directory.

```bash
cd services

# Download the Qwen model first
mkdir -p models
# Download qwen2-0_5b-instruct-q4_k_m.gguf from:
# https://huggingface.co/Qwen/Qwen2-0.5B-Instruct-GGUF

# Start all services with Docker Compose
docker-compose up -d
```

This starts:
- **WhisperLiveKit ASR** on port 9090
- **Qwen 0.5B LLM** on port 8080
- **VoxCPM TTS** on port 5500

See [services/README.md](services/README.md) for detailed setup instructions.

### 2. Install and Run the App

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

## Prerequisites

- **Node.js** 18+ and npm
- **Rust** 1.77+ (for Tauri)
- **Docker** and Docker Compose (for backend services)

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
│   │   └── services/      # AI service integrations
│   │       ├── asr.rs     # WhisperLiveKit client
│   │       ├── llm.rs     # Qwen LLM client
│   │       ├── tts.rs     # VoxCPM TTS client
│   │       └── mod.rs
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── services/              # Backend AI services
│   ├── docker-compose.yml # Docker Compose config
│   ├── whisper-livekit/   # ASR server
│   ├── voxcpm/            # TTS server
│   └── README.md          # Services documentation
├── package.json           # Node.js dependencies
└── vite.config.js         # Vite configuration
```

## Usage

1. Start the backend services (see Quick Start above)
2. Run the application with `npm run tauri dev`
3. Click the microphone button to start voice interaction
4. Speak your question - the VAD will detect when you start/stop speaking
5. Wait for the AI to process your speech and respond
6. Alternatively, type messages in the text input field

## Configuration

Click the ⚙️ button in the UI to configure service endpoints:

- **ASR Server**: WhisperLiveKit endpoint (default: http://localhost:9090)
- **LLM Server**: Qwen 0.5B API endpoint (default: http://localhost:8080)
- **TTS Server**: VoxCPM TTS endpoint (default: http://localhost:5500)

## Technical Details

### Voice Activity Detection (VAD)

The application uses browser-based VAD for detecting speech:
- Analyzes audio using Web Audio API frequency analysis
- Configurable silence threshold and duration
- Automatically stops recording after speech ends

### Audio Pipeline

1. **Capture**: Microphone input via browser MediaRecorder
2. **VAD**: Detect speech segments using frequency analysis
3. **ASR**: Transcribe speech to text using WhisperLiveKit
4. **LLM**: Generate response using Qwen 0.5B
5. **TTS**: Synthesize speech using VoxCPM
6. **Playback**: Play audio response in browser

## Building for Production

```bash
npm run tauri build
```

This creates distributable packages for your platform in `src-tauri/target/release/bundle/`.

## License

MIT
