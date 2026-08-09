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

use std::sync::{Mutex as StdMutex, OnceLock};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
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
      "prelogin-cookie" => res.prelogin_cookie = Some(v.to_string()),
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
      // Roomier than the first cut (+30% width, +10% height) so IdP pages fit.
      .inner_size(676.0, 704.0)
      // Start hidden: when the IdP session is still valid the flow auto-completes
      // in well under a second, so the window is closed before it ever shows —
      // a silent reconnect. It's only revealed (below) if auth stalls.
      .visible(false)
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
      .build();
    // On the Flatpak sandbox, install the TLS-trust fallback so a broken
    // p11-kit bridge (empty trust store) doesn't block sign-in (GH #23).
    #[cfg(target_os = "linux")]
    if let Ok(win) = &result {
      crate::webview_tls::install_trust_fallback(win);
    }
    let _ = built_tx.send(result.map(|_| ()).map_err(Into::into));
  })?;
  built_rx.await??;

  // If the flow doesn't auto-complete quickly (needs interaction), reveal the
  // window so the user can sign in. A valid session finishes first and this
  // never fires → no flash. Marshal the show onto the GTK main thread.
  let mut rx = rx;
  let result = match tokio::time::timeout(std::time::Duration::from_millis(1500), &mut rx).await {
    Ok(res) => res.ok().flatten(),
    Err(_) => {
      let app_show = app.clone();
      let _ = app.run_on_main_thread(move || {
        if let Some(w) = app_show.get_webview_window("saml") {
          let _ = w.show();
          let _ = w.set_focus();
        }
      });
      rx.await.ok().flatten()
    }
  };

  // Closing the window before completion counts as a cancel.
  if let Some(w) = app.get_webview_window("saml") {
    let _ = w.close();
  }

  match result {
    Some(res) => Ok(res),
    None => bail!("single sign-on was cancelled"),
  }
}

// ---------------------------------------------------------------------------
// System-browser SSO
//
// Some IdPs refuse to authenticate inside an embedded webview (device-trust /
// conditional-access policies), so the user can opt to sign in in their real
// browser (Settings → sign-in method "browser"). The browser can't hand a
// custom-scheme redirect back to us directly: GlobalProtect always redirects to
// `globalprotectcallback:<payload>`, so gp-client registers itself as that
// scheme's handler (the desktop file's `MimeType` + `Exec … %u`). The OS then
// launches a second gp-client with the URL, which `single_instance` forwards to
// the running primary — delivered here via `deliver_callback`.

/// The waiter for an in-flight browser sign-in. Only one connection runs at a
/// time (single-connection app), so a single slot suffices.
static PENDING_CALLBACK: OnceLock<StdMutex<Option<oneshot::Sender<String>>>> = OnceLock::new();

fn callback_slot() -> &'static StdMutex<Option<oneshot::Sender<String>>> {
  PENDING_CALLBACK.get_or_init(|| StdMutex::new(None))
}

/// Deliver a `globalprotectcallback:` URL to a waiting browser sign-in. Returns
/// `true` if a sign-in was actually waiting for it. Called from
/// `single_instance::serve` on the primary instance.
pub fn deliver_callback(url: String) -> bool {
  if let Some(tx) = callback_slot().lock().unwrap().take() {
    tx.send(url).is_ok()
  } else {
    false
  }
}

/// Sign in via the user's system browser, resolving when GlobalProtect redirects
/// to `globalprotectcallback:` (routed back to us as its scheme handler).
pub async fn authenticate_browser(saml_request: &str) -> Result<SamlResult> {
  let (tx, rx) = oneshot::channel::<String>();
  // Arm the waiter *before* opening the browser so a fast callback can't race us.
  *callback_slot().lock().unwrap() = Some(tx);

  if let Err(e) = open_in_browser(saml_request) {
    let _ = callback_slot().lock().unwrap().take();
    return Err(e);
  }

  // Generous cap: the user may need to complete MFA in the browser.
  let url = match tokio::time::timeout(Duration::from_secs(300), rx).await {
    Ok(Ok(url)) => url,
    Ok(Err(_)) => bail!("browser sign-in was cancelled"),
    Err(_) => {
      let _ = callback_slot().lock().unwrap().take();
      bail!("timed out waiting for browser sign-in (5 min)");
    }
  };

  parse_callback(&url).ok_or_else(|| anyhow!("the browser sign-in returned no cookie"))
}

/// Open the SAML request in the system browser. A REDIRECT-binding request is a
/// plain URL; a POST-binding request is an auto-submitting HTML page, which we
/// stage as a temp file and open.
fn open_in_browser(saml_request: &str) -> Result<()> {
  if saml_request.starts_with("http://") || saml_request.starts_with("https://") {
    crate::system::open_url(saml_request);
    return Ok(());
  }
  // POST-binding request: stage the auto-submitting HTML in a fresh, unpredictable
  // 0600 file. create_new (O_EXCL) means a pre-planted symlink at the path can't
  // redirect the write, and the random name avoids a predictable /tmp target.
  use chacha20poly1305::aead::{rand_core::RngCore, OsRng};
  use std::io::Write;
  use std::os::unix::fs::OpenOptionsExt;
  let mut rnd = [0u8; 16];
  OsRng.fill_bytes(&mut rnd);
  let name: String = format!("gp-client-sso-{}.html", rnd.iter().map(|b| format!("{b:02x}")).collect::<String>());
  let path = std::env::temp_dir().join(name);
  let mut f = std::fs::OpenOptions::new()
    .write(true)
    .create_new(true)
    .mode(0o600)
    .open(&path)
    .context("staging the browser sign-in page")?;
  f.write_all(saml_request.as_bytes()).context("staging the browser sign-in page")?;
  drop(f);
  crate::system::open_url(&format!("file://{}", path.display()));
  Ok(())
}
