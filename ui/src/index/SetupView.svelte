<script lang="ts">
  // First-run view: create the vault master PIN (+ auto-unlock opt-in).
  import { m } from '../paraglide/messages.js';
  import LanguagePicker from '../lib/LanguagePicker.svelte';
  import type { LogLine } from './types';

  interface Props {
    show: boolean;
    newMasterPin: string;
    confirmPin: string;
    setupLog: LogLine;
    keyringAvailable: boolean;
    autoUnlock: boolean;
    onSetup: () => void;
  }

  let { show, newMasterPin = $bindable(), confirmPin = $bindable(), setupLog, keyringAvailable, autoUnlock = $bindable(), onSetup }: Props = $props();

  let infoNoteHidden = $state(true);
  const autoOn = $derived(keyringAvailable && autoUnlock);
  function toggleAutoUnlock(): void {
    if (!keyringAvailable) return;
    autoUnlock = !autoUnlock;
  }
</script>

<div class="view setup-view" id="setupView" class:show={show}>
  <div class="setup-scroll">
    <div class="stack vault">
      <div class="vault-badge">
        <svg width="38" height="38" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l7 3v5c0 4.4-3 7.4-7 9-4-1.6-7-4.6-7-9V6l7-3z" /><rect x="9" y="11" width="6" height="5" rx="1" /><path d="M10.5 11V9.5a1.5 1.5 0 0 1 3 0V11" /></svg>
      </div>
      <h2 class="h2">{m.main_setup_title()}</h2>
      <p class="sub">{m.main_setup_sub()}</p>

      <div class="vault-form">
        <div class="vault-lbl">{m.main_setup_pin_label()}</div>
        <input
          class="vault-input"
          id="newMasterPin"
          type="password"
          maxlength="63"
          placeholder={m.main_setup_pin_placeholder()}
          bind:value={newMasterPin}
          onkeydown={(e) => {
            if (e.key === 'Enter') onSetup();
          }}
        />

        <div class="vault-lbl mt">{m.main_setup_confirm_label()}</div>
        <input
          class="vault-input"
          id="confirmPin"
          type="password"
          maxlength="63"
          placeholder={m.main_setup_confirm_placeholder()}
          bind:value={confirmPin}
          onkeydown={(e) => {
            if (e.key === 'Enter') onSetup();
          }}
        />

        <p class="formlog" id="setupLog" class:err={setupLog.err}>{setupLog.msg}</p>

        <div class="vault-optin" id="autoUnlockRow" class:disabled={!keyringAvailable} class:on={autoOn}>
          <div class="optin-main">
            <div class="optin-head">
              <span class="optin-label">{m.main_auto_unlock_label()}</span>
              <button
                class="optin-info"
                id="autoInfoBtn"
                type="button"
                title={m.main_auto_unlock_about()}
                aria-label={m.main_auto_unlock_about()}
                hidden={!keyringAvailable}
                onclick={() => {
                  infoNoteHidden = !infoNoteHidden;
                }}>?</button
              >
            </div>
            <div class="optin-help" id="autoHelp">
              {#if !keyringAvailable}
                {m.main_auto_unlock_unavailable()}
              {:else if autoUnlock}
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#34d399" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px;margin-right:5px;"><path d="M5 12l5 5L20 6" /></svg>{m.main_auto_unlock_on()}
              {:else}
                {m.main_auto_unlock_help()}
              {/if}
            </div>
          </div>
          <button class="switch" id="autoUnlockSwitch" type="button" role="switch" aria-checked={autoOn} aria-label={m.main_auto_unlock_label()} class:on={autoOn} disabled={!keyringAvailable} onclick={toggleAutoUnlock}><span class="knob"></span></button>
        </div>
        <div class="optin-note" id="autoInfoNote" hidden={infoNoteHidden || !keyringAvailable}>{m.main_auto_unlock_note()}</div>

        <div class="actions" style="width:100%"><button class="btn-action" id="setupBtn" onclick={onSetup}>{m.main_create_vault()}</button></div>
      </div>
    </div>
  </div>
  <div class="vault-lang"><LanguagePicker /></div>
</div>
