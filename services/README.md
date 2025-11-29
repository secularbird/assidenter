# Assidenter Backend Services

This directory contains Docker configurations for the backend AI services required by the Assidenter voice assistant.

## Services

### 1. WhisperLiveKit (ASR - Speech-to-Text)
- **Port**: 9090
- **Model**: OpenAI Whisper (configurable: tiny, base, small, medium, large)
- **Endpoint**: `POST /transcribe`

### 2. Qwen 0.5B (LLM - Language Model)
- **Port**: 8080
- **Model**: Qwen2-0.5B-Instruct (GGUF format)
- **Endpoint**: OpenAI-compatible API

### 3. VoxCPM (TTS - Text-to-Speech)
- **Port**: 5500
- **Engine**: espeak-ng / pyttsx3
- **Endpoint**: `POST /tts`

## Quick Start

### Using Docker Compose (Recommended)

```bash
cd services

# Download the Qwen model (required for LLM)
mkdir -p models
# Download from: https://huggingface.co/Qwen/Qwen2-0.5B-Instruct-GGUF
# Place qwen2-0_5b-instruct-q4_k_m.gguf in the models directory

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

#### VoxCPM (TTS)

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
- `TTS_VOICE`: Default voice - default: default
- `TTS_RATE`: Speech rate - default: 150
- `PORT`: Server port - default: 5500

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

### VoxCPM TTS API

#### POST /tts
Synthesize text to speech.

**Request:**
```json
{
    "text": "Text to synthesize",
    "voice": "default",  // optional
    "speed": 1.0,        // optional
    "sample_rate": 22050 // optional
}
```

**Response (with Accept: application/json):**
```json
{
    "audio": "<base64 encoded WAV audio>",
    "sample_rate": 22050,
    "duration": 1.5
}
```

**Response (default):**
Raw WAV audio bytes

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
