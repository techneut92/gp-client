<script lang="ts">
  // Action footer (shown only when unlocked): status log line + the
  // state-driven connect/cancel/disconnect/retry button.
  import type { LogLine } from './types';

  interface Props {
    hidden: boolean;
    footLog: LogLine;
    actionClass: string;
    actionLabel: string;
    visible: boolean;
    onAction: () => void;
  }

  let { hidden, footLog, actionClass, actionLabel, visible, onAction }: Props = $props();
</script>

<footer id="appFooter" {hidden}>
  <div class="log" id="footLog" class:err={footLog.err}>{footLog.msg}</div>
  <button class={actionClass} id="actionBtn" style:display={visible ? '' : 'none'} onclick={onAction}>{actionLabel}</button>
</footer>
