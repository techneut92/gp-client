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
const ABSTRACT_NAME: &[u8] = b"io.github.techneut92.gpgui.single-instance";

/// Message the second instance sends to ask the primary to show its window.
const SHOW: &[u8] = b"show";

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
      if let Ok(mut stream) = UnixStream::connect_addr(&addr) {
        let _ = stream.write_all(SHOW);
      }
      std::process::exit(0);
    }
    Err(_) => None,
  }
}

/// Service "show" pings on the primary's listener. Blocks, so run it on its own
/// thread. `on_show` is called for each relaunch attempt (reveal the window).
pub fn serve(listener: UnixListener, on_show: impl Fn() + Send + 'static) {
  for stream in listener.incoming() {
    match stream {
      Ok(mut stream) => {
        // Best-effort read; any contact means "a relaunch happened, surface".
        let mut buf = [0u8; 8];
        let _ = stream.read(&mut buf);
        on_show();
      }
      Err(_) => continue,
    }
  }
}
