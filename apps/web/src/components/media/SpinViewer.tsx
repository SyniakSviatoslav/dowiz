// 360° frame-scrub viewer. Pointer-drag scrubs through media.meta.frameUrls;
// optional auto-rotate stops the moment the user interacts. No WebGL — just an
// <img> whose src swaps to the current frame. Per-frame load has a 4s timeout;
// on timeout (or no frames) we stay on the poster. prefers-reduced-motion →
// poster only, no RAF / no auto-rotate. RAF + refs are torn down on unmount.

import { useEffect, useRef, useState } from 'react';
import type { ProductMedia } from './types';
import { useReducedMotion } from './hooks';

const FRAME_TIMEOUT_MS = 4000;
const AUTO_ROTATE_MS_PER_FRAME = 90;

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    const timer = setTimeout(() => reject(new Error('frame timeout')), FRAME_TIMEOUT_MS);
    img.onload = () => { clearTimeout(timer); resolve(img); };
    img.onerror = () => { clearTimeout(timer); reject(new Error('frame error')); };
    img.src = url;
  });
}

export default function SpinViewer({ media }: { media: ProductMedia }) {
  const reduced = useReducedMotion();
  const frames = media.meta?.frameUrls ?? [];
  const poster = media.posterUrl || frames[0] || media.url;

  const [ready, setReady] = useState(false);
  const [index, setIndex] = useState(0);
  const interacted = useRef(false);
  const dragging = useRef(false);
  const lastX = useRef(0);
  const rafRef = useRef<number | null>(null);
  const lastTick = useRef(0);

  // Preload all frames; only flip to interactive once they resolve. Any failure
  // (incl. the 4s timeout) leaves us on the poster.
  useEffect(() => {
    if (reduced || frames.length === 0) return;
    let alive = true;
    Promise.all(frames.map(loadImage))
      .then(() => { if (alive) setReady(true); })
      .catch(() => { if (alive) setReady(false); });
    return () => { alive = false; };
  }, [reduced, frames]);

  // Auto-rotate until first interaction.
  useEffect(() => {
    if (reduced || !ready || frames.length === 0) return;
    const step = (t: number) => {
      if (interacted.current) return; // stop permanently on interaction
      if (t - lastTick.current >= AUTO_ROTATE_MS_PER_FRAME) {
        lastTick.current = t;
        setIndex((i) => (i + 1) % frames.length);
      }
      rafRef.current = requestAnimationFrame(step);
    };
    rafRef.current = requestAnimationFrame(step);
    return () => { if (rafRef.current != null) cancelAnimationFrame(rafRef.current); rafRef.current = null; };
  }, [reduced, ready, frames.length]);

  const stopAuto = () => {
    interacted.current = true;
    if (rafRef.current != null) { cancelAnimationFrame(rafRef.current); rafRef.current = null; }
  };

  const onPointerDown = (e: React.PointerEvent) => {
    if (!ready || frames.length === 0) return;
    stopAuto();
    dragging.current = true;
    lastX.current = e.clientX;
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
  };
  const onPointerMove = (e: React.PointerEvent) => {
    if (!dragging.current || frames.length === 0) return;
    const dx = e.clientX - lastX.current;
    const threshold = 6; // px per frame step
    if (Math.abs(dx) >= threshold) {
      const delta = Math.trunc(dx / threshold);
      lastX.current = e.clientX;
      setIndex((i) => ((i + delta) % frames.length + frames.length) % frames.length);
    }
  };
  const onPointerUp = () => { dragging.current = false; };

  // Reduced motion or unloaded → poster still image.
  if (reduced || !ready || frames.length === 0) {
    return (
      <img
        src={poster}
        alt={media.alt ?? ''}
        draggable={false}
        style={{ width: '100%', height: '100%', objectFit: 'contain', display: 'block' }}
      />
    );
  }

  return (
    <img
      src={frames[index]}
      alt={media.alt ?? ''}
      draggable={false}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      style={{ width: '100%', height: '100%', objectFit: 'contain', display: 'block', cursor: 'grab', touchAction: 'pan-y' }}
    />
  );
}
