<script lang="ts">
  // settings window — port of gpgui ui/settings.html.
  // DOM structure, class names and inline styles mirror the original so
  // theme.css applies unchanged.
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { m } from '../paraglide/messages.js';
  import { applyChoice, currentChoice, localeOptions, type LocaleChoice } from '../lib/locale';
  import Dropdown from '../lib/Dropdown.svelte';
  import revolutQrUrl from '../revolut-qr.png';
  import ethQrUrl from '../eth-qr.png';

  const hasTauri = '__TAURI_INTERNALS__' in window;

  const REPO_URL = 'https://github.com/techneut92/GlobalProtect-openconnect-dw';
  const UPSTREAM_URL = 'https://github.com/yuezk/GlobalProtect-openconnect';
  const KOFI_URL = 'https://ko-fi.com/techneut92?amount=2.50#checkoutModal';
  const REVOLUT_URL = 'https://revolut.me/techneut92';
  const ETH_ADDR = '0x15d9B8383A7cbe9f99F72aC29106C53bbcf4ea40';

  // ───────── Tauri payload types (derived from actual usage) ─────────
  interface SystemInfo {
    guiVersion: string;
    running: string;
    installKind: string;
    flatpakRuntime?: string | null;
    backendInstalled: boolean;
    backendVersion?: string | null;
    osName: string;
  }
  interface UpdateInfo {
    error?: string | null;
    available?: boolean;
    backendUpdate?: boolean;
    current?: string | null;
    latest?: string | null;
    url?: string | null;
  }
  interface InstallBackendResult {
    ok: boolean;
    needsReboot?: boolean;
    message?: string | null;
  }
  type ConfigMap = Record<string, unknown>;

  // ───────── rail nav ─────────
  const SECTIONS = ['general', 'auth', 'conn', 'support', 'about'] as const;
  type Section = (typeof SECTIONS)[number];
  let active = $state<Section>('general');
  function activate(sec: string): void {
    if ((SECTIONS as readonly string[]).includes(sec)) active = sec as Section;
  }

  function onClose(): void {
    if (hasTauri) void getCurrentWindow().close();
    else window.close();
  }

  function openExt(url: string): void {
    if (hasTauri) void invoke('open_url', { url });
    else window.open(url, '_blank');
  }

  // ───────── form model — camelCase keys match Rust SettingsForm ─────────
  const model = $state({
    authView: 'webview',
    runAtStartup: false,
    startMinimized: false,
    rememberUnlock: false,
    trayIcon: 'shield',
    os: 'Linux',
    osVersion: '',
    userAgent: 'PAN GlobalProtect/6.2.1-1290',
    clientVersion: '',
    mtu: 0,
    reconnectTimeout: 30,
    forceDpd: 0,
    vpncScript: '',
    localHostname: '',
    disableIpv6: false,
    noDtls: false,
    noXmlpost: false,
    ignoreTlsErrors: false,
  });
  type BoolKey =
    | 'runAtStartup'
    | 'startMinimized'
    | 'rememberUnlock'
    | 'disableIpv6'
    | 'noDtls'
    | 'noXmlpost'
    | 'ignoreTlsErrors';

  // Raw text shown in the number inputs (the original never rewrote the
  // field while typing; the model gets the parsed value).
  let mtuText = $state('0');
  let reconnectText = $state('30');
  let forceDpdText = $state('0');

  let savedShow = $state(false);
  let savedTimer: ReturnType<typeof setTimeout> | undefined;
  async function save(): Promise<void> {
    savedShow = true;
    clearTimeout(savedTimer);
    savedTimer = setTimeout(() => {
      savedShow = false;
    }, 1300);
    if (hasTauri) {
      try {
        await invoke('save_settings', { form: $state.snapshot(model) });
      } catch {
        /* ignore */
      }
    }
  }

  function toggleKey(k: BoolKey): void {
    model[k] = !model[k];
    void save();
  }
  function pickTray(v: string): void {
    model.trayIcon = v;
    void save();
  }
  function numInput(raw: string, apply: (n: number) => void): void {
    apply(parseInt(raw, 10) || 0);
    void save();
  }

  // cfg holds snake_case config keys
  function cfgStr(v: unknown): string {
    return v == null ? '' : typeof v === 'string' ? v : String(v);
  }
  function cfgNum(v: unknown): number {
    if (typeof v === 'number' && Number.isFinite(v)) return v;
    if (typeof v === 'string') return parseInt(v, 10) || 0;
    return 0;
  }
  function applyConfig(cfg: ConfigMap): void {
    model.authView = cfgStr(cfg['auth_view']);
    model.runAtStartup = !!cfg['run_at_startup'];
    model.startMinimized = !!cfg['start_minimized'];
    model.rememberUnlock = !!cfg['remember_unlock'];
    model.trayIcon = cfgStr(cfg['tray_icon']);
    model.os = cfgStr(cfg['os']);
    model.osVersion = cfgStr(cfg['os_version']);
    model.userAgent = cfgStr(cfg['user_agent']);
    model.clientVersion = cfgStr(cfg['client_version']);
    model.mtu = cfgNum(cfg['mtu']);
    model.reconnectTimeout = cfgNum(cfg['reconnect_timeout']);
    model.forceDpd = cfgNum(cfg['force_dpd']);
    model.vpncScript = cfgStr(cfg['vpnc_script']);
    model.localHostname = cfgStr(cfg['local_hostname']);
    model.disableIpv6 = !!cfg['disable_ipv6'];
    model.noDtls = !!cfg['no_dtls'];
    model.noXmlpost = !!cfg['no_xmlpost'];
    model.ignoreTlsErrors = !!cfg['ignore_tls_errors'];
    mtuText = String(model.mtu);
    reconnectText = String(model.reconnectTimeout);
    forceDpdText = String(model.forceDpd);
  }

  const ssoOptions = [
    { value: 'webview', label: m.settings_sso_webview(), sub: m.settings_sso_webview_sub() },
    { value: 'browser', label: m.settings_sso_browser(), sub: m.settings_sso_browser_sub() },
  ];
  const osOptions = [
    { value: 'Linux', label: 'Linux' },
    { value: 'Windows', label: 'Windows' },
    { value: 'Mac', label: 'Mac' },
  ];

  // ───────── ETH copy ─────────
  let ethCopyState = $state<'idle' | 'ok' | 'fail'>('idle');
  let ethTimer: ReturnType<typeof setTimeout> | undefined;
  let ethAddrEl = $state<HTMLDivElement | null>(null);
  async function onEthCopy(): Promise<void> {
    let ok = false;
    try {
      await navigator.clipboard.writeText(ETH_ADDR);
      ok = true;
    } catch {
      // Fallback: select the address so the user can copy manually.
      try {
        if (ethAddrEl) {
          const r = document.createRange();
          r.selectNodeContents(ethAddrEl);
          const s = getSelection();
          if (s) {
            s.removeAllRanges();
            s.addRange(r);
          }
          ok = typeof document.execCommand === 'function' && document.execCommand('copy');
        }
      } catch {
        /* ignore */
      }
    }
    ethCopyState = ok ? 'ok' : 'fail';
    clearTimeout(ethTimer);
    ethTimer = setTimeout(() => {
      ethCopyState = 'idle';
    }, 1600);
  }

  // ───────── About / updates ─────────
  type UpdCell =
    | { kind: 'init' }
    | { kind: 'dash' }
    | { kind: 'current' }
    | { kind: 'avail'; latest: string };

  let aboutVer = $state('—');
  let aboutGuiVer = $state('—');
  let aboutRunning = $state('—');
  let runtimeVisible = $state(false);
  let aboutRuntime = $state('—');
  let backendState = $state<'unknown' | 'missing' | 'present'>('unknown');
  let backendText = $state('—');
  let aboutKind = $state('—');
  let aboutOs = $state('—');
  let guiUpd = $state<UpdCell>({ kind: 'init' });
  let beUpd = $state<UpdCell>({ kind: 'init' });
  let aboutHasUpdate = $state(false);

  let updateStatus = $state<string>(m.settings_update_check_prompt());
  let updateUrl = REPO_URL + '/releases';
  let updateLatest = '';
  let updGui = false;
  let updBackend = false;
  let updateAllVisible = $state(false);
  let updating = $state(false);
  let restartVisible = $state(false);
  let restartText = $state(m.settings_restart_now());
  let restartCmd: 'reboot_host' | 'restart_app' = 'restart_app';
  let ubLog = $state('');
  let ubKind = $state('');

  function setUbLog(msg: string, kind: string): void {
    ubLog = msg;
    ubKind = kind;
  }

  async function loadAbout(): Promise<void> {
    if (!hasTauri) {
      aboutVer = m.settings_dev_build();
      return;
    }
    try {
      const s = await invoke<SystemInfo>('system_info');
      aboutVer = 'v' + s.guiVersion;
      // App (frontend)
      aboutGuiVer = 'v' + s.guiVersion;
      aboutRunning = s.running + (s.running === 'Native package' ? ' (' + s.installKind + ')' : '');
      if (s.flatpakRuntime) {
        runtimeVisible = true;
        aboutRuntime = s.flatpakRuntime;
      }
      // Backend
      backendState = s.backendInstalled ? 'present' : 'missing';
      backendText = s.backendVersion ? 'v' + s.backendVersion : m.settings_installed();
      aboutKind = s.installKind;
      // System
      aboutOs = s.osName;

      // Update status — both the app and the backend (they version separately).
      let u: UpdateInfo | null = null;
      try {
        u = await invoke<UpdateInfo>('check_update');
      } catch {
        u = null;
      }
      const err = !!(u && u.error);
      guiUpd = err
        ? { kind: 'dash' }
        : u && u.available
          ? { kind: 'avail', latest: u.latest ?? '' }
          : { kind: 'current' };
      beUpd =
        !s.backendInstalled || err
          ? { kind: 'dash' }
          : u && u.backendUpdate
            ? { kind: 'avail', latest: u.latest ?? '' }
            : { kind: 'current' };
      if (u) {
        updateUrl = u.url || updateUrl;
        updateLatest = u.latest || '';
      }
      // One "Update all" button handles whichever of app/backend is behind.
      updGui = !!(u && u.available);
      updBackend = !!(u && u.backendUpdate);
      updateAllVisible = updGui || updBackend;
      restartVisible = false;
      // One-line summary
      if (err && u) updateStatus = m.settings_update_error({ error: u.error ?? '' });
      else if (u) {
        const parts = [
          u.available
            ? m.settings_update_summary_app_update({ current: u.current ?? '', latest: u.latest ?? '' })
            : m.settings_update_summary_app_latest({ current: u.current ?? '' }),
        ];
        if (s.backendInstalled) {
          const verStr = s.backendVersion ? 'v' + s.backendVersion : '?';
          parts.push(
            u.backendUpdate
              ? m.settings_update_summary_backend_update({ version: verStr, latest: u.latest ?? '' })
              : m.settings_update_summary_backend_latest({ version: verStr }),
          );
        }
        updateStatus = parts.join('  ·  ');
      }
      // Badge the About nav when either the app or the backend is behind.
      aboutHasUpdate = !!(u && (u.available || u.backendUpdate));
    } catch {
      /* ignore */
    }
  }

  async function onUpdateAll(): Promise<void> {
    if (!hasTauri) {
      openExt(updateUrl);
      return;
    }
    updating = true;
    restartVisible = false;
    // Run a step while narrating the phase with a live elapsed counter.
    const step = async <T,>(phase: string, fn: () => Promise<T>): Promise<T> => {
      let n = 0;
      setUbLog(phase, '');
      const t = setInterval(() => {
        n++;
        setUbLog(m.settings_update_elapsed({ phase, seconds: String(n) }), '');
      }, 1000);
      try {
        return await fn();
      } finally {
        clearInterval(t);
      }
    };
    const done: string[] = [];
    let failed = false;
    let needsReboot = false;
    let guiUpdated = false;
    // Backend first (host package / rpm-ostree layer), then the app.
    if (updBackend) {
      try {
        const r = await step(m.settings_backend_updating({ version: updateLatest }), () =>
          invoke<InstallBackendResult>('install_backend', { version: updateLatest }),
        );
        if (r.ok) {
          done.push(
            r.needsReboot
              ? m.settings_backend_updated_reboot({ version: updateLatest })
              : m.settings_backend_updated({ version: updateLatest }),
          );
          needsReboot = !!r.needsReboot;
        } else {
          setUbLog(m.settings_backend_update_failed({ error: r.message || m.settings_unknown_error() }), 'err');
          failed = true;
        }
      } catch (e) {
        setUbLog(m.settings_backend_update_failed({ error: String(e) }), 'err');
        failed = true;
      }
    }
    if (!failed && updGui) {
      try {
        const msg = await step(m.settings_gui_downloading({ version: updateLatest }), () =>
          invoke<string>('run_update', { url: updateUrl, version: updateLatest }),
        );
        done.push(msg);
        guiUpdated = true;
      } catch (e) {
        setUbLog(m.settings_gui_update_failed({ error: String(e) }), 'err');
        failed = true;
      }
    }
    updating = false;
    if (!failed) {
      setUbLog(done.join('  ·  ') || m.settings_already_up_to_date(), 'ok');
      // Offer the one action that actually applies the update.
      if (needsReboot) {
        restartText = m.settings_reboot_now();
        restartCmd = 'reboot_host';
        restartVisible = true;
        updateAllVisible = false;
      } else if (guiUpdated) {
        restartText = m.settings_restart_app();
        restartCmd = 'restart_app';
        restartVisible = true;
        updateAllVisible = false;
      }
    }
  }

  function onRestart(): void {
    void invoke(restartCmd);
  }

  async function onCheckUpdate(): Promise<void> {
    if (!hasTauri) {
      openExt(updateUrl);
      return;
    }
    updateStatus = m.settings_checking();
    await loadAbout();
  }

  onMount(() => {
    let unlisten: UnlistenFn | undefined;
    // Deep-link from another window (e.g. the main screen's "update available"
    // banner → About) when this window is already open.
    if (hasTauri) {
      void listen<string>('goto-section', (e) => {
        if (e.payload) activate(e.payload);
      }).then((f) => {
        unlisten = f;
      });
    }
    void (async () => {
      if (hasTauri) {
        try {
          applyConfig(await invoke<ConfigMap>('get_config'));
        } catch {
          /* ignore */
        }
      }
      void loadAbout();
      // Cold-open deep-link: open_settings stashes the section in a global.
      const g = (window as Window & { __gotoSection?: unknown }).__gotoSection;
      if (typeof g === 'string' && g) activate(g);
    })();
    return () => {
      unlisten?.();
    };
  });
</script>

<header data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#6fb6ff" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3.2"/><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/></svg>
    <div class="title">{m.settings_title()}</div>
  </div>
  <div class="winctl">
    <button class="wbtn close" id="winClose" title={m.common_close()} onclick={onClose}>
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M5 5l14 14M19 5L5 19"/></svg>
    </button>
  </div>
</header>

<div class="settings-layout">
  <nav class="settings-rail">
    <button class="settings-nav" class:active={active === 'general'} data-sec="general" onclick={() => activate('general')}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h9M19 7h1M4 17h5M15 17h5"/><circle cx="15" cy="7" r="2"/><circle cx="11" cy="17" r="2"/></svg>
      <span>{m.settings_nav_general()}</span>
    </button>
    <button class="settings-nav" class:active={active === 'auth'} data-sec="auth" onclick={() => activate('auth')}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="10" width="16" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/></svg>
      <span>{m.settings_nav_auth()}</span>
    </button>
    <button class="settings-nav" class:active={active === 'conn'} data-sec="conn" onclick={() => activate('conn')}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5a7 7 0 0 1 14 0"/><path d="M8.5 15a3.5 3.5 0 0 1 7 0"/><circle cx="12" cy="18.5" r="1.3" fill="currentColor" stroke="none"/></svg>
      <span>{m.settings_nav_connection()}</span>
    </button>
    <button class="settings-nav" class:active={active === 'support'} data-sec="support" onclick={() => activate('support')}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M19 14c1.5-1.5 3-3.3 3-5.5A3.5 3.5 0 0 0 18.5 5c-1.3 0-2.6.7-3.5 2-.9-1.3-2.2-2-3.5-2A3.5 3.5 0 0 0 8 8.5c0 2.2 1.5 4 3 5.5l4 4z"/></svg>
      <span>{m.settings_nav_support()}</span>
    </button>
    <button class="settings-nav" class:active={active === 'about'} class:has-update={aboutHasUpdate} data-sec="about" onclick={() => activate('about')}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M12 16v-4M12 8h.01"/></svg>
      <span>{m.settings_nav_about()}</span>
    </button>
  </nav>

  <div class="settings-content">
    <!-- GENERAL -->
    <div class="settings-sec" id="sec-general" hidden={active !== 'general'}>
      <h2>{m.settings_nav_general()}</h2>
      <p class="desc">{m.settings_general_desc()}</p>

      <div class="toggle-list" style="margin-top:0;">
        <div class="toggle-item">
          <div><div class="t">{m.settings_run_at_startup()}</div><div class="d">{m.settings_run_at_startup_desc()}</div></div>
          <button class="switch" class:on={model.runAtStartup} type="button" data-key="runAtStartup" aria-label={m.settings_run_at_startup()} onclick={() => toggleKey('runAtStartup')}><span class="knob"></span></button>
        </div>
        <div class="toggle-item">
          <div><div class="t">{m.settings_start_minimized()}</div><div class="d">{m.settings_start_minimized_desc()}</div></div>
          <button class="switch" class:on={model.startMinimized} type="button" data-key="startMinimized" aria-label={m.settings_start_minimized()} onclick={() => toggleKey('startMinimized')}><span class="knob"></span></button>
        </div>
        <div class="toggle-item">
          <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;">{m.language_label()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.language_system()} · English · Nederlands · Frysk</div></div>
          <div style="min-width:180px;"><Dropdown options={localeOptions()} value={currentChoice()} onChange={(v) => applyChoice(v as LocaleChoice)} /></div>
        </div>
        <div class="toggle-item">
          <div><div class="t">{m.settings_remember_unlock()}</div><div class="d">{m.settings_remember_unlock_desc()}</div></div>
          <button class="switch" class:on={model.rememberUnlock} type="button" data-key="rememberUnlock" aria-label={m.settings_remember_unlock()} onclick={() => toggleKey('rememberUnlock')}><span class="knob"></span></button>
        </div>
      </div>

      <div class="lbl" style="margin-top:18px;">{m.settings_tray_icon()}</div>
      <div class="tray-grid">
        <button class="tray-card" class:sel={(model.trayIcon || 'shield') === 'shield'} type="button" data-tray="shield" onclick={() => pickTray('shield')}>
          <div class="tray-icons">
            <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><path d="M12 2.5 L20 5.2 V12 C20 17.2 16.5 21 12 22.5 C7.5 21 4 17.2 4 12 V5.2 Z" fill="#6b7286"/><path d="M12 8.6 V12.4 M12 15.7 h0.01" stroke="#0a0c12" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><path d="M12 2.5 L20 5.2 V12 C20 17.2 16.5 21 12 22.5 C7.5 21 4 17.2 4 12 V5.2 Z" fill="#fbbf24"/><circle cx="8.7" cy="12.7" r="1.15" fill="#0a0c12"/><circle cx="12" cy="12.7" r="1.15" fill="#0a0c12"/><circle cx="15.3" cy="12.7" r="1.15" fill="#0a0c12"/></svg>
            <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><path d="M12 2.5 L20 5.2 V12 C20 17.2 16.5 21 12 22.5 C7.5 21 4 17.2 4 12 V5.2 Z" fill="#34d399"/><path d="M8.4 12.2 l2.5 2.5 l4.7-4.9" stroke="#0a0c12" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </div>
          <div class="tray-foot"><span>{m.settings_tray_shield()}</span><span class="check"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#06121f" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 6"/></svg></span></div>
        </button>
        <button class="tray-card" class:sel={(model.trayIcon || 'shield') === 'ring'} type="button" data-tray="ring" onclick={() => pickTray('ring')}>
          <div class="tray-icons">
            <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><path d="M7.4 17.6 A7 7 0 1 1 16.6 17.6" stroke="#6b7286" stroke-width="2.1" stroke-linecap="round" fill="none"/><circle cx="12" cy="12" r="1.6" fill="#6b7286"/></svg>
            <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="7" stroke="#fbbf24" stroke-width="2.1" fill="none" opacity="0.28"/><path d="M12 5 A7 7 0 0 1 19 12" stroke="#fbbf24" stroke-width="2.1" stroke-linecap="round" fill="none"/><circle cx="12" cy="12" r="1.6" fill="#fbbf24"/></svg>
            <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="7" stroke="#34d399" stroke-width="2.1" fill="none"/><circle cx="12" cy="12" r="5" fill="#34d399"/><path d="M9.4 12.1 l1.9 1.9 l3.4-3.6" stroke="#0a0c12" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" fill="none"/></svg>
          </div>
          <div class="tray-foot"><span>{m.settings_tray_ring()}</span><span class="check"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#06121f" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 6"/></svg></span></div>
        </button>
      </div>
      <p class="help">{m.settings_tray_help()}</p>
    </div>

    <!-- AUTH -->
    <div class="settings-sec" id="sec-auth" hidden={active !== 'auth'}>
      <h2>{m.settings_nav_auth()}</h2>
      <p class="desc">{m.settings_auth_desc()}</p>
      <div class="lbl">{m.settings_sso_method()}</div>
      <div id="ssoDD"><Dropdown options={ssoOptions} bind:value={model.authView} placeholder={m.common_select()} onChange={() => void save()} /></div>
      <p class="help">{m.settings_sso_help()}</p>
    </div>

    <!-- CONNECTION -->
    <div class="settings-sec" id="sec-conn" hidden={active !== 'conn'}>
      <h2>{m.settings_nav_connection()}</h2>
      <p class="desc">{m.settings_conn_desc()}</p>

      <div class="grid2 mb14">
        <div><div class="lbl">{m.settings_lbl_os()}</div><div id="osDD"><Dropdown options={osOptions} bind:value={model.os} placeholder={m.common_select()} onChange={() => void save()} /></div></div>
        <div><div class="lbl">{m.settings_lbl_os_version()}</div><input class="field" id="osVersion" placeholder={m.settings_ph_auto()} value={model.osVersion} oninput={(e) => { model.osVersion = e.currentTarget.value; void save(); }} /></div>
      </div>

      <div class="lbl">{m.settings_lbl_user_agent()}</div>
      <input class="field mb14" id="userAgent" value={model.userAgent} oninput={(e) => { model.userAgent = e.currentTarget.value; void save(); }} />

      <div class="lbl">{m.settings_lbl_client_version()}</div>
      <input class="field mb16" id="clientVersion" placeholder={m.settings_ph_default()} value={model.clientVersion} oninput={(e) => { model.clientVersion = e.currentTarget.value; void save(); }} />

      <div class="grid3 mb16">
        <div><div class="lbl">{m.settings_lbl_mtu()}</div><input class="field" id="mtu" type="number" min="0" value={mtuText} oninput={(e) => { mtuText = e.currentTarget.value; numInput(mtuText, (n) => { model.mtu = n; }); }} /></div>
        <div><div class="lbl">{m.settings_lbl_reconnect()}</div><input class="field" id="reconnectTimeout" type="number" min="0" value={reconnectText} oninput={(e) => { reconnectText = e.currentTarget.value; numInput(reconnectText, (n) => { model.reconnectTimeout = n; }); }} /></div>
        <div><div class="lbl">{m.settings_lbl_force_dpd()}</div><input class="field" id="forceDpd" type="number" min="0" value={forceDpdText} oninput={(e) => { forceDpdText = e.currentTarget.value; numInput(forceDpdText, (n) => { model.forceDpd = n; }); }} /></div>
      </div>

      <div class="lbl">{m.settings_lbl_vpnc_script()}</div>
      <input class="field mb14" id="vpncScript" placeholder="/usr/libexec/openconnect/vpnc-script" value={model.vpncScript} oninput={(e) => { model.vpncScript = e.currentTarget.value; void save(); }} />

      <div class="lbl">{m.settings_lbl_local_hostname()}</div>
      <input class="field mb18" id="localHostname" placeholder={m.settings_ph_auto()} value={model.localHostname} oninput={(e) => { model.localHostname = e.currentTarget.value; void save(); }} />

      <div class="toggle-list">
        <div class="toggle-item">
          <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;">{m.settings_disable_ipv6()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.settings_disable_ipv6_desc()}</div></div>
          <button class="switch" class:on={model.disableIpv6} type="button" data-key="disableIpv6" aria-label={m.settings_disable_ipv6()} onclick={() => toggleKey('disableIpv6')}><span class="knob"></span></button>
        </div>
        <div class="toggle-item">
          <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;">{m.settings_no_dtls()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.settings_no_dtls_desc()}</div></div>
          <button class="switch" class:on={model.noDtls} type="button" data-key="noDtls" aria-label={m.settings_no_dtls()} onclick={() => toggleKey('noDtls')}><span class="knob"></span></button>
        </div>
        <div class="toggle-item">
          <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;">{m.settings_no_xmlpost()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.settings_no_xmlpost_desc()}</div></div>
          <button class="switch" class:on={model.noXmlpost} type="button" data-key="noXmlpost" aria-label={m.settings_no_xmlpost()} onclick={() => toggleKey('noXmlpost')}><span class="knob"></span></button>
        </div>
        <div class="toggle-item">
          <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;">{m.settings_ignore_tls()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.settings_ignore_tls_desc()}</div></div>
          <button class="switch" class:on={model.ignoreTlsErrors} type="button" data-key="ignoreTlsErrors" aria-label={m.settings_ignore_tls()} onclick={() => toggleKey('ignoreTlsErrors')}><span class="knob"></span></button>
        </div>
      </div>
    </div>

    <!-- SUPPORT -->
    <div class="settings-sec" id="sec-support" hidden={active !== 'support'}>
      <h2>{m.settings_nav_support()}</h2>
      <p class="desc">{m.settings_support_desc()}</p>
      <div class="kofi-card">
        <div class="kofi-head">
          <div class="kofi-logo">
            <svg width="27" height="27" viewBox="0 0 24 24" fill="none"><path d="M5 5h12a4 4 0 0 1 0 8h-1.2A6 6 0 0 1 10 18H8a4 4 0 0 1-4-4V5z" fill="#FF5E5B"/><circle cx="9" cy="10" r="2.1" fill="#fff"/><path d="M17 7h.5a2 2 0 0 1 0 4H17" stroke="#FF5E5B" stroke-width="1.6"/></svg>
          </div>
          <div>
            <div class="kofi-title">{m.settings_kofi_title()}</div>
            <div class="kofi-sub">{m.settings_kofi_sub()}</div>
          </div>
        </div>
        <a class="kofi-btn" id="kofiBtn" href={KOFI_URL} target="_blank" rel="noreferrer" onclick={(e) => { if (hasTauri) { e.preventDefault(); void invoke('open_url', { url: KOFI_URL }); } }}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none"><path d="M5 5h12a4 4 0 0 1 0 8h-1.2A6 6 0 0 1 10 18H8a4 4 0 0 1-4-4V5z" fill="#fff"/><circle cx="9" cy="10" r="2.1" fill="#FF5E5B"/></svg>
          <span>{m.settings_kofi_btn()}</span>
        </a>
      </div>
      <div class="revolut-card">
        <div class="kofi-head">
          <div class="revolut-logo">
            <svg width="26" height="26" viewBox="0 0 24 24"><text x="12" y="17.5" text-anchor="middle" font-family="Arial, Helvetica, sans-serif" font-size="17" font-weight="800" fill="#0666EB">R</text></svg>
          </div>
          <div>
            <div class="kofi-title">{m.settings_revolut_title()}</div>
            <div class="kofi-sub">{m.settings_revolut_sub()}</div>
          </div>
        </div>
        <div class="revolut-qr-wrap">
          <img class="revolut-qr" src={revolutQrUrl} width="150" height="150" alt={m.settings_revolut_qr_alt()} />
          <div class="revolut-qr-cap">{m.settings_revolut_qr_cap()}</div>
        </div>
        <a class="revolut-btn" id="revolutBtn" href={REVOLUT_URL} target="_blank" rel="noreferrer" onclick={(e) => { if (hasTauri) { e.preventDefault(); void invoke('open_url', { url: REVOLUT_URL }); } }}>
          <svg width="18" height="18" viewBox="0 0 24 24"><text x="12" y="17.5" text-anchor="middle" font-family="Arial, Helvetica, sans-serif" font-size="17" font-weight="800" fill="#fff">R</text></svg>
          <span>{m.settings_revolut_btn()}</span>
        </a>
      </div>
      <div class="eth-card">
        <div class="kofi-head">
          <div class="eth-logo">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none"><path d="M12 2 L12 9.6 L18.5 12.5 Z" fill="#8fa2f5"/><path d="M12 2 L5.5 12.5 L12 9.6 Z" fill="#c5cff9"/><path d="M12 16.1 L12 22 L18.5 13.7 Z" fill="#8fa2f5"/><path d="M12 22 L12 16.1 L5.5 13.7 Z" fill="#c5cff9"/><path d="M12 14.9 L18.5 12.5 L12 9.6 Z" fill="#627EEA"/><path d="M5.5 12.5 L12 14.9 L12 9.6 Z" fill="#8295ef"/></svg>
          </div>
          <div>
            <div class="kofi-title">{m.settings_eth_title()}</div>
            <div class="kofi-sub">{m.settings_eth_sub()}</div>
          </div>
        </div>
        <div class="eth-qr-wrap">
          <img class="eth-qr" src={ethQrUrl} width="150" height="150" alt={m.settings_eth_qr_alt()} />
          <div class="revolut-qr-cap">{m.settings_eth_qr_cap()}</div>
        </div>
        <div class="eth-addr-row">
          <div class="eth-addr" id="ethAddr" title={ETH_ADDR} bind:this={ethAddrEl}>{ETH_ADDR}</div>
          <button class="eth-copy" class:copied={ethCopyState === 'ok'} id="ethCopy" type="button" onclick={onEthCopy}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></svg>
            <span id="ethCopyLabel">{ethCopyState === 'ok' ? m.settings_eth_copied() : ethCopyState === 'fail' ? m.settings_eth_select_copy() : m.settings_eth_copy()}</span>
          </button>
        </div>
        <div class="eth-net">{m.settings_eth_net()}</div>
      </div>
      <p class="help">{m.settings_support_help()}</p>
    </div>

    <!-- ABOUT -->
    <div class="settings-sec" id="sec-about" hidden={active !== 'about'}>
      <h2>{m.settings_nav_about()}</h2>
      <p class="desc">{m.settings_about_desc()}</p>

      <div class="about-app">
        <div class="about-icon">
          <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l7 3v5c0 4.4-3 7.4-7 9-4-1.6-7-4.6-7-9V6l7-3z"/></svg>
        </div>
        <div>
          <div class="about-name">GP <span style="color:var(--accent-soft)">Client</span></div>
          <div class="about-ver" id="aboutVer">{aboutVer}</div>
        </div>
      </div>

      <div class="update-box">
        <div class="update-status" id="updateStatus">{updateStatus}</div>
        <div class="update-actions">
          <button class="btn-soft" id="checkUpdateBtn" onclick={onCheckUpdate}>{m.settings_check_updates()}</button>
          <button class="btn-action auto" id="updateAllBtn" hidden={!updateAllVisible} disabled={updating} style="padding:0 16px;" onclick={onUpdateAll}>{updating ? m.settings_updating() : m.settings_update_all()}</button>
          <button class="btn-action auto" id="restartBtn" hidden={!restartVisible} style="padding:0 16px;" onclick={onRestart}>{restartText}</button>
        </div>
        <div class={ubKind ? `bk-status ${ubKind}` : 'bk-status'} id="updateBackendLog" hidden={!ubLog} style="margin-top:8px;">{ubLog}</div>
      </div>

      <div class="lbl" style="margin-top:18px;">{m.settings_app_section()}</div>
      <div class="detail-card">
        <div class="drow"><div class="k">{m.settings_lbl_version()}</div><div class="v" id="aboutGuiVer">{aboutGuiVer}</div></div>
        <div class="drow"><div class="k">{m.settings_running_as()}</div><div class="v" id="aboutRunning">{aboutRunning}</div></div>
        <div class="drow" id="aboutRuntimeRow" hidden={!runtimeVisible}><div class="k">{m.settings_flatpak_runtime()}</div><div class="v" id="aboutRuntime">{aboutRuntime}</div></div>
        <div class="drow"><div class="k">{m.settings_updates()}</div><div class="v" id="aboutGuiUpd">{#if guiUpd.kind === 'init'}—{:else if guiUpd.kind === 'dash'}<span style="color:var(--muted)">—</span>{:else if guiUpd.kind === 'current'}<span style="color:var(--muted)">{m.settings_up_to_date()}</span>{:else}<span style="color:var(--green)">{m.settings_update_available({ version: guiUpd.latest })}</span>{/if}</div></div>
      </div>

      <div class="lbl" style="margin-top:18px;">{m.settings_backend_section()}</div>
      <div class="detail-card">
        <div class="drow"><div class="k">{m.settings_lbl_version()}</div><div class="v" id="aboutBackend">{#if backendState === 'unknown'}—{:else if backendState === 'missing'}<span style="color:var(--red)">{m.settings_not_installed()}</span>{:else}{backendText}{/if}</div></div>
        <div class="drow"><div class="k">{m.settings_install_type()}</div><div class="v" id="aboutKind">{aboutKind}</div></div>
        <div class="drow"><div class="k">{m.settings_updates()}</div><div class="v" id="aboutBeUpd">{#if beUpd.kind === 'init'}—{:else if beUpd.kind === 'dash'}<span style="color:var(--muted)">—</span>{:else if beUpd.kind === 'current'}<span style="color:var(--muted)">{m.settings_up_to_date()}</span>{:else}<span style="color:var(--green)">{m.settings_update_available({ version: beUpd.latest })}</span>{/if}</div></div>
      </div>

      <div class="lbl" style="margin-top:18px;">{m.settings_system_section()}</div>
      <div class="detail-card">
        <div class="drow"><div class="k">{m.settings_lbl_os()}</div><div class="v" id="aboutOs">{aboutOs}</div></div>
      </div>

      <p class="help">
        {m.settings_footer_copyright()}
        <!-- svelte-ignore a11y_invalid_attribute -->
        <a href="#" id="upstreamLink" onclick={(e) => { e.preventDefault(); openExt(UPSTREAM_URL); }}>yuezk/GlobalProtect-openconnect</a>.
        <!-- svelte-ignore a11y_invalid_attribute -->
        <a href="#" id="repoLink" onclick={(e) => { e.preventDefault(); openExt(REPO_URL + '/releases'); }}>{m.settings_footer_project_page()}</a>
      </p>
    </div>

    <span class="saved" class:show={savedShow} id="saved">
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#06281c" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 6"/></svg>
      <span>{m.settings_saved()}</span>
    </span>
  </div>
</div>
