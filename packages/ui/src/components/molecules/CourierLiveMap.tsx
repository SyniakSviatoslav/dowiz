import { useState, useCallback } from 'react';
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
  // Internal readiness only — public prop API is unchanged. We feed MapLibreBase's
  // existing onMapReady signal so the soft skeleton clears exactly when tiles paint.
  const [mapReady, setMapReady] = useState(false);
  const handleMapReady = useCallback(() => setMapReady(true), []);
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

  // Nothing geolocatable to plot (no live courier, no fleet, no pins) → show a
  // composed, themed placeholder instead of a blank/grey tile box.
  const hasAnything =
    markers.length > 0 || Boolean(courierMarker) || (routeLine?.length ?? 0) >= 2;

  if (!hasAnything) {
    return (
      <div
        role="status"
        className={`relative flex flex-col items-center justify-center gap-2 px-6 text-center bg-brand-surface rounded-lg shadow-elevation-1 ${className}`}
      >
        <svg
          aria-hidden="true"
          viewBox="0 0 24 24"
          className="h-8 w-8 text-brand-text-muted"
          fill="none"
          stroke="currentColor"
          strokeWidth={1.75}
          strokeLinecap={`strokeLinecap`}
          strokeLinejoin={`strokeLinejoin`}
        >
          <path d={`d`} />
          <circle cx="12" cy="10" r="2.5" />
        </svg>
        <p className="text-sm text-brand-text">
          {t('map.unavailable', 'Live location unavailable')}
        </p>
        <p className="text-xs text-brand-text-muted">
          {t('map.unavailable_hint', 'No position to show yet — it’ll appear here once tracking starts.')}
        </p>
      </div>
    );
  }

  return (
    <MapLibreBase
      className={className}
      center={center}
      zoom={zoom}
      markers={markers}
      courier={courierMarker}
      routeLine={routeLine}
      onMapReady={handleMapReady}
    >
      {/* Soft-UI loading skeleton — fills the container with a tokened shimmer
          until the tiles paint (mapReady), reduced-motion aware. */}
      {!mapReady && (
        <div
          role="status"
          aria-busy="true"
          aria-label={t('map.loading', 'Loading map…')}
          className="skeleton absolute inset-0 z-30 flex items-center justify-center overflow-hidden bg-brand-surface rounded-lg shadow-elevation-1"
        >
          <div className="absolute inset-0 animate-pulse bg-brand-surface-raised motion-reduce:animate-none" />
          <span className="relative text-sm text-brand-text-muted">
            {t('map.loading', 'Loading map…')}
          </span>
        </div>
      )}
      {couriers.length === 0 && !destinationPin && (
        <div className="absolute inset-0 flex items-center justify-center bg-brand-surface/50 z-10 pointer-events-none">
          <span className="text-sm text-brand-text-muted">{t('admin.no_couriers_online', 'No couriers online')}</span>
        </div>
      )}
    </MapLibreBase>
  );
}

export type { CourierOnMap };
