# Assidenter Backend Services

This directory contains Docker configurations for the backend AI services required by the Assidenter voice assistant.

## 🏠 100% Local - No Cloud APIs Required

All AI models run completely locally on your machine:
- **ASR**: OpenAI Whisper (open-source, runs locally)
- **LLM**: Qwen 0.5B (open-source, runs locally via llama.cpp)
- **TTS**: VoxCPM / Index-TTS2 / espeak-ng (multiple options, all local)

No API keys, no cloud services, no data sent externally!

## Services

### 1. WhisperLiveKit (ASR - Speech-to-Text)
- **Port**: 9090
- **Model**: OpenAI Whisper (local, configurable: tiny, base, small, medium, large)
- **Endpoint**: `POST /transcribe`

### 2. Qwen 0.5B (LLM - Language Model)
- **Port**: 8080
- **Model**: Qwen2-0.5B-Instruct (local GGUF format via llama.cpp)
- **Endpoint**: OpenAI-compatible API

### 3. TTS Server (Text-to-Speech)
- **Port**: 5500
- **Supported Engines** (all local):
  - **VoxCPM**: Neural TTS (requires model download)
  - **Index-TTS2**: High-quality neural TTS (requires model download)
  - **espeak-ng**: Fast, lightweight (built-in, always available)
- **Endpoint**: `POST /tts`

## Quick Start

### 1. Download Local Models

```bash
cd services

# Run the setup script to download all local models
./setup.sh
```

This downloads:
- Qwen2-0.5B-Instruct GGUF (~400MB) for the LLM
- Whisper model downloads automatically on first use (~140MB for base)

### 2. Start Services with Docker Compose

```bash
# Start all services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

### Running Services Individually

#### WhisperLiveKit (ASR)

```bash
cd whisper-livekit

# Build and run with Docker
docker build -t whisper-livekit .
docker run -p 9090:9090 -e WHISPER_MODEL=base whisper-livekit

# Or run locally with Python
pip install -r requirements.txt
python server.py
```

Environment variables:
- `WHISPER_MODEL`: Model size (tiny, base, small, medium, large) - default: base
- `DEVICE`: Device to use (cpu, cuda) - default: cpu
- `PORT`: Server port - default: 9090

#### TTS Server (VoxCPM / Index-TTS2 / espeak)

```bash
cd voxcpm

# Build and run with Docker
docker build -t voxcpm-tts .
docker run -p 5500:5500 voxcpm-tts

# Or run locally with Python
pip install -r requirements.txt
python server.py
```

Environment variables:
- `TTS_BACKEND`: TTS engine (auto, voxcpm, index-tts2, espeak) - default: auto
- `TTS_VOICE`: Default voice - default: default
- `TTS_RATE`: Speech rate - default: 150
- `VOXCPM_MODEL_PATH`: Path to VoxCPM model - default: /app/models/voxcpm
- `INDEX_TTS_MODEL_PATH`: Path to Index-TTS2 model - default: /app/models/index-tts2
- `PORT`: Server port - default: 5500

**Note**: The server automatically detects available TTS engines and uses the best available option:
1. VoxCPM (if model installed)
2. Index-TTS2 (if model installed)
3. espeak-ng (always available as fallback)

#### Qwen LLM

The LLM uses the llama.cpp server. You need to download a GGUF model:

```bash
# Create models directory
mkdir -p models

# Download Qwen2-0.5B-Instruct GGUF model
# Option 1: From Hugging Face
wget https://huggingface.co/Qwen/Qwen2-0.5B-Instruct-GGUF/resolve/main/qwen2-0_5b-instruct-q4_k_m.gguf -O models/qwen2-0_5b-instruct-q4_k_m.gguf

# Option 2: Use huggingface-cli
pip install huggingface-hub
huggingface-cli download Qwen/Qwen2-0.5B-Instruct-GGUF qwen2-0_5b-instruct-q4_k_m.gguf --local-dir models

# Run with Docker
docker run -p 8080:8080 -v $(pwd)/models:/models ghcr.io/ggerganov/llama.cpp:server \
    -m /models/qwen2-0_5b-instruct-q4_k_m.gguf \
    --host 0.0.0.0 --port 8080 -c 2048 -ngl 0
```

## API Documentation

### WhisperLiveKit ASR API

#### POST /transcribe
Transcribe audio to text.

**Request:**
```json
{
    "audio": "<base64 encoded WAV audio>",
    "language": "auto",  // optional: auto, en, zh, etc.
    "format": "wav"      // optional: wav, mp3, webm
}
```

**Response:**
```json
{
    "text": "Transcribed text here",
    "language": "en",
    "duration": 2.5
}
```

### TTS API

#### POST /tts
Synthesize text to speech.

**Request:**
```json
{
    "text": "Text to synthesize",
    "voice": "default",  // optional
    "speed": 1.0,        // optional
    "sample_rate": 22050, // optional
    "engine": "voxcpm"   // optional: voxcpm, index-tts2, espeak
}
```

**Response (with Accept: application/json):**
```json
{
    "audio": "<base64 encoded WAV audio>",
    "sample_rate": 22050,
    "duration": 1.5,
    "engine": "espeak"
}
```

**Response (default):**
Raw WAV audio bytes

#### GET /engines
List available TTS engines.

**Response:**
```json
{
    "available": ["espeak", "pyttsx3"],
    "active": "espeak",
    "supported": ["voxcpm", "index-tts2", "espeak", "pyttsx3"]
}
```

### Qwen LLM API (OpenAI-compatible)

#### POST /v1/chat/completions
Generate chat completions.

**Request:**
```json
{
    "model": "qwen-0.5b",
    "messages": [
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello!"}
    ],
    "temperature": 0.7,
    "max_tokens": 512
}
```

**Response:**
```json
{
    "choices": [
        {
            "message": {
                "role": "assistant",
                "content": "Hello! How can I help you today?"
            },
            "finish_reason": "stop"
        }
    ]
}
```

## Troubleshooting

### Whisper model download is slow
The first run will download the Whisper model. Use `WHISPER_MODEL=tiny` for faster downloads during testing.

### Out of memory errors
- Use smaller Whisper models: tiny < base < small < medium < large
- Use quantized LLM models (q4_k_m is recommended)
- Reduce context size for LLM: `-c 1024`

### GPU acceleration
To use GPU acceleration:
1. Install NVIDIA Container Toolkit
2. Use CUDA-enabled Docker images
3. Set `DEVICE=cuda` for Whisper
4. Set `-ngl 99` for llama.cpp to offload layers to GPU

## License

MIT
