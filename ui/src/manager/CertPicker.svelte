<script lang="ts">
  // Smart-card block: PKCS#11 module + a "Store certificate" / "Store PIN" flow.
  //  - Store certificate off → the cert is picked at connect time (no picker here).
  //  - Store certificate on  → cert picker; once a cert is chosen, a "Store PIN"
  //                            toggle appears; on → the PIN field appears.
  // Newly-revealed rows slide in (.idn-reveal) and the content smoothly scrolls to
  // keep them in view (design §identity editor).
  import { tick } from 'svelte';
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
    /** ON = store the chosen cert on the identity; OFF = pick it at connect. */
    storeCert: boolean;
    /** ON = store the PIN; OFF = enter it at connect. Only meaningful when storeCert. */
    storePin: boolean;
    onModuleChange: () => void;
    onRescan: () => void;
  }

  let {
    hidden,
    certs,
    certValue = $bindable(),
    moduleOptions,
    moduleValue = $bindable(),
    pin = $bindable(),
    storeCert = $bindable(),
    storePin = $bindable(),
    onModuleChange,
    onRescan,
  }: Props = $props();

  const certOptions = $derived(certs.map((c): Option => ({ value: c.uri, label: c.display, sub: c.slot + ' · ' + m.manager_expires({ date: c.expiry }) })));
  const selectedCert = $derived(certs.find((c) => c.uri === certValue));
  const expirySoon = $derived(selectedCert ? new Date(selectedCert.expiry).getTime() - Date.now() < 30 * 864e5 : false);

  // Smoothly scroll the editor content to reveal a just-shown row (340ms ease-out
  // cubic). scrollHeight is recomputed each frame so it tracks the .idn-reveal
  // slide as the row expands.
  async function scrollReveal(): Promise<void> {
    await tick();
    const sc = document.querySelector<HTMLElement>('.settings-content');
    if (!sc) return;
    const start = sc.scrollTop;
    const dur = 340;
    const ease = (t: number): number => 1 - Math.pow(1 - t, 3);
    let t0: number | null = null;
    const step = (now: number): void => {
      if (t0 === null) t0 = now;
      const p = Math.min(1, (now - t0) / dur);
      sc.scrollTop = start + (sc.scrollHeight - sc.clientHeight - start) * ease(p);
      if (p < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }

  function toggleStoreCert(): void {
    storeCert = !storeCert;
    if (storeCert) void scrollReveal();
  }
  function toggleStorePin(): void {
    storePin = !storePin;
    if (storePin) void scrollReveal();
  }
</script>

<div id="scBlock" {hidden} style="margin-top:16px;">
  <div class="lbl">{m.manager_module()}</div>
  <div id="moduleDD" class="mb16">
    <Dropdown options={moduleOptions} bind:value={moduleValue} placeholder={m.common_select()} onChange={onModuleChange} />
  </div>

  <div class="idn-toggle">
    <div>
      <div class="t">{m.manager_store_cert()}</div>
      <div class="d">{m.manager_store_cert_sub()}</div>
    </div>
    <button class="switch" class:on={storeCert} id="storeCert" type="button" aria-pressed={storeCert} aria-label={m.manager_store_cert()} onclick={toggleStoreCert}><span class="knob"></span></button>
  </div>

  {#if storeCert}
    <div class="idn-reveal" style="margin-top:16px;">
      <div class="lbl">{m.manager_smartcard_cert()}</div>
      <div style="display:flex; gap:8px;">
        <div style="flex:1; min-width:0;" id="certDD">
          <Dropdown options={certOptions} bind:value={certValue} placeholder={m.manager_no_certs()} onChange={() => void scrollReveal()} />
        </div>
        <button class="iconbtn" id="rescan" title={m.manager_rescan_tooltip()} onclick={onRescan}>⟳</button>
      </div>
      <div class="cert-detail" id="certDetail" hidden={!selectedCert}>
        {#if selectedCert}<span>{selectedCert.slot}</span><span class="sep">·</span><span class="exp" class:soon={expirySoon}>{m.manager_expires({ date: selectedCert.expiry })}</span>{/if}
      </div>
    </div>

    {#if selectedCert}
      <div class="idn-toggle idn-reveal" style="margin-top:12px;">
        <div>
          <div class="t">{m.manager_store_pin()}</div>
          <div class="d">{m.manager_store_pin_sub()}</div>
        </div>
        <button class="switch" class:on={storePin} id="storePin" type="button" aria-pressed={storePin} aria-label={m.manager_store_pin()} onclick={toggleStorePin}><span class="knob"></span></button>
      </div>

      {#if storePin}
        <div class="idn-reveal">
          <div class="lbl mt">{m.manager_pin()} <span style="color:var(--hint); font-weight:500; text-transform:none; letter-spacing:0;">{m.manager_stored_encrypted()}</span></div>
          <input class="field" id="pin" type="password" placeholder={m.manager_pin_placeholder()} bind:value={pin} />
        </div>
      {/if}
    {/if}
  {/if}
</div>
