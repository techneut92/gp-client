<script lang="ts">
  // manager window — port of gpgui ui/manager.html (identities / connection
  // manager). App.svelte is the shell (header, locked notice, layout); the
  // rail and the editor form live in sibling components.
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { m } from '../paraglide/messages.js';
  import { hasTauri, vaultStatus, type Identity } from '../lib/api';
  import { mountShim } from '../lib/shim';
  import IdentityRail from './IdentityRail.svelte';
  import IdentityForm from './IdentityForm.svelte';

  // The original page was styled off <body class="settings"> with the layout as
  // direct body children; replicate that around the #app mount point.
  document.title = m.manager_title();
  mountShim('settings');

  // ───────── demo data ─────────
  let identities = $state<Identity[]>([
    { name: 'Acme Corp', portal: 'vpn.acme-corp.com', auth_method: 0, as_gateway: true, module_path: '/usr/lib/opensc-pkcs11.so', cert_id: '01', cert_manufacturer: 'Yubico' },
    { name: 'Personal Lab', portal: 'lab.example.net', auth_method: 3, as_gateway: false, username: 'jane' },
  ]);
  let editing = $state<string | null>(null);
  let locked = $state(false);
  let formRef = $state<ReturnType<typeof IdentityForm> | undefined>(undefined);

  function closeWindow(): void {
    if (hasTauri) void getCurrentWindow().close();
    else window.close();
  }

  async function init(): Promise<void> {
    if (hasTauri) {
      const vs = await vaultStatus();
      if (!vs.unlocked) {
        locked = true;
        return;
      }
    }
    await formRef?.initForm();
  }
  onMount(() => {
    void init();
  });
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
  <IdentityRail {identities} {editing} onNew={() => formRef?.blankForm()} onSelect={(id) => void formRef?.loadForm(id)} />

  <div class="settings-content">
    <IdentityForm bind:identities bind:editing bind:this={formRef} />
  </div>
</div>
