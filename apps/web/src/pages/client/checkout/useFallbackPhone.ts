import { useState, useEffect } from 'react';

// #4 — restaurant phone for the failure fallback, cached on mount so the "call the
// restaurant" CTA never depends on a network fetch made under the same load that
// caused the failure. Null = no CTA (fail-soft to the generic toast).
export function useFallbackPhone(slug: string | undefined) {
  const [fallbackPhone, setFallbackPhone] = useState<string | null>(null);

  useEffect(() => {
    if (!slug) return;
    // Cache the restaurant phone NOW (on mount) for the order-failure fallback, so
    // the CTA is available even when the order POST fails under DB/load pressure.
    fetch(`/api/public/locations/${slug}/fallback-config`).then(r => r.json())
      .then((cfg: any) => {
        if (cfg && cfg.showPhoneOnError !== false && cfg.phone) setFallbackPhone(cfg.phone);
      })
      .catch(() => {/* fail-soft: no CTA, generic toast only */});
  }, [slug]);

  return fallbackPhone;
}
