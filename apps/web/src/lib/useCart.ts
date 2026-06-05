import { useState, useEffect, useCallback } from 'react';
import type { CartItem } from '@deliveryos/ui';

const CART_SCHEMA_VERSION = 1;

function isValidCartItem(item: unknown): item is CartItem {
  if (!item || typeof item !== 'object') return false;
  const i = item as Record<string, unknown>;
  return (
    typeof i.id === 'string' &&
    typeof i.productId === 'string' &&
    typeof i.price === 'number' &&
    typeof i.quantity === 'number' &&
    i.quantity > 0
  );
}

function parseStoredCart(raw: string): CartItem[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return [];
  }

  let items: unknown;

  if (Array.isArray(parsed)) {
    items = parsed;
  } else if (parsed && typeof parsed === 'object' && 'version' in parsed && 'items' in parsed) {
    const obj = parsed as Record<string, unknown>;
    if (obj.version === CART_SCHEMA_VERSION) {
      items = obj.items;
    } else {
      return [];
    }
  } else {
    return [];
  }

  if (!Array.isArray(items)) return [];
  return items.filter(isValidCartItem);
}

export function useCart(locationId: string) {
  const cartKey = `dos_cart_${locationId}`;

  const loadCart = (): CartItem[] => {
    if (typeof window === 'undefined') return [];
    const stored = localStorage.getItem(cartKey);
    if (!stored) return [];
    return parseStoredCart(stored);
  };

  const [items, setItems] = useState<CartItem[]>(loadCart);

  useEffect(() => {
    if (typeof window !== 'undefined') {
      localStorage.setItem(cartKey, JSON.stringify({ version: CART_SCHEMA_VERSION, items }));
    }
  }, [items, cartKey]);

  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === cartKey && e.newValue) {
        setItems(parseStoredCart(e.newValue));
      }
    };
    window.addEventListener('storage', handleStorage);
    return () => {
      window.removeEventListener('storage', handleStorage);
    };
  }, [cartKey]);

  const addItem = useCallback((item: CartItem) => {
    setItems(prev => {
      const existingIdx = prev.findIndex(i => i.productId === item.productId && JSON.stringify(i.options) === JSON.stringify(item.options));
      if (existingIdx >= 0) {
        const next = [...prev];
        const existing = next[existingIdx];
        if (existing) {
          existing.quantity += item.quantity;
        }
        return next;
      }
      return [...prev, item];
    });
  }, []);

  const updateQuantity = useCallback((id: string, quantity: number) => {
    setItems(prev => {
      if (quantity <= 0) return prev.filter(i => i.id !== id);
      return prev.map(i => i.id === id ? { ...i, quantity } : i);
    });
  }, []);

  const clearCart = useCallback(() => {
    setItems([]);
  }, []);

  const bounceCart = useCallback(() => {
    if (typeof window !== 'undefined') {
      window.dispatchEvent(new CustomEvent('dos:bounceCart'));
    }
  }, []);

  return { items, addItem, updateQuantity, clearCart, bounceCart };
}
