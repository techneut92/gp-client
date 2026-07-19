<script lang="ts">
  // Single-identity editor window. The identity roster now lives in Settings →
  // Identities; this window edits (or creates) one identity, with its own rail:
  // Details · Authentication · Connection. Opened via open_identity_editor(name?);
  // the target arrives as window.__editIdentity (cold) or an edit-identity event.
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import { m } from '../paraglide/messages.js';
  import {
    deleteIdentity,
    availableModules,
    getConfig,
    listIdentities,
    probeAuth,
    saveIdentity,
    scanCerts,
    type CertInfo,
    type Identity,
    type ProbeResult,
    type UnlistenFn,
  } from '../lib/api';
  import Dropdown from '../lib/Dropdown.svelte';
  import CertPicker from './CertPicker.svelte';
  import AuthFields from './AuthFields.svelte';
  import DnsDomains from './DnsDomains.svelte';
  import { mountShim } from '../lib/shim';

  document.title = m.manager_title();
  mountShim('settings');

  interface Option {
    value: string;
    label: string;
    sub?: string;
  }

  type Section = 'details' | 'auth' | 'conn';
  let section = $state<Section>('details');
  let editing = $state<string | null>(null);

  // ── Details ──
  let name = $state('');
  let portal = $state('');
  let username = $state('');
  let password = $state('');
  let pin = $state('');
  let certFile = $state('');
  let keyFile = $state('');
  let keyPassword = $state('');
  let dnsDomains = $state<string[]>([]);
  let asGateway = $state(true);
  let authValue = $state('-1');
  let moduleValue = $state('');
  let certValue = $state('');
  // Smart-card storage toggles (ON = stored on the identity; OFF = at connect).
  let storeCert = $state(true);
  let storePin = $state(true);
  let certs = $state<CertInfo[]>([]);
  let moduleOptions = $state<Option[]>([]);
  let hint = $state('');
  let dnsEditor: DnsDomains | undefined = $state();

  // ── Authentication (SSO) ──
  let authView = $state('webview');

  // ── Connection ──
  let os = $state('Windows');
  let osVersion = $state('');
  let userAgent = $state('PAN GlobalProtect');
  let clientVersion = $state('');
  let mtu = $state(0);
  let reconnectTimeout = $state(30);
  let forceDpd = $state(0);
  let vpncScript = $state('');
  let localHostname = $state('');
  let disableIpv6 = $state(false);
  let noDtls = $state(false);
  let noXmlpost = $state(false);
  let ignoreTlsErrors = $state(false);
  let mtuText = $state('0');
  let reconnectText = $state('30');
  let forceDpdText = $state('0');

  // ── footer ──
  let log = $state('');
  let logErr = $state(false);

  const authOptions: Option[] = [
    { value: '-1', label: m.manager_auth_auto(), sub: m.manager_auth_auto_sub() },
    { value: '0', label: m.manager_auth_smartcard(), sub: m.manager_auth_smartcard_sub() },
    { value: '1', label: m.manager_auth_certfile(), sub: m.manager_auth_certfile_sub() },
    { value: '2', label: m.manager_auth_saml(), sub: m.manager_auth_saml_sub() },
    { value: '3', label: m.manager_auth_password(), sub: m.manager_auth_password_sub() },
  ];
  const ssoOptions: Option[] = [
    { value: 'webview', label: m.settings_sso_webview(), sub: m.settings_sso_webview_sub() },
    { value: 'browser', label: m.settings_sso_browser(), sub: m.settings_sso_browser_sub() },
  ];
  const osOptions: Option[] = [
    { value: 'Linux', label: 'Linux' },
    { value: 'Windows', label: 'Windows' },
    { value: 'Mac', label: 'Mac' },
  ];

  const method = $derived(Number.parseInt(authValue, 10));
  const titleInitial = $derived((name.trim() || portal.trim() || '+').slice(0, 1).toUpperCase());
  const titleText = $derived(editing ? name.trim() || portal.trim() || m.manager_new_identity() : name.trim() || m.editor_new_title());
  const titleSub = $derived(editing ? m.editor_editing() : m.editor_creating());

  function moduleOption(p: string): Option {
    return { value: p, label: p.split('/').pop() ?? p, sub: p };
  }
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
  function num(raw: string, apply: (n: number) => void): void {
    apply(Number.parseInt(raw, 10) || 0);
  }

  async function rescanCerts(selId?: string, selManu?: string): Promise<void> {
    try {
      certs = await scanCerts(moduleValue);
    } catch {
      certs = [];
    }
    let pick: CertInfo | undefined = certs[0];
    if (selId) {
      const f = certs.find((c) => c.id === selId && c.manufacturer === selManu);
      if (f) pick = f;
    }
    certValue = pick ? pick.uri : '';
  }

  function currentCertKind(): number {
    if (method === 0) return 1;
    if (method === 1) return 2;
    return 0;
  }
  async function detect(): Promise<void> {
    hint = m.manager_detecting();
    let res: ProbeResult;
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
        asGateway,
        os,
        userAgent,
      });
    } catch (e) {
      hint = m.manager_probe_failed({ error: String(e) });
      return;
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
    dnsDomains = [];
    asGateway = true;
    certValue = '';
    storeCert = false;
    storePin = false;
    authView = 'webview';
    os = 'Windows';
    osVersion = '';
    userAgent = 'PAN GlobalProtect';
    clientVersion = '';
    mtu = 0;
    reconnectTimeout = 30;
    forceDpd = 0;
    vpncScript = '';
    localHostname = '';
    disableIpv6 = false;
    noDtls = false;
    noXmlpost = false;
    ignoreTlsErrors = false;
    mtuText = '0';
    reconnectText = '30';
    forceDpdText = '0';
    section = 'details';
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
    // Store toggles: cert stored unless ask_cert; PIN stored only if a PIN is
    // actually present (an older identity that prompted for its PIN has none).
    storeCert = !(id.ask_cert ?? false);
    storePin = !(id.ask_pin ?? false) && !!(id.pin ?? '');
    certFile = id.cert_file ?? '';
    keyFile = id.key_file ?? '';
    keyPassword = id.key_password ?? '';
    dnsDomains = id.dns_domains ?? [];
    if (id.module_path) moduleValue = id.module_path;
    authView = id.auth_view || 'webview';
    os = id.os || 'Windows';
    osVersion = id.os_version ?? '';
    userAgent = id.user_agent || 'PAN GlobalProtect';
    clientVersion = id.client_version ?? '';
    mtu = id.mtu ?? 0;
    reconnectTimeout = id.reconnect_timeout ?? 30;
    forceDpd = id.force_dpd ?? 0;
    vpncScript = id.vpnc_script ?? '';
    localHostname = id.local_hostname ?? '';
    disableIpv6 = !!id.disable_ipv6;
    noDtls = !!id.no_dtls;
    noXmlpost = !!id.no_xmlpost;
    ignoreTlsErrors = !!id.ignore_tls_errors;
    mtuText = String(mtu);
    reconnectText = String(reconnectTimeout);
    forceDpdText = String(forceDpd);
    section = 'details';
    hint = '';
    setLog('', false);
    setMethod(typeof id.auth_method === 'number' ? id.auth_method : -1);
    await rescanCerts(id.cert_id, id.cert_manufacturer);
  }

  async function loadTarget(target: string): Promise<void> {
    if (!target) {
      blankForm();
      return;
    }
    try {
      const list = await listIdentities();
      const found = list.find((i) => i.name === target);
      if (found) await loadForm(found);
      else blankForm();
    } catch {
      blankForm();
    }
  }

  async function save(): Promise<void> {
    dnsEditor?.flush();
    const nm = name.trim() || portal.trim();
    if (!nm) {
      setLog(m.manager_log_need_name(), true);
      section = 'details';
      return;
    }
    const cert = certs.find((c) => c.uri === certValue);
    // Smart-card storage: only persist the cert / PIN when their toggle is on, so
    // the emptiness of cert_id / pin always reflects the user's choice.
    const scStoreCert = method === 0 && storeCert;
    const scStorePin = scStoreCert && storePin;
    const id: Identity = {
      name: nm,
      portal: portal.trim(),
      auth_method: method,
      as_gateway: asGateway,
      module_path: moduleValue,
      username,
      password,
      pin: scStorePin ? pin : '',
      cert_id: scStoreCert && cert ? cert.id : '',
      cert_manufacturer: scStoreCert && cert ? cert.manufacturer : '',
      ask_cert: method === 0 && !storeCert,
      ask_pin: method === 0 && (storeCert ? !storePin : true),
      cert_file: certFile,
      key_file: keyFile,
      key_password: keyPassword,
      dns_domains: dnsDomains,
      auth_view: authView,
      os,
      os_version: osVersion,
      user_agent: userAgent,
      client_version: clientVersion,
      mtu,
      reconnect_timeout: reconnectTimeout,
      force_dpd: forceDpd,
      vpnc_script: vpncScript,
      local_hostname: localHostname,
      disable_ipv6: disableIpv6,
      no_dtls: noDtls,
      no_xmlpost: noXmlpost,
      ignore_tls_errors: ignoreTlsErrors,
    };
    try {
      await saveIdentity(id);
      // Rename: the vault keys identities by name, so the new name creates a new
      // entry — remove the old one so editing never leaves a duplicate (GPC-31).
      if (editing && editing !== nm) {
        await deleteIdentity(editing);
      }
    } catch (e) {
      setLog(m.manager_error({ error: String(e) }), true);
      return;
    }
    // Saving closes the editor window; the roster refreshes via identities-changed.
    void getCurrentWindow().close();
  }

  async function del(): Promise<void> {
    if (editing) {
      try {
        await deleteIdentity(editing);
      } catch {
        /* ignore */
      }
    }
    void getCurrentWindow().close();
  }
  function close(): void {
    void getCurrentWindow().close();
  }

  onMount(() => {
    let unlisten: UnlistenFn | undefined;
    void (async () => {
      try {
        const mods = await availableModules();
        if (mods.length) moduleOptions = mods.map(moduleOption);
        const cfg = await getConfig();
        const mp = cfg.module_path;
        moduleValue = typeof mp === 'string' && mp && mods.includes(mp) ? mp : mods[0] ?? '';
      } catch {
        /* ignore */
      }
      const g = (window as Window & { __editIdentity?: unknown }).__editIdentity;
      await loadTarget(typeof g === 'string' ? g : '');
    })();
    void listen<string>('edit-identity', (e) => void loadTarget(e.payload || '')).then((f) => {
      unlisten = f;
    });
    return () => unlisten?.();
  });
</script>

<header data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <span class="idn-avatar">{titleInitial}</span>
    <div style="min-width:0;">
      <div class="title" style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">{titleText}</div>
      <div class="idn-sub">{titleSub}</div>
    </div>
  </div>
  <div class="winctl">
    <button class="wbtn close" id="winClose" title={m.common_close()} onclick={close}>
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M5 5l14 14M19 5L5 19"/></svg>
    </button>
  </div>
</header>

<div class="settings-layout">
  <nav class="settings-rail">
    <button class="settings-nav" class:active={section === 'details'} data-sec="details" onclick={() => (section = 'details')}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="8" r="3.4"/><path d="M5 20a7 7 0 0 1 14 0"/></svg>
      <span>{m.editor_details()}</span>
    </button>
    <button class="settings-nav" class:active={section === 'auth'} data-sec="auth" onclick={() => (section = 'auth')}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="10" width="16" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/></svg>
      <span>{m.settings_nav_auth()}</span>
    </button>
    <button class="settings-nav" class:active={section === 'conn'} data-sec="conn" onclick={() => (section = 'conn')}>
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5a7 7 0 0 1 14 0"/><path d="M8.5 15a3.5 3.5 0 0 1 7 0"/><circle cx="12" cy="18.5" r="1.3" fill="currentColor" stroke="none"/></svg>
      <span>{m.settings_nav_connection()}</span>
    </button>
  </nav>

  <div class="settings-content">
    <!-- DETAILS -->
    <div class="settings-sec" id="sec-details" hidden={section !== 'details'}>
      <h2>{m.editor_details()}</h2>
      <p class="desc">{m.editor_details_desc()}</p>

      <div class="lbl">{m.manager_name()}</div>
      <input class="field mb14" id="name" placeholder={m.manager_name_placeholder()} bind:value={name} />

      <div class="lbl">{m.manager_portal()}</div>
      <div class="field-icon mb14">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#5d6378" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.5 2.5 2.5 15 0 18M12 3c-2.5 2.5-2.5 15 0 18"/></svg>
        <input class="field" id="portal" placeholder={m.manager_portal_placeholder()} bind:value={portal} />
      </div>

      <div class="lbl">{m.manager_authentication()}</div>
      <div style="display:flex; align-items:stretch;">
        <div style="flex:1; min-width:0;" id="authDD">
          <Dropdown options={authOptions} bind:value={authValue} onChange={syncHint} />
        </div>
        <button class="detect-btn" id="detectBtn" style="margin-left:8px;" title={m.manager_detect_tooltip()} onclick={detect}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.2-3.2"/></svg>
          {m.manager_detect()}
        </button>
      </div>
      <div class="cert-detail" id="authHint" style="margin-top:9px;">{hint}</div>

      <CertPicker hidden={method !== 0} {certs} bind:certValue {moduleOptions} bind:moduleValue bind:pin bind:storeCert bind:storePin onModuleChange={() => void rescanCerts()} onRescan={() => void rescanCerts()} />
      <AuthFields {method} bind:certFile bind:keyFile bind:keyPassword bind:username bind:password />

      <div class="switch-row">
        <div>
          <div class="t">{m.manager_as_gateway()}</div>
          <div class="d">{m.manager_as_gateway_sub()}</div>
        </div>
        <button class="switch" class:on={asGateway} id="asGateway" type="button" aria-pressed={asGateway} aria-label={m.manager_as_gateway()} onclick={() => (asGateway = !asGateway)}><span class="knob"></span></button>
      </div>
    </div>

    <!-- AUTHENTICATION -->
    <div class="settings-sec" id="sec-auth" hidden={section !== 'auth'}>
      <h2>{m.settings_nav_auth()}</h2>
      <p class="desc">{m.settings_auth_desc()}</p>
      <div class="lbl">{m.settings_sso_method()}</div>
      <Dropdown options={ssoOptions} bind:value={authView} placeholder={m.common_select()} />
      <p class="help">{m.settings_sso_help()}</p>
    </div>

    <!-- CONNECTION -->
    <div class="settings-sec" id="sec-conn" hidden={section !== 'conn'}>
      <h2>{m.settings_nav_connection()}</h2>
      <p class="desc">{m.settings_conn_desc()}</p>

      <div class="grid2 mb14">
        <div><div class="lbl">{m.settings_lbl_os()}</div><Dropdown options={osOptions} bind:value={os} placeholder={m.common_select()} /></div>
        <div><div class="lbl">{m.settings_lbl_os_version()}</div><input class="field" id="osVersion" placeholder={m.settings_ph_auto()} bind:value={osVersion} /></div>
      </div>

      <div class="lbl">{m.settings_lbl_user_agent()}</div>
      <input class="field mb14" id="userAgent" bind:value={userAgent} />

      <div class="lbl">{m.settings_lbl_client_version()}</div>
      <input class="field mb16" id="clientVersion" placeholder={m.settings_ph_default()} bind:value={clientVersion} />

      <div class="grid3 mb16">
        <div><div class="lbl">{m.settings_lbl_mtu()}</div><input class="field" id="mtu" type="number" min="0" value={mtuText} oninput={(e) => { mtuText = e.currentTarget.value; num(mtuText, (n) => (mtu = n)); }} /></div>
        <div><div class="lbl">{m.settings_lbl_reconnect()}</div><input class="field" id="reconnectTimeout" type="number" min="0" value={reconnectText} oninput={(e) => { reconnectText = e.currentTarget.value; num(reconnectText, (n) => (reconnectTimeout = n)); }} /></div>
        <div><div class="lbl">{m.settings_lbl_force_dpd()}</div><input class="field" id="forceDpd" type="number" min="0" value={forceDpdText} oninput={(e) => { forceDpdText = e.currentTarget.value; num(forceDpdText, (n) => (forceDpd = n)); }} /></div>
      </div>

      <div class="lbl">{m.settings_lbl_vpnc_script()}</div>
      <input class="field mb14" id="vpncScript" placeholder="/usr/libexec/openconnect/vpnc-script" bind:value={vpncScript} />

      <div class="lbl">{m.settings_lbl_local_hostname()}</div>
      <input class="field mb18" id="localHostname" placeholder={m.settings_ph_auto()} bind:value={localHostname} />

      <div class="toggle-list">
        <div class="toggle-item">
          <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;">{m.settings_disable_ipv6()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.settings_disable_ipv6_desc()}</div></div>
          <button class="switch" class:on={disableIpv6} type="button" data-key="disableIpv6" aria-label={m.settings_disable_ipv6()} onclick={() => (disableIpv6 = !disableIpv6)}><span class="knob"></span></button>
        </div>
        <div class="toggle-item">
          <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;">{m.settings_no_dtls()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.settings_no_dtls_desc()}</div></div>
          <button class="switch" class:on={noDtls} type="button" data-key="noDtls" aria-label={m.settings_no_dtls()} onclick={() => (noDtls = !noDtls)}><span class="knob"></span></button>
        </div>
        <div class="toggle-item">
          <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;">{m.settings_no_xmlpost()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.settings_no_xmlpost_desc()}</div></div>
          <button class="switch" class:on={noXmlpost} type="button" data-key="noXmlpost" aria-label={m.settings_no_xmlpost()} onclick={() => (noXmlpost = !noXmlpost)}><span class="knob"></span></button>
        </div>
        <div class="toggle-item">
          <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;">{m.settings_ignore_tls()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.settings_ignore_tls_desc()}</div></div>
          <button class="switch" class:on={ignoreTlsErrors} type="button" data-key="ignoreTlsErrors" aria-label={m.settings_ignore_tls()} onclick={() => (ignoreTlsErrors = !ignoreTlsErrors)}><span class="knob"></span></button>
        </div>
      </div>

      <div style="margin-top:16px;">
        <DnsDomains bind:this={dnsEditor} bind:domains={dnsDomains} />
      </div>
    </div>
  </div>
</div>

<footer class="idn-footer">
  <p class="formlog" class:err={logErr} id="log" style="flex:1; min-width:0; margin:0; text-align:left;">{log}</p>
  <button class="btn danger" id="delBtn" onclick={del}>{editing ? m.manager_delete() : m.manager_clear()}</button>
  <button class="btn-action" id="saveBtn" onclick={save}>{m.manager_save_identity()}</button>
</footer>
