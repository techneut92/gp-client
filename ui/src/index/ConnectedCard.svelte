<script lang="ts">
  // Connected content: IP/interface tiles plus the tunnel-facts detail card.
  import { m } from '../paraglide/messages.js';

  interface Props {
    hidden: boolean;
    cIp: string;
    cIface: string;
    cPortal: string;
    cGateway: string;
    cExpires: string;
  }

  let { hidden, cIp, cIface, cPortal, cGateway, cExpires }: Props = $props();
</script>

<div id="connContent" {hidden}>
  <div class="tiles">
    <div class="tile ip"><div class="k">{m.main_your_ip()}</div><div class="v" id="cIp">{cIp}</div></div>
    <div class="tile"><div class="k">{m.main_interface()}</div><div class="v" id="cIface">{cIface}</div></div>
  </div>
  <div class="detail-card">
    <div class="drow"><div class="k">{m.main_portal()}</div><div class="v" id="cPortal">{cPortal}</div></div>
    <div class="drow"><div class="k">{m.main_gateway()}</div><div class="v" id="cGateway">{cGateway}</div></div>
    <div class="drow"><div class="k">{m.main_session_expires()}</div><div class="v" id="cExpires">{cExpires}</div></div>
  </div>
</div>
