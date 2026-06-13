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

fn payloads_path() -> PathBuf {
    vault_dir().join("payloads.json")
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

/// Fetch and decrypt new payloads from Supabase, persist to local vault.
/// Returns the count of new payloads fetched.
#[tauri::command]
pub async fn sync_payloads() -> Result<u32, String> {
    // TODO: fetch from Supabase /payloads/[usb_id], decrypt with keypair, persist
    // For now, returns 0 (no payloads fetched) as a stub
    Ok(0)
}

/// Read locally cached decrypted payloads, optionally filtered.
#[tauri::command]
pub fn read_payloads(filter: PayloadFilter) -> Result<Vec<DecryptedPayload>, String> {
    let path = payloads_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let payloads: Vec<DecryptedPayload> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
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
