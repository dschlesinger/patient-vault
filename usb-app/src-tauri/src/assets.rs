//! Resolves on-device binary and model asset paths for the USB app
//! (Ubuntu, portable USB model — see PROJECT.md).
//!
//! In the portable model the app binary, its engine binaries, and the (multi-GB)
//! model/data files all live together on the USB drive, resolved relative to the
//! running executable. A normal `.deb` install is also supported, in which case
//! binaries come from the installed application directory.
//!
//! Two resolvers:
//!   - [`binary_path`] looks in bundled/installed executable locations.
//!   - [`model_path`] prefers the portable data directory, then bundled resources.

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
    // Standard install location for a system-wide `.deb` install.
    dirs.push(PathBuf::from("/usr/lib/patient-vault/bin"));
    dirs
}

/// Candidate directories for model/data assets, most-specific first.
fn model_dirs(
    env_override: Option<PathBuf>,
    portable_models: Option<PathBuf>,
    resource_dir: Option<PathBuf>,
    exe_dir: Option<PathBuf>,
    manifest_dir: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = env_override {
        dirs.push(d);
    }
    if let Some(d) = portable_models {
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

/// Portable models directory on the USB drive, next to the app binary
/// (`<exe_dir>/patient-vault-data/models`) — matches the vault's data location.
fn portable_models_dir() -> Option<PathBuf> {
    exe_dir().map(|d| d.join("patient-vault-data").join("models"))
}

/// Resolve an executable binary by name (e.g. `"llama-server"`).
///
/// Searches the bundled/installed executable locations, then falls back to the
/// bare name (PATH lookup) so a binary available on `$PATH` during development
/// still works.
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
/// Prefers the portable data directory on the USB drive, then bundled resources.
/// Returns an error if the asset cannot be found so callers can surface an
/// actionable "model not installed" message to the UI.
pub fn model_path(app: &AppHandle, rel: &str) -> Result<PathBuf, String> {
    let dirs = model_dirs(
        env_dir(MODEL_DIR_ENV),
        portable_models_dir(),
        app.path().resource_dir().ok(),
        exe_dir(),
        manifest_dir(),
    );
    first_existing(&dirs, rel).ok_or_else(|| {
        format!(
            "Model asset `{rel}` not found. Place it in the app's \
             `patient-vault-data/models/` directory on the USB drive or set {MODEL_DIR_ENV}."
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
    fn binary_dirs_orders_override_first_and_includes_install_fallback() {
        let dirs = binary_dirs(
            Some(PathBuf::from("/override")),
            Some(PathBuf::from("/res")),
            Some(PathBuf::from("/opt/app")),
            Some(PathBuf::from("/work")),
        );
        // The env override is honoured first.
        assert_eq!(dirs.first(), Some(&PathBuf::from("/override")));
        // The system-wide install location is always a fallback.
        assert!(dirs.contains(&PathBuf::from("/usr/lib/patient-vault/bin")));
        // The portable location next to the binary is a candidate.
        assert!(dirs.contains(&PathBuf::from("/opt/app/bin")));
    }

    #[test]
    fn model_dirs_prefers_portable_data_over_resources() {
        let dirs = model_dirs(
            None,
            Some(PathBuf::from("/media/user/PVAULT/patient-vault-data/models")),
            Some(PathBuf::from("/res")),
            None,
            None,
        );
        let portable_idx = dirs
            .iter()
            .position(|d| d.to_string_lossy().contains("patient-vault-data"))
            .unwrap();
        let resource_idx = dirs
            .iter()
            .position(|d| d == &PathBuf::from("/res").join("models"))
            .unwrap();
        assert!(portable_idx < resource_idx);
    }
}
