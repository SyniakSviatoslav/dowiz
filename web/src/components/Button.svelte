<script>
  export let variant = 'primary';
  export let size = 'md';
  export let disabled = false;
  export let href = null;
  export let type = 'button';
  export let dataSkin = null;

  $: classes = [
    'btn',
    `btn-${variant}`,
    size === 'sm' ? 'btn-sm' : size === 'lg' ? 'btn-lg' : '',
    disabled ? 'disabled' : ''
  ].filter(Boolean).join(' ');
</script>

{#if href}
  <a {href} class={classes} data-skin={dataSkin} role="button">
    <slot />
  </a>
{:else}
  <button {type} class={classes} {disabled} on:click data-skin={dataSkin}>
    <slot />
  </button>
{/if}

<style>
  .disabled { opacity: 0.5; cursor: not-allowed; pointer-events: none; }
</style>
