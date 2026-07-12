//! App-side smart-card PIN prompt.
//!
//! The prelogin mTLS runs in the root backend, which has no display and can't
//! pop a pinentry. So when a smart-card identity has no stored PIN, gp-client
//! collects it here in a tiny local webview and passes it down for this
//! connection only — the PIN is never written to disk.
//!
//! The form is a self-contained `data:` page (no network, no Tauri IPC): on
//! submit it navigates to `pinsubmit:<url-encoded-pin>`, which we intercept.

use anyhow::Result;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

const FORM_HTML: &str = r#"<!doctype html><html><head><meta charset="utf-8">
<style>
  :root{color-scheme:dark}
  body{margin:0;background:#12141c;color:#e7eaf3;font:14px system-ui,sans-serif;
    height:100vh;display:flex;align-items:center;justify-content:center}
  form{width:260px;text-align:center}
  h1{font-size:15px;font-weight:600;margin:0 0 4px}
  p{font-size:12px;color:#8a91a8;margin:0 0 16px}
  input{width:100%;box-sizing:border-box;padding:10px 12px;border-radius:9px;
    border:1px solid #2a2e3c;background:#1a1d28;color:#e7eaf3;font-size:15px;
    letter-spacing:3px;text-align:center}
  input:focus{outline:none;border-color:#4d7cff}
  button{margin-top:14px;width:100%;padding:10px;border:0;border-radius:9px;
    background:#4d7cff;color:#fff;font-size:14px;font-weight:600;cursor:pointer}
  .cancel{background:transparent;color:#8a91a8;margin-top:8px}
</style></head><body>
<form onsubmit="location.href='pinsubmit:'+encodeURIComponent(document.getElementById('p').value);return false">
  <h1>Smart-card PIN</h1>
  <p>Enter your PIN to sign in. It is used once and not stored.</p>
  <input id="p" type="password" inputmode="numeric" autofocus autocomplete="off">
  <button type="submit">Unlock</button>
  <button type="button" class="cancel" onclick="location.href='pinsubmit:'">Cancel</button>
</form>
<script>document.getElementById('p').focus()</script>
</body></html>"#;

/// Show the PIN prompt. Returns `Ok(Some(pin))` on submit, `Ok(None)` if the
/// user cancels or closes the window.
pub async fn prompt(app: &AppHandle) -> Result<Option<String>> {
  if let Some(w) = app.get_webview_window("pin") {
    let _ = w.close();
  }

  let (tx, rx) = oneshot::channel::<Option<String>>();
  let tx = std::sync::Mutex::new(Some(tx));

  let (built_tx, built_rx) = oneshot::channel::<Result<()>>();
  let app2 = app.clone();
  app.run_on_main_thread(move || {
    let res = WebviewWindowBuilder::new(&app2, "pin", WebviewUrl::External("about:blank".parse().unwrap()))
      .title("Smart-card PIN")
      .inner_size(340.0, 300.0)
      .resizable(false)
      .focused(true)
      .on_navigation(move |nav| {
        if nav.scheme() == "pinsubmit" {
          let pin = urlencoding_decode(nav.as_str().trim_start_matches("pinsubmit:"));
          if let Some(sender) = tx.lock().unwrap().take() {
            let _ = sender.send((!pin.is_empty()).then_some(pin));
          }
          return false;
        }
        true
      })
      .build();
    // Load the form after creation (a data: URL as the initial URL is flaky on
    // some WebKitGTK builds; navigating post-build is reliable).
    let out = res.and_then(|w| {
      let data = format!(
        "data:text/html;charset=utf-8;base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, FORM_HTML)
      );
      let _ = w.navigate(data.parse().unwrap());
      Ok(())
    });
    let _ = built_tx.send(out.map_err(Into::into));
  })?;
  built_rx.await??;

  let pin = rx.await.ok().flatten();
  if let Some(w) = app.get_webview_window("pin") {
    let _ = w.close();
  }
  Ok(pin)
}

/// Minimal percent-decoding for the PIN payload (the form url-encodes it).
fn urlencoding_decode(s: &str) -> String {
  let bytes = s.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut i = 0;
  while i < bytes.len() {
    match bytes[i] {
      b'%' if i + 2 < bytes.len() => {
        let hi = (bytes[i + 1] as char).to_digit(16);
        let lo = (bytes[i + 2] as char).to_digit(16);
        if let (Some(h), Some(l)) = (hi, lo) {
          out.push((h * 16 + l) as u8);
          i += 3;
          continue;
        }
        out.push(bytes[i]);
        i += 1;
      }
      b'+' => {
        out.push(b' ');
        i += 1;
      }
      b => {
        out.push(b);
        i += 1;
      }
    }
  }
  String::from_utf8_lossy(&out).into_owned()
}
