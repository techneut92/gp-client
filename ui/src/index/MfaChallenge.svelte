<script lang="ts">
  // Inline MFA challenge shown in the connecting window (design §9). Handles the
  // code methods (TOTP / SMS / RSA token — a 6-box one-time code) and the
  // push / tap-to-confirm variant ("Waiting for approval"). The backend wiring
  // that actually surfaces the challenge and completes it is GPS-16 (push:
  // GPS-17); this component is dormant until the `state` event carries
  // mfa_required, but it is fully built for that day.
  import { m } from '../paraglide/messages.js';

  interface Props {
    method: string; // "totp" | "sms" | "push"
    prompt?: string | undefined;
    onSubmit: (code: string) => void;
    onResend: () => void;
  }

  let { method, prompt, onSubmit, onResend }: Props = $props();

  // Local override for the "switch method" affordance; falls back to the prop.
  let override = $state<string | null>(null);
  const curMethod = $derived(override ?? method ?? 'totp');
  let digits = $state<string[]>(['', '', '', '', '', '']);
  let inputs = $state<HTMLInputElement[]>([]);
  let error = $state('');
  let resent = $state(false);

  const isPush = $derived(curMethod === 'push');
  const code = $derived(digits.join(''));
  const promptTitle = $derived(prompt || (curMethod === 'sms' ? m.mfa_prompt_sms() : curMethod === 'push' ? m.mfa_prompt_push() : m.mfa_prompt_totp()));
  const hint = $derived(curMethod === 'sms' ? m.mfa_hint_sms() : m.mfa_hint_totp());
  const pushHint = $derived(resent ? m.mfa_push_resent() : m.mfa_push_sent());
  const switchLabel = $derived(curMethod === 'totp' ? m.mfa_switch_sms() : curMethod === 'sms' ? m.mfa_switch_push() : m.mfa_switch_code());
  const resendLabel = $derived(isPush ? m.mfa_resend_push() : m.mfa_resend_code());

  function setDigit(i: number, v: string): void {
    const d = v.replace(/\D/g, '').slice(0, 1);
    digits[i] = d;
    error = '';
    if (d && i < 5) inputs[i + 1]?.focus();
  }
  function onKey(i: number, e: KeyboardEvent): void {
    if (e.key === 'Backspace' && !digits[i] && i > 0) inputs[i - 1]?.focus();
    else if (e.key === 'Enter') verify();
  }
  function onPaste(e: ClipboardEvent): void {
    const t = (e.clipboardData?.getData('text') || '').replace(/\D/g, '').slice(0, 6);
    if (!t) return;
    e.preventDefault();
    const d = ['', '', '', '', '', ''];
    for (let k = 0; k < t.length; k++) d[k] = t.charAt(k);
    digits = d;
    error = '';
    inputs[Math.min(t.length, 5)]?.focus();
  }
  function verify(): void {
    if (code.length < 6) {
      error = m.mfa_enter_all();
      return;
    }
    onSubmit(code);
  }
  function resend(): void {
    resent = true;
    error = '';
    onResend();
  }
  function switchMethod(): void {
    override = curMethod === 'totp' ? 'sms' : curMethod === 'sms' ? 'push' : 'totp';
    digits = ['', '', '', '', '', ''];
    error = '';
    resent = false;
  }
</script>

<div class="chal-card mfa">
  <div class="chal-head">
    <span class="chal-ico">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="10" width="16" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/><circle cx="12" cy="15.5" r="1.3" fill="currentColor" stroke="none"/></svg>
    </span>
    <div style="min-width:0;">
      <div class="chal-title">{m.mfa_title()}</div>
      <div class="chal-sub">{promptTitle}</div>
    </div>
  </div>

  {#if !isPush}
    <div class="otp-row" onpaste={onPaste}>
      {#each digits as d, i (i)}
        <input
          class="otp-box"
          bind:this={inputs[i]}
          value={d}
          oninput={(e) => setDigit(i, e.currentTarget.value)}
          onkeydown={(e) => onKey(i, e)}
          inputmode="numeric"
          maxlength="1"
          aria-label={`digit ${i + 1}`}
        />
      {/each}
    </div>
    {#if error}<div class="chal-err">{error}</div>{/if}
    <div class="chal-hint">{hint}</div>
    <button class="btn-action chal-btn" disabled={code.length < 6} onclick={verify}>{m.mfa_verify()}</button>
  {:else}
    <div class="push-wait">
      <span class="push-ico">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="7" y="3" width="10" height="18" rx="2.5"/><path d="M11 18h2"/></svg>
      </span>
      <div class="push-title">{m.mfa_waiting()}</div>
      <div class="push-line"><span class="chal-spin"></span><span class="chal-hint" style="margin:0;">{pushHint}</span></div>
    </div>
  {/if}

  <div class="chal-links">
    <button class="chal-link accent" onclick={resend}>{resendLabel}</button>
    <span class="chal-dot"></span>
    <button class="chal-link" onclick={switchMethod}>{switchLabel}</button>
  </div>
</div>
