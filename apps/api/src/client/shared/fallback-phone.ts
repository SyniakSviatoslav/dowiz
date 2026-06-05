// @ts-nocheck
export interface FallbackConfig {
  phone?: string;
  showPhoneOnError?: boolean;
  showPhoneOnOffline?: boolean;
}

const DEFAULT_CONFIG: FallbackConfig = {
  showPhoneOnError: true,
  showPhoneOnOffline: true,
};

let cachedConfig: FallbackConfig | null = null;

export async function fetchFallbackConfig(locationId: string): Promise<FallbackConfig> {
  if (cachedConfig) return cachedConfig;
  try {
    const res = await fetch(`/api/public/locations/${locationId}/fallback-config`);
    if (res.ok) {
      const data = await res.json();
      cachedConfig = { ...DEFAULT_CONFIG, ...data };
      return cachedConfig!;
    }
  } catch {
    // network error, use defaults
  }
  return DEFAULT_CONFIG;
}

export function getCachedFallbackConfig(): FallbackConfig | null {
  return cachedConfig;
}

export function showFallbackBanner(opts: {
  reason: string;
  phone?: string;
  message?: string;
}): HTMLElement {
  const existing = document.getElementById('fallbackBanner');
  if (existing) existing.remove();

  const phone = opts.phone || cachedConfig?.phone;
  const banner = document.createElement('div');
  banner.id = 'fallbackBanner';
  banner.style.cssText =
    'position:fixed;bottom:0;left:0;right:0;z-index:9999;padding:12px 16px;' +
    'background:#DC2626;color:#fff;font-size:14px;line-height:1.4;' +
    'box-shadow:0 -4px 12px rgba(0,0,0,0.15);';

  let text = opts.message || getDefaultMessage(opts.reason);
  if (phone) {
    text += ` Call us at <a href="tel:${phone}" style="color:#fff;font-weight:700;text-decoration:underline;">${phone}</a>`;
  }

  banner.innerHTML = `
    <div style="display:flex;align-items:flex-start;justify-content:space-between;gap:8px;max-width:800px;margin:0 auto;">
      <div style="flex:1;">${text}</div>
      <button onclick="this.parentElement.parentElement.remove()" style="background:none;border:none;color:#fff;font-size:20px;cursor:pointer;padding:0;line-height:1;">&times;</button>
    </div>
  `;

  document.body.appendChild(banner);
  return banner;
}

export function hideFallbackBanner(): void {
  const existing = document.getElementById('fallbackBanner');
  if (existing) existing.remove();
}

function getDefaultMessage(reason: string): string {
  const msgs: Record<string, string> = {
    jwt_expired: 'Your session has expired. Please try placing your order again.',
    ws_offline: 'We are experiencing a temporary connection issue.',
    post_failed: 'We could not process your order right now.',
    geocode_failed: 'We could not detect your location.',
    server_error: 'Our server is temporarily unavailable.',
    payment_failed: 'Payment processing is unavailable right now.',
    service_degraded: 'Some features may be unavailable.',
  };
  return msgs[reason] || 'Something went wrong.';
}

export function showDegradedBanner(reason: string): HTMLElement {
  return showFallbackBanner({ reason, message: getDefaultMessage(reason) });
}
