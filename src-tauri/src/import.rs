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

  // Record that we migrated, so we can later offer to remove the old app — only
  // AFTER the identities are safely here.
  write_marker();
  tracing::info!("imported settings from a previous GP Client (gpgui) install at {}", src.display());
}

const GPGUI_ID: &str = "io.github.techneut92.gpgui";

fn marker_path() -> Option<PathBuf> {
  our_dir().map(|d| d.join(".migrated-from-gpgui"))
}
fn write_marker() {
  if let Some(p) = marker_path() {
    let _ = std::fs::write(p, b"");
  }
}
fn clear_marker() {
  if let Some(p) = marker_path() {
    let _ = std::fs::remove_file(p);
  }
}

/// True if we imported from gpgui on a previous run (and haven't removed it yet).
pub fn migrated() -> bool {
  marker_path().map(|p| p.exists()).unwrap_or(false)
}

/// Whether the predecessor gpgui is still installed.
pub fn predecessor_installed() -> bool {
  if crate::system::is_flatpak() {
    std::process::Command::new("flatpak-spawn")
      .args(["--host", "flatpak", "info", GPGUI_ID])
      .output()
      .map(|o| o.status.success())
      .unwrap_or(false)
  } else {
    // Native package removal is distro-specific; we detect the binary but leave
    // the actual uninstall to the user (see remove_predecessor).
    std::path::Path::new("/usr/bin/gpgui").exists()
  }
}

/// Remove the predecessor gpgui — called only after the migration is confirmed
/// (identities already imported here). Flatpak only; native is left to the user.
pub fn remove_predecessor() -> Result<(), String> {
  if !crate::system::is_flatpak() {
    return Err("Please remove the old app with your package manager.".into());
  }
  let out = std::process::Command::new("flatpak-spawn")
    .args(["--host", "flatpak", "uninstall", "-y", GPGUI_ID])
    .output()
    .map_err(|e| format!("couldn't run the uninstaller: {e}"))?;
  if out.status.success() {
    clear_marker();
    Ok(())
  } else {
    let stderr = String::from_utf8_lossy(&out.stderr);
    Err(stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("uninstall failed").trim().chars().take(160).collect())
  }
}
