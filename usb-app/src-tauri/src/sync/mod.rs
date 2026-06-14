use crate::crypto::{decrypt, parse_blob, HybridKeypair};
use crate::vault::DecryptedPayload;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RemotePayload {
    pub id: String,
    #[serde(rename = "type")]
    pub payload_type: String,
    pub encrypted_blob: Option<String>,
    pub storage_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    ok: bool,
    error: Option<String>,
    provider_id: Option<String>,
}

pub fn supabase_config() -> Result<(String, String)> {
    if let Ok(url) = std::env::var("SUPABASE_URL") {
        if let Ok(key) = std::env::var("SUPABASE_ANON_KEY") {
            return Ok((url.trim_end_matches('/').to_string(), key));
        }
    }

    if let Some((url, key)) = read_supabase_from_env_files() {
        std::env::set_var("SUPABASE_URL", &url);
        std::env::set_var("SUPABASE_ANON_KEY", &key);
        return Ok((url, key));
    }

    Err(anyhow::anyhow!(
        "SUPABASE_URL and SUPABASE_ANON_KEY must be set (env or provider-frontend/.env)"
    ))
}

fn read_supabase_from_env_files() -> Option<(String, String)> {
    for candidate in env_file_candidates() {
        if let Some((url, key)) = parse_supabase_env_file(&candidate) {
            return Some((url, key));
        }
    }
    None
}

fn env_file_candidates() -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(".env"));
        candidates.push(cwd.join("../provider-frontend/.env"));
        candidates.push(cwd.join("provider-frontend/.env"));
        candidates.push(cwd.join("../../provider-frontend/.env"));
    }
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest = std::path::PathBuf::from(manifest_dir);
        candidates.push(manifest.join(".env"));
        candidates.push(manifest.join("../../provider-frontend/.env"));
    }
    candidates
}

fn parse_supabase_env_file(path: &std::path::Path) -> Option<(String, String)> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut url: Option<String> = None;
    let mut key: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let value = v.trim().trim_matches('"').to_string();
        match k.trim() {
            "SUPABASE_URL" | "PUBLIC_SUPABASE_URL" => url = Some(value),
            "SUPABASE_ANON_KEY" | "PUBLIC_SUPABASE_ANON_KEY" => key = Some(value),
            _ => {}
        }
    }

    match (url, key) {
        (Some(url), Some(key)) if !url.is_empty() && !key.is_empty() => {
            Some((url.trim_end_matches('/').to_string(), key))
        }
        _ => None,
    }
}

fn auth_headers(key: &str) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("apikey", key.parse().expect("valid apikey header"));
    headers.insert(
        "Authorization",
        format!("Bearer {key}").parse().expect("valid auth header"),
    );
    headers
}

pub async fn register_with_supabase(
    usb_id: &str,
    public_key: &str,
    patient_name: &str,
    provider_code: &str,
) -> Result<String, String> {
    let (url, key) = supabase_config().map_err(|e| e.to_string())?;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{url}/rest/v1/rpc/register_patient"))
        .headers(auth_headers(&key))
        .json(&serde_json::json!({
            "p_usb_id": usb_id,
            "p_public_key": public_key,
            "p_patient_name": patient_name,
            "p_provider_code": provider_code,
        }))
        .send()
        .await
        .map_err(|e| format!("Registration request failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Registration failed ({body})"));
    }

    let result: RegisterResponse = resp
        .json()
        .await
        .map_err(|e| format!("Invalid registration response: {e}"))?;

    if result.ok {
        result
            .provider_id
            .filter(|id| !id.is_empty())
            .ok_or_else(|| "Registration succeeded but provider_id was missing.".to_string())
    } else {
        Err(result
            .error
            .unwrap_or_else(|| "Registration failed.".to_string()))
    }
}

pub async fn fetch_remote_payloads(usb_id: &str) -> Result<Vec<RemotePayload>, String> {
    let (url, key) = supabase_config().map_err(|e| e.to_string())?;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{url}/rest/v1/payloads"))
        .headers(auth_headers(&key))
        .query(&[
            ("usb_id", format!("eq.{usb_id}")),
            (
                "select",
                "id,type,encrypted_blob,storage_path,created_at".to_string(),
            ),
            ("order", "created_at.asc".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to fetch payloads: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Payload fetch failed ({body})"));
    }

    resp.json::<Vec<RemotePayload>>()
        .await
        .map_err(|e| format!("Invalid payloads response: {e}"))
}

pub async fn download_encrypted_blob(storage_path: &str) -> Result<String, String> {
    let (url, key) = supabase_config().map_err(|e| e.to_string())?;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{url}/storage/v1/object/public/documents/{storage_path}"))
        .headers(auth_headers(&key))
        .send()
        .await
        .map_err(|e| format!("Failed to download document: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Document download failed ({body})"));
    }

    resp.text()
        .await
        .map_err(|e| format!("Failed to read downloaded document: {e}"))
}

pub fn process_payload(
    remote: &RemotePayload,
    blob_raw: &str,
    keypair: &HybridKeypair,
    documents_dir: &std::path::Path,
) -> Result<DecryptedPayload, String> {
    let blob = parse_blob(blob_raw.trim()).map_err(|e| e.to_string())?;
    let plaintext = decrypt(&blob, keypair).map_err(|e| format!("Decrypt failed: {e}"))?;

    let mut provider_id = String::new();

    let content = match remote.payload_type.as_str() {
        "document" => {
            let (name, mime, file_bytes, doc_provider_id) = unpack_document(&plaintext)?;
            if let Some(pid) = doc_provider_id {
                provider_id = pid;
            }
            std::fs::create_dir_all(documents_dir).map_err(|e| e.to_string())?;
            let safe_name = sanitize_filename(&name);
            let file_path = documents_dir.join(format!("{}_{}", remote.id, safe_name));
            std::fs::write(&file_path, &file_bytes).map_err(|e| e.to_string())?;
            serde_json::json!({
                "name": name,
                "type": mime,
                "path": file_path.to_string_lossy(),
                "size": file_bytes.len(),
                "provider_id": provider_id,
            })
            .to_string()
        }
        "message" | "questionnaire" => {
            let text = String::from_utf8(plaintext)
                .map_err(|e| format!("Payload is not valid UTF-8: {e}"))?;
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(pid) = parsed.get("provider_id").and_then(|v| v.as_str()) {
                    provider_id = pid.to_string();
                }
            }
            text
        }
        other => return Err(format!("Unknown payload type: {other}")),
    };

    Ok(DecryptedPayload {
        id: remote.id.clone(),
        payload_type: remote.payload_type.clone(),
        content,
        provider_id,
        received_at: remote.created_at.clone(),
    })
}

fn unpack_document(plaintext: &[u8]) -> Result<(String, String, Vec<u8>, Option<String>), String> {
    if plaintext.len() < 4 {
        return Err("Document payload too short".to_string());
    }
    let meta_len = u32::from_be_bytes(plaintext[0..4].try_into().unwrap()) as usize;
    if plaintext.len() < 4 + meta_len {
        return Err("Document metadata length invalid".to_string());
    }
    let meta: serde_json::Value = serde_json::from_slice(&plaintext[4..4 + meta_len])
        .map_err(|e| format!("Invalid document metadata JSON: {e}"))?;
    let name = meta
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("document")
        .to_string();
    let mime = meta
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("application/octet-stream")
        .to_string();
    let provider_id = meta
        .get("provider_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let file_bytes = plaintext[4 + meta_len..].to_vec();
    Ok((name, mime, file_bytes, provider_id))
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_document_matches_browser_layout() {
        let meta = br#"{"name":"report.pdf","type":"application/pdf"}"#;
        let mut packed = Vec::new();
        packed.extend_from_slice(&(meta.len() as u32).to_be_bytes());
        packed.extend_from_slice(meta);
        packed.extend_from_slice(b"%PDF-1.4");

        let (name, mime, bytes, provider_id) = unpack_document(&packed).unwrap();
        assert_eq!(name, "report.pdf");
        assert_eq!(mime, "application/pdf");
        assert_eq!(bytes, b"%PDF-1.4");
        assert_eq!(provider_id, None);
    }
}
