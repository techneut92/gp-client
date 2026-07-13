//! One-time, silent import of settings from the predecessor GUI (gpgui) on
//! gp-client's first run.
//!
//! The vault is wire-compatible (identical `Identity`, `[salt][nonce][ct]` layout
//! and Argon2 params), so `identities.enc` copies over verbatim and the user's
//! existing master PIN unlocks it. The config deserializes into gp-client's own
//! `Config` (matching fields carry; gpgui-only fields are ignored, missing ones
//! default), then re-saves in our format.
//!
//! Everything here is best-effort: nothing to import, gp-client already used, or
//! any error — we just carry on. It runs before the config/vault are loaded.

use std::path::{Path, PathBuf};

/// gp-client's own config dir (…/gp-client). In a Flatpak this is the sandboxed
/// XDG config dir.
fn our_dir() -> Option<PathBuf> {
  directories::ProjectDirs::from("", "", "gp-client").map(|d| d.config_dir().to_path_buf())
}

/// Candidate locations of the predecessor gpgui's config dir — its native path
/// and its Flatpak per-app path. A native gp-client can read both; a Flatpak
/// gp-client reads the granted `~/.var/app/…gpgui` one (see the manifest's
/// `--filesystem=…:ro`).
fn gpgui_dirs() -> Vec<PathBuf> {
  let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
    return Vec::new();
  };
  vec![
    home.join(".config/gpgui-ng"),
    home.join(".var/app/io.github.techneut92.gpgui/config/gpgui-ng"),
  ]
}

fn has_data(dir: &Path) -> bool {
  dir.join("identities.enc").exists() || dir.join("config.json").exists()
}

/// Import gpgui's vault + config into gp-client, once, if this is a fresh install.
pub fn run() {
  let Some(dst) = our_dir() else { return };
  // First run only: never touch an install that already has its own data.
  if has_data(&dst) {
    return;
  }
  let Some(src) = gpgui_dirs().into_iter().find(|d| has_data(d)) else {
    return;
  };
  if std::fs::create_dir_all(&dst).is_err() {
    return;
  }

  // Vault copies verbatim — identical format + crypto, so the same master PIN
  // unlocks it in gp-client.
  let src_vault = src.join("identities.enc");
  if src_vault.exists() {
    if let Ok(data) = std::fs::read(&src_vault) {
      let _ = crate::config::write_secret_file(&dst.join("identities.enc"), &data);
    }
  }

  // Config: deserialize gpgui's JSON into our Config and re-save in our format.
  if let Ok(json) = std::fs::read_to_string(src.join("config.json")) {
    if let Ok(mut cfg) = serde_json::from_str::<crate::config::Config>(&json) {
      // Don't carry "remember unlock": the master PIN lives in gpgui's keyring
      // (and Flatpak's secret portal scopes it per-app), so it can't transfer.
      // The user re-enables it once and gp-client stores its own copy.
      cfg.remember_unlock = false;
      cfg.save();
    }
  }

  tracing::info!("imported settings from a previous GP Client (gpgui) install at {}", src.display());
}
