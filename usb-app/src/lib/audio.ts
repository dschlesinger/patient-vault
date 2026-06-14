// Web Audio playback for Piper TTS clips delivered as base64-encoded WAV via the
// "tts://audio" Tauri event. A single shared AudioContext is reused; only one
// clip plays at a time so a new reply (or a barge-in) cleanly supersedes the old.

import type { TtsLanguage } from '$lib/tauri/commands';

let audioContext: AudioContext | null = null;
let currentSource: AudioBufferSourceNode | null = null;

function getContext(): AudioContext {
  if (!audioContext) {
    audioContext = new AudioContext();
  }
  return audioContext;
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

/** Stop any audio currently playing. Safe to call when nothing is playing. */
export function stopAudio(): void {
  if (currentSource) {
    try {
      currentSource.onended = null;
      currentSource.stop();
    } catch {
      // Already stopped — ignore.
    }
    currentSource = null;
  }
}

/**
 * Decode and play a base64-encoded WAV clip, replacing any current playback.
 * Resolves when playback finishes (or is superseded/stopped).
 */
export async function playWavBase64(base64: string): Promise<void> {
  const ctx = getContext();
  if (ctx.state === 'suspended') {
    await ctx.resume();
  }

  const buffer = await ctx.decodeAudioData(base64ToArrayBuffer(base64));

  // Supersede any in-flight clip.
  stopAudio();

  const source = ctx.createBufferSource();
  source.buffer = buffer;
  source.connect(ctx.destination);
  currentSource = source;

  await new Promise<void>((resolve) => {
    source.onended = () => {
      if (currentSource === source) currentSource = null;
      resolve();
    };
    source.start();
  });
}

/**
 * Best-effort language guess for voice selection. Whisper transcripts and LLM
 * replies have no language tag, so this uses a coarse script/keyword heuristic:
 * CJK characters → Mandarin, common Spanish markers → Spanish, else English.
 */
export function detectTtsLanguage(text: string): TtsLanguage {
  if (/[\u4e00-\u9fff]/.test(text)) return 'zh';
  if (/[¿¡]|\b(el|la|los|las|usted|gracias|salud|medicamento|dolor)\b/i.test(text)) {
    return 'es';
  }
  return 'en';
}
