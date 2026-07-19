//! Persisted options. Everything here is cached to `~/.config/gp-client/config.json`
//! — note there is intentionally **no PIN field**, so the PIN is never written to disk.

use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEFAULT_MODULE: &str = "/usr/lib64/opensc-pkcs11.so";
/// Installed location of the backend.
pub const INSTALLED_GPSERVICE: &str = "/usr/bin/gpservice";

/// Atomically write `data` to `path` with mode 0600: write a sibling temp with
/// the mode set at creation, fsync, then rename over the target. Avoids both the
/// "default umask, then chmod" window and a torn file on crash mid-write.
fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
  use std::io::Write;
  let dir = path.parent().unwrap_or_else(|| Path::new("."));
  std::fs::create_dir_all(dir)?;
  let tmp = dir.join(format!(
    ".{}.tmp",
    path.file_name().and_then(|s| s.to_str()).unwrap_or("f")
  ));
  {
    let mut f = std::fs::OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .mode(0o600)
      .open(&tmp)?;
    f.write_all(data)?;
    f.sync_all()?;
  }
  std::fs::rename(&tmp, path)
}

/// Atomic private write, exposed for the vault and other secret files.
pub fn write_secret_file(path: &Path, data: &[u8]) -> std::io::Result<()> {
  write_private(path, data)
}

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
  /// One-time migration marker: once true, the connection/SSO settings above
  /// have been copied onto each stored identity (they are now edited per
  /// identity). The `os`/`user_agent`/`auth_view`/mtu/... fields on `Config`
  /// remain only as the migration source and are no longer read at connect.
  pub conn_migrated_to_identity: bool,
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
      conn_migrated_to_identity: false,
    }
  }
}

fn config_path() -> Option<PathBuf> {
  directories::ProjectDirs::from("", "", "gp-client").map(|d| d.config_dir().join("config.json"))
}

/// Path to the encrypted identity vault.
pub fn vault_path() -> Option<PathBuf> {
  directories::ProjectDirs::from("", "", "gp-client").map(|d| d.config_dir().join("identities.enc"))
}

/// Resolve the gpservice binary. Normally the installed path (which the polkit
/// passwordless rule matches exactly); a dev build can point at an uninstalled
/// binary via `GP_GPSERVICE_BIN` so no personal path is baked into the release.
pub fn gpservice_binary() -> String {
  match std::env::var("GP_GPSERVICE_BIN") {
    Ok(p) if !p.is_empty() => p,
    _ => INSTALLED_GPSERVICE.to_string(),
  }
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
    if let Ok(json) = serde_json::to_string_pretty(self) {
      let _ = write_private(&path, json.as_bytes());
    }
  }
}
