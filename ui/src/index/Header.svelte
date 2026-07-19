<script lang="ts">
  // Main-window header: brand, status pill and window controls.
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { m } from '../paraglide/messages.js';
  import { openUrl } from '../lib/api';
  import type { View } from './types';

  interface Props {
    view: View;
    pillText: string;
    stateColor: string;
    dotLive: boolean;
    hasUpdate: boolean;
    onOpenManager: () => void;
    onOpenSettings: () => void;
  }

  let { view, pillText, stateColor, dotLive, hasUpdate, onOpenManager, onOpenSettings }: Props = $props();

  const KOFI_URL = 'https://ko-fi.com/techneut92?amount=2.50#checkoutModal';

  function winMin(): void {
    void getCurrentWindow().minimize();
  }
  function winClose(): void {
    void getCurrentWindow().close();
  }
  function onKofi(e: MouseEvent): void {
    e.preventDefault();
    void openUrl(KOFI_URL);
  }
</script>

<header data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <div class="mark">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l7 3v5c0 4.4-3 7.4-7 9-4-1.6-7-4.6-7-9V6l7-3z" /></svg>
    </div>
    <div class="title">{m.main_brand_gp()} <span class="ng">{m.main_brand_client()}</span></div>
  </div>
  <div class="hbar-right">
    <div class="pill"><span class="dot" id="dot" class:live={dotLive} style="background:{stateColor};color:{stateColor}"></span><span id="statusText">{pillText}</span></div>
    <div class="winctl">
      <a class="wbtn kofi" id="winKofi" href={KOFI_URL} target="_blank" rel="noreferrer" title={m.main_kofi_title()} onclick={onKofi}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M5 8h11v5a4 4 0 0 1-4 4H9a4 4 0 0 1-4-4z" /><path d="M16 9h1.5a2.5 2.5 0 0 1 0 5H16" /><path d="M8 3v2M11 3v2" /></svg>
      </a>
      <button class="wbtn" id="winManage" title={m.main_manage_identities_title()} hidden={view !== '' && view !== 'unlocked'} onclick={onOpenManager}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="9" cy="8" r="3.2" /><path d="M3.5 20a5.5 5.5 0 0 1 11 0" /><path d="M16 4.5a3 3 0 0 1 0 6M18 20a5.5 5.5 0 0 0-3-4.9" /></svg>
      </button>
      <button class="wbtn" id="winSettings" title={m.main_settings_title()} class:has-update={hasUpdate} hidden={view !== '' && view !== 'unlocked'} onclick={onOpenSettings}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3" /><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" /></svg>
      </button>
      <button class="wbtn" id="winMin" title={m.main_minimize()} onclick={winMin}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M5 12h14" /></svg>
      </button>
      <button class="wbtn close" id="winClose" title={m.common_close()} onclick={winClose}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M5 5l14 14M19 5L5 19" /></svg>
      </button>
    </div>
  </div>
</header>
