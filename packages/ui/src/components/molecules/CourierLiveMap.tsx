import { MapLibreBase, type LngLatLike } from './MapLibreBase.js';
import { useCourierMarker } from '../../hooks/use-courier-marker.js';
import { useI18n } from '../../lib/I18nProvider.js';

function getCSSVar(name: string, fallback = ''): string {
  if (typeof document === 'undefined') return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

interface CourierOnMap {
  id: string;
  name: string;
  initials: string;
  lngLat: LngLatLike;
  status: 'online' | 'busy' | 'offline';
  heading?: number;
}

interface CourierLiveMapProps {
  className?: string;
  couriers?: CourierOnMap[];
  /** Single live courier (client view) — smoothly tweened + rotated by heading.
   *  Pass the RAW latest ping; the rAF tween is isolated inside this component. */
  liveCourier?: { lat: number; lng: number; recordedAt?: number } | null;
  destinationPin?: LngLatLike;
  clientLocation?: LngLatLike;
  routeLine?: LngLatLike[];
  center?: LngLatLike;
  zoom?: number;
}

export function CourierLiveMap({
  className = 'h-64 w-full rounded-lg',
  couriers = [],
  liveCourier,
  destinationPin,
  clientLocation,
  routeLine,
  center = [19.817, 41.331],
  zoom = 13,
}: CourierLiveMapProps) {
  const { t } = useI18n();
  const smoothed = useCourierMarker(
    liveCourier ? { lat: liveCourier.lat, lng: liveCourier.lng, recordedAt: liveCourier.recordedAt } : null,
    { pingIntervalMs: 3000 },
  );
  const courierMarker = smoothed
    ? { lngLat: [smoothed.lng, smoothed.lat] as LngLatLike, bearing: smoothed.bearing }
    : null;
  const statusColors: Record<string, string> = {
    online: getCSSVar('--color-success'),
    busy: getCSSVar('--color-warning'),
    offline: getCSSVar('--brand-text-muted'),
  };

  const markers = couriers.map(c => ({
    id: c.id,
    lngLat: c.lngLat,
    color: statusColors[c.status] || getCSSVar('--brand-text-muted'),
    label: c.initials,
  }));

  if (destinationPin) {
    markers.push({
      id: 'destination',
      lngLat: destinationPin,
      color: getCSSVar('--color-info'),
      label: '🏠',
    });
  }

  if (clientLocation) {
    markers.push({
      id: 'client-live',
      lngLat: clientLocation,
      color: getCSSVar('--color-success'),
      label: '📍',
    });
  }

  return (
    <MapLibreBase
      className={className}
      center={center}
      zoom={zoom}
      markers={markers}
      courier={courierMarker}
      routeLine={routeLine}
    >
      {couriers.length === 0 && !destinationPin && (
        <div className="absolute inset-0 flex items-center justify-center bg-[var(--brand-surface)]/50 z-10 pointer-events-none">
          <span className="text-sm text-[var(--brand-text-muted)]">{t('admin.no_couriers_online', 'No couriers online')}</span>
        </div>
      )}
    </MapLibreBase>
  );
}

export type { CourierOnMap };
