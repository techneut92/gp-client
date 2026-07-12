//! ChaCha20-Poly1305 framing, wire-compatible with gpservice's `Crypto`.
//!
//! Frame layout: `[12-byte nonce][ciphertext]`, plaintext is `serde_json`.
//! Mirrors `crates/gpapi/src/utils/crypto.rs` in the GlobalProtect-openconnect
//! fork — keep the algorithm/format in sync with that file.

use anyhow::{Context, Result, ensure};
use chacha20poly1305::{
  AeadCore, ChaCha20Poly1305, Key, KeyInit,
  aead::{Aead, OsRng},
};
use serde::{Serialize, de::DeserializeOwned};

const NONCE_LEN: usize = 12;

pub struct Crypto {
  key: Vec<u8>,
}

impl Crypto {
  /// `key` must be 32 bytes (ChaCha20-Poly1305 key length).
  pub fn new(key: Vec<u8>) -> Self {
    Self { key }
  }

  pub fn encrypt<T: Serialize>(&self, value: &T) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&self.key));
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let plaintext = serde_json::to_vec(value)?;
    let ciphertext = cipher
      .encrypt(&nonce, plaintext.as_ref())
      .map_err(|e| anyhow::anyhow!("encrypt failed: {e}"))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
  }

  pub fn decrypt<T: DeserializeOwned>(&self, frame: &[u8]) -> Result<T> {
    ensure!(frame.len() > NONCE_LEN, "frame shorter than nonce");
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&self.key));

    let (nonce, ciphertext) = frame.split_at(NONCE_LEN);
    let plaintext = cipher
      .decrypt(nonce.into(), ciphertext)
      .map_err(|e| anyhow::anyhow!("decrypt failed: {e}"))?;

    serde_json::from_slice(&plaintext).context("deserialize plaintext")
  }
}
