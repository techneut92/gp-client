<script lang="ts">
  // "Update available" banner at the top of the unlocked view: icon + title +
  // version subtitle + Update button (opens Settings → About).
  import { m } from '../paraglide/messages.js';
  import type { Banner } from './types';

  interface Props {
    banner: Banner | null;
  }

  let { banner }: Props = $props();
</script>

<div class="topbanner{banner ? ' ' + banner.kind : ''}" id="topBanner" hidden={!banner}>
  {#if banner}
    <div class="banner-icon">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19V5M5 12l7-7 7 7" /></svg>
    </div>
    <div class="banner-txt">
      <div class="banner-title">{banner.text}</div>
      {#if banner.sub}<div class="banner-sub">{banner.sub}</div>{/if}
    </div>
    <button class="banner-btn" onclick={banner.onClick}>{m.main_update_btn()}</button>
  {/if}
</div>
