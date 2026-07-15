<script lang="ts">
  // Migration screen: offer to import everything from the predecessor gpgui, then
  // remove the old app + data. Shown on a fresh install when gpgui is present.
  import { m } from '../paraglide/messages.js';

  interface Props {
    show: boolean;
    busy: boolean;
    error: string;
    onImport: () => void;
    onSkip: () => void;
  }
  let { show, busy, error, onImport, onSkip }: Props = $props();
</script>

<div class="view center" id="importView" class:show={show}>
  <div class="stack vault">
    <div class="vault-badge">
      <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12" /><path d="M8 11l4 4 4-4" /><path d="M5 21h14" /></svg>
    </div>
    <h2 class="h2">{m.migrate_title()}</h2>
    <p class="sub">{m.migrate_sub()}</p>
    {#if error}<p class="formlog err" id="importLog">{error}</p>{/if}
    <div class="actions" style="width:100%">
      <button class="btn-action" id="importBtn" disabled={busy} onclick={onImport}>{busy ? m.migrate_importing() : m.migrate_import()}</button>
    </div>
    <button class="link" id="skipImportBtn" style="margin-top:8px;" disabled={busy} onclick={onSkip}>{m.migrate_skip()}</button>
  </div>
</div>
