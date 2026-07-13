#!/usr/bin/env bash
# Build (and install) the GP Client Flatpak.
#
# Prerequisites: flatpak on the host. flatpak-builder is used if present,
# otherwise the org.flatpak.Builder flatpak is used automatically. On atomic
# Fedora, install it once with:
#   flatpak install -y flathub org.flatpak.Builder
#
# The frontend (dist/) must already be built — run `pnpm build` (in the
# gp-build distrobox) before this script. tauri-build embeds dist/ at compile
# time.
#
# Usage: src-tauri/packaging/flatpak/flatpak-build.sh
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../../.." && pwd)"          # gp-client repo root
manifest="$here/io.github.techneut92.GPClient.yml"
cd "$root"

if [ ! -f dist/index.html ]; then
  echo "error: dist/ not built. Run 'pnpm build' first (in the gp-build distrobox)." >&2
  exit 1
fi

builder=${FLATPAK_BUILDER:-flatpak-builder}
command -v "$builder" >/dev/null 2>&1 || builder="flatpak run org.flatpak.Builder"

# 1. Runtime, SDK and the matching rust-stable SDK extension. GNOME 50 is built
#    on the freedesktop 25.08 base, so the rust-stable extension is //25.08.
flatpak install -y --user flathub \
  org.gnome.Platform//50 org.gnome.Sdk//50 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08 || true

# 2. Vendor the cargo registry for the sandboxed (offline) build. The generator
#    is a small upstream tool; fetch it if it isn't already here.
gen="$here/flatpak-cargo-generator.py"
[ -f "$gen" ] || curl -fsSL \
  https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py \
  -o "$gen"
python3 "$gen" src-tauri/Cargo.lock -o "$here/cargo-sources.json"

# 3. Build + install to the user installation.
$builder --force-clean --user --install build-flatpak "$manifest"

echo
echo "Built. Run it with:  flatpak run io.github.techneut92.GPClient"
