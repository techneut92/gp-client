# Changelog

All notable changes to GP Client are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Releases up to and including **1.4.0** were published as the *gpgui* build of GP
Client, where the GUI and the `gpservice` backend were released in lockstep. From
**1.5.0** onward GP Client is a standalone application in its own repository and
versions **independently** of the backend (which the GUI updates from the
[GlobalProtect-openconnect-dw](https://github.com/techneut92/GlobalProtect-openconnect-dw)
fork).

## [1.5.0] - 2026-07-19

### Added
- GP Client is now a standalone application with its own release line, continuing
  from the earlier gpgui builds and versioning independently of the `gpservice`
  backend.
- **Store certificate / Store PIN, per smart-card identity** (Details tab, GPC-50).
  A smart-card identity can now choose *not* to store its certificate and/or PIN:
  *Store certificate* off picks the cert at connect; when on, a *Store PIN* toggle
  appears, and off enters the PIN at connect. Revealed rows slide in and the editor
  smoothly scrolls to keep them in view. Existing identities are unchanged (cert +
  PIN stored as before). At connect the smart-card block is a **certificate picker**:
  it scans the token and lists the certs whenever the PIN isn't stored, pre-selecting
  the stored cert / your last pick (remembered per identity) / the first cert; the
  chosen cert is threaded through `submit_pin` so the connection re-targets to it —
  so *Store certificate off* now picks at connect instead of failing.
- **Smart-card-removed detection** (GPC-37 / GPS-2): when a smart-card session
  ends unexpectedly (e.g. the hourly HIP recheck can't find the card because it
  was removed, and the portal logs the session out), the disconnect
  notification and status line now say "Smart card not found — re-insert your
  card and reconnect" instead of a bare "connection ended". User-initiated
  disconnects are unaffected.
- **Scoped DNS per identity** (GPC-35): the identity editor has a new *DNS
  domains* list. When set, only those domains resolve through the VPN's DNS
  servers; everything else stays on your normal resolvers. Requires a
  1.5-or-newer backend (older backends ignore the setting and keep routing all
  DNS through the tunnel).
- **Portal mode, experimental** (GPC-36): the identity's *Connect directly as
  gateway* toggle now works when turned off — the backend runs the portal flow
  (portal login, gateway list, gateway login with the portal cookie). Needs a
  1.5-or-newer backend. Interactive MFA/token challenges are answered in portal
  mode too — the portal's RSA/OTP challenge opens the same inline MFA card as a
  gateway challenge. Portal mode has not been verified against a live portal.
- **Connect-time gateway picker** (GPC-36): when the portal offers more than one
  gateway, the connecting window pauses on a picker — each row shows the
  gateway's name, address and a live latency estimate (a TCP handshake to its
  TLS port, color-coded), with the backend's region-preferred gateway
  pre-selected so *Continue* keeps the old automatic behavior. Needs a backend
  with the `GatewaySelect`/`select_gateway` handshake (1.5-or-newer); single-
  gateway portals and direct-gateway identities never see the prompt.
- **Inline connect-time challenges (UI ready, dormant).** The connecting window
  can now show an **MFA challenge** — a 6-digit code for TOTP/SMS/RSA-token, plus
  a **push / tap-to-confirm** "Waiting for approval" variant — and a **smart-card
  PIN prompt** that names the card in use (e.g. "Unlock your Yubico smart card to
  authenticate") and shows a **smart-card block** — a card row with the token
  label and the PKCS#11 module beneath it (GPC-43) — with the hero orb shrinking
  to make room. They're wired to `submit_mfa` / `resend_mfa` / `submit_pin`; push
  tap-to-confirm stays dormant until a backend surfaces it (GPS-17).
- **"Update GP Client" screen for a too-new backend** (GPC-42 / GPS-18): the
  inverse of the backend-install screen. When the host's `gpservice` speaks a
  newer wire protocol than this app understands, GP Client now shows an update
  screen — the two app/backend versions side by side, an **Update GP Client**
  button, and a manual command **matched to how the app was installed** (Flatpak
  installs the `.flatpak` bundle straight from the latest GitHub release with
  `curl`, since there's no Flatpak remote yet; native uses the package manager) —
  instead of failing to connect. Detection reads
  the backend's advertised protocol range (`gpservice --protocol`); backends too
  old to advertise one are treated as compatible.
- **Create-vault now confirms your master PIN.** First-run setup has a second
  "Re-enter your PIN" field and rejects a mismatch or a PIN under 4 characters —
  the master PIN can't be recovered, so this guards against a typo locking you out.
- Licensed under the GNU General Public License v3.0-or-later (see `LICENSE`).
- Multi-language interface: English, Dutch (Nederlands) and Frisian (Frysk),
  including the localized desktop entry.
- One-time import of settings from the previous gpgui app on first run, with an
  optional prompt to remove the old app after a confirmed switch.

### Changed
- **First-run vault screen tidied.** The *New master PIN* and *Confirm PIN* fields
  now have proper labels and breathing room between them (was a cramped pair of
  boxes), and the language picker is pinned to the bottom of the window behind a
  divider. The "unlock automatically on this device" option is unchanged.
- **Connection settings and the SSO method are now per-identity.** Reported OS,
  user-agent, client version, MTU, reconnect timeout, force-DPD, VPNC script,
  local hostname, the IPv6 / DTLS / XML-POST / ignore-TLS toggles, and the SSO
  method (embedded webview vs. system browser) moved off the global settings
  window onto each identity, so different portals can carry different tuning. Your
  existing global values are migrated onto every saved identity automatically on
  the first unlock after updating — nothing is lost.
- **Settings reorganised.** The settings window now opens on an **Identities**
  list at the top of the sidebar (above General · Support · About); the old
  Authentication and Connection tabs are gone — those options live inside each
  identity now. Adding or opening an identity opens a dedicated editor window.
  The main window's **Manage identities** now opens Settings → Identities (the
  standalone manager window is retired).
- The main-window identity block is now a compact **portal · auth · gateway**
  summary — the DNS-scope row moved into the identity editor (Connection tab)
  where it's edited — and the window is back to **822 px** tall, which the
  trimmed block fits without scrolling.
- **Every language picker is now searchable** — the Settings › General row and
  the picker on the setup / lock / backend screens — and lists each language by
  its native name with the English name beneath it (e.g. *Nederlands · Dutch*),
  so it stays findable whatever language the interface is in (GPC-46). The
  underlying dropdown component gained optional search + a "no matches" state.
- The identity editor's **DNS domains** field now carries a short caveat that
  Chrome's built-in "Async DNS" resolver can cause intermittent timeouts on VPN
  DNS, with a *Learn more* link to [`docs/scoped-dns.md`](docs/scoped-dns.md) —
  which documents the scoped-DNS feature and how to disable Chrome's async
  resolver (flag or the `BuiltInDnsClientEnabled` policy). Firefox and other
  glibc-based apps are unaffected.
- Trimmed the "Requires backend 1.5 or newer" sentence from the identity editor's
  scoped-DNS hint; the main window already flags the backend requirement where it
  matters.
- Backend compatibility now enforces a real minimum backend version (**1.3.1**,
  the first backend that can run the authentication handoff) instead of comparing
  the GUI's version against the backend's `major.minor`. That comparison assumed
  the two shared a version line and reported a spurious mismatch once they drifted
  apart (e.g. GUI 1.5.0 against backend 1.4.0). A backend older than the minimum
  is now treated like a missing backend and routes to the install/upgrade screen.

### Fixed
- The install version used when GitHub's release API is unreachable or
  rate-limited now points at 1.4.0 (was a stale 1.3.0, below the minimum the
  client can connect through). The package download itself still requires a
  connection — this only covers the API lookup.
- Widened the spacing between the language picker's globe icon/label and the
  dropdown on the backend-install, setup, lock and import screens — the tight
  gap read as if they were touching (GPC-34).
- The outdated-backend screen's action button reads "Update backend" again — a
  duplicated translation key had replaced it with the status text "Backend
  update available".
- **Flatpak: the smart-card cert picker works again** (GPC-52). The bundled
  `libpcsclite` was installing to `/app/lib64`, which isn't on the Flatpak
  loader path, so the in-sandbox token scan failed (`CKR_GENERAL_ERROR`) and the
  connect-time picker showed an empty "Smart card" row. Forcing the pcsc-lite
  build to `--libdir=lib` pins it where the loader looks, independent of the
  SDK's meson default. Stored-cert connections were unaffected.

## [1.4.0] - 2026-07-14

### Changed
- The client reaches the privileged backend exclusively over the D-Bus system
  service; the loopback WebSocket transport has been removed. Requires the 1.4.0
  backend.

### Fixed
- Reconnecting after resume from sleep is fast and leak-proof: the gateway's host
  route is re-pinned to the physical interface before reconnecting (the NIC flap
  drops it), so the tunnel returns in seconds instead of stalling on a dead route
  for minutes, and nothing leaks while it is down since the tunnel is never torn
  down.

## [1.3.1] - 2026-07-12

### Added
- Backend groundwork for the independent GUI: `gpservice` can now run
  authentication itself (prelogin, smart-card mTLS, SAML and gateway login),
  driven over the wire protocol. No change to how the app connects.

## [1.3.0] - 2026-07-12

### Added
- Reconnect immediately on resume from sleep (reusing the existing session) with
  an honest "Reconnecting…" state instead of claiming Connected while the tunnel
  is dead.
- Ported upstream 2.6.x improvements: host-id is sent so gateway SAML gets its
  authentication-override cookie, a portal-cookie cache lets CLI reconnects skip
  SAML, more robust external-browser sign-in, and Microsoft Defender detection in
  the HIP report.

### Fixed
- Fixed the remaining daily SSO crash (the sign-in window was created and revealed
  off the GTK main thread).

## [1.2.11] - 2026-07-12

### Fixed
- Fixed an intermittent crash when the window was revealed from the tray or by a
  second launch (the window was shown from a background thread, which GTK
  disallows).

## [1.2.10] - 2026-07-12

### Fixed
- Fixed the window not coming to the front when opened from the tray icon or menu
  — it now reliably raises and takes focus, including on Wayland compositors like
  COSMIC.

## [1.2.9] - 2026-07-08

### Fixed
- Fixed smart-card reconnects failing with "data not available" on Flatpak (the
  backend now restarts cleanly between sessions for a fresh card read).
- Disabled accidental zoom (Ctrl+scroll / pinch) in the fixed-size windows.

## [1.2.8] - 2026-07-07

### Fixed
- Fixed "Start minimized" being ignored at login — with the option off, the app
  now shows its window when it autostarts instead of always starting hidden.

## [1.2.7] - 2026-07-07

### Fixed
- Fixed "Update all" so it moves the backend to the latest version too, instead of
  re-installing the current one and leaving the backend behind.

## [1.2.6] - 2026-07-07

### Fixed
- Fixed the relaunch-from-tray crash on Flatpak (the 1.2.4 fix didn't work inside
  the sandbox).
- The VPN tunnel is now torn down if the app quits or crashes while connected, so
  `tun0` is never left dangling.

## [1.2.5] - 2026-07-07

### Fixed
- Fixed smart-card (PKCS#11 / YubiKey) connections failing with "data not
  available" after a card re-seat, pcscd restart, or suspend/resume — the backend
  now recovers automatically without a service restart.

## [1.2.4] - 2026-07-07

### Fixed
- Fixed a crash when relaunching the app from the launcher while it was still
  running in the tray — it now reveals the existing window instead.

## [1.2.3] - 2026-06-28

### Changed
- One "Update all" button (replacing the separate app/backend buttons) that
  narrates each step and offers a Restart/Reboot button to apply the update.

## [1.2.2] - 2026-06-28

### Fixed
- Under Flatpak, the About screen now reads the backend version and install type
  from the host (they showed "?" / "Flatpak" before).

## [1.2.1] - 2026-06-28

### Fixed
- Fixed the main-screen connection timer (was stuck at 00:00:00) and stopped the
  tray menu from showing long error text.

## [1.2.0] - 2026-06-28

### Changed
- Webkit-free backend: the SAML SSO webview now runs in-process in the GUI, so the
  backend package no longer needs libwebkit2gtk. SSO is remembered across
  reconnects within a session.

## [1.1.2] - 2026-06-28

### Fixed
- Fixed the main-screen Update banner button (it did nothing); the main window is
  slightly taller so the banner no longer makes the content scroll.

## [1.1.1] - 2026-06-27

### Added
- Update-available badge (app and backend) with a startup check; redesigned About
  showing app and backend side by side.

### Fixed
- Fixed "Update backend" on atomic/rpm-ostree systems.

## [1.1.0] - 2026-06-27

### Added
- Shared `gp-protocol` wire crate with negotiated version ranges so the GUI and
  backend can't drift, with clearer update/downgrade guidance on mismatch.

### Changed
- Quieter `gpservice` journal.

## [1.0.5] - 2026-06-27

### Added
- Ubuntu 26.04 apt repository (openSUSE Build Service); per-distro install docs.

### Changed
- A deb install smoketest now gates releases.

## [1.0.4] - 2026-06-27

### Added
- Fedora COPR now also builds for Enterprise Linux 10 (RHEL / AlmaLinux / Rocky).

### Changed
- COPR publish is gated on the RPM install test.

## [1.0.3] - 2026-06-27

### Added
- Fedora COPR packaging; Nix flake builds from source; store screenshots.

### Changed
- Patch-tolerant backend compatibility warning.

## [1.0.2] - 2026-06-26

### Changed
- In-app update downloads and reinstalls the new Flatpak; tidier update UI.

## [1.0.1] - 2026-06-26

### Changed
- Flatpak: run the bundled gpauth for SSO; clearer backend-install progress.

## [1.0.0] - 2026-06-24

### Added
- Initial fork release: smart-card prelogin, Tauri GUI, tray client.
