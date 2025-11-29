#!/usr/bin/env python3
"""
VoxCPM-Compatible TTS Server

A simple HTTP server that provides text-to-speech synthesis.
Uses pyttsx3/espeak as a fallback TTS engine for compatibility.

Endpoints:
- POST /tts - Synthesize text to speech
- GET /health - Health check endpoint
"""

import os
import io
import base64
import logging
import tempfile
import wave
import struct
from flask import Flask, request, jsonify, Response
from flask_cors import CORS

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Initialize Flask app
app = Flask(__name__)
CORS(app)

# TTS configuration
DEFAULT_VOICE = os.environ.get('TTS_VOICE', 'default')
DEFAULT_RATE = int(os.environ.get('TTS_RATE', 150))
DEFAULT_SAMPLE_RATE = int(os.environ.get('TTS_SAMPLE_RATE', 22050))

# Try to import pyttsx3
try:
    import pyttsx3
    TTS_ENGINE = 'pyttsx3'
    logger.info("Using pyttsx3 TTS engine")
except ImportError:
    TTS_ENGINE = 'espeak'
    logger.info("Using espeak TTS engine (pyttsx3 not available)")


def synthesize_with_pyttsx3(text, voice='default', speed=1.0, sample_rate=22050):
    """Synthesize speech using pyttsx3"""
    engine = pyttsx3.init()
    
    # Set properties
    rate = int(DEFAULT_RATE * speed)
    engine.setProperty('rate', rate)
    
    # Get available voices
    voices = engine.getProperty('voices')
    if voices and voice != 'default':
        for v in voices:
            if voice.lower() in v.name.lower():
                engine.setProperty('voice', v.id)
                break
    
    # Save to temporary file
    with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as tmp_file:
        tmp_path = tmp_file.name
    
    try:
        engine.save_to_file(text, tmp_path)
        engine.runAndWait()
        
        # Read the generated audio
        with open(tmp_path, 'rb') as f:
            audio_data = f.read()
        
        return audio_data
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)


def synthesize_with_espeak(text, voice='default', speed=1.0, sample_rate=22050):
    """Synthesize speech using espeak-ng directly"""
    import subprocess
    
    # Calculate words per minute (espeak default is 175)
    wpm = int(175 * speed)
    
    with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as tmp_file:
        tmp_path = tmp_file.name
    
    try:
        # Run espeak-ng
        cmd = [
            'espeak-ng',
            '-w', tmp_path,
            '-s', str(wpm),
            text
        ]
        
        if voice != 'default':
            cmd.extend(['-v', voice])
        
        subprocess.run(cmd, check=True, capture_output=True)
        
        # Read the generated audio
        with open(tmp_path, 'rb') as f:
            audio_data = f.read()
        
        return audio_data
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)


def synthesize(text, voice='default', speed=1.0, sample_rate=22050):
    """Synthesize text to speech using available engine"""
    if TTS_ENGINE == 'pyttsx3':
        return synthesize_with_pyttsx3(text, voice, speed, sample_rate)
    else:
        return synthesize_with_espeak(text, voice, speed, sample_rate)


@app.route('/health', methods=['GET'])
def health_check():
    """Health check endpoint"""
    return jsonify({
        'status': 'healthy',
        'engine': TTS_ENGINE,
        'default_voice': DEFAULT_VOICE,
        'default_rate': DEFAULT_RATE
    })


@app.route('/tts', methods=['POST'])
def tts():
    """
    Synthesize text to speech.
    
    Request body (JSON):
    {
        "text": "<text to synthesize>",
        "voice": "<voice name>",  # optional
        "speed": <speed multiplier>,  # optional, default: 1.0
        "sample_rate": <sample rate>,  # optional, default: 22050
        "format": "wav"  # optional, default: wav
    }
    
    Response:
    - If Accept header is application/json:
      {
          "audio": "<base64 encoded audio>",
          "sample_rate": <sample rate>,
          "duration": <duration in seconds>
      }
    - Otherwise: Raw WAV audio bytes
    """
    try:
        data = request.get_json()
        
        if not data or 'text' not in data:
            return jsonify({'error': 'Missing text'}), 400
        
        text = data['text']
        if not text.strip():
            return jsonify({'error': 'Empty text'}), 400
        
        # Get optional parameters
        voice = data.get('voice', DEFAULT_VOICE)
        speed = float(data.get('speed', 1.0))
        sample_rate = int(data.get('sample_rate', DEFAULT_SAMPLE_RATE))
        
        logger.info(f"Synthesizing: '{text[:50]}...' (voice: {voice}, speed: {speed})")
        
        # Synthesize audio
        audio_data = synthesize(text, voice, speed, sample_rate)
        
        # Check if client wants JSON response
        accept_header = request.headers.get('Accept', '')
        if 'application/json' in accept_header:
            # Return as JSON with base64 encoded audio
            audio_base64 = base64.b64encode(audio_data).decode('utf-8')
            
            # Estimate duration (assuming 16-bit mono WAV)
            # Duration = (file_size - 44 header) / (sample_rate * 2 bytes per sample)
            duration = (len(audio_data) - 44) / (sample_rate * 2)
            
            return jsonify({
                'audio': audio_base64,
                'sample_rate': sample_rate,
                'duration': max(0, duration)
            })
        else:
            # Return raw audio bytes
            return Response(
                audio_data,
                mimetype='audio/wav',
                headers={
                    'Content-Disposition': 'attachment; filename=speech.wav'
                }
            )
            
    except Exception as e:
        logger.error(f"TTS error: {e}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@app.route('/', methods=['GET'])
def index():
    """Root endpoint with API information"""
    return jsonify({
        'name': 'VoxCPM-Compatible TTS Server',
        'version': '1.0.0',
        'endpoints': {
            '/health': 'GET - Health check',
            '/tts': 'POST - Synthesize text to speech'
        },
        'engine': TTS_ENGINE,
        'default_voice': DEFAULT_VOICE
    })


if __name__ == '__main__':
    port = int(os.environ.get('PORT', 5500))
    host = os.environ.get('HOST', '0.0.0.0')
    
    logger.info(f"Starting TTS server on {host}:{port}")
    app.run(host=host, port=port, debug=False)
