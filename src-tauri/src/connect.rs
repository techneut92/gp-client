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
use gp_protocol::{ConnectInfo, Gateway};
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

/// Authenticate via the backend handoff and build the `ConnectAuthRequest`.
///
/// The backend runs prelogin (incl. the PKCS#11 mTLS) and reports the auth
/// type; for SAML we run our own webview here and hand back the cookie; the
/// backend then does the gateway login and starts the tunnel. gp-client links
/// no portal/HTTP/GPL code — only `gp-protocol`.
pub async fn authenticate(
  p: &AuthParams,
  app_handle: &AppHandle,
  transport: &crate::transport::Transport,
) -> Result<gp_protocol::ConnectAuthRequest> {
  let os = ClientOs::from(p.os.as_str());
  let cert = (!p.certificate.is_empty()).then(|| p.certificate.clone());

  let probe = ProbeRequest {
    server: p.server.clone(),
    certificate: cert.clone(),
    sslkey: p.sslkey.clone(),
    key_password: p.key_password.clone(),
    ignore_tls_errors: p.opts.ignore_tls_errors,
    os: Some(os.clone()),
    os_version: Some(os_version(&os)),
    user_agent: Some(p.user_agent.clone()),
  };

  // 1. Ask the backend what the gateway wants (it does the mTLS prelogin).
  let credential = match transport.probe(probe).await? {
    ProbeReply::Saml { saml_request, .. } => {
      // 2a. SAML: run our own webview, hand back the cookie.
      let result = crate::saml::authenticate(app_handle, &saml_request).await?;
      result.into_credential()
    }
    ProbeReply::Standard { .. } => {
      // 2b. Standard: use the identity's username/password.
      match (p.username.as_deref(), p.password.as_deref()) {
        (Some(u), Some(pw)) if !u.is_empty() => gp_protocol::AuthCredential::Password {
          username: u.to_string(),
          password: pw.to_string(),
        },
        _ => bail!("This gateway needs a username and password — add them to the identity"),
      }
    }
    ProbeReply::Error { message, .. } => bail!(message),
  };

  // Tunnel options: build a throwaway ConnectRequest to assemble a ConnectArgs
  // (its builders are the only way to set the private fields), then reuse it.
  let o = &p.opts;
  let gateway = Gateway::new(p.server.clone(), p.server.clone());
  let info = ConnectInfo::new(p.server.clone(), gateway.clone(), vec![gateway]);
  let mut args_src = ConnectRequest::new(info, String::new())
    .with_mtu(o.mtu)
    .with_disable_ipv6(o.disable_ipv6)
    .with_no_dtls(o.no_dtls)
    .with_no_xmlpost(o.no_xmlpost)
    .with_force_dpd(o.force_dpd);
  if o.reconnect_timeout > 0 {
    args_src = args_src.with_reconnect_timeout(o.reconnect_timeout);
  }
  if !o.vpnc_script.is_empty() {
    args_src = args_src.with_vpnc_script(Some(o.vpnc_script.clone()));
  }
  if !o.local_hostname.is_empty() {
    args_src = args_src.with_local_hostname(Some(o.local_hostname.clone()));
  }
  if !o.client_version.is_empty() {
    args_src = args_src.with_client_version(&o.client_version);
  }

  Ok(gp_protocol::ConnectAuthRequest {
    server: p.server.clone(),
    credential,
    certificate: cert,
    sslkey: p.sslkey.clone(),
    key_password: p.key_password.clone(),
    ignore_tls_errors: o.ignore_tls_errors,
    os: Some(os),
    os_version: Some(os_version(&ClientOs::from(p.os.as_str()))),
    user_agent: Some(p.user_agent.clone()),
    args: args_src.args().clone(),
  })
}
