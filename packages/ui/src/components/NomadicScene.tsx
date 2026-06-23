import React from 'react';

// NOMADIC / MOEBIUS scene + Art-Nouveau ornament — the "poetic journey" centerpiece of the
// internal skin (target: makemepulse's Nomadic Tribe — Moebius line-art, Art-Nouveau framing,
// the #987654/#49c5b6/#ECD06F palette used VIBRANTLY, storytelling mood). Pure inline SVG:
// bold ink contours + flat colour fields, comic-panel framing. Inherits the paper tokens.

const INK = { fill: 'none', stroke: 'var(--ink, #241F1A)', strokeLinecap: 'round' as const, strokeLinejoin: 'round' as const };

export interface NomadicSceneProps { animated?: boolean; className?: string; style?: React.CSSProperties; }

// A layered desert-journey landscape: gold sun + rays, teal/sand dunes, a winding path, a lone
// caravan, drifting birds. Bold Moebius ink, flat vibrant fills.
export function NomadicScene({ animated = true, className, style }: NomadicSceneProps) {
  return (
    <svg viewBox="0 0 400 260" role="presentation" aria-hidden className={className}
      style={{ width: '100%', height: 'auto', display: 'block', ...style }}>
      {/* warm sky wash */}
      <defs>
        <linearGradient id="nt-sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="var(--gold, #ECD06F)" stopOpacity="0.35" />
          <stop offset="100%" stopColor="var(--paper-surface, #FBF5E9)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <rect x="0" y="0" width="400" height="200" fill="url(#nt-sky)" />

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
      <path d="M70 56 q8 -7 16 0 q8 -7 16 0" {...INK} strokeWidth={2} opacity="0.7" />
      <path d="M104 44 q6 -5 12 0 q6 -5 12 0" {...INK} strokeWidth={2} opacity="0.6" />

      {/* far dune ridge — teal-deep flat fill */}
      <path d="M0 150 q70 -40 150 -10 q90 34 150 -6 q60 -28 100 4 V200 H0 Z" fill="var(--teal-deep, #3EA094)" opacity="0.85" />
      <path d="M0 150 q70 -40 150 -10 q90 34 150 -6 q60 -28 100 4" {...INK} strokeWidth={3} />

      {/* mid dune — teal */}
      <path d="M0 178 q90 -34 180 -6 q90 28 220 -2 V210 H0 Z" fill="var(--teal, #49C5B6)" opacity="0.6" />
      {/* near dune — sand */}
      <path d="M0 196 q110 -28 210 2 q120 30 180 0 V260 H0 Z" fill="var(--sand, #987654)" opacity="0.45" />
      <path d="M0 196 q110 -28 210 2 q120 30 180 0" {...INK} strokeWidth={3} />

      {/* winding path to the horizon */}
      <path d="M150 256 q14 -40 -6 -70 q-18 -28 18 -52 q24 -16 12 -36" {...INK} strokeWidth={2.5} strokeDasharray="1 10" opacity="0.7" />

      {/* lone caravan — a two-hump camel + rider, Moebius ink */}
      <g transform="translate(188 150)">
        {/* body silhouette (sand fill) */}
        <path d="M4 22 q2 -9 9 -9 q2 -9 7 -2 q2 2 2 6 q3 -8 8 -7 q5 1 5 9 q0 3 -1 5 Z" fill="var(--sand, #987654)" opacity="0.55" />
        {/* back + two humps */}
        <path d="M4 22 q2 -9 9 -9 q2 -9 7 -2 q2 2 2 6 q3 -8 8 -7 q5 1 5 9" {...INK} strokeWidth={3.5} />
        {/* neck + head */}
        <path d="M35 19 q6 -3 8 -11 q1 -4 4 -2 q2 2 -1 5" {...INK} strokeWidth={3} />
        {/* four legs */}
        <path d="M9 21 l-1 13 M17 23 l0 12 M29 23 l1 12 M37 21 l1 13" {...INK} strokeWidth={3} />
        {/* rider */}
        <circle cx="22" cy="0" r="3" fill="var(--ink, #241F1A)" />
        <path d="M22 3 q0 6 -3 8" {...INK} strokeWidth={3} />
      </g>
    </svg>
  );
}

// Art-Nouveau ornamental divider — a flowing horizontal rule with mirrored whiplash curves and
// a central gold node. Used under section headings to carry the language into the chrome.
export function ArtNouveauDivider({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg viewBox="0 0 200 14" preserveAspectRatio="xMidYMid meet" aria-hidden
      className={className} style={{ width: '100%', height: 14, display: 'block', ...style }}>
      <path d="M2 7 H78 q8 0 12 -5 M198 7 H122 q-8 0 -12 -5" stroke="var(--teal-deep, #3EA094)" fill="none" strokeWidth={1.5} strokeLinecap="round" />
      <path d="M90 2 q10 5 0 10 q-10 -5 0 -10 M110 2 q-10 5 0 10 q10 -5 0 -10" stroke="var(--ink, #241F1A)" fill="none" strokeWidth={1.25} opacity="0.7" />
      <circle cx="100" cy="7" r="3.2" fill="var(--gold, #ECD06F)" stroke="var(--ink, #241F1A)" strokeWidth={1.25} />
    </svg>
  );
}

// Art-Nouveau ornamental frame — a flowing curved border with corner flourishes that wraps hero
// content. Children render above; the frame is a decorative overlay (pointer-events:none).
export function ArtNouveauFrame({ children, className, style }: { children?: React.ReactNode; className?: string; style?: React.CSSProperties }) {
  return (
    <div className={className} style={{ position: 'relative', ...style }}>
      <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden
        style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', pointerEvents: 'none', zIndex: 1 }}>
        {/* inset flowing border */}
        <rect x="3" y="3" width="94" height="94" rx="6" {...INK} strokeWidth={0.7} vectorEffect="non-scaling-stroke" opacity="0.8" />
        <rect x="5" y="5" width="90" height="90" rx="5" {...INK} strokeWidth={0.4} vectorEffect="non-scaling-stroke" opacity="0.5" />
      </svg>
      {/* corner flourishes (fixed-size, non-stretched) */}
      {[['0','0',''],['100%','0','scaleX(-1)'],['0','100%','scaleY(-1)'],['100%','100%','scale(-1)']].map(([x,y,t],i)=>(
        <svg key={i} viewBox="0 0 40 40" width="40" height="40" aria-hidden
          style={{ position:'absolute', left:x, top:y, transform:`translate(${String(x).includes('%')?'-100%':'0'},${String(y).includes('%')?'-100%':'0'}) ${t}`, pointerEvents:'none', zIndex:2 }}>
          <path d="M6 34 q0 -20 14 -24 q10 -3 14 -4 M10 30 q2 -12 12 -16" stroke="var(--teal-deep, #3EA094)" fill="none" strokeWidth={2} strokeLinecap="round" />
          <circle cx="34" cy="6" r="2.5" fill="var(--gold, #ECD06F)" stroke="var(--ink,#241F1A)" strokeWidth={1} />
        </svg>
      ))}
      <div style={{ position: 'relative', zIndex: 3 }}>{children}</div>
    </div>
  );
}
