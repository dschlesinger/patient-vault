pub mod crypto;
pub mod vault;
pub mod llm;
pub mod stt;
pub mod tts;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
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
