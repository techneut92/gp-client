// Shared typed API layer over the Tauri backend.
//
// Every window calls the backend through these wrappers so the invoke command
// names and payload/response shapes live in exactly one place. Command and
// event names must stay in sync with src-tauri; do not rename them here.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type { UnlistenFn };

/** True when running inside the Tauri shell (vs a plain-browser demo). */
export const hasTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Open a URL externally: via the backend on Tauri, a new tab otherwise. */
export function openExt(url: string): void {
  if (hasTauri) void openUrl(url);
  else window.open(url, '_blank');
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
  cert_file?: string;
  key_file?: string;
  key_password?: string;
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
}

// camelCase keys match the Rust SettingsForm.
export interface SettingsForm {
  authView: string;
  runAtStartup: boolean;
  startMinimized: boolean;
  rememberUnlock: boolean;
  trayIcon: string;
  os: string;
  osVersion: string;
  userAgent: string;
  clientVersion: string;
  mtu: number;
  reconnectTimeout: number;
  forceDpd: number;
  vpncScript: string;
  localHostname: string;
  disableIpv6: boolean;
  noDtls: boolean;
  noXmlpost: boolean;
  ignoreTlsErrors: boolean;
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

export function openManager(): Promise<void> {
  return invoke<void>('open_manager');
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

export function vaultStatus(): Promise<VaultStatus> {
  return invoke<VaultStatus>('vault_status');
}

export function keyringAvailable(): Promise<boolean> {
  return invoke<boolean>('keyring_available');
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
