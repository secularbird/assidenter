<script setup>
import { ref, watch, computed, onMounted, onUnmounted } from 'vue'

// Props
const props = defineProps({
  videoFile: {
    type: Object,
    default: null
  }
})

// Emits
const emit = defineEmits(['close', 'save'])

// State
const videoRef = ref(null)
const canvasRef = ref(null)
const videoSrc = ref('')
const duration = ref(0)
const currentTime = ref(0)
const startTime = ref(0)
const endTime = ref(0)
const isPlaying = ref(false)
const isTrimming = ref(false)
const trimProgress = ref(0)
const errorMessage = ref('')

// Constants for video trimming
const VIDEO_BITRATE = 2500000 // 2.5 Mbps video bitrate
const RECORDER_TIMESLICE_MS = 100 // Collect data every 100ms

// Computed
const formattedDuration = computed(() => formatTime(duration.value))
const formattedCurrentTime = computed(() => formatTime(currentTime.value))
const formattedStartTime = computed(() => formatTime(startTime.value))
const formattedEndTime = computed(() => formatTime(endTime.value))
const trimDuration = computed(() => endTime.value - startTime.value)
const formattedTrimDuration = computed(() => formatTime(trimDuration.value))

// Format time as MM:SS.ms
function formatTime(seconds) {
  if (!seconds || isNaN(seconds)) return '00:00.0'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  const ms = Math.floor((seconds % 1) * 10)
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}.${ms}`
}

// Parse time string to seconds
function parseTime(timeStr) {
  const match = timeStr.match(/^(\d+):(\d+)\.?(\d)?$/)
  if (!match) return 0
  const mins = parseInt(match[1], 10)
  const secs = parseInt(match[2], 10)
  const ms = match[3] ? parseInt(match[3], 10) / 10 : 0
  return mins * 60 + secs + ms
}

// Load video from file
function loadVideo(file) {
  if (!file) return
  
  errorMessage.value = ''
  
  // Validate file type
  if (!file.type.startsWith('video/')) {
    errorMessage.value = '请选择视频文件 / Please select a video file'
    return
  }
  
  // Revoke previous URL
  if (videoSrc.value) {
    URL.revokeObjectURL(videoSrc.value)
  }
  
  videoSrc.value = URL.createObjectURL(file)
}

// Handle video metadata loaded
function onLoadedMetadata() {
  if (videoRef.value) {
    duration.value = videoRef.value.duration
    endTime.value = duration.value
    startTime.value = 0
  }
}

// Handle video time update
function onTimeUpdate() {
  if (videoRef.value) {
    currentTime.value = videoRef.value.currentTime
    
    // Stop playback if reached end trim point
    if (currentTime.value >= endTime.value) {
      videoRef.value.pause()
      isPlaying.value = false
      videoRef.value.currentTime = startTime.value
    }
  }
}

// Handle video error
function onVideoError(event) {
  errorMessage.value = '视频加载失败 / Failed to load video'
  console.error('Video error:', event)
}

// Toggle play/pause
function togglePlay() {
  if (!videoRef.value) return
  
  if (isPlaying.value) {
    videoRef.value.pause()
    isPlaying.value = false
  } else {
    // Start from start trim point if at beginning or before start
    if (videoRef.value.currentTime < startTime.value || 
        videoRef.value.currentTime >= endTime.value) {
      videoRef.value.currentTime = startTime.value
    }
    videoRef.value.play()
    isPlaying.value = true
  }
}

// Seek to specific time
function seekTo(time) {
  if (videoRef.value) {
    videoRef.value.currentTime = Math.max(0, Math.min(time, duration.value))
    currentTime.value = videoRef.value.currentTime
  }
}

// Handle timeline click
function onTimelineClick(event) {
  const rect = event.target.getBoundingClientRect()
  const x = event.clientX - rect.left
  const percentage = x / rect.width
  const time = percentage * duration.value
  seekTo(time)
}

// Set start time to current time
function setStartAtCurrent() {
  startTime.value = Math.min(currentTime.value, endTime.value - 0.1)
}

// Set end time to current time
function setEndAtCurrent() {
  endTime.value = Math.max(currentTime.value, startTime.value + 0.1)
}

// Handle start time slider change
function onStartTimeChange(event) {
  const value = parseFloat(event.target.value)
  startTime.value = Math.min(value, endTime.value - 0.1)
  if (currentTime.value < startTime.value) {
    seekTo(startTime.value)
  }
}

// Handle end time slider change
function onEndTimeChange(event) {
  const value = parseFloat(event.target.value)
  endTime.value = Math.max(value, startTime.value + 0.1)
  if (currentTime.value > endTime.value) {
    seekTo(endTime.value)
  }
}

// Preview the trimmed portion
function previewTrim() {
  seekTo(startTime.value)
  if (!isPlaying.value) {
    togglePlay()
  }
}

// Trim and export the video
async function trimVideo() {
  if (!videoRef.value || !videoSrc.value) return
  
  isTrimming.value = true
  trimProgress.value = 0
  errorMessage.value = ''
  
  let audioCtx = null
  let source = null
  
  try {
    // Use MediaRecorder to capture the trimmed portion
    const video = videoRef.value
    const canvas = canvasRef.value
    const ctx = canvas.getContext('2d')
    
    // Set canvas dimensions to match video
    canvas.width = video.videoWidth
    canvas.height = video.videoHeight
    
    // Create a MediaStream from canvas
    const stream = canvas.captureStream(30)
    
    // Try to capture audio if available
    let audioTrack = null
    try {
      // Create audio context to capture video audio
      audioCtx = new (window.AudioContext || window.webkitAudioContext)()
      source = audioCtx.createMediaElementSource(video)
      const dest = audioCtx.createMediaStreamDestination()
      source.connect(dest)
      source.connect(audioCtx.destination) // Also play to speakers
      audioTrack = dest.stream.getAudioTracks()[0]
      if (audioTrack) {
        stream.addTrack(audioTrack)
      }
    } catch (audioError) {
      console.warn('Could not capture audio:', audioError)
    }
    
    // Determine supported MIME type
    const mimeTypes = [
      'video/webm;codecs=vp9',
      'video/webm;codecs=vp8',
      'video/webm',
      'video/mp4'
    ]
    let selectedMimeType = ''
    for (const mimeType of mimeTypes) {
      if (MediaRecorder.isTypeSupported(mimeType)) {
        selectedMimeType = mimeType
        break
      }
    }
    
    const mediaRecorder = new MediaRecorder(stream, {
      mimeType: selectedMimeType || undefined,
      videoBitsPerSecond: VIDEO_BITRATE
    })
    
    const chunks = []
    
    mediaRecorder.ondataavailable = (event) => {
      if (event.data.size > 0) {
        chunks.push(event.data)
      }
    }
    
    mediaRecorder.onstop = () => {
      const blob = new Blob(chunks, { type: selectedMimeType || 'video/webm' })
      const url = URL.createObjectURL(blob)
      
      // Create download link
      const a = document.createElement('a')
      a.href = url
      const extension = selectedMimeType.includes('mp4') ? 'mp4' : 'webm'
      a.download = `trimmed-video-${Date.now()}.${extension}`
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      
      // Cleanup
      URL.revokeObjectURL(url)
      isTrimming.value = false
      trimProgress.value = 100
      
      // Cleanup audio context
      if (source) {
        source.disconnect()
      }
      if (audioCtx) {
        audioCtx.close().catch(() => {})
      }
      
      emit('save', blob)
    }
    
    // Start recording and playback
    video.currentTime = startTime.value
    video.muted = true // Mute to avoid echo during recording
    
    // Wait for seek to complete and cleanup the event handler
    await new Promise(resolve => {
      const onSeeked = () => {
        video.removeEventListener('seeked', onSeeked)
        resolve()
      }
      video.addEventListener('seeked', onSeeked)
    })
    
    mediaRecorder.start(RECORDER_TIMESLICE_MS)
    video.play()
    
    // Draw frames to canvas and track progress
    const drawFrame = () => {
      if (video.currentTime >= endTime.value || video.paused) {
        video.pause()
        mediaRecorder.stop()
        video.muted = false
        return
      }
      
      ctx.drawImage(video, 0, 0, canvas.width, canvas.height)
      trimProgress.value = Math.round(
        ((video.currentTime - startTime.value) / trimDuration.value) * 100
      )
      
      requestAnimationFrame(drawFrame)
    }
    
    drawFrame()
    
  } catch (error) {
    console.error('Trim error:', error)
    errorMessage.value = `裁剪失败 / Trim failed: ${error.message}`
    isTrimming.value = false
  }
}

// Handle file selection
function onFileSelect(event) {
  const file = event.target.files?.[0]
  if (file) {
    loadVideo(file)
  }
}

// Open file dialog
function openFileDialog() {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = 'video/*'
  input.onchange = (e) => {
    const file = e.target.files?.[0]
    if (file) {
      loadVideo(file)
    }
  }
  input.click()
}

// Close trimmer
function close() {
  if (videoSrc.value) {
    URL.revokeObjectURL(videoSrc.value)
  }
  emit('close')
}

// Watch for prop changes
watch(() => props.videoFile, (newFile) => {
  if (newFile) {
    loadVideo(newFile)
  }
}, { immediate: true })

// Cleanup on unmount
onUnmounted(() => {
  if (videoSrc.value) {
    URL.revokeObjectURL(videoSrc.value)
  }
})
</script>

<template>
  <div class="video-trimmer-modal" @click.self="close">
    <div class="video-trimmer-container">
      <div class="trimmer-header">
        <h3>🎬 视频裁剪 / Video Trimmer</h3>
        <button class="btn-icon" @click="close" title="Close">✕</button>
      </div>
      
      <div class="trimmer-content">
        <!-- File Selection -->
        <div v-if="!videoSrc" class="file-select-area">
          <div class="file-drop-zone" @click="openFileDialog">
            <span class="drop-icon">📁</span>
            <p>点击选择视频文件</p>
            <p class="drop-hint">Click to select a video file</p>
          </div>
        </div>
        
        <!-- Video Preview -->
        <div v-else class="video-preview-area">
          <div class="video-wrapper">
            <video
              ref="videoRef"
              :src="videoSrc"
              @loadedmetadata="onLoadedMetadata"
              @timeupdate="onTimeUpdate"
              @error="onVideoError"
              @ended="isPlaying = false"
            />
            <canvas ref="canvasRef" style="display: none;" />
          </div>
          
          <!-- Error Message -->
          <div v-if="errorMessage" class="error-message">
            {{ errorMessage }}
          </div>
          
          <!-- Controls -->
          <div class="video-controls">
            <!-- Playback Controls -->
            <div class="playback-controls">
              <button class="btn-control" @click="togglePlay" :disabled="isTrimming">
                {{ isPlaying ? '⏸️' : '▶️' }}
              </button>
              <span class="time-display">
                {{ formattedCurrentTime }} / {{ formattedDuration }}
              </span>
            </div>
            
            <!-- Timeline -->
            <div class="timeline-container" @click="onTimelineClick">
              <div class="timeline">
                <!-- Trim Range Indicator -->
                <div 
                  class="trim-range"
                  :style="{
                    left: (startTime / duration * 100) + '%',
                    width: ((endTime - startTime) / duration * 100) + '%'
                  }"
                />
                <!-- Current Position -->
                <div 
                  class="playhead"
                  :style="{ left: (currentTime / duration * 100) + '%' }"
                />
              </div>
            </div>
            
            <!-- Trim Points -->
            <div class="trim-controls">
              <div class="trim-point">
                <label>开始 / Start:</label>
                <input 
                  type="range" 
                  :min="0" 
                  :max="duration" 
                  :value="startTime"
                  step="0.1"
                  @input="onStartTimeChange"
                  :disabled="isTrimming"
                />
                <span class="time-value">{{ formattedStartTime }}</span>
                <button class="btn-small" @click="setStartAtCurrent" :disabled="isTrimming">
                  设为当前 / Set Current
                </button>
              </div>
              
              <div class="trim-point">
                <label>结束 / End:</label>
                <input 
                  type="range" 
                  :min="0" 
                  :max="duration" 
                  :value="endTime"
                  step="0.1"
                  @input="onEndTimeChange"
                  :disabled="isTrimming"
                />
                <span class="time-value">{{ formattedEndTime }}</span>
                <button class="btn-small" @click="setEndAtCurrent" :disabled="isTrimming">
                  设为当前 / Set Current
                </button>
              </div>
              
              <div class="trim-info">
                <span>裁剪时长 / Trim Duration: {{ formattedTrimDuration }}</span>
              </div>
            </div>
            
            <!-- Progress Bar (during trimming) -->
            <div v-if="isTrimming" class="progress-container">
              <div class="progress-bar">
                <div class="progress-fill" :style="{ width: trimProgress + '%' }" />
              </div>
              <span class="progress-text">{{ trimProgress }}%</span>
            </div>
            
            <!-- Action Buttons -->
            <div class="action-buttons">
              <button class="btn-secondary" @click="openFileDialog" :disabled="isTrimming">
                📂 选择其他 / Choose Another
              </button>
              <button class="btn-secondary" @click="previewTrim" :disabled="isTrimming">
                👁️ 预览 / Preview
              </button>
              <button class="btn-primary" @click="trimVideo" :disabled="isTrimming || trimDuration < 0.1">
                {{ isTrimming ? '裁剪中... / Trimming...' : '✂️ 裁剪并保存 / Trim & Save' }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.video-trimmer-modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.85);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 1rem;
}

.video-trimmer-container {
  background: #1a1a3e;
  border-radius: 12px;
  width: 100%;
  max-width: 900px;
  max-height: 90vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.trimmer-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem 1.5rem;
  border-bottom: 1px solid #2a2a4e;
}

.trimmer-header h3 {
  color: #667eea;
  margin: 0;
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

.trimmer-content {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
}

/* File Selection Area */
.file-select-area {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 300px;
}

.file-drop-zone {
  border: 2px dashed #667eea;
  border-radius: 12px;
  padding: 3rem;
  text-align: center;
  cursor: pointer;
  transition: all 0.3s;
}

.file-drop-zone:hover {
  background: rgba(102, 126, 234, 0.1);
  border-color: #764ba2;
}

.drop-icon {
  font-size: 3rem;
  display: block;
  margin-bottom: 1rem;
}

.drop-hint {
  color: #888;
  font-size: 0.9rem;
  margin-top: 0.5rem;
}

/* Video Preview */
.video-preview-area {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.video-wrapper {
  position: relative;
  width: 100%;
  background: #000;
  border-radius: 8px;
  overflow: hidden;
}

.video-wrapper video {
  width: 100%;
  max-height: 400px;
  display: block;
}

/* Error Message */
.error-message {
  background: rgba(245, 87, 108, 0.2);
  color: #f5576c;
  padding: 0.75rem 1rem;
  border-radius: 8px;
  text-align: center;
}

/* Video Controls */
.video-controls {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.playback-controls {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.btn-control {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  border: none;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  cursor: pointer;
  font-size: 1.2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.btn-control:hover:not(:disabled) {
  transform: scale(1.05);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.btn-control:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.time-display {
  color: #aaa;
  font-family: monospace;
  font-size: 0.9rem;
}

/* Timeline */
.timeline-container {
  padding: 0.5rem 0;
  cursor: pointer;
}

.timeline {
  height: 8px;
  background: #2a2a4e;
  border-radius: 4px;
  position: relative;
}

.trim-range {
  position: absolute;
  top: 0;
  height: 100%;
  background: rgba(102, 126, 234, 0.5);
  border-radius: 4px;
}

.playhead {
  position: absolute;
  top: -4px;
  width: 4px;
  height: 16px;
  background: #fff;
  border-radius: 2px;
  transform: translateX(-50%);
}

/* Trim Controls */
.trim-controls {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.trim-point {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.trim-point label {
  color: #aaa;
  min-width: 100px;
  font-size: 0.9rem;
}

.trim-point input[type="range"] {
  flex: 1;
  min-width: 150px;
  -webkit-appearance: none;
  appearance: none;
  height: 6px;
  background: #2a2a4e;
  border-radius: 3px;
  outline: none;
}

.trim-point input[type="range"]::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 16px;
  height: 16px;
  background: #667eea;
  border-radius: 50%;
  cursor: pointer;
}

.trim-point input[type="range"]::-moz-range-thumb {
  width: 16px;
  height: 16px;
  background: #667eea;
  border-radius: 50%;
  cursor: pointer;
  border: none;
}

.time-value {
  color: #667eea;
  font-family: monospace;
  min-width: 70px;
}

.btn-small {
  padding: 0.4rem 0.8rem;
  border-radius: 6px;
  border: 1px solid #667eea;
  background: transparent;
  color: #667eea;
  cursor: pointer;
  font-size: 0.8rem;
  transition: all 0.2s;
}

.btn-small:hover:not(:disabled) {
  background: rgba(102, 126, 234, 0.2);
}

.btn-small:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.trim-info {
  color: #888;
  font-size: 0.9rem;
  text-align: center;
  padding: 0.5rem;
  background: rgba(102, 126, 234, 0.1);
  border-radius: 6px;
}

/* Progress Bar */
.progress-container {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.progress-bar {
  flex: 1;
  height: 8px;
  background: #2a2a4e;
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  transition: width 0.1s;
}

.progress-text {
  color: #667eea;
  font-family: monospace;
  min-width: 50px;
}

/* Action Buttons */
.action-buttons {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  flex-wrap: wrap;
  padding-top: 0.5rem;
}

.btn-primary,
.btn-secondary {
  padding: 0.75rem 1.25rem;
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

.btn-primary:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.btn-secondary:hover:not(:disabled) {
  background: #3a3a5e;
}

.btn-primary:disabled,
.btn-secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Responsive */
@media (max-width: 600px) {
  .video-trimmer-container {
    max-height: 95vh;
  }
  
  .trimmer-content {
    padding: 1rem;
  }
  
  .trim-point {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }
  
  .trim-point input[type="range"] {
    width: 100%;
  }
  
  .action-buttons {
    flex-direction: column;
  }
  
  .action-buttons button {
    width: 100%;
  }
}
</style>
