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

  pub async fn send_connect(&self, request: String) -> Result<()> {
    self.proxy().await?.connect(request).await?;
    Ok(())
  }

  pub async fn send_disconnect(&self) -> Result<()> {
    self.proxy().await?.disconnect().await?;
    Ok(())
  }
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
