<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// State
const isListening = ref(false)
const isSpeaking = ref(false)
const isProcessing = ref(false)
const processingStatus = ref('')
const messages = ref([])
const textInput = ref('')
const audioContext = ref(null)

// Screenshot state
const isCapturing = ref(false)
const screenshotImage = ref(null)
const showScreenshot = ref(false)

// Audio capture state
let mediaRecorder = null
let audioChunks = []
let vadTimeout = null
let silenceStart = null

// Audio format constants
const INT16_MAX = 0x7FFF  // Maximum value for signed 16-bit integer
const INT16_MIN_ABS = 0x8000  // Absolute value of minimum signed 16-bit integer

// Settings (including configurable VAD parameters)
const settings = ref({
  asrUrl: 'http://localhost:9090',
  llmUrl: 'http://localhost:8080',
  ttsUrl: 'http://localhost:5500',
  // VAD configuration
  silenceThreshold: 0.01,  // Audio level threshold for voice detection
  silenceDuration: 1500,   // Milliseconds of silence to end recording
  minRecordingDuration: 500  // Minimum recording time in milliseconds
})
const showSettings = ref(false)

// Event unlisteners
let unlisteners = []

// Start listening for voice input
async function startListening() {
  try {
    await invoke('start_listening')
    isListening.value = true
    
    // Start audio capture
    await startAudioCapture()
  } catch (error) {
    console.error('Failed to start listening:', error)
    addMessage('system', `Error: ${error}`)
  }
}

// Start audio capture with VAD
async function startAudioCapture() {
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ 
      audio: {
        sampleRate: 16000,
        channelCount: 1,
        echoCancellation: true,
        noiseSuppression: true
      } 
    })
    
    // Create audio context for VAD analysis
    if (!audioContext.value) {
      audioContext.value = new (window.AudioContext || window.webkitAudioContext)()
    }
    
    const source = audioContext.value.createMediaStreamSource(stream)
    const analyser = audioContext.value.createAnalyser()
    analyser.fftSize = 2048
    source.connect(analyser)
    
    // Determine best supported audio format
    const preferredMimeTypes = [
      'audio/webm;codecs=opus',
      'audio/webm',
      'audio/ogg;codecs=opus',
      'audio/mp4'
    ]
    let selectedMimeType = ''
    for (const mimeType of preferredMimeTypes) {
      if (MediaRecorder.isTypeSupported(mimeType)) {
        selectedMimeType = mimeType
        break
      }
    }
    
    // Create media recorder with supported format
    const recorderOptions = selectedMimeType ? { mimeType: selectedMimeType } : {}
    mediaRecorder = new MediaRecorder(stream, recorderOptions)
    
    audioChunks = []
    let recordingStartTime = null
    
    mediaRecorder.ondataavailable = (event) => {
      if (event.data.size > 0) {
        audioChunks.push(event.data)
      }
    }
    
    mediaRecorder.onstop = async () => {
      if (audioChunks.length > 0) {
        const audioBlob = new Blob(audioChunks, { type: selectedMimeType || 'audio/webm' })
        await processRecording(audioBlob)
      }
      audioChunks = []
    }
    
    // VAD using analyser with configurable thresholds
    const bufferLength = analyser.frequencyBinCount
    const dataArray = new Uint8Array(bufferLength)
    
    function checkVoiceActivity() {
      if (!isListening.value) return
      
      analyser.getByteFrequencyData(dataArray)
      
      // Calculate average volume
      let sum = 0
      for (let i = 0; i < bufferLength; i++) {
        sum += dataArray[i]
      }
      const average = sum / bufferLength / 255
      
      const now = Date.now()
      
      if (average > settings.value.silenceThreshold) {
        // Voice detected
        silenceStart = null
        
        if (!isSpeaking.value) {
          isSpeaking.value = true
          recordingStartTime = now
          mediaRecorder.start(100) // Collect data every 100ms
          console.log('Speech started')
        }
      } else if (isSpeaking.value) {
        // Silence detected while speaking
        if (!silenceStart) {
          silenceStart = now
        } else if (now - silenceStart > settings.value.silenceDuration) {
          // End of speech
          const duration = now - recordingStartTime
          if (duration > settings.value.minRecordingDuration) {
            isSpeaking.value = false
            mediaRecorder.stop()
            console.log('Speech ended, duration:', duration)
          }
          silenceStart = null
        }
      }
      
      if (isListening.value) {
        vadTimeout = requestAnimationFrame(checkVoiceActivity)
      }
    }
    
    checkVoiceActivity()
    addMessage('system', 'Listening... speak now')
    
  } catch (error) {
    console.error('Failed to start audio capture:', error)
    addMessage('system', `Microphone access error: ${error.message}. Please ensure microphone permissions are granted.`)
    isListening.value = false
  }
}

// Process recorded audio
async function processRecording(audioBlob) {
  isProcessing.value = true
  processingStatus.value = 'Processing audio...'
  
  try {
    // Convert to WAV format
    const wavBlob = await convertToWav(audioBlob)
    
    // Convert to base64
    const base64Audio = await blobToBase64(wavBlob)
    
    // Send to backend
    const result = await invoke('process_audio', {
      audioBase64: base64Audio
    })
    
    if (result.transcription) {
      addMessage('user', result.transcription)
    }
    if (result.response) {
      addMessage('assistant', result.response)
    }
  } catch (error) {
    console.error('Failed to process recording:', error)
    addMessage('system', `Error: ${error}`)
  } finally {
    isProcessing.value = false
    processingStatus.value = ''
  }
}

// Convert audio blob to WAV format
async function convertToWav(blob) {
  const arrayBuffer = await blob.arrayBuffer()
  
  if (!audioContext.value) {
    audioContext.value = new (window.AudioContext || window.webkitAudioContext)()
  }
  
  const audioBuffer = await audioContext.value.decodeAudioData(arrayBuffer)
  
  // Convert to 16-bit PCM WAV
  const numberOfChannels = 1
  const sampleRate = 16000
  const length = Math.floor(audioBuffer.duration * sampleRate)
  
  // Resample to 16kHz if needed
  let samples
  if (audioBuffer.sampleRate !== sampleRate) {
    const offlineContext = new OfflineAudioContext(numberOfChannels, length, sampleRate)
    const bufferSource = offlineContext.createBufferSource()
    bufferSource.buffer = audioBuffer
    bufferSource.connect(offlineContext.destination)
    bufferSource.start(0)
    const resampled = await offlineContext.startRendering()
    samples = resampled.getChannelData(0)
  } else {
    samples = audioBuffer.getChannelData(0)
  }
  
  // Create WAV buffer
  const wavBuffer = new ArrayBuffer(44 + samples.length * 2)
  const view = new DataView(wavBuffer)
  
  // WAV header
  writeString(view, 0, 'RIFF')
  view.setUint32(4, 36 + samples.length * 2, true)
  writeString(view, 8, 'WAVE')
  writeString(view, 12, 'fmt ')
  view.setUint32(16, 16, true) // Subchunk1Size
  view.setUint16(20, 1, true) // AudioFormat (PCM)
  view.setUint16(22, numberOfChannels, true)
  view.setUint32(24, sampleRate, true)
  view.setUint32(28, sampleRate * numberOfChannels * 2, true) // ByteRate
  view.setUint16(32, numberOfChannels * 2, true) // BlockAlign
  view.setUint16(34, 16, true) // BitsPerSample
  writeString(view, 36, 'data')
  view.setUint32(40, samples.length * 2, true)
  
  // Write audio data - convert float samples to 16-bit signed integers
  const offset = 44
  for (let i = 0; i < samples.length; i++) {
    const sample = Math.max(-1, Math.min(1, samples[i]))
    // Scale to 16-bit range using named constants
    view.setInt16(offset + i * 2, sample < 0 ? sample * INT16_MIN_ABS : sample * INT16_MAX, true)
  }
  
  return new Blob([wavBuffer], { type: 'audio/wav' })
}

function writeString(view, offset, string) {
  for (let i = 0; i < string.length; i++) {
    view.setUint8(offset + i, string.charCodeAt(i))
  }
}

// Convert blob to base64
function blobToBase64(blob) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const base64 = reader.result.split(',')[1]
      resolve(base64)
    }
    reader.onerror = reject
    reader.readAsDataURL(blob)
  })
}

// Stop listening
async function stopListening() {
  try {
    if (vadTimeout) {
      cancelAnimationFrame(vadTimeout)
      vadTimeout = null
    }
    
    if (mediaRecorder && mediaRecorder.state !== 'inactive') {
      mediaRecorder.stop()
    }
    
    await invoke('stop_listening')
    isListening.value = false
    isSpeaking.value = false
  } catch (error) {
    console.error('Failed to stop listening:', error)
  }
}

// Toggle listening state
function toggleListening() {
  if (isListening.value) {
    stopListening()
  } else {
    startListening()
  }
}

// Send text message
async function sendTextMessage() {
  if (!textInput.value.trim()) return
  
  const message = textInput.value.trim()
  textInput.value = ''
  addMessage('user', message)
  isProcessing.value = true
  
  try {
    const result = await invoke('send_text_message', { message })
    if (result.response) {
      addMessage('assistant', result.response)
    }
  } catch (error) {
    console.error('Failed to send message:', error)
    addMessage('system', `Error: ${error}`)
  } finally {
    isProcessing.value = false
    processingStatus.value = ''
  }
}

// Clear conversation
async function clearConversation() {
  try {
    await invoke('clear_conversation')
    messages.value = []
    addMessage('system', 'Conversation cleared')
  } catch (error) {
    console.error('Failed to clear conversation:', error)
  }
}

// Save settings
async function saveSettings() {
  try {
    await invoke('configure_services', {
      config: {
        asr_url: settings.value.asrUrl,
        llm_url: settings.value.llmUrl,
        tts_url: settings.value.ttsUrl
      }
    })
    showSettings.value = false
    addMessage('system', 'Settings saved')
  } catch (error) {
    console.error('Failed to save settings:', error)
    addMessage('system', `Error saving settings: ${error}`)
  }
}

// Add a message to the chat
function addMessage(role, content) {
  messages.value.push({
    id: Date.now(),
    role,
    content,
    timestamp: new Date().toLocaleTimeString()
  })
  // Auto-scroll to bottom
  setTimeout(() => {
    const container = document.querySelector('.messages-container')
    if (container) {
      container.scrollTop = container.scrollHeight
    }
  }, 100)
}

// Play audio from base64 data
async function playAudio(base64Data) {
  try {
    if (!audioContext.value) {
      audioContext.value = new (window.AudioContext || window.webkitAudioContext)()
    }
    
    // Decode base64 to array buffer
    const binaryString = atob(base64Data)
    const bytes = new Uint8Array(binaryString.length)
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i)
    }
    
    // Decode audio data
    const audioBuffer = await audioContext.value.decodeAudioData(bytes.buffer)
    
    // Create and play source
    const source = audioContext.value.createBufferSource()
    source.buffer = audioBuffer
    source.connect(audioContext.value.destination)
    source.start(0)
  } catch (error) {
    console.error('Failed to play audio:', error)
  }
}

// Take a screenshot
async function takeScreenshot() {
  if (isCapturing.value) return
  
  isCapturing.value = true
  try {
    const result = await invoke('take_screenshot', { monitorIndex: null })
    
    if (result.success && result.image_base64) {
      screenshotImage.value = `data:image/png;base64,${result.image_base64}`
      showScreenshot.value = true
      addMessage('system', `Screenshot captured (${result.width}x${result.height})`)
    } else {
      addMessage('system', `Screenshot failed: ${result.error || 'Unknown error'}`)
    }
  } catch (error) {
    console.error('Failed to take screenshot:', error)
    addMessage('system', `Screenshot error: ${error}`)
  } finally {
    isCapturing.value = false
  }
}

// Close screenshot modal
function closeScreenshot() {
  showScreenshot.value = false
}

// Download screenshot
function downloadScreenshot() {
  if (!screenshotImage.value) return
  
  const link = document.createElement('a')
  link.href = screenshotImage.value
  link.download = `screenshot-${Date.now()}.png`
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
}

// Setup event listeners
onMounted(async () => {
  // Listen for backend events
  unlisteners.push(
    await listen('processing-status', (event) => {
      processingStatus.value = event.payload
    }),
    await listen('transcription', (event) => {
      // Transcription handled in processRecording
    }),
    await listen('llm-response', (event) => {
      // Response handled in processRecording
    }),
    await listen('tts-audio', (event) => {
      playAudio(event.payload)
    }),
    await listen('processing-error', (event) => {
      isProcessing.value = false
      processingStatus.value = ''
      addMessage('system', `Error: ${event.payload}`)
    })
  )
  
  addMessage('system', 'Welcome to Assidenter! Click the microphone button to start talking.')
})

// Cleanup event listeners
onUnmounted(() => {
  unlisteners.forEach(unlisten => unlisten())
  if (vadTimeout) {
    cancelAnimationFrame(vadTimeout)
  }
})
</script>

<template>
  <div class="voice-assistant">
    <header class="header">
      <h1>🎤 Assidenter</h1>
      <p class="subtitle">Voice Assistant powered by WhisperLiveKit + Qwen 0.5 + VoxCPM</p>
      <div class="header-actions">
        <button 
          class="btn-icon" 
          @click="takeScreenshot" 
          :disabled="isCapturing"
          title="Take screenshot"
        >
          📷
        </button>
        <button class="btn-icon" @click="clearConversation" title="Clear conversation">
          🗑️
        </button>
        <button class="btn-icon" @click="showSettings = !showSettings" title="Settings">
          ⚙️
        </button>
      </div>
    </header>

    <!-- Screenshot Modal -->
    <div v-if="showScreenshot" class="screenshot-modal" @click.self="closeScreenshot">
      <div class="screenshot-container">
        <div class="screenshot-header">
          <h3>Screenshot</h3>
          <div class="screenshot-actions">
            <button class="btn-secondary" @click="downloadScreenshot" title="Download">
              💾 Download
            </button>
            <button class="btn-icon" @click="closeScreenshot" title="Close">
              ✕
            </button>
          </div>
        </div>
        <div class="screenshot-preview">
          <img :src="screenshotImage" alt="Screenshot" />
        </div>
      </div>
    </div>

    <!-- Settings Panel -->
    <div v-if="showSettings" class="settings-panel">
      <h3>Service Configuration</h3>
      <div class="settings-form">
        <div class="form-group">
          <label>ASR Server (WhisperLiveKit)</label>
          <input v-model="settings.asrUrl" placeholder="http://localhost:9090" />
        </div>
        <div class="form-group">
          <label>LLM Server (Qwen 0.5)</label>
          <input v-model="settings.llmUrl" placeholder="http://localhost:8080" />
        </div>
        <div class="form-group">
          <label>TTS Server (VoxCPM)</label>
          <input v-model="settings.ttsUrl" placeholder="http://localhost:5500" />
        </div>
        <div class="settings-actions">
          <button class="btn-secondary" @click="showSettings = false">Cancel</button>
          <button class="btn-primary" @click="saveSettings">Save</button>
        </div>
      </div>
    </div>

    <!-- Messages with ARIA live region for screen readers -->
    <div class="messages-container" role="log" aria-live="polite" aria-label="Conversation messages">
      <div
        v-for="msg in messages"
        :key="msg.id"
        :class="['message', `message-${msg.role}`]"
        :aria-label="`${msg.role}: ${msg.content}`"
      >
        <div class="message-content">
          <span class="message-role" aria-hidden="true">{{ msg.role === 'user' ? '🧑' : msg.role === 'assistant' ? '🤖' : '⚙️' }}</span>
          <p>{{ msg.content }}</p>
        </div>
        <span class="message-time">{{ msg.timestamp }}</span>
      </div>
    </div>

    <!-- Status Indicator with ARIA live region -->
    <div v-if="processingStatus" class="status-indicator" role="status" aria-live="assertive">
      <div class="spinner" aria-hidden="true"></div>
      <span>{{ processingStatus }}</span>
    </div>

    <!-- Input Area -->
    <div class="input-area">
      <div class="voice-button-container">
        <button
          class="voice-button"
          :class="{ listening: isListening, speaking: isSpeaking }"
          @click="toggleListening"
          :disabled="isProcessing"
          :aria-label="isProcessing ? 'Processing, please wait' : (isListening ? (isSpeaking ? 'Recording speech' : 'Listening for speech, click to stop') : 'Click to start listening')"
          :aria-pressed="isListening"
        >
          <span v-if="isListening && isSpeaking" aria-hidden="true">🎤</span>
          <span v-else-if="isListening" aria-hidden="true">👂</span>
          <span v-else aria-hidden="true">🎙️</span>
        </button>
        <p class="voice-status" aria-live="polite">
          {{ isProcessing ? 'Processing...' : (isListening ? (isSpeaking ? 'Speaking...' : 'Listening...') : 'Click to start') }}
        </p>
      </div>
      
      <div class="text-input-container">
        <input
          v-model="textInput"
          placeholder="Or type a message..."
          @keyup.enter="sendTextMessage"
          :disabled="isProcessing"
          aria-label="Type a message"
        />
        <button
          class="btn-send"
          @click="sendTextMessage"
          :disabled="!textInput.trim() || isProcessing"
          aria-label="Send message"
        >
          Send
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.voice-assistant {
  display: flex;
  flex-direction: column;
  height: 100vh;
  max-height: 100vh;
  padding: 1rem;
}

.header {
  text-align: center;
  padding: 1rem 0;
  border-bottom: 1px solid #2a2a4e;
  position: relative;
}

.header h1 {
  font-size: 1.8rem;
  margin-bottom: 0.5rem;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle {
  color: #888;
  font-size: 0.9rem;
}

.header-actions {
  position: absolute;
  top: 1rem;
  right: 0;
  display: flex;
  gap: 0.5rem;
}

.btn-icon {
  background: transparent;
  border: none;
  font-size: 1.2rem;
  cursor: pointer;
  padding: 0.5rem;
  border-radius: 50%;
  transition: background 0.2s;
}

.btn-icon:hover {
  background: rgba(255, 255, 255, 0.1);
}

.settings-panel {
  background: #1a1a3e;
  border-radius: 12px;
  padding: 1.5rem;
  margin: 1rem 0;
}

.settings-panel h3 {
  margin-bottom: 1rem;
  color: #667eea;
}

.settings-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.form-group label {
  color: #aaa;
  font-size: 0.9rem;
}

.form-group input {
  padding: 0.75rem;
  border-radius: 8px;
  border: 1px solid #2a2a4e;
  background: #0f0f23;
  color: #fff;
  font-size: 0.9rem;
}

.settings-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 1rem;
}

.btn-primary,
.btn-secondary {
  padding: 0.75rem 1.5rem;
  border-radius: 8px;
  border: none;
  cursor: pointer;
  font-size: 0.9rem;
  transition: all 0.2s;
}

.btn-primary {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.btn-secondary {
  background: #2a2a4e;
  color: #fff;
}

.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.btn-secondary:hover {
  background: #3a3a5e;
}

.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 1rem 0;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.message {
  display: flex;
  flex-direction: column;
  padding: 1rem;
  border-radius: 12px;
  max-width: 85%;
}

.message-user {
  align-self: flex-end;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.message-assistant {
  align-self: flex-start;
  background: #1a1a3e;
}

.message-system {
  align-self: center;
  background: transparent;
  color: #666;
  font-style: italic;
  font-size: 0.85rem;
  padding: 0.5rem;
}

.message-content {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
}

.message-role {
  font-size: 1.2rem;
}

.message-content p {
  line-height: 1.5;
}

.message-time {
  font-size: 0.7rem;
  color: rgba(255, 255, 255, 0.5);
  margin-top: 0.5rem;
  text-align: right;
}

.status-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.5rem;
  color: #667eea;
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid #667eea;
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.input-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
  padding: 1rem 0;
  border-top: 1px solid #2a2a4e;
}

.voice-button-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.voice-button {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  border: none;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  cursor: pointer;
  font-size: 2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s;
  box-shadow: 0 4px 15px rgba(102, 126, 234, 0.4);
}

.voice-button:hover:not(:disabled) {
  transform: scale(1.05);
  box-shadow: 0 6px 20px rgba(102, 126, 234, 0.5);
}

.voice-button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.voice-button.listening {
  animation: pulse 2s ease-in-out infinite;
  background: linear-gradient(135deg, #42b883 0%, #35495e 100%);
  box-shadow: 0 4px 15px rgba(66, 184, 131, 0.4);
}

.voice-button.speaking {
  animation: pulse 0.5s ease-in-out infinite;
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  box-shadow: 0 4px 15px rgba(245, 87, 108, 0.4);
}

@keyframes pulse {
  0%, 100% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.1);
  }
}

.voice-status {
  color: #888;
  font-size: 0.9rem;
}

.text-input-container {
  display: flex;
  width: 100%;
  gap: 0.5rem;
}

.text-input-container input {
  flex: 1;
  padding: 0.75rem 1rem;
  border-radius: 24px;
  border: 1px solid #2a2a4e;
  background: #1a1a3e;
  color: #fff;
  font-size: 0.9rem;
}

.text-input-container input:focus {
  outline: none;
  border-color: #667eea;
}

.btn-send {
  padding: 0.75rem 1.5rem;
  border-radius: 24px;
  border: none;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  cursor: pointer;
  font-size: 0.9rem;
  transition: all 0.2s;
}

.btn-send:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.btn-send:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Screenshot Modal Styles */
.screenshot-modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.8);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 1rem;
}

.screenshot-container {
  background: #1a1a3e;
  border-radius: 12px;
  max-width: 90vw;
  max-height: 90vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.screenshot-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.5rem;
  border-bottom: 1px solid #2a2a4e;
}

.screenshot-header h3 {
  color: #667eea;
  margin: 0;
}

.screenshot-actions {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.screenshot-preview {
  padding: 1rem;
  overflow: auto;
  display: flex;
  align-items: center;
  justify-content: center;
}

.screenshot-preview img {
  max-width: 100%;
  max-height: calc(90vh - 100px);
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
}

.btn-icon:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
