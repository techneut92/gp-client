// GP Client GUI
// Copyright (C) 2026 Dylan Westra (techneut92)
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later
// version. See the LICENSE file for the full text.
//
// Links no GPL code itself: authentication runs in the separate `gpservice`
// backend, which this GUI drives only over the `gp-protocol` wire contract.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! gpgui — Tauri front-end for GlobalProtect-openconnect (PKCS#11 fork).
//!
//! The privileged tunnel runs in `gpservice`; this GUI is unprivileged. It
//! authenticates (prelogin + SAML, via gpauth or the browser), then drives
//! gpservice over an encrypted loopback WebSocket.
//!
//! Transport seam for Flatpak: today the GUI launches gpservice via pkexec and
//! talks to it over loopback (`client` + `vpn::ensure_service`). A Flatpak build
//! can't pkexec or see `/var/run`, so that pair is the single place a future
//! D-Bus system-service transport slots in — nothing above it changes.

mod autostart;
mod config;
mod connect;
mod import;
mod dbus_client;
mod pkcs11;
mod saml;
mod secrets;
mod single_instance;
mod state;
mod system;
mod tiling;
mod transport;
mod tray;
mod vault;
mod vpn;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State, WindowEvent};

use config::Config;
use state::{Shared, Status};
use tray::GpTray;
use vault::{Identity, Vault};
use vpn::{ConnectParams, Notifier, UiCommand};

/// Shared handles exposed to the Tauri commands.
struct AppState {
  cmd_tx: std::sync::mpsc::Sender<UiCommand>,
  /// Bridge for the inline smart-card PIN prompt (see `vpn::PinSlot`).
  pin_tx: vpn::PinSlot,
  shared: Arc<Mutex<Shared>>,
  cfg: Arc<Mutex<Config>>,
  vault: Arc<Mutex<Vault>>,
  /// True once a system tray was registered. The main window only hides on
  /// close (close-to-tray) when this holds — otherwise there would be no way to
  /// bring it back, so we let the close quit the app instead.
  tray_available: Arc<AtomicBool>,
  /// The live tray handle, so a settings change can repaint it immediately.
  tray: Arc<Mutex<Option<Arc<tray::TrayHandle>>>>,
}

/// Advanced options edited in the settings window (persisted; read at connect).
/// The settings window now edits general (startup + tray) options only; the
/// connection/SSO options moved onto each identity (see the editor window).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsForm {
  tray_icon: String,
  run_at_startup: bool,
  start_minimized: bool,
  remember_unlock: bool,
}

/// A certificate row for the picker.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CertDto {
  id: String,
  manufacturer: String,
  label: String,
  subject: String,
  cn: String,
  token: String,
  model: String,
  slot: String,
  expiry: String,
  display: String,
  uri: String,
}

/// Connection state pushed to the webview (also returned by `get_state`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatePayload {
  status: String,
  kind: i32,
  log: String,
  active: bool,
  busy: bool,
  portal: String,
  gateway: String,
  ip: String,
  iface: String,
  expires: String,
  /// Drives the inline smart-card PIN prompt in the connecting view.
  pin_required: bool,
  /// The smart-card manufacturer being unlocked (subtitle + card-block label).
  pin_prompt: String,
  /// The PKCS#11 module file name behind the prompt (card-block sub-line).
  pin_module: String,
  /// Drives the inline MFA challenge card (from the backend's MfaChallenge).
  mfa_required: bool,
  mfa_prompt: String,
  /// Drives the inline gateway picker (from the backend's GatewaySelect).
  gw_required: bool,
  /// The portal's gateways as `{name, host}` rows.
  gw_list: Vec<GwOption>,
  /// Address of the region-preferred gateway (pre-selected in the picker).
  gw_preferred: String,
}

/// One row of the connect-time gateway picker.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GwOption {
  name: String,
  host: String,
}

fn status_kind(status: &Status) -> i32 {
  match status {
    Status::Disconnected => 0,
    Status::Connecting | Status::Disconnecting => 1,
    Status::Connected => 2,
    Status::Error(_) => 3,
    Status::Reconnecting => 4,
  }
}

fn build_state(shared: &Arc<Mutex<Shared>>) -> StatePayload {
  let s = shared.lock().unwrap();
  StatePayload {
    status: s.status.label(),
    kind: status_kind(&s.status),
    log: s.log.clone(),
    active: s.status.is_active(),
    busy: matches!(s.status, Status::Connecting | Status::Disconnecting),
    portal: s.conn.portal.clone(),
    gateway: s.conn.gateway.clone(),
    ip: s.conn.ip.clone(),
    iface: s.conn.iface.clone(),
    expires: s.conn.expires.clone(),
    pin_required: s.pin_required,
    pin_prompt: s.pin_prompt.clone(),
    pin_module: s.pin_module.clone(),
    mfa_required: s.mfa_required,
    mfa_prompt: s.mfa_prompt.clone(),
    gw_required: s.gw_required,
    gw_list: s
      .gw_list
      .iter()
      .map(|(name, host)| GwOption {
        name: name.clone(),
        host: host.clone(),
      })
      .collect(),
    gw_preferred: s.gw_preferred.clone(),
  }
}

// ---- commands ----

#[tauri::command]
fn get_config(state: State<AppState>) -> Config {
  state.cfg.lock().unwrap().clone()
}

#[tauri::command]
fn get_state(state: State<AppState>) -> StatePayload {
  build_state(&state.shared)
}

#[tauri::command]
fn available_modules() -> Vec<String> {
  pkcs11::available_modules()
}

#[tauri::command]
fn scan_certs(module: String) -> Vec<CertDto> {
  pkcs11::enumerate(&module)
    .unwrap_or_default()
    .into_iter()
    .map(|c| CertDto {
      display: c.display(),
      uri: c.uri(),
      cn: c.cn(),
      id: c.id,
      manufacturer: c.manufacturer,
      label: c.label,
      subject: c.subject,
      token: c.token,
      model: c.model,
      slot: c.slot,
      expiry: c.expiry,
    })
    .collect()
}

#[tauri::command]
fn browse_file(title: String) -> Option<String> {
  // Runs on a Tauri worker thread, so blocking on the portal dialog is fine.
  pollster::block_on(rfd::AsyncFileDialog::new().set_title(&title).pick_file())
    .map(|f| f.path().to_string_lossy().to_string())
}

#[tauri::command]
fn disconnect(state: State<AppState>) {
  // If a smart-card PIN prompt is pending, cancel it (unblocking the connect
  // pipeline) rather than queuing a disconnect behind the blocked manager loop.
  if let Some(tx) = state.pin_tx.lock().unwrap().take() {
    let _ = tx.send(None);
    return;
  }
  let _ = state.cmd_tx.send(UiCommand::Disconnect);
}

/// Open an http(s) URL in the user's browser (used by the Ko-fi / About links).
#[tauri::command]
fn open_url(url: String) {
  system::open_url(&url);
}

/// App / OS / backend status for the About and "backend missing" screens.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemInfo {
  gui_version: String,
  os_name: String,
  /// How the app is running (Source build / Native package / Flatpak).
  running: String,
  /// The OS package manager (used for install/update commands).
  install_kind: String,
  is_flatpak: bool,
  /// The Flatpak runtime ("GNOME Platform 50"), shown as its own row; None natively.
  flatpak_runtime: Option<String>,
  backend_installed: bool,
  backend_version: Option<String>,
  /// False when an installed backend is older than `MIN_BACKEND` (can't speak the
  /// v3 auth handoff), so the UI routes to the install/upgrade screen just as it
  /// does for a missing backend. True when the backend is new enough, or when it's
  /// present but its version couldn't be read (don't hide a working backend).
  backend_supported: bool,
  /// True when the backend speaks a *newer* wire protocol than this GUI's
  /// `gp_protocol::PROTOCOL_MAX` — the inverse of too-old. The UI routes to the
  /// "Update GP Client" screen (GPC-42) rather than the install/upgrade screen.
  /// False for backends too old to advertise a protocol range (never too new).
  backend_too_new: bool,
  /// The manual "update this app" command, matched to how the GUI was installed
  /// (Flatpak vs the native package manager) — the too-new screen's fallback.
  gui_update_cmd: String,
  /// Per-OS install steps, so the UI can render and offer a manual override.
  install_options: Vec<system::InstallOption>,
}

#[tauri::command]
async fn system_info() -> SystemInfo {
  // The backend lives on the host; under Flatpak `detect()` would report the GUI's
  // own kind ("Flatpak"), so use the host-aware probe for the backend's package mgr.
  let kind = system::host_install_kind();
  let backend_version = system::backend_version();
  // An installed-but-too-old backend can't drive the v3 auth handoff, so treat it
  // like a missing backend. Unknown version (present but unreadable) stays true so
  // we never hide a working backend behind the install screen.
  let backend_supported = match &backend_version {
    Some(v) => system::version_cmp(v, system::MIN_BACKEND) != std::cmp::Ordering::Less,
    None => true,
  };
  // `installed` is the value the GUI actually gates on: in the Flatpak it's the
  // D-Bus *activatability* of gpservice, not just whether the binary runs — so
  // log both. A common mismatch: the binary probes fine but the service isn't
  // activatable (e.g. the D-Bus daemon wasn't reloaded after an rpm-ostree
  // apply-live), which shows the "backend required" screen.
  let backend_installed = system::backend_installed();
  // Too-new: the oldest protocol the backend speaks is newer than the newest this
  // GUI understands, so they can't talk — update the GUI, not the backend. Old
  // backends don't advertise a range (None), so they're never flagged too new.
  let backend_too_new = system::backend_protocol().is_some_and(|(bmin, _)| bmin > gp_protocol::PROTOCOL_MAX);
  match &backend_version {
    Some(v) => tracing::info!(
      "Backend gpservice {v}: installed(dbus-activatable)={backend_installed}, supported={backend_supported} (min {}), too_new={backend_too_new} (gui protocol_max={})",
      system::MIN_BACKEND,
      gp_protocol::PROTOCOL_MAX
    ),
    None => tracing::info!("No backend gpservice version from the host (installed(dbus-activatable)={backend_installed})"),
  }
  SystemInfo {
    gui_version: system::GUI_VERSION.to_string(),
    os_name: system::os_pretty_name(),
    running: system::run_mode().to_string(),
    install_kind: system::install_kind_str(kind).to_string(),
    is_flatpak: system::is_flatpak(),
    flatpak_runtime: system::flatpak_runtime(),
    backend_installed,
    backend_version,
    backend_supported,
    backend_too_new,
    gui_update_cmd: system::gui_update_command(),
    install_options: {
      let v = system::latest_backend_release()
        .await
        .map(|r| r.version)
        .unwrap_or_else(|_| system::BACKEND_FALLBACK_VERSION.to_string());
      system::install_options(&v)
    },
  }
}

/// One-click backend install: download + install via a single pkexec prompt,
/// waiting for the real result so the UI reports success/failure honestly.
///
/// `version` is the target release to install. The updater passes the latest
/// release so the backend reaches it; when omitted (first-run install of a
/// missing backend) it defaults to this GUI's own version — a matched pair.
/// Passing it matters because the running GUI's `GUI_VERSION` doesn't advance
/// until a restart, so deriving the version from it would pin the backend to the
/// *old* version during an update.
#[tauri::command]
async fn install_backend(kind: Option<String>, version: Option<String>) -> serde_json::Value {
  let kind = kind.map(|s| system::kind_from_str(&s)).unwrap_or_else(system::host_install_kind);
  let version = match version {
    Some(v) => v,
    None => system::latest_backend_release()
      .await
      .map(|r| r.version)
      .unwrap_or_else(|_| system::BACKEND_FALLBACK_VERSION.to_string()),
  };
  let Some(script) = system::backend_install_script(kind, &version) else {
    return serde_json::json!({ "ok": false, "message": "No installer for this system type — use the steps below." });
  };
  let needs_reboot = kind == system::InstallKind::RpmOstree;
  match tauri::async_runtime::spawn_blocking(move || system::run_root_script_wait(&script)).await {
    Ok(Ok(())) => serde_json::json!({ "ok": true, "needsReboot": needs_reboot }),
    Ok(Err(message)) => serde_json::json!({ "ok": false, "message": message }),
    Err(_) => serde_json::json!({ "ok": false, "message": "The install task failed to run." }),
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfo {
  current: String,
  latest: String,
  /// Installed backend version and the backend's latest release — the backend
  /// versions independently of the GUI, so these are separate from current/latest.
  backend_current: String,
  backend_latest: String,
  /// The GUI itself is behind the latest release.
  available: bool,
  /// The installed backend is behind the latest release (separate from the GUI —
  /// they can be on different versions). Drives the "update backend" affordance.
  backend_update: bool,
  url: String,
  error: Option<String>,
}

/// Check the GitHub Releases API for a newer fork version (covers both the GUI
/// and the backend — they ship from the same release).
#[tauri::command]
async fn check_update() -> UpdateInfo {
  let current = system::GUI_VERSION.to_string();
  let backend = system::backend_version();
  let gui_repo_url = "https://github.com/techneut92/gp-client/releases".to_string();

  // The GUI updates from its own repo; the backend from the fork. They version
  // independently now, so check both releases separately.
  let gui = system::latest_gui_release().await;
  let backend_latest = system::latest_backend_release().await;
  let backend_current = backend.clone().unwrap_or_default();
  let backend_latest_ver = backend_latest.as_ref().map(|r| r.version.clone()).unwrap_or_default();

  let backend_update = match backend_latest.as_ref() {
    Ok(r) => {
      let newer = |v: &str| system::version_cmp(&r.version, v) == std::cmp::Ordering::Greater;
      // Old backend is an update even when the GUI is current. If installed but
      // its version can't be read, offer the update rather than skipping it.
      match backend.as_deref() {
        Some(v) => newer(v),
        None => system::backend_installed(),
      }
    }
    Err(_) => false,
  };

  match gui {
    Ok(r) => {
      let available = system::version_cmp(&r.version, &current) == std::cmp::Ordering::Greater;
      UpdateInfo {
        available,
        backend_update,
        current,
        latest: r.version,
        backend_current,
        backend_latest: backend_latest_ver,
        url: if r.url.is_empty() { gui_repo_url } else { r.url },
        // The GUI check succeeded; a backend-release fetch hiccup is reflected by
        // an empty backend_latest, not surfaced as a check error.
        error: None,
      }
    }
    Err(e) => UpdateInfo {
      current,
      latest: String::new(),
      backend_current,
      backend_latest: backend_latest_ver,
      available: false,
      backend_update,
      url: gui_repo_url,
      error: Some(e),
    },
  }
}

/// Relaunch GP Client to apply a freshly-installed GUI update. Under Flatpak, exit
/// and launch a fresh instance from the updated deployment; natively, re-exec.
#[tauri::command]
fn restart_app(app: tauri::AppHandle) {
  if system::is_flatpak() {
    system::spawn_fresh_flatpak();
    app.exit(0);
  } else {
    app.restart();
  }
}

/// Reboot the host (to finish applying a layered backend update).
#[tauri::command]
fn reboot_host() {
  system::reboot_host();
}

/// True when we migrated from the predecessor gpgui and it's still installed —
/// so the UI can offer to remove it (identities are already imported by then).
#[tauri::command]
fn predecessor_removable() -> bool {
  import::migrated() && import::predecessor_installed()
}

/// Remove the predecessor gpgui (Flatpak). Safe to call: it only runs after the
/// first-run import already copied the identities here.
#[tauri::command]
async fn remove_predecessor() -> Result<(), String> {
  tauri::async_runtime::spawn_blocking(import::remove_predecessor)
    .await
    .map_err(|e| e.to_string())?
}

/// Whether to show the "Import from GP Client" migration screen — gp-client is a
/// fresh install and a predecessor gpgui with data is present.
#[tauri::command]
fn import_available() -> bool {
  import::import_available()
}

/// Whether the predecessor gpgui *app* is still installed (vs. only its leftover
/// data). Drives the import-screen copy: with the app present, importing also
/// removes it; when only data lingers, the import keeps it (nothing to uninstall).
#[tauri::command]
fn predecessor_app_installed() -> bool {
  import::predecessor_installed()
}

/// Refresh the in-memory vault + config from the just-imported files on disk, so
/// the imported vault is recognised (unlock screen, not a fresh setup) and the
/// imported settings (incl. auto-unlock) take effect immediately.
fn reload_after_import(state: &AppState) {
  let vault_path = config::vault_path().unwrap_or_else(|| std::path::PathBuf::from("identities.enc"));
  *state.vault.lock().unwrap() = Vault::load(vault_path);
  *state.cfg.lock().unwrap() = Config::load();
}

/// Import everything from gpgui (identities + all settings, incl. auto-unlock),
/// then remove the old app and its data. The old app is only removed if the
/// import succeeded first.
#[tauri::command]
async fn import_from_gpgui(state: State<'_, AppState>) -> Result<(), String> {
  tauri::async_runtime::spawn_blocking(|| -> Result<(), String> {
    import::import_now()?;
    import::remove_predecessor()
  })
  .await
  .map_err(|e| e.to_string())??;
  reload_after_import(&state);
  Ok(())
}

/// Update action. On Flatpak: download the new `.flatpak` from the release and
/// reinstall it (no hosted/Flathub remote yet, so `flatpak update` can't pull
/// it). On native: open the release to grab the new packages.
#[tauri::command]
async fn run_update(url: String, version: String) -> String {
  if system::is_flatpak() {
    match tauri::async_runtime::spawn_blocking(move || system::flatpak_self_update(&version)).await {
      Ok(Ok(())) => "Updated — fully quit GP Client (tray → Quit) and reopen it to use the new version.".into(),
      Ok(Err(e)) => {
        system::open_url(&url);
        format!("Couldn't auto-update ({e}). Opened the release so you can download it manually.")
      }
      Err(_) => "The update task failed to run.".into(),
    }
  } else {
    system::open_url(&url);
    "Opened the latest release — download and install the new version.".into()
  }
}

/// Open (or focus) the separate Advanced settings window. An optional `section`
/// (e.g. "about") deep-links to that nav section — via a `goto-section` event
/// when the window is already open, or an init-script global on a cold open.
#[tauri::command]
fn open_settings(app: tauri::AppHandle, section: Option<String>) -> Result<(), String> {
  if let Some(w) = app.get_webview_window("settings") {
    let _ = w.set_focus();
    if let Some(sec) = section {
      let _ = w.emit("goto-section", sec);
    }
    return Ok(());
  }
  let mut builder =
    tauri::WebviewWindowBuilder::new(&app, "settings", tauri::WebviewUrl::App("settings.html".into()))
      .title("Advanced settings")
      .inner_size(560.0, 620.0)
      .min_inner_size(560.0, 620.0)
      .resizable(false)
      .decorations(false)
      .transparent(true);
  // Cold-open deep-link: stash the target section in a global before the page
  // loads (robust — no URL-fragment encoding or emit/listen race). The init
  // script only takes a fixed JSON string literal we control.
  if let Some(sec) = section {
    let js = format!("window.__gotoSection = {};", serde_json::to_string(&sec).unwrap_or_default());
    builder = builder.initialization_script(&js);
  }
  builder.build().map_err(|e| e.to_string())?;
  Ok(())
}

/// Persist the advanced options edited in the settings window.
#[tauri::command]
fn save_settings(app: tauri::AppHandle, state: State<AppState>, form: SettingsForm) {
  {
    let mut c = state.cfg.lock().unwrap();
    c.tray_icon = form.tray_icon;
    c.run_at_startup = form.run_at_startup;
    c.start_minimized = form.start_minimized;
    let remember_was = c.remember_unlock;
    c.remember_unlock = form.remember_unlock;
    c.save();
    autostart::set(c.run_at_startup, c.start_minimized);
    // Turning "remember unlock" off clears any stored PIN. Turning it on stores
    // the PIN at the next unlock (we don't hold the plaintext PIN here).
    if remember_was && !c.remember_unlock {
      secrets::clear_pin();
    }
  }
  // Repaint the tray immediately so a Shield/Ring change shows without waiting
  // for the next state transition.
  if let Some(t) = state.tray.lock().unwrap().as_ref() {
    let _ = t.update(|_| {});
  }
  // The main window rescans (module/cert) and uses these at connect time.
  let _ = app.emit("config-changed", ());
}

/// Whether the desktop secret store is reachable (for the "Unlock automatically"
/// opt-in — disabled when no keyring is present).
#[tauri::command]
fn keyring_available() -> bool {
  secrets::available()
}

/// Toggle "remember unlock". Set this to `true` **before** creating/unlocking the
/// vault so the PIN gets stored; turning it off clears any stored PIN.
#[tauri::command]
fn set_remember_unlock(state: State<AppState>, enabled: bool) {
  let mut c = state.cfg.lock().unwrap();
  let was = c.remember_unlock;
  c.remember_unlock = enabled;
  c.save();
  if was && !enabled {
    secrets::clear_pin();
  }
}

/// Answer a mid-connect MFA challenge with a one-time code — resolves the
/// prompt the backend parked while emitting `MfaChallenge` (GPS-16). The
/// push/tap-to-confirm variant is GPS-17.
#[tauri::command]
async fn submit_mfa(code: String) -> Result<(), String> {
  dbus_client::submit_mfa(code).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn resend_mfa() -> Result<(), String> {
  dbus_client::resend_mfa().await.map_err(|e| e.to_string())
}

/// Answer the mid-connect gateway picker with the chosen gateway's address —
/// resolves the prompt the backend parked while emitting `GatewaySelect`.
#[tauri::command]
async fn select_gateway(gateway: String) -> Result<(), String> {
  dbus_client::select_gateway(gateway).await.map_err(|e| e.to_string())
}

/// Best-effort latency probe for a gateway picker row: time a TCP handshake to
/// the gateway's TLS port. Runs on the blocking pool; returns milliseconds, or
/// an error string when the host is unreachable within the timeout.
#[tauri::command]
async fn ping_gateway(host: String) -> Result<u64, String> {
  tauri::async_runtime::spawn_blocking(move || {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::{Duration, Instant};
    let addr = format!("{host}:443")
      .to_socket_addrs()
      .map_err(|e| e.to_string())?
      .next()
      .ok_or_else(|| "no address".to_string())?;
    let t0 = Instant::now();
    TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| e.to_string())?;
    Ok(t0.elapsed().as_millis() as u64)
  })
  .await
  .map_err(|e| e.to_string())?
}

/// Answer a mid-connect smart-card prompt: the PIN plus the certificate chosen in
/// the connect-time picker (its `pkcs11:` URI, empty to keep the stored cert).
#[tauri::command]
fn submit_pin(state: State<AppState>, pin: String, cert_uri: Option<String>) -> Result<(), String> {
  // Deliver the PIN + chosen cert to the connect pipeline waiting on the prompt.
  match state.pin_tx.lock().unwrap().take() {
    Some(tx) => {
      let _ = tx.send(Some(vpn::PinReply { pin, cert_uri: cert_uri.unwrap_or_default() }));
      Ok(())
    }
    None => Err("no PIN prompt is active".into()),
  }
}

#[tauri::command]
fn connect(state: State<AppState>, identity: String, portal: String) -> Result<(), String> {
  start_connect(&state.vault, &state.cmd_tx, &identity, &portal)
}

/// Build a connect request from a saved identity and hand it to the VPN manager.
/// Shared by the `connect` command and the tray's "Connect with" submenu.
pub(crate) fn start_connect(
  vault: &Arc<Mutex<Vault>>,
  cmd_tx: &std::sync::mpsc::Sender<UiCommand>,
  identity: &str,
  portal: &str,
) -> Result<(), String> {
  let id = {
    let v = vault.lock().unwrap();
    if !v.unlocked {
      return Err("vault is locked".into());
    }
    v.identities()
      .iter()
      .find(|i| i.name == identity)
      .cloned()
      .ok_or_else(|| format!("no identity '{identity}'"))?
  };

  let server = if portal.trim().is_empty() {
    id.portal.clone()
  } else {
    portal.trim().to_string()
  };

  // os / user-agent / SSO method / CLI tuning are all per-identity now.
  let os = id.os.clone();
  let user_agent = id.user_agent.clone();
  let use_browser = id.auth_view == "browser";
  let opts = connect::ConnOpts {
    mtu: id.mtu,
    reconnect_timeout: id.reconnect_timeout,
    force_dpd: id.force_dpd,
    disable_ipv6: id.disable_ipv6,
    no_dtls: id.no_dtls,
    no_xmlpost: id.no_xmlpost,
    ignore_tls_errors: id.ignore_tls_errors,
    vpnc_script: id.vpnc_script.clone(),
    local_hostname: id.local_hostname.clone(),
    os_version: id.os_version.clone(),
    client_version: id.client_version.clone(),
  };

  let cert_kind = match id.auth_method {
    0 => 1,
    1 => 2,
    _ => 0,
  };
  let cert_uri = if id.cert_id.is_empty() {
    String::new()
  } else {
    format!("pkcs11:manufacturer={};id=%{};type=cert", id.cert_manufacturer, id.cert_id)
  };

  let params = ConnectParams {
    url: server,
    as_gateway: id.as_gateway,
    os,
    user_agent,
    module_path: id.module_path,
    cert_kind,
    use_browser,
    cert_uri,
    pin: id.pin,
    cert_file: id.cert_file,
    key_file: id.key_file,
    key_password: id.key_password,
    username: id.username,
    password: id.password,
    dns_domains: id.dns_domains,
    opts,
  };
  cmd_tx.send(UiCommand::Connect(params)).map_err(|e| e.to_string())
}

/// Auto-detect probe form.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProbeForm {
  portal: String,
  cert_kind: i32,
  cert_uri: String,
  pin: String,
  cert_file: String,
  key_file: String,
  key_password: String,
  module_path: String,
  #[serde(default = "default_true")]
  as_gateway: bool,
  /// Reported client identity for the probe — sent by the per-identity editor.
  #[serde(default)]
  os: String,
  #[serde(default)]
  user_agent: String,
}

fn default_true() -> bool {
  true
}

/// Probe the portal's prelogin to discover the required auth method.
#[tauri::command]
async fn probe_auth(state: State<'_, AppState>, form: ProbeForm) -> Result<connect::ProbeResult, String> {
  // Prefer the editor's in-form values (per-identity); fall back to the last
  // global values for older callers that don't send them.
  let (os, user_agent) = {
    let c = state.cfg.lock().unwrap();
    (
      if form.os.trim().is_empty() { c.os.clone() } else { form.os.clone() },
      if form.user_agent.trim().is_empty() { c.user_agent.clone() } else { form.user_agent.clone() },
    )
  };
  let (certificate, sslkey, key_password) = match form.cert_kind {
    1 => {
      if !form.module_path.is_empty() {
        // SAFETY: set before the probe; the GUI is single-connection.
        unsafe { std::env::set_var("GP_PKCS11_MODULE", &form.module_path) };
      }
      let cert = if form.pin.is_empty() {
        form.cert_uri.clone()
      } else {
        format!("{}?pin-value={}", form.cert_uri, form.pin)
      };
      (Some(cert), None, None)
    }
    2 => (
      Some(form.cert_file.clone()),
      (!form.key_file.is_empty()).then(|| form.key_file.clone()),
      (!form.key_password.is_empty()).then(|| form.key_password.clone()),
    ),
    _ => (None, None, None),
  };
  Ok(connect::probe(form.portal.trim(), &os, &user_agent, certificate, sslkey, key_password, false, form.as_gateway).await)
}

#[tauri::command]
fn vault_status(state: State<AppState>) -> serde_json::Value {
  let v = state.vault.lock().unwrap();
  serde_json::json!({ "exists": v.exists, "unlocked": v.unlocked })
}

/// Rebuild the tray menu (the ksni menu is static until told to update) so the
/// "Connect with" submenu reflects the current vault/identity state.
fn refresh_tray(tray: &Arc<Mutex<Option<Arc<tray::TrayHandle>>>>) {
  if let Some(t) = tray.lock().unwrap().as_ref() {
    let _ = t.update(|_| {});
  }
}

/// One-time migration: copy the formerly-global connection/SSO settings onto
/// every stored identity, then mark it done on `Config`. No-op once done, or
/// while the vault is locked (it retries after a real unlock). Runs after every
/// unlock/first-PIN path so an upgrading user keeps their tuning per identity.
fn migrate_conn_to_identity(vault: &Arc<Mutex<Vault>>, cfg: &Arc<Mutex<Config>>) {
  let snapshot = {
    let c = cfg.lock().unwrap();
    if c.conn_migrated_to_identity {
      return;
    }
    c.clone()
  };
  {
    let mut v = vault.lock().unwrap();
    if !v.unlocked {
      return; // retry after a real unlock; leave the flag unset for now
    }
    let _ = v.seed_all_connection_from_config(&snapshot);
  }
  let mut c = cfg.lock().unwrap();
  c.conn_migrated_to_identity = true;
  c.save();
}

#[tauri::command]
fn set_master_pin(state: State<AppState>, pin: String) -> Result<(), String> {
  state.vault.lock().unwrap().set_master_pin(&pin).map_err(|e| e.to_string())?;
  if state.cfg.lock().unwrap().remember_unlock {
    secrets::store_pin(&pin);
  }
  migrate_conn_to_identity(&state.vault, &state.cfg);
  refresh_tray(&state.tray);
  Ok(())
}

#[tauri::command]
fn unlock_vault(state: State<AppState>, pin: String) -> Result<(), String> {
  state.vault.lock().unwrap().unlock(&pin).map_err(|e| e.to_string())?;
  // Remember the PIN for next launch if opted in (best-effort).
  if state.cfg.lock().unwrap().remember_unlock {
    secrets::store_pin(&pin);
  }
  migrate_conn_to_identity(&state.vault, &state.cfg);
  refresh_tray(&state.tray);
  Ok(())
}

#[tauri::command]
fn lock_vault(state: State<AppState>) {
  state.vault.lock().unwrap().lock();
  refresh_tray(&state.tray);
}

/// Forgotten-PIN reset: delete the encrypted vault (losing all saved identities)
/// and any stored keyring PIN, returning to first-run setup.
#[tauri::command]
fn reset_vault(state: State<AppState>) {
  state.vault.lock().unwrap().reset();
  secrets::clear_pin();
  refresh_tray(&state.tray);
}

#[tauri::command]
fn list_identities(state: State<AppState>) -> Result<Vec<Identity>, String> {
  let v = state.vault.lock().unwrap();
  if !v.unlocked {
    return Err("vault is locked".into());
  }
  Ok(v.identities().to_vec())
}

#[tauri::command]
fn save_identity(app: tauri::AppHandle, state: State<AppState>, identity: Identity) -> Result<(), String> {
  state.vault.lock().unwrap().upsert(identity).map_err(|e| e.to_string())?;
  let _ = app.emit("identities-changed", ());
  refresh_tray(&state.tray);
  Ok(())
}

#[tauri::command]
fn delete_identity(app: tauri::AppHandle, state: State<AppState>, name: String) -> Result<(), String> {
  state.vault.lock().unwrap().remove(&name).map_err(|e| e.to_string())?;
  let _ = app.emit("identities-changed", ());
  refresh_tray(&state.tray);
  Ok(())
}

/// Open (or focus) the single-identity editor window, targeting `name` (an
/// existing identity) or a new one when omitted. Shares the `manager` window;
/// the target is stashed before load (cold open) or pushed via an event (open).
#[tauri::command]
fn open_identity_editor(app: tauri::AppHandle, name: Option<String>) -> Result<(), String> {
  let target = name.unwrap_or_default();
  if let Some(w) = app.get_webview_window("manager") {
    let _ = w.emit("edit-identity", target);
    let _ = w.set_focus();
    return Ok(());
  }
  let js = format!("window.__editIdentity = {};", serde_json::to_string(&target).unwrap_or_default());
  tauri::WebviewWindowBuilder::new(&app, "manager", tauri::WebviewUrl::App("manager.html".into()))
    .title("Identity")
    .inner_size(600.0, 640.0)
    .min_inner_size(600.0, 640.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .initialization_script(&js)
    .build()
    .map_err(|e| e.to_string())?;
  Ok(())
}

fn main() {
  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("gp_client=info")),
    )
    .init();

  tracing::info!(
    "GP Client {} starting ({})",
    system::GUI_VERSION,
    if system::is_flatpak() { "flatpak" } else { "native" }
  );

  // Single-instance guard — the very first thing, before any GTK/Tauri init. If
  // another instance is already running this signals it to reveal its window and
  // exits; otherwise we hold the listener and service "show" pings in `setup`.
  // Doing this pre-init is what makes it work in the Flatpak sandbox (where the
  // D-Bus-based plugin didn't) and prevents the relaunch-crash entirely.
  let instance_listener = single_instance::acquire_or_signal();

  // Migration from the predecessor gpgui is now user-driven: the frontend shows
  // the "Import from GP Client" screen when `import_available()` is true and calls
  // `import_from_gpgui` on confirm (see import.rs).

  let cfg = Arc::new(Mutex::new(Config::load()));
  // Keep the autostart entry in sync with the preferences (which default on). On
  // a fresh install this seeds it on first launch; the in-app toggle is the
  // source of truth and updates it via `save_settings`.
  {
    let c = cfg.lock().unwrap();
    autostart::set(c.run_at_startup, c.start_minimized);
  }
  // Register a float exception in any tiling shell present (Pop Shell, …) so the
  // window opens floating instead of tiled.
  tiling::ensure_float_exceptions();
  let shared = Arc::new(Mutex::new(Shared::default()));
  let vault_path = config::vault_path().unwrap_or_else(|| std::path::PathBuf::from("identities.enc"));
  let vault = Arc::new(Mutex::new(Vault::load(vault_path)));
  // Auto-unlock from the desktop secret store if the user opted in. Best-effort:
  // a missing/locked/corrupt keyring or a stale PIN just leaves the vault locked
  // and the user is prompted as usual.
  if cfg.lock().unwrap().remember_unlock {
    if let Some(pin) = secrets::load_pin() {
      let _ = vault.lock().unwrap().unlock(&pin);
    }
  }
  // Seed per-identity connection settings from the old globals on first run
  // after upgrading (no-op if already migrated or still locked).
  migrate_conn_to_identity(&vault, &cfg);
  let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<UiCommand>();
  let pin_slot: vpn::PinSlot = Arc::new(Mutex::new(None));

  let tray_available = Arc::new(AtomicBool::new(false));
  let tray_slot: Arc<Mutex<Option<Arc<tray::TrayHandle>>>> = Arc::new(Mutex::new(None));
  // Connecting-animation frame counter, advanced by the animator thread.
  let frame = Arc::new(AtomicUsize::new(0));

  let app_state = AppState {
    cmd_tx: cmd_tx.clone(),
    pin_tx: pin_slot.clone(),
    shared: shared.clone(),
    cfg: cfg.clone(),
    vault: vault.clone(),
    tray_available: tray_available.clone(),
    tray: tray_slot.clone(),
  };

  // Moved into setup (which owns the receiver and the AppHandle).
  let setup_shared = shared.clone();
  let setup_cfg = cfg.clone();
  let setup_vault = vault.clone();
  let setup_cmd_tx = cmd_tx.clone();
  let setup_tray_available = tray_available.clone();
  let setup_tray_slot = tray_slot.clone();
  let setup_frame = frame.clone();
  let setup_pin_slot = pin_slot.clone();
  let cmd_rx = Mutex::new(Some(cmd_rx));

  // Moved into setup so the accept loop can hold the AppHandle.
  let instance_listener = Mutex::new(instance_listener);

  tauri::Builder::default()
    .manage(app_state)
    .on_window_event(|window, event| {
      // Close-to-tray: the X button hides the main window so the app keeps
      // running (tunnel + notifications + tray) in the background. Falls back to
      // a real close when no tray is available, so the user is never stranded.
      // Secondary windows (settings/manager) close normally.
      if window.label() == "main" {
        if let WindowEvent::CloseRequested { api, .. } = event {
          if window.state::<AppState>().tray_available.load(Ordering::Relaxed) {
            let _ = window.hide();
            api.prevent_close();
          }
        }
      }
    })
    .invoke_handler(tauri::generate_handler![
      get_config,
      get_state,
      available_modules,
      scan_certs,
      browse_file,
      connect,
      disconnect,
      open_url,
      system_info,
      install_backend,
      keyring_available,
      set_remember_unlock,
      check_update,
      run_update,
      restart_app,
      reboot_host,
      predecessor_removable,
      remove_predecessor,
      import_available,
      predecessor_app_installed,
      import_from_gpgui,
      open_settings,
      save_settings,
      probe_auth,
      vault_status,
      set_master_pin,
      unlock_vault,
      lock_vault,
      reset_vault,
      list_identities,
      save_identity,
      delete_identity,
      open_identity_editor,
      submit_mfa,
      resend_mfa,
      select_gateway,
      ping_gateway,
      submit_pin
    ])
    .setup(move |app| {
      let handle = app.handle().clone();

      // Serve single-instance "show" pings: a relaunch while we're running (incl.
      // hidden in the tray) connects to our abstract socket; reveal the window.
      if let Some(listener) = instance_listener.lock().unwrap().take() {
        let show_handle = handle.clone();
        std::thread::spawn(move || {
          single_instance::serve(listener, move |signal| match signal {
            single_instance::Signal::Show => tray::reveal_window(&show_handle),
            // A browser-SSO redirect came back on a relaunch: hand the URL to the
            // waiting sign-in flow and bring the app forward.
            single_instance::Signal::Callback(url) => {
              saml::deliver_callback(url);
              tray::reveal_window(&show_handle);
            }
          });
        });
      }

      // Auto-tiling window managers tile every normal toplevel, including this
      // fixed-size one. On X11 (Pop Shell on Xorg, i3, bspwm) marking it a dialog
      // makes them float it. We do NOT do this on Wayland: it gives no floating
      // benefit there (tilers float via an app-id rule — see `tiling.rs`), and
      // Mutter treats dialogs differently, which drops the window's rounded
      // corners. Wayland keeps the normal type so the shell still rounds it.
      #[cfg(target_os = "linux")]
      if std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("x11") {
        if let Some(w) = app.get_webview_window("main") {
          use gtk::prelude::GtkWindowExt;
          if let Ok(gw) = w.gtk_window() {
            gw.set_type_hint(gtk::gdk::WindowTypeHint::Dialog);
          }
        }
      }

      // Tray (optional — needs a StatusNotifierItem host: native on KDE/COSMIC,
      // the AppIndicator extension on GNOME).
      let tray = GpTray {
        shared: setup_shared.clone(),
        cfg: setup_cfg.clone(),
        vault: setup_vault.clone(),
        cmd_tx: setup_cmd_tx.clone(),
        app: handle.clone(),
        frame: setup_frame.clone(),
      };
      // In a Flatpak sandbox we can't own the `org.kde.StatusNotifierItem-PID-ID`
      // well-known name, so register the tray under our unique bus name instead
      // (ksni's documented sandbox path). Native keeps the well-known name.
      let spawned = if system::is_flatpak() {
        ksni::blocking::TrayMethods::disable_dbus_name(tray, true).spawn()
      } else {
        ksni::blocking::TrayMethods::spawn(tray)
      };
      let tray_handle = match spawned {
        Ok(h) => {
          setup_tray_available.store(true, Ordering::Relaxed);
          Some(Arc::new(h))
        }
        Err(e) => {
          tracing::warn!("tray unavailable (install/enable AppIndicator on GNOME): {e}");
          None
        }
      };

      if let Some(t) = &tray_handle {
        // Expose the handle so a settings change can repaint the tray.
        *setup_tray_slot.lock().unwrap() = Some(t.clone());

        // Animator: while connecting/disconnecting, advance the frame and
        // repaint (~12.5fps). SNI hosts don't play GIFs, so we swap frames.
        let t = t.clone();
        let shared = setup_shared.clone();
        let frame = setup_frame.clone();
        std::thread::spawn(move || {
          let mut was_connecting = false;
          loop {
            std::thread::sleep(Duration::from_millis(80));
            let connecting = matches!(
              shared.lock().unwrap().status,
              Status::Connecting | Status::Reconnecting | Status::Disconnecting
            );
            if connecting {
              frame.fetch_add(1, Ordering::Relaxed);
              let _ = t.update(|_| {});
              was_connecting = true;
            } else if was_connecting {
              was_connecting = false;
              // Just left the connecting state. The SNI host may have throttled
              // icon fetches during the animation and missed the status-change
              // repaint (ksni dedups identical icons), leaving a spinner frame on
              // screen. Force a few spaced re-emits with a changing hash (frame
              // parity flips the static icon's size order) so the host re-fetches
              // the final, static icon once things have quieted down.
              for _ in 0..3 {
                frame.fetch_add(1, Ordering::Relaxed);
                let _ = t.update(|_| {});
                std::thread::sleep(Duration::from_millis(160));
              }
            }
          }
        });

        // Start hidden to the tray when launched with `--hidden` (login
        // autostart) or when the "Start minimized" preference is set.
        let start_hidden = std::env::args().any(|a| a == "--hidden")
          || setup_cfg.lock().unwrap().start_minimized;
        if start_hidden {
          if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
          }
        }
      }

      // UI-agnostic change hook → emit the current state to the webview.
      let on_change: Arc<dyn Fn() + Send + Sync> = {
        let handle = handle.clone();
        let shared = setup_shared.clone();
        Arc::new(move || {
          let _ = handle.emit("state", build_state(&shared));
        })
      };

      let notifier = Notifier::new(setup_shared.clone(), tray_handle, on_change);
      let rx = cmd_rx.lock().unwrap().take().expect("setup runs once");
      std::thread::spawn(move || vpn::run(rx, notifier, handle, setup_pin_slot));

      // Background: notify once on launch if a newer release is out. This covers
      // the start-hidden case, where the in-window update banner isn't visible.
      tauri::async_runtime::spawn(async {
        if let Ok(rel) = system::latest_gui_release().await {
          if system::version_cmp(&rel.version, system::GUI_VERSION) == std::cmp::Ordering::Greater {
            vpn::notify_desktop(
              "GP Client update available".into(),
              format!("Version {} is available — open Settings → About to update.", rel.version),
            );
          }
        }
      });

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running gpgui");
}
