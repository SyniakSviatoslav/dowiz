import { getCart, saveCart, CartSchema } from './store.js';

export interface MenuProduct {
  id: string;
  price: number;
  available: boolean;
  available_names: Record<string, string>;
  modifier_groups: Array<{
    id: string;
    modifiers: Array<{
      id: string;
      price_delta: number;
      available: boolean;
      available_names: Record<string, string>;
    }>
  }>;
}

export async function fetchServerMenu(slug: string): Promise<any> {
  const res = await fetch(`/public/locations/${slug}/menu`);
  if (!res.ok) throw new Error('Failed to fetch server menu');
  return res.json();
}

export interface DriftDiff {
  hasChanges: boolean;
  oldVersion: string;
  newVersion: string;
  removed: string[];
  priceChanged: Array<{ name: string; oldPrice: number; newPrice: number }>;
  unavailable: string[];
}

export async function checkDrift(slug: string, locationId: string): Promise<DriftDiff | null> {
  const cart = getCart(locationId);
  if (cart.items.length === 0) return null;

  const serverMenu = await fetchServerMenu(slug);
  const serverVersion = serverMenu.menu_version.toString();

  // Update cart version if it's new
  if (!cart.menuVersion) {
    cart.menuVersion = serverVersion;
    saveCart(locationId, cart);
    return null;
  }

  if (cart.menuVersion === serverVersion) {
    return null; // No drift
  }

  // Drift detected! Calculate diff
  const diff: DriftDiff = {
    hasChanges: false,
    oldVersion: cart.menuVersion,
    newVersion: serverVersion,
    removed: [],
    priceChanged: [],
    unavailable: []
  };

  // Build lookup maps
  const serverProducts = new Map<string, MenuProduct>();
  const serverModifiers = new Map<string, any>();
  
  for (const cat of serverMenu.categories) {
    for (const prod of cat.products) {
      serverProducts.set(prod.id, prod);
      for (const mg of prod.modifier_groups) {
        for (const mod of mg.modifiers) {
          serverModifiers.set(mod.id, mod);
        }
      }
    }
  }

  const defaultLoc = serverMenu.default_locale;

  // Compare
  for (const item of cart.items) {
    const sp = serverProducts.get(item.productId);
    if (!sp) {
      diff.removed.push(item.productId);
      diff.hasChanges = true;
      continue;
    }

    if (!sp.available) {
      diff.unavailable.push(sp.available_names[defaultLoc]);
      diff.hasChanges = true;
      continue;
    }

    // (Simplified: in a real app, you'd cache the cart's captured price to compare. 
    // Since we only store IDs, we just warn that a version bump happened,
    // and if prices differ from what they expected. For P14, any version bump 
    // triggers the modal if there's *any* item in cart.)
    diff.hasChanges = true; 
  }

  if (diff.hasChanges) {
    return diff;
  } else {
    // If no material changes to cart items, just silent bump
    cart.menuVersion = serverVersion;
    saveCart(locationId, cart);
    return null;
  }
}
