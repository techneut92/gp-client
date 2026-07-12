<script lang="ts">
  // About section: versions, update status and the unified "Update all" flow
  // (backend first, then the app, narrated, with restart/reboot offer).
  import { m } from '../paraglide/messages.js';
  import { checkUpdate, hasTauri, installBackend, openExt, restart, runUpdate, systemInfo, type UpdateInfo } from '../lib/api';

  interface Props {
    hidden: boolean;
    /** Badges the About nav entry when the app or the backend is behind. */
    hasUpdate: boolean;
  }

  let { hidden, hasUpdate = $bindable() }: Props = $props();

  const REPO_URL = 'https://github.com/techneut92/GlobalProtect-openconnect-dw';
  const UPSTREAM_URL = 'https://github.com/yuezk/GlobalProtect-openconnect';

  type UpdCell =
    | { kind: 'init' }
    | { kind: 'dash' }
    | { kind: 'current' }
    | { kind: 'avail'; latest: string };

  let aboutVer = $state('—');
  let aboutGuiVer = $state('—');
  let aboutRunning = $state('—');
  let runtimeVisible = $state(false);
  let aboutRuntime = $state('—');
  let backendState = $state<'unknown' | 'missing' | 'present'>('unknown');
  let backendText = $state('—');
  let aboutKind = $state('—');
  let aboutOs = $state('—');
  let guiUpd = $state<UpdCell>({ kind: 'init' });
  let beUpd = $state<UpdCell>({ kind: 'init' });

  let updateStatus = $state<string>(m.settings_update_check_prompt());
  let updateUrl = REPO_URL + '/releases';
  let updateLatest = '';
  let updGui = false;
  let updBackend = false;
  let updateAllVisible = $state(false);
  let updating = $state(false);
  let restartVisible = $state(false);
  let restartText = $state(m.settings_restart_now());
  let restartCmd: 'reboot_host' | 'restart_app' = 'restart_app';
  let ubLog = $state('');
  let ubKind = $state('');

  function setUbLog(msg: string, kind: string): void {
    ubLog = msg;
    ubKind = kind;
  }

  export async function loadAbout(): Promise<void> {
    if (!hasTauri) {
      aboutVer = m.settings_dev_build();
      return;
    }
    try {
      const s = await systemInfo();
      aboutVer = 'v' + s.guiVersion;
      // App (frontend)
      aboutGuiVer = 'v' + s.guiVersion;
      aboutRunning = s.running + (s.running === 'Native package' ? ' (' + s.installKind + ')' : '');
      if (s.flatpakRuntime) {
        runtimeVisible = true;
        aboutRuntime = s.flatpakRuntime;
      }
      // Backend
      backendState = s.backendInstalled ? 'present' : 'missing';
      backendText = s.backendVersion ? 'v' + s.backendVersion : m.settings_installed();
      aboutKind = s.installKind;
      // System
      aboutOs = s.osName;

      // Update status — both the app and the backend (they version separately).
      let u: UpdateInfo | null = null;
      try {
        u = await checkUpdate();
      } catch {
        u = null;
      }
      const err = !!(u && u.error);
      guiUpd = err
        ? { kind: 'dash' }
        : u && u.available
          ? { kind: 'avail', latest: u.latest ?? '' }
          : { kind: 'current' };
      beUpd =
        !s.backendInstalled || err
          ? { kind: 'dash' }
          : u && u.backendUpdate
            ? { kind: 'avail', latest: u.latest ?? '' }
            : { kind: 'current' };
      if (u) {
        updateUrl = u.url || updateUrl;
        updateLatest = u.latest || '';
      }
      // One "Update all" button handles whichever of app/backend is behind.
      updGui = !!(u && u.available);
      updBackend = !!(u && u.backendUpdate);
      updateAllVisible = updGui || updBackend;
      restartVisible = false;
      // One-line summary
      if (err && u) updateStatus = m.settings_update_error({ error: u.error ?? '' });
      else if (u) {
        const parts = [
          u.available
            ? m.settings_update_summary_app_update({ current: u.current ?? '', latest: u.latest ?? '' })
            : m.settings_update_summary_app_latest({ current: u.current ?? '' }),
        ];
        if (s.backendInstalled) {
          const verStr = s.backendVersion ? 'v' + s.backendVersion : '?';
          parts.push(
            u.backendUpdate
              ? m.settings_update_summary_backend_update({ version: verStr, latest: u.latest ?? '' })
              : m.settings_update_summary_backend_latest({ version: verStr }),
          );
        }
        updateStatus = parts.join('  ·  ');
      }
      // Badge the About nav when either the app or the backend is behind.
      hasUpdate = !!(u && (u.available || u.backendUpdate));
    } catch {
      /* ignore */
    }
  }

  async function onUpdateAll(): Promise<void> {
    if (!hasTauri) {
      openExt(updateUrl);
      return;
    }
    updating = true;
    restartVisible = false;
    // Run a step while narrating the phase with a live elapsed counter.
    const step = async <T,>(phase: string, fn: () => Promise<T>): Promise<T> => {
      let n = 0;
      setUbLog(phase, '');
      const t = setInterval(() => {
        n++;
        setUbLog(m.settings_update_elapsed({ phase, seconds: String(n) }), '');
      }, 1000);
      try {
        return await fn();
      } finally {
        clearInterval(t);
      }
    };
    const done: string[] = [];
    let failed = false;
    let needsReboot = false;
    let guiUpdated = false;
    // Backend first (host package / rpm-ostree layer), then the app.
    if (updBackend) {
      try {
        const r = await step(m.settings_backend_updating({ version: updateLatest }), () =>
          installBackend({ version: updateLatest }),
        );
        if (r.ok) {
          done.push(
            r.needsReboot
              ? m.settings_backend_updated_reboot({ version: updateLatest })
              : m.settings_backend_updated({ version: updateLatest }),
          );
          needsReboot = !!r.needsReboot;
        } else {
          setUbLog(m.settings_backend_update_failed({ error: r.message || m.settings_unknown_error() }), 'err');
          failed = true;
        }
      } catch (e) {
        setUbLog(m.settings_backend_update_failed({ error: String(e) }), 'err');
        failed = true;
      }
    }
    if (!failed && updGui) {
      try {
        const msg = await step(m.settings_gui_downloading({ version: updateLatest }), () =>
          runUpdate(updateUrl, updateLatest),
        );
        done.push(msg);
        guiUpdated = true;
      } catch (e) {
        setUbLog(m.settings_gui_update_failed({ error: String(e) }), 'err');
        failed = true;
      }
    }
    updating = false;
    if (!failed) {
      setUbLog(done.join('  ·  ') || m.settings_already_up_to_date(), 'ok');
      // Offer the one action that actually applies the update.
      if (needsReboot) {
        restartText = m.settings_reboot_now();
        restartCmd = 'reboot_host';
        restartVisible = true;
        updateAllVisible = false;
      } else if (guiUpdated) {
        restartText = m.settings_restart_app();
        restartCmd = 'restart_app';
        restartVisible = true;
        updateAllVisible = false;
      }
    }
  }

  function onRestart(): void {
    void restart(restartCmd);
  }

  async function onCheckUpdate(): Promise<void> {
    if (!hasTauri) {
      openExt(updateUrl);
      return;
    }
    updateStatus = m.settings_checking();
    await loadAbout();
  }
</script>

<div class="settings-sec" id="sec-about" {hidden}>
  <h2>{m.settings_nav_about()}</h2>
  <p class="desc">{m.settings_about_desc()}</p>

  <div class="about-app">
    <div class="about-icon">
      <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l7 3v5c0 4.4-3 7.4-7 9-4-1.6-7-4.6-7-9V6l7-3z"/></svg>
    </div>
    <div>
      <div class="about-name">GP <span style="color:var(--accent-soft)">Client</span></div>
      <div class="about-ver" id="aboutVer">{aboutVer}</div>
    </div>
  </div>

  <div class="update-box">
    <div class="update-status" id="updateStatus">{updateStatus}</div>
    <div class="update-actions">
      <button class="btn-soft" id="checkUpdateBtn" onclick={onCheckUpdate}>{m.settings_check_updates()}</button>
      <button class="btn-action auto" id="updateAllBtn" hidden={!updateAllVisible} disabled={updating} style="padding:0 16px;" onclick={onUpdateAll}>{updating ? m.settings_updating() : m.settings_update_all()}</button>
      <button class="btn-action auto" id="restartBtn" hidden={!restartVisible} style="padding:0 16px;" onclick={onRestart}>{restartText}</button>
    </div>
    <div class={ubKind ? `bk-status ${ubKind}` : 'bk-status'} id="updateBackendLog" hidden={!ubLog} style="margin-top:8px;">{ubLog}</div>
  </div>

  <div class="lbl" style="margin-top:18px;">{m.settings_app_section()}</div>
  <div class="detail-card">
    <div class="drow"><div class="k">{m.settings_lbl_version()}</div><div class="v" id="aboutGuiVer">{aboutGuiVer}</div></div>
    <div class="drow"><div class="k">{m.settings_running_as()}</div><div class="v" id="aboutRunning">{aboutRunning}</div></div>
    <div class="drow" id="aboutRuntimeRow" hidden={!runtimeVisible}><div class="k">{m.settings_flatpak_runtime()}</div><div class="v" id="aboutRuntime">{aboutRuntime}</div></div>
    <div class="drow"><div class="k">{m.settings_updates()}</div><div class="v" id="aboutGuiUpd">{#if guiUpd.kind === 'init'}—{:else if guiUpd.kind === 'dash'}<span style="color:var(--muted)">—</span>{:else if guiUpd.kind === 'current'}<span style="color:var(--muted)">{m.settings_up_to_date()}</span>{:else}<span style="color:var(--green)">{m.settings_update_available({ version: guiUpd.latest })}</span>{/if}</div></div>
  </div>

  <div class="lbl" style="margin-top:18px;">{m.settings_backend_section()}</div>
  <div class="detail-card">
    <div class="drow"><div class="k">{m.settings_lbl_version()}</div><div class="v" id="aboutBackend">{#if backendState === 'unknown'}—{:else if backendState === 'missing'}<span style="color:var(--red)">{m.settings_not_installed()}</span>{:else}{backendText}{/if}</div></div>
    <div class="drow"><div class="k">{m.settings_install_type()}</div><div class="v" id="aboutKind">{aboutKind}</div></div>
    <div class="drow"><div class="k">{m.settings_updates()}</div><div class="v" id="aboutBeUpd">{#if beUpd.kind === 'init'}—{:else if beUpd.kind === 'dash'}<span style="color:var(--muted)">—</span>{:else if beUpd.kind === 'current'}<span style="color:var(--muted)">{m.settings_up_to_date()}</span>{:else}<span style="color:var(--green)">{m.settings_update_available({ version: beUpd.latest })}</span>{/if}</div></div>
  </div>

  <div class="lbl" style="margin-top:18px;">{m.settings_system_section()}</div>
  <div class="detail-card">
    <div class="drow"><div class="k">{m.settings_lbl_os()}</div><div class="v" id="aboutOs">{aboutOs}</div></div>
  </div>

  <p class="help">
    {m.settings_footer_copyright()}
    <!-- svelte-ignore a11y_invalid_attribute -->
    <a href="#" id="upstreamLink" onclick={(e) => { e.preventDefault(); openExt(UPSTREAM_URL); }}>yuezk/GlobalProtect-openconnect</a>.
    <!-- svelte-ignore a11y_invalid_attribute -->
    <a href="#" id="repoLink" onclick={(e) => { e.preventDefault(); openExt(REPO_URL + '/releases'); }}>{m.settings_footer_project_page()}</a>
  </p>
</div>
