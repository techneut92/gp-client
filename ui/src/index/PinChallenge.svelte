<script lang="ts">
  // Inline smart-card prompt in the connecting window (design §10). The smart-card
  // block is a cert *picker*: it lists the certificates on the token so the user
  // can confirm (a stored cert, pre-selected) or choose one (Store certificate
  // off). Plus a PIN field (up to 63 chars — PKCS#11 PINs can be passphrases).
  // The chosen cert URI + PIN go back through submit_pin.
  import { m } from '../paraglide/messages.js';
  import type { CertInfo } from '../lib/api';

  interface Props {
    /** Smart-card manufacturer (subtitle); "" → generic prompt. */
    prompt?: string | undefined;
    /** PKCS#11 module file name; shown when no cert is selected yet. */
    module?: string | undefined;
    /** Certificates on the token, for the picker. */
    certs?: CertInfo[];
    /** Pre-selected cert URI (stored / remembered / first). */
    defaultUri?: string;
    onSubmit: (pin: string, certUri: string) => void;
  }

  let { prompt, module, certs = [], defaultUri = '', onSubmit }: Props = $props();

  let pin = $state('');
  let error = $state('');
  let open = $state(false);
  // The chosen cert URI. Adopts the computed default until the user picks; resets
  // if the current choice isn't among the scanned certs (e.g. list arrived late).
  let chosen = $state('');
  $effect(() => {
    if (!certs.some((c) => c.uri === chosen)) chosen = defaultUri;
  });

  const promptTitle = $derived(prompt ? m.pin_prompt_device({ device: prompt }) : m.pin_prompt_default());
  const selected = $derived(certs.find((c) => c.uri === chosen));
  const cardLabel = $derived(selected ? selected.display : certs.length ? m.pin_card_generic() : prompt ? m.pin_card_named({ device: prompt }) : m.pin_card_generic());
  const cardSub = $derived(selected ? selected.slot + ' · ' + m.manager_expires({ date: selected.expiry }) : module || m.pin_card_token());

  function pick(c: CertInfo): void {
    chosen = c.uri;
    open = false;
    error = '';
  }
  function onInput(v: string): void {
    pin = v.slice(0, 63);
    error = '';
  }
  function onKey(e: KeyboardEvent): void {
    if (e.key === 'Enter') submit();
  }
  function submit(): void {
    if (certs.length && !chosen) {
      error = m.pin_pick_cert();
      return;
    }
    if (!pin) {
      error = m.pin_enter();
      return;
    }
    onSubmit(pin, chosen);
  }

  // Close the picker on an outside click.
  function root(node: HTMLElement): { destroy(): void } {
    const onDown = (e: MouseEvent): void => {
      if (open && !node.contains(e.target as Node)) open = false;
    };
    document.addEventListener('mousedown', onDown, true);
    return { destroy: () => document.removeEventListener('mousedown', onDown, true) };
  }
</script>

<div class="chal-card pin">
  <div class="chal-head">
    <span class="chal-ico">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="2.5" y="5" width="19" height="14" rx="2.5"/><rect x="5.5" y="9" width="5" height="6" rx="1"/><path d="M14 10h4M14 14h4"/></svg>
    </span>
    <div style="min-width:0;">
      <div class="chal-title">{m.pin_title()}</div>
      <div class="chal-sub">{promptTitle}</div>
    </div>
  </div>

  <!-- smart-card block: a picker over the certificates on the token -->
  <div class="pin-pick" use:root>
    <button type="button" class="pin-card" class:open={open} disabled={certs.length < 2} onclick={() => (open = !open)}>
      <span class="pin-card-ico">
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="2.5" y="5" width="19" height="14" rx="2.5"/><rect x="5.5" y="9" width="5" height="6" rx="1"/><path d="M14 10h4M14 14h4"/></svg>
      </span>
      <span class="pin-card-body">
        <span class="pin-card-label">{cardLabel}</span>
        <span class="pin-card-sub">{cardSub}</span>
      </span>
      {#if certs.length > 1}
        <svg class="pin-card-chev" width="12" height="8" viewBox="0 0 12 8" fill="none" stroke="#8a91a8" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M1 1.5l5 5 5-5" /></svg>
      {/if}
    </button>

    {#if open}
      <div class="pin-menu">
        {#each certs as c (c.uri)}
          <button type="button" class="pin-opt" class:sel={c.uri === chosen} onclick={() => pick(c)}>
            <span class="pin-card-ico"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="2.5" y="5" width="19" height="14" rx="2.5"/><rect x="5.5" y="9" width="5" height="6" rx="1"/><path d="M14 10h4M14 14h4"/></svg></span>
            <span class="pin-card-body">
              <span class="pin-card-label">{c.display}</span>
              <span class="pin-card-sub">{c.slot} · {m.manager_expires({ date: c.expiry })}</span>
            </span>
            {#if c.uri === chosen}
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" style="flex:0 0 auto;"><path d="M5 12l5 5L20 6" /></svg>
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <input
    class="chal-input"
    type="password"
    autocomplete="off"
    maxlength="63"
    placeholder={m.pin_placeholder()}
    value={pin}
    oninput={(e) => onInput(e.currentTarget.value)}
    onkeydown={onKey}
  />
  {#if error}<div class="chal-err">{error}</div>{/if}
  <div class="chal-hint">{m.pin_hint()}</div>
  <button class="btn-action chal-btn" disabled={!pin || (certs.length > 0 && !chosen)} onclick={submit}>{m.pin_unlock()}</button>
</div>

<style>
  .pin-pick { position: relative; margin-top: 14px; }
  .pin-card {
    width: 100%; box-sizing: border-box; display: flex; align-items: center; gap: 11px;
    text-align: left; padding: 10px 12px; background: #0b0f1a; color: inherit;
    border: 1px solid rgba(255, 255, 255, 0.07); border-radius: 11px; cursor: pointer;
    font-family: inherit; transition: border-color 0.12s, box-shadow 0.12s;
  }
  .pin-card:disabled { cursor: default; }
  .pin-card.open { border-color: rgba(84, 168, 255, 0.55); box-shadow: 0 0 0 3px rgba(84, 168, 255, 0.15); }
  .pin-card-ico {
    flex: 0 0 auto; width: 32px; height: 32px; border-radius: 9px; display: flex;
    align-items: center; justify-content: center; background: rgba(255, 255, 255, 0.05); color: #9aa1b6;
  }
  .pin-card-body { flex: 1; min-width: 0; }
  .pin-card-label { display: block; font-size: 12.5px; font-weight: 600; color: #eef1f8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pin-card-sub { display: block; font-size: 11px; color: #7e8599; margin-top: 1px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pin-card-chev { flex: 0 0 auto; }
  .pin-menu {
    position: absolute; top: calc(100% + 6px); left: 0; right: 0; z-index: 30; max-height: 246px;
    overflow-y: auto; background: #151b29; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 12px;
    padding: 5px; box-shadow: 0 18px 50px -12px rgba(0, 0, 0, 0.75), 0 0 0 1px rgba(0, 0, 0, 0.4);
  }
  .pin-opt {
    display: flex; align-items: center; gap: 10px; width: 100%; box-sizing: border-box; text-align: left;
    border: none; border-radius: 8px; padding: 8px 9px; cursor: pointer; font-family: inherit; background: transparent;
  }
  .pin-opt:hover { background: rgba(255, 255, 255, 0.05); }
  .pin-opt.sel { background: rgba(84, 168, 255, 0.14); }
</style>
