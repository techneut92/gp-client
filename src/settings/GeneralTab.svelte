<script lang="ts">
  // General section: startup/tray/language/remember-unlock preferences.
  import { m } from '../paraglide/messages.js';
  import { applyChoice, currentChoice, localeOptions, type LocaleChoice } from '../lib/locale';
  import Dropdown from '../lib/Dropdown.svelte';
  import type { SettingsBoolKey, SettingsForm } from '../lib/api';

  interface Props {
    hidden: boolean;
    model: SettingsForm;
    onSave: () => void;
  }

  let { hidden, model = $bindable(), onSave }: Props = $props();

  function toggleKey(k: SettingsBoolKey): void {
    model[k] = !model[k];
    void onSave();
  }
  function pickTray(v: string): void {
    model.trayIcon = v;
    void onSave();
  }
</script>

<div class="settings-sec" id="sec-general" {hidden}>
  <h2>{m.settings_nav_general()}</h2>
  <p class="desc">{m.settings_general_desc()}</p>

  <div class="toggle-list" style="margin-top:0;">
    <div class="toggle-item">
      <div><div class="t">{m.settings_run_at_startup()}</div><div class="d">{m.settings_run_at_startup_desc()}</div></div>
      <button class="switch" class:on={model.runAtStartup} type="button" data-key="runAtStartup" aria-label={m.settings_run_at_startup()} onclick={() => toggleKey('runAtStartup')}><span class="knob"></span></button>
    </div>
    <div class="toggle-item">
      <div><div class="t">{m.settings_start_minimized()}</div><div class="d">{m.settings_start_minimized_desc()}</div></div>
      <button class="switch" class:on={model.startMinimized} type="button" data-key="startMinimized" aria-label={m.settings_start_minimized()} onclick={() => toggleKey('startMinimized')}><span class="knob"></span></button>
    </div>
    <div class="toggle-item">
      <div><div class="t" style="font-size:13px;color:var(--text-soft);font-weight:500;display:flex;align-items:center;"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="vertical-align:-2px;margin-right:6px;"><circle cx="12" cy="12" r="10" /><path d="M2 12h20" /><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" /></svg>{m.language_label()}</div><div class="d" style="font-size:11.5px;color:var(--faint);margin-top:2px;">{m.language_sub()}</div></div>
      <div style="min-width:180px;"><Dropdown options={localeOptions()} value={currentChoice()} onChange={(v) => applyChoice(v as LocaleChoice)} /></div>
    </div>
    <div class="toggle-item">
      <div><div class="t">{m.settings_remember_unlock()}</div><div class="d">{m.settings_remember_unlock_desc()}</div></div>
      <button class="switch" class:on={model.rememberUnlock} type="button" data-key="rememberUnlock" aria-label={m.settings_remember_unlock()} onclick={() => toggleKey('rememberUnlock')}><span class="knob"></span></button>
    </div>
  </div>

  <div class="lbl" style="margin-top:18px;">{m.settings_tray_icon()}</div>
  <div class="tray-grid">
    <button class="tray-card" class:sel={(model.trayIcon || 'shield') === 'shield'} type="button" data-tray="shield" onclick={() => pickTray('shield')}>
      <div class="tray-icons">
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><path d="M12 2.5 L20 5.2 V12 C20 17.2 16.5 21 12 22.5 C7.5 21 4 17.2 4 12 V5.2 Z" fill="#6b7286"/><path d="M12 8.6 V12.4 M12 15.7 h0.01" stroke="#0a0c12" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><path d="M12 2.5 L20 5.2 V12 C20 17.2 16.5 21 12 22.5 C7.5 21 4 17.2 4 12 V5.2 Z" fill="#fbbf24"/><circle cx="8.7" cy="12.7" r="1.15" fill="#0a0c12"/><circle cx="12" cy="12.7" r="1.15" fill="#0a0c12"/><circle cx="15.3" cy="12.7" r="1.15" fill="#0a0c12"/></svg>
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><path d="M12 2.5 L20 5.2 V12 C20 17.2 16.5 21 12 22.5 C7.5 21 4 17.2 4 12 V5.2 Z" fill="#34d399"/><path d="M8.4 12.2 l2.5 2.5 l4.7-4.9" stroke="#0a0c12" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
      </div>
      <div class="tray-foot"><span>{m.settings_tray_shield()}</span><span class="check"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#06121f" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 6"/></svg></span></div>
    </button>
    <button class="tray-card" class:sel={(model.trayIcon || 'shield') === 'ring'} type="button" data-tray="ring" onclick={() => pickTray('ring')}>
      <div class="tray-icons">
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><path d="M7.4 17.6 A7 7 0 1 1 16.6 17.6" stroke="#6b7286" stroke-width="2.1" stroke-linecap="round" fill="none"/><circle cx="12" cy="12" r="1.6" fill="#6b7286"/></svg>
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="7" stroke="#fbbf24" stroke-width="2.1" fill="none" opacity="0.28"/><path d="M12 5 A7 7 0 0 1 19 12" stroke="#fbbf24" stroke-width="2.1" stroke-linecap="round" fill="none"/><circle cx="12" cy="12" r="1.6" fill="#fbbf24"/></svg>
        <svg width="26" height="26" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="7" stroke="#34d399" stroke-width="2.1" fill="none"/><circle cx="12" cy="12" r="5" fill="#34d399"/><path d="M9.4 12.1 l1.9 1.9 l3.4-3.6" stroke="#0a0c12" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" fill="none"/></svg>
      </div>
      <div class="tray-foot"><span>{m.settings_tray_ring()}</span><span class="check"><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#06121f" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 6"/></svg></span></div>
    </button>
  </div>
  <p class="help">{m.settings_tray_help()}</p>
</div>
