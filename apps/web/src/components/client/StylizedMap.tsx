// A decorative, brand-tinted "map" graphic with a vendor pin — NOT a live map tile (no API key, no cost).
// Used as the footer's map panel and as the storefront hero background when the venue has no photo.
// Pure SVG + brand CSS variables. aria-hidden (decorative); the surrounding link carries the label.
export function StylizedMap({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 320 180" className={className ?? 'w-full h-full'} preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      <rect width="320" height="180" fill="var(--brand-surface)" />
      <g stroke="var(--brand-border)" strokeWidth="6" opacity="0.9">
        <path d="M-10 60 L330 40" /><path d="M-10 130 L330 150" />
        <path d="M70 -10 L50 190" /><path d="M210 -10 L230 190" />
      </g>
      <g stroke="var(--brand-border)" strokeWidth="2" opacity="0.55">
        <path d="M-10 95 L330 92" /><path d="M140 -10 L150 190" />
      </g>
      <g fill="color-mix(in srgb, var(--brand-primary) 9%, var(--brand-surface))">
        <rect x="86" y="48" width="50" height="34" rx="3" /><rect x="244" y="60" width="46" height="40" rx="3" />
        <rect x="92" y="150" width="60" height="34" rx="3" /><rect x="246" y="150" width="50" height="30" rx="3" />
      </g>
      {/* Vendor pin */}
      <g transform="translate(160 86)">
        <ellipse cx="0" cy="30" rx="14" ry="4" fill="color-mix(in srgb, var(--brand-primary) 30%, transparent)" />
        <path d="M0 28 C-13 8 -14 -2 -14 -8 A14 14 0 1 1 14 -8 C14 -2 13 8 0 28 Z" fill="var(--brand-primary)" />
        <circle cx="0" cy="-8" r="5.5" fill="var(--brand-bg)" />
      </g>
    </svg>
  );
}
