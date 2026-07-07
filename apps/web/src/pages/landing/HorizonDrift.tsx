import React from 'react';

/**
 * HorizonDrift — the signature Dowiz ambient scene.
 * A cold, stylish machine drifting warm and slow across a distant, lit horizon —
 * the hybrid thesis made visible. Parallax layers: sky → planet → stars → ship
 * (+ anamorphic flare) → haze → vignette. GPU-only (transform/opacity), frozen
 * to a still frame under prefers-reduced-motion (see landing.css). Ship silhouette
 * is original (no trademarked craft). Purely decorative → aria-hidden.
 */
export function HorizonDrift() {
  return (
    <div className="lp-horizon stage-surface" aria-hidden="true">
      <div className="lp-sky" />
      <div className="lp-stars lp-stars--far" />
      <div className="lp-stars" />
      <div className="lp-planet" />
      <div className="lp-ship">
        <div className="lp-ship__flare" />
        <svg viewBox="0 0 240 90" width="100%" role="presentation">
          <defs>
            <linearGradient id="hullGrad" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0" stopColor="#2a2f34" />
              <stop offset="1" stopColor="#0c0b0a" />
            </linearGradient>
            <radialGradient id="engineGlow" cx="0.5" cy="0.5" r="0.5">
              <stop offset="0" stopColor="#f0b95e" />
              <stop offset="0.5" stopColor="#e8a544" />
              <stop offset="1" stopColor="rgba(232,165,68,0)" />
            </radialGradient>
          </defs>
          {/* engine bloom */}
          <ellipse cx="30" cy="52" rx="34" ry="10" fill="url(#engineGlow)" opacity="0.7" />
          {/* hull — sleek, swept, original silhouette */}
          <path
            d="M44 50 C70 40 108 34 150 36 C182 37 210 42 232 50 C214 55 196 58 168 60 C150 74 120 76 104 62 C86 64 62 60 44 50 Z"
            fill="url(#hullGrad)"
          />
          {/* swept wing */}
          <path d="M120 58 C132 70 150 82 176 82 C160 70 150 62 146 58 Z" fill="#0c0b0a" opacity="0.9" />
          {/* cockpit — the one warm light */}
          <path d="M176 44 C190 43 202 45 214 49 C204 51 194 52 182 52 C179 49 177 46 176 44 Z" fill="#e8a544" opacity="0.9" />
          <circle cx="196" cy="48" r="2.4" fill="#f2e9db" />
        </svg>
      </div>
      <div className="lp-haze" />
      <div className="lp-vignette" />
    </div>
  );
}
