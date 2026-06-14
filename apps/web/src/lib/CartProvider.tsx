import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import type { CartItem } from '@deliveryos/ui';

const CART_SCHEMA_VERSION = 1;

function isValidCartItem(item: unknown): item is CartItem {
  if (!item || typeof item !== 'object') return false;
  const i = item as Record<string, unknown>;
  return typeof i.id === 'string' && typeof i.productId === 'string' && typeof i.price === 'number' && typeof i.quantity === 'number' && i.quantity > 0;
}

function parseStoredCart(raw: string): CartItem[] {
  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed.filter(isValidCartItem);
    if (parsed && typeof parsed === 'object' && 'version' in parsed && 'items' in parsed) {
      const obj = parsed as Record<string, unknown>;
      if (obj.version === CART_SCHEMA_VERSION) {
        const items = obj.items;
        return Array.isArray(items) ? items.filter(isValidCartItem) : [];
      }
    }
    return [];
  } catch (err) { console.debug('[CartProvider] parse stored cart failed:', err); return []; }
}

interface CartContextType {
  items: CartItem[];
  addItem: (item: CartItem) => void;
  updateQuantity: (id: string, quantity: number) => void;
  clearCart: () => void;
  bounceCart: () => void;
}

const CartContext = createContext<CartContextType | null>(null);

export function CartProvider({ children, locationId }: { children: React.ReactNode; locationId: string }) {
  const cartKey = `dos_cart_${locationId}`;
  const [items, setItems] = useState<CartItem[]>(() => {
    if (typeof window === 'undefined') return [];
    const stored = localStorage.getItem(cartKey);
    return stored ? parseStoredCart(stored) : [];
  });

  useEffect(() => {
    localStorage.setItem(cartKey, JSON.stringify({ version: CART_SCHEMA_VERSION, items }));
  }, [items, cartKey]);

  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === cartKey && e.newValue) setItems(parseStoredCart(e.newValue));
    };
    window.addEventListener('storage', handleStorage);
    return () => window.removeEventListener('storage', handleStorage);
  }, [cartKey]);

  const addItem = useCallback((item: CartItem) => {
    setItems(prev => {
      const key = (i: CartItem) => `${i.productId}_${JSON.stringify(i.options || {})}`;
      const itemKey = key(item);
      const idx = prev.findIndex(i => key(i) === itemKey);
      if (idx >= 0) {
        return prev.map((existing, i) => i === idx ? { ...existing, quantity: existing.quantity + item.quantity } : existing);
      }
      return [...prev, item];
    });
  }, []);

  const updateQuantity = useCallback((id: string, quantity: number) => {
    setItems(prev => quantity <= 0 ? prev.filter(i => i.id !== id) : prev.map(i => i.id === id ? { ...i, quantity } : i));
  }, []);

  const clearCart = useCallback(() => setItems([]), []);

  const bounceCart = useCallback(() => {
    if (typeof window !== 'undefined') window.dispatchEvent(new CustomEvent('dos:bounceCart'));
  }, []);

  return (
    <CartContext.Provider value={{ items, addItem, updateQuantity, clearCart, bounceCart }}>
      {children}
    </CartContext.Provider>
  );
}

export function useSharedCart() {
  const ctx = useContext(CartContext);
  if (!ctx) throw new Error('useSharedCart must be used inside CartProvider');
  return ctx;
}
