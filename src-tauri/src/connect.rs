//! v2 connect path — the authentication half (gp-client edition).
//!
//! In the original gpgui this file linked the fork's `gpapi`/`auth` crates
//! (GPL) to run prelogin + SAML in-process. gp-client links **no GPL code**:
//! the portal/gateway HTTP moves into `gpservice` behind gp-protocol handoff
//! messages (ownership-plan O2), and the SAML webview is re-authored here.
//!
//! Until that handoff lands in gp-protocol/gpservice, `probe` and
//! `build_connect_request` return descriptive errors — the UI runs, connect
//! reports "pending".

use anyhow::{bail, Result};
use gp_protocol::request::ConnectRequest;
use gp_protocol::{ClientOs, ProbeReply, ProbeRequest};
use tauri::AppHandle;

use crate::dbus_client;

/// Inputs captured from the UI for a connection attempt.
pub struct AuthParams {
  /// Server to authenticate against. Currently only the gateway flow is
  /// supported, so this is treated as the gateway.
  pub server: String,
  pub os: String,
  pub user_agent: String,
  /// Client certificate: either a pkcs11 URI **including** `?pin-value=…`, or a
  /// path to a PEM/PKCS#12 cert file.
  pub certificate: String,
  /// Private key file (for PEM cert-file auth where the key is separate).
  pub sslkey: Option<String>,
  /// Passphrase for an encrypted key / PKCS#12 file.
  pub key_password: Option<String>,
  /// Standard (non-SSO) username/password, when the user picks that method.
  pub username: Option<String>,
  pub password: Option<String>,
  /// Run SAML in the system browser instead of the embedded webview.
  pub use_browser: bool,
  /// Advanced connection options (from the settings window).
  pub opts: ConnOpts,
}

/// Advanced connection/tunnel options (the gpclient CLI surface).
#[derive(Debug, Clone, Default)]
pub struct ConnOpts {
  pub mtu: u32,
  pub reconnect_timeout: u32,
  pub force_dpd: u32,
  pub disable_ipv6: bool,
  pub no_dtls: bool,
  pub no_xmlpost: bool,
  pub ignore_tls_errors: bool,
  /// Empty string = unset.
  pub vpnc_script: String,
  pub local_hostname: String,
  pub os_version: String,
  pub client_version: String,
}

/// What the portal's prelogin asks for.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
  /// "saml" | "standard" | "cert" (client cert required) | "error".
  pub kind: String,
  pub supports_browser: bool,
  pub username_label: String,
  pub password_label: String,
  pub message: String,
}

/// Probe a portal/gateway's prelogin to discover the required auth.
///
/// O2: becomes a gp-protocol `Probe` request answered by gpservice (which owns
/// the portal HTTP + the pkcs11 mTLS signing).
pub async fn probe(
  server: &str,
  os: &str,
  user_agent: &str,
  certificate: Option<String>,
  sslkey: Option<String>,
  key_password: Option<String>,
  ignore_tls_errors: bool,
) -> ProbeResult {
  match probe_impl(server, os, user_agent, certificate, sslkey, key_password, ignore_tls_errors).await {
    Ok(reply) => match reply {
      ProbeReply::Saml { supports_browser, .. } => ProbeResult {
        kind: "saml".into(),
        supports_browser,
        username_label: String::new(),
        password_label: String::new(),
        message: String::new(),
      },
      ProbeReply::Standard {
        username_label,
        password_label,
      } => ProbeResult {
        kind: "standard".into(),
        supports_browser: false,
        username_label,
        password_label,
        message: String::new(),
      },
      ProbeReply::Error { message, cert_needed } => ProbeResult {
        kind: if cert_needed { "cert" } else { "error" }.into(),
        supports_browser: false,
        username_label: String::new(),
        password_label: String::new(),
        message,
      },
    },
    Err(e) => ProbeResult {
      kind: "error".into(),
      supports_browser: false,
      username_label: String::new(),
      password_label: String::new(),
      message: format!("{e:#}"),
    },
  }
}

/// A reasonable OS-version string per client OS. gp-client links no gpapi, so
/// this stands in for its `host_utils` (the exact value rarely affects
/// prelogin; the portal keys off `os`).
fn os_version(os: &ClientOs) -> String {
  match os {
    ClientOs::Linux => "Linux".to_string(),
    ClientOs::Windows => "Microsoft Windows 11 Pro, 64-bit".to_string(),
    ClientOs::Mac => "Apple Mac OS X 14.0.0".to_string(),
  }
}

/// The real probe: hands a `ProbeRequest` to the backend over D-Bus (the
/// backend runs prelogin, incl. the PKCS#11 mTLS). WS transport support is
/// pending; this requires the D-Bus transport (`GP_TRANSPORT=dbus`) and a
/// backend ≥ 1.3.1.
async fn probe_impl(
  server: &str,
  os: &str,
  user_agent: &str,
  certificate: Option<String>,
  sslkey: Option<String>,
  key_password: Option<String>,
  ignore_tls_errors: bool,
) -> Result<ProbeReply> {
  let req = ProbeRequest {
    server: server.to_string(),
    certificate,
    sslkey,
    key_password,
    ignore_tls_errors,
    os: Some(ClientOs::from(os)),
    os_version: Some(os_version(&ClientOs::from(os))),
    user_agent: Some(user_agent.to_string()),
  };
  let (handle, _rx) = dbus_client::open().await?;
  let reply = handle.probe(serde_json::to_string(&req)?).await?;
  Ok(serde_json::from_str(&reply)?)
}

/// Authenticate and build the `ConnectRequest` for gpservice.
///
/// O2: prelogin + gateway login run in gpservice over gp-protocol; the SAML
/// webview (when needed) runs here in our own code and only the resulting
/// prelogin-cookie crosses the wire.
pub async fn build_connect_request(_p: &AuthParams, _app_handle: &AppHandle) -> Result<ConnectRequest> {
  bail!("gp-client: connect pending the gp-protocol handoff (O2) — use the gpgui client to connect meanwhile");
}
