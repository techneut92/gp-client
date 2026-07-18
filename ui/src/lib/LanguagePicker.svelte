<script lang="ts">
  // Language picker row used by the setup and lock views (identical markup in
  // the original page, including the globe icon).
  import { m } from '../paraglide/messages.js';
  import { applyChoice, currentChoice, localeOptions, type LocaleChoice } from './locale';
  import Dropdown from './Dropdown.svelte';
</script>

<div style="width:100%;display:flex;align-items:center;justify-content:center;gap:16px;margin-top:14px;">
  <span style="font-size:12px;color:var(--faint);display:inline-flex;align-items:center;"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px;margin-right:8px;"><circle cx="12" cy="12" r="10" /><path d="M2 12h20" /><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" /></svg>{m.language_label()}</span>
  <div style="min-width:170px;"><Dropdown options={localeOptions()} value={currentChoice()} onChange={(v) => applyChoice(v as LocaleChoice)} /></div>
</div>
