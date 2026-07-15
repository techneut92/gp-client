<script lang="ts">
  // Authentication section: how the SAML/SSO flow is presented.
  import { m } from '../paraglide/messages.js';
  import Dropdown from '../lib/Dropdown.svelte';
  import type { SettingsForm } from '../lib/api';

  interface Props {
    hidden: boolean;
    model: SettingsForm;
    onSave: () => void;
  }

  let { hidden, model = $bindable(), onSave }: Props = $props();

  const ssoOptions = [
    { value: 'webview', label: m.settings_sso_webview(), sub: m.settings_sso_webview_sub() },
    { value: 'browser', label: m.settings_sso_browser(), sub: m.settings_sso_browser_sub() },
  ];
</script>

<div class="settings-sec" id="sec-auth" {hidden}>
  <h2>{m.settings_nav_auth()}</h2>
  <p class="desc">{m.settings_auth_desc()}</p>
  <div class="lbl">{m.settings_sso_method()}</div>
  <div id="ssoDD"><Dropdown options={ssoOptions} bind:value={model.authView} placeholder={m.common_select()} onChange={() => void onSave()} /></div>
  <p class="help">{m.settings_sso_help()}</p>
</div>
