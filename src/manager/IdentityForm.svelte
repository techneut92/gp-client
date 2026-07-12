<script lang="ts">
  // Identity editor form: name/portal, auth method + detect, the
  // method-specific blocks, gateway switch and save/delete actions.
  import { m } from '../paraglide/messages.js';
  import Dropdown from '../lib/Dropdown.svelte';
  import { deleteIdentity, availableModules, getConfig, hasTauri, listIdentities, probeAuth, saveIdentity, scanCerts, type CertInfo, type Identity, type ProbeResult } from '../lib/api';
  import CertPicker from './CertPicker.svelte';
  import AuthFields from './AuthFields.svelte';

  interface Option {
    value: string;
    label: string;
    sub?: string;
  }

  interface Props {
    identities: Identity[];
    editing: string | null;
  }

  let { identities = $bindable(), editing = $bindable() }: Props = $props();

  // ───────── demo data ─────────
  const DEMO_CERTS: CertInfo[] = [
    { uri: 'pkcs11:id=%01;manufacturer=Yubico', display: 'Jane Doe — YubiKey 5C', id: '01', manufacturer: 'Yubico', slot: 'Slot 01', expiry: '2026-07-10' },
    { uri: 'pkcs11:id=%02;manufacturer=Yubico', display: 'Jane Doe — PIV Authentication', id: '02', manufacturer: 'Yubico', slot: 'Slot 02', expiry: '2027-04-12' },
  ];
  const DEMO_MODULES: string[] = ['/usr/lib/opensc-pkcs11.so', '/usr/lib/pkcs11/p11-kit-proxy.so'];

  let certs = $state<CertInfo[]>([]);
  let asGateway = $state(true);

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
        certs = await scanCerts(moduleValue);
      } catch {
        certs = [];
      }
    } else {
      certs = DEMO_CERTS.slice();
    }
    let pick: CertInfo | undefined = certs[0];
    if (selId) {
      const f = certs.find((c) => c.id === selId && c.manufacturer === selManu);
      if (f) pick = f;
    }
    certValue = pick ? pick.uri : '';
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
        res = await probeAuth({
          portal: portal.trim(),
          certKind: currentCertKind(),
          certUri: cert ? cert.uri : '',
          pin,
          certFile,
          keyFile,
          keyPassword,
          modulePath: moduleValue,
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

  function toggleGateway(): void {
    asGateway = !asGateway;
  }

  // ───────── form ─────────
  export function blankForm(): void {
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
  export async function loadForm(id: Identity): Promise<void> {
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
        await saveIdentity(id);
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
        await deleteIdentity(editing);
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
        identities = await listIdentities();
      } catch {
        return;
      }
    }
  }

  export async function initForm(): Promise<void> {
    if (hasTauri) {
      try {
        const mods = await availableModules();
        if (mods.length) moduleOptions = mods.map(moduleOption);
        const cfg = await getConfig();
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
</script>

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
  <CertPicker hidden={method !== 0} {certs} bind:certValue {moduleOptions} bind:moduleValue bind:pin onModuleChange={() => void rescanCerts()} onRescan={() => void rescanCerts()} />

  <!-- cert file / username + password / saml note -->
  <AuthFields {method} bind:certFile bind:keyFile bind:keyPassword bind:username bind:password />

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
