# PatientVault — Implementation Checklist

## Provider Web Frontend

**Encryption wiring (missing client-side crypto calls)**
- [x] Wire `hybrid.ts` `encryptPayload()` into the Send Message form — `use:enhance` encrypts to `encrypted_blob` before POST
- [x] Wire `hybrid.ts` `encryptPayload()` into the Send Questionnaire form — questions JSON-encrypted into one blob
- [x] Wire `hybrid.ts` `encryptPayload()` into the Send Document form — file bytes (with filename+mime packed in) encrypted before upload as `encrypted_file`
  - Also: fixed `hybrid.ts` to use the Web Crypto `X25519` algorithm name (not `ECDH`+`namedCurve`, which Chrome rejects)
  - Verified end-to-end via Chrome DevTools against live Supabase (4-segment ciphertext blobs; document metadata stays encrypted)

**Missing routes**
- [ ] `/register` endpoint — the open POST route the USB app calls to pair a patient (`{usb_id, public_key, patient_name, provider_code}`); validates pairing code, inserts into `patient_provider_links`

**Auth**
- [ ] Provider sign-up/registration flow (login page exists; no way to create a provider account unless manually seeded)

**Infrastructure**
- [x] SQL migration file(s) for all tables (`providers`, `pairing_codes`, `patient_provider_links`, `payloads`, `provider_sent_log`) + RLS policies — applied to live Supabase (`initial_schema`)
  - [x] Follow-up migration `payloads_public_read_policy`: added the missing `payloads` SELECT policy (`using (true)`) — without it `INSERT ... RETURNING` was blocked by RLS
- [x] Supabase Storage bucket creation (`documents`) — created in `initial_schema`
  - [ ] **Storage read policy mismatch**: `documents` SELECT policy currently requires `auth.uid() IS NOT NULL`, but PROJECT.md specifies the bucket is publicly readable (USB app downloads with no auth; encryption is the access control). Needs an anon-readable policy before USB document download works.

---

## USB App — Rust Backend

**Cryptography** (`src-tauri/src/crypto/mod.rs`)
- [ ] `generate_keypair()` — currently `todo!()`, needs `x25519_dalek::StaticSecret::random()` + `ml_kem::MlKem768::generate()`
- [ ] `decrypt()` — currently `todo!()`, needs full X25519 ECDH + ML-KEM-768 decapsulate + HKDF-SHA256 + AES-256-GCM decrypt

**Vault** (`src-tauri/src/vault/mod.rs`)
- [ ] `sync_payloads()` — stub returning `Ok(0)`, needs to: fetch from Supabase `/payloads/[usb_id]`, call `decrypt()` on each blob, persist to `payloads.json`
- [ ] Document download in `sync_payloads()` — when `payload.type = 'document'`, download encrypted file from Supabase Storage, decrypt, save to local vault
- [ ] `save_questionnaire_response()` Tauri command — mentioned in PROJECT.md but not yet implemented in `vault/mod.rs`
- [ ] `get_transcripts(date_range?)` Tauri command — vault storage + retrieval for meeting transcripts

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

**TTS audio playback**
- [ ] Handle `"tts://audio"` Tauri events in patient and provider pages — wire up a Web Audio API player to receive and play WAV chunks as LLM speaks

**Meeting recording**
- [ ] Record button UI in patient and provider session pages
- [ ] Audio capture (MediaRecorder API or ALSA) + save to temp WAV file
- [ ] Post-recording: call `sttTranscribeFile()`, save transcript to vault
- [ ] Recording consent confirmation step before capture begins

**Patient registration flow (first-run)**
- [ ] After keypair generation: show `usb_id`, let patient enter a 6-digit pairing code, POST to `/register` on the provider server
- [ ] Handle registration success/failure feedback

**Provider login on USB**
- [ ] The "Provider login" button on the home screen goes directly to the provider session — needs actual authentication (PIN, passcode, or provider code entry) before granting access

**Vault management UI**
- [ ] Screen for viewing/adding/editing vault entries (categories, content, `is_private` tagging)

**Guardrail configuration UI**
- [ ] Natural-language guardrail text field for the patient to set
- [ ] Per-entry `is_private` toggle in vault entry UI (already exists in the data model)
- [ ] Persistence of guardrail text to vault/config file

**Multi-provider UI**
- [ ] Display which provider each payload came from (payload filter by `provider_id` exists in Rust but no UI surfaces it)

---

## Rust/Tauri Binary Dependencies (build-time)
- [ ] llama.cpp binary (`llama-server` or `llama-cli`) — needs to be compiled and placed in resources or referenced from PATH
- [ ] whisper.cpp binaries (`whisper-stream`, `whisper-cli`) — same
- [ ] Piper binary (`piper` from OHF-Voice/piper1-gpl) — same
- [ ] Download scripts or Tauri build hooks to place these in `resources/` at build time
