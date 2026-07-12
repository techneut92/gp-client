//! Auto-float in tiling shells. Some Linux tilers tile *every* window, including
//! our small fixed-size one. On Wayland an app can't float itself, but most
//! tilers keep a user "float exception" list — so we register our window there
//! on startup (per-user, with user privileges). This is why it lives in the app
//! and not the package installer: a root postinstall can't safely touch every
//! user's `~/.config`, and a Flatpak build can reconcile it the same way (given
//! host config access in the manifest).
//!
//! Our window's WM_CLASS / app-id is `gpgui` natively and
//! `io.github.techneut92.gpgui` under Flatpak. Pop Shell matches its `class`
//! field as a case-insensitive regex, so the single token `gpgui` covers both.

/// Regex/token that matches our window across native and Flatpak builds.
const MATCH: &str = "gpgui";

/// Register float exceptions in whatever tiler is present. Idempotent; safe to
/// call on every startup.
pub fn ensure_float_exceptions() {
  ensure_pop_shell();
}

/// Pop Shell keeps float exceptions in `~/.config/pop-shell/config.json`
/// (`{ "float": [{ "class": <regex> }] }`). Add ours if absent.
fn ensure_pop_shell() {
  // Pop Shell reads ~/.config/pop-shell on the host, so target the host config
  // (exposed under Flatpak via --filesystem=xdg-config/pop-shell).
  let Some(base) = crate::system::host_config_dir() else { return };
  let dir = base.join("pop-shell");
  // Only act when Pop Shell is actually present (it creates this dir).
  if !dir.exists() {
    return;
  }
  let path = dir.join("config.json");

  let mut root: serde_json::Value = std::fs::read_to_string(&path)
    .ok()
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_else(|| serde_json::json!({ "float": [], "skiptaskbarhidden": [], "log_on_focus": false }));

  // Ensure `float` is an array.
  if !root.get("float").map(|v| v.is_array()).unwrap_or(false) {
    root["float"] = serde_json::json!([]);
  }
  let arr = root["float"].as_array_mut().unwrap();

  // Idempotent: skip if we already have a rule for our class.
  let present = arr.iter().any(|r| {
    r.get("class")
      .and_then(|c| c.as_str())
      .map(|c| c.eq_ignore_ascii_case(MATCH))
      .unwrap_or(false)
  });
  if present {
    return;
  }

  arr.push(serde_json::json!({ "class": MATCH }));
  if let Ok(s) = serde_json::to_string_pretty(&root) {
    let _ = std::fs::write(&path, s);
    tracing::info!("registered Pop Shell float exception for '{MATCH}'");
  }
}
