import type { CourierTask, LngLatLike } from '@deliveryos/ui';

export interface DestinationRoute {
  hasCustomerCoords: boolean;
  hasRestaurantCoords: boolean;
  destPin: LngLatLike | undefined;
  routeLine: LngLatLike[] | undefined;
}

/**
 * LC9/S3 fix: derive the destination pin + route line from REAL task coordinates
 * only. `typeof === 'number'` (not `||`) so a legitimate 0 (equator / prime
 * meridian) coordinate is never treated as "missing". When a coordinate is
 * genuinely missing, both `destPin` and `routeLine` come back `undefined` —
 * the caller must render an explicit no-location state, never a hardcoded
 * Tirana/Durrës stand-in that would confidently route the courier to the wrong
 * place. Extracted to a pure function so this logic is unit-testable without
 * rendering the page (this repo's test runner has no DOM/jsdom).
 */
export function computeDestinationRoute(
  task: Pick<CourierTask, 'customer' | 'restaurant'> | null | undefined,
  courierPos: LngLatLike,
): DestinationRoute {
  const hasCustomerCoords = typeof task?.customer?.lat === 'number' && typeof task?.customer?.lng === 'number';
  const hasRestaurantCoords = typeof task?.restaurant?.lat === 'number' && typeof task?.restaurant?.lng === 'number';

  const destPin: LngLatLike | undefined = hasCustomerCoords
    ? [task!.customer.lng as number, task!.customer.lat as number]
    : undefined;

  const routeLine: LngLatLike[] | undefined = destPin
    ? [
        courierPos,
        ...(hasRestaurantCoords
          ? [[task!.restaurant.lng as number, task!.restaurant.lat as number] as LngLatLike]
          : []),
        destPin,
      ]
    : undefined;

  return { hasCustomerCoords, hasRestaurantCoords, destPin, routeLine };
}
