# GP Client

A GlobalProtect-compatible VPN client GUI for Linux — Svelte + Tauri, with
smart-card (PKCS#11) authentication, SAML SSO, a tray client, and an encrypted
identity vault.

This is the successor GUI to the `gpgui` app inside
[GlobalProtect-openconnect-dw](https://github.com/techneut92/GlobalProtect-openconnect-dw).
It talks to the privileged backend (`gpservice`) over the
[`gp-protocol`](https://github.com/techneut92/gp-protocol) wire contract via the
D-Bus system service.

> “GlobalProtect” is a trademark of Palo Alto Networks, Inc. This is an
> independent, community project and is **not** affiliated with, endorsed by, or
> sponsored by Palo Alto Networks. The name is used only to describe
> compatibility.

## Development

The web frontend lives in `ui/`; the Rust/Tauri app is in `src-tauri/`. Run the
frontend tooling from `ui/` (the Tauri CLI finds `src-tauri/` from there):

```bash
pnpm -C ui install
pnpm -C ui tauri dev      # run against a local gpservice
pnpm -C ui tauri build
```

The frontend is Svelte; the application layer (tray, D-Bus transport, vault,
config, updater, single-instance) is Rust/Tauri. The privileged tunnel itself
runs in `gpservice` — this GUI is unprivileged and drives it over `gp-protocol`.

## License

Copyright © 2026 Dylan Westra (techneut92).

GP Client is free software, licensed under the **GNU General Public License,
version 3 or later (GPL-3.0-or-later)**. See [`LICENSE`](LICENSE) for the full
text. As the sole copyright holder, the author may also make the software
available under other terms.

The GUI itself links no GPL-licensed code; the copyleft backend (`gpservice`) is
a separate program, reached only over the `gp-protocol` D-Bus contract, and is
distributed under its own license.

