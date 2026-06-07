import { useState, useEffect, useCallback } from 'react';

export interface CartItem {
  id: string;
  menuItemId: string;
  name: string;
  price: number;
  quantity: number;
  modifiers?: Record<string, string>;
  specialInstructions?: string;
}

export interface CartState {
  items: CartItem[];
  restaurantId: string;
  menuVersion: number;
  embedPrefix: string;
}

interface CartPersist {
  items: CartItem[];
  restaurantId: string;
  menuVersion: number;
  embedPrefix: string;
}

function getStorageKey(): string {
  if (typeof window === 'undefined') return 'dos_cart';
  const params = new URLSearchParams(window.location.search);
  const prefix = params.get('embed') === 'true' ? 'embed_' : '';
  return `${prefix}dos_cart`;
}

function loadCart(): CartState {
  try {
    const key = getStorageKey();
    const raw = localStorage.getItem(key);
    if (!raw) return { items: [], restaurantId: '', menuVersion: 0, embedPrefix: key.replace('dos_cart', '') };
    const parsed: CartPersist = JSON.parse(raw);
    return {
      items: parsed.items || [],
      restaurantId: parsed.restaurantId || '',
      menuVersion: parsed.menuVersion || 0,
      embedPrefix: parsed.embedPrefix || '',
    };
  } catch {
    return { items: [], restaurantId: '', menuVersion: 0, embedPrefix: '' };
  }
}

function persistCart(state: CartState): void {
  try {
    const key = getStorageKey();
    const persist: CartPersist = {
      items: state.items,
      restaurantId: state.restaurantId,
      menuVersion: state.menuVersion,
      embedPrefix: state.embedPrefix,
    };
    localStorage.setItem(key, JSON.stringify(persist));
  } catch {
    // Storage full or unavailable — silently fail
    console.debug('[use-cart] localStorage write failed');
  }
}

export function useCart(restaurantId?: string, menuVersion?: number) {
  const [cart, setCart] = useState<CartState>(loadCart);

  // Drift detection on menu version change
  useEffect(() => {
    if (menuVersion !== undefined && cart.menuVersion > 0 && cart.menuVersion !== menuVersion) {
      setCart((prev) => ({ ...prev, items: [] }));
    }
  }, [menuVersion, cart.menuVersion]);

  // Initialize restaurantId
  useEffect(() => {
    if (restaurantId && !cart.restaurantId) {
      setCart((prev) => ({ ...prev, restaurantId: restaurantId! }));
    }
  }, [restaurantId, cart.restaurantId]);

  // Persist on change
  useEffect(() => {
    persistCart(cart);
  }, [cart]);

  const addItem = useCallback(
    (item: Omit<CartItem, 'quantity'>, quantity = 1) => {
      setCart((prev) => {
        if (prev.restaurantId && prev.restaurantId !== restaurantId) {
          return { ...prev, items: [{ ...item, quantity }], restaurantId: restaurantId || prev.restaurantId, menuVersion: menuVersion ?? prev.menuVersion };
        }
        const existing = prev.items.find((i) => i.id === item.id);
        if (existing) {
          return {
            ...prev,
            items: prev.items.map((i) =>
              i.id === item.id ? { ...i, quantity: i.quantity + quantity } : i,
            ),
          };
        }
        return {
          ...prev,
          items: [...prev.items, { ...item, quantity }],
          restaurantId: restaurantId || prev.restaurantId,
          menuVersion: menuVersion ?? prev.menuVersion,
        };
      });
    },
    [restaurantId, menuVersion],
  );

  const updateQuantity = useCallback((itemId: string, delta: number) => {
    setCart((prev) => ({
      ...prev,
      items: prev.items
        .map((i) => (i.id === itemId ? { ...i, quantity: Math.max(0, i.quantity + delta) } : i))
        .filter((i) => i.quantity > 0),
    }));
  }, []);

  const clearCart = useCallback(() => {
    setCart({ items: [], restaurantId: '', menuVersion: 0, embedPrefix: '' });
  }, []);

  const total = cart.items.reduce((sum, item) => sum + item.price * item.quantity, 0);
  const itemCount = cart.items.reduce((sum, item) => sum + item.quantity, 0);

  return {
    items: cart.items,
    restaurantId: cart.restaurantId,
    total,
    itemCount,
    addItem,
    updateQuantity,
    clearCart,
  };
}
