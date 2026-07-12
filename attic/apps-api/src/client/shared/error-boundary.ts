// @ts-nocheck
import { showFallbackBanner } from './fallback-phone.js';

export function installCustomerErrorBoundary(locationId?: string): void {
  window.addEventListener('error', (event) => {
    console.error('[ErrorBoundary] Unhandled error:', event.error || event.message);
    showFallbackBanner({
      reason: 'server_error',
      message: 'Something went wrong. Please try refreshing the page.',
    });
  });

  window.addEventListener('unhandledrejection', (event) => {
    console.error('[ErrorBoundary] Unhandled rejection:', event.reason);
    showFallbackBanner({
      reason: 'server_error',
      message: 'Something went wrong. Please try refreshing the page.',
    });
  });

  const origFetch = window.fetch;
  window.fetch = function (...args: Parameters<typeof fetch>): Promise<Response> {
    return origFetch.apply(this, args).catch((err) => {
      console.error('[ErrorBoundary] Fetch failed:', err);
      showFallbackBanner({
        reason: 'server_error',
        message: 'Network error. Please check your connection.',
      });
      throw err;
    });
  };
}
