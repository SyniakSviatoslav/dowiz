import { useRef, useEffect, useState, useCallback, type ReactNode } from 'react';
import { useI18n } from '../../lib/I18nProvider.js';

function getCSSVar(name: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

type LngLatLike = [number, number];

// TileSource seam (G3): the map style comes from VITE_TILE_STYLE_URL, never a
// hardcoded provider URL. packages/ui can't import apps/web's tileConfig, so the
// env var is read here directly; the fallback equals tileConfig's documented
// default (openfreemap), so behavior is unchanged until the env is set. Swapping to
// a self-hosted tileserver-gl / Protomaps in `fra` is one env change — see
// docs/adr/ADR-GEO-SEAMS.md.
const TILE_STYLE_URL: string =
  (typeof import.meta !== 'undefined' && (import.meta as any).env?.VITE_TILE_STYLE_URL) ||
  'https://tiles.openfreemap.org/styles/liberty';

interface MapLibreBaseProps {
  center?: LngLatLike;
  zoom?: number;
  className?: string;
  children?: ReactNode;
  markers?: Array<{ lngLat: LngLatLike; color?: string; label?: string; id?: string }>;
  /** Single persistent, imperatively-updated marker (live courier) — moved via
   *  setLngLat + rotated by bearing each frame, never recreated. */
  courier?: { lngLat: LngLatLike; bearing?: number } | null;
  routeLine?: LngLatLike[];
  radiusCircle?: { center: LngLatLike; radiusKm: number };
  onClick?: (lngLat: LngLatLike) => void;
  onMapReady?: (map: unknown) => void;
  interactive?: boolean;
}

export function MapLibreBase({
  center = [19.817, 41.331],
  zoom = 13,
  className = '',
  children,
  markers = [],
  courier,
  routeLine,
  radiusCircle,
  onClick,
  onMapReady,
  interactive = true,
}: MapLibreBaseProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<any>(null);
  const maplibreRef = useRef<any>(null);
  const markerRefs = useRef<any[]>([]);
  const courierMarkerRef = useRef<any>(null);
  const { t } = useI18n();
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const clearMarkers = useCallback(() => {
    markerRefs.current.forEach(m => m?.remove());
    markerRefs.current = [];
  }, []);

  useEffect(() => {
    let cancelled = false;
    const container = containerRef.current;
    if (!container) return;

    async function initMap() {
      try {
        const maplibregl = await import('maplibre-gl');
        if (cancelled || !containerRef.current) return;
        maplibreRef.current = maplibregl;

        const map = new maplibregl.Map({
          container: containerRef.current,
          style: TILE_STYLE_URL,
          center,
          zoom,
          interactive,
        });

        map.addControl(new maplibregl.NavigationControl(), 'top-right');
        mapRef.current = map;

        if (onClick) {
          map.on('click', (e: any) => {
            const { lng, lat } = e.lngLat;
            onClick([lng, lat]);
          });
        }

        map.on('load', () => {
          if (cancelled) return;
          setLoaded(true);
          onMapReady?.(map);
        });
      } catch {
        if (!cancelled) {
          setError('MapLibre GL not available. Install maplibre-gl to enable maps.');
        }
      }
    }

    initMap();

    return () => {
      cancelled = true;
      clearMarkers();
      mapRef.current?.remove();
    };
  }, []);

  // Update markers
  useEffect(() => {
    if (!loaded || !mapRef.current) return;

    async function updateMarkers() {
      const maplibregl = await import('maplibre-gl');
      clearMarkers();

      markers.forEach(m => {
        const el = document.createElement('div');
        const color = m.color || getCSSVar('--brand-primary', '#ea4f16');
        const label = m.label || '';
        el.style.width = '28px';
        el.style.height = '28px';
        el.style.background = color;
        el.style.border = '3px solid white';
        el.style.borderRadius = '50%';
        el.style.boxShadow = '0 2px 8px rgba(0,0,0,0.3)';
        el.style.display = 'flex';
        el.style.alignItems = 'center';
        el.style.justifyContent = 'center';
        el.style.fontSize = '10px';
        el.style.fontWeight = 'bold';
        el.style.color = 'white';
        el.style.transform = 'translate(-50%, -50%)';
        el.textContent = label;
        // Stable hooks for tests/automation (e.g. asserting live courier pins).
        el.dataset.testid = 'map-marker';
        if (m.id) el.dataset.markerId = m.id;

        const marker = new maplibregl.Marker({ element: el })
          .setLngLat(m.lngLat)
          .addTo(mapRef.current);

        markerRefs.current.push(marker);
      });
    }

    updateMarkers();
  }, [markers, loaded, clearMarkers]);

  // Live courier marker — created once, then moved/rotated imperatively each frame
  // (no DOM recreation, so it's cheap to drive at animation framerate).
  useEffect(() => {
    const maplibregl = maplibreRef.current;
    if (!loaded || !mapRef.current || !maplibregl) return;

    if (!courier) {
      courierMarkerRef.current?.remove();
      courierMarkerRef.current = null;
      return;
    }

    if (!courierMarkerRef.current) {
      const el = document.createElement('div');
      el.style.width = '34px';
      el.style.height = '34px';
      el.style.transform = 'translate(-50%, -50%)';
      el.style.display = 'flex';
      el.style.alignItems = 'center';
      el.style.justifyContent = 'center';
      const dot = document.createElement('div');
      dot.style.width = '18px';
      dot.style.height = '18px';
      dot.style.borderRadius = '50%';
      dot.style.background = getCSSVar('--brand-primary', '#ea4f16');
      dot.style.border = '3px solid white';
      dot.style.boxShadow = '0 2px 8px rgba(0,0,0,0.3)';
      const arrow = document.createElement('div');
      arrow.className = 'dos-courier-arrow';
      arrow.style.position = 'absolute';
      arrow.style.top = '-2px';
      arrow.style.width = '0';
      arrow.style.height = '0';
      arrow.style.borderLeft = '5px solid transparent';
      arrow.style.borderRight = '5px solid transparent';
      arrow.style.borderBottom = `8px solid ${getCSSVar('--brand-primary', '#ea4f16')}`;
      el.style.position = 'relative';
      el.appendChild(dot);
      el.appendChild(arrow);
      courierMarkerRef.current = new maplibregl.Marker({ element: el })
        .setLngLat(courier.lngLat)
        .addTo(mapRef.current);
    } else {
      courierMarkerRef.current.setLngLat(courier.lngLat);
    }

    const arrowEl = courierMarkerRef.current.getElement()?.querySelector('.dos-courier-arrow') as HTMLElement | null;
    if (arrowEl) {
      // Rotate the whole marker element around its centre so the arrow points along travel.
      const markerEl = courierMarkerRef.current.getElement() as HTMLElement;
      markerEl.style.transformOrigin = 'center';
      markerEl.style.transform = `translate(-50%, -50%) rotate(${courier.bearing ?? 0}deg)`;
    }
  }, [courier, loaded]);

  // Update route line
  useEffect(() => {
    if (!loaded || !mapRef.current || !routeLine || routeLine.length < 2) return;

    async function updateRoute() {
      const maplibregl = await import('maplibre-gl');

      const sourceId = 'route-line';
      if (mapRef.current.getSource(sourceId)) {
        mapRef.current.getSource(sourceId).setData({
          type: 'Feature',
          properties: {},
          geometry: { type: 'LineString', coordinates: routeLine },
        });
        return;
      }

      mapRef.current.addSource(sourceId, {
        type: 'geojson',
        data: {
          type: 'Feature',
          properties: {},
          geometry: { type: 'LineString', coordinates: routeLine },
        },
      });

      mapRef.current.addLayer({
        id: sourceId,
        type: 'line',
        source: sourceId,
        layout: { 'line-join': 'round', 'line-cap': 'round' },
        paint: { 'line-color': getCSSVar('--brand-primary', '#ea4f16'), 'line-width': 4, 'line-opacity': 0.8 },
      });
    }

    updateRoute();
  }, [routeLine, loaded]);

  // Update radius circle
  useEffect(() => {
    if (!loaded || !mapRef.current || !radiusCircle) return;

    async function updateRadius() {
      const maplibregl = await import('maplibre-gl');
      const sourceId = 'radius-circle';
      const [clng, clat] = radiusCircle!.center;
      const radiusKm = radiusCircle!.radiusKm;
      const points = 64;
      const coords: LngLatLike[] = [];

      for (let i = 0; i <= points; i++) {
        const angle = (i / points) * 2 * Math.PI;
        const lat = clat + (radiusKm / 111.32) * Math.cos(angle);
        const lng = clng + (radiusKm / (111.32 * Math.cos(clat * Math.PI / 180))) * Math.sin(angle);
        coords.push([lng, lat]);
      }

      const data = {
        type: 'Feature' as const,
        properties: {},
        geometry: { type: 'Polygon' as const, coordinates: [coords] },
      };

      if (mapRef.current.getSource(sourceId)) {
        mapRef.current.getSource(sourceId).setData(data);
        return;
      }

      mapRef.current.addSource(sourceId, { type: 'geojson', data });

      mapRef.current.addLayer({
        id: sourceId,
        type: 'fill',
        source: sourceId,
        paint: { 'fill-color': getCSSVar('--brand-primary', '#ea4f16'), 'fill-opacity': 0.15 },
      });

      mapRef.current.addLayer({
        id: `${sourceId}-outline`,
        type: 'line',
        source: sourceId,
        paint: { 'line-color': getCSSVar('--brand-primary', '#ea4f16'), 'line-width': 2, 'line-opacity': 0.6 },
      });
    }

    updateRadius();
  }, [radiusCircle, loaded]);

  useEffect(() => {
    if (!loaded || !mapRef.current) return;
    mapRef.current.flyTo({ center, zoom, duration: 800 });
  }, [center[0], center[1], zoom, loaded]);

  if (error) {
    return (
      <div className={`flex items-center justify-center bg-[var(--brand-surface)] text-[var(--brand-text-muted)] text-sm rounded-lg ${className}`}>
        {error}
      </div>
    );
  }

  return (
    // data-dynamic: live map tiles + courier marker positions vary run-to-run —
    // masked from the visual-regression net (all maps render through this base).
    <div ref={containerRef} data-testid="map-container" data-dynamic className={`relative ${className}`}>
      {!loaded && (
        <div className="absolute inset-0 flex items-center justify-center bg-[var(--brand-surface)] text-[var(--brand-text-muted)] text-sm z-20">
          {t('map.loading', 'Loading map…')}
        </div>
      )}
      {children}
    </div>
  );
}

export type { LngLatLike };
