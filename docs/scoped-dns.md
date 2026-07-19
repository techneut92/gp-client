# Scoped DNS (per-identity "DNS domains")

Each identity has an optional **DNS domains** list (identity editor → Connection).
It controls how DNS behaves while that identity's tunnel is up:

- **List has entries** → *split DNS*: only those domains (plus any the gateway
  pushes) resolve through the VPN's DNS servers. Everything else keeps using your
  normal resolvers. The tunnel is also removed as the default DNS route, so
  unrelated lookups never touch the corporate resolvers.
- **List empty** → the tunnel's DNS becomes the system-wide default: **all** DNS
  goes through the VPN while connected.

Requires a backend that understands protocol v4 (gpservice **1.5.0+**); older
backends ignore the field and keep routing all DNS through the tunnel.

Under the hood the backend hands the (validated) domain list to the `vpnc-script`
as `GP_DNS_DOMAINS`, which programs **systemd-resolved** per-link via
`resolvectl` — the routing-domain + `default-route false` approach that resolved
[documents for VPN software](https://blogs.gnome.org/mcatanzaro/2020/12/17/understanding-systemd-resolved-split-dns-and-vpn-configuration/).
Domains are validated (dot-separated `[a-z0-9_-]` labels) to keep them safe to
pass to a root-run `resolvectl`.

## Google Chrome: disable the built-in ("async") DNS resolver

**Symptom:** with a VPN DNS active, Chrome intermittently shows `ERR_TIMED_OUT`
(or "can't find the page") on internal sites — roughly 1 in 3–5 loads — and a
refresh a few seconds later works.

**Cause:** Chrome ships its own DNS client (`net::DnsClient`, "async DNS"). Instead
of using the OS resolver, it reads `/etc/resolv.conf` and queries the
systemd-resolved stub (`127.0.0.53`) directly with its own aggressive
timeout/retry and A/AAAA + search-suffix fan-out. The split-DNS *routing* is
correct (resolved applies it); what's flaky is Chrome's transaction layer against
the stub — an occasional lost/slow first (uncached) query times out inside Chrome
before resolved can answer over the tunnel. Every other DNS path we tested
(glibc/`getent`, `curl`, `dig`, even `curl` from inside Chrome's own Flatpak
sandbox) is 100% reliable, which is why the OS resolver is the fix.

This is **not** specific to scoped DNS — it happens whenever Chrome resolves a
name through the VPN's DNS, scoped or not.

### Fix

Force Chrome onto the OS resolver (which is reliable here):

- **Quick, per-user:** `chrome://flags` → search **async** → *Async DNS resolver*
  → **Disabled** → relaunch. (Equivalent CLI: `--disable-features=AsyncDns`.)
- **Persistent / fleet-wide (recommended for managed machines):** the
  [`BuiltInDnsClientEnabled`](https://chromeenterprise.google/policies/#BuiltInDnsClientEnabled)
  policy set to **Disabled**. Supported on Linux; survives restarts; doesn't change
  *which* DNS servers are used — only that Chrome uses the system stack.

A plain Chrome restart *appears* to help because it resets Chrome's resolver
state and cache, but the flakiness returns with async DNS re-enabled.

### Background / references

- [Chromium — Host resolution (`net/dns/README.md`)](https://chromium.googlesource.com/chromium/src/+/main/net/dns/README.md)
- [Understanding systemd-resolved, Split DNS, and VPN configuration — Michael Catanzaro](https://blogs.gnome.org/mcatanzaro/2020/12/17/understanding-systemd-resolved-split-dns-and-vpn-configuration/)
- [Chrome policy — `BuiltInDnsClientEnabled`](https://chromeenterprise.google/policies/#BuiltInDnsClientEnabled)
- [Pi-hole community — Disable Async DNS resolver in Google Chrome](https://discourse.pi-hole.net/t/disable-async-dns-resolver-in-google-chrome/9500)

> Firefox and other glibc-based apps are unaffected — they use the OS resolver and
> honour the split-DNS routing correctly.
