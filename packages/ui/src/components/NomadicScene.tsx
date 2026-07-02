import React from 'react';

// Art-Nouveau ornamental divider — a flowing horizontal rule with mirrored whiplash curves and
// a central gold node. Used under section headings (EmptyState paper-skin fallback) to carry
// the internal-skin language into the chrome.
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
