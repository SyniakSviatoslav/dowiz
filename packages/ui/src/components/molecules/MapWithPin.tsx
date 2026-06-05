import { useState, useCallback } from 'react';
import { MapLibreBase, type LngLatLike } from './MapLibreBase.js';

interface MapWithPinProps {
  className?: string;
  initialCenter?: LngLatLike;
  initialPin?: LngLatLike;
  onPinChange?: (lngLat: LngLatLike) => void;
  confirmLabel?: string;
  placeholder?: string;
}

export function MapWithPin({
  className = 'h-64 w-full rounded-lg',
  initialCenter = [19.817, 41.331],
  initialPin,
  onPinChange,
  confirmLabel = 'Confirm location',
  placeholder = 'Tap the map to place your delivery pin',
}: MapWithPinProps) {
  const [pin, setPin] = useState<LngLatLike | null>(initialPin || null);
  const [confirmed, setConfirmed] = useState(false);

  const handleMapClick = useCallback((lngLat: LngLatLike) => {
    setPin(lngLat);
    setConfirmed(false);
    onPinChange?.(lngLat);
  }, [onPinChange]);

  const handleConfirm = () => {
    if (!pin) return;
    setConfirmed(true);
  };

  const markers = pin ? [{ lngLat: pin, color: confirmed ? '#059669' : '#ea4f16', label: confirmed ? '✓' : '📍' }] : [];

  return (
    <div className="relative">
      <MapLibreBase
        className={className}
        center={pin || initialCenter}
        zoom={pin ? 15 : 13}
        markers={markers}
        onClick={handleMapClick}
      />
      {!pin && (
        <div className="absolute top-3 left-1/2 -translate-x-1/2 bg-[var(--brand-surface)] border border-[var(--brand-border)] px-4 py-2 rounded-full text-sm text-[var(--brand-text-muted)] shadow-lg z-10">
          {placeholder}
        </div>
      )}
      {pin && (
        <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-10">
          <button
            onClick={handleConfirm}
            className={`px-6 py-2.5 rounded-full font-medium text-sm shadow-lg transition-all ${
              confirmed
                ? 'bg-green-600 text-white'
                : 'bg-[var(--brand-primary)] text-white hover:bg-[var(--brand-primary-hover)]'
            }`}
          >
            {confirmed ? <><i className="ti ti-check" /> Location confirmed</> : confirmLabel}
          </button>
        </div>
      )}
    </div>
  );
}
