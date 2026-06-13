// llama.cpp subprocess management for on-device LLM inference.
//
// Manages a llama-server or llama-cli subprocess with the Qwen2.5 7B Q4 GGUF model.
// Tokens stream back to the frontend via Tauri events ("llm://token").
//
// Tool-call handling: when the LLM emits a JSON tool-call block, the response is
// parsed and dispatched to the appropriate vault command. The tool result is injected
// back into the conversation context before continuing generation.
//
// Config:
//   Model path: resolved from resources/models/ (bundled) or $HOME/Persistent/patient-vault/models/
//   Context window: 4096 tokens (conservative for 8GB machines)
//   Temperature: 0.7 for patient sessions, 0.3 for provider debrief sessions

/// Start a new LLM session with the given system prompt.
/// Spawns the llama.cpp process and begins listening for tokens.
#[tauri::command]
pub async fn llm_start_session(system_prompt: String) -> Result<(), String> {
    // TODO: spawn llama-server or llama-cli subprocess
    // Emit tokens via app.emit("llm://token", token_str)
    tracing::info!("llm_start_session called with system_prompt (len={})", system_prompt.len());
    Ok(())
}

/// Send a user message to the running LLM session.
/// Response tokens stream via "llm://token" Tauri event.
#[tauri::command]
pub async fn llm_send_message(message: String) -> Result<(), String> {
    // TODO: write message to llama.cpp stdin or HTTP endpoint
    tracing::info!("llm_send_message: {}", &message[..message.len().min(80)]);
    Ok(())
}

/// Stop the running LLM session and terminate the subprocess.
#[tauri::command]
pub async fn llm_stop_session() -> Result<(), String> {
    // TODO: kill the llama.cpp subprocess cleanly
    tracing::info!("llm_stop_session called");
    Ok(())
}
