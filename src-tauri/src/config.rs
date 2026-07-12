//! Persisted options. Everything here is cached to `~/.config/gpgui-ng/config.json`
//! — note there is intentionally **no PIN field**, so the PIN is never written to disk.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use base64::Engine;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, aead::OsRng};
use serde::{Deserialize, Serialize};

pub const DEFAULT_MODULE: &str = "/usr/lib64/opensc-pkcs11.so";
/// Installed location; a dev build falls back to the workspace target dir.
pub const INSTALLED_GPSERVICE: &str = "/usr/bin/gpservice";
pub const DEV_GPSERVICE: &str =
  "/home/dylan/Projects/GlobalProtect-openconnect-pkcs11/target/debug/gpservice";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
  pub url: String,
  pub as_gateway: bool,
  pub os: String,
  pub user_agent: String,
  pub module_path: String,
  /// Remembered selection (id is the PKCS#11 hex id, e.g. "03").
  pub last_cert_id: String,
  pub last_cert_manufacturer: String,
  /// 0 = PKCS#11 smart card, 1 = certificate file.
  pub auth_method: i32,
  /// SSO method: "webview" (embedded) or "browser" (system browser).
  pub auth_view: String,
  /// Remembered cert-file paths (key password is never persisted).
  pub cert_file: String,
  pub key_file: String,
  // ---- advanced connection options (settings window) ----
  pub mtu: u32,
  pub reconnect_timeout: u32,
  pub force_dpd: u32,
  pub disable_ipv6: bool,
  pub no_dtls: bool,
  pub no_xmlpost: bool,
  pub ignore_tls_errors: bool,
  pub vpnc_script: String,
  pub local_hostname: String,
  pub os_version: String,
  pub client_version: String,
  // ---- general (startup & tray) ----
  /// Tray icon concept: "shield" or "ring".
  pub tray_icon: String,
  /// Launch the GUI at login (XDG autostart). Defaults on; the entry is created
  /// on first run.
  pub run_at_startup: bool,
  /// Start with the window hidden to the tray (any launch, not just autostart).
  pub start_minimized: bool,
  /// Remember the vault master PIN in the desktop secret store (Secret Service)
  /// and auto-unlock on launch. Off by default.
  pub remember_unlock: bool,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      url: String::new(),
      as_gateway: true,
      os: "Windows".into(),
      user_agent: "PAN GlobalProtect".into(),
      module_path: DEFAULT_MODULE.into(),
      last_cert_id: String::new(),
      last_cert_manufacturer: String::new(),
      auth_method: 0,
      auth_view: "webview".into(),
      cert_file: String::new(),
      key_file: String::new(),
      mtu: 0,
      reconnect_timeout: 0,
      force_dpd: 0,
      disable_ipv6: false,
      no_dtls: false,
      no_xmlpost: false,
      ignore_tls_errors: false,
      vpnc_script: String::new(),
      local_hostname: String::new(),
      os_version: String::new(),
      client_version: String::new(),
      tray_icon: "shield".into(),
      run_at_startup: true,
      start_minimized: false,
      remember_unlock: false,
    }
  }
}

fn config_path() -> Option<PathBuf> {
  directories::ProjectDirs::from("", "", "gpgui-ng").map(|d| d.config_dir().join("config.json"))
}

/// Path to the encrypted identity vault.
pub fn vault_path() -> Option<PathBuf> {
  directories::ProjectDirs::from("", "", "gpgui-ng").map(|d| d.config_dir().join("identities.enc"))
}

/// Resolve the gpservice binary: the installed path if present, else the dev
/// build. The polkit passwordless rule matches the installed path exactly.
pub fn gpservice_binary() -> String {
  if std::path::Path::new(INSTALLED_GPSERVICE).exists() {
    INSTALLED_GPSERVICE.to_string()
  } else {
    DEV_GPSERVICE.to_string()
  }
}

/// Load the persisted gpservice API key, or generate and persist a fresh 32-byte
/// key. This is the shared secret for the loopback WS, stored `0600` in the
/// config dir so only this user can drive the (root) service.
pub fn load_or_create_api_key() -> Vec<u8> {
  let path = config_path().map(|p| p.with_file_name("api_key"));

  if let Some(path) = &path {
    if let Ok(b64) = std::fs::read_to_string(path) {
      if let Ok(key) = base64::engine::general_purpose::STANDARD.decode(b64.trim()) {
        if key.len() == 32 {
          return key;
        }
      }
    }
  }

  let key = ChaCha20Poly1305::generate_key(&mut OsRng).to_vec();
  if let Some(path) = path {
    if let Some(dir) = path.parent() {
      let _ = std::fs::create_dir_all(dir);
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(&key);
    if std::fs::write(&path, b64).is_ok() {
      let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
  }
  key
}

impl Config {
  pub fn load() -> Self {
    config_path()
      .and_then(|p| std::fs::read_to_string(p).ok())
      .and_then(|s| serde_json::from_str(&s).ok())
      .unwrap_or_default()
  }

  pub fn save(&self) {
    let Some(path) = config_path() else { return };
    if let Some(dir) = path.parent() {
      let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(self) {
      let _ = std::fs::write(path, json);
    }
  }
}
