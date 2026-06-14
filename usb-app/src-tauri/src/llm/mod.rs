//! llama.cpp subprocess management for on-device LLM inference.
//!
//! Spawns a `llama-server` process serving the Qwen2.5 7B Q4 GGUF model bound to
//! loopback, then drives it over its OpenAI-compatible `/v1/chat/completions`
//! endpoint. Generated tokens stream to the frontend via the `llm://token`
//! event; a final `llm://done` event carries the complete assistant reply (used
//! to trigger TTS).
//!
//! Tool calling: the model is given the [`tools`] schema with `--jinja`-enabled
//! `llama-server`. When it emits tool calls, they are dispatched to the local
//! vault and the results are fed back, looping until the model produces a plain
//! answer. Guardrail filtering for provider sessions happens in [`tools`].
//!
//! Portability: `llama-server` and the GGUF model it loads are both resolved
//! relative to the app on the USB drive (see [`crate::assets`]). The server binds
//! loopback HTTP on an ephemeral port and is killed on drop.

mod tools;

pub use tools::Role;

use crate::assets;
use serde_json::{json, Value};
use std::process::Stdio;
use tauri::{AppHandle, Emitter, State};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Relative model path under the models asset directory.
const MODEL_REL: &str = "llm/qwen2.5-7b-instruct-q4_k_m.gguf";
/// Context window — conservative for 8GB machines.
const CONTEXT_TOKENS: u32 = 4096;
/// Upper bound on tool-call rounds before forcing a plain answer.
const MAX_TOOL_ROUNDS: usize = 5;

/// Managed Tauri state: at most one live LLM session.
#[derive(Default)]
pub struct LlmState {
    session: Mutex<Option<LlmSession>>,
}

/// A running `llama-server` plus its conversation context.
struct LlmSession {
    child: Child,
    base_url: String,
    /// OpenAI-format message history (system, user, assistant, tool).
    history: Vec<Value>,
    role: Role,
    temperature: f32,
}

impl Drop for LlmSession {
    fn drop(&mut self) {
        // Best-effort kill if the session is dropped without an explicit stop.
        let _ = self.child.start_kill();
    }
}

/// Start a new LLM session: spawn `llama-server`, wait for health, seed history.
/// `role` is `"patient"` or `"provider"` and selects temperature + guardrails.
#[tauri::command]
pub async fn llm_start_session(
    app: AppHandle,
    state: State<'_, LlmState>,
    system_prompt: String,
    role: String,
) -> Result<(), String> {
    let role = Role::parse(&role);
    let temperature = match role {
        Role::Patient => 0.7,
        Role::Provider => 0.3,
    };

    // Tear down any prior session first.
    {
        let mut guard = state.session.lock().await;
        *guard = None;
    }

    let model = assets::model_path(&app, MODEL_REL)?;
    let server = assets::binary_path(&app, "llama-server");
    let port = pick_loopback_port()?;
    let base_url = format!("http://127.0.0.1:{port}");

    tracing::info!(
        "Starting llama-server {} (model={}, port={}, role={:?})",
        server.display(),
        model.display(),
        port,
        role
    );

    let child = Command::new(&server)
        .arg("--model")
        .arg(&model)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--ctx-size")
        .arg(CONTEXT_TOKENS.to_string())
        .arg("--jinja") // enable tool-call template support
        .arg("-ngl")
        .arg("0") // CPU-only inference (no GPU offload)
        .arg("--no-webui")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to spawn llama-server ({}): {e}", server.display()))?;

    let session = LlmSession {
        child,
        base_url: base_url.clone(),
        history: vec![json!({ "role": "system", "content": system_prompt })],
        role,
        temperature,
    };

    {
        let mut guard = state.session.lock().await;
        *guard = Some(session);
    }

    // Poll health until the model is loaded (or give up and tear down).
    if let Err(e) = wait_for_health(&base_url).await {
        let mut guard = state.session.lock().await;
        *guard = None;
        return Err(e);
    }

    tracing::info!("llama-server ready at {base_url}");
    Ok(())
}

/// Send a user message and stream the assistant reply. Resolves tool calls
/// against the local vault, looping until the model produces a final answer.
#[tauri::command]
pub async fn llm_send_message(
    app: AppHandle,
    state: State<'_, LlmState>,
    message: String,
) -> Result<(), String> {
    let mut guard = state.session.lock().await;
    let session = guard
        .as_mut()
        .ok_or("No active LLM session. Call llm_start_session first.")?;

    session
        .history
        .push(json!({ "role": "user", "content": message }));

    let client = reqwest::Client::new();
    let mut final_content = String::new();

    for round in 0..MAX_TOOL_ROUNDS {
        // On the last allowed round, drop tools so the model must answer plainly.
        let allow_tools = round + 1 < MAX_TOOL_ROUNDS;
        let body = build_request(&session.history, session.temperature, allow_tools);

        let completion = stream_completion(&app, &client, &session.base_url, &body).await?;

        if completion.tool_calls.is_empty() {
            final_content = completion.content.clone();
            session
                .history
                .push(json!({ "role": "assistant", "content": completion.content }));
            break;
        }

        // Record the assistant's tool-call turn, then each tool result.
        session.history.push(assistant_tool_call_message(&completion));
        for call in &completion.tool_calls {
            let args: Value = serde_json::from_str(&call.arguments).unwrap_or(json!({}));
            let result = tools::dispatch(&call.name, &args, session.role);
            tracing::info!("tool {} -> {} bytes", call.name, result.len());
            session.history.push(json!({
                "role": "tool",
                "tool_call_id": call.id,
                "content": result,
            }));
        }
    }

    let _ = app.emit("llm://done", &final_content);
    Ok(())
}

/// Stop the session and terminate `llama-server`.
#[tauri::command]
pub async fn llm_stop_session(state: State<'_, LlmState>) -> Result<(), String> {
    let mut guard = state.session.lock().await;
    if let Some(mut session) = guard.take() {
        let _ = session.child.kill().await;
    }
    Ok(())
}

// ── HTTP / SSE plumbing ──────────────────────────────────────────────────────────

fn build_request(history: &[Value], temperature: f32, allow_tools: bool) -> Value {
    let mut body = json!({
        "model": "local",
        "messages": history,
        "temperature": temperature,
        "stream": true,
        "cache_prompt": true,
    });
    if allow_tools {
        body["tools"] = tools::tool_specs();
        body["tool_choice"] = json!("auto");
    }
    body
}

fn assistant_tool_call_message(completion: &Completion) -> Value {
    let calls: Vec<Value> = completion
        .tool_calls
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "type": "function",
                "function": { "name": c.name, "arguments": c.arguments },
            })
        })
        .collect();
    json!({
        "role": "assistant",
        "content": completion.content,
        "tool_calls": calls,
    })
}

/// Accumulated result of one streamed completion.
struct Completion {
    content: String,
    tool_calls: Vec<ToolCall>,
}

#[derive(Default, Clone)]
struct ToolCall {
    id: String,
    name: String,
    arguments: String,
}

/// POST a chat-completion request and consume the SSE stream, emitting
/// `llm://token` for each content delta and accumulating any tool calls.
async fn stream_completion(
    app: &AppHandle,
    client: &reqwest::Client,
    base_url: &str,
    body: &Value,
) -> Result<Completion, String> {
    let mut resp = client
        .post(format!("{base_url}/v1/chat/completions"))
        .json(body)
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("LLM returned {status}: {text}"));
    }

    let mut buffer = String::new();
    let mut content = String::new();
    let mut tool_acc: Vec<ToolCall> = Vec::new();

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("LLM stream error: {e}"))?
    {
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(newline) = buffer.find('\n') {
            let line: String = buffer.drain(..=newline).collect();
            match classify_sse_line(line.trim_end()) {
                Sse::Done => {
                    return Ok(Completion {
                        content,
                        tool_calls: tool_acc,
                    });
                }
                Sse::Data(payload) => {
                    if let Some(delta) = parse_chunk(&payload) {
                        if let Some(text) = delta.content {
                            if !text.is_empty() {
                                let _ = app.emit("llm://token", &text);
                                content.push_str(&text);
                            }
                        }
                        apply_tool_deltas(&mut tool_acc, delta.tool_calls);
                    }
                }
                Sse::Ignore => {}
            }
        }
    }

    Ok(Completion {
        content,
        tool_calls: tool_acc,
    })
}

/// Merge streamed tool-call fragments (keyed by index) into the accumulator.
fn apply_tool_deltas(acc: &mut Vec<ToolCall>, deltas: Vec<ToolCallDelta>) {
    for d in deltas {
        if acc.len() <= d.index {
            acc.resize(d.index + 1, ToolCall::default());
        }
        let entry = &mut acc[d.index];
        if let Some(id) = d.id {
            entry.id = id;
        }
        if let Some(name) = d.name {
            entry.name.push_str(&name);
        }
        if let Some(args) = d.arguments {
            entry.arguments.push_str(&args);
        }
    }
}

enum Sse {
    Data(String),
    Done,
    Ignore,
}

/// Classify one line of an SSE stream.
fn classify_sse_line(line: &str) -> Sse {
    let Some(rest) = line.strip_prefix("data:") else {
        return Sse::Ignore;
    };
    let rest = rest.trim();
    if rest == "[DONE]" {
        Sse::Done
    } else if rest.is_empty() {
        Sse::Ignore
    } else {
        Sse::Data(rest.to_string())
    }
}

struct ChunkDelta {
    content: Option<String>,
    tool_calls: Vec<ToolCallDelta>,
}

struct ToolCallDelta {
    index: usize,
    id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
}

/// Parse one `data:` JSON chunk into its content + tool-call deltas.
fn parse_chunk(json_str: &str) -> Option<ChunkDelta> {
    let v: Value = serde_json::from_str(json_str).ok()?;
    let delta = v.get("choices")?.get(0)?.get("delta")?;

    let content = delta
        .get("content")
        .and_then(|c| c.as_str())
        .map(String::from);

    let mut tool_calls = Vec::new();
    if let Some(calls) = delta.get("tool_calls").and_then(|c| c.as_array()) {
        for (i, call) in calls.iter().enumerate() {
            let index = call.get("index").and_then(|x| x.as_u64()).unwrap_or(i as u64) as usize;
            let func = call.get("function");
            tool_calls.push(ToolCallDelta {
                index,
                id: call.get("id").and_then(|x| x.as_str()).map(String::from),
                name: func
                    .and_then(|f| f.get("name"))
                    .and_then(|x| x.as_str())
                    .map(String::from),
                arguments: func
                    .and_then(|f| f.get("arguments"))
                    .and_then(|x| x.as_str())
                    .map(String::from),
            });
        }
    }

    Some(ChunkDelta {
        content,
        tool_calls,
    })
}

// ── Process lifecycle helpers ────────────────────────────────────────────────────

/// Ports commonly reserved by Tor / other local daemons; avoid binding these.
const BLOCKED_PORTS: &[u16] = &[9050, 9051, 9052, 9040, 9062, 9150, 953, 5353];

/// Pick a free loopback TCP port, avoiding commonly-reserved ports.
fn pick_loopback_port() -> Result<u16, String> {
    for _ in 0..16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Could not allocate a loopback port: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .port();
        drop(listener);
        if !BLOCKED_PORTS.contains(&port) {
            return Ok(port);
        }
    }
    Err("Could not find a usable loopback port.".to_string())
}

/// Poll `GET /health` until the server reports ready, or time out.
/// Model load on CPU can take a while, so the timeout is generous.
async fn wait_for_health(base_url: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    loop {
        if let Ok(resp) = client.get(format!("{base_url}/health")).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err("llama-server did not become healthy within 180s.".to_string());
        }
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_sse_line_handles_data_done_and_noise() {
        assert!(matches!(classify_sse_line("data: [DONE]"), Sse::Done));
        assert!(matches!(classify_sse_line("data:[DONE]"), Sse::Done));
        assert!(matches!(classify_sse_line(": keep-alive"), Sse::Ignore));
        assert!(matches!(classify_sse_line(""), Sse::Ignore));
        match classify_sse_line("data: {\"a\":1}") {
            Sse::Data(p) => assert_eq!(p, "{\"a\":1}"),
            _ => panic!("expected data"),
        }
    }

    #[test]
    fn parse_chunk_extracts_content() {
        let chunk = r#"{"choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let delta = parse_chunk(chunk).unwrap();
        assert_eq!(delta.content.as_deref(), Some("Hello"));
        assert!(delta.tool_calls.is_empty());
    }

    #[test]
    fn parse_and_accumulate_streamed_tool_call() {
        // Tool calls arrive split across chunks; they must reassemble by index.
        let c1 = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","function":{"name":"get_vault_entries","arguments":"{\"cat"}}]}}]}"#;
        let c2 = r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"egory\":\"meds\"}"}}]}}]}"#;

        let mut acc: Vec<ToolCall> = Vec::new();
        apply_tool_deltas(&mut acc, parse_chunk(c1).unwrap().tool_calls);
        apply_tool_deltas(&mut acc, parse_chunk(c2).unwrap().tool_calls);

        assert_eq!(acc.len(), 1);
        assert_eq!(acc[0].id, "call_1");
        assert_eq!(acc[0].name, "get_vault_entries");
        assert_eq!(acc[0].arguments, r#"{"category":"meds"}"#);
    }

    #[test]
    fn build_request_toggles_tools() {
        let history = vec![json!({"role":"user","content":"hi"})];
        let with = build_request(&history, 0.7, true);
        assert!(with.get("tools").is_some());
        assert_eq!(with["tool_choice"], json!("auto"));

        let without = build_request(&history, 0.3, false);
        assert!(without.get("tools").is_none());
    }

    #[test]
    fn pick_loopback_port_avoids_blocked() {
        let port = pick_loopback_port().unwrap();
        assert!(!BLOCKED_PORTS.contains(&port));
        assert!(port > 1024);
    }
}
