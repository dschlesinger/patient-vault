# PatientVault — Setup Guide

## Repository Structure

```
patient-vault/
├── provider-frontend/   SvelteKit 5 web app (provider dashboard)
└── usb-app/             Tauri 2 desktop app (patient/provider USB interface)
```

Both are pnpm workspace packages managed from the repo root.

---

## Prerequisites

| Tool | Version | Install |
|---|---|---|
| Node.js | 22+ (LTS) | https://nodejs.org |
| pnpm | 10+ | `npm install -g pnpm` |
| Rust (stable) | Latest stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| WebKit2GTK (Linux) | — | `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev` |

**Note**: `liboqs` is NOT required. PatientVault uses the pure-Rust `ml-kem` crate (RustCrypto, FIPS 203) with no C dependencies — Tails OS compatible.

Optional:
- Supabase CLI: `pnpm dlx supabase` (for local DB development)

---

## First-Time Setup

```bash
# 1. Install all workspace dependencies
pnpm install

# 2. Set up provider frontend environment
cp provider-frontend/.env.example provider-frontend/.env
# Edit provider-frontend/.env and fill in your Supabase project values
```

---

## Supabase Setup (Provider Frontend)

1. Create a project at https://supabase.com
2. Enable **Email** auth provider (Authentication → Providers)
3. Create a Storage bucket named `documents` and set it to **public**
4. Run the SQL migrations (will live in `provider-frontend/supabase/migrations/` — not yet created)
5. Copy **Project URL** and **anon/public key** to `provider-frontend/.env`:
   ```
   PUBLIC_SUPABASE_URL=https://<ref>.supabase.co
   PUBLIC_SUPABASE_ANON_KEY=<key>
   ```

---

## Running in Development

```bash
# Provider web frontend (http://localhost:5173)
pnpm dev:provider

# USB app (Tauri webview — requires Rust + WebKit2GTK)
pnpm dev:usb

# TypeScript checks
pnpm check:provider
pnpm check:usb
```

---

## Building

```bash
# Provider frontend (outputs to provider-frontend/build/)
pnpm build:provider

# USB app — produces .deb package for Tails OS deployment
pnpm build:usb
```

---

## Model Assets (USB App)

Large binary files are gitignored. Download and place them manually:

| Asset | Destination |
|---|---|
| `Qwen2.5-7B-Instruct-Q4_K_M.gguf` | `usb-app/src-tauri/resources/models/` |
| `ggml-base.bin` (whisper base multilingual) | `usb-app/src-tauri/resources/whisper/` |
| `en_US-lessac-medium.onnx` + `.json` | `usb-app/src-tauri/resources/piper-voices/` |
| `es_MX-ald-medium.onnx` + `.json` | `usb-app/src-tauri/resources/piper-voices/` |
| `zh_CN-huayan-medium.onnx` + `.json` | `usb-app/src-tauri/resources/piper-voices/` |

Sources:
- Qwen2.5 GGUF: https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF
- whisper base: https://huggingface.co/ggerganov/whisper.cpp
- Piper voices: https://huggingface.co/rhasspy/piper-voices

The Piper binary itself comes from https://github.com/OHF-Voice/piper1-gpl (maintained fork of rhasspy/piper, archived Oct 2025).

---

## Tails OS Deployment

```bash
# Build the .deb package
pnpm build:usb

# Install on Tails via "Additional Software" (survives reboots)
sudo apt install ./usb-app/src-tauri/target/release/bundle/deb/patient-vault-usb_*.deb
```

The app writes the patient keypair and vault to `~/Persistent/patient-vault/` (Tails Persistent Storage).

---

## pnpm + Tauri Gotcha

**Never use `tauri add <plugin>`** — it doesn't detect pnpm when run inside a workspace (Tauri issues #11859 and #12706). Always add dependencies manually:

1. Add the Cargo crate to `usb-app/src-tauri/Cargo.toml`
2. Add the npm package to `usb-app/package.json`
3. Run `pnpm install` from the repo root

---

## Verifying the Rust Build

Rust is required for USB app compilation. After installing Rust:

```bash
cd usb-app/src-tauri
cargo check        # fast type-check without linking
cargo clippy -- -D warnings   # lint
```

The Rust module stubs (`crypto/`, `vault/`, `llm/`, `stt/`, `tts/`) use `todo!()` for unimplemented bodies. `cargo check` will succeed; `cargo build` will panic at runtime on unimplemented functions.
