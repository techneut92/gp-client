<script lang="ts">
  // "Update available" banner shown at the top of the unlocked view.
  import { m } from '../paraglide/messages.js';
  import type { Banner } from './types';

  interface Props {
    banner: Banner | null;
  }

  let { banner }: Props = $props();
</script>

<div class="topbanner{banner ? ' ' + banner.kind : ''}" id="topBanner" hidden={!banner}>
  {#if banner}<span>{banner.text}</span><button class="banner-btn" onclick={banner.onClick}>{m.main_update_view()}</button>{/if}
</div>
