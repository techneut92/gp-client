<script lang="ts">
  // settings window — port of gpgui ui/settings.html.
  // App.svelte is the tab shell (header + rail + saved badge); each section
  // lives in its own component. DOM structure, class names and inline styles
  // mirror the original so theme.css applies unchanged.
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { m } from '../paraglide/messages.js';
  import { getConfig, onGotoSection, saveSettings, type ConfigMap, type SettingsForm, type UnlistenFn } from '../lib/api';
  import { mountShim } from '../lib/shim';
  import GeneralTab from './GeneralTab.svelte';
  import AuthTab from './AuthTab.svelte';
  import ConnectionTab from './ConnectionTab.svelte';
  import SupportCards from './SupportCards.svelte';
  import AboutTab from './AboutTab.svelte';

  mountShim('settings');

  // ───────── rail nav ─────────
  const SECTIONS = ['general', 'auth', 'conn', 'support', 'about'] as const;
  type Section = (typeof SECTIONS)[number];
  let active = $state<Section>('general');
  function activate(sec: string): void {
    if ((SECTIONS as readonly string[]).includes(sec)) active = sec as Section;
  }

  function onClose(): void {
    void getCurrentWindow().close();
  }

  // ───────── form model — camelCase keys match Rust SettingsForm ─────────
  let model = $state<SettingsForm>({
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
    try {
      await saveSettings($state.snapshot(model));
    } catch {
      /* ignore */
    }
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

  // ───────── About / updates (state lives in AboutTab) ─────────
  let aboutHasUpdate = $state(false);
  let aboutRef = $state<ReturnType<typeof AboutTab> | undefined>(undefined);

  onMount(() => {
    let unlisten: UnlistenFn | undefined;
    // Deep-link from another window (e.g. the main screen's "update available"
    // banner → About) when this window is already open.
    void onGotoSection((section) => {
      if (section) activate(section);
    }).then((f) => {
      unlisten = f;
    });
    void (async () => {
      try {
        applyConfig(await getConfig());
      } catch {
        /* ignore */
      }
      void aboutRef?.loadAbout();
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
    <GeneralTab hidden={active !== 'general'} bind:model onSave={save} />

    <!-- AUTH -->
    <AuthTab hidden={active !== 'auth'} bind:model onSave={save} />

    <!-- CONNECTION -->
    <ConnectionTab hidden={active !== 'conn'} bind:model bind:mtuText bind:reconnectText bind:forceDpdText onSave={save} />

    <!-- SUPPORT -->
    <SupportCards hidden={active !== 'support'} />

    <!-- ABOUT -->
    <AboutTab hidden={active !== 'about'} bind:hasUpdate={aboutHasUpdate} bind:this={aboutRef} />

    <span class="saved" class:show={savedShow} id="saved">
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#06281c" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 6"/></svg>
      <span>{m.settings_saved()}</span>
    </span>
  </div>
</div>
