<script lang="ts">
  // Inline gateway picker in the connecting window: portal mode returned more
  // than one gateway and the backend parked the connect on our choice. Rows
  // show the portal's gateway name, its address, and a best-effort latency
  // (TCP handshake to :443, measured as the prompt opens). The chosen address
  // goes back through select_gateway and the connect resumes.
  import { m } from '../paraglide/messages.js';
  import { pingGateway } from '../lib/api';

  interface Props {
    /** The portal's gateways, in the backend's preference order. */
    gateways: { name: string; host: string }[];
    /** Address of the region-preferred gateway (pre-selected). */
    preferred?: string;
    /** Identity (or portal) name for the subtitle. */
    identity?: string;
    onSubmit: (host: string) => void;
  }

  let { gateways, preferred = '', identity = '', onSubmit }: Props = $props();

  // The chosen address. Follows the preferred default until the user picks;
  // resets if the current choice isn't in the offered list.
  let chosen = $state('');
  $effect(() => {
    if (!gateways.some((g) => g.host === chosen)) chosen = preferred || gateways[0]?.host || '';
  });

  // Latency per host, filled in as probes come back. undefined → measuring.
  let pings = $state<Record<string, number | null>>({});
  $effect(() => {
    const hosts = gateways.map((g) => g.host);
    pings = {};
    for (const host of hosts) {
      pingGateway(host).then(
        (ms) => (pings = { ...pings, [host]: ms }),
        () => (pings = { ...pings, [host]: null }),
      );
    }
  });

  function pingColor(ms: number): string {
    return ms < 60 ? '#6ee7b7' : ms < 120 ? '#fbbf24' : '#fb7185';
  }
  function submit(): void {
    if (chosen) onSubmit(chosen);
  }
</script>

<div class="chal-card gw">
  <div class="chal-head">
    <span class="chal-ico">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9.5"/><path d="M2.5 12h19M12 2.5c2.7 2.6 4 6 4 9.5s-1.3 6.9-4 9.5c-2.7-2.6-4-6-4-9.5s1.3-6.9 4-9.5z"/></svg>
    </span>
    <div style="min-width:0;">
      <div class="chal-title">{m.gw_title()}</div>
      <div class="chal-sub">{identity ? m.gw_sub({ name: identity }) : m.gw_sub_generic()}</div>
    </div>
  </div>

  <div class="gw-list" role="radiogroup" aria-label={m.gw_title()}>
    {#each gateways as g (g.host)}
      <button type="button" class="gw-row" class:sel={g.host === chosen} role="radio" aria-checked={g.host === chosen} onclick={() => (chosen = g.host)}>
        <span class="gw-body">
          <span class="gw-name">{g.name}</span>
          <span class="gw-host">{g.host}</span>
        </span>
        {#if pings[g.host] === undefined}
          <span class="gw-ping" style="color:#7e8599;">…</span>
        {:else if pings[g.host] === null}
          <span class="gw-ping" style="color:#7e8599;">—</span>
        {:else}
          <span class="gw-ping" style="color:{pingColor(pings[g.host] ?? 0)};">{pings[g.host]} ms</span>
        {/if}
      </button>
    {/each}
  </div>

  <button class="btn-action chal-btn" disabled={!chosen} onclick={submit}>{m.gw_continue()}</button>
</div>

<style>
  .gw-list {
    display: flex; flex-direction: column; gap: 7px; margin-top: 14px;
    max-height: 246px; overflow-y: auto;
  }
  .gw-row {
    display: flex; align-items: center; gap: 10px; width: 100%; box-sizing: border-box;
    text-align: left; padding: 11px 12px; background: #0b0f1a; color: inherit;
    border: 1px solid rgba(255, 255, 255, 0.07); border-radius: 11px; cursor: pointer;
    font-family: inherit; transition: background 0.12s, border-color 0.12s;
  }
  .gw-row.sel { background: #182034; border-color: rgba(84, 168, 255, 0.45); }
  .gw-body { flex: 1; min-width: 0; }
  .gw-name { display: block; font-size: 12.5px; font-weight: 600; color: #eef1f8; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gw-host { display: block; font-size: 11px; color: #7e8599; margin-top: 1px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gw-ping { flex: 0 0 auto; font-size: 11px; font-variant-numeric: tabular-nums; }
</style>
