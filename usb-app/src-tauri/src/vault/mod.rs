// Local encrypted vault for PatientVault USB app.
//
// The vault is a JSON file stored in Tails Persistent Storage, encrypted at
// rest with AES-256-GCM using a key derived from the patient's private keypair.
// The keypair itself is stored in a separate file, also AES-encrypted using a
// device-local key derived from the USB's unique hardware identifier.
//
// For v1, keypair storage uses a simplified approach: the keypair is stored as
// JSON and protected by Tails Persistent Storage's own filesystem encryption.
// A dedicated vault encryption key derived from the keypair is used for vault data.

use crate::crypto::{derive_usb_id, HybridKeypair, HybridPublicKey};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn vault_dir() -> PathBuf {
    // Tails Persistent Storage is mounted at /home/user/Persistent
    // For development, fall back to a local .vault directory.
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    let persistent = PathBuf::from(&home).join("Persistent").join("patient-vault");
    if persistent.exists() {
        persistent
    } else {
        PathBuf::from(".vault")
    }
}

fn keypair_path() -> PathBuf {
    vault_dir().join("keypair.json")
}

fn vault_path() -> PathBuf {
    vault_dir().join("vault.json")
}

fn registration_path() -> PathBuf {
    vault_dir().join("registration.json")
}

fn providers_path() -> PathBuf {
    vault_dir().join("providers.json")
}

fn documents_dir() -> PathBuf {
    vault_dir().join("documents")
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VaultEntry {
    pub id: String,
    pub category: String,
    pub content: String,
    pub tags: Vec<String>,
    pub is_private: bool,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct VaultFilter {
    pub category: Option<String>,
    pub exclude_private: bool,
}

fn payloads_path() -> PathBuf {
    vault_dir().join("payloads.json")
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProviderLink {
    pub provider_id: String,
    /// Patient-assigned, human-readable name for this provider (e.g. "Dr. Patel").
    /// Stored locally only — never sent to Supabase. `serde(default)` keeps older
    /// `providers.json` files (written before this field existed) loadable.
    #[serde(default)]
    pub display_name: String,
    pub patient_name: String,
    pub registered_at: String,
}

/// Legacy single-provider registration record (migrated to providers.json).
#[derive(Serialize, Deserialize, Clone)]
pub struct RegistrationState {
    pub patient_name: String,
    pub registered_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DecryptedPayload {
    pub id: String,
    #[serde(rename = "type")]
    pub payload_type: String,
    pub content: String,
    pub provider_id: String,
    pub received_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct PayloadFilter {
    #[serde(rename = "type")]
    pub payload_type: Option<String>,
    pub provider_id: Option<String>,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Check whether a keypair and vault already exist (first-run detection).
#[tauri::command]
pub fn vault_exists() -> bool {
    keypair_path().exists()
}

/// Generate and persist a new hybrid keypair. Fails if a keypair already exists.
#[tauri::command]
pub fn generate_keypair() -> Result<(), String> {
    let path = keypair_path();
    if path.exists() {
        return Err("Keypair already exists. To regenerate, delete the existing vault.".to_string());
    }
    std::fs::create_dir_all(vault_dir()).map_err(|e| e.to_string())?;
    let keypair = crate::crypto::generate_keypair();
    let json = serde_json::to_string_pretty(&keypair).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_keypair() -> Result<HybridKeypair, String> {
    let json = std::fs::read_to_string(keypair_path()).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Return the patient's usb_id as a hex string.
#[tauri::command]
pub fn get_usb_id() -> Result<String, String> {
    let keypair = load_keypair()?;
    let pk = HybridPublicKey {
        x25519: keypair.x25519_public,
        mlkem: keypair.mlkem_encap.clone(),
    };
    let id_bytes = derive_usb_id(&pk);
    Ok(hex::encode(id_bytes))
}

/// Return the patient's hybrid public key as base64(x25519_pub || mlkem_encap).
#[tauri::command]
pub fn get_public_key() -> Result<String, String> {
    let keypair = load_keypair()?;
    let mut combined = Vec::with_capacity(32 + keypair.mlkem_encap.len());
    combined.extend_from_slice(&keypair.x25519_public);
    combined.extend_from_slice(&keypair.mlkem_encap);
    Ok(BASE64.encode(&combined))
}

/// Write a new vault entry to the local vault.
#[tauri::command]
pub fn write_vault_entry(entry: VaultEntry) -> Result<(), String> {
    let mut entries = read_all_entries()?;
    entries.push(entry);
    let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
    std::fs::write(vault_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Read vault entries, optionally filtered.
#[tauri::command]
pub fn read_vault_entries(filter: VaultFilter) -> Result<Vec<VaultEntry>, String> {
    let entries = read_all_entries()?;
    Ok(entries
        .into_iter()
        .filter(|e| {
            if filter.exclude_private && e.is_private {
                return false;
            }
            if let Some(ref cat) = filter.category {
                if &e.category != cat {
                    return false;
                }
            }
            true
        })
        .collect())
}

fn read_all_entries() -> Result<Vec<VaultEntry>, String> {
    let path = vault_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Register this device with a provider using a 6-digit pairing code.
/// `display_name` is the patient's local label for the provider (e.g. "Dr. Patel").
#[tauri::command]
pub async fn register_patient(
    patient_name: String,
    provider_code: String,
    display_name: String,
) -> Result<(), String> {
    let name = patient_name.trim();
    let code = provider_code.trim();
    let label = display_name.trim();
    if name.is_empty() {
        return Err("Patient name is required.".to_string());
    }
    if label.is_empty() {
        return Err("A name for this provider is required.".to_string());
    }
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return Err("Pairing code must be 6 digits.".to_string());
    }

    let usb_id = get_usb_id()?;
    let public_key = get_public_key()?;
    let provider_id =
        crate::sync::register_with_supabase(&usb_id, &public_key, name, code).await?;

    let link = ProviderLink {
        provider_id,
        display_name: label.to_string(),
        patient_name: name.to_string(),
        registered_at: chrono_now(),
    };
    upsert_provider_link(link)?;
    Ok(())
}

fn upsert_provider_link(link: ProviderLink) -> Result<(), String> {
    let mut links = read_provider_links()?;
    if let Some(existing) = links
        .iter_mut()
        .find(|l| l.provider_id == link.provider_id)
    {
        existing.display_name = link.display_name;
        existing.patient_name = link.patient_name;
        existing.registered_at = link.registered_at;
    } else {
        links.push(link);
    }
    write_provider_links(&links)
}

fn read_provider_links() -> Result<Vec<ProviderLink>, String> {
    let path = providers_path();
    if path.exists() {
        let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        return serde_json::from_str(&json).map_err(|e| e.to_string());
    }

    // Migrate legacy single-provider registration.json if present.
    let legacy = registration_path();
    if legacy.exists() {
        let json = std::fs::read_to_string(&legacy).map_err(|e| e.to_string())?;
        let old: RegistrationState = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let migrated = vec![ProviderLink {
            provider_id: String::new(),
            display_name: "Your provider".to_string(),
            patient_name: old.patient_name,
            registered_at: old.registered_at,
        }];
        write_provider_links(&migrated)?;
        return Ok(migrated);
    }

    Ok(vec![])
}

fn write_provider_links(links: &[ProviderLink]) -> Result<(), String> {
    std::fs::create_dir_all(vault_dir()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(links).map_err(|e| e.to_string())?;
    std::fs::write(providers_path(), json).map_err(|e| e.to_string())
}

/// List all providers this device is paired with.
#[tauri::command]
pub fn list_provider_links() -> Result<Vec<ProviderLink>, String> {
    read_provider_links()
}

/// Return local registration state for the first paired provider (legacy API).
#[tauri::command]
pub fn get_registration_state() -> Result<Option<RegistrationState>, String> {
    let links = read_provider_links()?;
    Ok(links.first().map(|l| RegistrationState {
        patient_name: l.patient_name.clone(),
        registered_at: l.registered_at.clone(),
    }))
}

/// Fetch and decrypt new payloads from Supabase, persist to local vault.
/// Returns the count of new payloads fetched.
#[tauri::command]
pub async fn sync_payloads() -> Result<u32, String> {
    let keypair = load_keypair()?;
    let usb_id = get_usb_id()?;
    let remote_payloads = crate::sync::fetch_remote_payloads(&usb_id).await?;

    let mut cached = read_all_payloads()?;
    let known_ids: std::collections::HashSet<String> =
        cached.iter().map(|p| p.id.clone()).collect();

    let mut new_count = 0u32;
    for remote in remote_payloads {
        if known_ids.contains(&remote.id) {
            continue;
        }

        let blob_raw = match remote.encrypted_blob.as_deref() {
            Some(blob) => blob.to_string(),
            None => {
                let storage_path = remote
                    .storage_path
                    .as_deref()
                    .ok_or_else(|| format!("Payload {} has no encrypted data", remote.id))?;
                crate::sync::download_encrypted_blob(storage_path).await?
            }
        };

        let decrypted = crate::sync::process_payload(
            &remote,
            &blob_raw,
            &keypair,
            &documents_dir(),
        )?;

        cached.push(decrypted);
        new_count += 1;
    }

    if new_count > 0 {
        let json = serde_json::to_string_pretty(&cached).map_err(|e| e.to_string())?;
        std::fs::write(payloads_path(), json).map_err(|e| e.to_string())?;
    }

    Ok(new_count)
}

fn read_all_payloads() -> Result<Vec<DecryptedPayload>, String> {
    let path = payloads_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

/// Read locally cached decrypted payloads, optionally filtered.
#[tauri::command]
pub fn read_payloads(filter: PayloadFilter) -> Result<Vec<DecryptedPayload>, String> {
    let payloads = read_all_payloads()?;
    Ok(payloads
        .into_iter()
        .filter(|p| {
            if let Some(ref t) = filter.payload_type {
                if &p.payload_type != t { return false; }
            }
            if let Some(ref pid) = filter.provider_id {
                if &p.provider_id != pid { return false; }
            }
            true
        })
        .collect())
}

/// Return a single questionnaire payload by id, or `None` if not found.
/// Used by the LLM `get_questionnaire` tool to walk through questions.
pub fn get_questionnaire(payload_id: &str) -> Result<Option<DecryptedPayload>, String> {
    let payloads = read_all_payloads()?;
    Ok(payloads
        .into_iter()
        .find(|p| p.id == payload_id && p.payload_type == "questionnaire"))
}

// ── Meeting transcripts ─────────────────────────────────────────────────────────

fn transcripts_path() -> PathBuf {
    vault_dir().join("transcripts.json")
}

/// A locally stored meeting-recording transcript.
#[derive(Serialize, Deserialize, Clone)]
pub struct Transcript {
    pub id: String,
    pub title: String,
    /// Unix seconds (as a string) when the transcript was saved.
    pub created_at: String,
    pub content: String,
}

fn read_all_transcripts() -> Result<Vec<Transcript>, String> {
    let path = transcripts_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Fetch meeting transcripts, optionally bounded by a `from`/`to` Unix-second
/// window (inclusive). Unparsable bounds are ignored rather than erroring, so a
/// best-effort range from the LLM still returns useful results.
#[tauri::command]
pub fn get_transcripts(
    from: Option<String>,
    to: Option<String>,
) -> Result<Vec<Transcript>, String> {
    let from_secs = from.as_deref().and_then(|s| s.trim().parse::<i64>().ok());
    let to_secs = to.as_deref().and_then(|s| s.trim().parse::<i64>().ok());
    Ok(read_all_transcripts()?
        .into_iter()
        .filter(|t| {
            let secs = t.created_at.parse::<i64>().unwrap_or(0);
            if let Some(lo) = from_secs {
                if secs < lo {
                    return false;
                }
            }
            if let Some(hi) = to_secs {
                if secs > hi {
                    return false;
                }
            }
            true
        })
        .collect())
}

/// Persist a meeting-recording transcript to the local vault. Returns its id.
#[tauri::command]
pub fn save_transcript(title: String, content: String) -> Result<String, String> {
    let mut transcripts = read_all_transcripts()?;
    let now = chrono_now();
    let id = format!("transcript-{}-{}", now, transcripts.len());
    transcripts.push(Transcript {
        id: id.clone(),
        title: if title.trim().is_empty() {
            "Untitled meeting".to_string()
        } else {
            title.trim().to_string()
        },
        created_at: now,
        content,
    });
    std::fs::create_dir_all(vault_dir()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&transcripts).map_err(|e| e.to_string())?;
    std::fs::write(transcripts_path(), json).map_err(|e| e.to_string())?;
    Ok(id)
}

// ── Questionnaire responses ─────────────────────────────────────────────────────

fn responses_path() -> PathBuf {
    vault_dir().join("questionnaire_responses.json")
}

/// A patient's saved answer to one questionnaire question.
#[derive(Serialize, Deserialize, Clone)]
pub struct QuestionnaireResponse {
    pub payload_id: String,
    pub question_id: String,
    pub response: String,
    /// Unix seconds (as a string) when the answer was recorded.
    pub answered_at: String,
}

fn read_all_responses() -> Result<Vec<QuestionnaireResponse>, String> {
    let path = responses_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Save (or replace) a patient's voice response to a questionnaire question.
/// A later answer to the same (`payload_id`, `question_id`) overwrites the prior.
#[tauri::command]
pub fn save_questionnaire_response(
    payload_id: String,
    question_id: String,
    response: String,
) -> Result<(), String> {
    if payload_id.trim().is_empty() || question_id.trim().is_empty() {
        return Err("payload_id and question_id are required.".to_string());
    }
    let mut responses = read_all_responses()?;
    let answered_at = chrono_now();
    if let Some(existing) = responses
        .iter_mut()
        .find(|r| r.payload_id == payload_id && r.question_id == question_id)
    {
        existing.response = response;
        existing.answered_at = answered_at;
    } else {
        responses.push(QuestionnaireResponse {
            payload_id,
            question_id,
            response,
            answered_at,
        });
    }
    std::fs::create_dir_all(vault_dir()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&responses).map_err(|e| e.to_string())?;
    std::fs::write(responses_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Read all saved questionnaire responses for a specific questionnaire payload.
/// Used by the LLM/provider debrief to report what the patient answered.
pub fn read_responses_for(payload_id: &str) -> Result<Vec<QuestionnaireResponse>, String> {
    Ok(read_all_responses()?
        .into_iter()
        .filter(|r| r.payload_id == payload_id)
        .collect())
}
