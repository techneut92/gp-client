# GP Client

A GlobalProtect-compatible VPN client GUI for Linux — Svelte + Tauri, with
smart-card (PKCS#11) authentication, SAML SSO, a tray client, and an encrypted
identity vault.

This is the successor GUI to the `gpgui` app inside
[GlobalProtect-openconnect-dw](https://github.com/techneut92/GlobalProtect-openconnect-dw).
It talks to the same privileged backend (`gpservice`) over the
[`gp-protocol`](https://github.com/techneut92/gp-protocol) wire contract
(loopback WebSocket or D-Bus system service) and links **no GPL code** — see
`LICENSE`.

## Status

Early port. The Rust application layer (tray, transports, vault, config,
updater, single-instance) is fully ported and building; the Svelte UI and the
gp-protocol connect handoff are in progress. Until then, the shipping GUI
remains the one bundled with the backend releases.

## Development

```bash
pnpm install
pnpm tauri dev      # run against a local gpservice
pnpm tauri build
```

Copyright © 2026 Dylan Westra. All rights reserved (see LICENSE).
