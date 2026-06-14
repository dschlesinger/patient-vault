//! Resolves on-device binary and model asset paths for the USB app.
//!
//! Tails OS constraint (see PROJECT.md): writable locations such as Persistent
//! Storage and `$HOME` are mounted `noexec`, so *executable* binaries must be
//! run from the installed application directory (under `/usr` via the `.deb`),
//! never from Persistent Storage. Large model/data files are read-only data —
//! `noexec` is irrelevant — so they live in Persistent Storage where there is
//! room for multi-gigabyte files.
//!
//! Two resolvers encode that split:
//!   - [`binary_path`] looks only in installed/bundled executable locations.
//!   - [`model_path`] prefers Persistent Storage, then bundled resources.

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Env override for the directory containing engine binaries
/// (`llama-server`, `whisper-stream`, `whisper-cli`, `piper`). Used in dev/tests.
pub const BIN_DIR_ENV: &str = "PATIENT_VAULT_BIN_DIR";

/// Env override for the directory containing model/data assets
/// (GGUF, whisper `ggml-base.bin`, Piper voices). Used in dev/tests.
pub const MODEL_DIR_ENV: &str = "PATIENT_VAULT_MODEL_DIR";

/// Return the first `dir.join(name)` that exists, if any.
fn first_existing(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.exists())
}

/// Candidate directories for executable binaries, most-specific first.
///
/// `resource_dir` and `exe_dir` are passed in (rather than read from an
/// `AppHandle`) so the ordering logic is unit-testable without a Tauri runtime.
fn binary_dirs(
    env_override: Option<PathBuf>,
    resource_dir: Option<PathBuf>,
    exe_dir: Option<PathBuf>,
    manifest_dir: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = env_override {
        dirs.push(d);
    }
    if let Some(d) = resource_dir {
        dirs.push(d.join("bin"));
        dirs.push(d);
    }
    if let Some(d) = exe_dir {
        dirs.push(d.join("bin"));
        dirs.push(d);
    }
    if let Some(d) = manifest_dir {
        dirs.push(d.join("resources").join("bin"));
    }
    // Standard install location for the Tails `.deb`.
    dirs.push(PathBuf::from("/usr/lib/patient-vault/bin"));
    dirs
}

/// Candidate directories for model/data assets, most-specific first.
fn model_dirs(
    env_override: Option<PathBuf>,
    persistent_models: Option<PathBuf>,
    resource_dir: Option<PathBuf>,
    exe_dir: Option<PathBuf>,
    manifest_dir: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = env_override {
        dirs.push(d);
    }
    if let Some(d) = persistent_models {
        dirs.push(d);
    }
    if let Some(d) = resource_dir {
        dirs.push(d.join("models"));
    }
    if let Some(d) = exe_dir {
        dirs.push(d.join("models"));
    }
    if let Some(d) = manifest_dir {
        dirs.push(d.join("resources").join("models"));
    }
    dirs
}

fn env_dir(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

fn manifest_dir() -> Option<PathBuf> {
    std::env::var("CARGO_MANIFEST_DIR").ok().map(PathBuf::from)
}

/// Persistent Storage models directory (`~/Persistent/patient-vault/models`),
/// matching the vault's storage location convention.
fn persistent_models_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let dir = PathBuf::from(home)
        .join("Persistent")
        .join("patient-vault")
        .join("models");
    Some(dir)
}

/// Resolve an executable binary by name (e.g. `"llama-server"`).
///
/// Searches installed/bundled locations only — never Persistent Storage, which
/// is `noexec` on Tails. Falls back to the bare name (PATH lookup) so a binary
/// available on `$PATH` during development still works.
pub fn binary_path(app: &AppHandle, name: &str) -> PathBuf {
    let dirs = binary_dirs(
        env_dir(BIN_DIR_ENV),
        app.path().resource_dir().ok(),
        exe_dir(),
        manifest_dir(),
    );
    first_existing(&dirs, name).unwrap_or_else(|| PathBuf::from(name))
}

/// Resolve a model/data asset by relative path (e.g. `"whisper/ggml-base.bin"`).
///
/// Prefers Persistent Storage, then bundled resources. Returns an error if the
/// asset cannot be found so callers can surface an actionable "model not
/// installed" message to the UI.
pub fn model_path(app: &AppHandle, rel: &str) -> Result<PathBuf, String> {
    let dirs = model_dirs(
        env_dir(MODEL_DIR_ENV),
        persistent_models_dir(),
        app.path().resource_dir().ok(),
        exe_dir(),
        manifest_dir(),
    );
    first_existing(&dirs, rel).ok_or_else(|| {
        format!(
            "Model asset `{rel}` not found. Install it under Persistent Storage \
             (~/Persistent/patient-vault/models/) or set {MODEL_DIR_ENV}."
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn first_existing_picks_first_present_dir() {
        let tmp = std::env::temp_dir().join(format!("pv-assets-{}", std::process::id()));
        let a = tmp.join("a");
        let b = tmp.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        fs::write(b.join("thing"), b"x").unwrap();

        // `a` is searched first but lacks the file; `b` has it.
        let found = first_existing(&[a.clone(), b.clone()], "thing").unwrap();
        assert_eq!(found, b.join("thing"));

        // Missing everywhere → None.
        assert!(first_existing(&[a, b], "missing").is_none());

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn binary_dirs_never_includes_persistent_storage() {
        let dirs = binary_dirs(
            Some(PathBuf::from("/override")),
            Some(PathBuf::from("/res")),
            Some(PathBuf::from("/opt/app")),
            Some(PathBuf::from("/work")),
        );
        // The env override is honoured first.
        assert_eq!(dirs.first(), Some(&PathBuf::from("/override")));
        // The installed deb location is always a fallback.
        assert!(dirs.contains(&PathBuf::from("/usr/lib/patient-vault/bin")));
        // No Persistent Storage path is ever an execution candidate.
        assert!(dirs.iter().all(|d| !d.to_string_lossy().contains("Persistent")));
    }

    #[test]
    fn model_dirs_prefers_persistent_over_resources() {
        let dirs = model_dirs(
            None,
            Some(PathBuf::from("/home/user/Persistent/patient-vault/models")),
            Some(PathBuf::from("/res")),
            None,
            None,
        );
        let persistent_idx = dirs
            .iter()
            .position(|d| d.to_string_lossy().contains("Persistent"))
            .unwrap();
        let resource_idx = dirs
            .iter()
            .position(|d| d == &PathBuf::from("/res").join("models"))
            .unwrap();
        assert!(persistent_idx < resource_idx);
    }
}
