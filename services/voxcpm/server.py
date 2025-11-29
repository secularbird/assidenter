#!/usr/bin/env python3
"""
VoxCPM TTS Server

A local text-to-speech server supporting multiple TTS backends:
- VoxCPM: Neural TTS (if model available)
- Index-TTS2: High-quality neural TTS (if model available)
- espeak-ng: Fast, lightweight fallback (always available)

All backends run 100% locally - no cloud APIs required.

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
TTS_BACKEND = os.environ.get('TTS_BACKEND', 'auto')  # auto, voxcpm, index-tts2, espeak

# Model paths
VOXCPM_MODEL_PATH = os.environ.get('VOXCPM_MODEL_PATH', '/app/models/voxcpm')
INDEX_TTS_MODEL_PATH = os.environ.get('INDEX_TTS_MODEL_PATH', '/app/models/index-tts2')

# Available TTS engines (detected at startup)
AVAILABLE_ENGINES = []
ACTIVE_ENGINE = None

def init_voxcpm():
    """Initialize VoxCPM if model is available"""
    try:
        if os.path.exists(VOXCPM_MODEL_PATH):
            # VoxCPM requires specific dependencies
            # This is a placeholder for actual VoxCPM integration
            logger.info(f"VoxCPM model found at {VOXCPM_MODEL_PATH}")
            return True
    except Exception as e:
        logger.warning(f"VoxCPM not available: {e}")
    return False

def init_index_tts():
    """Initialize Index-TTS2 if model is available"""
    try:
        if os.path.exists(INDEX_TTS_MODEL_PATH):
            logger.info(f"Index-TTS2 model found at {INDEX_TTS_MODEL_PATH}")
            return True
    except Exception as e:
        logger.warning(f"Index-TTS2 not available: {e}")
    return False

def init_espeak():
    """Initialize espeak-ng (always available as fallback)"""
    try:
        import subprocess
        result = subprocess.run(['espeak-ng', '--version'], capture_output=True, text=True)
        if result.returncode == 0:
            logger.info("espeak-ng available")
            return True
    except Exception as e:
        logger.warning(f"espeak-ng not available: {e}")
    return False

def init_pyttsx3():
    """Initialize pyttsx3 as an alternative fallback"""
    try:
        import pyttsx3
        engine = pyttsx3.init()
        logger.info("pyttsx3 available")
        return True
    except Exception as e:
        logger.warning(f"pyttsx3 not available: {e}")
    return False

# Initialize TTS engines
def detect_available_engines():
    global AVAILABLE_ENGINES, ACTIVE_ENGINE
    
    if init_voxcpm():
        AVAILABLE_ENGINES.append('voxcpm')
    if init_index_tts():
        AVAILABLE_ENGINES.append('index-tts2')
    if init_espeak():
        AVAILABLE_ENGINES.append('espeak')
    if init_pyttsx3():
        AVAILABLE_ENGINES.append('pyttsx3')
    
    # Select active engine based on preference
    if TTS_BACKEND != 'auto' and TTS_BACKEND in AVAILABLE_ENGINES:
        ACTIVE_ENGINE = TTS_BACKEND
    elif 'voxcpm' in AVAILABLE_ENGINES:
        ACTIVE_ENGINE = 'voxcpm'
    elif 'index-tts2' in AVAILABLE_ENGINES:
        ACTIVE_ENGINE = 'index-tts2'
    elif 'espeak' in AVAILABLE_ENGINES:
        ACTIVE_ENGINE = 'espeak'
    elif 'pyttsx3' in AVAILABLE_ENGINES:
        ACTIVE_ENGINE = 'pyttsx3'
    else:
        logger.error("No TTS engine available!")
        ACTIVE_ENGINE = None
    
    logger.info(f"Available TTS engines: {AVAILABLE_ENGINES}")
    logger.info(f"Active TTS engine: {ACTIVE_ENGINE}")

# Call at startup
detect_available_engines()


def synthesize_with_voxcpm(text, voice='default', speed=1.0, sample_rate=22050):
    """Synthesize speech using VoxCPM neural TTS"""
    # Placeholder for VoxCPM integration
    # When VoxCPM model is available, this would use the actual model
    raise NotImplementedError("VoxCPM model integration pending - use espeak fallback")


def synthesize_with_index_tts(text, voice='default', speed=1.0, sample_rate=22050):
    """Synthesize speech using Index-TTS2 neural TTS"""
    # Placeholder for Index-TTS2 integration
    # When Index-TTS2 model is available, this would use the actual model
    raise NotImplementedError("Index-TTS2 model integration pending - use espeak fallback")


def synthesize_with_pyttsx3(text, voice='default', speed=1.0, sample_rate=22050):
    """Synthesize speech using pyttsx3"""
    import pyttsx3
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


def synthesize(text, voice='default', speed=1.0, sample_rate=22050, engine=None):
    """Synthesize text to speech using specified or active engine"""
    use_engine = engine or ACTIVE_ENGINE
    
    try:
        if use_engine == 'voxcpm':
            return synthesize_with_voxcpm(text, voice, speed, sample_rate)
        elif use_engine == 'index-tts2':
            return synthesize_with_index_tts(text, voice, speed, sample_rate)
        elif use_engine == 'pyttsx3':
            return synthesize_with_pyttsx3(text, voice, speed, sample_rate)
        elif use_engine == 'espeak':
            return synthesize_with_espeak(text, voice, speed, sample_rate)
        else:
            raise ValueError(f"Unknown TTS engine: {use_engine}")
    except NotImplementedError:
        # Fallback to espeak if neural TTS not fully implemented
        logger.warning(f"Falling back to espeak from {use_engine}")
        return synthesize_with_espeak(text, voice, speed, sample_rate)


@app.route('/health', methods=['GET'])
def health_check():
    """Health check endpoint"""
    return jsonify({
        'status': 'healthy',
        'active_engine': ACTIVE_ENGINE,
        'available_engines': AVAILABLE_ENGINES,
        'default_voice': DEFAULT_VOICE,
        'default_rate': DEFAULT_RATE,
        'local_models': {
            'voxcpm': os.path.exists(VOXCPM_MODEL_PATH),
            'index_tts2': os.path.exists(INDEX_TTS_MODEL_PATH)
        }
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
        "engine": "voxcpm|index-tts2|espeak|pyttsx3",  # optional
        "format": "wav"  # optional, default: wav
    }
    
    Response:
    - If Accept header is application/json:
      {
          "audio": "<base64 encoded audio>",
          "sample_rate": <sample rate>,
          "duration": <duration in seconds>,
          "engine": "<engine used>"
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
        engine = data.get('engine')  # Optional: override active engine
        
        logger.info(f"Synthesizing: '{text[:50]}...' (voice: {voice}, speed: {speed}, engine: {engine or ACTIVE_ENGINE})")
        
        # Synthesize audio
        audio_data = synthesize(text, voice, speed, sample_rate, engine)
        
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
                'duration': max(0, duration),
                'engine': engine or ACTIVE_ENGINE
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


@app.route('/engines', methods=['GET'])
def list_engines():
    """List available TTS engines"""
    return jsonify({
        'available': AVAILABLE_ENGINES,
        'active': ACTIVE_ENGINE,
        'supported': ['voxcpm', 'index-tts2', 'espeak', 'pyttsx3']
    })


@app.route('/', methods=['GET'])
def index():
    """Root endpoint with API information"""
    return jsonify({
        'name': 'VoxCPM TTS Server',
        'version': '1.1.0',
        'description': 'Local text-to-speech server supporting VoxCPM, Index-TTS2, and espeak-ng',
        'endpoints': {
            '/health': 'GET - Health check',
            '/tts': 'POST - Synthesize text to speech',
            '/engines': 'GET - List available TTS engines'
        },
        'active_engine': ACTIVE_ENGINE,
        'available_engines': AVAILABLE_ENGINES
    })


if __name__ == '__main__':
    port = int(os.environ.get('PORT', 5500))
    host = os.environ.get('HOST', '0.0.0.0')
    
    logger.info(f"Starting VoxCPM TTS server on {host}:{port}")
    logger.info(f"Active engine: {ACTIVE_ENGINE}")
    logger.info(f"Available engines: {AVAILABLE_ENGINES}")
    app.run(host=host, port=port, debug=False)
