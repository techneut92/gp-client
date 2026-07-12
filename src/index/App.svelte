<script lang="ts">
  // Main window — Svelte 5 port of gpgui ui/index.html.
  // DOM structure, class names and ids mirror the original so theme.css
  // applies unchanged.
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { m } from '../paraglide/messages.js';
  import { applyChoice, currentChoice, localeOptions, type LocaleChoice } from '../lib/locale';
  import Dropdown from '../lib/Dropdown.svelte';

  // The original page laid header/main/footer directly under the flex-column
  // <body>; main.ts mounts into <div id="app">, so make that wrapper
  // transparent to layout for identical rendering with the unchanged theme.css.
  document.getElementById('app')?.style.setProperty('display', 'contents');

  const hasTauri = '__TAURI_INTERNALS__' in window;

  // ───────── backend payload shapes (as read by the original JS) ─────────
  interface Identity {
    name: string;
    portal?: string;
    auth_method?: number;
    as_gateway?: boolean;
    username?: string;
    cert_file?: string;
    module_path?: string;
    cert_id?: string;
    cert_manufacturer?: string;
  }
  interface CertInfo {
    id: string;
    manufacturer?: string;
    slot?: string;
    expiry?: string;
  }
  interface InstallStep {
    label: string;
    cmd: string;
  }
  interface InstallOption {
    kind: string;
    label: string;
    steps: InstallStep[];
    note?: string;
  }
  interface SystemInfo {
    backendInstalled: boolean;
    installKind?: string;
    installOptions?: InstallOption[];
  }
  interface UpdateInfo {
    available?: boolean;
    backendUpdate?: boolean;
    latest?: string;
  }
  interface InstallResult {
    ok: boolean;
    needsReboot?: boolean;
    message?: string;
  }
  interface VaultStatus {
    exists: boolean;
    unlocked: boolean;
  }
  interface VpnState {
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
  interface Chip {
    k: string;
    v: string;
    color?: string;
  }
  interface LogLine {
    msg: string;
    err: boolean;
  }

  const KOFI_URL = 'https://ko-fi.com/techneut92?amount=2.50#checkoutModal';
  const RAW: readonly string[] = ['#6b7286', '#fbbf24', '#34d399', '#fb7185', '#fbbf24'];
  function authName(method: number | undefined): string {
    switch (method) {
      case -1:
        return m.main_auth_auto();
      case 0:
        return m.main_auth_smart_card();
      case 1:
        return m.main_auth_cert_file();
      case 2:
        return m.main_auth_saml();
      case 3:
        return m.main_auth_password();
      default:
        return '—';
    }
  }

  // ───────── demo data (replaced by backend when on Tauri) ─────────
  let identities = $state<Identity[]>([
    { name: 'Acme Corp', portal: 'vpn.acme-corp.com', auth_method: 0, as_gateway: true, module_path: '/usr/lib/opensc-pkcs11.so', cert_id: '01', cert_manufacturer: 'Yubico' },
    { name: 'Personal Lab', portal: 'lab.example.net', auth_method: 3, as_gateway: false, username: 'jane' },
  ]);
  const DEMO_CERTS: CertInfo[] = [
    { id: '01', manufacturer: 'Yubico', slot: 'Slot 01', expiry: '2026-07-10' },
    { id: '02', manufacturer: 'Yubico', slot: 'Slot 02', expiry: '2027-04-12' },
  ];
  let selected = $state<string | null>('Acme Corp');
  let active = false;
  let lastKind = $state(0);

  // ───────── window controls ─────────
  function winMin(): void {
    if (hasTauri) void getCurrentWindow().minimize();
  }
  function winClose(): void {
    if (hasTauri) void getCurrentWindow().close();
  }
  function openSettings(): void {
    if (hasTauri) void invoke('open_settings');
    else window.open('settings.html', 'gp-settings', 'width=560,height=620');
  }
  function onKofi(e: MouseEvent): void {
    if (hasTauri) {
      e.preventDefault();
      void invoke('open_url', { url: KOFI_URL });
    }
  }
  function openManager(): void {
    if (hasTauri) void invoke('open_manager');
    else window.open('manager.html', 'gp-manager', 'width=720,height=620');
  }

  // ───────── view switching ─────────
  let view = $state<'' | 'setup' | 'lock' | 'unlocked' | 'backend'>('');

  // ───────── backend presence + version banners ─────────
  let sysInfo = $state<SystemInfo | null>(null);
  let bkKind = $state('');
  const bkOptions = $derived((sysInfo?.installOptions ?? []).map((o) => ({ value: o.kind, label: o.label })));
  const bkOption = $derived((sysInfo?.installOptions ?? []).find((o) => o.kind === bkKind));
  const COPY_ICON = 'M9 9h11v11H9z M5 15V5a2 2 0 0 1 2-2h10';
  const CHECK_ICON = 'M5 12l5 5L20 6';
  let stepDone = $state<boolean[]>([]);
  const stepTimers = new Map<number, ReturnType<typeof setTimeout>>();
  let copyAllDone = $state(false);
  let copyAllTimer: ReturnType<typeof setTimeout> | undefined;
  let installing = $state(false);
  let bkStatus = $state<{ msg: string; kind: string } | null>(null);
  let checking = $state(false);
  let recheckStyle = $state('');

  function bkCopy(text: string): void {
    try {
      void navigator.clipboard.writeText(text);
    } catch {
      /* ignore */
    }
  }
  function copyStep(i: number, cmd: string): void {
    bkCopy(cmd);
    stepDone[i] = true;
    const t = stepTimers.get(i);
    if (t !== undefined) clearTimeout(t);
    stepTimers.set(
      i,
      setTimeout(() => {
        stepDone[i] = false;
      }, 1500)
    );
  }
  async function checkBackend(): Promise<boolean> {
    if (!hasTauri) return true;
    try {
      const si = await invoke<SystemInfo>('system_info');
      sysInfo = si;
      if (!si.backendInstalled) {
        const opts = (si.installOptions ?? []).map((o) => ({ value: o.kind, label: o.label }));
        // Default to the detected kind when it's a valid backend option, else the first.
        bkKind = opts.some((o) => o.value === si.installKind) ? (si.installKind ?? '') : (opts[0]?.value ?? '');
        stepDone = [];
        view = 'backend';
        return false;
      }
      return true;
    } catch {
      return true;
    }
  }

  let banner = $state<{ text: string; kind: string; onClick: () => void } | null>(null);
  let hasUpdate = $state(false);
  async function refreshBanners(): Promise<void> {
    if (!hasTauri) return;
    // Check for a newer release on startup. This drives the up-arrow badge on
    // the settings gear, independent of the compatibility banner below.
    let u: UpdateInfo | null = null;
    try {
      u = await invoke<UpdateInfo>('check_update');
    } catch {
      /* ignore */
    }
    hasUpdate = !!(u && (u.available || u.backendUpdate));

    // No version-mismatch banner: real GUI<->backend compatibility is enforced
    // by the protocol handshake at connect (a hard error on no overlap). A mere
    // version difference is fine, so we only surface "update available" here.
    if (u && (u.available || u.backendUpdate)) {
      // The app and backend update separately; the About page has the unified
      // "Update all" flow (both, narrated, with restart/reboot). Send the user
      // there rather than running a frontend-only update here.
      const what = u.available && u.backendUpdate ? m.main_update_app_backend() : u.available ? m.main_update_app() : m.main_update_backend();
      banner = {
        text: m.main_update_available({ what, version: u.latest ?? '' }),
        kind: 'info',
        onClick: () => {
          if (hasTauri) void invoke('open_settings', { section: 'about' });
        },
      };
      return;
    }
    banner = null;
  }
  async function installBackend(): Promise<void> {
    installing = true;
    bkStatus = { msg: m.main_install_prompt(), kind: '' };
    // pkexec approval isn't observable mid-command, so once the prompt has had
    // time to be approved, reflect that the install is running (rpm-ostree
    // layering can take a few minutes) instead of leaving the prompt message.
    const progressing = setTimeout(() => {
      bkStatus = { msg: m.main_install_progress(), kind: '' };
    }, 4000);
    try {
      const r = await invoke<InstallResult>('install_backend', { kind: bkKind });
      clearTimeout(progressing);
      if (r.ok) bkStatus = { msg: r.needsReboot ? m.main_install_done_reboot() : m.main_install_done(), kind: 'ok' };
      else bkStatus = { msg: r.message || m.main_install_failed(), kind: 'err' };
    } catch (e) {
      clearTimeout(progressing);
      bkStatus = { msg: String(e), kind: 'err' };
    }
    installing = false;
  }
  function copyAll(): void {
    const o = bkOption;
    if (!o) return;
    bkCopy(o.steps.map((s) => s.cmd).join('\n'));
    copyAllDone = true;
    if (copyAllTimer !== undefined) clearTimeout(copyAllTimer);
    copyAllTimer = setTimeout(() => {
      copyAllDone = false;
    }, 1500);
  }
  async function recheck(): Promise<void> {
    recheckStyle = 'transition: transform .6s; transform: rotate(360deg);';
    checking = true;
    const ok = await checkBackend();
    if (ok) {
      await bootVault();
      void refreshBanners();
    }
    setTimeout(() => {
      recheckStyle = 'transition: transform .6s; transform: none;';
      checking = false;
    }, 800);
  }

  // ───────── rich identity picker ─────────
  let idOpen = $state(false);
  let idTriggerEl = $state<HTMLButtonElement | null>(null);
  let idMenuEl: HTMLDivElement | null = null;
  const curIdentity = $derived(identities.find((i) => i.name === selected));
  const initials = (n: string): string => (n || '?').trim().slice(0, 1).toUpperCase();

  function toggleIdMenu(e: MouseEvent): void {
    e.stopPropagation();
    idOpen = !idOpen;
  }
  function pickIdentity(id: Identity): void {
    selected = id.name;
    idOpen = false;
    void renderChips();
  }
  function idPortal(node: HTMLDivElement): { destroy(): void } {
    document.body.appendChild(node);
    idMenuEl = node;
    if (idTriggerEl) {
      const r = idTriggerEl.getBoundingClientRect();
      node.style.top = `${r.bottom + 6}px`;
      node.style.left = `${r.left}px`;
      node.style.width = `${r.width}px`;
    }
    return {
      destroy: () => {
        node.remove();
        idMenuEl = null;
      },
    };
  }
  function onDocDown(e: MouseEvent): void {
    const t = e.target as Node;
    if (idOpen && idMenuEl && !idMenuEl.contains(t) && idTriggerEl && !idTriggerEl.contains(t)) idOpen = false;
  }
  function closeIdMenu(): void {
    if (idOpen) idOpen = false;
  }
  document.addEventListener('mousedown', onDocDown, true);
  window.addEventListener('scroll', closeIdMenu, true);
  window.addEventListener('resize', closeIdMenu);
  onDestroy(() => {
    document.removeEventListener('mousedown', onDocDown, true);
    window.removeEventListener('scroll', closeIdMenu, true);
    window.removeEventListener('resize', closeIdMenu);
  });

  let chips = $state<Chip[]>([]);
  async function renderChips(): Promise<void> {
    const id = identities.find((i) => i.name === selected);
    if (!id) {
      chips = [];
      return;
    }
    const row = (k: string, v: string | undefined, color?: string): Chip => (color !== undefined ? { k, v: v || '—', color } : { k, v: v || '—' });
    const head: Chip[] = [
      row(m.main_portal(), id.portal),
      row(m.main_auth(), authName(id.auth_method)),
      row(m.main_gateway(), id.as_gateway ? m.main_gateway_direct() : m.main_gateway_via_portal()),
    ];
    let mid: Chip[] = [];
    if (id.auth_method === 1) mid = [row(m.main_cert_file(), (id.cert_file ?? '').split('/').pop())];
    else if (id.auth_method === 3) mid = [row(m.main_username(), id.username)];
    chips = [...head, ...mid];

    // smart-card: read cert live so expiry reflects renewals
    if (id.auth_method === 0 && id.cert_id) {
      chips = [...head, row(m.main_certificate(), m.main_reading_token())];
      let c: CertInfo | null | undefined = DEMO_CERTS.find((x) => x.id === id.cert_id);
      if (hasTauri) {
        try {
          const list = await invoke<CertInfo[]>('scan_certs', { module: id.module_path });
          c = list.find((x) => x.id === id.cert_id && x.manufacturer === id.cert_manufacturer);
        } catch {
          c = null;
        }
      }
      if (selected !== id.name) return;
      let cm: Chip[];
      if (c) {
        cm = [row(m.main_certificate(), c.slot || m.main_cert_id({ id: id.cert_id }))];
        if (c.expiry) {
          const soon = new Date(c.expiry).getTime() - Date.now() < 30 * 864e5;
          cm.push(row(m.main_expires(), c.expiry, soon ? 'var(--yellow)' : 'var(--green)'));
        }
      } else {
        cm = [row(m.main_certificate(), m.main_cert_id({ id: id.cert_id })), row('', m.main_token_not_present(), 'var(--muted)')];
      }
      chips = [...head, ...cm];
    }
  }

  function renderIdentityUI(): void {
    const has = identities.length > 0;
    if (has) {
      if (!identities.find((i) => i.name === selected)) selected = identities[0]?.name ?? null;
      void renderChips();
    }
  }

  // ───────── state / orb / footer action ─────────
  let pillText = $state<string>(m.status_disconnected());
  let statusSubText = $state(m.main_sub_not_encrypted());
  let footLog = $state<LogLine>({ msg: '', err: false });
  let cIp = $state('—');
  let cIface = $state('—');
  let cPortal = $state('—');
  let cGateway = $state('—');
  let cExpires = $state('—');

  const stateColor = $derived(RAW[lastKind] ?? '#6b7286');
  const dotLive = $derived(lastKind === 1 || lastKind === 2 || lastKind === 4);
  const orbConnecting = $derived(lastKind === 1 || lastKind === 4);
  const orbConnected = $derived(lastKind === 2);
  const orbGlyphInner = $derived(lastKind === 2 ? 'M9 12l2 2 4-4' : 'M12 8v4M12 15.5h.01');
  const statusTitleText = $derived(
    [m.main_status_not_connected(), m.status_connecting(), m.main_status_protected(), m.main_status_failed(), m.status_reconnecting()][lastKind] ?? m.main_status_not_connected()
  );
  // Reconnecting keeps the connected details view — the session survives.
  const isConnectedView = $derived(lastKind === 2 || lastKind === 4);

  const actionMode = $derived(lastKind === 2 || lastKind === 4 ? 'disconnect' : lastKind === 1 ? 'cancel' : lastKind === 3 ? 'retry' : 'connect');
  const actionLabel = $derived(actionMode === 'disconnect' ? m.main_disconnect() : actionMode === 'cancel' ? m.common_cancel() : actionMode === 'retry' ? m.main_try_again() : m.main_connect());
  const actionClass = $derived(actionMode === 'disconnect' ? 'btn-action danger' : actionMode === 'cancel' ? 'btn-action ghost' : 'btn-action');
  // The footer Connect button is hidden when there's nothing to connect to.
  const actionVisible = $derived(actionMode !== 'connect' || identities.length > 0);
  function onAction(): void {
    if (actionMode === 'disconnect' || actionMode === 'cancel') disconnect();
    else void connect();
  }

  // Live connection timer. The backend doesn't send an elapsed value, so we
  // count from when we first see "Connected" and tick the label every second.
  let liveT0: number | null = null;
  function elapsedStr(): string {
    if (liveT0 === null) return '00:00:00';
    const d = Math.floor((Date.now() - liveT0) / 1000);
    return [d / 3600, (d % 3600) / 60, d % 60].map((n) => String(Math.floor(n)).padStart(2, '0')).join(':');
  }
  setInterval(() => {
    if (liveT0 === null) return;
    if (lastKind === 2) statusSubText = m.main_sub_connected({ elapsed: elapsedStr() });
  }, 1000);

  function render(s: VpnState): void {
    const kind = s.kind ?? 0;
    lastKind = kind;
    // kind 4 (reconnecting) keeps the elapsed clock — the session survives.
    if (kind === 2 || kind === 4) {
      if (liveT0 === null) liveT0 = Date.now();
    } else {
      liveT0 = null;
    }
    pillText = kind === 3 ? m.status_error() : (s.status ?? '');
    let sub = m.main_sub_not_encrypted();
    if (kind === 1) sub = m.main_sub_establishing();
    else if (kind === 2) sub = m.main_sub_connected({ elapsed: s.elapsed || elapsedStr() });
    else if (kind === 3) sub = m.main_sub_check_credentials();
    else if (kind === 4) sub = m.main_sub_reestablishing();
    statusSubText = sub;

    active = !!s.active;
    // only flip content when vault is unlocked
    if (view !== 'unlocked') return;
    const connected = kind === 2 || kind === 4;

    // Footer status line + state-driven action button.
    let logMsg = s.log ?? '';
    let isErr = false;
    if (kind === 3) {
      logMsg = (s.status ?? '').replace(/^Error:\s*/i, '') || m.main_status_failed();
      isErr = true;
    }
    footLog = { msg: logMsg, err: isErr };

    if (connected) {
      cIp = s.ip || '—';
      cIface = s.iface || '—';
      cPortal = s.portal || '—';
      cGateway = s.gateway || '—';
      cExpires = s.expires || '—';
    }
  }

  // ───────── connect flow ─────────
  let timer: ReturnType<typeof setInterval> | null = null;
  let demoTimers: ReturnType<typeof setTimeout>[] = [];
  async function connect(): Promise<void> {
    const id = identities.find((i) => i.name === selected);
    if (!id) {
      footLog = { msg: m.main_add_identity_first(), err: true };
      return;
    }
    if (hasTauri) {
      try {
        await invoke('connect', { identity: id.name, portal: '' });
      } catch (e) {
        render({ kind: 3, status: m.status_error(), log: 'Error: ' + String(e) });
      }
      return;
    }
    render({ kind: 1, status: m.status_connecting(), log: m.main_demo_authenticating({ portal: id.portal ?? '' }) });
    demoTimers.push(
      setTimeout(() => {
        render({ kind: 1, status: m.status_connecting(), log: m.main_demo_negotiating() });
        demoTimers.push(
          setTimeout(() => {
            const t0 = Date.now();
            const fmt = (): string => {
              const d = Math.floor((Date.now() - t0) / 1000);
              return [d / 3600, (d % 3600) / 60, d % 60].map((n) => String(Math.floor(n)).padStart(2, '0')).join(':');
            };
            const detail: VpnState = {
              kind: 2,
              status: m.status_connected(),
              active: true,
              portal: id.portal ?? '',
              gateway: id.as_gateway ? (id.portal ?? '') : 'gw1.' + (id.portal ?? '').replace(/^[^.]+\./, ''),
              ip: '10.42.7.' + (20 + Math.floor(Math.random() * 200)),
              iface: 'gpd0',
              expires: m.main_demo_expires(),
              elapsed: fmt(),
            };
            render(detail);
            timer = setInterval(() => {
              detail.elapsed = fmt();
              if (active && detail.elapsed !== undefined) statusSubText = m.main_sub_connected({ elapsed: detail.elapsed });
            }, 1000);
          }, 1100)
        );
      }, 1300)
    );
  }
  function disconnect(): void {
    demoTimers.forEach(clearTimeout);
    demoTimers = [];
    if (timer !== null) {
      clearInterval(timer);
      timer = null;
    }
    if (hasTauri) {
      void invoke('disconnect');
      return;
    }
    render({ kind: 0, status: m.status_disconnected(), active: false, log: '' });
  }

  // ───────── vault ─────────
  async function unlocked(): Promise<void> {
    if (hasTauri) {
      try {
        identities = await invoke<Identity[]>('list_identities');
      } catch {
        identities = [];
      }
    }
    if (!identities.find((i) => i.name === selected)) selected = identities[0]?.name ?? null;
    view = 'unlocked';
    renderIdentityUI();
    render(active ? { kind: 2, status: m.status_connected(), active: true } : { kind: 0, status: m.status_disconnected(), active: false, log: '' });
  }

  // ── "Unlock automatically" opt-in on the create-vault screen ──
  let autoUnlock = $state(false); // default OFF — deliberate opt-in
  let keyringAvailable = $state(true); // detected at runtime
  let infoNoteHidden = $state(true);
  const autoOn = $derived(keyringAvailable && autoUnlock);
  function toggleAutoUnlock(): void {
    if (!keyringAvailable) return;
    autoUnlock = !autoUnlock;
  }

  let newMasterPin = $state('');
  let masterPin = $state('');
  let setupLog = $state<LogLine>({ msg: '', err: false });
  let lockLog = $state<LogLine>({ msg: '', err: false });

  async function doSetup(): Promise<void> {
    if (hasTauri) {
      try {
        // Enable remember-unlock first so set_master_pin stores the PIN.
        if (autoUnlock && keyringAvailable) await invoke('set_remember_unlock', { enabled: true });
        await invoke('set_master_pin', { pin: newMasterPin });
      } catch (e) {
        setupLog = { msg: String(e), err: true };
        return;
      }
    }
    await unlocked();
  }
  async function doUnlock(): Promise<void> {
    if (hasTauri) {
      try {
        await invoke('unlock_vault', { pin: masterPin });
      } catch (e) {
        lockLog = { msg: String(e), err: true };
        return;
      }
    }
    await unlocked();
  }

  // forgotten-PIN reset (deletes identities)
  let resetOpen = $state(false);
  async function resetConfirm(): Promise<void> {
    if (hasTauri) {
      try {
        await invoke('reset_vault');
      } catch {
        /* ignore */
      }
    }
    identities = [];
    selected = null;
    masterPin = '';
    lockLog = { msg: '', err: false };
    resetOpen = false;
    view = 'setup'; // start fresh: set a new master PIN
  }

  // ───────── init ─────────
  async function bootVault(): Promise<void> {
    const vs = await invoke<VaultStatus>('vault_status');
    if (!vs.exists) {
      try {
        keyringAvailable = await invoke<boolean>('keyring_available');
      } catch {
        /* ignore */
      }
      view = 'setup';
    } else if (!vs.unlocked) view = 'lock';
    else await unlocked();
  }
  onMount(() => {
    void (async () => {
      if (hasTauri) {
        if (!(await checkBackend())) return; // backend missing → install screen, stop here
        await bootVault();
        render(await invoke<VpnState>('get_state'));
        await listen<VpnState>('state', (e) => {
          render(e.payload);
        });
        await listen('identities-changed', async () => {
          try {
            identities = await invoke<Identity[]>('list_identities');
          } catch {
            /* keep current list */
          }
          renderIdentityUI();
        });
        void refreshBanners(); // version-mismatch / update-available banner (non-blocking)
      } else {
        // demo: start locked to show the vault unlock, any PIN unlocks
        view = 'lock';
      }
    })();
  });
</script>

{#snippet authIcon(method: number | undefined)}
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
    {#if method === 0}
      <rect x="2" y="5" width="20" height="14" rx="2.5" /><path d="M2 10h20" />
    {:else if method === 1}
      <path d="M14 3v4a1 1 0 0 0 1 1h4" /><path d="M17 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2z" />
    {:else if method === 2}
      <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" /><path d="M10 17l5-5-5-5M15 12H3" />
    {:else if method === 3}
      <circle cx="12" cy="8" r="4" /><path d="M4 21a8 8 0 0 1 16 0" />
    {:else}
      <circle cx="11" cy="11" r="7" /><path d="m20 20-3.2-3.2" />
    {/if}
  </svg>
{/snippet}

<header data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <div class="mark">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l7 3v5c0 4.4-3 7.4-7 9-4-1.6-7-4.6-7-9V6l7-3z" /></svg>
    </div>
    <div class="title">{m.main_brand_gp()} <span class="ng">{m.main_brand_client()}</span></div>
  </div>
  <div class="hbar-right">
    <div class="pill"><span class="dot" id="dot" class:live={dotLive} style="background:{stateColor};color:{stateColor}"></span><span id="statusText">{pillText}</span></div>
    <div class="winctl">
      <a class="wbtn kofi" id="winKofi" href={KOFI_URL} target="_blank" rel="noreferrer" title={m.main_kofi_title()} onclick={onKofi}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M5 8h11v5a4 4 0 0 1-4 4H9a4 4 0 0 1-4-4z" /><path d="M16 9h1.5a2.5 2.5 0 0 1 0 5H16" /><path d="M8 3v2M11 3v2" /></svg>
      </a>
      <button class="wbtn" id="winManage" title={m.main_manage_identities_title()} hidden={view !== '' && view !== 'unlocked'} onclick={openManager}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="8" r="3.2" /><path d="M3.5 20a5.5 5.5 0 0 1 11 0" /><path d="M16 4.5a3 3 0 0 1 0 6M18 20a5.5 5.5 0 0 0-3-4.9" /></svg>
      </button>
      <button class="wbtn" id="winSettings" title={m.main_settings_title()} class:has-update={hasUpdate} hidden={view !== '' && view !== 'unlocked'} onclick={openSettings}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3" /><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" /></svg>
      </button>
      <button class="wbtn" id="winMin" title={m.main_minimize()} onclick={winMin}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M5 12h14" /></svg>
      </button>
      <button class="wbtn close" id="winClose" title={m.common_close()} onclick={winClose}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M5 5l14 14M19 5L5 19" /></svg>
      </button>
    </div>
  </div>
</header>

<main>
  <!-- ============ SETUP (first run) ============ -->
  <div class="view center" id="setupView" class:show={view === 'setup'}>
    <div class="stack vault">
      <div class="vault-badge">
        <svg width="38" height="38" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l7 3v5c0 4.4-3 7.4-7 9-4-1.6-7-4.6-7-9V6l7-3z" /><rect x="9" y="11" width="6" height="5" rx="1" /><path d="M10.5 11V9.5a1.5 1.5 0 0 1 3 0V11" /></svg>
      </div>
      <h2 class="h2">{m.main_setup_title()}</h2>
      <p class="sub">{m.main_setup_sub()}</p>
      <input
        class="field pin-input"
        id="newMasterPin"
        type="password"
        placeholder={m.main_setup_pin_placeholder()}
        bind:value={newMasterPin}
        onkeydown={(e) => {
          if (e.key === 'Enter') void doSetup();
        }}
      />
      <p class="formlog" id="setupLog" class:err={setupLog.err}>{setupLog.msg}</p>

      <div class="vault-optin" id="autoUnlockRow" class:disabled={!keyringAvailable} class:on={autoOn}>
        <div class="optin-main">
          <div class="optin-head">
            <span class="optin-label">{m.main_auto_unlock_label()}</span>
            <button
              class="optin-info"
              id="autoInfoBtn"
              type="button"
              title={m.main_auto_unlock_about()}
              aria-label={m.main_auto_unlock_about()}
              hidden={!keyringAvailable}
              onclick={() => {
                infoNoteHidden = !infoNoteHidden;
              }}>?</button
            >
          </div>
          <div class="optin-help" id="autoHelp">
            {#if !keyringAvailable}
              {m.main_auto_unlock_unavailable()}
            {:else if autoUnlock}
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#34d399" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px;margin-right:5px;"><path d="M5 12l5 5L20 6" /></svg>{m.main_auto_unlock_on()}
            {:else}
              {m.main_auto_unlock_help()}
            {/if}
          </div>
        </div>
        <button class="switch" id="autoUnlockSwitch" type="button" role="switch" aria-checked={autoOn} aria-label={m.main_auto_unlock_label()} class:on={autoOn} disabled={!keyringAvailable} onclick={toggleAutoUnlock}><span class="knob"></span></button>
      </div>
      <div class="optin-note" id="autoInfoNote" hidden={infoNoteHidden || !keyringAvailable}>{m.main_auto_unlock_note()}</div>

      <div class="actions" style="width:100%"><button class="btn-action" id="setupBtn" onclick={() => void doSetup()}>{m.main_create_vault()}</button></div>
      <div style="width:100%;display:flex;align-items:center;justify-content:center;gap:10px;margin-top:14px;">
        <span style="font-size:12px;color:var(--faint);display:inline-flex;align-items:center;"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px;margin-right:6px;"><circle cx="12" cy="12" r="10" /><path d="M2 12h20" /><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" /></svg>{m.language_label()}</span>
        <div style="min-width:170px;"><Dropdown options={localeOptions()} value={currentChoice()} onChange={(v) => applyChoice(v as LocaleChoice)} /></div>
      </div>
    </div>
  </div>

  <!-- ============ LOCK ============ -->
  <div class="view center" id="lockView" class:show={view === 'lock'}>
    <div class="stack vault">
      <div class="vault-badge">
        <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="10" width="16" height="11" rx="2.5" /><path d="M8 10V7a4 4 0 0 1 8 0v3" /><circle cx="12" cy="15.5" r="1.4" /></svg>
      </div>
      <h2 class="h2">{m.main_unlock()}</h2>
      <p class="sub">{m.main_lock_sub()}</p>
      <input
        class="field pin-input"
        id="masterPin"
        type="password"
        placeholder={m.main_master_pin_placeholder()}
        bind:value={masterPin}
        onkeydown={(e) => {
          if (e.key === 'Enter') void doUnlock();
        }}
      />
      <p class="formlog" id="lockLog" class:err={lockLog.err}>{lockLog.msg}</p>
      <div class="actions" style="width:100%"><button class="btn-action" id="unlockBtn" onclick={() => void doUnlock()}>{m.main_unlock()}</button></div>
      <button class="link" id="forgotPinBtn" style="margin-top:8px;" hidden={resetOpen} onclick={() => (resetOpen = true)}>{m.main_forgot_pin()}</button>
      <div style="width:100%;display:flex;align-items:center;justify-content:center;gap:10px;margin-top:14px;">
        <span style="font-size:12px;color:var(--faint);display:inline-flex;align-items:center;"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px;margin-right:6px;"><circle cx="12" cy="12" r="10" /><path d="M2 12h20" /><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" /></svg>{m.language_label()}</span>
        <div style="min-width:170px;"><Dropdown options={localeOptions()} value={currentChoice()} onChange={(v) => applyChoice(v as LocaleChoice)} /></div>
      </div>
      <div class="reset-warn" id="resetWarn" hidden={!resetOpen}>
        <div class="warn-text">
          <strong>{m.main_reset_title()}</strong> {m.main_reset_body()}
        </div>
        <div class="reset-actions">
          <button class="btn-action danger" id="resetConfirmBtn" onclick={() => void resetConfirm()}>{m.main_reset_confirm()}</button>
          <button class="link" id="resetCancelBtn" onclick={() => (resetOpen = false)}>{m.common_cancel()}</button>
        </div>
      </div>
    </div>
  </div>

  <!-- ============ BACKEND MISSING (privileged service not installed) ============ -->
  <div class="view" id="backendView" class:show={view === 'backend'}>
    <div class="svc-wrap">
      <div class="svc-hero">
        <div class="svc-badge">
          <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="#fbbf24" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="6" rx="1.6" /><rect x="3" y="14" width="18" height="6" rx="1.6" /><path d="M7 7h.01M7 17h.01" /><path d="M11 7h6M11 17h6" /></svg>
          <span class="alert"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#0a0c12" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M12 8v5M12 16.5h.01" /></svg></span>
        </div>
        <div class="svc-title">{m.main_backend_title()}</div>
        <div class="svc-sub">{m.main_backend_sub()}</div>
      </div>

      <div class="svc-section">
        <div class="svc-syslabel"><span class="lbl" style="margin:0;">{m.main_system_type()}</span><span class="hint-inline">{m.main_system_type_hint()}</span></div>
        <div id="sysDD">
          <Dropdown
            options={bkOptions}
            bind:value={bkKind}
            onChange={() => {
              stepDone = [];
            }}
          />
        </div>
      </div>

      <button class="btn-action svc-install" id="bkInstall" disabled={installing} onclick={() => void installBackend()}>
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12M8 11l4 4 4-4M5 21h14" /></svg>
        <span id="bkInstallLabel">{installing ? m.main_installing() : m.main_install_backend()}</span>
      </button>
      <div class="bk-status{bkStatus && bkStatus.kind ? ' ' + bkStatus.kind : ''}" id="bkInstallLog" hidden={!bkStatus}>{bkStatus ? bkStatus.msg : ''}</div>
      <div class="svc-or"><span>{m.main_or_manual()}</span></div>

      <div class="term-card">
        <div class="term-head">
          <span class="term-dots"><i style="background:#fb6f6c"></i><i style="background:#fbbf24"></i><i style="background:#34d399"></i></span>
          <span class="term-label">{m.main_term_label()}</span>
        </div>
        <div class="term-body">
          <div id="steps">
            {#each bkOption?.steps ?? [] as s, i (i)}
              <div class="step" class:last={i === (bkOption?.steps.length ?? 0) - 1}>
                <span class="step-num">{i + 1}</span>
                <div class="step-body">
                  <div class="step-label">{s.label}</div>
                  <div class="step-cmd-row">
                    <div class="cmd"><code>{s.cmd}</code></div>
                    <button class="copy-btn" class:done={stepDone[i] === true} title={m.main_copy()} onclick={() => copyStep(i, s.cmd)}>
                      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d={stepDone[i] === true ? CHECK_ICON : COPY_ICON} /></svg>
                    </button>
                  </div>
                </div>
              </div>
            {/each}
          </div>
          <div class="term-note">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9" /><path d="M12 8h.01M11 12h1v4h1" /></svg>
            <span id="note">{bkOption?.note ?? ''}</span>
          </div>
        </div>
      </div>

      <button class="svc-copyall svc-secondary" id="copyAll" class:done={copyAllDone} onclick={copyAll}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path id="copyAllIcon" d={copyAllDone ? CHECK_ICON : COPY_ICON} /></svg>
        <span id="copyAllLabel">{copyAllDone ? m.main_copied() : m.main_copy_all()}</span>
      </button>

      <div class="svc-recheck-wrap">
        <button class="svc-recheck" id="recheck" onclick={() => void recheck()}>
          <svg id="recheckIcon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" style={recheckStyle}><path d="M3 12a9 9 0 0 1 15.5-6.2L21 8" /><path d="M21 4v4h-4" /><path d="M21 12a9 9 0 0 1-15.5 6.2L3 16" /><path d="M3 20v-4h4" /></svg>
          <span id="recheckLabel">{checking ? m.main_checking() : m.main_recheck()}</span>
        </button>
      </div>
    </div>
  </div>

  <!-- ============ UNLOCKED (orb + content; action button lives in the footer) ============ -->
  <div class="view" id="unlockedView" class:show={view === 'unlocked'}>
    <div class="topbanner{banner ? ' ' + banner.kind : ''}" id="topBanner" hidden={!banner}>
      {#if banner}<span>{banner.text}</span><button class="banner-btn" onclick={banner.onClick}>{m.main_update_view()}</button>{/if}
    </div>
    <div class="hero">
      <div class="orb-wrap" id="orb" class:connecting={orbConnecting} class:connected={orbConnected} style="--c: {stateColor}">
        <div class="orb-glow"></div>
        <div class="orb-ring"></div>
        <div class="orb-disc">
          <svg id="orbGlyph" width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 3l7 3v5c0 4.4-3 7.4-7 9-4-1.6-7-4.6-7-9V6l7-3z" />
            <path id="orbGlyphInner" d={orbGlyphInner} />
          </svg>
        </div>
      </div>
      <div class="status-title" id="statusTitle">{statusTitleText}</div>
      <div class="status-sub" id="statusSub">{statusSubText}</div>
    </div>

    <!-- disconnected / connecting: pick identity -->
    <div id="formContent" hidden={isConnectedView}>
      <div class="card" id="idCard" class:hidden={identities.length === 0}>
        <div class="lbl">{m.main_identity()}</div>
        <div id="identityDD" class="mb14">
          <button type="button" class="id-trigger" class:active={idOpen} bind:this={idTriggerEl} onclick={toggleIdMenu}>
            {#if curIdentity}
              <div class="avatar">{initials(curIdentity.name)}</div>
              <div class="meta"><div class="name">{curIdentity.name}</div><div class="portal">{curIdentity.portal ?? ''}</div></div>
            {:else}
              <div class="avatar" style="background:#2a3142;color:#8a91a8">+</div>
              <div class="meta"><div class="name">{m.main_no_identity()}</div><div class="portal">{m.main_add_one_to_connect()}</div></div>
            {/if}
            <svg class="dd-chev" width="12" height="8" viewBox="0 0 12 8" fill="none" stroke="#8a91a8" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M1 1.5l5 5 5-5" /></svg>
          </button>
        </div>
        <div class="detail-card" id="idInfo">
          {#each chips as c, i (i)}
            <div class="drow">
              <div class="k">{c.k}</div>
              <div class="v">{#if c.color !== undefined}<span style="color:{c.color}">{c.v}</span>{:else}{c.v}{/if}</div>
            </div>
          {/each}
        </div>
      </div>
      <div class="card" id="emptyCard" class:hidden={identities.length > 0} style="text-align:center;">
        <div class="sub" style="margin:2px 0 12px;">{m.main_no_identities()}</div>
        <button class="btn-action auto" id="emptyAddBtn" style="padding:11px 18px;" onclick={openManager}>{m.main_add_identity()}</button>
      </div>
      <button class="link manage-link" id="manageBtn" onclick={openManager}>
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="8" r="3.2" /><path d="M3.5 20a5.5 5.5 0 0 1 11 0" /><path d="M16 4.5a3 3 0 0 1 0 6M18 20a5.5 5.5 0 0 0-3-4.9" /></svg>
        <span>{m.main_manage_identities()}</span>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-left:auto;"><path d="M9 6l6 6-6 6" /></svg>
      </button>
    </div>

    <!-- connected: session details -->
    <div id="connContent" hidden={!isConnectedView}>
      <div class="tiles">
        <div class="tile ip"><div class="k">{m.main_your_ip()}</div><div class="v" id="cIp">{cIp}</div></div>
        <div class="tile"><div class="k">{m.main_interface()}</div><div class="v" id="cIface">{cIface}</div></div>
      </div>
      <div class="detail-card">
        <div class="drow"><div class="k">{m.main_portal()}</div><div class="v" id="cPortal">{cPortal}</div></div>
        <div class="drow"><div class="k">{m.main_gateway()}</div><div class="v" id="cGateway">{cGateway}</div></div>
        <div class="drow"><div class="k">{m.main_session_expires()}</div><div class="v" id="cExpires">{cExpires}</div></div>
      </div>
    </div>
  </div>
</main>

<!-- action footer (shown only when unlocked) -->
<footer id="appFooter" hidden={view !== 'unlocked'}>
  <div class="log" id="footLog" class:err={footLog.err}>{footLog.msg}</div>
  <button class={actionClass} id="actionBtn" style:display={actionVisible ? '' : 'none'} onclick={onAction}>{actionLabel}</button>
</footer>

{#if idOpen}
  <div class="dd-menu" use:idPortal>
    {#each identities as ident (ident.name)}
      <button type="button" class="id-item" class:active={ident.name === selected} onclick={() => pickIdentity(ident)}>
        <span class="ico">{@render authIcon(ident.auth_method)}</span>
        <span class="txt"><span class="nm">{ident.name}</span><span class="pt">{ident.portal ?? ''}</span></span>
      </button>
    {/each}
  </div>
{/if}
