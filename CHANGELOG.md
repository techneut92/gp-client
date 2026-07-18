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

## [1.5.0] - Unreleased

### Added
- GP Client is now a standalone application with its own release line, continuing
  from the earlier gpgui builds and versioning independently of the `gpservice`
  backend.
- **Smart-card-removed detection** (GPC-37 / GPS-2): when a smart-card session
  ends unexpectedly (e.g. the hourly HIP recheck can't find the card because it
  was removed, and the portal logs the session out), the disconnect
  notification and status line now say "Smart card not found — re-insert your
  card and reconnect" instead of a bare "connection ended". User-initiated
  disconnects are unaffected.
- **Scoped DNS per identity** (GPC-35): the identity editor has a new *DNS
  domains* list. When set, only those domains resolve through the VPN's DNS
  servers; everything else stays on your normal resolvers. The main window shows
  the identity's DNS scope. Requires a 1.5-or-newer backend (older backends
  ignore the setting and keep routing all DNS through the tunnel).
- **Portal mode, experimental** (GPC-36): the identity's *Connect directly as
  gateway* toggle now works when turned off — the backend runs the portal flow
  (portal login, gateway list, gateway login with the portal cookie). Needs a
  1.5-or-newer backend. Interactive MFA/token challenges during portal auth are
  not handled yet, and portal mode has not been verified against a live portal.
- Licensed under the GNU General Public License v3.0-or-later (see `LICENSE`).
- Multi-language interface: English, Dutch (Nederlands) and Frisian (Frysk),
  including the localized desktop entry.
- One-time import of settings from the previous gpgui app on first run, with an
  optional prompt to remove the old app after a confirmed switch.

### Changed
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
