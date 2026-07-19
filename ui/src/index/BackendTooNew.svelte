<script lang="ts">
  // "Update GP Client" view (design: Backend Too New) — the inverse of the
  // backend-missing/outdated screen. Shown when the host backend speaks a newer
  // wire protocol than this GUI understands (system_info.backendTooNew), so the
  // two can't talk. The fix is to update the app, not the backend (GPC-42).
  import { m } from '../paraglide/messages.js';
  import LanguagePicker from '../lib/LanguagePicker.svelte';

  interface Props {
    show: boolean;
    guiVersion: string;
    backendVersion?: string | null;
    /** Manual "update this app" command, matched to the GUI's install kind. */
    updateCmd: string;
    updating: boolean;
    checking: boolean;
    status?: { msg: string; kind: string } | null;
    onUpdate: () => void;
    onRecheck: () => void;
  }

  let { show, guiVersion, backendVersion, updateCmd, updating, checking, status, onUpdate, onRecheck }: Props = $props();

  let copied = $state(false);

  async function copyCmd(): Promise<void> {
    try {
      await navigator.clipboard.writeText(updateCmd);
      copied = true;
      setTimeout(() => (copied = false), 1400);
    } catch {
      /* clipboard denied — the command stays visible to copy by hand */
    }
  }

  const COPY_ICON = 'M9 9h11v11H9z M5 15V5a2 2 0 0 1 2-2h10';
  const CHECK_ICON = 'M5 12l5 5L20 6';
</script>

<div class="view" id="backendNewView" class:show={show}>
  <div class="svc-wrap">
    <div class="svc-lang"><LanguagePicker /></div>

    <div class="svc-hero">
      <div class="svc-badge bknew-badge">
        <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="#fbbf24" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l7 3v5c0 4.4-3 7.4-7 9-4-1.6-7-4.6-7-9V6l7-3z" /><path d="M12 8.5v4M12 15.6h.01" /></svg>
        <span class="alert"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#0a0c12" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19V5M6 11l6-6 6 6" /></svg></span>
      </div>
      <div class="svc-title">{m.main_bknew_title()}</div>
      <div class="svc-sub">{m.main_bknew_sub()}</div>
    </div>

    <!-- version mismatch: this app vs the newer backend -->
    <div class="bknew-vers">
      <div class="bknew-card">
        <div class="bknew-lbl">{m.main_bknew_gui()}</div>
        <div class="bknew-ver">v{guiVersion}</div>
      </div>
      <div class="bknew-arrow">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14M13 6l6 6-6 6" /></svg>
      </div>
      <div class="bknew-card hot">
        <div class="bknew-lbl">{m.main_bknew_backend()}</div>
        <div class="bknew-ver">{backendVersion ? 'v' + backendVersion : '—'}</div>
      </div>
    </div>

    <button class="btn-action svc-install bknew-update" disabled={updating} onclick={onUpdate}>
      <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19V5M6 11l6-6 6 6" /></svg>
      <span>{updating ? m.main_bknew_updating() : m.main_bknew_update()}</span>
    </button>
    <div class="bk-status{status && status.kind ? ' ' + status.kind : ''}" hidden={!status}>{status ? status.msg : ''}</div>

    <div class="svc-or"><span>{m.main_bknew_or_manual()}</span></div>

    <div class="term-card">
      <div class="term-head">
        <span class="term-dots"><i style="background:#fb6f6c"></i><i style="background:#fbbf24"></i><i style="background:#34d399"></i></span>
        <span class="term-label">{m.main_bknew_term_label()}</span>
      </div>
      <div class="term-body">
        <div class="bknew-cmd-title">{m.main_bknew_cmd_title()}</div>
        <div class="bknew-cmd-row">
          <div class="bknew-cmd"><code>{updateCmd}</code></div>
          <button class="copy-btn" class:done={copied} title={m.main_copy()} onclick={copyCmd}>
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d={copied ? CHECK_ICON : COPY_ICON} /></svg>
          </button>
        </div>
        <div class="term-note">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9" /><path d="M12 8h.01M11 12h1v4h1" /></svg>
          <span>{m.main_bknew_cmd_note()}</span>
        </div>
      </div>
    </div>

    <div class="svc-recheck-wrap">
      <button class="svc-recheck" onclick={onRecheck}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" style={checking ? 'animation:spin 1s linear infinite' : ''}><path d="M3 12a9 9 0 0 1 15.5-6.2L21 8" /><path d="M21 4v4h-4" /><path d="M21 12a9 9 0 0 1-15.5 6.2L3 16" /><path d="M3 20v-4h4" /></svg>
        <span>{checking ? m.main_checking() : m.main_recheck()}</span>
      </button>
    </div>
  </div>
</div>

<style>
  .bknew-vers { display: flex; gap: 10px; margin-top: 22px; }
  .bknew-card { flex: 1 1 0; background: var(--field); border: 1px solid var(--line); border-radius: 14px; padding: 13px 14px; }
  .bknew-card.hot { background: rgba(251, 191, 36, 0.08); border-color: rgba(251, 191, 36, 0.3); }
  .bknew-lbl { font-size: 10.5px; text-transform: uppercase; letter-spacing: 0.7px; color: #717892; }
  .bknew-card.hot .bknew-lbl { color: #b79246; }
  .bknew-ver { margin-top: 5px; font-size: 15px; font-weight: 620; color: #eef1f8; font-variant-numeric: tabular-nums; }
  .bknew-card.hot .bknew-ver { color: #fbbf24; }
  .bknew-arrow { flex: 0 0 auto; display: flex; align-items: center; color: #5d6378; }

  .bknew-update { margin-top: 18px; }

  .bknew-cmd-title { font-size: 12.5px; color: #c2c8d8; font-weight: 550; margin: 6px 0 9px; }
  .bknew-cmd-row { display: flex; gap: 8px; align-items: stretch; }
  .bknew-cmd { flex: 1 1 0; min-width: 0; background: #070a10; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 9px; padding: 10px 11px; }
  .bknew-cmd code { display: block; white-space: pre-wrap; word-break: break-all; font-size: 12px; line-height: 1.5; color: #dbe7f5; font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace; }
</style>
