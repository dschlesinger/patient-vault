// Typed wrappers around Tauri invoke() calls.
// Each function corresponds to a #[tauri::command] in src-tauri/src/.

import { invoke } from '@tauri-apps/api/core';

// ── Vault & identity ──────────────────────────────────────────────────────────

/** Returns true if a keypair and vault already exist in Persistent Storage. */
export async function vaultExists(): Promise<boolean> {
  return invoke<boolean>('vault_exists');
}

/** Generate and persist a new hybrid X25519 + ML-KEM-768 keypair (first-run only). */
export async function generateKeypair(): Promise<void> {
  return invoke('generate_keypair');
}

/** Return the patient's usb_id (SHA-256 of combined public key), base64-encoded. */
export async function getUsbId(): Promise<string> {
  return invoke<string>('get_usb_id');
}

/** Return the patient's hybrid public key as a base64-encoded blob. */
export async function getPublicKey(): Promise<string> {
  return invoke<string>('get_public_key');
}

export interface VaultEntry {
  id: string;
  category: string;
  content: string;
  tags: string[];
  is_private: boolean;
  created_at: string;
}

export interface VaultFilter {
  category?: string;
  exclude_private: boolean;
}

export async function writeVaultEntry(entry: Omit<VaultEntry, 'id' | 'created_at'>): Promise<void> {
  return invoke('write_vault_entry', { entry });
}

export async function readVaultEntries(filter: VaultFilter): Promise<VaultEntry[]> {
  return invoke<VaultEntry[]>('read_vault_entries', { filter });
}

// ── Payload fetching ───────────────────────────────────────────────────────────

export type PayloadType = 'message' | 'questionnaire' | 'document';

export interface DecryptedPayload {
  id: string;
  type: PayloadType;
  content: string;
  provider_id: string;
  received_at: string;
}

/** Fetch and decrypt new payloads from Supabase, persist to local vault. */
export async function syncPayloads(): Promise<number> {
  return invoke<number>('sync_payloads');
}

export interface PayloadFilter {
  type?: PayloadType;
  provider_id?: string;
}

export async function readPayloads(filter: PayloadFilter): Promise<DecryptedPayload[]> {
  return invoke<DecryptedPayload[]>('read_payloads', { filter });
}

// ── LLM ───────────────────────────────────────────────────────────────────────

/** Start a new LLM session with the given system prompt. Tokens stream via Tauri event "llm://token". */
export async function llmStartSession(systemPrompt: string): Promise<void> {
  return invoke('llm_start_session', { systemPrompt });
}

/** Send a user message to the running LLM session. Response streams via "llm://token". */
export async function llmSendMessage(message: string): Promise<void> {
  return invoke('llm_send_message', { message });
}

/** Stop the running LLM session and subprocess. */
export async function llmStopSession(): Promise<void> {
  return invoke('llm_stop_session');
}

// ── STT ───────────────────────────────────────────────────────────────────────

/** Start streaming STT (whisper.cpp). Partial transcripts stream via "stt://partial". */
export async function sttStartStream(): Promise<void> {
  return invoke('stt_start_stream');
}

/** Stop streaming STT and return the final transcript. */
export async function sttStopStream(): Promise<string> {
  return invoke<string>('stt_stop_stream');
}

/** Batch-transcribe a recorded audio file (for meeting recordings). */
export async function sttTranscribeFile(path: string): Promise<string> {
  return invoke<string>('stt_transcribe_file', { path });
}

// ── TTS ───────────────────────────────────────────────────────────────────────

export type TtsLanguage = 'en' | 'es' | 'zh';

/** Synthesize text to speech. Audio streams via "tts://audio" Tauri event. */
export async function ttsSynthesize(text: string, lang: TtsLanguage): Promise<void> {
  return invoke('tts_synthesize', { text, lang });
}

/** Stop current TTS playback. */
export async function ttsStop(): Promise<void> {
  return invoke('tts_stop');
}
