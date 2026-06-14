# PatientVault — Implementation Checklist

> Last verified against source: 2026-06-13. Items are checked only after
> confirming the actual implementation, not just intent.

## Provider Web Frontend

**Encryption wiring (missing client-side crypto calls)**
- [x] Wire `hybrid.ts` `encryptPayload()` into the Send Message form — `use:enhance` encrypts to `encrypted_blob` before POST
- [x] Wire `hybrid.ts` `encryptPayload()` into the Send Questionnaire form — questions JSON-encrypted into one blob
- [x] Wire `hybrid.ts` `encryptPayload()` into the Send Document form — file bytes (with filename+mime packed in) encrypted before upload as `encrypted_file`
  - Also: fixed `hybrid.ts` to use the Web Crypto `X25519` algorithm name (not `ECDH`+`namedCurve`, which Chrome rejects)
  - Verified end-to-end via Chrome DevTools against live Supabase (4-segment ciphertext blobs; document metadata stays encrypted)

**Missing routes**
- [x] `/register` endpoint — open POST route + Supabase `register_patient` RPC for USB pairing

**Auth**
- [x] Provider sign-up/registration flow — `+page.svelte` has a Sign in / Create account toggle; `+page.server.ts` implements the `signup` action (`supabase.auth.signUp` with `name` metadata + password length validation) alongside `signin`

**Infrastructure**
- [x] SQL migration file(s) for all tables (`providers`, `pairing_codes`, `patient_provider_links`, `payloads`, `provider_sent_log`) + RLS policies — applied to live Supabase (`initial_schema`)
  - [x] Follow-up migration `payloads_public_read_policy`: added the missing `payloads` SELECT policy (`using (true)`) — without it `INSERT ... RETURNING` was blocked by RLS
- [x] Supabase Storage bucket creation (`documents`) — created in `initial_schema`
  - [x] **Storage read policy** (migration `documents_bucket_public_read`): bucket set `public = true` and SELECT policy replaced with `documents_public_read` (`using (bucket_id = 'documents')`); uploads stay provider-only. Verified an encrypted document downloads anonymously via the public object URL (HTTP 200, ciphertext bytes) — matches PROJECT.md's "encryption is the access control" design.

---

## USB App — Rust Backend

**Cryptography** (`src-tauri/src/crypto/mod.rs`)
- [x] `generate_keypair()` — X25519 + ML-KEM-768 key generation
- [x] `decrypt()` — X25519 ECDH + ML-KEM-768 decapsulate + HKDF-SHA256 + AES-256-GCM decrypt

**Sync** (`src-tauri/src/sync/mod.rs` — Supabase networking, split out of `vault/mod.rs`)
- [x] `register_with_supabase()` — POST to `register_patient` RPC
- [x] `fetch_remote_payloads()` / `download_encrypted_blob()` — REST + Storage fetch
- [x] `process_payload()` — parse blob, decrypt, unpack document metadata, write document bytes to `.vault/documents/` (has a unit test for the document layout)
- [x] Supabase config resolution from env or `provider-frontend/.env`

**Vault** (`src-tauri/src/vault/mod.rs`)
- [x] `sync_payloads()` Tauri command — orchestrates `sync::*`, dedupes by id, persists to `payloads.json`, returns new-payload count
- [x] `read_payloads()` — read cached decrypted payloads, filter by type / `provider_id`
- [x] `vault_exists()`, `generate_keypair()`, `get_usb_id()`, `get_public_key()` — first-run + identity commands
- [x] `register_patient()` Tauri command + multi-provider link storage (`providers.json`, with legacy `registration.json` migration)
- [x] `list_provider_links()` / `get_registration_state()` — provider link retrieval
- [x] `write_vault_entry()` / `read_vault_entries()` — vault entry persistence with `is_private` + category filtering (backend ready; no UI consumes these yet)
- [x] `save_questionnaire_response()` Tauri command — upsert per `(payload_id, question_id)` into `questionnaire_responses.json`
- [x] `get_transcripts(from?, to?)` Tauri command — Unix-seconds range filter over `transcripts.json`; plus `save_transcript()` for meeting-recording persistence and a `get_questionnaire()` helper

> Implemented: the LLM/STT/TTS commands below spawn real engine subprocesses
> via `tokio::process` and stream results over Tauri events. Process lifecycles
> are held in managed state (`LlmState`/`SttState`/`TtsState`). Binary/model
> paths resolve through `src-tauri/src/assets.rs` (binaries from executable
> locations, models from Persistent Storage — a Tails constraint).

**LLM** (`src-tauri/src/llm/mod.rs`)
- [x] `llm_start_session()` — spawn `llama-server` (`--jinja`, CPU, loopback ephemeral port), poll `/health`, seed system prompt; `role` selects temperature (patient 0.7 / provider 0.3)
- [x] `llm_send_message()` — POST `/v1/chat/completions` with `stream:true`, parse SSE, emit `"llm://token"` per delta, emit `"llm://done"` with the full reply
- [x] `llm_stop_session()` — kill the subprocess (also killed on state drop)
- [x] LLM tool-call dispatch (`src-tauri/src/llm/tools.rs`) — advertise the 5 tools, reassemble streamed tool-call deltas, route to vault (`get_vault_entries`, `get_payloads`, `get_questionnaire`, `get_transcripts`, `save_questionnaire_response`), feed results back, loop (cap 5 rounds). Provider sessions force `exclude_private` and block response writes (data-layer guardrail). Unit-tested.

**STT** (`src-tauri/src/stt/mod.rs`)
- [x] `stt_start_stream()` — spawn `whisper-stream` (SDL2 mic, VAD), clean stdout (ANSI/marker stripping), emit `"stt://partial"` with the running transcript
- [x] `stt_stop_stream()` — kill the subprocess, return the final transcript
- [x] `stt_transcribe_file()` — run `whisper-cli -f <path> -nt -np`, return the full transcript (meeting batch mode)

**TTS** (`src-tauri/src/tts/mod.rs`)
- [x] `tts_synthesize()` — spawn Piper for the `en`/`es`/`zh` voice, write text to stdin, emit the WAV as base64 over `"tts://audio"`
- [x] `tts_stop()` — kill the Piper subprocess (barge-in)

---

## USB App — Frontend

**Session UI shell (built)**
- [x] Home screen (`routes/+page.svelte`) — first-run identity generation, pairing form, paired-provider list, patient/provider entry buttons
- [x] Patient & provider session pages (`routes/patient`, `routes/provider`) — chat view, mic/Speak button, `llm://token` + `stt://partial` event listeners, role-specific system prompts
- [x] Tabbed session panels — `SessionTabBar`, `MessagesPanel`, `QuestionnairesPanel`, `DocumentsPanel` (documents open via `@tauri-apps/plugin-shell`)
- [x] Browser-dev fallback — `isTauriAvailable()` + mock data (`lib/mock/data.ts`) so the UI previews in a plain browser; `BrowserDevBanner` indicates the mode
- [x] Tauri command/event wrappers (`lib/tauri/commands.ts`, `events.ts`, `env.ts`) and payload parsing helpers (`lib/payloads.ts`)

**TTS audio playback**
- [x] Handle `"tts://audio"` Tauri events in patient and provider pages — `lib/audio.ts` decodes/plays the base64 WAV via Web Audio API; `"llm://done"` triggers synthesis (language auto-detected), starting the mic stops playback (barge-in)

**Meeting recording**
- [ ] Record button UI in patient and provider session pages
- [ ] Audio capture (MediaRecorder API or ALSA) + save to temp WAV file
- [ ] Post-recording: call `sttTranscribeFile()`, save transcript to vault
- [ ] Recording consent confirmation step before capture begins

**Patient registration flow (first-run)**
- [x] After keypair generation: show `usb_id`, enter 6-digit pairing code, register via Supabase RPC
- [x] Handle registration success/failure feedback

**Provider login on USB**
- [ ] The "Provider login" button on the home screen goes directly to the provider session — needs actual authentication (PIN, passcode, or provider code entry) before granting access

**Vault management UI**
- [ ] Screen for viewing/adding/editing vault entries (categories, content, `is_private` tagging) — Rust commands (`write_vault_entry`/`read_vault_entries`) exist but no UI calls them yet

**Guardrail configuration UI**
- [ ] Natural-language guardrail text field for the patient to set
- [ ] Per-entry `is_private` toggle in vault entry UI (already exists in the data model)
- [ ] Persistence of guardrail text to vault/config file
- Note: provider session page shows a static "🔒 Guardrails active" badge, but no guardrail data is set, persisted, or enforced yet

**Multi-provider UI**
- [x] Display which provider each payload came from — `providerLabel(payload.provider_id, providerLinks)` is rendered in the Messages/Questionnaires/Documents panels; pages load links via `listProviderLinks()`
- [x] Home screen lists all paired providers and supports pairing with additional providers

---

## Rust/Tauri Binary Dependencies (build-time)
- [x] `usb-app/scripts/fetch-assets.sh` — idempotent fetch/build of all engines + models (pinned URLs). Builds `llama.cpp` and `whisper.cpp` via CMake (whisper with `-DWHISPER_SDL2=ON` for the mic), fetches the archived Piper v1.2.0 CLI tarball, downloads the Qwen2.5 7B Q4 GGUF, Whisper `base`, and the three Piper voices.
- [x] `tauri.conf.json` — bundles `resources/bin/*` into the `.deb` at `/usr/lib/patient-vault/bin/` and declares the `libsdl2-2.0-0` runtime dependency; `assets.rs` resolves these at runtime.
- [ ] Run `fetch-assets.sh` on the build host and commit/stage binaries into `resources/bin/` before packaging (binaries are git-ignored; produced per build host).
- [ ] On Tails: place model files in `~/Persistent/patient-vault/models/` (or set `PATIENT_VAULT_MODEL_DIR`) — see SETUP.md "AI Engines" + "Tails verification checklist".
