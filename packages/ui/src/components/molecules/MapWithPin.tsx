import { useState, useCallback } from 'react';
import { MapLibreBase, type LngLatLike } from './MapLibreBase.js';

function getCSSVar(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

interface MapWithPinProps {
  className?: string;
  initialCenter?: LngLatLike;
  initialPin?: LngLatLike;
  onPinChange?: (lngLat: LngLatLike) => void;
  confirmLabel?: string;
  placeholder?: string;
}

function getCurrentPosition(): Promise<GeolocationPosition> {
  return new Promise((resolve, reject) => {
    if (typeof navigator === 'undefined' || !navigator.geolocation) {
      reject(new Error('Geolocation not supported'));
      return;
    }
    navigator.geolocation.getCurrentPosition(resolve, reject, {
      enableHighAccuracy: true,
      timeout: 10000,
      maximumAge: 30000,
    });
  });
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
  const [locating, setLocating] = useState(false);

  const handleMapClick = useCallback((lngLat: LngLatLike) => {
    setPin(lngLat);
    setConfirmed(false);
    onPinChange?.(lngLat);
  }, [onPinChange]);

  const handleConfirm = () => {
    if (!pin) return;
    setConfirmed(true);
  };

  const handleMyLocation = useCallback(async () => {
    setLocating(true);
    try {
      const pos = await getCurrentPosition();
      const lngLat: LngLatLike = [pos.coords.longitude, pos.coords.latitude];
      setPin(lngLat);
      setConfirmed(false);
      onPinChange?.(lngLat);
    } catch {
      // Permission denied or unavailable — silently ignore
    } finally {
      setLocating(false);
    }
  }, [onPinChange]);

  const markers = pin ? [{ lngLat: pin, color: confirmed ? getCSSVar('--color-success', '#059669') : getCSSVar('--brand-primary', '#ea4f16'), label: confirmed ? '✓' : '📍' }] : [];

  return (
    <div className="relative">
      <MapLibreBase
        className={className}
        center={pin || initialCenter}
        zoom={pin ? 15 : 13}
        markers={markers}
        onClick={handleMapClick}
      />
      <button
        onClick={handleMyLocation}
        disabled={locating}
        className="absolute top-3 right-3 w-10 h-10 rounded-full flex items-center justify-center shadow-lg border z-10 transition-all active:scale-95 disabled:opacity-50"
        style={{
          background: 'var(--brand-surface)',
          borderColor: 'var(--brand-border)',
          color: 'var(--brand-primary)',
        }}
        title="My Location"
      >
        {locating ? (
          <i className="ti ti-loader animate-spin text-base" />
        ) : (
          <i className="ti ti-crosshair text-base" />
        )}
      </button>
      {!pin && (
        <div className="absolute top-3 left-1/2 -translate-x-1/2 bg-[var(--brand-surface)] border border-[var(--brand-border)] px-4 py-2 rounded-full text-sm text-[var(--brand-text-muted)] shadow-lg z-10 pointer-events-none">
          {placeholder}
        </div>
      )}
      {pin && (
        <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-10">
          <button
            onClick={handleConfirm}
            className={`px-6 py-2.5 rounded-full font-medium text-sm shadow-lg transition-all ${
              confirmed
                ? 'bg-[var(--color-success)] text-[var(--color-on-success)]'
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
