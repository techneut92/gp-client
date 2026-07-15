//! Migration from the predecessor GUI (gpgui). The user drives it from the
//! "Import from GP Client" screen: [`import_available`] decides whether to show
//! it, and [`import_now`] copies everything over when they click Import.
//!
//! The vault is wire-compatible (identical `Identity`, `[salt][nonce][ct]` layout
//! and Argon2 params), so `identities.enc` copies over verbatim and the user's
//! existing master PIN unlocks it. The config deserializes into gp-client's own
//! `Config` (matching fields carry; gpgui-only fields are ignored, missing ones
//! default), then re-saves in our format — all settings included.
//!
// TODO(2026-10, GPC-29): delete this whole migration path once users have moved.

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

/// Whether to offer the import screen: gp-client has no data of its own yet, and
/// a predecessor gpgui with data is present to import from.
pub fn import_available() -> bool {
  match our_dir() {
    Some(d) if !has_data(&d) => gpgui_dirs().iter().any(|d| has_data(d)),
    _ => false,
  }
}

/// Copy gpgui's vault + config into gp-client. Returns an error (without touching
/// the source) if anything critical fails, so the caller must NOT remove the old
/// app unless this succeeded.
pub fn import_now() -> Result<(), String> {
  let dst = our_dir().ok_or("couldn't locate gp-client's config directory")?;
  let src = gpgui_dirs()
    .into_iter()
    .find(|d| has_data(d))
    .ok_or("nothing to import from")?;
  std::fs::create_dir_all(&dst).map_err(|e| format!("couldn't create the config directory: {e}"))?;

  // Vault copies verbatim — identical format + crypto, so the same master PIN
  // unlocks it in gp-client.
  let src_vault = src.join("identities.enc");
  if src_vault.exists() {
    let data = std::fs::read(&src_vault).map_err(|e| format!("couldn't read the vault: {e}"))?;
    crate::config::write_secret_file(&dst.join("identities.enc"), &data)
      .map_err(|e| format!("couldn't write the vault: {e}"))?;
  }

  // Config: deserialize gpgui's JSON into our Config and re-save in our format.
  // Carry everything, including "remember unlock" (auto-unlock) — the user set
  // it once and expects it to survive the move.
  if let Ok(json) = std::fs::read_to_string(src.join("config.json")) {
    if let Ok(cfg) = serde_json::from_str::<crate::config::Config>(&json) {
      cfg.save();
    }
  }

  write_marker();
  tracing::info!("imported settings from a previous GP Client (gpgui) install at {}", src.display());
  Ok(())
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

/// Remove the predecessor gpgui and its data — called only after [`import_now`]
/// succeeded. Under Flatpak we uninstall the app with `--delete-data`; natively
/// we can't remove the distro package, so we at least wipe its data directories.
pub fn remove_predecessor() -> Result<(), String> {
  if crate::system::is_flatpak() {
    let out = std::process::Command::new("flatpak-spawn")
      .args(["--host", "flatpak", "uninstall", "-y", "--delete-data", GPGUI_ID])
      .output()
      .map_err(|e| format!("couldn't run the uninstaller: {e}"))?;
    if !out.status.success() {
      let stderr = String::from_utf8_lossy(&out.stderr);
      return Err(stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("uninstall failed").trim().chars().take(160).collect());
    }
  } else {
    // Native: leave the package to the user's package manager, but remove the
    // leftover data so nothing of the old app remains.
    for dir in gpgui_dirs() {
      let _ = std::fs::remove_dir_all(&dir);
    }
  }
  clear_marker();
  Ok(())
}
