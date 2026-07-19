<script lang="ts">
  // Identities section: overview list of saved identities + Add. Selecting a row
  // (or Add) opens the single-identity editor window. The connection/SSO tuning
  // lives inside each identity now, so this is just the roster.
  import { onMount } from 'svelte';
  import { m } from '../paraglide/messages.js';
  import { listIdentities, onIdentitiesChanged, openIdentityEditor, type Identity, type UnlistenFn } from '../lib/api';
  import AuthIcon from '../lib/AuthIcon.svelte';

  interface Props {
    hidden: boolean;
  }

  let { hidden }: Props = $props();

  let identities = $state<Identity[]>([]);

  function authName(method: number | undefined): string {
    switch (method) {
      case 0:
        return m.main_auth_smart_card();
      case 1:
        return m.main_auth_cert_file();
      case 2:
        return m.main_auth_saml();
      case 3:
        return m.main_auth_password();
      default:
        return m.main_auth_auto();
    }
  }

  async function load(): Promise<void> {
    try {
      identities = await listIdentities();
    } catch {
      // vault locked / no identities
    }
  }

  onMount(() => {
    void load();
    let unlisten: UnlistenFn | undefined;
    void onIdentitiesChanged(() => void load()).then((f) => {
      unlisten = f;
    });
    return () => unlisten?.();
  });
</script>

<div class="settings-sec" id="sec-identities" {hidden}>
  <div class="id-head">
    <div>
      <h2 style="margin:0;">{m.settings_nav_identities()}</h2>
      <p class="desc" style="margin:5px 0 0;">{m.settings_identities_desc()}</p>
    </div>
    <button class="id-add-btn" id="addIdentityBtn" type="button" onclick={() => void openIdentityEditor()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>
      <span>{m.settings_identities_add()}</span>
    </button>
  </div>

  {#if identities.length}
    <div class="id-list">
      {#each identities as id (id.name)}
        <button class="id-list-row" type="button" onclick={() => void openIdentityEditor(id.name)}>
          <span class="id-list-ico"><AuthIcon method={id.auth_method} /></span>
          <span class="id-list-txt">
            <span class="id-list-nm">{id.name}</span>
            <span class="id-list-pt">{id.portal || ''}</span>
          </span>
          <span class="id-list-badge">{authName(id.auth_method)}</span>
          <svg class="id-list-chev" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 6l6 6-6 6"/></svg>
        </button>
      {/each}
    </div>
  {:else}
    <div class="id-empty">
      <div class="id-empty-txt">{m.settings_identities_empty()}</div>
      <button class="btn-action" style="width:auto;padding:11px 18px;" type="button" onclick={() => void openIdentityEditor()}>{m.main_add_identity()}</button>
    </div>
  {/if}

  <p class="help">{m.settings_identities_footer()}</p>
</div>
