//! Single-instance guard that runs **before** any GTK/Tauri initialization.
//!
//! Why not `tauri-plugin-single-instance`: on Linux it detects the second
//! instance via a D-Bus *session* name, and that doesn't behave the same inside
//! the Flatpak sandbox (the same reason the tray needs `disable_dbus_name`). When
//! the plugin fails to detect the running instance, GTK's own `GApplication`
//! (keyed on the app-id, which *is* honored in the sandbox) forwards the relaunch
//! into the primary and re-runs Tauri's setup there — panicking with "a webview
//! with label `main` already exists" and taking the tray-resident app (and its
//! live tunnel) down. That is exactly the reported crash.
//!
//! Instead we claim an **abstract-namespace Unix socket** as the very first thing
//! in `main()`. The abstract namespace is shared across processes in the same
//! network namespace — including multiple Flatpak instances of this app, since
//! the manifest uses `--share=network`. The second instance connects, tells the
//! primary to reveal its window, and `exit`s before GTK is ever constructed, so
//! the crash is structurally impossible.

use std::io::{Read, Write};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixListener, UnixStream};

/// Abstract socket name (no leading NUL — `from_abstract_name` adds it). Tied to
/// the app-id so it never collides with another program.
const ABSTRACT_NAME: &[u8] = b"io.github.techneut92.GPClient.single-instance";

/// Message the second instance sends to ask the primary to show its window.
const SHOW: &str = "show";
/// Prefix marking a forwarded SAML browser-SSO callback URL.
const CALLBACK_PREFIX: &str = "cb:";
/// The custom URI scheme GlobalProtect redirects to at the end of browser SSO.
/// A relaunch carrying such an argument is a callback to forward, not a "show".
const CALLBACK_SCHEME: &str = "globalprotectcallback:";

/// What a relaunch of the app is asking the primary instance to do.
pub enum Signal {
  /// Reveal the primary's window (a plain relaunch).
  Show,
  /// A browser-SSO callback URL to hand to the waiting sign-in flow.
  Callback(String),
}

/// Acquire the single-instance lock.
///
/// - Returns `Some(listener)` when we are the **primary** instance; the caller
///   must keep it and service incoming "show" pings (see [`serve`]).
/// - When another instance already holds it, signals that instance to reveal its
///   window and **exits the process** — before any GTK/Tauri init.
/// - Returns `None` if the lock can't be used at all (non-Linux abstract-socket
///   failure); the app then runs without the guard rather than refusing to start.
pub fn acquire_or_signal() -> Option<UnixListener> {
  let addr = match SocketAddr::from_abstract_name(ABSTRACT_NAME) {
    Ok(addr) => addr,
    Err(_) => return None,
  };

  match UnixListener::bind_addr(&addr) {
    Ok(listener) => Some(listener),
    Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
      // A primary is already running — poke it to surface, then bow out. We do
      // this before returning so GTK/Tauri never initializes in this process.
      // If this relaunch is a browser-SSO callback (the desktop file registers
      // gp-client as the `globalprotectcallback:` scheme handler and passes the
      // URL via `%u`), forward the URL so the primary's sign-in flow completes.
      if let Ok(mut stream) = UnixStream::connect_addr(&addr) {
        let msg = std::env::args()
          .find(|a| a.starts_with(CALLBACK_SCHEME))
          .map(|url| format!("{CALLBACK_PREFIX}{url}"))
          .unwrap_or_else(|| SHOW.to_string());
        let _ = stream.write_all(msg.as_bytes());
      }
      std::process::exit(0);
    }
    Err(_) => None,
  }
}

/// Service relaunch pings on the primary's listener. Blocks, so run it on its own
/// thread. `on_signal` is called for each relaunch attempt with what it wants
/// (reveal the window, or deliver a browser-SSO callback URL).
pub fn serve(listener: UnixListener, on_signal: impl Fn(Signal) + Send + 'static) {
  for stream in listener.incoming() {
    match stream {
      Ok(mut stream) => {
        // The second instance writes its message and exits, closing the stream —
        // so a read to EOF gets the whole (possibly long) callback URL.
        let mut buf = String::new();
        let _ = stream.read_to_string(&mut buf);
        let signal = match buf.strip_prefix(CALLBACK_PREFIX) {
          Some(url) => Signal::Callback(url.to_string()),
          None => Signal::Show,
        };
        on_signal(signal);
      }
      Err(_) => continue,
    }
  }
}
