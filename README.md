# Assidenter

A Tauri 2 voice assistant application built with Vue.js that runs **100% locally** using open-source AI models:

- **WhisperLiveKit** for ASR (Automatic Speech Recognition) - Local Whisper model
- **Qwen 0.5** as the LLM (Large Language Model) - Local GGUF model via llama.cpp
- **VoxCPM** for TTS (Text-to-Speech) - Local espeak-ng engine
- **Browser VAD** for Voice Activity Detection

## 🏠 100% Local - No Cloud APIs Required

All AI models run completely on your machine. No API keys needed, no data sent to external servers!

## 📱 Cross-Platform Support

This app runs on:
- **Desktop**: Windows, macOS, Linux
- **Mobile**: Android (API 24+)

## Features

- 🎤 Voice-based conversation with VAD (Voice Activity Detection)
- 🧠 AI-powered responses using local Qwen 0.5B model
- 🔊 Text-to-speech output using local espeak-ng
- ⌨️ Text input support for hybrid interaction
- ⚙️ Configurable service endpoints
- 🌙 Modern dark theme UI
- 🔒 Privacy-first: all processing happens locally
- 📱 Mobile-ready: Responsive UI for Android devices

## Quick Start

### 1. Download Local Models & Start Services

```bash
cd services

# Download all required local models (~500MB total)
./setup.sh

# Start all services with Docker Compose
docker-compose up -d
```

This starts local servers for:
- **WhisperLiveKit ASR** on port 9090 (local Whisper model)
- **Qwen 0.5B LLM** on port 8080 (local GGUF model)
- **VoxCPM TTS** on port 5500 (local espeak-ng)

See [services/README.md](services/README.md) for detailed setup instructions.

### 2. Install and Run the App

```bash
# Install dependencies
npm install

# Run in development mode (desktop)
npm run tauri dev
```

## Prerequisites

### Desktop Development
- **Node.js** 18+ and npm
- **Rust** 1.77+ (for Tauri)
- **Docker** and Docker Compose (for backend services)

### Android Development
- All desktop prerequisites, plus:
- **Android Studio** with SDK (API level 24+)
- **Android NDK** (install via Android Studio SDK Manager)
- **Java JDK** 17+
- Set environment variables:
  ```bash
  export ANDROID_HOME="$HOME/Android/Sdk"
  export NDK_HOME="$ANDROID_HOME/ndk/<version>"
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

**For Android**: Configure the server URLs to point to your backend server's IP address instead of localhost.

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

### Desktop Build

```bash
npm run tauri build
```

This creates distributable packages for your platform in `src-tauri/target/release/bundle/`.

### Android Build

1. **Initialize Android project** (first time only):
   ```bash
   npm run tauri android init
   ```

2. **Development on Android device/emulator**:
   ```bash
   npm run tauri android dev
   ```

3. **Build Android APK**:
   ```bash
   npm run tauri android build
   ```
   
   The APK will be generated at `src-tauri/gen/android/app/build/outputs/apk/`.

### Android Notes

- Ensure your Android device has USB debugging enabled
- For physical device testing, both the device and the backend services must be on the same network
- Update service URLs in the app settings to point to your server's LAN IP (e.g., `http://192.168.1.x:9090`)

## License

MIT
