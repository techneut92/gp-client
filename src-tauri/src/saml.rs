//! SAML SSO in gp-client's own code — re-authored from the GlobalProtect
//! callback behavior (protocol facts), linking no GPL auth crate.
//!
//! A GP SAML prelogin hands us a `saml_request` (either a URL to open, or an
//! HTML page that auto-submits to the IdP). We show it in a webview; the flow
//! finishes when the IdP redirects the browser to a `globalprotectcallback:`
//! URL carrying the auth cookies. We intercept that navigation, parse the
//! cookies out, and return them for the backend's gateway login.
//!
//! (A secondary completion mode uses `saml-auth-status`/`prelogin-cookie`
//! response *headers*; that needs a WebKitGTK response hook and is a later
//! addition — most flows complete via the callback URL handled here.)

use anyhow::{bail, Result};
use base64::Engine;
use gp_protocol::AuthCredential;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

/// The SAML result the backend needs for gateway login.
#[derive(Debug, Clone, Default)]
pub struct SamlResult {
  pub username: String,
  pub prelogin_cookie: Option<String>,
  pub portal_userauthcookie: Option<String>,
}

impl SamlResult {
  pub fn into_credential(self) -> AuthCredential {
    AuthCredential::Saml {
      username: self.username,
      prelogin_cookie: self.prelogin_cookie,
      portal_userauthcookie: self.portal_userauthcookie,
    }
  }
}

/// Parse a `globalprotectcallback:<payload>` URL into a [`SamlResult`].
///
/// The payload is either base64-encoded XML holding `<prelogin-cookie>` /
/// `<saml-username>` / `<portal-userauthcookie>` elements, or a plain
/// `key=value&…` query string with `un` / `prelogin-cookie` /
/// `portal-userauthcookie` (older CAS form).
pub fn parse_callback(url: &str) -> Option<SamlResult> {
  let payload = url.strip_prefix("globalprotectcallback:")?;
  let payload = payload.trim();

  // Try base64-XML first.
  if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(payload) {
    if let Ok(xml) = String::from_utf8(bytes) {
      let get = |tag: &str| extract_tag(&xml, tag);
      let res = SamlResult {
        username: get("saml-username").unwrap_or_default(),
        prelogin_cookie: get("prelogin-cookie"),
        portal_userauthcookie: get("portal-userauthcookie"),
      };
      if res.prelogin_cookie.is_some() || res.portal_userauthcookie.is_some() {
        return Some(res);
      }
    }
  }

  // Fall back to a query-string form.
  let mut res = SamlResult::default();
  for pair in payload.split('&') {
    let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
    match k {
      "un" | "saml-username" => res.username = v.to_string(),
      "prelogin-cookie" | "prelogin-cookie" => res.prelogin_cookie = Some(v.to_string()),
      "portal-userauthcookie" => res.portal_userauthcookie = Some(v.to_string()),
      _ => {}
    }
  }
  (res.prelogin_cookie.is_some() || res.portal_userauthcookie.is_some()).then_some(res)
}

/// Minimal `<tag>value</tag>` extraction (the callback XML is flat and simple;
/// avoids pulling an XML crate into the GUI).
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
  let open = format!("<{tag}>");
  let close = format!("</{tag}>");
  let start = xml.find(&open)? + open.len();
  let end = xml[start..].find(&close)? + start;
  let val = xml[start..end].trim();
  (!val.is_empty()).then(|| val.to_string())
}

/// Show the SAML request in a webview and resolve when the IdP redirects to
/// `globalprotectcallback:`. Returns an error if the user closes the window
/// first.
pub async fn authenticate(app: &AppHandle, saml_request: &str) -> Result<SamlResult> {
  // A prior attempt may have left the window around.
  if let Some(w) = app.get_webview_window("saml") {
    let _ = w.close();
  }

  let url = if saml_request.starts_with("http://") || saml_request.starts_with("https://") {
    WebviewUrl::External(saml_request.parse()?)
  } else {
    // Inline HTML page (auto-submits to the IdP) → data URL.
    let b64 = base64::engine::general_purpose::STANDARD.encode(saml_request.as_bytes());
    WebviewUrl::External(format!("data:text/html;base64,{b64}").parse()?)
  };

  let (tx, rx) = oneshot::channel::<Option<SamlResult>>();
  let tx = std::sync::Mutex::new(Some(tx));

  // Build the window on the GTK main thread (window creation is a GTK call;
  // this runs from an async worker — same lesson as the fork's GPC-21 fix).
  let (built_tx, built_rx) = oneshot::channel::<Result<()>>();
  let app2 = app.clone();
  app.run_on_main_thread(move || {
    let result = WebviewWindowBuilder::new(&app2, "saml", url)
      .title("Sign in")
      .inner_size(520.0, 640.0)
      .focused(true)
      .on_navigation(move |nav| {
        if nav.scheme() == "globalprotectcallback" {
          if let Some(sender) = tx.lock().unwrap().take() {
            let _ = sender.send(parse_callback(nav.as_str()));
          }
          return false; // cancel the nav to the custom scheme
        }
        true
      })
      .build()
      .map(|_| ());
    let _ = built_tx.send(result.map_err(Into::into));
  })?;
  built_rx.await??;

  // Closing the window before completion counts as a cancel.
  let app3 = app.clone();
  let result = rx.await.ok().flatten();
  if let Some(w) = app3.get_webview_window("saml") {
    let _ = w.close();
  }

  match result {
    Some(res) => Ok(res),
    None => bail!("single sign-on was cancelled"),
  }
}
