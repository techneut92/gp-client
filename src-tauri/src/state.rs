//! Shared UI state, updated by the VPN manager and read by the egui window + tray.

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Status {
  #[default]
  Disconnected,
  Connecting,
  Connected,
  /// The tunnel dropped (resume from sleep, network change) and gpservice is
  /// re-establishing it with the existing session — no re-auth needed.
  Reconnecting,
  Disconnecting,
  Error(String),
}

impl Status {
  pub fn label(&self) -> String {
    match self {
      Status::Disconnected => "Disconnected".into(),
      Status::Connecting => "Connecting…".into(),
      Status::Connected => "Connected".into(),
      Status::Reconnecting => "Reconnecting…".into(),
      Status::Disconnecting => "Disconnecting…".into(),
      Status::Error(e) => format!("Error: {e}"),
    }
  }

  /// Short, fixed-width label for the tray menu — never the error message, which
  /// would blow out the menu width.
  pub fn short_label(&self) -> &'static str {
    match self {
      Status::Disconnected => "Disconnected",
      Status::Connecting => "Connecting…",
      Status::Connected => "Connected",
      Status::Reconnecting => "Reconnecting…",
      Status::Disconnecting => "Disconnecting…",
      Status::Error(_) => "Error",
    }
  }

  /// True while a connection exists or is being set up/torn down.
  pub fn is_active(&self) -> bool {
    matches!(
      self,
      Status::Connecting | Status::Connected | Status::Reconnecting | Status::Disconnecting
    )
  }
}

/// Details of the live connection, shown on the connected view. Populated from
/// gpservice's `VpnState::Connected` payload, plus a best-effort tun lookup for
/// the assigned IP/iface (which the protocol doesn't carry).
#[derive(Debug, Clone, Default)]
pub struct ConnDetails {
  pub portal: String,
  /// Gateway as `name (address)`.
  pub gateway: String,
  /// Human-readable session expiry, e.g. "expires in 11h" (updated every second).
  pub expires: String,
  /// Unix epoch the session expires at, for the live countdown.
  pub expires_at: Option<u64>,
  pub ip: String,
  pub iface: String,
}

#[derive(Default)]
pub struct Shared {
  pub status: Status,
  /// Last log line from gpclient (for the status area).
  pub log: String,
  /// Generation counter so a stale connection's reader can't clobber newer state.
  pub current_gen: u64,
  /// Live connection details (valid while `status` is Connected).
  pub conn: ConnDetails,
  /// True when the current session's last teardown was user-initiated, so an
  /// unexpected session end can be told apart and explained.
  pub user_disconnect: bool,
  /// PKCS#11 module of the current smart-card connection (`None` otherwise).
  /// Lets an unexpected session end check whether the card is still present.
  pub smartcard_module: Option<String>,
  /// True while the connect pipeline is waiting for a smart-card PIN — drives
  /// the inline PIN prompt in the connecting view (replaces the old popup).
  pub pin_required: bool,
  /// The smart-card manufacturer the PIN prompt is for (e.g. "Yubico"); drives the
  /// prompt subtitle and the card block's label. Empty → a generic prompt.
  pub pin_prompt: String,
  /// The PKCS#11 module file name (e.g. "opensc-pkcs11.so") behind the prompt;
  /// shown as the card block's sub-line. Empty → "PKCS#11 token".
  pub pin_module: String,
  /// True while the backend is waiting for an MFA/token code (gpservice's
  /// `VpnState::MfaChallenge`) — drives the inline MFA card.
  pub mfa_required: bool,
  /// The gateway/IdP prompt for the current MFA challenge.
  pub mfa_prompt: String,
  /// True while the backend is waiting for a portal gateway choice (gpservice's
  /// `VpnState::GatewaySelect`) — drives the inline gateway picker.
  pub gw_required: bool,
  /// The gateways the portal offered, as `(name, address)` pairs.
  pub gw_list: Vec<(String, String)>,
  /// Address of the region-preferred gateway (the picker pre-selects it).
  pub gw_preferred: String,
}
