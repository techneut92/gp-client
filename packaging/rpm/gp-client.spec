# Cargo's release profile emits no compile-dir debug sources, so the
# debugsource file list comes out empty — EL10's rpmbuild errors on that
# ("Empty %%files file .../debugsourcefiles.list"). No debuginfo to package.
%global debug_package %{nil}

Name:           gp-client
Version:        1.6.0
Release:        1%{?dist}
Summary:        GlobalProtect-compatible VPN client GUI (Svelte + Tauri)

License:        GPL-3.0-or-later
URL:            https://github.com/techneut92/gp-client
Source0:        %{name}-%{version}.tar.gz

# The frontend + Rust build needs network (pnpm install, cargo fetch). COPR
# build roots have network access, so this builds there; plain mock does not.
BuildRequires:  cargo
BuildRequires:  rust >= 1.89
BuildRequires:  nodejs
BuildRequires:  npm
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  gtk3-devel
BuildRequires:  libappindicator-gtk3-devel
BuildRequires:  librsvg2-devel
BuildRequires:  openssl-devel
BuildRequires:  pkgconf-pkg-config

Requires:       webkit2gtk4.1
Requires:       gtk3
Requires:       opensc
Requires:       pcsc-lite

%description
A graphical client for GlobalProtect VPN. Sign in with a smart card / PKCS#11
certificate (such as a YubiKey PIV), SAML single sign-on, or a username and
password. The privileged backend (gpservice) ships separately and is reached
over D-Bus.

%prep
%autosetup -n %{name}-%{version}

%build
# The mock build user can't write /usr/local, so `corepack enable` and
# `npm install -g` both fail with EACCES. Install pnpm into a writable prefix in
# the build dir and put it on PATH instead.
export npm_config_prefix="$PWD/.npm-global"
npm install -g pnpm
export PATH="$PWD/.npm-global/bin:$PATH"
export CI=1
pnpm -C ui install --frozen-lockfile
pnpm -C ui build
# --features custom-protocol: embed the frontend (production mode); otherwise the
# app tries to load the UI from the Vite dev server at localhost:5173.
cargo build --release --features custom-protocol --manifest-path src-tauri/Cargo.toml

%install
install -Dm755 src-tauri/target/release/gp-client %{buildroot}%{_bindir}/gp-client
install -Dm644 packaging/flatpak/io.github.techneut92.GPClient.desktop \
  %{buildroot}%{_datadir}/applications/io.github.techneut92.GPClient.desktop
install -Dm644 src-tauri/icons/128x128.png \
  %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/io.github.techneut92.GPClient.png
install -Dm644 LICENSE %{buildroot}%{_datadir}/licenses/%{name}/LICENSE

%files
%{_bindir}/gp-client
%{_datadir}/applications/io.github.techneut92.GPClient.desktop
%{_datadir}/icons/hicolor/128x128/apps/io.github.techneut92.GPClient.png
%license %{_datadir}/licenses/%{name}/LICENSE

%changelog
* Sat Aug 09 2026 Dylan Westra <dylanwestra@gmail.com> - 1.6.0-1
- Flatpak sign-in no longer fails with "Unacceptable TLS certificate" when the
  host runs a newer p11-kit (e.g. Fedora 44): the SSO webview falls back to the
  runtime/host CA bundle when the sandbox trust bridge is broken (GH #23).
- A smart card removed mid-session is now reported by the backend
  ("Smart card not found - re-insert your card and reconnect") instead of
  guessed by the GUI (GPS-2; requires backend 1.6.0, wire-protocol v5).
- The window no longer stays pinned above everything after being revealed from
  the tray.

* Mon Jul 20 2026 Dylan Westra <dylanwestra@gmail.com> - 1.5.2-1
- Hotfix: "Update all" crashed with a ReferenceError before updating anything
  (build-toolchain miscompilation of a helper). Flatpak bundles no longer
  report a stale version in software centers.

* Mon Jul 20 2026 Dylan Westra <dylanwestra@gmail.com> - 1.5.1-1
- New identities default to portal mode (existing identities keep their saved
  mode). Portal mode verified end-to-end against a live portal with a PKCS#11
  smart card; smart-card portal connects need backend 1.5.1.

* Tue Jul 14 2026 Dylan Westra <dylanwestra@gmail.com> - 1.5.0-1
- New independent GP Client GUI (Svelte + Tauri): connect, identity manager,
  settings and About; smart-card, SAML (embedded or system browser) and password
  sign-in over the gpservice backend.
