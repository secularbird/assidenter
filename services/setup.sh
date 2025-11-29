#!/bin/bash
# Assidenter Services Setup Script
# Downloads all required local models for the AI services

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODELS_DIR="$SCRIPT_DIR/models"

echo "=========================================="
echo "Assidenter Local Models Setup"
echo "=========================================="
echo ""
echo "This script downloads all required AI models to run locally."
echo "No cloud services or API keys required!"
echo ""

# Create models directory
mkdir -p "$MODELS_DIR"

# Download Qwen 0.5B GGUF model for LLM
echo "----------------------------------------"
echo "1. Downloading Qwen 0.5B LLM model..."
echo "----------------------------------------"
QWEN_MODEL="$MODELS_DIR/qwen2-0_5b-instruct-q4_k_m.gguf"
if [ -f "$QWEN_MODEL" ]; then
    echo "✓ Qwen model already exists: $QWEN_MODEL"
else
    echo "Downloading Qwen2-0.5B-Instruct (quantized Q4_K_M, ~400MB)..."
    if command -v wget &> /dev/null; then
        wget -q --show-progress -O "$QWEN_MODEL" \
            "https://huggingface.co/Qwen/Qwen2-0.5B-Instruct-GGUF/resolve/main/qwen2-0_5b-instruct-q4_k_m.gguf"
    elif command -v curl &> /dev/null; then
        curl -L --progress-bar -o "$QWEN_MODEL" \
            "https://huggingface.co/Qwen/Qwen2-0.5B-Instruct-GGUF/resolve/main/qwen2-0_5b-instruct-q4_k_m.gguf"
    else
        echo "Error: wget or curl is required to download models"
        exit 1
    fi
    echo "✓ Qwen model downloaded successfully"
fi

echo ""
echo "----------------------------------------"
echo "2. Whisper ASR model"
echo "----------------------------------------"
echo "The Whisper model (OpenAI's open-source speech recognition)"
echo "will be automatically downloaded on first use."
echo "Default: 'base' model (~140MB)"
echo "You can change this with WHISPER_MODEL environment variable:"
echo "  - tiny: ~40MB (fastest, less accurate)"
echo "  - base: ~140MB (recommended)"
echo "  - small: ~470MB (better accuracy)"
echo "  - medium: ~1.5GB (high accuracy)"
echo "  - large: ~3GB (best accuracy)"
echo ""

echo "----------------------------------------"
echo "3. TTS Engine"
echo "----------------------------------------"
echo "Using espeak-ng for text-to-speech (included in Docker image)."
echo "No additional downloads required."
echo ""

echo "=========================================="
echo "Setup Complete!"
echo "=========================================="
echo ""
echo "All models run 100% locally - no cloud APIs needed!"
echo ""
echo "To start the services:"
echo "  cd services"
echo "  docker-compose up -d"
echo ""
echo "Model locations:"
echo "  - LLM: $QWEN_MODEL"
echo "  - ASR: Downloaded on first use (~/.cache/whisper/)"
echo "  - TTS: Built-in (espeak-ng)"
echo ""
