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
- [ ] `save_questionnaire_response()` Tauri command — mentioned in PROJECT.md, still not implemented
- [ ] `get_transcripts(date_range?)` Tauri command — vault storage + retrieval for meeting transcripts (not implemented)

> Note: the LLM/STT/TTS commands below exist as stubs (they log via `tracing`
> and return `Ok` without doing work) and are already invoked from the patient
> and provider session pages — so the frontend wiring is in place, but the
> subprocess/inference logic is unimplemented.

**LLM** (`src-tauri/src/llm/mod.rs`)
- [ ] `llm_start_session()` — spawn llama-server/llama-cli subprocess with Qwen2.5 7B Q4 GGUF; stream tokens via `"llm://token"` Tauri events
- [ ] `llm_send_message()` — write user turn to llama.cpp stdin or HTTP endpoint
- [ ] `llm_stop_session()` — kill the subprocess cleanly
- [ ] LLM tool-call dispatch — parse JSON tool-call blocks from llama.cpp output and route to vault commands (`get_vault_entries`, `get_payloads`, `get_questionnaire`, `get_transcripts`, `save_questionnaire_response`), inject tool results back into context

**STT** (`src-tauri/src/stt/mod.rs`)
- [ ] `stt_start_stream()` — spawn `whisper-stream`, read stdout partial transcripts, emit `"stt://partial"` Tauri events
- [ ] `stt_stop_stream()` — send stop signal, collect and return final transcript
- [ ] `stt_transcribe_file()` — spawn `whisper-cli -f <path>`, capture stdout, return full transcript (meeting recording batch mode)

**TTS** (`src-tauri/src/tts/mod.rs`)
- [ ] `tts_synthesize()` — spawn Piper with appropriate voice model (`en`/`es`/`zh`), pipe text to stdin, emit WAV bytes via `"tts://audio"` Tauri events
- [ ] `tts_stop()` — kill the Piper subprocess

---

## USB App — Frontend

**Session UI shell (built)**
- [x] Home screen (`routes/+page.svelte`) — first-run identity generation, pairing form, paired-provider list, patient/provider entry buttons
- [x] Patient & provider session pages (`routes/patient`, `routes/provider`) — chat view, mic/Speak button, `llm://token` + `stt://partial` event listeners, role-specific system prompts
- [x] Tabbed session panels — `SessionTabBar`, `MessagesPanel`, `QuestionnairesPanel`, `DocumentsPanel` (documents open via `@tauri-apps/plugin-shell`)
- [x] Browser-dev fallback — `isTauriAvailable()` + mock data (`lib/mock/data.ts`) so the UI previews in a plain browser; `BrowserDevBanner` indicates the mode
- [x] Tauri command/event wrappers (`lib/tauri/commands.ts`, `events.ts`, `env.ts`) and payload parsing helpers (`lib/payloads.ts`)

**TTS audio playback**
- [ ] Handle `"tts://audio"` Tauri events in patient and provider pages — wire up a Web Audio API player to receive and play WAV chunks as LLM speaks

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
- [ ] llama.cpp binary (`llama-server` or `llama-cli`) — needs to be compiled and placed in resources or referenced from PATH
- [ ] whisper.cpp binaries (`whisper-stream`, `whisper-cli`) — same
- [ ] Piper binary (`piper` from OHF-Voice/piper1-gpl) — same
- [ ] Download scripts or Tauri build hooks to place these in `resources/` at build time
