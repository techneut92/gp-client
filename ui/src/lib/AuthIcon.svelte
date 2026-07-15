<script lang="ts">
  // Auth-method glyph shared by the identity lists (main-window picker menu
  // and the manager rail). Same svg bodies as the original gpgui markup.
  interface Props {
    method: number | undefined;
  }

  let { method }: Props = $props();
</script>

<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
  {#if method === 0}
    <rect x="2" y="5" width="20" height="14" rx="2.5" /><path d="M2 10h20" />
  {:else if method === 1}
    <path d="M14 3v4a1 1 0 0 0 1 1h4" /><path d="M17 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2z" />
  {:else if method === 2}
    <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" /><path d="M10 17l5-5-5-5M15 12H3" />
  {:else if method === 3}
    <circle cx="12" cy="8" r="4" /><path d="M4 21a8 8 0 0 1 16 0" />
  {:else}
    <circle cx="11" cy="11" r="7" /><path d="m20 20-3.2-3.2" />
  {/if}
</svg>
