// Real satellite snapshot of a venue, zoomed to its coordinates, with a pin — no API key, no billing.
// Source: Esri World Imagery static export (ArcGIS REST), free with attribution. Used in the footer.
// The pin is overlaid in the DOM (the export has no marker support). object-cover crops to fill the panel.
export function SatelliteMap({ lat, lng, className }: { lat: number; lng: number; className?: string }) {
  // ~250 m box around the point → a close, legible zoom on the building.
  const d = 0.0016;
  const bbox = `${lng - d},${lat - d},${lng + d},${lat + d}`;
  const url =
    `https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/export` +
    `?bbox=${bbox}&bboxSR=4326&imageSR=3857&size=640,400&format=jpg&f=image`;
  return (
    <div className={`relative overflow-hidden ${className ?? ''}`}>
      <img src={url} alt="" aria-hidden="true" className="w-full h-full object-cover" loading="lazy" />
      {/* Centered vendor pin (tip at the exact center of the box). */}
      <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-full" aria-hidden="true">
        <svg width="30" height="38" viewBox="0 0 30 38">
          <ellipse cx="15" cy="35" rx="6" ry="2" fill="rgba(0,0,0,0.35)" />
          <path d="M15 34 C6 21 2 15 2 11 A13 13 0 1 1 28 11 C28 15 24 21 15 34 Z" fill="var(--brand-primary)" stroke="rgba(255,255,255,0.95)" strokeWidth="1.6" />
          <circle cx="15" cy="11" r="5" fill="rgba(255,255,255,0.97)" />
        </svg>
      </div>
      {/* Required Esri attribution. */}
      <span
        className="absolute bottom-0.5 right-1 px-1 rounded"
        style={{ fontSize: '9px', lineHeight: '14px', background: 'rgba(0,0,0,0.45)', color: 'rgba(255,255,255,0.92)' }}
      >
        © Esri
      </span>
    </div>
  );
}
