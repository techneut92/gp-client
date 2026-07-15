<script lang="ts">
  // Smart-card block: PKCS#11 module + certificate pickers, rescan and PIN.
  import { m } from '../paraglide/messages.js';
  import Dropdown from '../lib/Dropdown.svelte';
  import type { CertInfo } from '../lib/api';

  interface Option {
    value: string;
    label: string;
    sub?: string;
  }

  interface Props {
    hidden: boolean;
    certs: CertInfo[];
    certValue: string;
    moduleOptions: Option[];
    moduleValue: string;
    pin: string;
    onModuleChange: () => void;
    onRescan: () => void;
  }

  let { hidden, certs, certValue = $bindable(), moduleOptions, moduleValue = $bindable(), pin = $bindable(), onModuleChange, onRescan }: Props = $props();

  const certOptions = $derived(certs.map((c): Option => ({ value: c.uri, label: c.display, sub: c.slot + ' · ' + m.manager_expires({ date: c.expiry }) })));
  const selectedCert = $derived(certs.find((c) => c.uri === certValue));
  const expirySoon = $derived(selectedCert ? new Date(selectedCert.expiry).getTime() - Date.now() < 30 * 864e5 : false);
</script>

<div id="scBlock" {hidden} style="margin-top:16px;">
  <div class="lbl">{m.manager_module()}</div>
  <div id="moduleDD" class="mb16">
    <Dropdown options={moduleOptions} bind:value={moduleValue} placeholder={m.common_select()} onChange={onModuleChange} />
  </div>
  <div class="lbl">{m.manager_smartcard_cert()}</div>
  <div style="display:flex; gap:8px;">
    <div style="flex:1; min-width:0;" id="certDD">
      <Dropdown options={certOptions} bind:value={certValue} placeholder={m.manager_no_certs()} />
    </div>
    <button class="iconbtn" id="rescan" title={m.manager_rescan_tooltip()} onclick={onRescan}>⟳</button>
  </div>
  <div class="cert-detail" id="certDetail" hidden={!selectedCert}>
    {#if selectedCert}<span>{selectedCert.slot}</span><span class="sep">·</span><span class="exp" class:soon={expirySoon}>{m.manager_expires({ date: selectedCert.expiry })}</span>{/if}
  </div>
  <div class="lbl mt">{m.manager_pin()} <span style="color:var(--hint); font-weight:500; text-transform:none; letter-spacing:0;">{m.manager_stored_encrypted()}</span></div>
  <input class="field" id="pin" type="password" placeholder={m.manager_pin_placeholder()} bind:value={pin} />
</div>
