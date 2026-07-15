<script lang="ts">
  // Method-specific field blocks: certificate file, username/password and
  // the SAML/SSO note. Which block shows follows the selected auth method.
  import { m } from '../paraglide/messages.js';
  import { browseFile, hasTauri } from '../lib/api';

  interface Props {
    method: number;
    certFile: string;
    keyFile: string;
    keyPassword: string;
    username: string;
    password: string;
  }

  let { method, certFile = $bindable(), keyFile = $bindable(), keyPassword = $bindable(), username = $bindable(), password = $bindable() }: Props = $props();

  async function browseCert(): Promise<void> {
    if (hasTauri) {
      const p = await browseFile(m.manager_browse_cert_title());
      if (p) certFile = p;
    } else {
      certFile = '/home/jane/certs/client.p12';
    }
  }
  async function browseKey(): Promise<void> {
    if (hasTauri) {
      const p = await browseFile(m.manager_browse_key_title());
      if (p) keyFile = p;
    } else {
      keyFile = '/home/jane/certs/client-key.pem';
    }
  }
</script>

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
