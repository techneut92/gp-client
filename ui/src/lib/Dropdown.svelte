<script lang="ts">
  // Reusable custom dropdown — Svelte port of gpgui's dropdown.js.
  // Same DOM structure and class names (dd-trigger/dd-menu/dd-opt…) so
  // theme.css applies unchanged: a styled trigger button plus a
  // fixed-position popover appended to <body> (escapes overflow containers).
  import { onDestroy } from 'svelte';

  export interface DropdownOption {
    value: string;
    label: string;
    sub?: string;
  }

  interface Props {
    options?: DropdownOption[];
    value?: string;
    placeholder?: string;
    onChange?: (value: string) => void;
  }

  let { options = [], value = $bindable(''), placeholder = 'Select…', onChange = undefined }: Props = $props();

  let trigger = $state<HTMLButtonElement | null>(null);
  let open = $state(false);
  let menuEl: HTMLDivElement | null = null;

  const selected = $derived(options.find((o) => o.value === value));

  function position(): void {
    if (!menuEl || !trigger) return;
    const r = trigger.getBoundingClientRect();
    menuEl.style.top = `${r.bottom + 6}px`;
    menuEl.style.left = `${r.left}px`;
    menuEl.style.width = `${r.width}px`;
  }

  function toggle(e: MouseEvent): void {
    e.stopPropagation();
    open = !open;
    if (open) queueMicrotask(position);
  }

  function pick(o: DropdownOption): void {
    value = o.value;
    open = false;
    onChange?.(o.value);
  }

  function onDocDown(e: MouseEvent): void {
    const t = e.target as Node;
    if (open && menuEl && !menuEl.contains(t) && trigger && !trigger.contains(t)) open = false;
  }
  function closeIfOpen(): void {
    if (open) open = false;
  }

  document.addEventListener('mousedown', onDocDown, true);
  window.addEventListener('scroll', closeIfOpen, true);
  window.addEventListener('resize', closeIfOpen);
  onDestroy(() => {
    document.removeEventListener('mousedown', onDocDown, true);
    window.removeEventListener('scroll', closeIfOpen, true);
    window.removeEventListener('resize', closeIfOpen);
    menuEl?.remove();
  });

  // The popover lives on <body> like the original, via a manual portal.
  function portal(node: HTMLDivElement): { destroy(): void } {
    document.body.appendChild(node);
    menuEl = node;
    return {
      destroy: () => {
        node.remove();
        menuEl = null;
      },
    };
  }
</script>

<button type="button" class="dd-trigger" class:active={open} bind:this={trigger} onclick={toggle}>
  <span class="dd-label" class:ph={!selected}>{selected ? selected.label : placeholder}</span>
  <svg class="dd-chev" width="12" height="8" viewBox="0 0 12 8" fill="none" stroke="#8a91a8" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M1 1.5l5 5 5-5" /></svg>
</button>

{#if open}
  <div class="dd-menu" use:portal>
    {#each options as o (o.value)}
      <button type="button" class="dd-opt" class:sel={o.value === value} onclick={() => pick(o)}>
        <span class="dd-opt-main">
          <span class="dd-opt-l">{o.label}</span>
          {#if o.sub}<span class="dd-opt-s">{o.sub}</span>{/if}
        </span>
        {#if o.value === value}
          <svg class="dd-check" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12l5 5L20 6" /></svg>
        {/if}
      </button>
    {/each}
  </div>
{/if}
