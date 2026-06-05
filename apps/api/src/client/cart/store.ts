export interface CartItem {
  productId: string;
  quantity: number;
  modifierIds: string[];
}

export interface CartSchema {
  v: 1;
  locationId: string;
  menuVersion: string;
  items: CartItem[];
  createdAt: number;
  updatedAt: number;
}

export function getCartKey(locationId: string) {
  return `dowiz:cart:${locationId}`;
}

export function getCart(locationId: string): CartSchema {
  const key = getCartKey(locationId);
  const raw = localStorage.getItem(key);
  if (!raw) {
    return { v: 1, locationId, menuVersion: '', items: [], createdAt: Date.now(), updatedAt: Date.now() };
  }

  try {
    const parsed = JSON.parse(raw);
    if (parsed.v !== 1 || parsed.locationId !== locationId) {
      throw new Error('Version mismatch or location mismatch');
    }
    return parsed;
  } catch (err) {
    console.error('Cart corruption detected, resetting.', err);
    // Send telemetry without PII
    fetch('/api/telemetry', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action: 'cart.corrupted', locationId })
    }).catch(() => {});
    
    return { v: 1, locationId, menuVersion: '', items: [], createdAt: Date.now(), updatedAt: Date.now() };
  }
}

export function saveCart(locationId: string, cart: CartSchema) {
  cart.updatedAt = Date.now();
  localStorage.setItem(getCartKey(locationId), JSON.stringify(cart));
  window.dispatchEvent(new CustomEvent('cart:updated', { detail: { locationId, cart } }));
}

export function clearCart(locationId: string) {
  localStorage.removeItem(getCartKey(locationId));
  window.dispatchEvent(new CustomEvent('cart:updated', { detail: { locationId, cart: getCart(locationId) } }));
}
