pub mod assets;
pub mod crypto;
pub mod sync;
pub mod vault;
pub mod llm;
pub mod stt;
pub mod tts;

use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

/// Resolve LOG_FOLDER from the process environment or a nearby `.env` file.
fn resolve_log_folder() -> Option<String> {
    if let Ok(value) = std::env::var("LOG_FOLDER") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    for candidate in log_folder_env_candidates() {
        if let Some(value) = read_log_folder_from_env_file(&candidate) {
            return Some(value);
        }
    }

    None
}

fn log_folder_env_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(".env"));
        candidates.push(cwd.join("../provider-frontend/.env"));
        candidates.push(cwd.join("provider-frontend/.env"));
        candidates.push(cwd.join("../../provider-frontend/.env"));
    }

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest = PathBuf::from(manifest_dir);
        candidates.push(manifest.join(".env"));
        candidates.push(manifest.join("../../provider-frontend/.env"));
    }

    candidates
}

fn read_log_folder_from_env_file(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim() == "LOG_FOLDER" {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

fn init_tracing(log_folder: Option<&str>) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if let Some(folder) = log_folder {
        let log_path = Path::new(folder).join("usb-app.log");
        std::fs::create_dir_all(folder).expect("failed to create LOG_FOLDER");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .expect("failed to open usb-app.log");

        tracing_subscriber::fmt()
            .with_writer(std::sync::Mutex::new(file))
            .with_ansi(false)
            .with_env_filter(filter)
            .init();

        tracing::info!("USB app logging enabled → {}", log_path.display());
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
        tracing::info!("USB app logging to stderr (set LOG_FOLDER to write usb-app.log)");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing(resolve_log_folder().as_deref());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .manage(llm::LlmState::default())
        .manage(stt::SttState::default())
        .manage(tts::TtsState::default())
        .invoke_handler(tauri::generate_handler![
            // vault
            vault::vault_exists,
            vault::generate_keypair,
            vault::get_usb_id,
            vault::get_public_key,
            vault::write_vault_entry,
            vault::read_vault_entries,
            vault::sync_payloads,
            vault::read_payloads,
            vault::register_patient,
            vault::get_registration_state,
            vault::list_provider_links,
            vault::get_transcripts,
            vault::save_transcript,
            vault::save_questionnaire_response,
            // llm
            llm::llm_start_session,
            llm::llm_send_message,
            llm::llm_stop_session,
            // stt
            stt::stt_start_stream,
            stt::stt_stop_stream,
            stt::stt_transcribe_file,
            // tts
            tts::tts_synthesize,
            tts::tts_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
