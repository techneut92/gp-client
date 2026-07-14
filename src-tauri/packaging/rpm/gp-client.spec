Name:           gp-client
Version:        1.5.0
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
# pnpm via corepack (bundled with nodejs); falls back to npm's global install.
corepack enable || npm install -g pnpm
export CI=1
pnpm install --frozen-lockfile
pnpm build
# --features custom-protocol: embed the frontend (production mode); otherwise the
# app tries to load the UI from the Vite dev server at localhost:5173.
cargo build --release --features custom-protocol --manifest-path src-tauri/Cargo.toml

%install
install -Dm755 src-tauri/target/release/gp-client %{buildroot}%{_bindir}/gp-client
install -Dm644 src-tauri/packaging/flatpak/io.github.techneut92.GPClient.desktop \
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
* Mon Jul 14 2026 Dylan Westra <dylanwestra@gmail.com> - 1.5.0-1
- New independent GP Client GUI (Svelte + Tauri): connect, identity manager,
  settings and About; smart-card, SAML (embedded or system browser) and password
  sign-in over the gpservice backend.
