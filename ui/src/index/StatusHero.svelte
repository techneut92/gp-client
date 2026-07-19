<script lang="ts">
  // Hero block of the unlocked view: state orb plus status title/subtitle.
  import { m } from '../paraglide/messages.js';

  interface Props {
    lastKind: number;
    stateColor: string;
    statusSubText: string;
    /** Shrink the orb to make room for an inline challenge card (MFA / PIN). */
    compact?: boolean;
  }

  let { lastKind, stateColor, statusSubText, compact = false }: Props = $props();

  const orbConnecting = $derived(lastKind === 1 || lastKind === 4);
  const orbConnected = $derived(lastKind === 2);
  const orbGlyphInner = $derived(lastKind === 2 ? 'M9 12l2 2 4-4' : 'M12 8v4M12 15.5h.01');
  const statusTitleText = $derived(
    [m.main_status_not_connected(), m.status_connecting(), m.main_status_protected(), m.main_status_failed(), m.status_reconnecting()][lastKind] ?? m.main_status_not_connected()
  );
</script>

<div class="hero" class:compact>
  <div class="orb-wrap" id="orb" class:connecting={orbConnecting} class:connected={orbConnected} style="--c: {stateColor}">
    <div class="orb-glow"></div>
    <div class="orb-ring"></div>
    <div class="orb-disc">
      <svg id="orbGlyph" width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 3l7 3v5c0 4.4-3 7.4-7 9-4-1.6-7-4.6-7-9V6l7-3z" />
        <path id="orbGlyphInner" d={orbGlyphInner} />
      </svg>
    </div>
  </div>
  <div class="status-title" id="statusTitle">{statusTitleText}</div>
  <div class="status-sub" id="statusSub">{statusSubText}</div>
</div>
