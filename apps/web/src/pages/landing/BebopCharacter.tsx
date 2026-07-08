import React, { useRef, useState } from 'react';
import { motion, useScroll, useSpring, useTransform, useReducedMotion } from 'framer-motion';
import { useNavigate } from 'react-router-dom';

/**
 * BebopCharacter — an ORIGINAL Spike-homage silhouette (Bebop vibe, no trademarked
 * likeness: lean figure, long coat, wild hair, a lit cigarette). Built as layered
 * SVG groups so it can:
 *   • parallax against HorizonDrift on scroll (depth),
 *   • glance / turn on hover (the ember brightens, a dry-wit line appears),
 *   • act on click → smooth-scroll to the primary CTA ("Claim your storefront").
 * Visible AND usable — not a passive backdrop.
 *
 * Real Krea/ComfyUI/Viggle art can later replace the inline SVG by swapping the
 * <g> fills for <image> layers; the rig (parallax/hover/click) stays identical.
 * Pass `imageSrc` to drop in a raster key-frame (e.g. Gemini-Pro / Krea / ComfyUI
 * generated art placed in /public) — rendered as the character, SVG stays as the
 * reduced-motion fallback. Pass `videoSrc` for an animated clip (Viggle/ComfyUI),
 * which takes priority over the still image; SVG remains the poster + fallback.
 */
export function BebopCharacter({
  scrollRef,
  className = '',
  videoSrc,
  imageSrc,
}: {
  scrollRef?: React.RefObject<HTMLElement>;
  className?: string;
  videoSrc?: string;
  imageSrc?: string;
}) {
  const navigate = useNavigate();
  const reduce = useReducedMotion();
  const [hover, setHover] = useState(false);

  // Scroll-linked parallax. If no scroll container is passed, fall back to window.
  const { scrollYProgress } = useScroll(
    scrollRef ? { container: scrollRef } : undefined,
  );
  const spring = useSpring(scrollYProgress, { stiffness: 120, damping: 30, mass: 0.4 });

  // Character drifts up + scales slightly as you scroll the pitch (depth vs. the ship).
  const y = useTransform(spring, [0, 0.18], [0, -60]);
  const scale = useTransform(spring, [0, 0.18], [1, 1.06]);
  const emberOpacity = useTransform(spring, [0, 0.18], [0.7, 1]);

  // Hover glance: the head tilts + ember flares.
  const headRotate = hover && !reduce ? -4 : 0;
  const emberScale = hover && !reduce ? 1.4 : 1;

  return (
    <motion.div
      className={`lp-character ${className}`}
      style={reduce ? undefined : { y, scale }}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.9, ease: [0.23, 1, 0.32, 1] }}
      role="button"
      tabIndex={0}
      aria-label="Bebop courier — scroll to claim your storefront"
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      onFocus={() => setHover(true)}
      onBlur={() => setHover(false)}
      onClick={() => {
        const el = document.getElementById('lp-claim');
        if (el) el.scrollIntoView({ behavior: reduce ? 'auto' : 'smooth' });
        else navigate('/claim');
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          navigate('/claim');
        }
      }}
    >
      {videoSrc && !reduce ? (
        <video
          className="lp-character__video"
          src={videoSrc}
          autoPlay
          loop
          muted
          playsInline
          poster=""
          aria-hidden="true"
        />
      ) : imageSrc && !reduce ? (
        <img
          className="lp-character__img"
          src={imageSrc}
          alt=""
          aria-hidden="true"
          draggable={false}
        />
      ) : (
      <svg viewBox="0 0 320 520" width="100%" height="100%" preserveAspectRatio="xMidYMax meet" role="img" aria-hidden="true">
        <defs>
          <linearGradient id="coatGrad" x1="0" y1="0" x2="0.4" y2="1">
            <stop offset="0" stopColor="#4a6ea0" />
            <stop offset="1" stopColor="#22324a" />
          </linearGradient>
          <linearGradient id="hairGrad" x1="0" y1="0" x2="0.3" y2="1">
            <stop offset="0" stopColor="#4a3a32" />
            <stop offset="1" stopColor="#241b17" />
          </linearGradient>
          <radialGradient id="emberGrad" cx="0.5" cy="0.5" r="0.5">
            <stop offset="0" stopColor="#ffd98a" />
            <stop offset="0.5" stopColor="#e8a544" />
            <stop offset="1" stopColor="rgba(232,165,68,0)" />
          </radialGradient>
          <linearGradient id="skinGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="#d8b690" />
            <stop offset="1" stopColor="#a07e5e" />
          </linearGradient>
          <radialGradient id="backGlow" cx="0.5" cy="0.5" r="0.5">
            <stop offset="0" stopColor="rgba(232,165,68,0.28)" />
            <stop offset="1" stopColor="rgba(232,165,68,0)" />
          </radialGradient>
        </defs>

        {/* back-glow so the figure separates from the void */}
        <ellipse cx="170" cy="300" rx="160" ry="220" fill="url(#backGlow)" />

        {/* ground shadow */}
        <ellipse cx="160" cy="500" rx="86" ry="12" fill="rgba(0,0,0,0.45)" />

        {/* ── COAT (back panel) ── */}
        <path
          d="M96 196 C70 230 58 320 70 430 C76 478 96 496 120 500 L200 500 C224 496 244 478 250 430 C262 320 250 230 224 196 C210 250 200 280 160 286 C120 280 110 250 96 196 Z"
          fill="url(#coatGrad)"
          stroke="rgba(232,165,68,0.9)"
          strokeWidth="3"
        />
        {/* amber backlight rim along the right edge (sunset key-light) */}
        <path d="M224 196 C250 230 262 320 250 430 C244 478 224 496 200 500" fill="none" stroke="rgba(240,185,94,0.85)" strokeWidth="3.5" strokeLinecap="round" />
        {/* coat seam / lapel */}
        <path d="M160 286 L160 500" stroke="#0c0b0a" strokeWidth="3" opacity="0.6" />
        <path d="M160 286 C140 300 120 360 120 500" stroke="rgba(232,165,68,0.25)" strokeWidth="2" fill="none" />
        <path d="M160 286 C180 300 200 360 200 500" stroke="rgba(232,165,68,0.25)" strokeWidth="2" fill="none" />

        {/* ── TORSO / SHIRT ── */}
        <path d="M124 196 C120 250 130 280 160 286 C190 280 200 250 196 196 Z" fill="#1a1f2b" />
        <path d="M160 196 L160 286" stroke="#0c0b0a" strokeWidth="2" />

        {/* ── NECK ── */}
        <rect x="146" y="168" width="28" height="36" rx="8" fill="url(#skinGrad)" />

        {/* ── HEAD (group: glances on hover) ── */}
        <motion.g
          style={{ transformOrigin: '160px 150px' }}
          animate={{ rotate: headRotate }}
          transition={{ type: 'spring', stiffness: 200, damping: 18 }}
        >
          {/* face */}
          <path d="M132 110 C132 78 150 64 160 64 C170 64 188 78 188 110 C188 146 172 166 160 166 C148 166 132 146 132 110 Z" fill="url(#skinGrad)" />
          {/* jaw shadow */}
          <path d="M138 132 C148 158 172 158 182 132 C178 150 142 150 138 132 Z" fill="rgba(0,0,0,0.18)" />
          {/* eye (one lit, dry) */}
          <path d="M144 112 q8 -5 16 0" stroke="#0c0b0a" strokeWidth="2.4" fill="none" strokeLinecap="round" />
          <circle cx="172" cy="112" r="2.4" fill="#0c0b0a" />
          {/* nose + mouth */}
          <path d="M160 118 l-3 12 l5 1" stroke="rgba(0,0,0,0.3)" strokeWidth="1.6" fill="none" />
          <path d="M150 142 q10 6 20 0" stroke="rgba(0,0,0,0.4)" strokeWidth="2" fill="none" strokeLinecap="round" />

          {/* ── HAIR (wild, swept) ── */}
          <path
            d="M126 112 C120 70 142 44 162 46 C188 48 200 74 196 104 C204 92 206 70 196 58 C214 74 214 104 200 122 C210 140 204 160 190 162 C196 140 186 124 176 122 C188 120 196 132 192 146 C180 138 170 132 158 132 C150 132 142 138 138 148 C128 138 122 126 126 112 Z"
            fill="url(#hairGrad)"
          />
          {/* hair highlight (amber rim) */}
          <path d="M140 66 C152 54 174 54 188 70" stroke="rgba(232,165,68,0.45)" strokeWidth="2" fill="none" />
        </motion.g>

        {/* ── COLLAR (turns with hover via slight lift) ── */}
        <motion.path
          d="M124 196 C128 176 146 172 160 184 C174 172 192 176 196 196 C188 188 172 192 160 200 C148 192 132 188 124 196 Z"
          fill="#0e1622"
          animate={{ y: hover && !reduce ? -3 : 0 }}
          transition={{ type: 'spring', stiffness: 200, damping: 18 }}
        />

        {/* ── ARM + HAND holding cigarette (brought forward, in-frame) ── */}
        <path d="M196 210 C232 222 252 252 250 290 C251 304 244 310 236 306 C234 272 220 246 192 238 Z" fill="url(#coatGrad)" stroke="rgba(232,165,68,0.4)" strokeWidth="1.5" />
        {/* cigarette — clearly in frame, lit tip glowing */}
        <rect x="238" y="284" width="40" height="6" rx="3" transform="rotate(-16 258 287)" fill="#e8e0d2" />
        <rect x="272" y="279" width="8" height="6" rx="3" transform="rotate(-16 276 282)" fill="#c0392b" />
        {/* ember (pulses on hover + scroll) — brighter halo for legibility */}
        <motion.circle
          cx="278"
          cy="278"
          r="22"
          fill="url(#emberGrad)"
          opacity={0.4}
          style={{ opacity: emberOpacity }}
          animate={{ scale: emberScale }}
          transition={{ type: 'spring', stiffness: 260, damping: 14 }}
        />
        <motion.circle
          cx="278"
          cy="278"
          r="8"
          fill="#ffd98a"
          style={{ opacity: emberOpacity }}
          animate={{ scale: emberScale }}
          transition={{ type: 'spring', stiffness: 260, damping: 14 }}
        />
        {/* smoke wisp (subtle, only when not reduced) */}
        {!reduce && (
          <motion.path
            d="M281 270 q8 -16 -2 -30 q-10 -14 2 -28"
            stroke="rgba(242,233,219,0.4)"
            strokeWidth="2.5"
            fill="none"
            strokeLinecap="round"
            animate={{ opacity: [0.15, 0.45, 0.15], y: [0, -5, 0] }}
            transition={{ duration: 4, repeat: Infinity, ease: 'easeInOut' }}
          />
        )}

        {/* ── OTHER ARM (resting) ── */}
        <path d="M124 210 C92 224 74 256 76 292 C77 306 84 310 92 304 C94 272 106 246 124 238 Z" fill="url(#coatGrad)" />

        {/* ── BOOTS ── */}
        <path d="M112 470 l44 0 l4 28 l-52 0 z" fill="#0c0b0a" />
        <path d="M164 470 l44 0 l4 28 l-52 0 z" fill="#0c0b0a" />
      </svg>
      )}

      {/* dry-wit line, appears on hover */}
      <motion.div
        className="lp-character__quip"
        initial={false}
        animate={{ opacity: hover ? 1 : 0, y: hover ? 0 : 8 }}
        transition={{ duration: 0.3 }}
        aria-hidden="true"
      >
        See you, space cowboy.
      </motion.div>
    </motion.div>
  );
}
