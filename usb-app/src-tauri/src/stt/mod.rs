// whisper.cpp subprocess management for speech-to-text.
//
// Two modes:
//   Streaming: spawns whisper-stream binary, reads stdout lines (partial transcripts),
//     emits them via "stt://partial" Tauri event. Used during conversational LLM turns.
//   Batch: spawns whisper-cli on a complete audio file. Used for meeting recording transcription.
//
// Audio capture: handled by the webview's MediaRecorder API (JS side).
// Audio chunks are passed to Rust via Tauri commands as base64-encoded PCM, or
// whisper-stream reads directly from a microphone device using ALSA.
//
// Model: whisper `base` multilingual model (~140MB), stored in resources/whisper/.

/// Start streaming STT. Partial transcripts stream via "stt://partial" Tauri event.
#[tauri::command]
pub async fn stt_start_stream() -> Result<(), String> {
    // TODO: spawn whisper-stream subprocess, read stdout, emit "stt://partial" events
    tracing::info!("stt_start_stream called");
    Ok(())
}

/// Stop streaming STT and return the final committed transcript.
#[tauri::command]
pub async fn stt_stop_stream() -> Result<String, String> {
    // TODO: send stop signal to whisper-stream, collect final transcript from stdout
    tracing::info!("stt_stop_stream called");
    Ok(String::new())
}

/// Batch-transcribe a recorded audio file (WAV format).
/// Returns the complete transcript. Used for meeting recording post-processing.
#[tauri::command]
pub async fn stt_transcribe_file(path: String) -> Result<String, String> {
    // TODO: spawn whisper-cli -f <path> -m resources/whisper/ggml-base.bin, capture stdout
    tracing::info!("stt_transcribe_file: {}", path);
    Ok(String::new())
}
