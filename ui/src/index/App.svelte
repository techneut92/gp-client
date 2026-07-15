<script lang="ts">
  // Main window — Svelte 5 port of gpgui ui/index.html.
  // App.svelte orchestrates the state machine and which view shows; the
  // views themselves live in sibling components whose DOM structure, class
  // names and ids mirror the original so theme.css applies unchanged.
  import { onMount } from 'svelte';
  import { m } from '../paraglide/messages.js';
  import * as api from '../lib/api';
  import { type CertInfo, type Identity, type UpdateInfo, type VpnState } from '../lib/api';
  import { mountShim } from '../lib/shim';
  import type { Banner, Chip, LogLine, View } from './types';
  import Header from './Header.svelte';
  import SetupView from './SetupView.svelte';
  import LockView from './LockView.svelte';
  import BackendMissing from './BackendMissing.svelte';
  import ImportView from './ImportView.svelte';
  import UpdateBanner from './UpdateBanner.svelte';
  import StatusHero from './StatusHero.svelte';
  import IdentityPicker from './IdentityPicker.svelte';
  import ConnectedCard from './ConnectedCard.svelte';
  import ActionFooter from './ActionFooter.svelte';

  mountShim();

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

  let identities = $state<Identity[]>([]);
  let selected = $state<string | null>(null);
  let active = false;
  let lastKind = $state(0);

  // ───────── window controls ─────────
  function openSettings(): void {
    void api.openSettings();
  }
  function openManager(): void {
    void api.openManager();
  }

  // ───────── view switching ─────────
  let view = $state<View>('');

  // ───────── gpgui migration screen ─────────
  let importBusy = $state(false);
  let importError = $state('');

  // ───────── backend presence + version banners ─────────
  let sysInfo = $state<api.SystemInfo | null>(null);
  let bkKind = $state('');
  const bkOptions = $derived((sysInfo?.installOptions ?? []).map((o) => ({ value: o.kind, label: o.label })));
  const bkOption = $derived((sysInfo?.installOptions ?? []).find((o) => o.kind === bkKind));
  // A backend is present but too old — the install screen becomes an upgrade prompt.
  const bkOutdated = $derived(!!(sysInfo?.backendInstalled && !sysInfo?.backendSupported));
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
    try {
      const si = await api.systemInfo();
      sysInfo = si;
      // Missing backend, or one too old to speak the auth handoff — both route to
      // the install/upgrade screen (installing lands the latest, which upgrades).
      if (!si.backendInstalled || !si.backendSupported) {
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

  let banner = $state<Banner | null>(null);
  let hasUpdate = $state(false);
  async function refreshBanners(): Promise<void> {
    // Check for a newer release on startup. This drives the up-arrow badge on
    // the settings gear, independent of the compatibility banner below.
    let u: UpdateInfo | null = null;
    try {
      u = await api.checkUpdate();
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
      const both = !!(u.available && u.backendUpdate);
      const title = both ? m.main_updates_available() : m.main_update_available_one();
      // Version subtitle — list whichever component is behind, with its target
      // version (skip a component whose latest version is unknown/empty).
      const parts: string[] = [];
      if (u.available && u.latest) parts.push(m.main_update_sub_app({ version: u.latest }));
      if (u.backendUpdate && u.backendLatest) parts.push(m.main_update_sub_backend({ version: u.backendLatest }));
      banner = {
        text: title,
        sub: parts.join(' · '),
        kind: 'info',
        onClick: () => {
          void api.openSettings('about');
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
      const r = await api.installBackend({ kind: bkKind });
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
  function pickIdentity(id: Identity): void {
    selected = id.name;
    void renderChips();
  }

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
      let c: CertInfo | null | undefined;
      try {
        const list = await api.scanCerts(id.module_path);
        c = list.find((x) => x.id === id.cert_id && x.manufacturer === id.cert_manufacturer);
      } catch {
        c = null;
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
  async function connect(): Promise<void> {
    const id = identities.find((i) => i.name === selected);
    if (!id) {
      footLog = { msg: m.main_add_identity_first(), err: true };
      return;
    }
    try {
      await api.connect(id.name, '');
    } catch (e) {
      render({ kind: 3, status: m.status_error(), log: 'Error: ' + String(e) });
    }
  }
  function disconnect(): void {
    void api.disconnect();
  }

  // ───────── vault ─────────
  async function unlocked(): Promise<void> {
    try {
      identities = await api.listIdentities();
    } catch {
      identities = [];
    }
    if (!identities.find((i) => i.name === selected)) selected = identities[0]?.name ?? null;
    view = 'unlocked';
    renderIdentityUI();
    render(active ? { kind: 2, status: m.status_connected(), active: true } : { kind: 0, status: m.status_disconnected(), active: false, log: '' });
  }

  // ── "Unlock automatically" opt-in on the create-vault screen ──
  let autoUnlock = $state(false); // default OFF — deliberate opt-in
  let keyringAvailable = $state(true); // detected at runtime

  let newMasterPin = $state('');
  let masterPin = $state('');
  let setupLog = $state<LogLine>({ msg: '', err: false });
  let lockLog = $state<LogLine>({ msg: '', err: false });

  async function doSetup(): Promise<void> {
    try {
      // Enable remember-unlock first so set_master_pin stores the PIN.
      if (autoUnlock && keyringAvailable) await api.setRememberUnlock(true);
      await api.setMasterPin(newMasterPin);
    } catch (e) {
      setupLog = { msg: String(e), err: true };
      return;
    }
    await unlocked();
  }
  async function doUnlock(): Promise<void> {
    try {
      await api.unlockVault(masterPin);
    } catch (e) {
      lockLog = { msg: String(e), err: true };
      return;
    }
    await unlocked();
  }

  // forgotten-PIN reset (deletes identities)
  let resetOpen = $state(false);
  async function resetConfirm(): Promise<void> {
    try {
      await api.resetVault();
    } catch {
      /* ignore */
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
    const vs = await api.vaultStatus();
    if (!vs.exists) {
      try {
        keyringAvailable = await api.keyringAvailable();
      } catch {
        /* ignore */
      }
      view = 'setup';
    } else if (!vs.unlocked) view = 'lock';
    else await unlocked();
  }
  // The normal startup path, once any gpgui migration is resolved: backend check,
  // then boot the vault and wire up the live listeners.
  async function continueStartup(): Promise<void> {
    if (!(await checkBackend())) return; // backend missing → install screen, stop here
    await bootVault();
    render(await api.getState());
    await api.onVpnState((s) => {
      render(s);
    });
    await api.onIdentitiesChanged(async () => {
      try {
        identities = await api.listIdentities();
      } catch {
        /* keep current list */
      }
      renderIdentityUI();
    });
    void refreshBanners(); // version-mismatch / update-available banner (non-blocking)
  }

  // Import screen: pull everything from gpgui and remove the old app, then carry
  // on. On failure the old app is left untouched (see import.rs) and we show why.
  async function doImport(): Promise<void> {
    importBusy = true;
    importError = '';
    try {
      await api.importFromGpgui();
    } catch (e) {
      importError = String(e);
      importBusy = false;
      return;
    }
    importBusy = false;
    await continueStartup();
  }
  // Skip = don't import, but still remove the old app + its data (start fresh).
  async function skipImport(): Promise<void> {
    importBusy = true;
    importError = '';
    try {
      await api.removePredecessor();
    } catch (e) {
      importError = String(e);
      importBusy = false;
      return;
    }
    importBusy = false;
    await continueStartup();
  }

  onMount(() => {
    void (async () => {
      // Fresh install with a predecessor gpgui → offer to import first.
      if (await api.importAvailable()) {
        view = 'import';
        return;
      }
      await continueStartup();
    })();
  });
</script>

<Header {view} {pillText} {stateColor} {dotLive} {hasUpdate} onOpenManager={openManager} onOpenSettings={openSettings} />

<main>
  <!-- ============ IMPORT (migrate from gpgui) ============ -->
  <ImportView show={view === 'import'} busy={importBusy} error={importError} onImport={() => void doImport()} onSkip={() => void skipImport()} />

  <!-- ============ SETUP (first run) ============ -->
  <SetupView show={view === 'setup'} bind:newMasterPin {setupLog} {keyringAvailable} bind:autoUnlock onSetup={() => void doSetup()} />

  <!-- ============ LOCK ============ -->
  <LockView show={view === 'lock'} bind:masterPin {lockLog} bind:resetOpen onUnlock={() => void doUnlock()} onResetConfirm={() => void resetConfirm()} />

  <!-- ============ BACKEND MISSING (privileged service not installed) ============ -->
  <BackendMissing
    show={view === 'backend'}
    outdated={bkOutdated}
    options={bkOptions}
    bind:kind={bkKind}
    option={bkOption}
    {stepDone}
    {copyAllDone}
    {installing}
    status={bkStatus}
    {checking}
    {recheckStyle}
    onKindChange={() => {
      stepDone = [];
    }}
    onInstall={() => void installBackend()}
    onCopyStep={copyStep}
    onCopyAll={copyAll}
    onRecheck={() => void recheck()}
  />

  <!-- ============ UNLOCKED (orb + content; action button lives in the footer) ============ -->
  <div class="view" id="unlockedView" class:show={view === 'unlocked'}>
    <UpdateBanner {banner} />
    <StatusHero {lastKind} {stateColor} {statusSubText} />

    <!-- disconnected / connecting: pick identity -->
    <IdentityPicker hidden={isConnectedView} {identities} {selected} {chips} onPick={pickIdentity} onOpenManager={openManager} />

    <!-- connected: session details -->
    <ConnectedCard hidden={!isConnectedView} {cIp} {cIface} {cPortal} {cGateway} {cExpires} />
  </div>
</main>

<!-- action footer (shown only when unlocked) -->
<ActionFooter hidden={view !== 'unlocked'} {footLog} {actionClass} {actionLabel} visible={actionVisible} {onAction} />
