// @ts-nocheck
import { getCart, saveCart, clearCart } from './store.js';
import { checkDrift } from './drift.js';
import { fetchFallbackConfig, showFallbackBanner, hideFallbackBanner, getCachedFallbackConfig } from '../shared/fallback-phone.js';
import { installCustomerErrorBoundary } from '../shared/error-boundary.js';

installCustomerErrorBoundary();

// Expose fallback banner on cart errors
window.addEventListener('fallback:needed', ((e: CustomEvent) => {
  const { reason } = e.detail;
  showFallbackBanner({ reason });
}) as EventListener);

// Minimal export to window for integration with HTML scripts
(window as any).DowizCart = {
  getCart,
  saveCart,
  clearCart,
  checkDrift,
  fetchFallbackConfig,
  showFallbackBanner,
  hideFallbackBanner,
  getCachedFallbackConfig
};
