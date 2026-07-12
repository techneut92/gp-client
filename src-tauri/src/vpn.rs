//! VPN connection manager (v2 — gpservice architecture).
//!
//! Runs on a dedicated thread with its own multi-threaded tokio runtime.
//! On `Connect` it: (1) authenticates **unprivileged** in-process (prelogin +
//! SAML via gpauth, which now has this user's display), (2) ensures the root
//! `gpservice` is running, (3) sends the `ConnectRequest` over the encrypted
//! WebSocket, and (4) streams `VpnState` events back into the UI/tray. The
//! tunnel itself runs inside gpservice — we never shell out openconnect here.

use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::connect::{build_connect_request, AuthParams, ConnOpts};
use crate::state::{ConnDetails, Shared, Status};
use crate::transport::{self, Transport};
use crate::tray::TrayHandle;
use gp_protocol::{ConnectedInfo, VpnState};

/// Parameters captured from the UI at connect time. `pin` lives only here and
/// in the spawned processes — it is never persisted.
pub struct ConnectParams {
  pub url: String,
  pub as_gateway: bool,
  pub os: String,
  pub user_agent: String,
  pub module_path: String,
  /// Client certificate kind: 0 = none, 1 = PKCS#11 smart card, 2 = file.
  pub cert_kind: i32,
  /// SSO in the system browser instead of the embedded webview.
  pub use_browser: bool,
  /// Cert selector URI without PIN, e.g. `pkcs11:manufacturer=piv_II;id=%03;type=cert`.
  pub cert_uri: String,
  pub pin: String,
  /// PEM/PKCS#12 certificate file (auth_method == 1).
  pub cert_file: String,
  pub key_file: String,
  pub key_password: String,
  /// Standard username/password (auth_method == 3).
  pub username: String,
  pub password: String,
  /// Advanced connection options (settings window).
  pub opts: ConnOpts,
}

pub enum UiCommand {
  Connect(ConnectParams),
  Disconnect,
}

/// Pushes state changes to the shared store, the tray, and the UI.
/// `on_change` is a UI-agnostic hook (Slint pushes shared state into its event loop).
#[derive(Clone)]
pub struct Notifier {
  shared: Arc<Mutex<Shared>>,
  /// `None` when no StatusNotifierWatcher is available (e.g. GNOME without the
  /// AppIndicator extension) — the window still works.
  tray: Option<Arc<TrayHandle>>,
  on_change: Arc<dyn Fn() + Send + Sync>,
}

impl Notifier {
  pub fn new(
    shared: Arc<Mutex<Shared>>,
    tray: Option<Arc<TrayHandle>>,
    on_change: Arc<dyn Fn() + Send + Sync>,
  ) -> Self {
    Self { shared, tray, on_change }
  }

  /// Begin a new connection generation; returns its id.
  fn bump(&self) -> u64 {
    let mut s = self.shared.lock().unwrap();
    s.current_gen += 1;
    s.current_gen
  }

  /// Apply a status, but only if `generation` is still the current one. Fires a
  /// desktop notification on the connect / disconnect / error transitions.
  fn set_status(&self, generation: u64, status: Status) {
    let mut notify: Option<(String, String)> = None;
    {
      let mut s = self.shared.lock().unwrap();
      if s.current_gen != generation {
        return;
      }
      let was_connected = matches!(s.status, Status::Connected);
      let was_active = s.status.is_active();

      // Clear the transient progress line once we reach any terminal state
      // (connected, disconnected, or error) so a stale "Authenticating…" line
      // never lingers — the status itself then conveys the outcome.
      if matches!(status, Status::Connected | Status::Disconnected | Status::Error(_)) {
        s.log.clear();
      }

      if !was_connected && matches!(status, Status::Connected) {
        let portal = if s.conn.portal.is_empty() {
          "GP Client".to_string()
        } else {
          s.conn.portal.clone()
        };
        notify = Some(("GP Client connected".into(), format!("Connected to {portal}")));
      } else if was_active && matches!(status, Status::Disconnected) {
        notify = Some(("GP Client disconnected".into(), "The VPN connection has ended".into()));
      } else if let Status::Error(e) = &status {
        if was_active {
          notify = Some(("GP Client error".into(), e.clone()));
        }
      }

      s.status = status;
    }
    if let Some((summary, body)) = notify {
      notify_desktop(summary, body);
    }
    self.refresh();
  }

  fn log(&self, line: &str) {
    self.shared.lock().unwrap().log = line.to_string();
    self.refresh();
  }

  /// Store live connection details (generation-guarded).
  fn set_conn(&self, generation: u64, conn: ConnDetails) {
    {
      let mut s = self.shared.lock().unwrap();
      if s.current_gen != generation {
        return;
      }
      s.conn = conn;
    }
    self.refresh();
  }

  fn refresh(&self) {
    if let Some(tray) = &self.tray {
      let _ = tray.update(|_| {});
    }
    (self.on_change)();
  }
}

pub fn run(rx: Receiver<UiCommand>, notifier: Notifier, app_handle: tauri::AppHandle) {
  let rt = match tokio::runtime::Runtime::new() {
    Ok(rt) => rt,
    Err(e) => {
      notifier.set_status(notifier.bump(), Status::Error(format!("tokio runtime: {e}")));
      return;
    }
  };

  // The live transport to gpservice, kept across commands so Disconnect can
  // reach the same connection. Background tasks (the read loop, the event
  // monitor) run on the multi-threaded runtime's worker threads.
  let mut handle: Option<Transport> = None;

  for cmd in rx {
    match cmd {
      UiCommand::Connect(params) => {
        let generation = notifier.bump();
        notifier.set_status(generation, Status::Connecting);

        // Tear down any previous connection first.
        if let Some(h) = handle.take() {
          rt.block_on(async { let _ = h.send_disconnect().await; });
        }

        match rt.block_on(connect(&params, &notifier, generation, &app_handle)) {
          Ok(h) => handle = Some(h),
          // `{:#}` includes the full anyhow context chain (e.g. "single sign-on
          // was cancelled or failed: …") so the user sees the real reason.
          Err(e) => notifier.set_status(generation, Status::Error(format!("{e:#}"))),
        }
      }
      UiCommand::Disconnect => {
        let generation = notifier.bump();
        notifier.set_status(generation, Status::Disconnecting);
        if let Some(h) = handle.take() {
          rt.block_on(async { let _ = h.send_disconnect().await; });
        }
        notifier.set_status(generation, Status::Disconnected);
      }
    }
  }
}

/// The full v2 connect pipeline. Returns the live transport on success.
async fn connect(p: &ConnectParams, notifier: &Notifier, generation: u64, app_handle: &tauri::AppHandle) -> Result<Transport> {
  if !p.as_gateway {
    bail!("Only 'connect directly as gateway' is supported in this build");
  }

  // Resolve the client certificate (cert axis) independently of the credential
  // (SAML vs password). The credential is decided downstream: username/password
  // present → standard login, otherwise SAML SSO.
  let opt = |s: &String| (!s.is_empty()).then(|| s.clone());
  let (certificate, sslkey, key_password) = match p.cert_kind {
    1 => {
      if p.cert_uri.is_empty() {
        bail!("Select a smart-card certificate");
      }
      // Honour the chosen PKCS#11 module for the GUI-side prelogin.
      if !p.module_path.is_empty() {
        // SAFETY: set before the auth runs; the GUI is single-connection.
        unsafe { std::env::set_var("GP_PKCS11_MODULE", &p.module_path) };
      }
      let cert = if p.pin.is_empty() {
        p.cert_uri.clone()
      } else {
        format!("{}?pin-value={}", p.cert_uri, p.pin)
      };
      (cert, None, None)
    }
    2 => {
      if p.cert_file.is_empty() {
        bail!("Select a certificate file");
      }
      (p.cert_file.clone(), opt(&p.key_file), opt(&p.key_password))
    }
    _ => (String::new(), None, None),
  };
  let username = opt(&p.username);
  let password = opt(&p.password);

  let auth = AuthParams {
    server: p.url.clone(),
    os: p.os.clone(),
    user_agent: p.user_agent.clone(),
    certificate,
    sslkey,
    key_password,
    username,
    password,
    use_browser: p.use_browser,
    opts: p.opts.clone(),
  };

  notifier.log("Authenticating (prelogin + SSO)…");
  let request = build_connect_request(&auth, app_handle).await?;

  // Shared loopback secret (used only by the loopback transport).
  let key = crate::config::load_or_create_api_key();

  notifier.log("Connecting to gpservice…");
  let (transport, mut events) = transport::open(&key).await?;

  transport
    .send_connect(request)
    .await
    .context("sending Connect to gpservice")?;
  notifier.log("Connect request sent; bringing up tunnel…");

  // Stream VpnState back into the UI until the transport closes.
  let n = notifier.clone();
  tokio::spawn(async move {
    while let Some(state) = events.recv().await {
      match state {
        VpnState::Connected(info) => {
          let details = parse_conn_details(&info);
          // gpservice now reports the iface/IP; only scan as a fallback.
          let need_scan = details.iface.is_empty();
          let expires_at = details.expires_at;
          n.set_conn(generation, details);
          n.set_status(generation, Status::Connected);
          if need_scan {
            spawn_tunnel_lookup(n.clone(), generation);
          }
          if let Some(at) = expires_at {
            spawn_session_timer(n.clone(), generation, at);
          }
        }
        VpnState::Connecting(_) => n.set_status(generation, Status::Connecting),
        // Keep the connection details — the session survives the reconnect.
        VpnState::Reconnecting(_) => n.set_status(generation, Status::Reconnecting),
        VpnState::Disconnecting => n.set_status(generation, Status::Disconnecting),
        VpnState::Disconnected => {
          n.set_conn(generation, Default::default());
          n.set_status(generation, Status::Disconnected);
        }
      }
    }

    // The event stream ended — gpservice closed the connection or died. If this
    // is still the current connection (a user disconnect bumps the generation),
    // treat it as a dropped connection.
    n.set_conn(generation, Default::default());
    n.set_status(generation, Status::Disconnected);
  });

  Ok(transport)
}

/// Pull display details out of the `ConnectedInfo` JSON (`{info, sessionInfo}`).
fn parse_conn_details(info: &ConnectedInfo) -> ConnDetails {
  let ci = info.info();
  let portal = ci.portal().to_string();
  let gw = ci.gateway();
  let (gw_name, gw_addr) = (gw.name(), gw.server());
  let gateway = if gw_name.is_empty() || gw_name == gw_addr {
    gw_addr.to_string()
  } else {
    format!("{gw_name} ({gw_addr})")
  };

  // Prefer an absolute expiry epoch (for a live countdown); fall back to
  // now + lifetime, then to the static human string.
  let now = unix_now();
  let (expires_at, expires) = match info.session_info() {
    Some(s) => {
      let expires_at = s
        .user_expires
        .map(|e| e as u64)
        .filter(|&e| e > now)
        .or_else(|| s.lifetime_secs.map(|l| now + l as u64));
      let expires = match expires_at {
        Some(at) => humanize_expiry(at),
        None => s.expires_in_human.clone().unwrap_or_default(),
      };
      (expires_at, expires)
    }
    None => (None, String::new()),
  };

  // Tunnel facts reported by gpservice (added to ConnectedInfo).
  let iface = info.tun_iface().unwrap_or_default().to_string();
  let ip = info.ipv4().or(info.ipv6()).unwrap_or_default().to_string();

  ConnDetails { portal, gateway, expires, expires_at, ip, iface }
}

fn unix_now() -> u64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0)
}

/// "expires in 11h 59m" / "expires in 4m 12s" / "expired".
fn humanize_expiry(expires_at: u64) -> String {
  let now = unix_now();
  if expires_at <= now {
    return "expired".into();
  }
  let mut secs = expires_at - now;
  let h = secs / 3600;
  secs %= 3600;
  let m = secs / 60;
  let s = secs % 60;
  let body = if h > 0 {
    format!("{h}h {m}m")
  } else if m > 0 {
    format!("{m}m {s}s")
  } else {
    format!("{s}s")
  };
  format!("expires in {body}")
}

/// Tick the session countdown once a second while connected.
fn spawn_session_timer(n: Notifier, generation: u64, expires_at: u64) {
  tokio::spawn(async move {
    loop {
      {
        let mut s = n.shared.lock().unwrap();
        if s.current_gen != generation || !matches!(s.status, Status::Connected) {
          return;
        }
        s.conn.expires = humanize_expiry(expires_at);
      }
      n.refresh();
      tokio::time::sleep(Duration::from_secs(1)).await;
    }
  });
}

/// Fire a desktop notification (off-thread; D-Bus call shouldn't block callers).
pub(crate) fn notify_desktop(summary: String, body: String) {
  std::thread::spawn(move || {
    let _ = notify_rust::Notification::new()
      .summary(&summary)
      .body(&body)
      .icon("network-vpn")
      .appname("GP Client")
      .show();
  });
}

/// The tunnel IP/iface isn't in the protocol — poll for a tun device for a few
/// seconds after connect and fill it in.
fn spawn_tunnel_lookup(n: Notifier, generation: u64) {
  tokio::spawn(async move {
    for _ in 0..12 {
      if let Some((iface, ip)) = tunnel_addr() {
        let mut s = n.shared.lock().unwrap();
        if s.current_gen != generation || !matches!(s.status, Status::Connected) {
          return;
        }
        s.conn.iface = iface;
        s.conn.ip = ip;
        drop(s);
        n.refresh();
        return;
      }
      tokio::time::sleep(Duration::from_millis(500)).await;
    }
  });
}

/// Best-effort: find a VPN tun interface and its IPv4 via `ip -o -4 addr`.
fn tunnel_addr() -> Option<(String, String)> {
  let out = std::process::Command::new("ip")
    .args(["-o", "-4", "addr", "show"])
    .output()
    .ok()?;
  let text = String::from_utf8_lossy(&out.stdout);
  for line in text.lines() {
    // e.g. "12: tun0    inet 10.1.2.3/32 scope global tun0\..."
    let iface = line.split_whitespace().nth(1)?;
    if iface.starts_with("tun") || iface.starts_with("gpd") || iface.starts_with("vpn") {
      let mut it = line.split_whitespace();
      while let Some(tok) = it.next() {
        if tok == "inet" {
          if let Some(addr) = it.next() {
            let ip = addr.split('/').next().unwrap_or(addr).to_string();
            return Some((iface.to_string(), ip));
          }
        }
      }
    }
  }
  None
}
