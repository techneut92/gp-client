# Flathub submission — prep

These are **submission-ready materials**, not a submission. Flathub asks that a
human — not an automated agent — open and shepherd the PR, so do the steps below
yourself. Nothing here is pushed anywhere; it just lives in this repo until you
choose to use it.

Files:
- `io.github.techneut92.GPClient.yaml` — the Flathub manifest (pinned sources,
  fully offline build).
- `io.github.techneut92.GPClient.metainfo.xml` — Flathub-compliant AppStream.

## Before you submit — things that need a real value

1. **Screenshots (required).** Flathub rejects apps without at least one
   reachable screenshot. Add PNGs under `docs/screenshots/` (`connect.png`,
   `manager.png`, `settings.png`) or edit the `<image>` URLs in the metainfo to
   wherever you host them.
2. **The source tarball + sha256.** The manifest builds fully offline, so the
   frontend (`dist/`) must be prebuilt inside the source tarball. Build it on the
   release tag:

   ```sh
   ver=1.4.0
   prefix=gp-client-$ver
   pnpm install --frozen-lockfile
   pnpm build                                   # produces dist/
   rm -rf /tmp/$prefix && mkdir -p /tmp/$prefix
   git archive --format=tar HEAD | tar -x -C /tmp/$prefix   # committed source
   cp -r dist /tmp/$prefix/dist                             # prebuilt frontend
   tar -czf $prefix-flathub.tar.gz -C /tmp $prefix
   sha256sum $prefix-flathub.tar.gz
   ```

   Upload `gp-client-1.4.0-flathub.tar.gz` to the GitHub release, then paste its
   URL + sha256 into the manifest's `gp-client` module.
3. **cargo-sources.json.** Copy `../cargo-sources.json` (regenerate it with
   `flatpak-cargo-generator.py src-tauri/Cargo.lock` if deps changed) next to the
   manifest in the flathub repo — the manifest references it by name.

## Validate locally before opening the PR

```sh
flatpak install -y flathub org.flatpak.Builder
# builds offline from the pinned sources exactly as Flathub will:
flatpak run org.flatpak.Builder --user --install --force-clean build-dir \
  io.github.techneut92.GPClient.yaml
# AppStream must validate clean:
flatpak run --command=appstreamcli org.flatpak.Builder validate \
  io.github.techneut92.GPClient.metainfo.xml
```

## Submitting (you, by hand)

Open a pull request against the **`new-pr`** branch of
`github.com/flathub/flathub`, adding the manifest, metainfo, and
`cargo-sources.json`. Reviewers build and test it; once merged they create the
`flathub/io.github.techneut92.GPClient` repo you'll maintain.

## Review considerations to expect

- **Proprietary license.** `project_license` is `LicenseRef-Proprietary`. Flathub
  accepts proprietary apps but flags them and applies extra scrutiny.
- **Host system service.** `--system-talk-name=io.github.techneut92.GPService`
  reaches a privileged backend the user installs separately (it can't be
  sandboxed). Be ready to explain why, and that the app degrades gracefully with
  the in-app "backend not installed" screen.
- **`--socket=pcsc` and broad `--talk-name`s.** Justify each (smart-card auth via
  host pcscd; secrets/notifications/tray/portals/flatpak-spawn). Reviewers push
  back on unexplained holes.
- **`--share=network`** is for the update check against the GitHub Releases API.
