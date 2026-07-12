//! Service transport — how the GUI reaches gpservice. Two impls behind one type,
//! chosen at runtime; everything above it (auth, UI, the state monitor) is
//! transport-agnostic.
//!
//!  - `Loopback`: pkexec-launch gpservice + encrypted loopback WebSocket (native
//!    .deb/.rpm/.apk install).
//!  - `Dbus`: a host D-Bus *system* service (Flatpak sandbox — no pkexec, no
//!    `/var/run`). Selected when running inside a Flatpak (`/.flatpak-info`) or
//!    via `GP_TRANSPORT=dbus`.

use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::Engine;
use tokio::sync::mpsc;

use crate::client::{self, Handle};
use crate::dbus_client::{self, DbusHandle};
use gp_protocol::{ConnectRequest, DisconnectRequest, VpnState, WsEvent, WsRequest};

pub enum Transport {
  Loopback(Handle),
  Dbus(DbusHandle),
}

impl Transport {
  pub async fn send_connect(&self, request: ConnectRequest) -> Result<()> {
    match self {
      Transport::Loopback(h) => h.send(WsRequest::Connect(Box::new(request))).await,
      Transport::Dbus(h) => h.send_connect(serde_json::to_string(&request)?).await,
    }
  }

  pub async fn send_disconnect(&self) -> Result<()> {
    match self {
      Transport::Loopback(h) => h.send(WsRequest::Disconnect(DisconnectRequest)).await,
      Transport::Dbus(h) => h.send_disconnect().await,
    }
  }
}

fn use_dbus() -> bool {
  match std::env::var("GP_TRANSPORT") {
    Ok(v) => v.eq_ignore_ascii_case("dbus"),
    Err(_) => std::path::Path::new("/.flatpak-info").exists(),
  }
}

/// Open a transport and return a unified stream of `VpnState` changes.
pub async fn open(key: &[u8]) -> Result<(Transport, mpsc::Receiver<VpnState>)> {
  if use_dbus() {
    let (h, rx) = dbus_client::open().await.context("connecting to gpservice over D-Bus")?;
    return Ok((Transport::Dbus(h), rx));
  }

  let port = ensure_service(key).await.context("could not start gpservice")?;
  let (handle, mut events) = client::connect(port, key.to_vec()).await?;

  // gpservice pushes an encrypted VpnEnv right after the handshake. If we can't
  // decrypt it, a stale/foreign gpservice is running with a different key — fail
  // with an actionable message instead of silently dropping on the first send.
  match tokio::time::timeout(Duration::from_secs(5), events.recv()).await {
    Ok(Some(ev)) => {
      // Protocol handshake: the first event is VpnEnv, which advertises the
      // backend's MIN..=MAX protocol range. Refuse only if it doesn't overlap
      // ours; otherwise the highest common version is used (both speak v1 today).
      // The direction tells the user which side is too old.
      if let WsEvent::VpnEnv(env) = &ev {
        let (g_min, g_max) = (gp_protocol::PROTOCOL_MIN, gp_protocol::PROTOCOL_MAX);
        let (b_min, b_max) = (env.protocol_min, env.protocol_max);
        if g_min.max(b_min) > g_max.min(b_max) {
          // No overlap. Point the user at the right move for *the client*: if the
          // GUI is behind the backend, upgrade it; if it's ahead, the backend
          // needs updating (or the client can be downgraded to match).
          let advice = if g_max < b_min {
            "GP Client is older than the backend — update GP Client (Settings → About)"
          } else {
            "GP Client is newer than the backend — update the backend, or downgrade GP Client to match"
          };
          bail!(
            "incompatible wire protocol: GP Client speaks v{g_min}..={g_max}, \
             the backend speaks v{b_min}..={b_max} — {advice}"
          );
        }
      }
    }
    Ok(None) => bail!("gpservice closed the connection immediately"),
    Err(_) => bail!(
      "couldn't authenticate to gpservice — another instance is likely running with a different key. \
       Run `sudo pkill gpservice` and reconnect."
    ),
  }

  // Adapt the WsEvent stream to a plain VpnState stream (dropping VpnEnv etc.).
  let (tx, rx) = mpsc::channel::<VpnState>(32);
  tokio::spawn(async move {
    while let Some(ev) = events.recv().await {
      if let WsEvent::VpnState(state) = ev {
        if tx.send(state).await.is_err() {
          break;
        }
      }
    }
    // dropping tx closes rx → the monitor treats it as a disconnect
  });

  Ok((Transport::Loopback(handle), rx))
}

/// Ensure the root gpservice is running and return its loopback port; launch it
/// via pkexec (passwordless with the shipped polkit rule), piping the shared key.
async fn ensure_service(key: &[u8]) -> Result<u16> {
  if let Ok(port) = client::read_port().await {
    if is_listening(port).await {
      return Ok(port);
    }
  }

  let bin = crate::config::gpservice_binary();
  let b64 = base64::engine::general_purpose::STANDARD.encode(key);

  let mut child = std::process::Command::new("pkexec")
    .arg(&bin)
    .arg("--api-key-on-stdin")
    .stdin(Stdio::piped())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .context("spawning pkexec gpservice")?;

  if let Some(mut stdin) = child.stdin.take() {
    use std::io::Write;
    let _ = writeln!(stdin, "{b64}");
  }

  // Wait for gpservice to come up. The cap must NOT count the time the pkexec
  // password dialog is open (it can sit there for as long as the user takes),
  // so the real readiness signal is the pkexec child: while it's alive we keep
  // waiting; if it exits before the port is listening, polkit denied/cancelled
  // the auth or gpservice crashed — bail with that instead of timing out.
  // A generous absolute backstop only guards a child that's alive but wedged.
  for _ in 0..3000 {
    if let Ok(port) = client::read_port().await {
      if is_listening(port).await {
        return Ok(port);
      }
    }
    match child.try_wait() {
      // pkexec is still running: either awaiting the password, or gpservice is
      // up and starting. Keep waiting.
      Ok(None) => {}
      // pkexec exited without the port ever opening.
      Ok(Some(status)) if status.success() => {
        bail!("gpservice exited during startup before it was ready")
      }
      Ok(Some(status)) => {
        bail!("could not start gpservice (polkit denied/cancelled, or it crashed): pkexec exited with {status}")
      }
      Err(err) => bail!("failed to poll pkexec gpservice: {err}"),
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
  }
  bail!("gpservice did not become ready within 5 minutes")
}

async fn is_listening(port: u16) -> bool {
  tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok()
}
