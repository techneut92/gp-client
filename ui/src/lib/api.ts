// Shared typed API layer over the Tauri backend.
//
// Every window calls the backend through these wrappers so the invoke command
// names and payload/response shapes live in exactly one place. Command and
// event names must stay in sync with src-tauri; do not rename them here.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type { UnlistenFn };

/** Open a URL externally via the backend. */
export function openExt(url: string): void {
  void openUrl(url);
}

// ───────── payload shapes (as read from / sent to the Rust side) ─────────

export interface Identity {
  name: string;
  portal?: string;
  auth_method?: number;
  as_gateway?: boolean;
  module_path?: string;
  username?: string;
  password?: string;
  pin?: string;
  cert_id?: string;
  cert_manufacturer?: string;
  /** Smart-card: true → cert not stored, pick it at connect. Default false. */
  ask_cert?: boolean;
  /** Smart-card: true → PIN not stored, enter it at connect. Default false. */
  ask_pin?: boolean;
  cert_file?: string;
  key_file?: string;
  key_password?: string;
  /** Scoped-DNS opt-in: domains that resolve through the VPN (empty = all DNS). */
  dns_domains?: string[];
  // ---- SSO + per-connection tuning (snake_case, matches Rust Identity) ----
  /** SSO method: "webview" (embedded) or "browser" (system browser). */
  auth_view?: string;
  os?: string;
  os_version?: string;
  user_agent?: string;
  client_version?: string;
  mtu?: number;
  reconnect_timeout?: number;
  force_dpd?: number;
  vpnc_script?: string;
  local_hostname?: string;
  disable_ipv6?: boolean;
  no_dtls?: boolean;
  no_xmlpost?: boolean;
  ignore_tls_errors?: boolean;
}

export interface CertInfo {
  uri: string;
  display: string;
  id: string;
  manufacturer: string;
  slot: string;
  expiry: string;
}

export interface InstallStep {
  label: string;
  cmd: string;
}

export interface InstallOption {
  kind: string;
  label: string;
  steps: InstallStep[];
  note?: string;
}

export interface SystemInfo {
  guiVersion: string;
  running: string;
  installKind: string;
  flatpakRuntime?: string | null;
  backendInstalled: boolean;
  /** False when an installed backend is older than the minimum gp-client supports
   *  — routed to the install/upgrade screen just like a missing backend. */
  backendSupported: boolean;
  /** True when the backend speaks a newer wire protocol than this GUI — routed to
   *  the "Update GP Client" screen (the inverse of backendSupported). */
  backendTooNew: boolean;
  /** Manual "update this app" command, matched to how the GUI was installed. */
  guiUpdateCmd: string;
  backendVersion?: string | null;
  installOptions?: InstallOption[];
  osName: string;
}

export interface UpdateInfo {
  error?: string | null;
  available?: boolean;
  backendUpdate?: boolean;
  current?: string | null;
  latest?: string | null;
  backendCurrent?: string | null;
  backendLatest?: string | null;
  url?: string | null;
}

export interface InstallResult {
  ok: boolean;
  needsReboot?: boolean;
  message?: string | null;
}

export interface VaultStatus {
  exists: boolean;
  unlocked: boolean;
}

export interface VpnState {
  kind?: number;
  status?: string;
  active?: boolean;
  log?: string;
  ip?: string;
  iface?: string;
  portal?: string;
  gateway?: string;
  expires?: string;
  elapsed?: string;
  // Interactive challenges surfaced mid-connect. camelCase — the Rust
  // StatePayload serializes with rename_all = "camelCase". While set, kind stays
  // 1 (connecting). pinRequired is driven gp-client-side; the mfa* fields need
  // the backend to surface the challenge (GPS-16).
  mfaRequired?: boolean;
  /** "totp" | "sms" | "push". */
  mfaMethod?: string;
  mfaPrompt?: string;
  pinRequired?: boolean;
  /** Smart-card manufacturer (subtitle + card-block label); "" → generic. */
  pinPrompt?: string;
  /** PKCS#11 module file name (card-block sub-line); "" → "PKCS#11 token". */
  pinModule?: string;
  /** Mid-connect gateway picker (the portal offered several gateways). */
  gwRequired?: boolean;
  gwList?: { name: string; host: string }[];
  /** Address of the region-preferred gateway (pre-select it). */
  gwPreferred?: string;
}

export interface ProbeResult {
  kind: string;
  message?: string;
  usernameLabel?: string;
  passwordLabel?: string;
  supportsBrowser?: boolean;
}

export interface ProbeForm {
  portal: string;
  certKind: number;
  certUri: string;
  pin: string;
  certFile: string;
  keyFile: string;
  keyPassword: string;
  modulePath: string;
  asGateway: boolean;
  /** Reported client identity for the probe (per-identity editor sends these). */
  os?: string;
  userAgent?: string;
}

// camelCase keys match the Rust SettingsForm. General (startup + tray) only —
// connection/SSO options are edited per identity now (see the identity editor).
export interface SettingsForm {
  runAtStartup: boolean;
  startMinimized: boolean;
  rememberUnlock: boolean;
  trayIcon: string;
}

/** Keys of SettingsForm whose value is a boolean (the toggle switches). */
export type SettingsBoolKey = {
  [K in keyof SettingsForm]: SettingsForm[K] extends boolean ? K : never;
}[keyof SettingsForm];

/** The stored config uses snake_case keys; only module_path is read typed. */
export interface ConfigMap {
  module_path?: string | null;
  [key: string]: unknown;
}

// ───────── commands ─────────

export function systemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>('system_info');
}

export function checkUpdate(): Promise<UpdateInfo> {
  return invoke<UpdateInfo>('check_update');
}

export function installBackend(args: { kind?: string; version?: string }): Promise<InstallResult> {
  return invoke<InstallResult>('install_backend', args);
}

export function runUpdate(url: string, version: string): Promise<string> {
  return invoke<string>('run_update', { url, version });
}

export function restart(cmd: 'reboot_host' | 'restart_app'): Promise<void> {
  return invoke<void>(cmd);
}

export function openSettings(section?: string): Promise<void> {
  return section === undefined ? invoke<void>('open_settings') : invoke<void>('open_settings', { section });
}

/** Open the single-identity editor window. `name` = edit that identity; omit to
 *  create a new one. */
export function openIdentityEditor(name?: string): Promise<void> {
  return name === undefined ? invoke<void>('open_identity_editor') : invoke<void>('open_identity_editor', { name });
}

export function openUrl(url: string): Promise<void> {
  return invoke<void>('open_url', { url });
}

export function getConfig(): Promise<ConfigMap> {
  return invoke<ConfigMap>('get_config');
}

export function saveSettings(form: SettingsForm): Promise<void> {
  return invoke<void>('save_settings', { form });
}

export function getState(): Promise<VpnState> {
  return invoke<VpnState>('get_state');
}

export function connect(identity: string, portal: string): Promise<void> {
  return invoke<void>('connect', { identity, portal });
}

export function disconnect(): Promise<void> {
  return invoke<void>('disconnect');
}

/** Answer a mid-connect MFA challenge with a one-time code (TOTP/SMS/RSA
 *  token) — resolves the backend's parked `MfaChallenge` prompt (GPS-16). */
export function submitMfa(code: string): Promise<void> {
  return invoke<void>('submit_mfa', { code });
}

/** Answer the mid-connect gateway picker with the chosen gateway's address. */
export function selectGateway(gateway: string): Promise<void> {
  return invoke<void>('select_gateway', { gateway });
}

/** Time a TCP handshake to the gateway's TLS port, in ms (picker latency hint). */
export function pingGateway(host: string): Promise<number> {
  return invoke<number>('ping_gateway', { host });
}

/** Ask the backend to re-send the MFA challenge (new code / re-push). */
export function resendMfa(): Promise<void> {
  return invoke<void>('resend_mfa');
}

/** Answer a mid-connect smart-card PIN prompt, with the cert chosen in the
 *  connect-time picker (its `pkcs11:` URI; omit to keep the identity's stored cert). */
export function submitPin(pin: string, certUri?: string): Promise<void> {
  return invoke<void>('submit_pin', { pin, certUri });
}

export function vaultStatus(): Promise<VaultStatus> {
  return invoke<VaultStatus>('vault_status');
}

export function keyringAvailable(): Promise<boolean> {
  return invoke<boolean>('keyring_available');
}

/** Whether to show the "Import from GP Client" migration screen (fresh install
 *  with a predecessor gpgui present). */
export function importAvailable(): Promise<boolean> {
  return invoke<boolean>('import_available');
}

/** Import everything from gpgui (identities + settings, incl. auto-unlock), then
 *  remove the old app and its data. Rejects if the import failed. */
export function importFromGpgui(): Promise<void> {
  return invoke<void>('import_from_gpgui');
}

/** Whether the predecessor gpgui *app* is still installed (vs. only leftover
 *  data) — drives the import-screen button copy/behaviour. */
export function predecessorAppInstalled(): Promise<boolean> {
  return invoke<boolean>('predecessor_app_installed');
}

/** Remove the predecessor gpgui and its data WITHOUT importing — the "remove old
 *  app / start fresh" path on the migration screen. */
export function removePredecessor(): Promise<void> {
  return invoke<void>('remove_predecessor');
}

export function setRememberUnlock(enabled: boolean): Promise<void> {
  return invoke<void>('set_remember_unlock', { enabled });
}

export function setMasterPin(pin: string): Promise<void> {
  return invoke<void>('set_master_pin', { pin });
}

export function unlockVault(pin: string): Promise<void> {
  return invoke<void>('unlock_vault', { pin });
}

export function resetVault(): Promise<void> {
  return invoke<void>('reset_vault');
}

export function listIdentities(): Promise<Identity[]> {
  return invoke<Identity[]>('list_identities');
}

export function saveIdentity(identity: Identity): Promise<void> {
  return invoke<void>('save_identity', { identity });
}

export function deleteIdentity(name: string): Promise<void> {
  return invoke<void>('delete_identity', { name });
}

export function scanCerts(module: string | undefined): Promise<CertInfo[]> {
  return invoke<CertInfo[]>('scan_certs', { module });
}

export function availableModules(): Promise<string[]> {
  return invoke<string[]>('available_modules');
}

export function browseFile(title: string): Promise<string | null> {
  return invoke<string | null>('browse_file', { title });
}

export function probeAuth(form: ProbeForm): Promise<ProbeResult> {
  return invoke<ProbeResult>('probe_auth', { form });
}

// ───────── events ─────────

export function onVpnState(cb: (s: VpnState) => void): Promise<UnlistenFn> {
  return listen<VpnState>('state', (e) => cb(e.payload));
}

export function onIdentitiesChanged(cb: () => void): Promise<UnlistenFn> {
  return listen('identities-changed', () => cb());
}

export function onGotoSection(cb: (section: string) => void): Promise<UnlistenFn> {
  return listen<string>('goto-section', (e) => cb(e.payload));
}
