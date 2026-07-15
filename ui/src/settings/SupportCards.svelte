<script lang="ts">
  // Support section: Ko-fi / Revolut / Ethereum donation cards.
  import { m } from '../paraglide/messages.js';
  import { openUrl } from '../lib/api';
  import revolutQrUrl from '../assets/revolut-qr.png';
  import ethQrUrl from '../assets/eth-qr.png';

  interface Props {
    hidden: boolean;
  }

  let { hidden }: Props = $props();

  const KOFI_URL = 'https://ko-fi.com/techneut92?amount=2.50#checkoutModal';
  const REVOLUT_URL = 'https://revolut.me/techneut92';
  const ETH_ADDR = '0x15d9B8383A7cbe9f99F72aC29106C53bbcf4ea40';

  // ───────── ETH copy ─────────
  let ethCopyState = $state<'idle' | 'ok' | 'fail'>('idle');
  let ethTimer: ReturnType<typeof setTimeout> | undefined;
  let ethAddrEl = $state<HTMLDivElement | null>(null);
  async function onEthCopy(): Promise<void> {
    let ok = false;
    try {
      await navigator.clipboard.writeText(ETH_ADDR);
      ok = true;
    } catch {
      // Fallback: select the address so the user can copy manually.
      try {
        if (ethAddrEl) {
          const r = document.createRange();
          r.selectNodeContents(ethAddrEl);
          const s = getSelection();
          if (s) {
            s.removeAllRanges();
            s.addRange(r);
          }
          ok = typeof document.execCommand === 'function' && document.execCommand('copy');
        }
      } catch {
        /* ignore */
      }
    }
    ethCopyState = ok ? 'ok' : 'fail';
    clearTimeout(ethTimer);
    ethTimer = setTimeout(() => {
      ethCopyState = 'idle';
    }, 1600);
  }
</script>

<div class="settings-sec" id="sec-support" {hidden}>
  <h2>{m.settings_nav_support()}</h2>
  <p class="desc">{m.settings_support_desc()}</p>
  <div class="kofi-card">
    <div class="kofi-head">
      <div class="kofi-logo">
        <svg width="27" height="27" viewBox="0 0 24 24" fill="none"><path d="M5 5h12a4 4 0 0 1 0 8h-1.2A6 6 0 0 1 10 18H8a4 4 0 0 1-4-4V5z" fill="#FF5E5B"/><circle cx="9" cy="10" r="2.1" fill="#fff"/><path d="M17 7h.5a2 2 0 0 1 0 4H17" stroke="#FF5E5B" stroke-width="1.6"/></svg>
      </div>
      <div>
        <div class="kofi-title">{m.settings_kofi_title()}</div>
        <div class="kofi-sub">{m.settings_kofi_sub()}</div>
      </div>
    </div>
    <a class="kofi-btn" id="kofiBtn" href={KOFI_URL} target="_blank" rel="noreferrer" onclick={(e) => { e.preventDefault(); void openUrl(KOFI_URL); }}>
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none"><path d="M5 5h12a4 4 0 0 1 0 8h-1.2A6 6 0 0 1 10 18H8a4 4 0 0 1-4-4V5z" fill="#fff"/><circle cx="9" cy="10" r="2.1" fill="#FF5E5B"/></svg>
      <span>{m.settings_kofi_btn()}</span>
    </a>
  </div>
  <div class="revolut-card">
    <div class="kofi-head">
      <div class="revolut-logo">
        <svg width="26" height="26" viewBox="0 0 24 24"><text x="12" y="17.5" text-anchor="middle" font-family="Arial, Helvetica, sans-serif" font-size="17" font-weight="800" fill="#0666EB">R</text></svg>
      </div>
      <div>
        <div class="kofi-title">{m.settings_revolut_title()}</div>
        <div class="kofi-sub">{m.settings_revolut_sub()}</div>
      </div>
    </div>
    <div class="revolut-qr-wrap">
      <img class="revolut-qr" src={revolutQrUrl} width="150" height="150" alt={m.settings_revolut_qr_alt()} />
      <div class="revolut-qr-cap">{m.settings_revolut_qr_cap()}</div>
    </div>
    <a class="revolut-btn" id="revolutBtn" href={REVOLUT_URL} target="_blank" rel="noreferrer" onclick={(e) => { e.preventDefault(); void openUrl(REVOLUT_URL); }}>
      <svg width="18" height="18" viewBox="0 0 24 24"><text x="12" y="17.5" text-anchor="middle" font-family="Arial, Helvetica, sans-serif" font-size="17" font-weight="800" fill="#fff">R</text></svg>
      <span>{m.settings_revolut_btn()}</span>
    </a>
  </div>
  <div class="eth-card">
    <div class="kofi-head">
      <div class="eth-logo">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none"><path d="M12 2 L12 9.6 L18.5 12.5 Z" fill="#8fa2f5"/><path d="M12 2 L5.5 12.5 L12 9.6 Z" fill="#c5cff9"/><path d="M12 16.1 L12 22 L18.5 13.7 Z" fill="#8fa2f5"/><path d="M12 22 L12 16.1 L5.5 13.7 Z" fill="#c5cff9"/><path d="M12 14.9 L18.5 12.5 L12 9.6 Z" fill="#627EEA"/><path d="M5.5 12.5 L12 14.9 L12 9.6 Z" fill="#8295ef"/></svg>
      </div>
      <div>
        <div class="kofi-title">{m.settings_eth_title()}</div>
        <div class="kofi-sub">{m.settings_eth_sub()}</div>
      </div>
    </div>
    <div class="eth-qr-wrap">
      <img class="eth-qr" src={ethQrUrl} width="150" height="150" alt={m.settings_eth_qr_alt()} />
      <div class="revolut-qr-cap">{m.settings_eth_qr_cap()}</div>
    </div>
    <div class="eth-addr-row">
      <div class="eth-addr" id="ethAddr" title={ETH_ADDR} bind:this={ethAddrEl}>{ETH_ADDR}</div>
      <button class="eth-copy" class:copied={ethCopyState === 'ok'} id="ethCopy" type="button" onclick={onEthCopy}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></svg>
        <span id="ethCopyLabel">{ethCopyState === 'ok' ? m.settings_eth_copied() : ethCopyState === 'fail' ? m.settings_eth_select_copy() : m.settings_eth_copy()}</span>
      </button>
    </div>
    <div class="eth-net">{m.settings_eth_net()}</div>
  </div>
  <p class="help">{m.settings_support_help()}</p>
</div>
