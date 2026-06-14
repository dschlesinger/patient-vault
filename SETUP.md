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
| Engine build tools (for `fetch-assets.sh`) | — | `sudo apt install git cmake build-essential libsdl2-dev` |
| SDL2 runtime (mic capture for `whisper-stream`) | — | `sudo apt install libsdl2-2.0-0` |

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

## AI Engines (LLM / STT / TTS)

The USB app runs three local engines as child processes — no network, all on-device:

| Subsystem | Engine | Model |
|---|---|---|
| LLM (chat + tool calls) | `llama-server` (llama.cpp) over loopback HTTP | Qwen2.5 7B Instruct Q4_K_M GGUF |
| STT (speech→text) | `whisper-stream` (live) / `whisper-cli` (batch) | Whisper `base` multilingual |
| TTS (text→speech) | `piper` (rhasspy v1.2.0 CLI) | Piper voices (en/es/zh) |

**Two asset classes, two locations** (a Tails constraint — see below):

- **Binaries** (`llama-server`, `whisper-stream`, `whisper-cli`, `piper` + their `.so` libs) → `usb-app/src-tauri/resources/bin/`. These are bundled into the `.deb` and installed to `/usr/lib/patient-vault/bin/` (an *executable* location).
- **Models** (GGUF, `ggml-base.bin`, Piper `.onnx`/`.onnx.json`) → `usb-app/src-tauri/resources/models/{llm,whisper,piper}/` for development, or Persistent Storage for Tails. These are read-only *data*, never executed.

### One-command fetch

All large files are gitignored. Download/build them with:

```bash
cd usb-app
./scripts/fetch-assets.sh            # models + all three engine binaries (idempotent)
# or selectively:
./scripts/fetch-assets.sh models     # models only
./scripts/fetch-assets.sh llama      # just llama-server
./scripts/fetch-assets.sh whisper    # just whisper-stream + whisper-cli (needs libsdl2-dev)
./scripts/fetch-assets.sh piper      # just the Piper CLI
```

The script downloads Qwen2.5 GGUF (bartowski), Whisper `ggml-base.bin` (ggerganov), and Piper voices (rhasspy/piper-voices); it builds `llama.cpp` and `whisper.cpp` from source (CMake, `whisper-stream` with `-DWHISPER_SDL2=ON`) and extracts the pinned Piper v1.2.0 release. URLs/tags are pinned at the top of the script — re-verify before a release.

### Path resolution overrides (dev/test)

`src-tauri/src/assets.rs` resolves binaries and models from several locations. Override either for development without touching the bundle:

| Env var | Purpose |
|---|---|
| `PATIENT_VAULT_BIN_DIR` | Directory containing the engine binaries |
| `PATIENT_VAULT_MODEL_DIR` | Directory containing `llm/`, `whisper/`, `piper/` model subdirs |

Models also resolve automatically from `~/Persistent/patient-vault/models/` (Tails).

---

## Tails OS Deployment

PatientVault is designed for Tails' security model:

- **Binaries run from an executable location.** Tails mounts Persistent Storage and `$HOME` `noexec`, so the engine binaries are bundled into the `.deb` and installed to `/usr/lib/patient-vault/bin/`. The model *data* files live in Persistent Storage (no execution, room for multi-GB files).
- **LLM uses loopback HTTP.** `llama-server` binds `127.0.0.1` on an ephemeral port (Tor-reserved ports are avoided); loopback is permitted for the `amnesia` user.
- **Microphone is unmediated** for the `amnesia` user, so `whisper-stream` captures directly via SDL2.

```bash
# 1. Build the .deb (binaries must already be in resources/bin — run fetch-assets.sh first)
pnpm build:usb

# 2. Install on Tails via "Additional Software" (survives reboots)
sudo apt install ./usb-app/src-tauri/target/release/bundle/deb/patient-vault-usb_*.deb

# 3. Place models in Persistent Storage (run once, from a machine with network)
MODELS_DIR="$HOME/Persistent/patient-vault/models" ./usb-app/scripts/fetch-assets.sh models
```

The app writes the patient keypair and vault to `~/Persistent/patient-vault/` (Tails Persistent Storage).

### Tails verification checklist (validate on real hardware)

These could not be verified outside Tails and must be checked on-device:

- [ ] **CPU baseline** — build the llama.cpp/whisper.cpp binaries with `-DGGML_NATIVE=OFF` (the script does this) so they don't use instructions absent on the target CPU. Confirm they run.
- [ ] **SDL2** — `libsdl2-2.0-0` is declared as a `.deb` dependency; confirm `whisper-stream` finds it (or static-link SDL2 if "Additional Software" can't pull it).
- [ ] **AppArmor** — confirm no profile blocks `/usr/lib/patient-vault/bin/*` execution or loopback.
- [ ] **Additional Software persistence** — confirm the locally-installed `.deb` is re-applied across reboots.
- [ ] **RAM** — Qwen 7B Q4 needs ~5–6 GB resident; confirm headroom on the target machine.

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
cargo clippy       # lint
cargo test --lib   # unit tests (assets, llm SSE/tools, stt cleaning, tts, crypto)
```

The `crypto/`, `vault/`, `llm/`, `stt/`, and `tts/` modules are implemented. The LLM/STT/TTS commands spawn their engine subprocesses on demand; if a binary or model is missing they return a descriptive error (the patient/provider pages surface it and fall back to preview mode), so the app still builds and runs without the assets installed.
