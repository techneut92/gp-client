<script lang="ts">
  // Scoped-DNS domain list: chip editor for the identity's dns_domains.
  // Accepts comma/space/semicolon-separated batches; Enter, leaving the field,
  // or the button adds them. Invalid tokens stay in the field with an error
  // line so nothing is ever silently lost.
  import { m } from '../paraglide/messages.js';

  interface Props {
    domains: string[];
  }

  let { domains = $bindable() }: Props = $props();

  let entry = $state('');
  let rejected = $state<string[]>([]);

  // Mirrors the backend validator (gpservice scoped_dns_domains): dot-separated
  // labels of [a-z0-9_-] that don't start or end with '-'. No '~' routing
  // syntax, no empty labels.
  function validDomain(d: string): boolean {
    if (!d) return false;
    return d
      .split('.')
      .every((label) => label.length > 0 && !label.startsWith('-') && !label.endsWith('-') && /^[a-z0-9_-]+$/.test(label));
  }

  function add(): void {
    const parts = entry
      .split(/[\s,;]+/)
      .map((s) => s.trim().toLowerCase().replace(/^\.+|\.+$/g, ''))
      .filter(Boolean);
    const good: string[] = [];
    const bad: string[] = [];
    for (const p of parts) (validDomain(p) ? good : bad).push(p);
    if (good.length) {
      const next = [...domains];
      for (const g of good) if (!next.includes(g)) next.push(g);
      domains = next;
    }
    rejected = bad;
    // Keep what didn't validate so the user can correct it.
    entry = bad.join(', ');
  }

  /** Commit any text still sitting in the entry field. The form calls this
   *  before saving so a typed-but-not-added domain is never silently lost
   *  (onblur usually covers it, but this removes the event-ordering
   *  assumption). */
  export function flush(): void {
    if (entry.trim()) add();
  }

  function remove(d: string): void {
    domains = domains.filter((x) => x !== d);
  }

  function onKey(e: KeyboardEvent): void {
    if (e.key === 'Enter') {
      e.preventDefault();
      add();
    }
  }
</script>

<div style="margin-top:16px;">
  <div class="lbl">{m.manager_dns_domains()}</div>
  <div style="display:flex; gap:8px;">
    <input
      class="field mono"
      id="dnsDomainEntry"
      style="flex:1; min-width:0;"
      placeholder={m.manager_dns_domains_placeholder()}
      bind:value={entry}
      onkeydown={onKey}
      onblur={add}
      oninput={() => (rejected = [])}
    />
    <button class="btn-soft" id="dnsDomainAdd" type="button" onclick={add}>{m.manager_dns_add()}</button>
  </div>
  {#if rejected.length}
    <div class="cert-detail" style="margin-top:7px; color: var(--red, #fb6f6c);">
      {m.manager_dns_invalid({ tokens: rejected.join(', ') })}
    </div>
  {/if}
  {#if domains.length}
    <div id="dnsDomainChips" style="display:flex; flex-wrap:wrap; gap:6px; margin-top:9px;">
      {#each domains as d (d)}
        <span class="chip">
          {d}
          <button
            type="button"
            aria-label={m.manager_dns_remove({ domain: d })}
            style="background:none; border:0; padding:0; margin-left:2px; color:inherit; cursor:pointer; font-size:13px; line-height:1;"
            onclick={() => remove(d)}>×</button
          >
        </span>
      {/each}
    </div>
  {/if}
  <div class="cert-detail" style="margin-top:9px;">
    {domains.length ? m.manager_dns_scoped_hint() : m.manager_dns_all_hint()}
  </div>
</div>
