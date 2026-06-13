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
| Tauri system libs (Linux) | — | `sudo apt install libdbus-1-dev libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev pkg-config` |

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
3. Run migrations — the `documents` bucket and public-read policy are created automatically:
   ```bash
   # Option A: Supabase CLI (linked to your project)
   cd provider-frontend && supabase db push

   # Option B: paste each file in provider-frontend/supabase/migrations/ into
   #           Supabase Dashboard → SQL Editor (in filename order)
   ```
4. Copy **Project URL** and **legacy JWT anon key** (`eyJ…` from Settings → API) to `provider-frontend/.env`:
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

### Dev logging

Set `LOG_FOLDER` in `provider-frontend/.env` (see `.env.example`). Both apps write there:

| File | App | Contents |
|---|---|---|
| `frontend.log` | Provider frontend | HTTP requests + `console.*` output |
| `usb-app.log` | USB app (Rust) | `tracing::info!` and above |

The provider frontend reads `LOG_FOLDER` through SvelteKit's `$env/dynamic/private`. The USB app uses the shell `LOG_FOLDER` if set; otherwise it loads the value from `provider-frontend/.env` automatically.

Override log verbosity for the USB app with `RUST_LOG` (default: `info`).

The USB app also reads `PUBLIC_SUPABASE_URL` and `PUBLIC_SUPABASE_ANON_KEY` from `provider-frontend/.env` when sync/register runs (same file as the provider frontend).

### Patient–provider pairing

Pairing links a USB device's cryptographic identity to a provider account via a one-time 6-digit code.

**1. Apply migrations** (required once — includes `register_patient` RPC):

```bash
cd provider-frontend
pnpm dlx supabase link --project-ref <your-project-ref>
pnpm dlx supabase db push
```

Or paste each file in `provider-frontend/supabase/migrations/` into Supabase Dashboard → SQL Editor (filename order).

**2. Provider portal**

```bash
pnpm dev:provider
```

- Open http://localhost:5173
- **Create account** (or sign in)
- Click **+ Pair new patient** — note the 6-digit code (expires in 10 minutes)

**3. USB app**

```bash
pnpm dev:usb
```

- **Generate identity** (first run only)
- Enter your name + the 6-digit code → **Register with provider**
- You should see **✓ Paired with provider** on the home screen
- The patient now appears on the provider dashboard

**4. Verify**

- Provider dashboard lists the patient by name
- Send a message from the provider portal → USB app **Sync** → **Messages** tab

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
