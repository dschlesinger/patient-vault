// Hybrid X25519 + ML-KEM-768 + AES-256-GCM cryptography for PatientVault.
//
// Key generation: X25519 keypair + ML-KEM-768 keypair generated together.
// usb_id = SHA-256(x25519_pubkey || mlkem_pubkey) — stable pseudonymous identity.
//
// Encryption flow (per payload, encrypted by provider browser, decrypted here):
//   1. Ephemeral X25519 ECDH → ss_classical (32 bytes)
//   2. ML-KEM-768 decapsulate(ct_pq, mlkem_decap_key) → ss_pq (32 bytes)
//   3. ss = HKDF-SHA256(ss_classical || ss_pq, salt=zeros, info="patient-vault-v1")
//   4. AES-256-GCM decrypt(ss, ciphertext, nonce) → plaintext
//
// Wire format: base64(ct_pq).base64(ciphertext).base64(iv).base64(ephemeral_x25519_pub)

use aes_gcm::{
    aead::{Aead, KeyInit as AesKeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hkdf::Hkdf;
use ml_kem::{
    kem::{Decapsulate, Encapsulate, FromSeed, Kem, KeyExport},
    Ciphertext, MlKem768,
};
use ml_kem::ml_kem_768::{DecapsulationKey as DecapsulationKey768, EncapsulationKey as EncapsulationKey768};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct HybridKeypair {
    /// X25519 private key bytes (32 bytes)
    pub x25519_private: [u8; 32],
    /// X25519 public key bytes (32 bytes)
    pub x25519_public: [u8; 32],
    /// ML-KEM-768 decapsulation seed (64 bytes)
    pub mlkem_decap: Vec<u8>,
    /// ML-KEM-768 encapsulation (public) key bytes (~1184 bytes)
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
    /// AES-256-GCM ciphertext (includes 16-byte auth tag)
    pub ciphertext: Vec<u8>,
    /// AES-256-GCM 12-byte nonce
    pub nonce: [u8; 12],
    /// Ephemeral X25519 public key used for ECDH (32 bytes)
    pub ephemeral_x25519_pub: [u8; 32],
}

/// Generate a new hybrid keypair. Called once on first run.
pub fn generate_keypair() -> HybridKeypair {
    let x25519_secret = StaticSecret::random_from_rng(&mut OsRng);
    let x25519_public = PublicKey::from(&x25519_secret);

    let (dk, ek) = MlKem768::generate_keypair();
    let mlkem_decap = dk.to_bytes().as_slice().to_vec();
    let mlkem_encap = ek.to_bytes().as_slice().to_vec();

    HybridKeypair {
        x25519_private: x25519_secret.to_bytes(),
        x25519_public: x25519_public.to_bytes(),
        mlkem_decap,
        mlkem_encap,
    }
}

/// Derive the usb_id from a hybrid public key: SHA-256(x25519_pub || mlkem_encap).
pub fn derive_usb_id(public_key: &HybridPublicKey) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(public_key.x25519);
    hasher.update(&public_key.mlkem);
    hasher.finalize().into()
}

/// Parse a dot-separated base64 blob from Supabase / Storage.
pub fn parse_blob(raw: &str) -> Result<EncryptedBlob> {
    let parts: Vec<&str> = raw.split('.').collect();
    if parts.len() != 4 {
        return Err(anyhow!("expected 4 dot-separated blob segments, got {}", parts.len()));
    }

    let ct_pq = BASE64
        .decode(parts[0])
        .context("invalid ct_pq base64")?;
    let ciphertext = BASE64
        .decode(parts[1])
        .context("invalid ciphertext base64")?;
    let iv_bytes = BASE64.decode(parts[2]).context("invalid iv base64")?;
    let ephemeral_bytes = BASE64
        .decode(parts[3])
        .context("invalid ephemeral X25519 pubkey base64")?;

    if iv_bytes.len() != 12 {
        return Err(anyhow!("invalid IV length: {}", iv_bytes.len()));
    }
    if ephemeral_bytes.len() != 32 {
        return Err(anyhow!(
            "invalid ephemeral X25519 pubkey length: {}",
            ephemeral_bytes.len()
        ));
    }

    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&iv_bytes);
    let mut ephemeral_x25519_pub = [0u8; 32];
    ephemeral_x25519_pub.copy_from_slice(&ephemeral_bytes);

    Ok(EncryptedBlob {
        ct_pq,
        ciphertext,
        nonce,
        ephemeral_x25519_pub,
    })
}

/// Decrypt a blob produced by the provider's browser-side hybrid encryption.
pub fn decrypt(blob: &EncryptedBlob, keypair: &HybridKeypair) -> Result<Vec<u8>> {
    // 1. X25519 ECDH with the ephemeral public key from the sender.
    let static_secret = StaticSecret::from(keypair.x25519_private);
    let ephemeral_public = PublicKey::from(blob.ephemeral_x25519_pub);
    let ss_classical = static_secret.diffie_hellman(&ephemeral_public);

    // 2. ML-KEM-768 decapsulation.
    let dk = load_mlkem_decapsulation_key(keypair)?;
    let ct: Ciphertext<MlKem768> = blob
        .ct_pq
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("invalid ML-KEM ciphertext length"))?;
    let ss_pq = dk.decapsulate(&ct);

    // 3. HKDF-SHA256(ss_classical || ss_pq) → AES-256 key.
    let mut ikm = Vec::with_capacity(64);
    ikm.extend_from_slice(ss_classical.as_bytes());
    ikm.extend_from_slice(ss_pq.as_slice());

    let mut aes_key = [0u8; 32];
    Hkdf::<Sha256>::new(Some(&[0u8; 32]), &ikm)
        .expand(b"patient-vault-v1", &mut aes_key)
        .map_err(|e| anyhow!("HKDF expand failed: {e}"))?;

    // 4. AES-256-GCM decrypt.
    let cipher = Aes256Gcm::new_from_slice(&aes_key).map_err(|e| anyhow!("invalid AES key: {e}"))?;
    cipher
        .decrypt(Nonce::from_slice(&blob.nonce), blob.ciphertext.as_ref())
        .map_err(|e| anyhow!("AES-GCM decrypt failed: {e}"))
}

fn load_mlkem_decapsulation_key(keypair: &HybridKeypair) -> Result<DecapsulationKey768> {
    if keypair.mlkem_decap.len() == 64 {
        let seed: ml_kem::Seed = keypair
            .mlkem_decap
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid ML-KEM seed length"))?;
        let (dk, _) = MlKem768::from_seed(&seed);
        Ok(dk)
    } else {
        // Legacy expanded key storage (~2400 bytes) from earlier scaffold versions.
        #[allow(deprecated)]
        use ml_kem::ExpandedKeyEncoding;
        let expanded: ml_kem::ml_kem_768::ExpandedDecapsulationKey = keypair
            .mlkem_decap
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid expanded ML-KEM decapsulation key length"))?;
        DecapsulationKey768::from_expanded_bytes(&expanded)
            .map_err(|_| anyhow!("invalid expanded ML-KEM decapsulation key"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_key_generation_and_usb_id() {
        let keypair = generate_keypair();
        assert_eq!(keypair.x25519_private.len(), 32);
        assert_eq!(keypair.x25519_public.len(), 32);
        assert_eq!(keypair.mlkem_decap.len(), 64);
        assert_eq!(keypair.mlkem_encap.len(), 1184);

        let pk = HybridPublicKey {
            x25519: keypair.x25519_public,
            mlkem: keypair.mlkem_encap.clone(),
        };
        let id1 = derive_usb_id(&pk);
        let id2 = derive_usb_id(&pk);
        assert_eq!(id1, id2);
    }

    #[test]
    fn local_encrypt_decrypt_roundtrip() {
        let keypair = generate_keypair();
        let recipient = HybridPublicKey {
            x25519: keypair.x25519_public,
            mlkem: keypair.mlkem_encap.clone(),
        };

        let plaintext = b"patient-vault test payload";
        let blob = encrypt_for_test(plaintext, &recipient).expect("encrypt");
        let decrypted = decrypt(&blob, &keypair).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    /// Mirror the browser-side encryptPayload flow for integration tests.
    fn encrypt_for_test(plaintext: &[u8], recipient: &HybridPublicKey) -> Result<EncryptedBlob> {
        use ml_kem::EncapsulationKey;
        use ml_kem::ml_kem_768::EncapsulationKey as EncapsulationKey768;
        use rand::RngCore;

        let ephemeral_secret = StaticSecret::random_from_rng(&mut OsRng);
        let ephemeral_public = PublicKey::from(&ephemeral_secret);
        let recipient_public = PublicKey::from(recipient.x25519);
        let ss_classical = ephemeral_secret.diffie_hellman(&recipient_public);

        let ek = EncapsulationKey768::new(recipient.mlkem.as_slice().try_into().map_err(|_| {
            anyhow!("invalid ML-KEM encapsulation key length")
        })?)
        .map_err(|_| anyhow!("invalid ML-KEM encapsulation key"))?;
        let (ct, ss_pq) = ek.encapsulate();

        let mut ikm = Vec::with_capacity(64);
        ikm.extend_from_slice(ss_classical.as_bytes());
        ikm.extend_from_slice(ss_pq.as_slice());

        let mut aes_key = [0u8; 32];
        Hkdf::<Sha256>::new(Some(&[0u8; 32]), &ikm)
            .expand(b"patient-vault-v1", &mut aes_key)
            .map_err(|e| anyhow!("HKDF expand failed: {e}"))?;

        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        let cipher = Aes256Gcm::new_from_slice(&aes_key).map_err(|e| anyhow!("invalid AES key: {e}"))?;
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|e| anyhow!("AES-GCM encrypt failed: {e}"))?;

        Ok(EncryptedBlob {
            ct_pq: ct.as_slice().to_vec(),
            ciphertext,
            nonce,
            ephemeral_x25519_pub: ephemeral_public.to_bytes(),
        })
    }
}
