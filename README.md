# GP Client

[![Release](https://img.shields.io/github/v/release/techneut92/gp-client?label=release&color=brightgreen)](https://github.com/techneut92/gp-client/releases/latest)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](./LICENSE)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-support-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/techneut92)

A GlobalProtect-compatible VPN client for Linux, with a modern **Svelte + Tauri**
GUI. Sign in with a **smart card / PKCS#11** (YubiKey PIV) certificate, **SAML
single sign-on**, or a **password**; save multiple connections in an encrypted
vault; and manage the tunnel from a system-tray icon.

GP Client is the standalone GUI successor to the `gpgui` app that shipped inside
[GlobalProtect-openconnect-dw](https://github.com/techneut92/GlobalProtect-openconnect-dw);
it versions independently and talks to that project's privileged `gpservice`
backend over [`gp-protocol`](https://github.com/techneut92/gp-protocol).

> **GlobalProtect** is a trademark of Palo Alto Networks. This is an independent,
> compatible client and is not affiliated with or endorsed by Palo Alto Networks.

<p align="center">
  <img width="440" src="docs/screenshots/connect.png" alt="GP Client — connect with a smart-card identity">
</p>
<p align="center"><em>Connect with a smart-card (PKCS#11 / YubiKey PIV) identity.</em></p>

<details>
<summary><b>More screenshots</b></summary>

<p align="center">
  <img width="320" src="docs/screenshots/identity_manager.png" alt="Identity manager — PKCS#11 module + certificate">
  <img width="320" src="docs/screenshots/settings.png" alt="Settings">
</p>
<p align="center"><em>Identity manager (PKCS#11 / YubiKey) · Settings.</em></p>

<p align="center">
  <img width="240" src="docs/screenshots/vault.png" alt="Encrypted vault — set a master PIN">
  <img width="240" src="docs/screenshots/backend_required.png" alt="Guided backend install">
  <img width="240" src="docs/screenshots/import.png" alt="Import from the previous app">
</p>
<p align="center"><em>Encrypted vault · guided backend install · one-click import from the old app.</em></p>

</details>

## Contents

- [Features](#features)
- [Architecture](#architecture)
- [Install](#install)
- [Development](#development)
- [Support](#support)
- [License](#license)

## Features

- **Smart-card / PKCS#11 auth** — a YubiKey PIV (or any PKCS#11 token) client
  certificate for portal *and* gateway login, with card auto-detection.
- **SAML single sign-on** — either an embedded webview or your system browser —
  plus username/password and client-certificate-file auth.
- **Encrypted identity vault** — save multiple connections, unlocked by a PIN;
  optionally remembered in your desktop keyring (GNOME Keyring / KWallet / COSMIC).
- **Portal and gateway** connections, including a direct "connect as gateway" mode.
- **System-tray client** with state-aware icons, connect-from-tray, and desktop
  notifications; optional autostart and start-minimized.
- **Fast, leak-proof reconnect** on resume from sleep — the tunnel is never torn
  down, so nothing escapes it while the network comes back.
- **Multi-language UI** — English, Dutch, Frisian, with more on the way.
- **Update checks** for both the app and the backend.

## Architecture

GP Client is split into an unprivileged GUI and a privileged backend that talk
over a stable wire contract:

```
┌─────────────────┐   gp-protocol    ┌──────────────────────────┐
│  gp-client (UI) │ ───────────────► │  gpservice (root backend)│
│  Tauri + Svelte │   D-Bus system   │  openconnect + tun0      │
│  unprivileged   │ ◄─────────────── │  runs the actual tunnel  │
└─────────────────┘  (polkit-gated)  └──────────────────────────┘
```

- **`gp-client`** (this repo) is the unprivileged GUI: tray, transport, the
  encrypted vault, config, updater and single-instance handling. It performs the
  interactive parts of authentication (the SAML webview, PIN entry) and links
  **no GPL-licensed code**.
- **`gpservice`** is the privileged backend that owns the OpenConnect FFI and the
  `tun0` device. It runs prelogin, portal/gateway login and the tunnel itself,
  driven over [`gp-protocol`](https://github.com/techneut92/gp-protocol) on the
  D-Bus system bus (polkit-gated). It ships as a separate host package because a
  VPN tunnel can't be sandboxed.

## Install

GP Client (the GUI) and the `gpservice` backend install separately.

1. **The GUI** — Flatpak is the recommended channel (Flathub submission pending);
   native `.deb`, `.rpm`, AppImage, Arch and Alpine packages are attached to each
   [release](https://github.com/techneut92/gp-client/releases):

   ```bash
   curl -fLO https://github.com/techneut92/gp-client/releases/latest/download/io.github.techneut92.GPClient.flatpak
   flatpak install --user --or-update --assumeyes io.github.techneut92.GPClient.flatpak
   flatpak run io.github.techneut92.GPClient
   ```

2. **The backend** — install the `gpservice` host package from the
   [backend releases](https://github.com/techneut92/GlobalProtect-openconnect-dw/releases)
   (`.rpm` / `.deb` / `.pkg.tar.zst` / `.apk`, or the Fedora COPR / Ubuntu apt repo).
   On first run GP Client detects whether a compatible backend (**≥ 1.3.1**) is
   present and, if not, shows an install/upgrade screen with the exact command for
   your distribution.

Migrating from the old `gpgui` app? On first launch GP Client offers to import
your identities and settings and remove the old app in one click.

## Development

The repository has three domains:

```
gp-client/
├── ui/          # the web frontend (Svelte + Vite): pages, components, i18n
├── src-tauri/   # the Rust/Tauri application (tray, transport, vault, updater)
├── packaging/   # flatpak · flathub · rpm · arch · alpine · linux desktop
└── docs/        # packaging notes, screenshots
```

Install the frontend deps with pnpm in `ui/`, then run the Tauri CLI **from the
repo root** (it locates `src-tauri/` as a subfolder, and its `beforeBuildCommand`
builds the frontend in `ui/` automatically):

```bash
pnpm -C ui install
ui/node_modules/.bin/tauri dev      # runs against a local gpservice
ui/node_modules/.bin/tauri build
```

Packaged builds are built with `--features custom-protocol` (the release CI does
this) so the frontend is embedded; otherwise the app loads the UI from the Vite
dev server at `localhost:5173`. See [`docs/packaging.md`](docs/packaging.md) for
the per-distribution build notes, and [`CHANGELOG.md`](CHANGELOG.md) for release
history.

## Support

If this project saves you some time, you can support its development:

- **Ko-fi** — [ko-fi.com/techneut92](https://ko-fi.com/techneut92) (one-off tips, no account needed)
- **Revolut** — [revolut.me/techneut92](https://revolut.me/techneut92)
- **Ethereum (ETH)** — `0x15d9B8383A7cbe9f99F72aC29106C53bbcf4ea40` (Ethereum network; ETH / ERC-20 only)

[![ko-fi](https://img.shields.io/badge/Support%20me%20on-Ko--fi-FF5E5B?logo=ko-fi&logoColor=white)](https://ko-fi.com/techneut92)
[![Revolut](https://img.shields.io/badge/Revolut-tip-0666EB?logo=revolut&logoColor=white)](https://revolut.me/techneut92)

## License

Copyright © 2026 Dylan Westra (techneut92).

GP Client is free software, licensed under the **GNU General Public License,
version 3 or later (GPL-3.0-or-later)** — see [`LICENSE`](LICENSE). As the sole
copyright holder, the author may also make the software available under other
terms.

The GUI itself links no GPL-licensed code; the copyleft backend (`gpservice`) is
a separate program, reached only over the `gp-protocol` D-Bus contract, and is
distributed under its own license.
