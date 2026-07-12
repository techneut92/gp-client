<script lang="ts">
  // Left rail: "new identity" button plus the saved-identity list.
  import { m } from '../paraglide/messages.js';
  import AuthIcon from '../lib/AuthIcon.svelte';
  import type { Identity } from '../lib/api';

  interface Props {
    identities: Identity[];
    editing: string | null;
    onNew: () => void;
    onSelect: (id: Identity) => void;
  }

  let { identities, editing, onNew, onSelect }: Props = $props();
</script>

<nav class="settings-rail" style="flex-basis:210px;">
  <button class="rail-add" id="newBtn" onclick={onNew}>
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>
    {m.manager_new_identity()}
  </button>
  <div id="idList">
    {#each identities as id (id.name)}
      <button type="button" class="id-item" class:active={editing === id.name} onclick={() => onSelect(id)}>
        <span class="ico"><AuthIcon method={id.auth_method} /></span><span class="txt"><span class="nm">{id.name}</span><span class="pt">{id.portal || ''}</span></span>
      </button>
    {/each}
  </div>
</nav>
