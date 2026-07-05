// RevealOverlay — a decorative Canvas-2D dissolve played once when the product
// modal opens.
//
// How MenuPage integrates this: the lead mounts <RevealOverlay active sourceRect={…}
// onDone={…}/> over the modal hero on open (only when MEDIA_RICH_ENABLED + business
// tier); onDone flips a flag that reveals the modal content. The component is
// React.lazy code-split by the lead. It is purely DECORATIVE: it renders with
// pointer-events:none above the hero and NEVER gates the Add-to-Cart button, which
// renders independently beneath it and is interactive the whole time. If the user
// prefers reduced motion, or the 2D context is unavailable, it calls onDone
// immediately (instant show) and paints nothing.
//
// No-leak guarantee (Phase-2 contract §Cinematic reveal): the rafId lives in a ref;
// every cleanup path — unmount AND `active` toggling off mid-pass — runs
// cancelAnimationFrame(ref) and clears the canvas. A single pass (~400–600ms) calls
// onDone exactly once via a guard ref, then the overlay unmounts. 50 rapid
// open/close cycles therefore leak no RAF handles.

import React, { useEffect, useRef } from 'react';
import { useReducedMotion } from './hooks';

interface RevealOverlayProps {
  active: boolean;
  onDone: () => void;
  /** Optional origin rect (e.g. the tapped card) — particles bias toward it. */
  sourceRect?: { x: number; y: number; width: number; height: number };
}

const PARTICLE_COUNT = 80;
const DURATION_MS = 520;

interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  r: number;
  hue: number;
}

export function RevealOverlay({ active, onDone, sourceRect }: RevealOverlayProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const rafRef = useRef<number | null>(null);
  const doneRef = useRef(false);
  const reducedMotion = useReducedMotion();

  // Keep the latest onDone without re-running the animation effect.
  const onDoneRef = useRef(onDone);
  onDoneRef.current = onDone;

  useEffect(() => {
    doneRef.current = false;

    const finish = () => {
      if (doneRef.current) return;
      doneRef.current = true;
      onDoneRef.current();
    };

    // Not active, or the user asked for no motion → instant show, no particle pass.
    if (!active || reducedMotion) {
      finish();
      return;
    }

    const canvas = canvasRef.current;
    const ctx = canvas ? canvas.getContext('2d') : null;
    // Graceful skip — no 2D context available.
    if (!canvas || !ctx) {
      finish();
      return;
    }

    const dpr = typeof window !== 'undefined' ? Math.min(window.devicePixelRatio || 1, 2) : 1;
    const w = canvas.clientWidth || canvas.width;
    const h = canvas.clientHeight || canvas.height;
    canvas.width = Math.max(1, Math.floor(w * dpr));
    canvas.height = Math.max(1, Math.floor(h * dpr));
    ctx.scale(dpr, dpr);

    const originX = sourceRect ? sourceRect.x + sourceRect.width / 2 : w / 2;
    const originY = sourceRect ? sourceRect.y + sourceRect.height / 2 : h / 2;

    const particles: Particle[] = Array.from({ length: PARTICLE_COUNT }, () => {
      const angle = Math.random() * Math.PI * 2;
      const speed = 0.6 + Math.random() * 1.8;
      return {
        x: originX + (Math.random() - 0.5) * (sourceRect?.width ?? w) * 0.4,
        y: originY + (Math.random() - 0.5) * (sourceRect?.height ?? h) * 0.4,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        r: 1.5 + Math.random() * 3,
        hue: 38 + Math.random() * 14, // warm amber, brand-ish
      };
    });

    let start: number | null = null;

    const frame = (ts: number) => {
      if (start === null) start = ts;
      const elapsed = ts - start;
      const progress = Math.min(elapsed / DURATION_MS, 1);
      const alpha = 1 - progress; // dissolve out

      ctx.clearRect(0, 0, w, h);
      for (const p of particles) {
        p.x += p.vx;
        p.y += p.vy;
        ctx.globalAlpha = alpha;
        ctx.fillStyle = `hsl(${p.hue}, 90%, 60%)`;
        ctx.beginPath();
        ctx.arc(p.x, p.y, p.r * (1 - progress * 0.4), 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;

      if (progress < 1) {
        rafRef.current = requestAnimationFrame(frame);
      } else {
        ctx.clearRect(0, 0, w, h);
        rafRef.current = null;
        finish();
      }
    };

    rafRef.current = requestAnimationFrame(frame);

    // Cleanup: cancel the RAF + clear the canvas. Runs on unmount AND whenever
    // `active`/reducedMotion change (i.e. active toggled off mid-pass) → no leak.
    return () => {
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
      ctx.clearRect(0, 0, w, h);
    };
  }, [active, reducedMotion, sourceRect]);

  // Nothing to paint when inactive — keeps the DOM clean between opens.
  if (!active) return null;

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      className="dz-reveal-overlay"
      style={{
        position: 'absolute',
        inset: 0,
        width: '100%',
        height: '100%',
        pointerEvents: 'none', // decorative — never blocks Add-to-Cart beneath it
        zIndex: 3,
      }}
    />
  );
}

export default RevealOverlay;
