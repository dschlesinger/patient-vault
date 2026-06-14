//! whisper.cpp subprocess management for speech-to-text.
//!
//! Two modes:
//!   - **Streaming** ([`stt_start_stream`]/[`stt_stop_stream`]): spawns
//!     `whisper-stream`, which captures the microphone directly (SDL2) and runs
//!     VAD-gated transcription. Finalized blocks are appended to a running
//!     transcript and emitted via `stt://partial`. Used for conversational turns.
//!   - **Batch** ([`stt_transcribe_file`]): runs `whisper-cli` over a complete
//!     WAV file. Used for meeting-recording transcription.
//!
//! Model: Whisper `base` multilingual (`whisper/ggml-base.bin`), resolved from
//! the app's models directory on the USB drive (see [`crate::assets`]). Direct
//! microphone capture via SDL2 requires standard Ubuntu audio access (ALSA/
//! PulseAudio/PipeWire).

use crate::assets;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;

const MODEL_REL: &str = "whisper/ggml-base.bin";

/// Managed Tauri state: at most one live streaming-STT session.
#[derive(Default)]
pub struct SttState {
    session: tokio::sync::Mutex<Option<SttSession>>,
}

struct SttSession {
    child: Child,
    transcript: Arc<Mutex<String>>,
    reader: JoinHandle<()>,
}

impl Drop for SttSession {
    fn drop(&mut self) {
        self.reader.abort();
        let _ = self.child.start_kill();
    }
}

/// Start streaming STT. Finalized transcript blocks stream via `stt://partial`.
#[tauri::command]
pub async fn stt_start_stream(app: AppHandle, state: State<'_, SttState>) -> Result<(), String> {
    // Replace any prior session.
    {
        let mut guard = state.session.lock().await;
        *guard = None;
    }

    let model = assets::model_path(&app, MODEL_REL)?;
    let bin = assets::binary_path(&app, "whisper-stream");

    tracing::info!("Starting whisper-stream {} (model={})", bin.display(), model.display());

    let mut child = Command::new(&bin)
        .arg("-m")
        .arg(&model)
        .arg("-t")
        .arg("6")
        .arg("--step")
        .arg("0") // VAD mode: emit a block when the speaker pauses
        .arg("--length")
        .arg("30000")
        .arg("-vth")
        .arg("0.6")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to spawn whisper-stream ({}): {e}", bin.display()))?;

    let stdout = child
        .stdout
        .take()
        .ok_or("whisper-stream produced no stdout handle")?;

    let transcript = Arc::new(Mutex::new(String::new()));
    let reader_transcript = Arc::clone(&transcript);
    let reader_app = app.clone();

    let reader = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(text) = clean_transcript_line(&line) {
                let full = {
                    let mut buf = reader_transcript.lock().unwrap();
                    if !buf.is_empty() {
                        buf.push(' ');
                    }
                    buf.push_str(&text);
                    buf.clone()
                };
                let _ = reader_app.emit("stt://partial", &full);
            }
        }
    });

    let mut guard = state.session.lock().await;
    *guard = Some(SttSession {
        child,
        transcript,
        reader,
    });
    Ok(())
}

/// Stop streaming STT and return the final committed transcript.
#[tauri::command]
pub async fn stt_stop_stream(state: State<'_, SttState>) -> Result<String, String> {
    let mut guard = state.session.lock().await;
    let Some(mut session) = guard.take() else {
        return Ok(String::new());
    };
    let _ = session.child.kill().await;
    session.reader.abort();
    let transcript = session.transcript.lock().unwrap().clone();
    Ok(transcript.trim().to_string())
}

/// Batch-transcribe a recorded WAV file. Returns the complete transcript.
#[tauri::command]
pub async fn stt_transcribe_file(app: AppHandle, path: String) -> Result<String, String> {
    let model = assets::model_path(&app, MODEL_REL)?;
    let bin = assets::binary_path(&app, "whisper-cli");

    tracing::info!("Batch transcribe {} with {}", path, bin.display());

    let output = Command::new(&bin)
        .arg("-m")
        .arg(&model)
        .arg("-f")
        .arg(&path)
        .arg("-nt") // no timestamps
        .arg("-np") // no progress prints
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .map_err(|e| format!("Failed to run whisper-cli ({}): {e}", bin.display()))?;

    if !output.status.success() {
        return Err(format!("whisper-cli exited with status {}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let transcript = stdout
        .lines()
        .filter_map(clean_transcript_line)
        .collect::<Vec<_>>()
        .join(" ");
    Ok(transcript.trim().to_string())
}

/// Strip ANSI escape sequences and whisper status/marker lines, returning the
/// spoken text or `None` if the line carries no transcript content.
fn clean_transcript_line(raw: &str) -> Option<String> {
    let stripped = strip_ansi(raw);
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Status lines and section markers, e.g. "[Start speaking]", "### Transcription".
    if trimmed.starts_with('[') || trimmed.starts_with('#') {
        return None;
    }
    // Whisper's silence/blank sentinels.
    let lower = trimmed.to_ascii_lowercase();
    if lower == "(blank_audio)" || lower == "[blank_audio]" || lower == "[silence]" {
        return None;
    }
    Some(trimmed.to_string())
}

/// Remove ANSI CSI escape sequences (e.g. cursor moves, colours) without a regex.
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // ESC: consume an optional '[' then up to a final byte in @-~.
            if chars.peek() == Some(&'[') {
                chars.next();
            }
            while let Some(&n) = chars.peek() {
                chars.next();
                if ('@'..='~').contains(&n) {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_escape_codes() {
        let input = "\u{1b}[2K\u{1b}[0mhello world\u{1b}[0m";
        assert_eq!(strip_ansi(input), "hello world");
    }

    #[test]
    fn clean_filters_status_and_blanks() {
        assert_eq!(clean_transcript_line("[Start speaking]"), None);
        assert_eq!(clean_transcript_line("### Transcription 1 START"), None);
        assert_eq!(clean_transcript_line("(blank_audio)"), None);
        assert_eq!(clean_transcript_line("   "), None);
        assert_eq!(clean_transcript_line(""), None);
    }

    #[test]
    fn clean_keeps_spoken_text_and_strips_ansi() {
        assert_eq!(
            clean_transcript_line("\u{1b}[2K I have a headache."),
            Some("I have a headache.".to_string())
        );
    }
}
