<script lang="ts">
  // Disconnected/connecting content: rich identity picker with detail chips,
  // the empty-state card and the manage-identities link. The picker menu is
  // portaled to <body> so it escapes overflow containers.
  import { onDestroy } from 'svelte';
  import { m } from '../paraglide/messages.js';
  import AuthIcon from '../lib/AuthIcon.svelte';
  import type { Identity } from '../lib/api';
  import type { Chip } from './types';

  interface Props {
    hidden: boolean;
    identities: Identity[];
    selected: string | null;
    chips: Chip[];
    onPick: (id: Identity) => void;
    onOpenManager: () => void;
  }

  let { hidden, identities, selected, chips, onPick, onOpenManager }: Props = $props();

  let idOpen = $state(false);
  let idTriggerEl = $state<HTMLButtonElement | null>(null);
  let idMenuEl: HTMLDivElement | null = null;
  const curIdentity = $derived(identities.find((i) => i.name === selected));
  const initials = (n: string): string => (n || '?').trim().slice(0, 1).toUpperCase();

  function toggleIdMenu(e: MouseEvent): void {
    e.stopPropagation();
    idOpen = !idOpen;
  }
  function pickIdentity(id: Identity): void {
    idOpen = false;
    onPick(id);
  }
  function idPortal(node: HTMLDivElement): { destroy(): void } {
    document.body.appendChild(node);
    idMenuEl = node;
    if (idTriggerEl) {
      const r = idTriggerEl.getBoundingClientRect();
      node.style.top = `${r.bottom + 6}px`;
      node.style.left = `${r.left}px`;
      node.style.width = `${r.width}px`;
    }
    return {
      destroy: () => {
        node.remove();
        idMenuEl = null;
      },
    };
  }
  function onDocDown(e: MouseEvent): void {
    const t = e.target as Node;
    if (idOpen && idMenuEl && !idMenuEl.contains(t) && idTriggerEl && !idTriggerEl.contains(t)) idOpen = false;
  }
  function closeIdMenu(): void {
    if (idOpen) idOpen = false;
  }
  document.addEventListener('mousedown', onDocDown, true);
  window.addEventListener('scroll', closeIdMenu, true);
  window.addEventListener('resize', closeIdMenu);
  onDestroy(() => {
    document.removeEventListener('mousedown', onDocDown, true);
    window.removeEventListener('scroll', closeIdMenu, true);
    window.removeEventListener('resize', closeIdMenu);
  });
</script>

<div id="formContent" {hidden}>
  <div class="card" id="idCard" class:hidden={identities.length === 0}>
    <div class="lbl">{m.main_identity()}</div>
    <div id="identityDD" class="mb14">
      <button type="button" class="id-trigger" class:active={idOpen} bind:this={idTriggerEl} onclick={toggleIdMenu}>
        {#if curIdentity}
          <div class="avatar">{initials(curIdentity.name)}</div>
          <div class="meta"><div class="name">{curIdentity.name}</div><div class="portal">{curIdentity.portal ?? ''}</div></div>
        {:else}
          <div class="avatar" style="background:#2a3142;color:#8a91a8">+</div>
          <div class="meta"><div class="name">{m.main_no_identity()}</div><div class="portal">{m.main_add_one_to_connect()}</div></div>
        {/if}
        <svg class="dd-chev" width="12" height="8" viewBox="0 0 12 8" fill="none" stroke="#8a91a8" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M1 1.5l5 5 5-5" /></svg>
      </button>
    </div>
    <div class="detail-card" id="idInfo">
      {#each chips as c, i (i)}
        <div class="drow">
          <div class="k">{c.k}</div>
          <div class="v">{#if c.color !== undefined}<span style="color:{c.color}">{c.v}</span>{:else}{c.v}{/if}</div>
        </div>
      {/each}
    </div>
  </div>
  <div class="card" id="emptyCard" class:hidden={identities.length > 0} style="text-align:center;">
    <div class="sub" style="margin:2px 0 12px;">{m.main_no_identities()}</div>
    <button class="btn-action auto" id="emptyAddBtn" style="padding:11px 18px;" onclick={onOpenManager}>{m.main_add_identity()}</button>
  </div>
  <button class="link manage-link" id="manageBtn" onclick={onOpenManager}>
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="8" r="3.2" /><path d="M3.5 20a5.5 5.5 0 0 1 11 0" /><path d="M16 4.5a3 3 0 0 1 0 6M18 20a5.5 5.5 0 0 0-3-4.9" /></svg>
    <span>{m.main_manage_identities()}</span>
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-left:auto;"><path d="M9 6l6 6-6 6" /></svg>
  </button>
</div>

{#if idOpen}
  <div class="dd-menu" use:idPortal>
    {#each identities as ident (ident.name)}
      <button type="button" class="id-item" class:active={ident.name === selected} onclick={() => pickIdentity(ident)}>
        <span class="ico"><AuthIcon method={ident.auth_method} /></span>
        <span class="txt"><span class="nm">{ident.name}</span><span class="pt">{ident.portal ?? ''}</span></span>
      </button>
    {/each}
  </div>
{/if}
