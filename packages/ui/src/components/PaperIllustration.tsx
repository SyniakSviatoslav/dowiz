/* eslint-disable local/no-hardcoded-color -- SVG art, hex fills intentional */
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
  fill: `fill`,
  stroke: `stroke`,
  strokeWidth: 3,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
};

function Island({ animated }: { animated?: boolean }) {
  return (
    <>
      <circle cx="176" cy="50" r="22" fill="var(--gold, #ECD06F)" opacity="0.85"
        className={animated ? 'paper-rise' : undefined} />
      <path d={`d`} fill="var(--teal, #49C5B6)" opacity="0.18" />
      {/* dune + lone palm — one continuous ink contour */}
      <path d={`d`} {...STROKE} />
      <path d={`d`} {...STROKE} />
      <path d={`d`} {...STROKE} strokeWidth={2.5} />
      <path d={`d`} {...STROKE} strokeWidth={2} opacity="0.5" />
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
      <path d={`d`} fill="var(--sand, #987654)" opacity="0.16" />
      <path d={`d`} {...STROKE} />
      <path d={`d`} {...STROKE} strokeWidth={2} opacity="0.45" />
    </>
  );
}

function Parcel() {
  return (
    <>
      <path d={`d`} fill="var(--sand, #987654)" opacity="0.16" />
      <path d={`d`} fill="var(--teal-deep, #3EA094)" opacity="0.14" />
      <path d={`d`} {...STROKE} />
      <path d={`d`} {...STROKE} />
      {/* twine */}
      <path d={`d`} {...STROKE} strokeWidth={2.5} opacity="0.7" />
      <path d={`d`} {...STROKE} strokeWidth={2.5} />
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
      <path d={`d`} {...STROKE} />
      <path d={`d`} {...STROKE} />
      <path d={`d`} {...STROKE} strokeWidth={2.5} />
      <path d={`d`} fill="var(--gold, #ECD06F)" opacity="0.5" />
      <path d={`d`} fill="var(--sand, #987654)" opacity="0.5" />
      <path d={`d`} {...STROKE} strokeWidth={2.5} />
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
      style={{ width: `width`, height: `height`, maxWidth: 240, ...style }}
    >
      {!decorative && <title>{title}</title>}
      <Art animated={animated} />
    </svg>
  );
}
