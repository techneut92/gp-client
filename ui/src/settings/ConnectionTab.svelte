<script lang="ts">
  // Connection section: OpenConnect tunables (spoofed OS/UA, MTU, flags…).
  import { m } from '../paraglide/messages.js';
  import Dropdown from '../lib/Dropdown.svelte';
  import type { SettingsBoolKey, SettingsForm } from '../lib/api';

  interface Props {
    hidden: boolean;
    model: SettingsForm;
    // Raw text shown in the number inputs (the original never rewrote the
    // field while typing; the model gets the parsed value).
    mtuText: string;
    reconnectText: string;
    forceDpdText: string;
    onSave: () => void;
  }

  let { hidden, model = $bindable(), mtuText = $bindable(), reconnectText = $bindable(), forceDpdText = $bindable(), onSave }: Props = $props();

  const osOptions = [
    { value: 'Linux', label: 'Linux' },
    { value: 'Windows', label: 'Windows' },
    { value: 'Mac', label: 'Mac' },
  ];

  function toggleKey(k: SettingsBoolKey): void {
    model[k] = !model[k];
    void onSave();
  }
  function numInput(raw: string, apply: (n: number) => void): void {
    apply(parseInt(raw, 10) || 0);
    void onSave();
  }
</script>

<div class="settings-sec" id="sec-conn" {hidden}>
  <h2>{m.settings_nav_connection()}</h2>
  <p class="desc">{m.settings_conn_desc()}</p>

  <div class="grid2 mb14">
    <div><div class="lbl">{m.settings_lbl_os()}</div><div id="osDD"><Dropdown options={osOptions} bind:value={model.os} placeholder={m.common_select()} onChange={() => void onSave()} /></div></div>
    <div><div class="lbl">{m.settings_lbl_os_version()}</div><input class="field" id="osVersion" placeholder={m.settings_ph_auto()} value={model.osVersion} oninput={(e) => { model.osVersion = e.currentTarget.value; void onSave(); }} /></div>
  </div>

  <div class="lbl">{m.settings_lbl_user_agent()}</div>
  <input class="field mb14" id="userAgent" value={model.userAgent} oninput={(e) => { model.userAgent = e.currentTarget.value; void onSave(); }} />

  <div class="lbl">{m.settings_lbl_client_version()}</div>
  <input class="field mb16" id="clientVersion" placeholder={m.settings_ph_default()} value={model.clientVersion} oninput={(e) => { model.clientVersion = e.currentTarget.value; void onSave(); }} />

  <div class="grid3 mb16">
    <div><div class="lbl">{m.settings_lbl_mtu()}</div><input class="field" id="mtu" type="number" min="0" value={mtuText} oninput={(e) => { mtuText = e.currentTarget.value; numInput(mtuText, (n) => { model.mtu = n; }); }} /></div>
    <div><div class="lbl">{m.settings_lbl_reconnect()}</div><input class="field" id="reconnectTimeout" type="number" min="0" value={reconnectText} oninput={(e) => { reconnectText = e.currentTarget.value; numInput(reconnectText, (n) => { model.reconnectTimeout = n; }); }} /></div>
    <div><div class="lbl">{m.settings_lbl_force_dpd()}</div><input class="field" id="forceDpd" type="number" min="0" value={forceDpdText} oninput={(e) => { forceDpdText = e.currentTarget.value; numInput(forceDpdText, (n) => { model.forceDpd = n; }); }} /></div>
  </div>

  <div class="lbl">{m.settings_lbl_vpnc_script()}</div>
  <input class="field mb14" id="vpncScript" placeholder="/usr/libexec/openconnect/vpnc-script" value={model.vpncScript} oninput={(e) => { model.vpncScript = e.currentTarget.value; void onSave(); }} />

  <div class="lbl">{m.settings_lbl_local_hostname()}</div>
  <input class="field mb18" id="localHostname" placeholder={m.settings_ph_auto()} value={model.localHostname} oninput={(e) => { model.localHostname = e.currentTarget.value; void onSave(); }} />

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
