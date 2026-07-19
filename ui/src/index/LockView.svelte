<script lang="ts">
  // Locked-vault view: master-PIN prompt plus the forgotten-PIN reset flow.
  import { m } from '../paraglide/messages.js';
  import LanguagePicker from '../lib/LanguagePicker.svelte';
  import type { LogLine } from './types';

  interface Props {
    show: boolean;
    masterPin: string;
    lockLog: LogLine;
    resetOpen: boolean;
    onUnlock: () => void;
    onResetConfirm: () => void;
  }

  let { show, masterPin = $bindable(), lockLog, resetOpen = $bindable(), onUnlock, onResetConfirm }: Props = $props();
</script>

<div class="view center" id="lockView" class:show={show}>
  <div class="stack vault">
    <div class="vault-badge">
      <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="10" width="16" height="11" rx="2.5" /><path d="M8 10V7a4 4 0 0 1 8 0v3" /><circle cx="12" cy="15.5" r="1.4" /></svg>
    </div>
    <h2 class="h2">{m.main_unlock()}</h2>
    <p class="sub">{m.main_lock_sub()}</p>
    <input
      class="field pin-input"
      id="masterPin"
      type="password"
      placeholder={m.main_master_pin_placeholder()}
      bind:value={masterPin}
      onkeydown={(e) => {
        if (e.key === 'Enter') onUnlock();
      }}
    />
    <p class="formlog" id="lockLog" class:err={lockLog.err}>{lockLog.msg}</p>
    <div class="actions" style="width:100%"><button class="btn-action" id="unlockBtn" onclick={onUnlock}>{m.main_unlock()}</button></div>
    <button class="link" id="forgotPinBtn" style="margin-top:8px;" hidden={resetOpen} onclick={() => (resetOpen = true)}>{m.main_forgot_pin()}</button>
    <LanguagePicker />
    <div class="reset-warn" id="resetWarn" hidden={!resetOpen}>
      <div class="warn-text">
        <strong>{m.main_reset_title()}</strong> {m.main_reset_body()}
      </div>
      <div class="reset-actions">
        <button class="btn-action danger" id="resetConfirmBtn" onclick={onResetConfirm}>{m.main_reset_confirm()}</button>
        <button class="link" id="resetCancelBtn" onclick={() => (resetOpen = false)}>{m.common_cancel()}</button>
      </div>
    </div>
  </div>
</div>
