import { MapLibreBase, type LngLatLike } from './MapLibreBase.js';

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
  destinationPin?: LngLatLike;
  routeLine?: LngLatLike[];
  center?: LngLatLike;
  zoom?: number;
}

export function CourierLiveMap({
  className = 'h-64 w-full rounded-lg',
  couriers = [],
  destinationPin,
  routeLine,
  center = [19.817, 41.331],
  zoom = 13,
}: CourierLiveMapProps) {
  const statusColors: Record<string, string> = {
    online: '#059669',
    busy: '#ea4f16',
    offline: '#6B7280',
  };

  const markers = couriers.map(c => ({
    id: c.id,
    lngLat: c.lngLat,
    color: statusColors[c.status] || '#6B7280',
    label: c.initials,
  }));

  if (destinationPin) {
    markers.push({
      id: 'destination',
      lngLat: destinationPin,
      color: '#2563EB',
      label: '🏠',
    });
  }

  return (
    <MapLibreBase
      className={className}
      center={center}
      zoom={zoom}
      markers={markers}
      routeLine={routeLine}
    >
      {couriers.length === 0 && !destinationPin && (
        <div className="absolute inset-0 flex items-center justify-center bg-[var(--brand-surface)]/50 z-10 pointer-events-none">
          <span className="text-sm text-[var(--brand-text-muted)]">No couriers online</span>
        </div>
      )}
    </MapLibreBase>
  );
}

export type { CourierOnMap };
