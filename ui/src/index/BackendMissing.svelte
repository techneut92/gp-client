<script lang="ts">
  // Backend-missing view: guided install (pkexec) or manual terminal steps.
  import { m } from '../paraglide/messages.js';
  import Dropdown from '../lib/Dropdown.svelte';
  import LanguagePicker from '../lib/LanguagePicker.svelte';
  import type { InstallOption } from '../lib/api';

  interface Props {
    show: boolean;
    /** A backend is installed but too old — show "update" copy instead of "install". */
    outdated: boolean;
    options: { value: string; label: string }[];
    kind: string;
    option: InstallOption | undefined;
    stepDone: boolean[];
    copyAllDone: boolean;
    installing: boolean;
    status: { msg: string; kind: string } | null;
    checking: boolean;
    recheckStyle: string;
    onKindChange: () => void;
    onInstall: () => void;
    onCopyStep: (i: number, cmd: string) => void;
    onCopyAll: () => void;
    onRecheck: () => void;
  }

  let { show, outdated, options, kind = $bindable(), option, stepDone, copyAllDone, installing, status, checking, recheckStyle, onKindChange, onInstall, onCopyStep, onCopyAll, onRecheck }: Props = $props();

  const COPY_ICON = 'M9 9h11v11H9z M5 15V5a2 2 0 0 1 2-2h10';
  const CHECK_ICON = 'M5 12l5 5L20 6';
</script>

<div class="view" id="backendView" class:show={show}>
  <div class="svc-wrap">
    <!-- Language picker up top so it's reachable before the backend is installed. -->
    <div class="svc-lang"><LanguagePicker /></div>
    <div class="svc-hero">
      <div class="svc-badge">
        <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="#fbbf24" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="6" rx="1.6" /><rect x="3" y="14" width="18" height="6" rx="1.6" /><path d="M7 7h.01M7 17h.01" /><path d="M11 7h6M11 17h6" /></svg>
        <span class="alert"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#0a0c12" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M12 8v5M12 16.5h.01" /></svg></span>
      </div>
      <div class="svc-title">{outdated ? m.main_backend_title_update() : m.main_backend_title()}</div>
      <div class="svc-sub">{outdated ? m.main_backend_sub_update() : m.main_backend_sub()}</div>
    </div>

    <div class="svc-section">
      <div class="svc-syslabel"><span class="lbl" style="margin:0;">{m.main_system_type()}</span><span class="hint-inline">{m.main_system_type_hint()}</span></div>
      <div id="sysDD">
        <Dropdown
          {options}
          bind:value={kind}
          onChange={() => {
            onKindChange();
          }}
        />
      </div>
    </div>

    <button class="btn-action svc-install" id="bkInstall" disabled={installing} onclick={onInstall}>
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12M8 11l4 4 4-4M5 21h14" /></svg>
      <span id="bkInstallLabel">{installing ? (outdated ? m.main_updating() : m.main_installing()) : (outdated ? m.main_update_backend() : m.main_install_backend())}</span>
    </button>
    <div class="bk-status{status && status.kind ? ' ' + status.kind : ''}" id="bkInstallLog" hidden={!status}>{status ? status.msg : ''}</div>
    <div class="svc-or"><span>{m.main_or_manual()}</span></div>

    <div class="term-card">
      <div class="term-head">
        <span class="term-dots"><i style="background:#fb6f6c"></i><i style="background:#fbbf24"></i><i style="background:#34d399"></i></span>
        <span class="term-label">{m.main_term_label()}</span>
      </div>
      <div class="term-body">
        <div id="steps">
          {#each option?.steps ?? [] as s, i (i)}
            <div class="step" class:last={i === (option?.steps.length ?? 0) - 1}>
              <span class="step-num">{i + 1}</span>
              <div class="step-body">
                <div class="step-label">{s.label}</div>
                <div class="step-cmd-row">
                  <div class="cmd"><code>{s.cmd}</code></div>
                  <button class="copy-btn" class:done={stepDone[i] === true} title={m.main_copy()} onclick={() => onCopyStep(i, s.cmd)}>
                    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d={stepDone[i] === true ? CHECK_ICON : COPY_ICON} /></svg>
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
        <div class="term-note">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9" /><path d="M12 8h.01M11 12h1v4h1" /></svg>
          <span id="note">{option?.note ?? ''}</span>
        </div>
      </div>
    </div>

    <button class="svc-copyall svc-secondary" id="copyAll" class:done={copyAllDone} onclick={onCopyAll}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path id="copyAllIcon" d={copyAllDone ? CHECK_ICON : COPY_ICON} /></svg>
      <span id="copyAllLabel">{copyAllDone ? m.main_copied() : m.main_copy_all()}</span>
    </button>

    <div class="svc-recheck-wrap">
      <button class="svc-recheck" id="recheck" onclick={onRecheck}>
        <svg id="recheckIcon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" style={recheckStyle}><path d="M3 12a9 9 0 0 1 15.5-6.2L21 8" /><path d="M21 4v4h-4" /><path d="M21 12a9 9 0 0 1-15.5 6.2L3 16" /><path d="M3 20v-4h4" /></svg>
        <span id="recheckLabel">{checking ? m.main_checking() : m.main_recheck()}</span>
      </button>
    </div>
  </div>
</div>
