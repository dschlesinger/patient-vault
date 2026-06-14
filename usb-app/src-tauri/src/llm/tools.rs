//! LLM tool (function-calling) schemas and dispatch to the local vault.
//!
//! The schema list is sent to `llama-server` as the OpenAI `tools` array. When
//! the model emits a tool call, [`dispatch`] routes it (by name) to the
//! corresponding vault function and returns a JSON string to feed back as the
//! `role:"tool"` message content.
//!
//! Guardrail enforcement (data layer): in a provider session, vault-entry reads
//! force `exclude_private = true` regardless of the arguments the model
//! supplies, so private entries never enter the context window.

use crate::vault::{self, PayloadFilter, VaultFilter};
use serde::Serialize;
use serde_json::{json, Value};

/// Which session the LLM is serving — controls guardrail filtering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Patient,
    Provider,
}

impl Role {
    /// Parse a role string from the frontend; unknown values default to the
    /// safer `Provider` (guardrails on).
    pub fn parse(s: &str) -> Role {
        match s.trim().to_ascii_lowercase().as_str() {
            "patient" => Role::Patient,
            _ => Role::Provider,
        }
    }

    fn is_provider(self) -> bool {
        self == Role::Provider
    }
}

/// The OpenAI-format `tools` array advertised to the model. Built fresh each
/// call so it is trivially testable and cheap to construct.
pub fn tool_specs() -> Value {
    json!([
        spec(
            "get_vault_entries",
            "Read the patient's local vault entries (personal and health information). \
             In a provider session, entries the patient marked private are never returned.",
            json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Optional category to filter by (e.g. 'medications')."
                    }
                }
            })
        ),
        spec(
            "get_payloads",
            "Fetch decrypted items sent by providers: messages, questionnaires, or documents.",
            json!({
                "type": "object",
                "properties": {
                    "type": {
                        "type": "string",
                        "enum": ["message", "questionnaire", "document"],
                        "description": "Optional payload type to filter by."
                    },
                    "provider_id": {
                        "type": "string",
                        "description": "Optional provider id to filter by."
                    }
                }
            })
        ),
        spec(
            "get_questionnaire",
            "Retrieve a single questionnaire (and any answers already saved) by its payload id, \
             to walk through its questions one at a time.",
            json!({
                "type": "object",
                "properties": {
                    "payload_id": { "type": "string", "description": "The questionnaire payload id." }
                },
                "required": ["payload_id"]
            })
        ),
        spec(
            "get_transcripts",
            "Fetch saved meeting-recording transcripts, optionally within a Unix-seconds time range.",
            json!({
                "type": "object",
                "properties": {
                    "from": { "type": "string", "description": "Optional start time (Unix seconds)." },
                    "to": { "type": "string", "description": "Optional end time (Unix seconds)." }
                }
            })
        ),
        spec(
            "save_questionnaire_response",
            "Save the patient's answer to a specific questionnaire question.",
            json!({
                "type": "object",
                "properties": {
                    "payload_id": { "type": "string", "description": "The questionnaire payload id." },
                    "question_id": { "type": "string", "description": "The question identifier." },
                    "response": { "type": "string", "description": "The patient's answer text." }
                },
                "required": ["payload_id", "question_id", "response"]
            })
        )
    ])
}

fn spec(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters
        }
    })
}

/// Dispatch a tool call to the vault and return a JSON string result.
/// Errors are returned as `{"error": "..."}` so the model can react gracefully
/// rather than the whole turn failing.
pub fn dispatch(name: &str, args: &Value, role: Role) -> String {
    let result: Result<Value, String> = match name {
        "get_vault_entries" => get_vault_entries(args, role),
        "get_payloads" => get_payloads(args),
        "get_questionnaire" => get_questionnaire(args, role),
        "get_transcripts" => get_transcripts(args),
        "save_questionnaire_response" => save_questionnaire_response(args, role),
        other => Err(format!("Unknown tool: {other}")),
    };

    match result {
        Ok(value) => value.to_string(),
        Err(e) => json!({ "error": e }).to_string(),
    }
}

fn to_json<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

fn get_vault_entries(args: &Value, role: Role) -> Result<Value, String> {
    let filter = VaultFilter {
        category: args
            .get("category")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        // Hard guardrail: providers never see private entries.
        exclude_private: role.is_provider(),
    };
    to_json(vault::read_vault_entries(filter)?)
}

fn get_payloads(args: &Value) -> Result<Value, String> {
    let filter = PayloadFilter {
        payload_type: args.get("type").and_then(|v| v.as_str()).map(String::from),
        provider_id: args
            .get("provider_id")
            .and_then(|v| v.as_str())
            .map(String::from),
    };
    to_json(vault::read_payloads(filter)?)
}

fn get_questionnaire(args: &Value, _role: Role) -> Result<Value, String> {
    let payload_id = args
        .get("payload_id")
        .and_then(|v| v.as_str())
        .ok_or("payload_id is required")?;
    let questionnaire = vault::get_questionnaire(payload_id)?;
    let responses = vault::read_responses_for(payload_id)?;
    Ok(json!({
        "questionnaire": questionnaire,
        "saved_responses": responses,
    }))
}

fn get_transcripts(args: &Value) -> Result<Value, String> {
    let from = args.get("from").and_then(|v| v.as_str()).map(String::from);
    let to = args.get("to").and_then(|v| v.as_str()).map(String::from);
    to_json(vault::get_transcripts(from, to)?)
}

fn save_questionnaire_response(args: &Value, role: Role) -> Result<Value, String> {
    // Providers must not write patient answers.
    if role.is_provider() {
        return Err("Saving questionnaire responses is not allowed in a provider session.".into());
    }
    let payload_id = args
        .get("payload_id")
        .and_then(|v| v.as_str())
        .ok_or("payload_id is required")?;
    let question_id = args
        .get("question_id")
        .and_then(|v| v.as_str())
        .ok_or("question_id is required")?;
    let response = args
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or("response is required")?;
    vault::save_questionnaire_response(
        payload_id.to_string(),
        question_id.to_string(),
        response.to_string(),
    )?;
    Ok(json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_parse_defaults_to_provider() {
        assert_eq!(Role::parse("patient"), Role::Patient);
        assert_eq!(Role::parse("Patient"), Role::Patient);
        assert_eq!(Role::parse("provider"), Role::Provider);
        assert_eq!(Role::parse("garbage"), Role::Provider);
    }

    #[test]
    fn tool_specs_lists_all_five_tools() {
        let specs = tool_specs();
        let names: Vec<&str> = specs
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "get_vault_entries",
                "get_payloads",
                "get_questionnaire",
                "get_transcripts",
                "save_questionnaire_response",
            ]
        );
    }

    #[test]
    fn provider_cannot_save_responses() {
        let out = dispatch(
            "save_questionnaire_response",
            &json!({"payload_id": "p", "question_id": "q", "response": "r"}),
            Role::Provider,
        );
        assert!(out.contains("error"));
        assert!(out.contains("provider session"));
    }

    #[test]
    fn unknown_tool_returns_error_json() {
        let out = dispatch("nope", &json!({}), Role::Patient);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert!(v["error"].as_str().unwrap().contains("Unknown tool"));
    }
}
