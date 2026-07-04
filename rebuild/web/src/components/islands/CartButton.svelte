<script lang="ts">
  // CartButton island — sticky cart bar, parity: ClientLayout.tsx:202-229 (StickyActionBar).
  // Hydration: client:idle here (README documents the real target — client:idle,
  // eager-upgraded to client:load on first add-to-cart — as Phase-B follow-up; Astro does not
  // yet expose a first-class "upgrade hydration at runtime" primitive, so today this ships as a
  // plain client:idle island that re-renders reactively once cart.count > 0).
  import { cart } from '../../lib/cart-store.svelte';
  import { cart_title } from '../../paraglide/messages.js';
</script>

{#if cart.count > 0}
  <div class="cart-bar" data-testid="cart-open">
    <button type="button" class="cart-bar-button">
      <span>{cart_title()}</span>
      <span class="cart-bar-count">{cart.count}</span>
      <span class="cart-bar-total">{(cart.total / 100).toFixed(2)}</span>
    </button>
  </div>
{/if}

<style>
  .cart-bar {
    position: fixed;
    left: 1rem;
    right: 1rem;
    bottom: 1rem;
    z-index: 50;
  }
  .cart-bar-button {
    width: 100%;
    height: 3rem;
    border-radius: var(--radius-full);
    border: none;
    background: var(--brand-primary);
    color: var(--brand-bg);
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }
  .cart-bar-count {
    background: rgba(0, 0, 0, 0.15);
    border-radius: var(--radius-full);
    padding: 0.1rem 0.5rem;
    font-size: 0.8rem;
  }
</style>
