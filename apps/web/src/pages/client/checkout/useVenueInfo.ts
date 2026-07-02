import { useState, useEffect } from 'react';
import type { LngLatLike } from '@deliveryos/ui';
import { fetchVenueInfo } from '../../../lib/publicApi.js';

// Delivery-fee inputs from /info → drives the client total MIRROR (ADR-0005). Defaults degrade
// safely: until /info loads (or for distance-tiered venues) the fee is "unknown" and we never
// pre-quote an exact total/cash figure — the server total + the cash-422 backstop are authoritative.
export interface FeeInputs {
  deliveryFeeFlat: number | null;
  freeDeliveryThreshold: number | null;
  minOrderValue: number | null;
  taxRate: number;
  priceIncludesTax: boolean;
  hasDistanceTiers: boolean;
}

// Venue identity + fee inputs for checkout, from GET /public/locations/:slug/info.
// BUG-1: when /info fails, locationId stays null and every Place-Order submit is a
// silent no-op behind an active-looking button. Track the failure so we can DISABLE
// the button and show a humane, retryable message instead of failing silently.
export function useVenueInfo(slug: string | undefined) {
  const [locationId, setLocationId] = useState<string | null>(null);
  // Pickup card shows the REAL venue (name + address) from /info — never a hardcoded address.
  const [pickupName, setPickupName] = useState('');
  const [pickupAddress, setPickupAddress] = useState('');
  const [locationLoadFailed, setLocationLoadFailed] = useState(false);
  const [locationCenter, setLocationCenter] = useState<LngLatLike>([19.456, 41.324]); // Durrës default
  const [feeInputs, setFeeInputs] = useState<FeeInputs | null>(null);
  const [currencyCode, setCurrencyCode] = useState<string>('ALL');

  useEffect(() => {
    if (!slug) return;
    setLocationLoadFailed(false);
    fetchVenueInfo(slug)
      .then((info) => {
        if (!info?.id) throw new Error('info: missing id');
        setLocationLoadFailed(false);
        setLocationId(info.id);
        if (info.name) setPickupName(info.name);
        if (info.address) setPickupAddress(info.address);
        if (info.currency_code) setCurrencyCode(info.currency_code);
        setFeeInputs({
          deliveryFeeFlat: info.deliveryFeeFlat ?? null,
          freeDeliveryThreshold: info.freeDeliveryThreshold ?? null,
          minOrderValue: info.minOrderValue ?? null,
          taxRate: typeof info.taxRate === 'number' ? info.taxRate : 0,
          priceIncludesTax: info.priceIncludesTax !== false,
          hasDistanceTiers: info.hasDistanceTiers === true,
        });
        if (info.lng && info.lat) setLocationCenter([info.lng, info.lat]);
      })
      .catch((err) => {
        console.debug('[CheckoutPage] failed to load location info:', err);
        setLocationId(null);
        setLocationLoadFailed(true);
      });
  }, [slug]);

  return {
    locationId,
    pickupName,
    pickupAddress,
    locationLoadFailed,
    setLocationLoadFailed,
    locationCenter,
    feeInputs,
    currencyCode,
  };
}
