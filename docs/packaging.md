# Packaging & distribution

## What CI builds

`.github/workflows/release.yaml` builds every format on a `v*` tag (or manual
`workflow_dispatch`) and attaches them to the GitHub Release:

| Format | x86_64 | aarch64 | How |
| --- | --- | --- | --- |
| `.deb` / `.rpm` / AppImage | ✓ | ✓ | `tauri build` (rustup toolchain) |
| `.flatpak` | ✓ | ✓ | `flatpak-builder`, manifest in `src-tauri/packaging/flatpak/` |
| Arch `.pkg.tar.zst` | ✓ | ✓ | `makepkg`, `src-tauri/packaging/arch/PKGBUILD` |
| Alpine `.apk` | ✓ | ✓ | `abuild`, `src-tauri/packaging/alpine/APKBUILD` |

All native builds pass `--features custom-protocol` so the frontend is embedded
(production mode); without it the app loads the UI from the Vite dev server at
`localhost:5173`.

Hard-won per-distro notes:
- **Arch**: `options=('!lto')` — Arch's global LTO drops `ring`'s asm objects
  (`undefined symbol: ring_core_*`). ARM also needs `DisableSandbox` in
  `pacman.conf` (pacman 7's Landlock sandbox can't init in the container).
- **Flatpak**: opensc's `PCSC_CFLAGS`/`PCSC_LIBS` are pinned so `winscard.h` is
  always found (pkg-config autodetection flaked on x86 runners).
- **Alpine**: enable the community repo + trust the generated abuild key; the
  arm leg checks out on the host and builds via `docker run` (JS actions can't
  run in an Alpine/musl container on arm64).

## COPR (`.rpm`)

`.github/workflows/copr.yaml` (manual) builds and validates the source RPM
(`src-tauri/packaging/rpm/gp-client.spec`). It **only submits to COPR when a
`COPR_CONFIG` secret is present** — otherwise it's a dry run.

To go live: add `COPR_CONFIG` (your `~/.config/copr` token block) as a repo
secret and set `COPR_PROJECT`. COPR build roots have network, so the spec's
`pnpm install` + `cargo` fetches work — but they need **rust ≥ 1.89** (edition
2024), so target **Fedora 41+/rawhide** chroots, not EL/older Fedora.

## OBS / Debian / Ubuntu — the rust MSRV wall

gp-client requires **rustc 1.89** (`edition = "2024"`, `rust-version = "1.89"`).
Stock distro rust is older:

| Distro | stock rustc | edition 2024 (≥1.85)? | ≥1.89? |
| --- | --- | --- | --- |
| Debian 12 bookworm | 1.63 | ✗ | ✗ |
| Debian 13 trixie | 1.85 | ✓ (barely) | ✗ |
| Ubuntu 22.04 / 24.04 | 1.75 | ✗ | ✗ |
| Ubuntu 25.04 plucky | 1.84 | ✗ | ✗ |

OBS builds are also **hermetic (no network)**, so every cargo crate and npm
package would have to be vendored into the source package.

**Conclusion:** an OBS source build using stock distro rust is not viable —
Debian 13 is the only target whose rust even reaches edition 2024, and it's still
below our 1.89 floor. This is the "rust MSRV distro wall".

### Recommended path for Debian/Ubuntu

The CI `.deb` is built with a **rustup** toolchain (current stable), not distro
rust, so the MSRV wall doesn't apply. It targets the runner's glibc
(ubuntu-24.04 → glibc 2.39), which means the **existing release `.deb` already
installs on Debian 13 (glibc 2.41) and Ubuntu 24.04 / 25.04**.

Two ways forward, in order of effort:

1. **Host an apt repo of the CI `.deb`s** (GitHub Pages `apt` repo, Cloudsmith,
   or packagecloud). Covers Debian 13 + Ubuntu 24.04+ immediately, no OBS.
2. **Wider glibc coverage**: add older base images to the `tauri-bundles` matrix
   (e.g. a Debian 12 / Ubuntu 22.04 container) still using a rustup toolchain,
   so the MSRV wall is bypassed and only the glibc baseline drops. Requires
   webkit2gtk-4.1 on that base (available on trixie; needs backports on older).
3. **OBS anyway**: only feasible by adding a newer rust to the OBS project
   (e.g. path to `devel:languages:rust`) *and* vendoring cargo+npm for the
   hermetic build. High effort; revisit only if an apt repo isn't enough.
