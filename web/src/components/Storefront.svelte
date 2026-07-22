<script>
  import Button from './Button.svelte';
  import StatusBadge from './StatusBadge.svelte';

  let items = [
    { id: 1, name: 'Margherita Pizza', price: 12.99, category: 'Pizza', desc: 'San Marzano tomatoes, fresh mozzarella, basil' },
    { id: 2, name: 'Truffle Burger', price: 18.50, category: 'Burgers', desc: 'Wagyu patty, truffle aioli, aged cheddar' },
    { id: 3, name: 'Caesar Salad', price: 9.99, category: 'Salads', desc: 'Romaine, parmesan, croutons, house dressing' },
    { id: 4, name: 'Sushi Platter', price: 24.00, category: 'Japanese', desc: '12-piece assorted nigiri and maki' },
    { id: 5, name: 'Pad Thai', price: 14.50, category: 'Thai', desc: 'Rice noodles, shrimp, tamarind sauce, peanuts' },
    { id: 6, name: 'Tiramisu', price: 7.50, category: 'Desserts', desc: 'Espresso-soaked ladyfingers, mascarpone cream' },
  ];
  let cart = [];
  let selectedCat = 'All';
  let showCart = false;

  $: categories = ['All', ...new Set(items.map(i => i.category))];
  $: filtered = selectedCat === 'All' ? items : items.filter(i => i.category === selectedCat);
  $: cartTotal = cart.reduce((s, i) => s + i.price * i.qty, 0).toFixed(2);
  $: cartCount = cart.reduce((s, i) => s + i.qty, 0);

  function addToCart(item) {
    const existing = cart.find(i => i.id === item.id);
    if (existing) existing.qty++;
    else cart = [...cart, { ...item, qty: 1 }];
    cart = cart;
  }
  function removeFromCart(id) {
    cart = cart.filter(i => i.id !== id);
  }
</script>

<div class="storefront">
  <header class="storefront-header">
    <h2>Menu</h2>
    <div class="cart-badge" on:click={() => showCart = !showCart}>
      <span class="cart-icon">🛒</span>
      {#if cartCount > 0}
        <span class="cart-count">{cartCount}</span>
      {/if}
    </div>
  </header>

  <div class="categories">
    {#each categories as cat}
      <button
        class="cat-btn"
        class:active={cat === selectedCat}
        on:click={() => selectedCat = cat}
      >{cat}</button>
    {/each}
  </div>

  <div class="menu-grid">
    {#each filtered as item}
      <div class="menu-item">
        <h4>{item.name}</h4>
        <p class="desc">{item.desc}</p>
        <div class="item-footer">
          <span class="price">${item.price.toFixed(2)}</span>
          <Button variant="primary" size="sm" on:click={() => addToCart(item)}>Add</Button>
        </div>
      </div>
    {/each}
  </div>
</div>

{#if showCart}
  <div class="cart-panel" transition:slide={{ duration: 200 }}>
    <div class="cart-header">
      <h3>Cart ({cartCount})</h3>
      <button class="close-btn" on:click={() => showCart = false}>✕</button>
    </div>
    {#if cart.length === 0}
      <p class="text-muted" style="padding:24px;text-align:center">Your cart is empty</p>
    {:else}
      <div class="cart-items">
        {#each cart as item}
          <div class="cart-item">
            <div>
              <strong>{item.name}</strong>
              <span class="text-muted" style="font-size:12px"> × {item.qty}</span>
            </div>
            <div class="cart-item-actions">
              <span>${(item.price * item.qty).toFixed(2)}</span>
              <button class="remove-btn" on:click={() => removeFromCart(item.id)}>✕</button>
            </div>
          </div>
        {/each}
      </div>
      <div class="cart-total">
        <strong>Total</strong>
        <strong>${cartTotal}</strong>
      </div>
      <Button variant="primary" size="lg" class="w-full" on:click={() => alert('Checkout flow — coming soon')}>
        Checkout
      </Button>
    {/if}
  </div>
{/if}

<style>
  .storefront { padding: 24px; max-width: 800px; margin: 0 auto; }
  .storefront-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
  .cart-badge { position: relative; cursor: pointer; font-size: 24px; }
  .cart-count { position: absolute; top: -8px; right: -8px; background: var(--brand-primary); color: white; font-size: 11px; width: 20px; height: 20px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-weight: 700; }
  .categories { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 24px; }
  .cat-btn { padding: 6px 16px; border: 1px solid var(--brand-border); border-radius: 20px; background: transparent; color: var(--brand-text); cursor: pointer; font-size: 13px; transition: all 150ms; }
  .cat-btn.active, .cat-btn:hover { background: var(--brand-primary); color: white; border-color: var(--brand-primary); }
  .menu-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 16px; }
  .menu-item { background: var(--brand-surface); border: 1px solid var(--brand-border); border-radius: var(--brand-radius); padding: 16px; }
  .menu-item h4 { font-size: 16px; margin-bottom: 4px; }
  .desc { font-size: 12px; color: var(--brand-text-muted); margin-bottom: 12px; }
  .item-footer { display: flex; justify-content: space-between; align-items: center; }
  .price { font-weight: 600; font-size: 18px; }
  .cart-panel { position: fixed; top: 0; right: 0; width: 360px; height: 100%; background: var(--brand-surface); border-left: 1px solid var(--brand-border); z-index: var(--z-modal); display: flex; flex-direction: column; box-shadow: var(--shadow-xl); }
  .cart-header { display: flex; justify-content: space-between; align-items: center; padding: 24px; border-bottom: 1px solid var(--brand-border); }
  .close-btn { background: none; border: none; color: var(--brand-text-muted); font-size: 20px; cursor: pointer; }
  .cart-items { flex: 1; overflow-y: auto; padding: 16px 24px; }
  .cart-item { display: flex; justify-content: space-between; align-items: center; padding: 12px 0; border-bottom: 1px solid var(--brand-border); }
  .cart-item-actions { display: flex; align-items: center; gap: 12px; }
  .remove-btn { background: none; border: none; color: var(--color-danger); cursor: pointer; font-size: 14px; }
  .cart-total { display: flex; justify-content: space-between; padding: 24px; border-top: 1px solid var(--brand-border); font-size: 18px; }
  :global(.cart-panel .btn) { margin: 0 24px 24px; }
</style>
