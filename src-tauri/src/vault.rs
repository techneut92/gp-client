//! Encrypted identity vault.
//!
//! Identities (including secrets — PIN, password, key passphrase) are stored in
//! `~/.config/gpgui-ng/identities.enc`, encrypted with a key derived from a
//! **master PIN** via Argon2id. The plaintext identities live in memory only
//! while the vault is unlocked.
//!
//! File layout: `[16-byte salt][12-byte nonce][ChaCha20-Poly1305 ciphertext]`,
//! plaintext = JSON `Vec<Identity>`.

use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use chacha20poly1305::aead::rand_core::RngCore;
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key, KeyInit};
use serde::{Deserialize, Serialize};

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

/// A saved identity (connection profile). Held in plaintext only while unlocked.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Identity {
  pub name: String,
  pub portal: String,
  /// 0 smart card, 1 cert file, 2 SAML, 3 username/password.
  pub auth_method: i32,
  pub as_gateway: bool,
  pub module_path: String,
  pub username: String,
  pub password: String,
  pub pin: String,
  pub cert_id: String,
  pub cert_manufacturer: String,
  pub cert_file: String,
  pub key_file: String,
  pub key_password: String,
}

pub struct Vault {
  path: PathBuf,
  salt: [u8; SALT_LEN],
  key: Option<[u8; 32]>,
  identities: Vec<Identity>,
  /// True if the encrypted file exists on disk.
  pub exists: bool,
  pub unlocked: bool,
}

fn derive_key(pin: &str, salt: &[u8]) -> Result<[u8; 32]> {
  let mut key = [0u8; 32];
  argon2::Argon2::default()
    .hash_password_into(pin.as_bytes(), salt, &mut key)
    .map_err(|e| anyhow!("key derivation failed: {e}"))?;
  Ok(key)
}

impl Vault {
  /// Load vault metadata. Reads the salt if the file exists; stays locked.
  pub fn load(path: PathBuf) -> Self {
    let data = std::fs::read(&path).ok();
    match data {
      Some(bytes) if bytes.len() > SALT_LEN + NONCE_LEN => {
        let mut salt = [0u8; SALT_LEN];
        salt.copy_from_slice(&bytes[..SALT_LEN]);
        Vault { path, salt, key: None, identities: Vec::new(), exists: true, unlocked: false }
      }
      _ => Vault { path, salt: [0u8; SALT_LEN], key: None, identities: Vec::new(), exists: false, unlocked: false },
    }
  }

  pub fn identities(&self) -> &[Identity] {
    &self.identities
  }

  /// First-time setup: pick a salt, derive the key, start an empty unlocked vault.
  pub fn set_master_pin(&mut self, pin: &str) -> Result<()> {
    if pin.is_empty() {
      bail!("master PIN cannot be empty");
    }
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    self.salt = salt;
    self.key = Some(derive_key(pin, &salt)?);
    self.identities = Vec::new();
    self.unlocked = true;
    self.exists = true;
    self.save()
  }

  /// Decrypt the vault with the master PIN.
  pub fn unlock(&mut self, pin: &str) -> Result<()> {
    let bytes = std::fs::read(&self.path)?;
    if bytes.len() <= SALT_LEN + NONCE_LEN {
      bail!("vault file is corrupt");
    }
    let key = derive_key(pin, &self.salt)?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let nonce = &bytes[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &bytes[SALT_LEN + NONCE_LEN..];
    let plaintext = cipher
      .decrypt(nonce.into(), ciphertext)
      .map_err(|_| anyhow!("wrong master PIN"))?;
    self.identities = serde_json::from_slice(&plaintext).unwrap_or_default();
    self.key = Some(key);
    self.unlocked = true;
    Ok(())
  }

  pub fn lock(&mut self) {
    self.key = None;
    self.identities.clear();
    self.unlocked = false;
  }

  /// Forgotten-PIN reset: delete the encrypted vault and return to the no-vault
  /// state so the user can set a new master PIN. Saved identities are gone — they
  /// were encrypted with the forgotten PIN and cannot be recovered.
  pub fn reset(&mut self) {
    let _ = std::fs::remove_file(&self.path);
    self.salt = [0u8; SALT_LEN];
    self.key = None;
    self.identities.clear();
    self.exists = false;
    self.unlocked = false;
  }

  pub fn upsert(&mut self, identity: Identity) -> Result<()> {
    if let Some(e) = self.identities.iter_mut().find(|i| i.name == identity.name) {
      *e = identity;
    } else {
      self.identities.push(identity);
    }
    self.save()
  }

  pub fn remove(&mut self, name: &str) -> Result<()> {
    self.identities.retain(|i| i.name != name);
    self.save()
  }

  fn save(&self) -> Result<()> {
    let key = self.key.ok_or_else(|| anyhow!("vault is locked"))?;
    if let Some(dir) = self.path.parent() {
      std::fs::create_dir_all(dir)?;
    }
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let plaintext = serde_json::to_vec(&self.identities)?;
    let ciphertext = cipher
      .encrypt(&nonce, plaintext.as_ref())
      .map_err(|e| anyhow!("encrypt failed: {e}"))?;
    let mut out = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&self.salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    std::fs::write(&self.path, out)?;
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
  }
}
