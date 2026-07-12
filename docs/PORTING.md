# Porting plan — gpgui → gp-client

Goal: behavior-identical successor to `apps/gpgui` (fork repo), Svelte UI,
**zero GPL links** (ownership plan O2). The old GUI keeps shipping until this
one is confirmed; then release links and the fork README switch over.

## Phase A — skeleton (done)
- [x] Rust layer ported verbatim (100% own-authored): tray, state, vpn,
      transport (WS + D-Bus), client, vault/crypto/secrets, config, autostart,
      single-instance, tiling, pkcs11 picker, system, updater paths.
- [x] `connect.rs` replaced by a stub: same surface (`probe`,
      `build_connect_request`, `AuthParams`, `ConnOpts`, `ProbeResult`), returns
      "pending O2 handoff" — the GPL `gpapi`/`auth` links stay behind.
- [x] Tauri v2 + Svelte 5 + Vite 8 (pnpm, all latest), 3-page build matching
      the 3-window model (index / settings / manager).
- [x] Compiles: `cargo check` clean, `pnpm build` clean.

## Phase B — identical UI in Svelte
Port each window from `apps/gpgui/ui/*.html` (+ dropdown.js/no-zoom.js
behaviors) into Svelte components, reusing `src/theme.css` unchanged:
- [ ] index.html → src/index/ (status card, connect controls, identity picker,
      update banner, timer)
- [ ] settings.html → src/settings/ (tabs: General/Connection/Identities/About,
      incl. support cards)
- [ ] manager.html → src/manager/
- [ ] no-zoom guard + dropdown behavior as Svelte actions
- [ ] visual diff against gpgui side-by-side

## Phase C — connect via gp-protocol handoff (O2)
- [ ] gp-protocol vNEXT: `Probe`/`ProbeResult`, `AuthConnect` (credential or
      prelogin-cookie) — additive, protocol v3
- [ ] fork PR: gpservice implements them (it owns portal HTTP + pkcs11 mTLS;
      backend stays GPL — separate program over IPC)
- [ ] gp-client: SAML webview re-authored in own code (Tauri webview,
      intercept `globalprotectcallback:` / `prelogin-cookie` header/HTML;
      **build & raise on the GTK main thread** — see fork GPC-21)
- [ ] browser-SSO flow re-authored (localhost listener + open default browser)
- [ ] fork release ships the handoff; gp-client requires backend >= that

## Phase D — switchover
- [ ] confirm feature parity (connect cert/SAML/user-pass, tray, vault,
      autostart, updater, Flatpak)
- [ ] Flatpak packaging for gp-client (own app id io.github.techneut92.GPClient)
- [ ] switch GUI release links in gpgui + fork README to this repo
- [ ] subsequent GUI releases from here; gpgui frozen, later removed

## Parity checklist (fill during B/C)
Window pixel-parity, tray states/animation, single-instance reveal,
close-to-tray, autostart toggle, vault lock/unlock + tray submenu, update
check/badge/Update-all, D-Bus + WS transports, reconnecting state (proto v2),
session expiry display, support page.
