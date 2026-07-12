//! Optional master-PIN storage in the desktop secret store (the freedesktop
//! Secret Service API — implemented by GNOME Keyring, KDE KWallet, and any
//! provider COSMIC runs). Every operation is **best-effort**: if the service is
//! absent, locked, or corrupt (a known GNOME-keyring failure mode), we behave as
//! if no PIN is stored and the app falls back to its own PIN prompt. We never
//! block startup on the keyring.

const SERVICE: &str = "io.github.techneut92.gpgui";
const ACCOUNT: &str = "vault-master-pin";

fn entry() -> Option<keyring::Entry> {
  match keyring::Entry::new(SERVICE, ACCOUNT) {
    Ok(e) => Some(e),
    Err(e) => {
      tracing::debug!("secret service unavailable: {e}");
      None
    }
  }
}

/// Retrieve the stored master PIN, or `None` on any failure (not found, locked,
/// corrupt, or no service).
pub fn load_pin() -> Option<String> {
  match entry()?.get_password() {
    Ok(pin) => Some(pin),
    Err(keyring::Error::NoEntry) => None,
    Err(e) => {
      tracing::warn!("could not read unlock PIN from keyring: {e}");
      None
    }
  }
}

/// Whether the desktop secret store is reachable (best-effort). A missing entry
/// still counts as available — the service is up, nothing is stored yet.
pub fn available() -> bool {
  let Some(e) = entry() else { return false };
  matches!(e.get_password(), Ok(_) | Err(keyring::Error::NoEntry))
}

/// Store the master PIN (best-effort).
pub fn store_pin(pin: &str) {
  if let Some(e) = entry() {
    if let Err(err) = e.set_password(pin) {
      tracing::warn!("could not store unlock PIN in keyring: {err}");
    }
  }
}

/// Remove the stored PIN; ignores "not found".
pub fn clear_pin() {
  if let Some(e) = entry() {
    match e.delete_credential() {
      Ok(()) | Err(keyring::Error::NoEntry) => {}
      Err(err) => tracing::warn!("could not clear unlock PIN from keyring: {err}"),
    }
  }
}
