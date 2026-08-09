//! Flatpak-only TLS-trust fallback for the SSO sign-in webview.
//!
//! The GNOME Flatpak runtime routes the system trust store through a p11-kit
//! *client* module that proxies to the host over a socket
//! (`/etc/pkcs11/modules/p11-kit-trust.module` → `p11-kit-client.so`). p11-kit
//! changed its RPC protocol in 0.26, so when the runtime's client (0.25) meets a
//! host server on 0.26+ (e.g. Fedora 44) the bridge fails to initialise and the
//! sandbox's trust store resolves to **zero** anchors. Every TLS certificate is
//! then rejected with "Unacceptable TLS certificate" and browser SSO is blocked
//! outright — no user-supplied env var or `--socket` fixes it, because Flatpak
//! manages the p11-kit socket itself (gp-client GH #23).
//!
//! The runtime still ships a perfectly good PEM CA bundle that needs no bridge
//! (`/etc/ssl/certs/ca-certificates.crt`). So instead of trusting the flaky
//! bridge, we install a fallback: when WebKit rejects a certificate, we
//! re-verify the offered chain **ourselves** against that bundle and, only if it
//! genuinely validates (chain *and* hostname), allow that exact certificate for
//! that host and reload. A truly bad certificate fails the bundle check too and
//! stays blocked — this never blanket-ignores TLS errors, so it is safe to leave
//! installed. When the bridge works the trust store is populated, no rejection
//! fires, and this code stays dormant.
//!
//! Scoped to Linux + the sandbox: on a native build the host trust store works,
//! so we don't touch WebKit's TLS behaviour there at all.

use tauri::WebviewWindow;

/// OID `1.3.6.1.5.5.7.3.1` — `G_TLS_DATABASE_PURPOSE_AUTHENTICATE_SERVER`
/// (id-kp-serverAuth). Passed to `verify_chain` so the bundle check validates a
/// server certificate exactly as a TLS client would.
const PURPOSE_SERVER_AUTH: &str = "1.3.6.1.5.5.7.3.1";

/// CA bundles to build the fallback trust database from, in preference order.
///
/// The host's *extracted* bundle is preferred: it already contains the public
/// roots **plus** any private/corporate anchors an admin added to the host
/// trust store — which matters here because corporate GlobalProtect portals and
/// their IdPs commonly sit behind a private CA. It's reachable only with
/// `--filesystem=host-etc:ro` (see the manifest); without that mount those
/// paths simply don't exist and we fall through to the runtime's own public
/// bundle, which still fixes the common public-IdP case. Each is a self-
/// contained PEM file that needs no p11-kit bridge.
const CA_BUNDLES: &[&str] = &[
  // Host trust (public + corporate), via --filesystem=host-etc:ro.
  "/run/host/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
  "/run/host/etc/ssl/certs/ca-certificates.crt",
  // Runtime's own public bundle (no host access needed).
  "/etc/ssl/certs/ca-certificates.crt",
  "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
];

/// Install the trust fallback on the SSO webview. No-op unless we're running in
/// a Flatpak sandbox (the only place the bridge is in play).
pub fn install_trust_fallback(window: &WebviewWindow) {
  if !crate::system::is_flatpak() {
    return;
  }
  // The closure is dispatched to the GTK main thread, so every glib object it
  // builds (the file database, network addresses) is created there — the one
  // place GObjects may be touched.
  let _ = window.with_webview(|platform| {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use webkit2gtk::gio;
    use webkit2gtk::gio::prelude::*;
    use webkit2gtk::{WebContextExt, WebViewExt};

    let webview = platform.inner();

    // Build the fallback database once from the most-preferred bundle present.
    // If none loads (unexpected in the runtime), skip installing the handler and
    // leave WebKit's own error handling in place.
    let db = match CA_BUNDLES
      .iter()
      .filter(|p| std::path::Path::new(p).exists())
      .find_map(|p| gio::TlsFileDatabase::new(p).ok().map(|db| (db, *p)))
    {
      Some((db, path)) => {
        tracing::info!("Flatpak TLS fallback armed from CA bundle {path}");
        db
      }
      None => {
        tracing::warn!("no CA bundle found for the Flatpak TLS fallback; SSO may fail if the p11-kit bridge is broken");
        return;
      }
    };

    // Hosts we've already re-verified and allowed, so a pathological re-fire of
    // the signal for the same host can never spin into a reload loop.
    let allowed: RefCell<HashSet<String>> = RefCell::new(HashSet::new());

    webview.connect_load_failed_with_tls_errors(move |wv, failing_uri, certificate, _errors| {
      let Some(host) = uri_host(failing_uri) else {
        return false; // can't identify the host — let WebKit report the error
      };
      if allowed.borrow().contains(&host) {
        return false; // already handled once; don't loop
      }

      // Re-verify the chain against the runtime PEM bundle, binding the expected
      // hostname so a valid-but-wrong-host certificate is still rejected. An
      // empty flag set means "no errors": genuinely trusted.
      let identity = gio::NetworkAddress::new(&host, 443);
      let trusted = db
        .verify_chain(
          certificate,
          PURPOSE_SERVER_AUTH,
          Some(&identity),
          gio::TlsInteraction::NONE,
          gio::TlsDatabaseVerifyFlags::NONE,
          gio::Cancellable::NONE,
        )
        .map(|flags| flags.is_empty())
        .unwrap_or(false);

      if !trusted {
        return false; // fails the bundle too — a real bad cert; keep it blocked
      }

      tracing::info!("Flatpak TLS fallback: certificate for {host} is trusted by the runtime CA bundle; allowing and reloading");
      if let Some(context) = wv.web_context() {
        context.allow_tls_certificate_for_host(certificate, &host);
      }
      allowed.borrow_mut().insert(host);
      wv.load_uri(failing_uri);
      true // handled — WebKit skips its TLS error page and honours the reload
    });
  });
}

/// Extract the host from an absolute URL, stripping scheme, any userinfo, and
/// the port (including bracketed IPv6 literals). Deliberately dependency-free —
/// the input is a WebKit `failing_uri`, always an absolute `https://…` URL.
fn uri_host(uri: &str) -> Option<String> {
  let after_scheme = uri.split("://").nth(1)?;
  let authority = after_scheme.split(['/', '?', '#']).next()?;
  // Drop any `user:pass@` prefix.
  let host_port = authority.rsplit('@').next()?;
  let host = if let Some(v6) = host_port.strip_prefix('[') {
    v6.split(']').next()? // [::1]:443 → ::1
  } else {
    host_port.split(':').next()? // host:443 → host
  };
  (!host.is_empty()).then(|| host.to_string())
}

#[cfg(test)]
mod tests {
  use super::uri_host;

  #[test]
  fn extracts_plain_host() {
    assert_eq!(uri_host("https://idp.example.com/saml?x=1").as_deref(), Some("idp.example.com"));
  }

  #[test]
  fn strips_port_and_userinfo() {
    assert_eq!(uri_host("https://user:pw@idp.example.com:8443/path").as_deref(), Some("idp.example.com"));
  }

  #[test]
  fn handles_ipv6_literal() {
    assert_eq!(uri_host("https://[2001:db8::1]:443/").as_deref(), Some("2001:db8::1"));
  }

  #[test]
  fn rejects_garbage() {
    assert_eq!(uri_host("not-a-url"), None);
  }
}
