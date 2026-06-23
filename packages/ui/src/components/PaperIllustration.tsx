import React from 'react';

// PAPER / MOEBIUS line-art illustrations — the "drawn by a master" soul of the internal
// skin. Pure inline SVG (no bitmap, no dep): continuous ink contours + flat palette fills,
// so they read as hand-inked and inherit the paper tokens (light + aged-paper dark) for
// free. Used in empty states, onboarding, login, and as the reduced-motion / low-end
// fallback for the animated "moment". Internal surfaces only — callers gate on the skin.
//
// Drawing language: stroke = var(--ink); fills pull from --sand / --teal(-deep) / --gold at
// low alpha so colour stays decorative and never sits as a bg under body text.

export type PaperIllustrationName = 'island' | 'sunrise' | 'parcel' | 'scooter';

export interface PaperIllustrationProps {
  name?: PaperIllustrationName;
  /** Gently animates the hero elements (sun rise, cloud drift). Honours reduced-motion. */
  animated?: boolean;
  className?: string;
  /** Accessible label; omit (or '') to mark the art decorative (aria-hidden). */
  title?: string;
  style?: React.CSSProperties;
}

const STROKE = {
  fill: 'none',
  stroke: 'var(--ink, #241F1A)',
  strokeWidth: 3,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
};

function Island({ animated }: { animated?: boolean }) {
  return (
    <>
      <circle cx="176" cy="50" r="22" fill="var(--gold, #ECD06F)" opacity="0.85"
        className={animated ? 'paper-rise' : undefined} />
      <path d="M16 120 q34 -54 72 -2 q30 44 64 4 q26 -30 56 8" fill="var(--teal, #49C5B6)" opacity="0.18" />
      {/* dune + lone palm — one continuous ink contour */}
      <path d="M8 124 q40 -44 80 -8 q34 34 68 -2 q28 -28 60 6" {...STROKE} />
      <path d="M150 122 q-2 -34 6 -52" {...STROKE} />
      <path d="M156 70 q-22 -10 -34 2 M156 70 q22 -10 36 4 M156 72 q-12 16 -26 18 M156 72 q14 14 30 14" {...STROKE} strokeWidth={2.5} />
      <path d="M40 124 q24 -16 56 0" {...STROKE} strokeWidth={2} opacity="0.5" />
    </>
  );
}

function Sunrise({ animated }: { animated?: boolean }) {
  return (
    <>
      <circle cx="120" cy="92" r="30" fill="var(--gold, #ECD06F)" opacity="0.9"
        className={animated ? 'paper-rise' : undefined} />
      {[0, 1, 2, 3, 4].map((i) => (
        <path key={i} d={`M120 92 L${120 + 56 * Math.cos((-Math.PI / 6) * (i - 2) - Math.PI / 2)} ${92 + 56 * Math.sin((-Math.PI / 6) * (i - 2) - Math.PI / 2)}`}
          {...STROKE} strokeWidth={2.5} opacity="0.55" />
      ))}
      <path d="M14 116 q40 -10 70 2 q40 14 76 -2 q30 -10 60 0" fill="var(--sand, #987654)" opacity="0.16" />
      <path d="M10 118 q44 -14 76 0 q42 16 80 -2 q28 -10 56 2" {...STROKE} />
      <path d="M30 138 q60 -12 110 0 q40 8 70 -2" {...STROKE} strokeWidth={2} opacity="0.45" />
    </>
  );
}

function Parcel() {
  return (
    <>
      <path d="M60 64 L120 40 L180 64 L120 88 Z" fill="var(--sand, #987654)" opacity="0.16" />
      <path d="M60 64 L120 88 L120 134 L60 110 Z" fill="var(--teal-deep, #3EA094)" opacity="0.14" />
      <path d="M60 64 L120 40 L180 64 L180 110 L120 134 L60 110 Z" {...STROKE} />
      <path d="M120 88 L120 134 M60 64 L120 88 L180 64" {...STROKE} />
      {/* twine */}
      <path d="M90 52 L150 122 M150 52 L90 122" {...STROKE} strokeWidth={2.5} opacity="0.7" />
      <path d="M112 70 q8 -12 16 0 q-8 6 -16 0" {...STROKE} strokeWidth={2.5} />
    </>
  );
}

function Scooter() {
  return (
    <>
      <circle cx="64" cy="118" r="20" fill="var(--teal, #49C5B6)" opacity="0.16" />
      <circle cx="176" cy="118" r="20" fill="var(--teal, #49C5B6)" opacity="0.16" />
      <circle cx="64" cy="118" r="20" {...STROKE} />
      <circle cx="176" cy="118" r="20" {...STROKE} />
      <path d="M64 118 L104 118 L120 78 L150 78" {...STROKE} />
      <path d="M150 78 q18 0 26 40" {...STROKE} />
      <path d="M120 78 L112 64 L98 64" {...STROKE} strokeWidth={2.5} />
      <path d="M104 118 q22 -40 46 -40" fill="var(--gold, #ECD06F)" opacity="0.5" />
      <path d="M150 40 l16 0 l-4 16 l-14 0 z" fill="var(--sand, #987654)" opacity="0.5" />
      <path d="M150 40 l16 0 l-4 16 l-14 0 z" {...STROKE} strokeWidth={2.5} />
    </>
  );
}

const ART: Record<PaperIllustrationName, (p: { animated?: boolean }) => React.ReactElement> = {
  island: Island,
  sunrise: Sunrise,
  parcel: Parcel,
  scooter: Scooter,
};

export function PaperIllustration({
  name = 'island',
  animated = false,
  className,
  title,
  style,
}: PaperIllustrationProps) {
  const Art = ART[name] ?? Island;
  const decorative = !title;
  return (
    <svg
      viewBox="0 0 240 160"
      role={decorative ? 'presentation' : 'img'}
      aria-hidden={decorative || undefined}
      aria-label={decorative ? undefined : title}
      className={className}
      style={{ width: '100%', height: 'auto', maxWidth: 240, ...style }}
    >
      {!decorative && <title>{title}</title>}
      <Art animated={animated} />
    </svg>
  );
}
