<script>
  // ── Kernel status enum mirror ───────────────────────────────────────
  // Mirrors kernel/src/order_machine.rs OrderStatus::as_str() so the
  // frontend can speak the canonical string form the kernel emits.
  // (Pending -> "PENDING", etc.) Not needed for the create payload (the
  // server stamps status = PENDING), but kept for status-driven UI later.
  const STATUS = Object.freeze({
    Pending: 'PENDING',
    Confirmed: 'CONFIRMED',
    Preparing: 'PREPARING',
    Ready: 'READY',
    InDelivery: 'IN_DELIVERY',
    Delivered: 'DELIVERED',
    Rejected: 'REJECTED',
    Cancelled: 'CANCELLED',
    Scheduled: 'SCHEDULED',
    PickedUp: 'PICKED_UP',
  });

  // locationId is passed from index.astro; defaulted here so the island is
  // usable standalone.
  let { locationId = '00000000-0000-0000-0000-000000000001' } = $props();

  // ── Mock menu (oracle-shaped: product/modifier ids are uuids) ───────
  // price / priceDelta are integer minor units (cents). No float money.
  const menu = [
    {
      id: '11111111-1111-1111-1111-111111111111',
      name: 'Margherita',
      price: 900,
      modifiers: [
        { id: '21111111-1111-1111-1111-111111111111', name: 'Extra cheese', delta: 150 },
        { id: '22111111-1111-1111-1111-111111111111', name: 'No onions', delta: 0 },
      ],
    },
    {
      id: '12222222-2222-2222-2222-222222222222',
      name: 'Pepperoni',
      price: 1100,
      modifiers: [
        { id: '23222222-2222-2222-2222-222222222222', name: 'Double pepperoni', delta: 250 },
      ],
    },
  ];

  // cart: product_id -> { quantity, modifierIds[] }
  // selected: product_id -> modifier_id[]
  let cart = $state({});
  let selected = $state({});
  let cashPayWith = $state(0); // integer minor units tendered
  let lastResult = $state(null);
  let submitting = $state(false);

  function toggleModifier(productId, modId) {
    const cur = new Set(selected[productId] || []);
    if (cur.has(modId)) cur.delete(modId);
    else cur.add(modId);
    selected[productId] = [...cur];
  }

  function addToCart(product) {
    const mods = [...(selected[product.id] || [])];
    const existing = cart[product.id];
    if (existing) {
      existing.quantity += 1;
      existing.modifierIds = mods;
    } else {
      cart[product.id] = { quantity: 1, modifierIds: mods };
    }
  }

  function setQty(productId, delta) {
    const line = cart[productId];
    if (!line) return;
    line.quantity += delta;
    if (line.quantity <= 0) delete cart[productId];
  }

  // ── Kernel-compatible payload builder ──────────────────────────────
  // Shape matches kernel/oracle canonical order body:
  //   { locationId, items[{ product_id, modifier_ids[], quantity }], cash_pay_with }
  // cash_pay_with is integer minor units (nullable when absent).
  function buildOrderPayload() {
    const items = Object.entries(cart).map(([product_id, line]) => ({
      product_id,
      modifier_ids: line.modifierIds,
      quantity: line.quantity,
    }));
    return {
      locationId,
      items,
      cash_pay_with: cashPayWith > 0 ? cashPayWith : null,
    };
  }

  // Static exact line math (NO money tween / animation — integer cents only).
  const cartLines = $derived(
    Object.entries(cart).map(([product_id, line]) => {
      const product = menu.find((m) => m.id === product_id);
      const modDelta = line.modifierIds.reduce((sum, mid) => {
        const mod = product?.modifiers.find((m) => m.id === mid);
        return sum + (mod ? mod.delta : 0);
      }, 0);
      const unit = (product?.price ?? 0) + modDelta;
      return {
        product_id,
        name: product?.name ?? product_id,
        unit,
        quantity: line.quantity,
        lineTotal: unit * line.quantity,
      };
    })
  );

  const subtotal = $derived(cartLines.reduce((s, l) => s + l.lineTotal, 0));

  // Kernel loader — dynamically imported in the browser only (the wasm glue is
  // --target web and relies on fetch/URL). SSR-safe: never imported at module
  // top-level, so the Astro server render never touches wasm.
  let kernelPlaceOrder = null;

  async function loadKernel() {
    if (kernelPlaceOrder) return kernelPlaceOrder;
    const mod = await import('../lib/kernel.js');
    kernelPlaceOrder = mod.placeOrder;
    return kernelPlaceOrder;
  }

  async function submitOrder() {
    if (typeof window === 'undefined') return; // SSR guard
    if (Object.keys(cart).length === 0) return;
    submitting = true;
    try {
      const placeOrder = await loadKernel();
      const order = await placeOrder(buildOrderPayload());
      lastResult = order; // real kernel Order JSON { id, status, ... }
    } finally {
      submitting = false;
    }
  }
</script>

<section>
  <h2>Menu</h2>
  {#each menu as product}
    <div>
      <strong>{product.name}</strong> — {(product.price / 100).toFixed(2)} €
      {#if product.modifiers.length}
        <div>
          {#each product.modifiers as mod}
            <label>
              <input
                type="checkbox"
                checked={selected[product.id]?.includes(mod.id) ?? false}
                onchange={() => toggleModifier(product.id, mod.id)}
              />
              {mod.name} (+{(mod.delta / 100).toFixed(2)} €)
            </label>
          {/each}
        </div>
      {/if}
      <button onclick={() => addToCart(product)}>Add to cart</button>
    </div>
  {/each}
</section>

<section>
  <h2>Cart</h2>
  {#if cartLines.length === 0}
    <p>Cart is empty.</p>
  {/if}
  {#each cartLines as line}
    <div>
      {line.name} ×{line.quantity} = {(line.lineTotal / 100).toFixed(2)} €
      <button onclick={() => setQty(line.product_id, -1)}>−</button>
      <button onclick={() => setQty(line.product_id, 1)}>＋</button>
    </div>
  {/each}
  {#if cartLines.length}
    <p><strong>Subtotal:</strong> {(subtotal / 100).toFixed(2)} €</p>
    <label>
      Cash pay with (minor units, e.g. 5000 = 50.00 €):
      <input type="number" min="0" step="1" bind:value={cashPayWith} />
    </label>
    <button onclick={submitOrder} disabled={submitting}>
      {submitting ? 'Placing…' : 'Place order'}
    </button>
  {/if}
</section>

{#if lastResult}
  <section>
    <h2>Order placed</h2>
    <p><strong>Order id:</strong> {lastResult.id}</p>
    <p><strong>Status:</strong> {lastResult.status}</p>
    <pre>{JSON.stringify(lastResult, null, 2)}</pre>
  </section>
{/if}
