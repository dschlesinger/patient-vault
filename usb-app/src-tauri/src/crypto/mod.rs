// Hybrid X25519 + ML-KEM-768 + AES-256-GCM cryptography for PatientVault.
//
// Key generation: X25519 keypair + ML-KEM-768 keypair generated together.
// usb_id = SHA-256(x25519_pubkey || mlkem_pubkey) — stable pseudonymous identity.
//
// Encryption flow (per payload, encrypted by provider browser, decrypted here):
//   1. Ephemeral X25519 ECDH → ss_classical (32 bytes)
//   2. ML-KEM-768 decapsulate(ct_pq, mlkem_decap_key) → ss_pq (32 bytes)
//   3. ss = HKDF-SHA256(ss_classical || ss_pq, info="patient-vault-v1")
//   4. AES-256-GCM decrypt(ss, ciphertext, nonce) → plaintext
//
// Libraries: ml-kem (RustCrypto, FIPS 203), x25519-dalek, aes-gcm, hkdf, sha2

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct HybridKeypair {
    /// X25519 private key bytes (32 bytes)
    pub x25519_private: [u8; 32],
    /// X25519 public key bytes (32 bytes)
    pub x25519_public: [u8; 32],
    /// ML-KEM-768 decapsulation key bytes (serialized, ~2400 bytes)
    pub mlkem_decap: Vec<u8>,
    /// ML-KEM-768 encapsulation (public) key bytes (serialized, ~1184 bytes)
    pub mlkem_encap: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HybridPublicKey {
    pub x25519: [u8; 32],
    pub mlkem: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct EncryptedBlob {
    /// ML-KEM-768 ciphertext (encapsulated PQ shared secret, ~1088 bytes)
    pub ct_pq: Vec<u8>,
    /// AES-256-GCM ciphertext
    pub ciphertext: Vec<u8>,
    /// AES-256-GCM 12-byte nonce
    pub nonce: [u8; 12],
    /// Ephemeral X25519 public key used for ECDH (32 bytes)
    pub ephemeral_x25519_pub: [u8; 32],
}

/// Generate a new hybrid keypair. Called once on first run.
pub fn generate_keypair() -> HybridKeypair {
    todo!("implement: x25519_dalek::StaticSecret::random() + ml_kem::MlKem768::generate()")
}

/// Derive the usb_id from a hybrid public key: SHA-256(x25519_pub || mlkem_encap).
pub fn derive_usb_id(public_key: &HybridPublicKey) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&public_key.x25519);
    hasher.update(&public_key.mlkem);
    hasher.finalize().into()
}

/// Decrypt a blob produced by the provider's browser-side hybrid encryption.
pub fn decrypt(_blob: &EncryptedBlob, _keypair: &HybridKeypair) -> anyhow::Result<Vec<u8>> {
    todo!("implement: x25519 ECDH + ml-kem decapsulate + HKDF-SHA256 + AES-256-GCM decrypt")
}
