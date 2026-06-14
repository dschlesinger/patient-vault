//! Piper TTS subprocess management.
//!
//! Uses the archived `rhasspy/piper` v1.2.0 C++ CLI (selected for a dependency-
//! free, Tails-friendly binary). Piper reads a line of text from stdin and
//! writes a WAV file; we read it back and emit it as one base64 `tts://audio`
//! event for the webview's Web Audio player. Piper is realtime-fast on CPU, so
//! synthesizing a sentence-length reply as a single clip keeps the code simple
//! and robust across Piper versions.
//!
//! Voice models (under the models dir `piper/`), selectable per interaction:
//!   En → en_US-lessac-medium.onnx
//!   Es → es_MX-ald-medium.onnx
//!   Zh → zh_CN-huayan-medium.onnx
//!
//! Tails note: Piper is an *executable* (installed location) while the `.onnx`
//! voices are *data* (Persistent Storage). The temp WAV lives in `/tmp` (tmpfs).

use crate::assets;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum TtsLanguage {
    En,
    Es,
    Zh,
}

impl TtsLanguage {
    fn voice_model(self) -> &'static str {
        match self {
            TtsLanguage::En => "en_US-lessac-medium.onnx",
            TtsLanguage::Es => "es_MX-ald-medium.onnx",
            TtsLanguage::Zh => "zh_CN-huayan-medium.onnx",
        }
    }
}

/// Managed Tauri state: the currently-running Piper process, if any.
#[derive(Default)]
pub struct TtsState {
    current: tokio::sync::Mutex<Option<Child>>,
}

/// Synthesize `text` to speech in `lang`; emits the WAV as base64 `tts://audio`.
/// Any in-flight synthesis is stopped first (so a new reply supersedes an old
/// one). Returns early without emitting if interrupted via [`tts_stop`].
#[tauri::command]
pub async fn tts_synthesize(
    app: AppHandle,
    state: State<'_, TtsState>,
    text: String,
    lang: TtsLanguage,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }

    // Supersede any current synthesis.
    {
        let mut guard = state.current.lock().await;
        if let Some(mut child) = guard.take() {
            let _ = child.kill().await;
        }
    }

    let voice = assets::model_path(&app, &format!("piper/{}", lang.voice_model()))?;
    let bin = assets::binary_path(&app, "piper");
    let out_path = std::env::temp_dir().join(format!(
        "patient-vault-tts-{}-{}.wav",
        std::process::id(),
        now_nanos()
    ));

    let mut child = Command::new(&bin)
        .arg("--model")
        .arg(&voice)
        .arg("--output_file")
        .arg(&out_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to spawn piper ({}): {e}", bin.display()))?;

    if let Some(mut stdin) = child.stdin.take() {
        let line = format!("{}\n", text.replace('\n', " "));
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to piper stdin: {e}"))?;
        // Drop closes stdin so piper begins synthesis.
        drop(stdin);
    }

    {
        let mut guard = state.current.lock().await;
        *guard = Some(child);
    }

    // Poll for completion while remaining interruptible by tts_stop, which takes
    // the child out of the shared slot.
    let status = loop {
        {
            let mut guard = state.current.lock().await;
            match guard.as_mut() {
                Some(child) => {
                    if let Some(status) = child
                        .try_wait()
                        .map_err(|e| format!("piper wait failed: {e}"))?
                    {
                        guard.take();
                        break Some(status);
                    }
                }
                None => break None, // interrupted via tts_stop
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    };

    match status {
        None => {
            // Interrupted/barged-in: discard partial output, emit nothing.
            let _ = std::fs::remove_file(&out_path);
            return Ok(());
        }
        Some(status) if !status.success() => {
            let _ = std::fs::remove_file(&out_path);
            return Err(format!("piper exited with status {status}"));
        }
        Some(_) => {}
    }

    let bytes = std::fs::read(&out_path)
        .map_err(|e| format!("Failed to read synthesized audio: {e}"))?;
    let _ = std::fs::remove_file(&out_path);

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let _ = app.emit("tts://audio", &encoded);
    Ok(())
}

/// Stop current TTS playback by killing the Piper subprocess.
#[tauri::command]
pub async fn tts_stop(state: State<'_, TtsState>) -> Result<(), String> {
    let mut guard = state.current.lock().await;
    if let Some(mut child) = guard.take() {
        let _ = child.kill().await;
    }
    Ok(())
}

fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_models_map_per_language() {
        assert_eq!(TtsLanguage::En.voice_model(), "en_US-lessac-medium.onnx");
        assert_eq!(TtsLanguage::Es.voice_model(), "es_MX-ald-medium.onnx");
        assert_eq!(TtsLanguage::Zh.voice_model(), "zh_CN-huayan-medium.onnx");
    }

    #[test]
    fn language_deserializes_lowercase() {
        let en: TtsLanguage = serde_json::from_str("\"en\"").unwrap();
        assert!(matches!(en, TtsLanguage::En));
        let zh: TtsLanguage = serde_json::from_str("\"zh\"").unwrap();
        assert!(matches!(zh, TtsLanguage::Zh));
    }
}
