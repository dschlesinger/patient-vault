// Hybrid X25519 + ML-KEM-768 + AES-256-GCM payload encryption (browser-side).
//
// Encryption flow per payload:
//   1. Ephemeral X25519 keypair + ECDH with recipient → ss_classical
//   2. MlKem768.encap(recipient.mlkem) → { cipherText: ct_pq, sharedSecret: ss_pq }
//   3. ss = HKDF-SHA256(ss_classical || ss_pq)
//   4. AES-256-GCM encrypt plaintext with ss → { ciphertext, iv }
//   5. Stored blob = base64(ct_pq) + "." + base64(ciphertext) + "." + base64(iv) + "." + base64(ephemeralX25519Pub)

import { MlKem768 } from 'mlkem';
import type { HybridPublicKey } from '$lib/types/database';

export interface EncryptedBlob {
  /** ML-KEM-768 ciphertext (encapsulated PQ shared secret) */
  ctPq: Uint8Array;
  /** AES-256-GCM ciphertext */
  ciphertext: Uint8Array;
  /** AES-256-GCM 12-byte nonce */
  iv: Uint8Array;
  /** Ephemeral X25519 public key for ECDH */
  ephemeralX25519Pub: Uint8Array;
}

/** Parse a stored public_key blob (base64) into its X25519 and ML-KEM-768 components. */
export function parsePublicKey(base64: string): HybridPublicKey {
  const raw = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
  // X25519 pubkey = 32 bytes; remainder is ML-KEM-768 pubkey (~1184 bytes)
  return {
    x25519: raw.slice(0, 32),
    mlkem: raw.slice(32)
  };
}

/** Serialize a HybridPublicKey to base64 for storage in patient_provider_links.public_key. */
export function serializePublicKey(key: HybridPublicKey): string {
  const combined = new Uint8Array(key.x25519.length + key.mlkem.length);
  combined.set(key.x25519, 0);
  combined.set(key.mlkem, key.x25519.length);
  return btoa(String.fromCharCode(...combined));
}

/** Encrypt a plaintext payload for a recipient's hybrid public key. */
export async function encryptPayload(
  plaintext: Uint8Array,
  recipientPublicKey: HybridPublicKey
): Promise<EncryptedBlob> {
  // 1. Ephemeral X25519 key exchange.
  // X25519 is exposed in the Web Crypto API under the algorithm name "X25519"
  // (WICG Secure Curves), not via ECDH + namedCurve.
  const ephemeralKeyPair = (await crypto.subtle.generateKey({ name: 'X25519' }, true, [
    'deriveKey',
    'deriveBits'
  ])) as CryptoKeyPair;

  const recipientX25519CryptoKey = await crypto.subtle.importKey(
    'raw',
    recipientPublicKey.x25519.buffer as ArrayBuffer,
    { name: 'X25519' },
    false,
    []
  );

  const ssClassical = await crypto.subtle.deriveBits(
    { name: 'X25519', public: recipientX25519CryptoKey },
    ephemeralKeyPair.privateKey,
    256
  );

  // 2. ML-KEM-768 encapsulation
  const mlkem = new MlKem768();
  const [ctPq, ssPq] = await mlkem.encap(recipientPublicKey.mlkem);

  // 3. Combine shared secrets via HKDF-SHA256
  const combinedSecret = new Uint8Array(32 + ssPq.length);
  combinedSecret.set(new Uint8Array(ssClassical), 0);
  combinedSecret.set(ssPq, 32);

  const hkdfKey = await crypto.subtle.importKey('raw', combinedSecret.buffer as ArrayBuffer, 'HKDF', false, [
    'deriveKey'
  ]);

  const aesKey = await crypto.subtle.deriveKey(
    { name: 'HKDF', hash: 'SHA-256', salt: new Uint8Array(32), info: new TextEncoder().encode('patient-vault-v1') },
    hkdfKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt']
  );

  // 4. AES-256-GCM encryption
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: iv.buffer as ArrayBuffer },
    aesKey,
    plaintext.buffer as ArrayBuffer
  );

  // 5. Export ephemeral X25519 public key
  const ephemeralX25519Pub = new Uint8Array(
    await crypto.subtle.exportKey('raw', ephemeralKeyPair.publicKey)
  );

  return {
    ctPq,
    ciphertext: new Uint8Array(ciphertext),
    iv,
    ephemeralX25519Pub
  };
}

/** Serialize an EncryptedBlob to a dot-separated base64 string for Supabase storage. */
export function serializeBlob(blob: EncryptedBlob): string {
  const encode = (b: Uint8Array) => btoa(String.fromCharCode(...b));
  return [
    encode(blob.ctPq),
    encode(blob.ciphertext),
    encode(blob.iv),
    encode(blob.ephemeralX25519Pub)
  ].join('.');
}
