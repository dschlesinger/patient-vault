// Typed wrappers around Tauri invoke() calls.
// Each function corresponds to a #[tauri::command] in src-tauri/src/.

import { invoke } from '@tauri-apps/api/core';
import { isTauriAvailable, requireTauri } from '$lib/tauri/env';

export { isTauriAvailable, TauriUnavailableError } from '$lib/tauri/env';

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  requireTauri();
  return invoke<T>(cmd, args);
}

// ── Vault & identity ──────────────────────────────────────────────────────────

/** Returns true if a keypair and vault already exist in Persistent Storage. */
export async function vaultExists(): Promise<boolean> {
  return tauriInvoke<boolean>('vault_exists');
}

/** Generate and persist a new hybrid X25519 + ML-KEM-768 keypair (first-run only). */
export async function generateKeypair(): Promise<void> {
  return tauriInvoke('generate_keypair');
}

/** Return the patient's usb_id (SHA-256 of combined public key), hex-encoded. */
export async function getUsbId(): Promise<string> {
  return tauriInvoke<string>('get_usb_id');
}

/** Return the patient's hybrid public key as a base64-encoded blob. */
export async function getPublicKey(): Promise<string> {
  return tauriInvoke<string>('get_public_key');
}

export interface RegistrationState {
  patient_name: string;
  registered_at: string;
}

export interface ProviderLink {
  provider_id: string;
  patient_name: string;
  registered_at: string;
}

/** Register with a provider using a 6-digit pairing code shown in the provider portal. */
export async function registerPatient(patientName: string, providerCode: string): Promise<void> {
  return tauriInvoke('register_patient', { patientName, providerCode });
}

/** List all providers this device is paired with. */
export async function listProviderLinks(): Promise<ProviderLink[]> {
  return tauriInvoke<ProviderLink[]>('list_provider_links');
}

/** Return first paired provider (legacy). Prefer listProviderLinks(). */
export async function getRegistrationState(): Promise<RegistrationState | null> {
  return tauriInvoke<RegistrationState | null>('get_registration_state');
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
  return tauriInvoke('write_vault_entry', { entry });
}

export async function readVaultEntries(filter: VaultFilter): Promise<VaultEntry[]> {
  return tauriInvoke<VaultEntry[]>('read_vault_entries', { filter });
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
  return tauriInvoke<number>('sync_payloads');
}

export interface PayloadFilter {
  type?: PayloadType;
  provider_id?: string;
}

export async function readPayloads(filter: PayloadFilter): Promise<DecryptedPayload[]> {
  return tauriInvoke<DecryptedPayload[]>('read_payloads', { filter });
}

// ── LLM ───────────────────────────────────────────────────────────────────────

/** Start a new LLM session with the given system prompt. Tokens stream via Tauri event "llm://token". */
export async function llmStartSession(systemPrompt: string): Promise<void> {
  return tauriInvoke('llm_start_session', { systemPrompt });
}

/** Send a user message to the running LLM session. Response streams via "llm://token". */
export async function llmSendMessage(message: string): Promise<void> {
  return tauriInvoke('llm_send_message', { message });
}

/** Stop the running LLM session and subprocess. */
export async function llmStopSession(): Promise<void> {
  if (!isTauriAvailable()) return;
  return invoke('llm_stop_session');
}

// ── STT ───────────────────────────────────────────────────────────────────────

/** Start streaming STT (whisper.cpp). Partial transcripts stream via "stt://partial". */
export async function sttStartStream(): Promise<void> {
  return tauriInvoke('stt_start_stream');
}

/** Stop streaming STT and return the final transcript. */
export async function sttStopStream(): Promise<string> {
  return tauriInvoke<string>('stt_stop_stream');
}

/** Batch-transcribe a recorded audio file (for meeting recordings). */
export async function sttTranscribeFile(path: string): Promise<string> {
  return tauriInvoke<string>('stt_transcribe_file', { path });
}

// ── TTS ───────────────────────────────────────────────────────────────────────

export type TtsLanguage = 'en' | 'es' | 'zh';

/** Synthesize text to speech. Audio streams via "tts://audio" Tauri event. */
export async function ttsSynthesize(text: string, lang: TtsLanguage): Promise<void> {
  return tauriInvoke('tts_synthesize', { text, lang });
}

/** Stop current TTS playback. */
export async function ttsStop(): Promise<void> {
  if (!isTauriAvailable()) return;
  return invoke('tts_stop');
}
