//! D-Bus client for the gpservice system service — the Flatpak transport.

use anyhow::Result;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use gp_protocol::VpnState;

#[zbus::proxy(
  interface = "io.github.techneut92.GPService1",
  default_service = "io.github.techneut92.GPService",
  default_path = "/io/github/techneut92/GPService"
)]
trait GpService {
  async fn connect(&self, request: String) -> zbus::Result<()>;
  async fn disconnect(&self) -> zbus::Result<()>;
  async fn status(&self) -> zbus::Result<String>;
  /// v3 handoff: run prelogin, return a JSON `ProbeReply`.
  async fn probe(&self, request: String) -> zbus::Result<String>;
  /// v3 handoff: authenticate with a captured credential and start the tunnel.
  async fn connect_auth(&self, request: String) -> zbus::Result<()>;
  /// v4: answer an interactive MFA/token challenge with the one-time code.
  async fn submit_mfa(&self, code: String) -> zbus::Result<()>;
  /// v4: re-request the MFA challenge.
  async fn resend_mfa(&self) -> zbus::Result<()>;

  #[zbus(signal)]
  fn vpn_state_changed(&self, state: String) -> zbus::Result<()>;
}

pub struct DbusHandle {
  conn: zbus::Connection,
}

impl DbusHandle {
  async fn proxy(&self) -> Result<GpServiceProxy<'_>> {
    Ok(GpServiceProxy::new(&self.conn).await?)
  }

  pub async fn send_disconnect(&self) -> Result<()> {
    self.proxy().await?.disconnect().await?;
    Ok(())
  }

  /// v3 handoff: probe a gateway and return the raw `ProbeReply` JSON.
  pub async fn probe(&self, request: String) -> Result<String> {
    Ok(self.proxy().await?.probe(request).await?)
  }

  /// v3 handoff: authenticate with a captured credential and start the tunnel.
  pub async fn send_connect_auth(&self, request: String) -> Result<()> {
    self.proxy().await?.connect_auth(request).await?;
    Ok(())
  }
}

async fn connect_bus() -> Result<zbus::Connection> {
  Ok(if std::env::var("GP_DBUS_SESSION").is_ok() {
    zbus::Connection::session().await?
  } else {
    zbus::Connection::system().await?
  })
}

/// Answer the backend's interactive MFA challenge with the one-time code. A
/// short-lived call (the connect pipeline is parked on the backend awaiting it),
/// so it doesn't go through the VPN manager.
pub async fn submit_mfa(code: String) -> Result<()> {
  let conn = connect_bus().await?;
  GpServiceProxy::new(&conn).await?.submit_mfa(code).await?;
  Ok(())
}

/// Ask the backend to re-request the MFA challenge.
pub async fn resend_mfa() -> Result<()> {
  let conn = connect_bus().await?;
  GpServiceProxy::new(&conn).await?.resend_mfa().await?;
  Ok(())
}

/// Connect to gpservice over D-Bus and stream `VpnState` changes. Uses the
/// session bus when `GP_DBUS_SESSION` is set (dev), otherwise the system bus.
pub async fn open() -> Result<(DbusHandle, mpsc::Receiver<VpnState>)> {
  let conn = if std::env::var("GP_DBUS_SESSION").is_ok() {
    zbus::Connection::session().await?
  } else {
    zbus::Connection::system().await?
  };

  let proxy = GpServiceProxy::new(&conn).await?;
  let mut signals = proxy.receive_vpn_state_changed().await?;

  let (tx, rx) = mpsc::channel::<VpnState>(32);
  tokio::spawn(async move {
    while let Some(sig) = signals.next().await {
      let Ok(args) = sig.args() else { continue };
      if let Ok(state) = serde_json::from_str::<VpnState>(&args.state) {
        if tx.send(state).await.is_err() {
          break;
        }
      }
    }
  });

  Ok((DbusHandle { conn }, rx))
}
