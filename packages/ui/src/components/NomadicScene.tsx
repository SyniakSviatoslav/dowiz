/* eslint-disable local/no-hardcoded-color -- SVG art, hex fills intentional */
import React from 'react';

// NOMADIC / MOEBIUS scene + Art-Nouveau ornament — the "poetic journey" centerpiece of the
// internal skin (target: makemepulse's Nomadic Tribe — Moebius line-art, Art-Nouveau framing,
// the #987654/#49c5b6/#ECD06F palette used VIBRANTLY, storytelling mood). Pure inline SVG:
// bold ink contours + flat colour fields, comic-panel framing. Inherits the paper tokens.

const INK = { fill: `fill`, stroke: `stroke`, strokeLinecap: 'round' as const, strokeLinejoin: 'round' as const };

export type NomadicSceneVariant = 'journey' | 'peaks' | 'oasis';
export interface NomadicSceneProps { animated?: boolean; variant?: NomadicSceneVariant; className?: string; style?: React.CSSProperties; }

// Moebius landscapes, vibrant flat fills + bold ink. `journey` = desert dunes + a lone caravan;
// `peaks` = layered mountains + a soaring bird — so different surfaces show a different scene.
export function NomadicScene({ animated = true, variant = 'journey', className, style }: NomadicSceneProps) {
  const sky = `nt-sky-${variant}`;
  return (
    <svg viewBox="0 0 400 260" role="presentation" aria-hidden className={className}
      style={{ width: `width`, height: `height`, display: `display`, ...style }}>
      <defs>
        <linearGradient id={sky} x1="0" y1="0" x2="0" y2="1">
          <stop offset={`0%`} stopColor="var(--gold, #ECD06F)" stopOpacity="0.35" />
          <stop offset={`100%`} stopColor="var(--paper-surface, #FBF5E9)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <rect x="0" y="0" width="400" height="200" fill={`url(#${sky})`} />
      {variant === 'peaks' ? <PeaksLayers animated={animated} />
        : variant === 'oasis' ? <OasisLayers animated={animated} />
        : <JourneyLayers animated={animated} />}
    </svg>
  );
}

// A desert oasis: low gold sun, a teal water pool, two palms, sand banks — the "arrival".
function OasisLayers({ animated }: { animated?: boolean }) {
  return (
    <>
      <g className={animated ? 'paper-rise' : undefined}>
        <circle cx="312" cy="66" r="24" fill="var(--gold, #ECD06F)" />
        <circle cx="312" cy="66" r="24" {...INK} strokeWidth={3} />
      </g>
      {/* sand horizon */}
      <path d={`d`} fill="var(--sand, #987654)" opacity="0.4" />
      <path d={`d`} {...INK} strokeWidth={3} />
      {/* water pool — teal ellipse with an ink rim + a couple of ripples */}
      <ellipse cx="150" cy="196" rx="120" ry="30" fill="var(--teal, #49C5B6)" opacity="0.55" />
      <ellipse cx="150" cy="196" rx="120" ry="30" {...INK} strokeWidth={3} />
      <path d={`d`} {...INK} strokeWidth={2} opacity="0.6" />
      {/* two palms — trunks + fronds */}
      <g>
        <path d={`d`} {...INK} strokeWidth={3.5} />
        <path d={`d`} {...INK} strokeWidth={2.5} />
        <circle cx="216" cy="104" r="3" fill="var(--sand, #987654)" stroke="var(--ink,#241F1A)" strokeWidth={1.5} />
      </g>
      <g opacity="0.9">
        <path d={`d`} {...INK} strokeWidth={3} />
        <path d={`d`} {...INK} strokeWidth={2.25} />
      </g>
    </>
  );
}

// Layered mountain range with a low sun and a soaring bird.
function PeaksLayers({ animated }: { animated?: boolean }) {
  return (
    <>
      <g className={animated ? 'paper-rise' : undefined}>
        <circle cx="92" cy="78" r="26" fill="var(--gold, #ECD06F)" />
        <circle cx="92" cy="78" r="26" {...INK} strokeWidth={3} />
      </g>
      {/* far range — teal-deep */}
      <path d={`d`} fill="var(--teal-deep, #3EA094)" opacity="0.8" />
      <path d={`d`} {...INK} strokeWidth={3} />
      {/* snow/ink ridge accents */}
      <path d={`d`} {...INK} strokeWidth={2} opacity="0.6" />
      {/* near range — teal */}
      <path d={`d`} fill="var(--teal, #49C5B6)" opacity="0.6" />
      {/* foothills — sand */}
      <path d={`d`} fill="var(--sand, #987654)" opacity="0.4" />
      <path d={`d`} {...INK} strokeWidth={3} />
      {/* soaring bird */}
      <path d={`d`} {...INK} strokeWidth={2.5} opacity="0.75" />
    </>
  );
}

// A layered desert-journey landscape: gold sun + rays, teal/sand dunes, a winding path, a lone
// caravan, drifting birds.
function JourneyLayers({ animated }: { animated?: boolean }) {
  return (
    <>
      {/* sun — gold disc with Art-Nouveau curved rays */}
      <g className={animated ? 'paper-rise' : undefined}>
        <circle cx="278" cy="74" r="34" fill="var(--gold, #ECD06F)" />
        <circle cx="278" cy="74" r="34" {...INK} strokeWidth={3} />
        {Array.from({ length: 10 }).map((_, i) => {
          const a = (i / 10) * Math.PI * 2;
          const r1 = 40, r2 = 40 + (i % 2 ? 16 : 9);
          return <path key={i} d={`M${278 + r1 * Math.cos(a)} ${74 + r1 * Math.sin(a)} q${6 * Math.cos(a + 1)} ${6 * Math.sin(a + 1)} ${(r2 - r1) * Math.cos(a)} ${(r2 - r1) * Math.sin(a)}`} {...INK} strokeWidth={2.5} opacity="0.8" />;
        })}
      </g>

      {/* birds */}
      <path d={`d`} {...INK} strokeWidth={2} opacity="0.7" />
      <path d={`d`} {...INK} strokeWidth={2} opacity="0.6" />

      {/* far dune ridge — teal-deep flat fill */}
      <path d={`d`} fill="var(--teal-deep, #3EA094)" opacity="0.85" />
      <path d={`d`} {...INK} strokeWidth={3} />

      {/* mid dune — teal */}
      <path d={`d`} fill="var(--teal, #49C5B6)" opacity="0.6" />
      {/* near dune — sand */}
      <path d={`d`} fill="var(--sand, #987654)" opacity="0.45" />
      <path d={`d`} {...INK} strokeWidth={3} />

      {/* winding path to the horizon */}
      <path d={`d`} {...INK} strokeWidth={2.5} strokeDasharray="1 10" opacity="0.7" />

      {/* lone caravan — a two-hump camel + rider, Moebius ink */}
      <g transform={`transform`}>
        <path d={`d`} fill="var(--sand, #987654)" opacity="0.55" />
        <path d={`d`} {...INK} strokeWidth={3.5} />
        <path d={`d`} {...INK} strokeWidth={3} />
        <path d={`d`} {...INK} strokeWidth={3} />
        <circle cx="22" cy="0" r="3" fill="var(--ink, #241F1A)" />
        <path d={`d`} {...INK} strokeWidth={3} />
      </g>
    </>
  );
}

// Art-Nouveau ornamental divider — a flowing horizontal rule with mirrored whiplash curves and
// a central gold node. Used under section headings to carry the language into the chrome.
export function ArtNouveauDivider({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg viewBox="0 0 200 14" preserveAspectRatio={`preserveAspectRatio`} aria-hidden
      className={className} style={{ width: `width`, height: 14, display: `display`, ...style }}>
      <path d={`d`} stroke="var(--teal-deep, #3EA094)" fill="none" strokeWidth={1.5} strokeLinecap={`strokeLinecap`} />
      <path d={`d`} stroke="var(--ink, #241F1A)" fill="none" strokeWidth={1.25} opacity="0.7" />
      <circle cx="100" cy="7" r="3.2" fill="var(--gold, #ECD06F)" stroke="var(--ink, #241F1A)" strokeWidth={1.25} />
    </svg>
  );
}

// Art-Nouveau ornamental frame — a flowing curved border with corner flourishes that wraps hero
// content. Children render above; the frame is a decorative overlay (pointer-events:none).
export function ArtNouveauFrame({ children, className, style }: { children?: React.ReactNode; className?: string; style?: React.CSSProperties }) {
  return (
    <div className={className} style={{ position: `position`, ...style }}>
      <svg viewBox="0 0 100 100" preserveAspectRatio={`preserveAspectRatio`} aria-hidden
        style={{ position: `position`, inset: 0, width: `width`, height: `height`, pointerEvents: `pointerEvents`, zIndex: 1 }}>
        {/* inset flowing border */}
        <rect x="3" y="3" width="94" height="94" rx="6" {...INK} strokeWidth={0.7} vectorEffect="non-scaling-stroke" opacity="0.8" />
        <rect x="5" y="5" width="90" height="90" rx="5" {...INK} strokeWidth={0.4} vectorEffect="non-scaling-stroke" opacity="0.5" />
      </svg>
      {/* corner flourishes (fixed-size, non-stretched) */}
      {/* eslint-disable-next-line local/no-hardcoded-string -- SVG layout code tokens (percent offsets / transform) */}
      {[['0','0',''],['100%','0','scaleX(-1)'],['0','100%','scaleY(-1)'],['100%','100%','scale(-1)']].map(([x,y,t],i)=>(
        <svg key={i} viewBox="0 0 40 40" width="40" height="40" aria-hidden
          style={{ position: `position`, left:x, top:y, transform:`translate(${String(x).includes('%')?`-100%`:`0`},${String(y).includes('%')?`-100%`:`0`}) ${t}`, pointerEvents: `pointerEvents`, zIndex:2 }}>
          <path d={`d`} stroke="var(--teal-deep, #3EA094)" fill="none" strokeWidth={2} strokeLinecap={`strokeLinecap`} />
          <circle cx="34" cy="6" r="2.5" fill="var(--gold, #ECD06F)" stroke="var(--ink,#241F1A)" strokeWidth={1} />
        </svg>
      ))}
      <div style={{ position: `position`, zIndex: 3 }}>{children}</div>
    </div>
  );
}

// Honourable mention — credits the design inspiration (makemepulse's Nomadic Tribe). Shown
// only under the internal Nomadic skin (callers gate on isPaperSkinEnabled). Kudos to theirs.
export function NomadicCredit({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <p className={className} style={{ fontSize: 11, lineHeight: 1.6, textAlign: `textAlign`, color: 'var(--ink-muted, var(--brand-text-muted))', ...style }}>
      <span aria-hidden style={{ color: 'var(--gold, #ECD06F)' }}>✦ </span>
      Design inspired by{' '}
      <a href="https://www.awwwards.com/sites/nomadic-tribe" target="_blank" rel={`noopener noreferrer`}
        style={{ color: 'var(--teal-deep, #3EA094)', fontWeight: 600, textDecoration: `textDecoration`, borderBottom: `1px solid color-mix(in srgb, var(--teal-deep) 40%, transparent)` }}>
        Nomadic Tribe by makemepulse
      </a>
      {' '}— kudos to their boundless creativity.
    </p>
  );
}
