#!/usr/bin/env bash
#
# Fill the Flathub manifest's source sha256 from the released tarball.
#
# The Flathub manifest pins its source tarball by URL + sha256. That tarball
# (gp-client-<version>-flathub.tar.gz) is produced by the release workflow's
# `flathub-tarball` job and attached to the GitHub Release. Once the release has
# published it, run this from the repo root: it downloads the exact released
# asset, hashes it, and writes the hash into the manifest — no local rebuild, so
# no reproducibility assumptions. Then commit the manifest and open the Flathub
# PR.
#
#   scripts/flathub-sha256.sh
#
set -euo pipefail

manifest="packaging/flathub/io.github.techneut92.GPClient.yaml"
[ -f "$manifest" ] || { echo "run from the repo root ($manifest not found)" >&2; exit 1; }

url="$(grep -oE 'https?://\S*flathub\.tar\.gz' "$manifest" | head -1)"
[ -n "$url" ] || { echo "no flathub.tar.gz source url found in $manifest" >&2; exit 1; }
echo "Fetching released asset:"
echo "  $url"

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
curl -fSL --retry 3 -o "$tmp" "$url"
sha="$(sha256sum "$tmp" | cut -d' ' -f1)"
echo "sha256: $sha"

# Replace the value on the `sha256:` line that immediately follows the url line,
# preserving indentation. Works whether it's still the all-zero placeholder or a
# hash from a previous release.
awk -v sha="$sha" '
  /flathub\.tar\.gz/ { print; found=1; next }
  found && /^[[:space:]]*sha256:/ {
    match($0, /^[[:space:]]*/)
    print substr($0, RSTART, RLENGTH) "sha256: " sha
    found=0
    next
  }
  { print }
' "$manifest" > "$manifest.tmp" && mv "$manifest.tmp" "$manifest"

echo "Updated $manifest"
grep -nE 'url:|sha256:' "$manifest"
