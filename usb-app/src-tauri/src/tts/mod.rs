// Piper TTS subprocess management.
//
// Uses OHF-Voice/piper1-gpl (the maintained fork of rhasspy/piper, archived Oct 2025).
// Piper reads text from stdin and outputs WAV bytes to stdout.
// Audio is played via the webview's Web Audio API by emitting bytes via "tts://audio" Tauri event.
//
// Voice models (bundled in resources/piper-voices/):
//   En  → en_US-lessac-medium.onnx
//   Es  → es_MX-ald-medium.onnx
//   Zh  → zh_CN-huayan-medium.onnx
//
// Language is selectable per interaction, not globally fixed.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TtsLanguage {
    En,
    Es,
    Zh,
}

impl TtsLanguage {
    fn voice_model(&self) -> &'static str {
        match self {
            TtsLanguage::En => "en_US-lessac-medium.onnx",
            TtsLanguage::Es => "es_MX-ald-medium.onnx",
            TtsLanguage::Zh => "zh_CN-huayan-medium.onnx",
        }
    }
}

/// Synthesize text to speech for the given language.
/// Audio bytes (WAV) are emitted via "tts://audio" Tauri event.
#[tauri::command]
pub async fn tts_synthesize(text: String, lang: TtsLanguage) -> Result<(), String> {
    // TODO: spawn piper binary with --model resources/piper-voices/<voice>, stdin=text,
    //   capture stdout WAV bytes, emit "tts://audio" Tauri event chunks
    tracing::info!("tts_synthesize: lang={:?}, text_len={}", lang, text.len());
    Ok(())
}

/// Stop current TTS playback by killing the Piper subprocess.
#[tauri::command]
pub async fn tts_stop() -> Result<(), String> {
    // TODO: kill the running Piper subprocess
    tracing::info!("tts_stop called");
    Ok(())
}
