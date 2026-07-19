<script lang="ts">
  // Scoped-DNS domain list: chip editor for the identity's dns_domains.
  // Accepts comma/space/semicolon-separated batches; Enter, leaving the field,
  // or the button adds them. Invalid tokens stay in the field with an error
  // line so nothing is ever silently lost.
  import { m } from '../paraglide/messages.js';
  import { openExt } from '../lib/api';

  // Docs: the scoped-DNS + Chrome async-DNS caveat lives in the repo.
  const CHROME_DNS_DOC = 'https://github.com/techneut92/gp-client/blob/main/docs/scoped-dns.md#google-chrome-disable-the-built-in-async-dns-resolver';

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
  <div class="cert-detail dns-chrome-warn" style="margin-top:7px;">
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" style="flex:0 0 auto; vertical-align:-2px; margin-right:5px;"><path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" /><path d="M12 9v4M12 17h.01" /></svg>{m.manager_dns_chrome_warn()}
    <a href={CHROME_DNS_DOC} target="_blank" rel="noreferrer" onclick={(e) => { e.preventDefault(); openExt(CHROME_DNS_DOC); }}>{m.manager_dns_chrome_learn()}</a>
  </div>
</div>

<style>
  .dns-chrome-warn {
    color: #d8a24a;
  }
  .dns-chrome-warn a {
    color: #e6b866;
    text-decoration: underline;
    text-underline-offset: 2px;
    white-space: nowrap;
  }
  .dns-chrome-warn a:hover {
    color: #f2c980;
  }
</style>
