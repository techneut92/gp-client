<script lang="ts">
  // manager window — port of gpgui ui/manager.html (identities / connection manager).
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { m } from '../paraglide/messages.js';
  import Dropdown from '../lib/Dropdown.svelte';

  interface Option {
    value: string;
    label: string;
    sub?: string;
  }

  interface Cert {
    uri: string;
    display: string;
    id: string;
    manufacturer: string;
    slot: string;
    expiry: string;
  }

  interface Identity {
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

  interface ProbeResult {
    kind: string;
    message?: string;
    usernameLabel?: string;
    passwordLabel?: string;
    supportsBrowser?: boolean;
  }

  interface VaultStatus {
    unlocked: boolean;
  }

  interface Config {
    module_path?: string | null;
  }

  const hasTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // The original page was styled off <body class="settings"> with the layout as
  // direct body children; replicate that around the #app mount point.
  document.title = m.manager_title();
  document.body.classList.add('settings');
  const mountEl = document.getElementById('app');
  if (mountEl) mountEl.style.display = 'contents';

  const SEARCH_ICON = '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.2-3.2"/>';
  const AUTH_ICONS: Record<string, string> = {
    '0': '<rect x="2" y="5" width="20" height="14" rx="2.5"/><path d="M2 10h20"/>',
    '1': '<path d="M14 3v4a1 1 0 0 0 1 1h4"/><path d="M17 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2z"/>',
    '2': '<path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/><path d="M10 17l5-5-5-5M15 12H3"/>',
    '3': '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
    '-1': SEARCH_ICON,
  };

  function authIcon(method: number | undefined, size: number = 15): string {
    const body = AUTH_ICONS[String(method)] ?? SEARCH_ICON;
    return (
      '<svg width="' + size + '" height="' + size +
      '" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">' +
      body + '</svg>'
    );
  }

  // ───────── demo data ─────────
  const DEMO_CERTS: Cert[] = [
    { uri: 'pkcs11:id=%01;manufacturer=Yubico', display: 'Jane Doe — YubiKey 5C', id: '01', manufacturer: 'Yubico', slot: 'Slot 01', expiry: '2026-07-10' },
    { uri: 'pkcs11:id=%02;manufacturer=Yubico', display: 'Jane Doe — PIV Authentication', id: '02', manufacturer: 'Yubico', slot: 'Slot 02', expiry: '2027-04-12' },
  ];
  const DEMO_MODULES: string[] = ['/usr/lib/opensc-pkcs11.so', '/usr/lib/pkcs11/p11-kit-proxy.so'];

  let identities = $state<Identity[]>([
    { name: 'Acme Corp', portal: 'vpn.acme-corp.com', auth_method: 0, as_gateway: true, module_path: '/usr/lib/opensc-pkcs11.so', cert_id: '01', cert_manufacturer: 'Yubico' },
    { name: 'Personal Lab', portal: 'lab.example.net', auth_method: 3, as_gateway: false, username: 'jane' },
  ]);
  let certs = $state<Cert[]>([]);
  let editing = $state<string | null>(null);
  let asGateway = $state(true);
  let locked = $state(false);

  // form fields
  let name = $state('');
  let portal = $state('');
  let username = $state('');
  let password = $state('');
  let pin = $state('');
  let certFile = $state('');
  let keyFile = $state('');
  let keyPassword = $state('');

  let hint = $state('');
  let log = $state('');
  let logErr = $state(false);

  // ───────── dropdowns ─────────
  const authOptions: Option[] = [
    { value: '-1', label: m.manager_auth_auto(), sub: m.manager_auth_auto_sub() },
    { value: '0', label: m.manager_auth_smartcard(), sub: m.manager_auth_smartcard_sub() },
    { value: '1', label: m.manager_auth_certfile(), sub: m.manager_auth_certfile_sub() },
    { value: '2', label: m.manager_auth_saml(), sub: m.manager_auth_saml_sub() },
    { value: '3', label: m.manager_auth_password(), sub: m.manager_auth_password_sub() },
  ];
  let authValue = $state('-1');

  function moduleOption(p: string): Option {
    return { value: p, label: p.split('/').pop() ?? p, sub: p };
  }
  let moduleOptions = $state<Option[]>(DEMO_MODULES.map(moduleOption));
  let moduleValue = $state(DEMO_MODULES[0] ?? '');

  let certValue = $state('');
  const certOptions = $derived(certs.map((c): Option => ({ value: c.uri, label: c.display, sub: c.slot + ' · ' + m.manager_expires({ date: c.expiry }) })));
  const selectedCert = $derived(certs.find((c) => c.uri === certValue));
  const expirySoon = $derived(selectedCert ? new Date(selectedCert.expiry).getTime() - Date.now() < 30 * 864e5 : false);

  const method = $derived(Number.parseInt(authValue, 10));

  function setMethod(n: number): void {
    authValue = String(n);
    syncHint();
  }
  function syncHint(): void {
    if (method === -1 && !hint) hint = m.manager_hint_detect();
    else if (method !== -1) hint = '';
  }
  function setLog(text: string, err: boolean): void {
    log = text;
    logErr = err;
  }

  async function rescanCerts(selId?: string, selManu?: string): Promise<void> {
    if (hasTauri) {
      try {
        certs = await invoke<Cert[]>('scan_certs', { module: moduleValue });
      } catch {
        certs = [];
      }
    } else {
      certs = DEMO_CERTS.slice();
    }
    let pick: Cert | undefined = certs[0];
    if (selId) {
      const f = certs.find((c) => c.id === selId && c.manufacturer === selManu);
      if (f) pick = f;
    }
    certValue = pick ? pick.uri : '';
  }

  async function browseCert(): Promise<void> {
    if (hasTauri) {
      const p = await invoke<string | null>('browse_file', { title: m.manager_browse_cert_title() });
      if (p) certFile = p;
    } else {
      certFile = '/home/jane/certs/client.p12';
    }
  }
  async function browseKey(): Promise<void> {
    if (hasTauri) {
      const p = await invoke<string | null>('browse_file', { title: m.manager_browse_key_title() });
      if (p) keyFile = p;
    } else {
      keyFile = '/home/jane/certs/client-key.pem';
    }
  }
  function toggleGateway(): void {
    asGateway = !asGateway;
  }
  function closeWindow(): void {
    if (hasTauri) void getCurrentWindow().close();
    else window.close();
  }

  // ───────── detect ─────────
  function currentCertKind(): number {
    if (method === 0) return 1;
    if (method === 1) return 2;
    return 0;
  }
  async function detect(): Promise<void> {
    hint = m.manager_detecting();
    let res: ProbeResult;
    if (hasTauri) {
      const cert = certs.find((c) => c.uri === certValue);
      try {
        res = await invoke<ProbeResult>('probe_auth', {
          form: {
            portal: portal.trim(),
            certKind: currentCertKind(),
            certUri: cert ? cert.uri : '',
            pin,
            certFile,
            keyFile,
            keyPassword,
            modulePath: moduleValue,
          },
        });
      } catch (e) {
        hint = m.manager_probe_failed({ error: String(e) });
        return;
      }
    } else {
      await new Promise((r) => setTimeout(r, 700));
      res = portal.includes('lab')
        ? { kind: 'standard', usernameLabel: 'Username', passwordLabel: 'Password' }
        : { kind: 'cert' };
    }
    if (res.kind === 'error') {
      hint = m.manager_error({ error: String(res.message ?? '') });
    } else if (res.kind === 'cert') {
      setMethod(0);
      await rescanCerts();
      hint = m.manager_hint_cert_required();
    } else if (res.kind === 'standard') {
      setMethod(3);
      hint = m.manager_hint_standard({ username: res.usernameLabel || '?', password: res.passwordLabel || '?' });
    } else {
      setMethod(2);
      hint = res.supportsBrowser ? m.manager_hint_saml_browser() : m.manager_hint_saml();
    }
  }

  // ───────── form ─────────
  function blankForm(): void {
    editing = null;
    name = '';
    portal = '';
    username = '';
    password = '';
    pin = '';
    certFile = '';
    keyFile = '';
    keyPassword = '';
    asGateway = true;
    setMethod(-1);
    hint = m.manager_hint_detect();
    setLog('', false);
    void rescanCerts();
  }
  async function loadForm(id: Identity): Promise<void> {
    editing = id.name;
    name = id.name;
    portal = id.portal ?? '';
    asGateway = !!id.as_gateway;
    username = id.username ?? '';
    password = id.password ?? '';
    pin = id.pin ?? '';
    certFile = id.cert_file ?? '';
    keyFile = id.key_file ?? '';
    keyPassword = id.key_password ?? '';
    if (id.module_path) moduleValue = id.module_path;
    setMethod(typeof id.auth_method === 'number' ? id.auth_method : -1);
    hint = '';
    setLog('', false);
    await rescanCerts(id.cert_id, id.cert_manufacturer);
  }

  async function save(): Promise<void> {
    const nm = name.trim() || portal.trim();
    if (!nm) {
      setLog(m.manager_log_need_name(), true);
      return;
    }
    const cert = certs.find((c) => c.uri === certValue);
    const id: Identity = {
      name: nm,
      portal: portal.trim(),
      auth_method: method,
      as_gateway: asGateway,
      module_path: moduleValue,
      username,
      password,
      pin,
      cert_id: cert ? cert.id : '',
      cert_manufacturer: cert ? cert.manufacturer : '',
      cert_file: certFile,
      key_file: keyFile,
      key_password: keyPassword,
    };
    if (hasTauri) {
      try {
        await invoke('save_identity', { identity: id });
      } catch (e) {
        setLog(m.manager_error({ error: String(e) }), true);
        return;
      }
    } else {
      const i = identities.findIndex((x) => x.name === nm);
      if (i >= 0) identities[i] = id;
      else identities.push(id);
    }
    editing = nm;
    await load();
    setLog(m.manager_log_saved(), false);
  }
  async function del(): Promise<void> {
    if (!editing) {
      blankForm();
      return;
    }
    if (hasTauri) {
      try {
        await invoke('delete_identity', { name: editing });
      } catch {
        // ignore
      }
    } else {
      identities = identities.filter((x) => x.name !== editing);
    }
    await load();
    blankForm();
  }

  async function load(): Promise<void> {
    if (hasTauri) {
      try {
        identities = await invoke<Identity[]>('list_identities');
      } catch {
        return;
      }
    }
  }

  async function init(): Promise<void> {
    if (hasTauri) {
      const vs = await invoke<VaultStatus>('vault_status');
      if (!vs.unlocked) {
        locked = true;
        return;
      }
      try {
        const mods = await invoke<string[]>('available_modules');
        if (mods.length) moduleOptions = mods.map(moduleOption);
        const cfg = await invoke<Config>('get_config');
        moduleValue = cfg.module_path && mods.includes(cfg.module_path) ? cfg.module_path : (mods[0] ?? '');
      } catch {
        // ignore
      }
    }
    await load();
    const first = identities[0];
    if (first) void loadForm(first);
    else blankForm();
  }
  void init();
</script>

<header data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#6fb6ff" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="8" r="3.2"/><path d="M3.5 20a5.5 5.5 0 0 1 11 0"/><path d="M16 4.5a3 3 0 0 1 0 6M18 20a5.5 5.5 0 0 0-3-4.9"/></svg>
    <div class="title">{m.manager_title()}</div>
  </div>
  <div class="winctl">
    <button class="wbtn close" id="winClose" title={m.common_close()} onclick={closeWindow}>
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M5 5l14 14M19 5L5 19"/></svg>
    </button>
  </div>
</header>

<!-- locked notice -->
<div id="locked" class="empty-state" hidden={!locked} style="flex:1;">
  <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#5d6378" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="10" width="16" height="11" rx="2.5"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/></svg>
  <div class="h2">{m.manager_vault_locked()}</div>
  <div class="sub" style="margin:0;">{m.manager_vault_locked_sub()}</div>
</div>

<div class="settings-layout" id="ui" hidden={locked}>
  <nav class="settings-rail" style="flex-basis:210px;">
    <button class="rail-add" id="newBtn" onclick={blankForm}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>
      {m.manager_new_identity()}
    </button>
    <div id="idList">
      {#each identities as id (id.name)}
        <button type="button" class="id-item" class:active={editing === id.name} onclick={() => void loadForm(id)}>
          <span class="ico">{@html authIcon(id.auth_method, 15)}</span><span class="txt"><span class="nm">{id.name}</span><span class="pt">{id.portal || ''}</span></span>
        </button>
      {/each}
    </div>
  </nav>

  <div class="settings-content">
    <div id="form">
      <div class="lbl">{m.manager_name()}</div>
      <input class="field mb14" id="name" placeholder={m.manager_name_placeholder()} bind:value={name} />

      <div class="lbl">{m.manager_portal()}</div>
      <div class="field-icon mb14">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#5d6378" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.5 2.5 2.5 15 0 18M12 3c-2.5 2.5-2.5 15 0 18"/></svg>
        <input class="field" id="portal" placeholder={m.manager_portal_placeholder()} bind:value={portal} />
      </div>

      <div class="lbl">{m.manager_authentication()}</div>
      <div style="display:flex; gap:8px; align-items:stretch;">
        <div style="flex:1; min-width:0;" id="authDD">
          <Dropdown options={authOptions} bind:value={authValue} onChange={syncHint} />
        </div>
        <button class="detect-btn" id="detectBtn" title={m.manager_detect_tooltip()} onclick={detect}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.2-3.2"/></svg>
          {m.manager_detect()}
        </button>
      </div>
      <div class="cert-detail" id="authHint" style="margin-top:9px;">{hint}</div>

      <!-- smart card -->
      <div id="scBlock" hidden={method !== 0} style="margin-top:16px;">
        <div class="lbl">{m.manager_module()}</div>
        <div id="moduleDD" class="mb16">
          <Dropdown options={moduleOptions} bind:value={moduleValue} placeholder={m.common_select()} onChange={() => void rescanCerts()} />
        </div>
        <div class="lbl">{m.manager_smartcard_cert()}</div>
        <div style="display:flex; gap:8px;">
          <div style="flex:1; min-width:0;" id="certDD">
            <Dropdown options={certOptions} bind:value={certValue} placeholder={m.manager_no_certs()} />
          </div>
          <button class="iconbtn" id="rescan" title={m.manager_rescan_tooltip()} onclick={() => void rescanCerts()}>⟳</button>
        </div>
        <div class="cert-detail" id="certDetail" hidden={!selectedCert}>
          {#if selectedCert}<span>{selectedCert.slot}</span><span class="sep">·</span><span class="exp" class:soon={expirySoon}>{m.manager_expires({ date: selectedCert.expiry })}</span>{/if}
        </div>
        <div class="lbl mt">{m.manager_pin()} <span style="color:var(--hint); font-weight:500; text-transform:none; letter-spacing:0;">{m.manager_stored_encrypted()}</span></div>
        <input class="field" id="pin" type="password" placeholder={m.manager_pin_placeholder()} bind:value={pin} />
      </div>

      <!-- cert file -->
      <div id="fileBlock" hidden={method !== 1} style="margin-top:16px;">
        <div class="lbl">{m.manager_certfile_label()}</div>
        <div style="display:flex; gap:8px;" class="mb14">
          <input class="field mono" id="certFile" style="flex:1; min-width:0;" placeholder={m.manager_certfile_placeholder()} bind:value={certFile} />
          <button class="btn-soft" id="browseCert" type="button" onclick={browseCert}>{m.manager_browse()}</button>
        </div>
        <div class="lbl">{m.manager_keyfile_label()}</div>
        <div style="display:flex; gap:8px;" class="mb14">
          <input class="field mono" id="keyFile" style="flex:1; min-width:0;" placeholder={m.manager_keyfile_placeholder()} bind:value={keyFile} />
          <button class="btn-soft" id="browseKey" type="button" onclick={browseKey}>{m.manager_browse()}</button>
        </div>
        <div class="lbl">{m.manager_key_passphrase()} <span style="color:var(--hint); font-weight:500; text-transform:none; letter-spacing:0;">{m.manager_stored_encrypted()}</span></div>
        <input class="field" id="keyPassword" type="password" bind:value={keyPassword} />
      </div>

      <!-- username + password -->
      <div id="userBlock" hidden={method !== 3} style="margin-top:16px;">
        <div class="lbl">{m.manager_username()}</div>
        <input class="field mb14" id="username" placeholder={m.manager_username_placeholder()} bind:value={username} />
        <div class="lbl">{m.manager_password()} <span style="color:var(--hint); font-weight:500; text-transform:none; letter-spacing:0;">{m.manager_stored_encrypted()}</span></div>
        <input class="field" id="password" type="password" bind:value={password} />
      </div>

      <!-- saml note -->
      <div id="samlBlock" hidden={method !== 2} style="margin-top:16px;">
        <div class="note">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#6fb6ff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/><path d="M10 17l5-5-5-5M15 12H3"/></svg>
          <div><div class="h">{m.manager_sso_title()}</div><div class="p">{m.manager_sso_note()}</div></div>
        </div>
      </div>

      <div class="switch-row">
        <div>
          <div class="t">{m.manager_as_gateway()}</div>
          <div class="d">{m.manager_as_gateway_sub()}</div>
        </div>
        <button class="switch" class:on={asGateway} id="asGateway" type="button" aria-pressed={asGateway} aria-label={m.manager_as_gateway()} onclick={toggleGateway}><span class="knob"></span></button>
      </div>

      <div class="id-actions">
        <button class="btn-action" id="saveBtn" onclick={save}>{m.manager_save_identity()}</button>
        <button class="btn danger" id="delBtn" onclick={del}>{editing ? m.manager_delete() : m.manager_clear()}</button>
      </div>
      <p class="formlog" class:err={logErr} id="log" style="text-align:left; margin-top:10px;">{log}</p>
    </div>
  </div>
</div>
