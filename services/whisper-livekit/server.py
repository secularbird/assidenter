#!/usr/bin/env python3
"""
WhisperLiveKit ASR Server

A simple HTTP server that provides speech-to-text transcription
using OpenAI's Whisper model. Compatible with the Assidenter voice assistant.

Endpoints:
- POST /transcribe - Transcribe audio data (base64 encoded WAV)
- GET /health - Health check endpoint
"""

import os
import io
import base64
import logging
import tempfile
from flask import Flask, request, jsonify
from flask_cors import CORS
import whisper
import soundfile as sf
import numpy as np

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Initialize Flask app
app = Flask(__name__)
CORS(app)

# Load Whisper model
MODEL_NAME = os.environ.get('WHISPER_MODEL', 'base')
DEVICE = os.environ.get('DEVICE', 'cpu')

logger.info(f"Loading Whisper model: {MODEL_NAME} on {DEVICE}")
model = whisper.load_model(MODEL_NAME, device=DEVICE)
logger.info("Model loaded successfully")


@app.route('/health', methods=['GET'])
def health_check():
    """Health check endpoint"""
    return jsonify({
        'status': 'healthy',
        'model': MODEL_NAME,
        'device': DEVICE
    })


@app.route('/transcribe', methods=['POST'])
def transcribe():
    """
    Transcribe audio data to text.
    
    Request body (JSON):
    {
        "audio": "<base64 encoded audio data>",
        "language": "auto" | "<language code>",  # optional
        "model": "<model name>",  # optional, ignored (uses loaded model)
        "format": "wav" | "mp3" | "webm"  # optional, default: wav
    }
    
    Response (JSON):
    {
        "text": "<transcribed text>",
        "language": "<detected language>",
        "duration": <audio duration in seconds>
    }
    """
    try:
        data = request.get_json()
        
        if not data or 'audio' not in data:
            return jsonify({'error': 'Missing audio data'}), 400
        
        # Decode base64 audio
        try:
            audio_bytes = base64.b64decode(data['audio'])
        except Exception as e:
            logger.error(f"Failed to decode base64 audio: {e}")
            return jsonify({'error': 'Invalid base64 audio data'}), 400
        
        # Get optional parameters
        language = data.get('language', 'auto')
        if language == 'auto':
            language = None
        
        # Save audio to temporary file and load with soundfile
        with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as tmp_file:
            tmp_file.write(audio_bytes)
            tmp_path = tmp_file.name
        
        try:
            # Load audio file
            audio_data, sample_rate = sf.read(tmp_path)
            
            # Convert to mono if stereo
            if len(audio_data.shape) > 1:
                audio_data = audio_data.mean(axis=1)
            
            # Resample to 16kHz if needed (Whisper expects 16kHz)
            if sample_rate != 16000:
                # Simple resampling using numpy
                duration = len(audio_data) / sample_rate
                num_samples = int(duration * 16000)
                audio_data = np.interp(
                    np.linspace(0, len(audio_data), num_samples),
                    np.arange(len(audio_data)),
                    audio_data
                )
                sample_rate = 16000
            
            # Ensure float32
            audio_data = audio_data.astype(np.float32)
            
            # Calculate duration
            duration = len(audio_data) / sample_rate
            
            logger.info(f"Processing audio: {duration:.2f}s, {sample_rate}Hz")
            
            # Transcribe with Whisper
            result = model.transcribe(
                audio_data,
                language=language,
                fp16=False  # Use FP32 for CPU compatibility
            )
            
            transcribed_text = result['text'].strip()
            detected_language = result.get('language', 'unknown')
            
            logger.info(f"Transcription: '{transcribed_text}' (lang: {detected_language})")
            
            return jsonify({
                'text': transcribed_text,
                'language': detected_language,
                'duration': duration
            })
            
        finally:
            # Clean up temporary file
            os.unlink(tmp_path)
            
    except Exception as e:
        logger.error(f"Transcription error: {e}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@app.route('/', methods=['GET'])
def index():
    """Root endpoint with API information"""
    return jsonify({
        'name': 'WhisperLiveKit ASR Server',
        'version': '1.0.0',
        'endpoints': {
            '/health': 'GET - Health check',
            '/transcribe': 'POST - Transcribe audio to text'
        },
        'model': MODEL_NAME,
        'device': DEVICE
    })


if __name__ == '__main__':
    port = int(os.environ.get('PORT', 9090))
    host = os.environ.get('HOST', '0.0.0.0')
    
    logger.info(f"Starting WhisperLiveKit server on {host}:{port}")
    app.run(host=host, port=port, debug=False)
