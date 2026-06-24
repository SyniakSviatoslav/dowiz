import { safeStorage } from './safeStorage.js';
import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import type { CartItem } from '@deliveryos/ui';
import { reconcileCart, type ReconcileProduct, type ReconcileSummary } from './cartReconcile.js';

const CART_SCHEMA_VERSION = 1;

function isValidCartItem(item: unknown): item is CartItem {
  if (!item || typeof item !== 'object') return false;
  const i = item as Record<string, unknown>;
  return typeof i.id === 'string' && typeof i.productId === 'string' && typeof i.price === 'number' && typeof i.quantity === 'number' && i.quantity > 0;
}

interface StoredCart {
  items: CartItem[];
  /** menu_version these prices were last reconciled against (null = legacy/unknown). */
  pricedVersion: number | null;
}

function parseStoredCart(raw: string): StoredCart {
  const empty: StoredCart = { items: [], pricedVersion: null };
  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return { items: parsed.filter(isValidCartItem), pricedVersion: null };
    if (parsed && typeof parsed === 'object' && 'version' in parsed && 'items' in parsed) {
      const obj = parsed as Record<string, unknown>;
      if (obj.version === CART_SCHEMA_VERSION) {
        const items = Array.isArray(obj.items) ? (obj.items as unknown[]).filter(isValidCartItem) : [];
        const pricedVersion = typeof obj.pricedVersion === 'number' ? obj.pricedVersion : null;
        return { items, pricedVersion };
      }
    }
    return empty;
  } catch (err) { console.debug('[CartProvider] parse stored cart failed:', err); return empty; }
}

function readStoredCart(cartKey: string): StoredCart {
  if (typeof window === 'undefined') return { items: [], pricedVersion: null };
  const stored = safeStorage.get(cartKey);
  return stored ? parseStoredCart(stored) : { items: [], pricedVersion: null };
}

interface CartContextType {
  items: CartItem[];
  addItem: (item: CartItem) => void;
  updateQuantity: (id: string, quantity: number) => void;
  clearCart: () => void;
  bounceCart: () => void;
  /**
   * F9: reconcile the persisted cart to a freshly-loaded menu so checkout never
   * ambushes the customer with a price/availability change the server would hard-block.
   * Re-prices modifier-free lines whose stored price drifted, drops items no longer on
   * the menu (sold-out/deleted), and stamps the menu_version. Returns a summary of
   * changes for a non-blocking notice, or null when nothing changed.
   */
  reconcileToMenu: (menuVersion: number, products: ReconcileProduct[]) => ReconcileSummary | null;
}

const CartContext = createContext<CartContextType | null>(null);

export function CartProvider({ children, locationId }: { children: React.ReactNode; locationId: string }) {
  const cartKey = `dos_cart_${locationId}`;
  const [items, setItems] = useState<CartItem[]>(() => readStoredCart(cartKey).items);
  const [pricedVersion, setPricedVersion] = useState<number | null>(() => readStoredCart(cartKey).pricedVersion);

  useEffect(() => {
    safeStorage.set(cartKey, JSON.stringify({ version: CART_SCHEMA_VERSION, items, pricedVersion }));
  }, [items, pricedVersion, cartKey]);

  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === cartKey && e.newValue) {
        const next = parseStoredCart(e.newValue);
        setItems(next.items);
        setPricedVersion(next.pricedVersion);
      }
    };
    window.addEventListener('storage', handleStorage);
    return () => window.removeEventListener('storage', handleStorage);
  }, [cartKey]);

  const reconcileToMenu = useCallback((menuVersion: number, products: ReconcileProduct[]): ReconcileSummary | null => {
    const result = reconcileCart(items, pricedVersion, menuVersion, products);
    if (result.items !== items) setItems(result.items);
    setPricedVersion(result.pricedVersion);
    return result.summary;
  }, [items, pricedVersion]);

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
    <CartContext.Provider value={{ items, addItem, updateQuantity, clearCart, bounceCart, reconcileToMenu }}>
      {children}
    </CartContext.Provider>
  );
}

export function useSharedCart() {
  const ctx = useContext(CartContext);
  if (!ctx) throw new Error('useSharedCart must be used inside CartProvider');
  return ctx;
}
