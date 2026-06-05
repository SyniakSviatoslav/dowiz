import { useState, useCallback } from 'react';
import { MapLibreBase, type LngLatLike } from './MapLibreBase.js';

interface MapWithRadiusProps {
  className?: string;
  initialCenter?: LngLatLike;
  initialRadiusKm?: number;
  onRadiusChange?: (center: LngLatLike, radiusKm: number) => void;
  minRadius?: number;
  maxRadius?: number;
}

export function MapWithRadius({
  className = 'h-80 w-full rounded-lg',
  initialCenter = [19.817, 41.331],
  initialRadiusKm = 3,
  onRadiusChange,
  minRadius = 0.5,
  maxRadius = 15,
}: MapWithRadiusProps) {
  const [center, setCenter] = useState<LngLatLike>(initialCenter);
  const [radiusKm, setRadiusKm] = useState(initialRadiusKm);

  const handleMapClick = useCallback((lngLat: LngLatLike) => {
    setCenter(lngLat);
    onRadiusChange?.(lngLat, radiusKm);
  }, [radiusKm, onRadiusChange]);

  const handleRadiusChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newRadius = Number(e.target.value);
    setRadiusKm(newRadius);
    onRadiusChange?.(center, newRadius);
  }, [center, onRadiusChange]);

  const markers = [{ lngLat: center, color: '#ea4f16', label: '📍' }];

  return (
    <div className="flex flex-col gap-3">
      <MapLibreBase
        className={className}
        center={center}
        zoom={12}
        markers={markers}
        radiusCircle={{ center, radiusKm }}
        onClick={handleMapClick}
      />
      <div className="flex items-center gap-3 px-2">
        <span className="text-sm text-[var(--brand-text-muted)]">Delivery radius:</span>
        <input
          type="range"
          min={minRadius}
          max={maxRadius}
          step={0.5}
          value={radiusKm}
          onChange={handleRadiusChange}
          className="flex-1 h-2 rounded-full appearance-none cursor-pointer"
          style={{
            background: `linear-gradient(to right, var(--brand-primary) ${((radiusKm - minRadius) / (maxRadius - minRadius)) * 100}%, var(--brand-border) ${((radiusKm - minRadius) / (maxRadius - minRadius)) * 100}%)`,
            accentColor: 'var(--brand-primary)',
          }}
        />
        <span className="text-sm font-bold text-[var(--brand-text)] min-w-[60px] text-right">{radiusKm} km</span>
      </div>
    </div>
  );
}
