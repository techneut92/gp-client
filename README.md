# GP Client

A GlobalProtect-compatible VPN client for Linux, with a modern Svelte + Tauri
GUI. Sign in with a smart card, SAML single sign-on, or a password; save multiple
connections in an encrypted vault; and manage the tunnel from a system-tray icon.

> “GlobalProtect” is a trademark of Palo Alto Networks, Inc. This is an
> independent, community project and is **not** affiliated with, endorsed by, or
> sponsored by Palo Alto Networks. The name is used only to describe
> compatibility.

## Features

- **Smart-card / PKCS#11** authentication (e.g. a YubiKey PIV certificate), with
  card auto-detection.
- **SAML single sign-on**, either in an embedded webview or via your system
  browser.
- **Username / password** and **client-certificate file** authentication.
- **Encrypted identity vault** — save multiple connections, unlocked by a PIN;
  optionally remember the unlock in your desktop keyring.
- **Portal and gateway** connections, including a direct “connect as gateway”
  mode.
- **System-tray client** with desktop notifications; optional autostart and
  start-minimized.
- **Fast, leak-proof reconnect** on resume from sleep — the tunnel is never torn
  down, so nothing escapes it while the network comes back.
- **Multi-language UI** — English, Dutch (Nederlands) and Frisian (Frysk).
- **Update checks** for both the app and the backend.

<!-- Screenshots coming soon (see docs/screenshots/). -->

## Architecture

GP Client is split into an unprivileged GUI and a privileged backend that talk
over a stable wire contract:

```
┌─────────────────┐   gp-protocol    ┌──────────────────────────┐
│  gp-client (UI) │ ───────────────► │  gpservice (root backend)│
│  Tauri + Svelte │   D-Bus system   │  openconnect + tun0      │
│  unprivileged   │ ◄─────────────── │  runs the actual tunnel  │
└─────────────────┘   (polkit-gated) └──────────────────────────┘
```

- **`gp-client`** (this repo) is the unprivileged GUI: tray, transports, the
  encrypted vault, config, updater and single-instance handling. It performs the
  interactive parts of authentication (the SAML webview, PIN entry) and links
  **no GPL-licensed code**.
- **`gpservice`** is the privileged backend that owns the OpenConnect FFI and the
  `tun0` device. It runs prelogin, portal/gateway login and the tunnel itself,
  driven entirely over [`gp-protocol`](https://github.com/techneut92/gp-protocol)
  on the D-Bus system bus (polkit-gated). It ships as a separate host package
  because a VPN tunnel can’t be sandboxed.

This is the successor GUI to the `gpgui` app that shipped inside
[GlobalProtect-openconnect-dw](https://github.com/techneut92/GlobalProtect-openconnect-dw);
GP Client now versions independently of the backend.

## Installing

GP Client (the GUI) and the `gpservice` backend are installed separately.

1. **The GUI** — Flatpak is the recommended channel (Flathub submission pending);
   native `.deb`, `.rpm`, AppImage, Arch and Alpine packages are attached to each
   [GitHub Release](https://github.com/techneut92/gp-client/releases).
2. **The backend** — install the `gpservice` host package from the
   [backend releases](https://github.com/techneut92/GlobalProtect-openconnect-dw/releases).
   On first run, GP Client detects whether a compatible backend (≥ 1.3.1) is
   present and, if not, shows an install/upgrade screen that guides the host
   install for your distribution.

## Development

The repository is organised into three domains:

```
gp-client/
├── ui/          # the web frontend (Svelte + Vite): pages, components, i18n
├── src-tauri/   # the Rust/Tauri application (tray, transport, vault, updater)
├── packaging/   # flatpak · flathub · rpm · arch · alpine · linux desktop
└── docs/        # packaging notes, screenshots
```

The frontend lives in `ui/`; the Rust/Tauri app is in `src-tauri/`. Install
frontend deps with pnpm in `ui/`, then run the Tauri CLI **from the repo root**
(it locates `src-tauri/` as a subfolder, and its `beforeBuildCommand` builds the
frontend in `ui/` automatically):

```bash
pnpm -C ui install
ui/node_modules/.bin/tauri dev      # runs against a local gpservice
ui/node_modules/.bin/tauri build
```

Packaged builds must be built with `--features custom-protocol` (the release CI
does this) so the frontend is embedded; otherwise the app loads the UI from the
Vite dev server at `localhost:5173`. See [`docs/packaging.md`](docs/packaging.md)
for the per-distribution build notes.

## License

Copyright © 2026 Dylan Westra (techneut92).

GP Client is free software, licensed under the **GNU General Public License,
version 3 or later (GPL-3.0-or-later)**. See [`LICENSE`](LICENSE) for the full
text. As the sole copyright holder, the author may also make the software
available under other terms.

The GUI itself links no GPL-licensed code; the copyleft backend (`gpservice`) is
a separate program, reached only over the `gp-protocol` D-Bus contract, and is
distributed under its own license.
