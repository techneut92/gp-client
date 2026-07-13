//! Service transport — how the GUI reaches gpservice.
//!
//! One transport for every install (native and Flatpak): the host D-Bus **system**
//! service. gpservice is a dbus-activated root service whose privileged methods
//! are polkit-gated (`io.github.techneut92.gpservice.manage` — an active local
//! user is allowed without a prompt; remote/inactive callers need admin auth), so
//! there's no per-launch pkexec and no loopback socket to secure.

use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::dbus_client::{self, DbusHandle};
use gp_protocol::{ConnectAuthRequest, ProbeReply, ProbeRequest, VpnState};

pub struct Transport {
  handle: DbusHandle,
}

impl Transport {
  pub async fn send_disconnect(&self) -> Result<()> {
    self.handle.send_disconnect().await
  }

  /// Probe a gateway and get the required auth back (the backend runs prelogin,
  /// including the PKCS#11 mTLS).
  pub async fn probe(&self, request: ProbeRequest) -> Result<ProbeReply> {
    let reply = self.handle.probe(serde_json::to_string(&request)?).await?;
    Ok(serde_json::from_str(&reply)?)
  }

  /// Authenticate with a captured credential and start the tunnel.
  pub async fn send_connect_auth(&self, request: ConnectAuthRequest) -> Result<()> {
    self.handle.send_connect_auth(serde_json::to_string(&request)?).await
  }
}

/// Open the transport and return a unified stream of `VpnState` changes.
pub async fn open() -> Result<(Transport, mpsc::Receiver<VpnState>)> {
  let (handle, rx) = dbus_client::open().await.context("connecting to gpservice over D-Bus")?;
  Ok((Transport { handle }, rx))
}
