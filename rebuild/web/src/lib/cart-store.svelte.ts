// Shared cart runes store — module-level $state singleton, consumed by the MenuBrowser and
// CartButton islands (parity: apps/web/src/lib/CartProvider.tsx + REBUILD-MAP inventory 11 §3.4
// "cart runes store 🔴 (shared by MenuBrowser + CartCheckout islands)").
//
// File extension NOTE (correctness, not style): Svelte 5 only compiles `$state`/`$derived` runes
// inside `.svelte` files OR files ending `.svelte.js`/`.svelte.ts` — a plain `.ts` file with
// `$state()` in it fails to compile. This file was originally authored as `cart-store.ts` and
// had to be renamed; kept as a one-line lesson for the next island that needs a shared store.
export interface CartItem {
  id: string;
  name: string;
  /** INTEGER minor units only (red-line #2; mirrors crates/domain Lek(i64) + api-types.d.ts
   *  "integer minor units"). Guarded at every mutation — floats never enter the store. */
  price: number;
  quantity: number;
}

// Red-line #2 boundary guard: money is integer minor units, quantities are positive integers.
// The S1 contract already promises integers; this makes a violating payload throw at the edge
// instead of silently doing float arithmetic in total().
function assertIntMinorUnits(price: number): number {
  if (!Number.isSafeInteger(price) || price < 0) {
    throw new RangeError(`cart: price must be a non-negative integer in minor units, got ${price}`);
  }
  return price;
}
function assertIntQty(qty: number): number {
  if (!Number.isSafeInteger(qty)) {
    throw new RangeError(`cart: quantity must be an integer, got ${qty}`);
  }
  return qty;
}

class CartStore {
  items = $state<CartItem[]>([]);

  get count(): number {
    return this.items.reduce((sum, i) => sum + i.quantity, 0);
  }

  get total(): number {
    return this.items.reduce((sum, i) => sum + i.price * i.quantity, 0);
  }

  add(item: Omit<CartItem, 'quantity'>, qty = 1): void {
    assertIntMinorUnits(item.price);
    assertIntQty(qty);
    const existing = this.items.find((i) => i.id === item.id);
    if (existing) {
      existing.quantity += qty;
    } else {
      this.items.push({ ...item, quantity: qty });
    }
  }

  setQuantity(id: string, quantity: number): void {
    assertIntQty(quantity);
    if (quantity <= 0) {
      this.items = this.items.filter((i) => i.id !== id);
      return;
    }
    const existing = this.items.find((i) => i.id === id);
    if (existing) existing.quantity = quantity;
  }

  clear(): void {
    this.items = [];
  }
}

// One instance per page load (module-scope singleton) — Astro islands sharing the same client
// bundle chunk see the same instance, matching today's React CartProvider context semantics.
export const cart = new CartStore();
