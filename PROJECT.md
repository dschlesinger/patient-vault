# PatientVault — Project Specification

## Overview

PatientVault is a secure, patient-controlled system for sharing sensitive personal health information with providers. The patient's data lives on a portable USB drive and is only ever decrypted locally on that device — it is never stored in plaintext on any server. Providers can send pre-visit questionnaires and secure messages to patients, and both parties can record provider meetings for the patient's reference.

The system consists of two independently deployed components:

1. **Provider Web Frontend** — a standard web application
2. **Patient/Provider USB App** — a portable Ubuntu desktop application with on-device AI, run from the USB drive

---

## Component 1: Provider Web Frontend

### Stack
- **Framework**: Svelte 5
- **Backend/Database**: Supabase
- **Styling**: Tailwind 4
- **Design system**: Neobrutalism, following design conventions from [olegpolin/neobrutalism-svelte](https://github.com/olegpolin/neobrutalism-svelte)

### Deployment
- Standard web deployment
- Accessed by providers via a regular browser on any device

### Functionality
- Provider authentication/login (Supabase Auth)
- Provider dashboard listing all patient vaults linked to that provider
- Ability to send questionnaires and messages to patients
- Ability to view sent message history per patient (providers see only what they sent — all stored content in Supabase is encrypted and unreadable to Supabase or any third party)

### Patient Registration Flow

Registration ties a patient's USB-generated identity to a specific provider via a one-time 6-digit code, ensuring the pairing happens in person.

1. **Provider generates a one-time code**: The provider clicks "Pair new patient" in the web frontend. The backend generates a random 6-digit code, stores it associated with the provider's ID with a short expiry (e.g., 10 minutes), and displays it to the provider.
2. **Patient keypair**: On first run, the USB app generates a hybrid X25519 + ML-KEM-768 keypair and stores both keys on the USB drive. The patient's `usb_id` is derived as a hash of the combined public key — self-authenticating and not dependent on USB hardware serials (which are spoofable). ML-KEM-768 public keys are ~800 bytes (vs 32 bytes for X25519 alone) — this is fully manageable for storage and transmission.
3. **Patient enters the code**: During the in-person visit, the patient enters the 6-digit code displayed by the provider into the USB app.
4. **Registration request**: The USB app POSTs to `/register` with `{usb_id, public_key, patient_name, provider_code}`. No authentication header is required — the 6-digit code is the trust proof, and the endpoint is open by design (see Security Model).
5. **Backend validation**: The backend checks that `provider_code` exists, has not expired, and has not already been used. If valid, it creates a row in the `patient_provider_links` table associating `usb_id` + `public_key` + `patient_name` with that provider, and marks the code as used.
6. **Multi-provider support**: A patient may register with multiple providers. Each registration creates a separate row in `patient_provider_links`. The UI and schema are designed to accommodate one `usb_id` having many provider associations. Messages in Supabase are tagged with `usb_id` only; provider context is included inside the encrypted blob.

**Note for future iteration**: Rate limiting on `/register` to mitigate brute-forcing of the 6-digit code space (1,000,000 combinations) should be added in a future version.

### Messaging & Document Flow (Provider → Patient)

Messages and questionnaires are encrypted as blobs stored directly in the `payloads` table. Documents are arbitrary files (PDFs, images, Word docs, etc.) — their encrypted bytes are stored in **Supabase Storage** (S3-compatible object store), with only a storage reference in the `payloads` table. All encryption happens client-side before anything reaches Supabase.

**Messages & questionnaires:**
1. Provider composes content in the web frontend
2. Payload (content + provider identity + timestamp + type) is encrypted using the patient's public key
3. Two writes happen atomically:
   - Encrypted blob → `payloads` table (`encrypted_blob` column)
   - Metadata-only log entry → `provider_sent_log` (type, usb_id, timestamp — no content)
4. USB app polls `/payloads/[usb_id]`, decrypts locally, renders by type

**Documents:**
1. Provider selects an arbitrary file (PDF, image, DOCX, etc.) in the web frontend
2. File bytes are encrypted client-side using the patient's public key — the entire ciphertext is an opaque binary
3. Encrypted file is uploaded to **Supabase Storage** under `documents/[payload_id]`
4. A row is written to `payloads` with `storage_path` populated and `encrypted_blob` null
5. A log entry is written to `provider_sent_log` with type `document`
6. USB app polls, finds new document payload, downloads the encrypted file from the storage URL, decrypts locally, saves the plaintext file to the local vault
7. Both patients and providers (when logged into the USB app during a visit) can open decrypted documents from the vault. Files are accessible offline after first download and decryption.

**File size limit**: 50MB per file for v1. Large files may be slow to encrypt in-browser on older hardware.

**Provider activity log**: Providers see a log of what type they sent (message / questionnaire / document), to which patient (`usb_id`), and when. No file content, no filenames, and no patient personal info beyond the pseudonymous `usb_id` is stored server-side.

### Supabase Schema (Key Tables)

**`providers`**
- `id` (uuid, PK)
- `name`, `email`, auth fields (managed by Supabase Auth)

**`pairing_codes`**
- `code` (6-digit string)
- `provider_id` (FK → providers)
- `expires_at` (timestamp)
- `used` (boolean)

**`patient_provider_links`**
- `usb_id` (hash of patient public key)
- `public_key` (patient's asymmetric public key)
- `patient_name` (self-reported at registration, not verified)
- `provider_id` (FK → providers)
- `registered_at` (timestamp)

**`payloads`**
- `id` (uuid, PK)
- `usb_id` (FK → patient_provider_links.usb_id)
- `type` (enum: `message` | `questionnaire` | `document`)
- `encrypted_blob` (encrypted payload for messages and questionnaires — null for documents)
- `storage_path` (Supabase Storage path for encrypted document files — null for messages/questionnaires)
- `created_at` (timestamp — the only metadata Supabase can observe)

**`provider_sent_log`**
- `id` (uuid, PK)
- `provider_id` (FK → providers)
- `usb_id` (pseudonymous patient identifier — hash of public key, not a real name or personal detail)
- `type` (enum: `message` | `questionnaire` | `document`)
- `sent_at` (timestamp)
- `payload_ref_id` (FK → payloads.id)

**Supabase Storage**
- Bucket: `documents`
- Path pattern: `documents/[payload_id]`
- Contents: encrypted binary ciphertext only — filenames, file types, and content are all inside the ciphertext and invisible to Supabase
- Bucket is public (no auth required to download) — encryption is the access control, consistent with the `payloads` table approach

**RLS Policies**

```sql
-- Providers can only read their own sent log entries
create policy "provider_own_sent_log_select"
on provider_sent_log
for select
using (provider_id = auth.uid());

-- Providers can only insert their own sent log entries
create policy "provider_own_sent_log_insert"
on provider_sent_log
for insert
with check (provider_id = auth.uid());

-- payloads table is publicly readable (no auth — encryption is the access control)
create policy "payloads_publicly_readable"
on payloads
for select
using (true);

-- Only authenticated providers can insert payloads
create policy "providers_insert_payloads"
on payloads
for insert
with check (auth.role() = 'authenticated');
```

---

## Component 2: USB App (Ubuntu, portable USB)

### Platform & Portability Model
The USB app targets **Ubuntu** (Linux) and follows a **portable, USB-resident model**: the compiled application and all patient data live together on a USB drive. When the drive is plugged into an Ubuntu machine, the app runs directly from it and reads/writes the patient vault on the same drive — so the patient's data physically travels with them and is never left on the host machine.

Key implications of this model:

- **Self-contained binary** — the app is a single compiled binary plus bundled model assets, runnable from the USB mount point without a system-wide install
- **Data co-located with the app** — the keypair and vault are stored next to the binary on the USB drive (resolved relative to the executable), not in the host's home directory, so unplugging the drive removes all patient data from the host
- **Optional install path** — for users who prefer it, the app can also be installed normally via a `.deb` package, but the default and recommended usage is portable-from-USB
- **No special OS hardening assumed** — unlike an amnesiac OS, Ubuntu is a general-purpose persistent system. Confidentiality therefore rests on end-to-end encryption and physical control of the USB drive rather than on host-OS guarantees (see Security & Access Control Model)

### Tech Stack

- **Application framework**: Tauri (Rust backend + webview-based frontend)
  - Produces a single compiled binary with a small footprint — ideal for running portably from a USB drive
  - Runs directly from the USB mount point on Ubuntu (no system-wide install required)
  - UI written in Svelte for stack consistency with the provider frontend, compiled via Tauri
- **On-device LLM**: llama.cpp (via Rust bindings or subprocess) running **Qwen2.5 7B at Q4 quantization** (~4.5GB RAM) for CPU-based inference
  - Qwen2.5 7B is chosen for its strong tool-calling capability (required for vault fetching), strong multilingual performance across English, Spanish, and Mandarin, and acceptable CPU inference speed (~3-6 tokens/second on mid-range hardware)
  - The LLM is a general conversational agent available to both patients and providers, with tool access scoped by role and guardrails (see Access Control Model)
- **Speech-to-Text**: whisper.cpp with the **`base` multilingual model** (~140MB) — supports English, Spanish, and Mandarin Chinese
  - **Live/streaming mode** for conversational LLM interactions: audio is processed in sliding ~1-3 second chunks, partial transcripts stream to screen in realtime as the user speaks
  - **Batch mode** for meeting recording transcription: full audio processed post-recording
- **Text-to-Speech**: Piper — CPU-realtime, ~20-60MB per voice model. The following models are bundled:
  - English: `en_US-lessac-medium`
  - Spanish: `es_MX-ald-medium` (Mexican Spanish — variant TBD)
  - Mandarin Chinese: `zh_CN-huayan-medium`
  - Language is selectable per interaction (not globally fixed)
- **Encryption**: Hybrid X25519 + ML-KEM-768 (see Cryptography section); patient keypair generated on first run and stored on the USB drive alongside the app; private key never transmitted
  - Rust side: `ml-kem` crate (RustCrypto, pure Rust, FIPS 203) for ML-KEM-768, `x25519-dalek` for X25519, `aes-gcm` for symmetric encryption
  - Browser side: `ml-kem` npm package for ML-KEM-768, WebCrypto API for X25519 and AES-256-GCM

### App Structure: Two Login Paths

The LLM agent is the primary interface for both patients and providers. It is conversational, turn-based, and tool-enabled — it can fetch and reason over local vault data, decrypted provider payloads, questionnaires, and meeting transcripts. It does not fetch from external sources; all data access is local only.

**Conversational turn structure:**
1. LLM generates a response or question → TTS speaks it aloud + text shown on screen → LLM ends its turn
2. Mic button becomes active — user clicks to begin their turn
3. User speaks → live transcript streams on screen in realtime (whisper.cpp streaming mode)
4. User clicks mic again to end their turn → transcript is committed → sent to LLM as next message
5. Repeat

The LLM may ask one question per turn or make statements/summaries — it is not restricted to question-only turns. If the user asks "the doctor sent me questions, let's go over them," the LLM fetches the relevant questionnaire via tool call and walks through questions one at a time, one per turn.

**LLM Tools (local only, no external network access):**
- `get_vault_entries(filter?)` — read patient vault entries, filtered by guardrail exclusions for provider sessions
- `get_payloads(type?, provider_id?)` — fetch decrypted messages, questionnaires, or documents from the local vault
- `get_questionnaire(payload_id)` — retrieve a specific questionnaire and its questions
- `get_transcripts(date_range?)` — fetch meeting transcripts from the vault
- `save_questionnaire_response(payload_id, question_id, response)` — save a patient's voice response to a questionnaire question

#### Patient Login

**Conversational LLM agent (patient-scoped)**
- Full conversational access to the LLM agent
- LLM can fetch and read provider messages, questionnaires, and documents from the local vault
- LLM can walk through provider questionnaires conversationally — asking questions one turn at a time, capturing voice responses via STT
- LLM can read and summarize vault entries
- Guardrails apply in the reverse direction here — the patient's guardrail text and data-layer exclusions are enforced only during *provider* sessions; in patient sessions the LLM has full vault access

**Vault management**
- Local encrypted storage of patient-provided personal and health information
- Patient configures guardrails: a natural-language text field where the patient instructs the LLM what not to disclose to a provider (e.g., "Do not mention my mental health history"), plus explicit data-layer exclusion tagging of specific vault entries
- Guardrail instructions apply only when a provider is logged in (see Access Control Model)

**Provider messages & documents**
- Automatic check for new payloads on app load
- "Check for new messages" button for manual polling
- Payloads are fetched, decrypted locally, and stored in the vault grouped by provider
- Accessible directly in the UI and via the LLM agent

**Meeting recording**
- Patient can initiate a meeting recording
- Audio is transcribed locally after the recording ends (batch, via whisper.cpp)
- Transcript is saved to the local vault

#### Provider Login (via patient's USB)

**Conversational LLM agent (provider-scoped)**
- Full conversational access to the LLM agent, with all vault access filtered through the patient's guardrails (data-layer exclusion + system prompt instruction — see Access Control Model)
- Provider can ask the LLM for a debrief on questionnaire answers, vault summaries, documents, and meeting transcripts — all within permitted scope
- Provider cannot instruct the LLM to bypass guardrails; excluded vault entries are never in the LLM's context window regardless of how questions are phrased

**Meeting recording**
- Provider can initiate or participate in meeting recording; transcript is saved to the vault

---

## Security & Access Control Model

### Endpoint Security

`/register` and `/messages/[usb_id]` are open endpoints — no Supabase auth header is required from the USB app. This is an intentional design decision:

- **`/register`**: Trust is established by the one-time 6-digit code, not by HTTP authentication. A valid, unexpired, unused code proves in-person presence.
- **`/messages/[usb_id]`**: Content security relies entirely on asymmetric encryption. Even if a third party polls this endpoint and retrieves encrypted blobs, they cannot decrypt them without the patient's private key. The `usb_id` is a hash and not easily guessable.
- **Metadata exposure**: Supabase can observe `usb_id` values and row creation timestamps (`created_at`). All other metadata — provider identity, message timing, content — is inside the encrypted blob and invisible to Supabase. This is considered acceptable for v1.

### Guardrail Architecture

Access control for what the LLM may disclose to a provider uses a two-layer approach:

1. **Data-layer exclusion (hard guarantee)**: Vault entries the patient marks as private are excluded from the context window passed to the LLM during a provider debrief session. The model never sees this data, regardless of how the provider phrases their questions.
2. **System prompt instruction (soft layer)**: The patient's natural-language guardrail text (e.g., "Do not discuss my anxiety medication") is included in the LLM system prompt. This handles nuanced topic guidance that isn't easily captured by entry-level tagging alone.

Both layers are active simultaneously. The data-layer exclusion provides the hard guarantee; the system prompt instruction handles nuance. Patients should be informed that the system prompt layer is best-effort (a 7B model is more reliable than a 2B model at following instructions, but can still be circumvented by a determined adversary) while the data-layer exclusion is deterministic. Guardrails apply only during provider sessions — in patient sessions the LLM has full vault access.

### Identity & Pairing

- Patient identity is a cryptographic keypair stored on the USB drive; `usb_id` = hash(public key)
- The USB is the physical security boundary — possession of the USB is possession of the identity
- Multi-provider: one `usb_id` may be linked to many providers via separate rows in `patient_provider_links`
- Re-registration (e.g., after USB loss or keypair regeneration) requires a new in-person pairing with each provider; old registrations are not automatically revoked (future work)

### Data Flow Summary
- **Patient data**: stored locally only, never transmitted
- **Provider → Patient messages**: encrypted client-side (provider's browser) using hybrid X25519 + ML-KEM-768 with AES-256-GCM symmetric encryption before storage in Supabase; decrypted only on the patient's USB using the private key
- **Supabase visibility**: `usb_id` (a hash), opaque encrypted blobs, and `created_at` timestamps only — provider identity, message content, and precise send time are all inside the encrypted blob

---

## Cryptography

### Scheme: Hybrid X25519 + ML-KEM-768 with AES-256-GCM

PatientVault uses a hybrid post-quantum encryption scheme combining classical and post-quantum algorithms. This protects against both classical attacks today and quantum attacks in the future. If either algorithm is later found to have a flaw, the other still holds.

**Why hybrid?**
- Pure classical (X25519/ECDH) is broken by Shor's algorithm on a sufficiently powerful quantum computer
- Pure post-quantum (ML-KEM alone) is newer and less battle-tested
- Combining both is the current best practice, used by Signal, Apple iMessage, and Google — the combined shared secret is only breakable if *both* algorithms are broken simultaneously

**ML-KEM-768** is NIST FIPS 203 (finalized 2024), formerly known as Kyber-768. It is the primary NIST-standardized key encapsulation mechanism for post-quantum key exchange.

### Encryption Flow (per payload)

1. Provider's browser holds the patient's hybrid public key (X25519 component + ML-KEM-768 component)
2. Browser performs X25519 key exchange → produces shared secret `ss_classical`
3. Browser performs ML-KEM-768 encapsulation → produces shared secret `ss_pq` + encapsulated key ciphertext `ct_pq`
4. Combined shared secret: `ss = HKDF(ss_classical || ss_pq)` (hash-combine via HKDF-SHA256)
5. Payload (content + provider identity + timestamp + type) is encrypted with AES-256-GCM using `ss` as the key
6. Blob stored in Supabase = `{ ct_pq | aes_ciphertext | aes_nonce }`

**Decryption (USB app):**
1. Patient's private key (X25519 + ML-KEM-768) decapsulates `ct_pq` → recovers `ss_pq`
2. X25519 shared secret `ss_classical` is re-derived from the ephemeral public key in the blob
3. `ss = HKDF(ss_classical || ss_pq)` → AES-256-GCM decrypts the payload

### Key Storage
- Patient private key stored on the USB drive; for v1 the on-disk files are plaintext JSON, so at-rest protection relies on physical control of the drive (full-disk encryption such as LUKS on the USB is recommended)
- Private key never leaves the USB under any circumstances
- `usb_id` = SHA-256(X25519_pubkey || ML-KEM-768_pubkey)

### Libraries
| Component | Library |
|---|---|
| Rust (USB app) | `ml-kem` (RustCrypto, FIPS 203, pure Rust — no C deps) for ML-KEM-768, `x25519-dalek` for X25519, `aes-gcm` for AES-256-GCM, `hkdf` for key derivation |
| Browser (provider frontend) | `ml-kem` npm package for ML-KEM-768, WebCrypto API for X25519 + AES-256-GCM + HKDF |

### Future Consideration
ML-DSA (FIPS 204, formerly Dilithium) digital signatures could be added in a future version to allow patients to verify that payloads were genuinely sent by their registered provider, rather than a third party who obtained the patient's public key.

---

1. **Rate limiting on `/register`**: 6-digit codes (1M combinations) are not rate-limited in v1. Should be added before public deployment.
2. **USB loss/recovery**: No second-factor or keypair recovery mechanism exists. Losing the USB means losing the patient identity and requiring fresh re-registration with all providers.
3. **STT streaming quality**: Whisper.cpp streaming mode processes audio in sliding windows and may produce shifting/correcting partial transcripts. Quality is acceptable for conversational use but not perfect. Meeting recording transcription uses batch mode and is higher accuracy.
4. **TTS language variant**: Spanish variant (Spain vs. Mexico) not yet finalized.
5. **Re-registration/revocation**: No mechanism to revoke an old `usb_id` registration when a patient gets a new USB or regenerates their keypair.
6. **Meeting recording consent UI**: A consent confirmation step before recording begins should be included — legal side deferred, but the UI affordance is good practice.

---

## V1 Build Scope

- Provider web frontend: full scope (auth, dashboard, patient management, questionnaire/message sending, encrypted blob storage)
- USB app: Tauri + Svelte, running portably from the USB drive on Ubuntu
- LLM: Qwen2.5 7B Q4 via llama.cpp — general conversational agent with tool calling for local vault access, scoped by role and guardrails
- STT: whisper.cpp `base` multilingual — streaming mode for conversational interactions, batch mode for meeting recording
- TTS: Piper with English, Spanish (MX), and Mandarin voice models; language selectable per interaction
- Encryption: hybrid X25519 + ML-KEM-768 with AES-256-GCM; full payload encryption (content + provider + timestamp inside blob); keypair-based identity
- Multi-provider support from day one
- Meeting recording: record → batch transcribe → save transcript

# PatientVault — Tech Stack Brief

---

## Provider Web Frontend

### Svelte 5
**Role in project**: UI framework for the provider web frontend. Handles the provider dashboard, patient list, questionnaire composer, message sending, and pairing code generation.

**Why it fits**: Svelte 5's runes-based reactivity model is well-suited to a real-time dashboard that needs to respond to polling state, form inputs, and multi-patient management without heavyweight framework overhead. Compiles to vanilla JS — no virtual DOM penalty. UI is written in the same framework as the Tauri USB app frontend, keeping the stack consistent across both components.

**MCP**: ✅ Official MCP available — `@sveltejs/mcp`
Provides live Svelte 5 docs, autofixing of anti-patterns, and Svelte-aware code analysis. Prevents the common failure mode of AI assistants defaulting to React/Vue patterns when writing Svelte code.
```json
"svelte": {
  "command": "npx",
  "args": ["-y", "@sveltejs/mcp"]
}
```
**Docs**: https://svelte.dev/docs

---

### Supabase
**Role in project**: Backend-as-a-service providing the PostgreSQL database, Row Level Security, Supabase Auth (provider authentication), and Supabase Storage (encrypted document blobs). Acts as the secure relay between provider frontend and patient USB app — stores only encrypted payloads, pseudonymous `usb_id` hashes, and activity log metadata. No plaintext patient data ever reaches Supabase.

**Why it fits**: Provides auth, database, storage, and RLS in a single managed service with a generous free tier. RLS policies enforce provider-scoped data access at the database level without custom API middleware. Supabase Storage handles arbitrary encrypted binary files (documents) without size constraints in the DB schema. The open-source nature means self-hosting is possible in future if needed.

**MCP**: ✅ Official remote MCP available at `https://mcp.supabase.com/mcp`
Connects AI tools directly to your Supabase project for schema management, query execution, RLS policy writing, and configuration. Use project-scoped mode to restrict access to the PatientVault project only. **Development use only — never connect to production.**
```json
"supabase": {
  "type": "http",
  "url": "https://mcp.supabase.com/mcp?project_ref=<your-project-ref>"
}
```
**Docs**: https://supabase.com/docs

---

### Tailwind CSS 4
**Role in project**: Utility-first CSS framework for styling the provider web frontend. Applied within the neobrutalism design system.

**Why it fits**: Tailwind 4's new CSS-first configuration (no `tailwind.config.js` required) pairs well with Svelte's scoped styles. Utility classes keep component styles co-located and eliminate stylesheet bloat.

**MCP**: ❌ No official MCP. Use docs directly.
**Docs**: https://tailwindcss.com/docs

---

### Neobrutalism Design System
**Role in project**: Visual design language for the provider frontend. Bold borders, high-contrast colors, flat shadows, and strong typographic hierarchy. Referenced implementation: `olegpolin/neobrutalism-svelte`.

**Why it fits**: Distinctive and intentional aesthetic that differentiates PatientVault from generic clinical software. Neobrutalism's high-contrast, legibility-first approach is also practically well-suited to a healthcare context where readability matters.

**MCP**: ❌ No official MCP. Reference the GitHub repo directly.
**Reference**: https://github.com/olegpolin/neobrutalism-svelte

---

## USB App (Ubuntu, portable USB)

### Tauri
**Role in project**: Application framework for the patient/provider USB app, targeting Ubuntu and run portably from the USB drive. Produces a single compiled binary (Rust backend + webview frontend) that runs directly from the drive without a system-wide install (a `.deb` install is also supported).

**Why it fits**: Tauri's output is a standalone native binary, not a bundled Chromium instance — a small footprint that is well suited to running portably from a USB drive. The Rust backend handles all security-sensitive operations (cryptography, local vault access, llama.cpp/whisper.cpp subprocess management) while the Svelte frontend (compiled via Tauri) provides the UI — consistent with the provider frontend's stack. Significantly smaller binary footprint than Electron.

**MCP**: ⚠️ No official MCP. A community MCP exists (`dirvine/tauri-mcp` on crates.io) for testing and debugging Tauri apps during development — useful but not officially maintained.
**Docs**: https://tauri.app/start/

---

### llama.cpp
**Role in project**: C/C++ inference engine that runs the Qwen2.5 7B Q4 GGUF model on-device. Called from the Tauri Rust backend as a subprocess or via Rust bindings. Powers the conversational LLM agent available to both patients and providers, with tool-calling capability for local vault fetching.

**Why it fits**: The de facto standard for local LLM inference — used internally by Ollama, LM Studio, and most local inference tools. Supports GGUF quantization (Q4 reduces Qwen2.5 7B to ~4.5GB RAM), CPU-optimized inference paths, and tool/function calling. No GPU required.

**MCP**: ❌ No official MCP. Community bridges exist but are not production-ready for this use case — the LLM is called programmatically from Rust, not via MCP.
**Docs**: https://github.com/ggml-org/llama.cpp

---

### Qwen2.5 7B (Q4 GGUF)
**Role in project**: The on-device language model. Handles all conversational interaction for both patient and provider sessions — answering questions, walking through questionnaires turn-by-turn, fetching and summarizing vault data via tool calls, and debriefing providers on patient answers (filtered by guardrails).

**Why it fits**: Qwen2.5 7B is selected over alternatives (Gemma2-2B, Gemma3-4B, Phi-4) for three specific reasons relevant to this project: strong tool/function calling capability (required for vault data fetching), strong multilingual performance across English, Spanish, and Mandarin (matching the TTS/STT language targets), and manageable RAM footprint at Q4 (~4.5GB, leaving headroom on 8GB machines). CPU inference speed is approximately 3-6 tokens/second on mid-range hardware — acceptable for conversational turn-taking.

**MCP**: ❌ N/A — model file, not a service.
**Model**: https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF

---

### whisper.cpp
**Role in project**: C/C++ speech-to-text engine. Runs the `base` multilingual Whisper model (~140MB) on-device. Used in two modes: streaming (sliding ~1-3 second audio windows, live transcript to screen during conversational LLM turns) and batch (full post-recording transcription for meeting recordings).

**Why it fits**: Port of OpenAI's Whisper model optimized for CPU and edge inference — no GPU required. The `base` multilingual model supports English, Spanish, and Mandarin, matching the three supported languages. Streaming mode provides the live transcript UX required for the conversational turn interface. Batch mode provides higher-accuracy transcription for meeting recordings.

**MCP**: ❌ No official MCP.
**Docs**: https://github.com/ggml-org/whisper.cpp

---

### Piper TTS
**Role in project**: Neural text-to-speech engine. Reads LLM output and questionnaire questions aloud during conversational turns. CPU-realtime. Three language voice models bundled on the USB: English (`en_US-lessac-medium`), Spanish (`es_MX-ald-medium`), and Mandarin Chinese (`zh_CN-huayan-medium`). Language selectable per interaction.

**Why it fits**: Designed specifically for CPU-realtime inference on low-power hardware (originally targeting Raspberry Pi 4) — well within the performance envelope of a typical Ubuntu laptop. VITS-based ONNX models, ~20-60MB per voice, no cloud dependency, offline-first.

**Note**: The original `rhasspy/piper` repository was archived in October 2025; active development moved to `OHF-Voice/piper1-gpl` (a Python package). PatientVault deliberately uses the **archived `rhasspy/piper` v1.2.0 standalone C++ CLI binary** (release tag `2023.11.14-2`) — it is a single dependency-free executable that reads text on stdin and writes a WAV, which is the simplest, most portable integration for a spawned subprocess. The USB app reads the resulting WAV and plays it via the Web Audio API (one `tts://audio` event per clip). See `usb-app/scripts/fetch-assets.sh` for the pinned download.

**Implementation**: `usb-app/src-tauri/src/tts/mod.rs` spawns `piper --model <voice> --output_file <tmp.wav>`, then emits the WAV as base64. Barge-in is supported: starting the mic kills any in-flight Piper process and stops playback.

**MCP**: ❌ No official MCP.
**Archived CLI (in use)**: https://github.com/rhasspy/piper (release `2023.11.14-2`)
**Maintained successor**: https://github.com/OHF-Voice/piper1-gpl
**Voice models**: https://huggingface.co/rhasspy/piper-voices

---

## Cryptography

### Hybrid X25519 + ML-KEM-768 with AES-256-GCM
**Role in project**: End-to-end encryption of all payloads (messages, questionnaires, documents) sent from provider to patient. Patient keypair is generated on first USB run and stored on the USB drive. Private key never leaves the USB.

**Why it fits**: Hybrid classical + post-quantum scheme combining X25519 (battle-tested, classical) and ML-KEM-768 (NIST FIPS 203, finalized 2024, post-quantum). Combined shared secret via HKDF-SHA256. Content encrypted with AES-256-GCM. Protects against both classical and quantum adversaries — if either algorithm is broken, the other still holds. Current best practice used by Signal, Apple, and Google.

**MCP**: ❌ N/A — cryptographic libraries, not services.
**Rust libraries**: `ml-kem` (RustCrypto, pure Rust, FIPS 203 — chosen over `oqs`, which requires the `liboqs` C dependency, to keep the build pure-Rust and portable), `x25519-dalek`, `aes-gcm`, `hkdf`
**Browser libraries**: `ml-kem` npm package, WebCrypto API (X25519, AES-256-GCM, HKDF)
**ML-KEM-768 spec**: https://csrc.nist.gov/pubs/fips/203/final