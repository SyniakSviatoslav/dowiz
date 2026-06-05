import { getCart, saveCart, CartItem } from '../cart/store.js';

let currentLocationId = '';
let currentMenuVersion = '';

document.addEventListener('DOMContentLoaded', () => {
  const metaLocation = document.querySelector('meta[name="dos-location-id"]');
  const metaVersion = document.querySelector('meta[name="dos-menu-version"]');
  
  if (metaLocation) currentLocationId = metaLocation.getAttribute('content') || '';
  if (metaVersion) currentMenuVersion = metaVersion.getAttribute('content') || '';

  updateCartUI();
});

function updateCartUI() {
  if (!currentLocationId) return;
  
  const cart = getCart(currentLocationId);
  const itemCount = cart.items.reduce((sum, item) => sum + item.quantity, 0);
  
  // Actually, price logic should ideally be calculated, but for optimistic UI 
  // we either store prices in cart or recalculate from DOM data.
  // In `DowizCart` we rely on backend for total, but for instant UI we can do a naive total.
  
  const headerCount = document.getElementById('headerCartCount');
  if (headerCount) headerCount.textContent = String(itemCount);
  
  const fabWrapper = document.getElementById('cartFabWrapper');
  if (fabWrapper) {
    if (itemCount > 0) {
      fabWrapper.classList.remove('hidden');
      document.getElementById('fabCount')!.textContent = String(itemCount);
      // Naive total update is complex without full cart price data, 
      // but let's assume we store optimistic prices or just leave it for now
    } else {
      fabWrapper.classList.add('hidden');
    }
  }
}

export function addToCart(event: Event, productId: string, basePrice: number) {
  if (event) {
    event.preventDefault();
    event.stopPropagation();
  }
  
  if (!currentLocationId) return;

  const cart = getCart(currentLocationId);
  
  // If cart is from an older menu version, we should ideally clear or warn.
  // For simplicity, just update version if empty
  if (cart.items.length === 0) {
    cart.menuVersion = currentMenuVersion;
  }

  const existing = cart.items.find((i: CartItem) => i.productId === productId && i.modifierIds.length === 0);
  if (existing) {
    existing.quantity += 1;
  } else {
    cart.items.push({
      productId,
      quantity: 1,
      modifierIds: []
    });
  }

  saveCart(currentLocationId, cart);
  updateCartUI();

  // Animation
  const fabBtn = document.getElementById('cartFabBtn');
  if (fabBtn) {
    fabBtn.classList.remove('cart-bounce');
    void fabBtn.offsetWidth; // trigger reflow
    fabBtn.classList.add('cart-bounce');
  }
}

export function toggleClosedOverlay() {
  const overlay = document.getElementById('closedOverlay');
  if (overlay) {
    if (overlay.classList.contains('hidden')) {
      overlay.classList.remove('hidden');
      overlay.classList.add('flex');
    } else {
      overlay.classList.add('hidden');
      overlay.classList.remove('flex');
    }
  }
}

// Expose to window for inline onclick handlers
(window as any).DowizMenu = {
  addToCart,
  toggleClosedOverlay,
  updateCartUI
};
