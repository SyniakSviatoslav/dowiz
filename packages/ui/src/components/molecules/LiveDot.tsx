import { useEffect, useRef, useState } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { ease } from '../../lib/motion.js';

interface LiveDotProps {
  color?: string;
  size?: number;
  pulse?: boolean;
}

export function LiveDot({ color, size = 8, pulse = true }: LiveDotProps) {
  const prefersReducedMotion = useReducedMotion();
  const [isVisible, setIsVisible] = useState(true);
  const [isPageVisible, setIsPageVisible] = useState(true);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!pulse) return;

    const el = ref.current;
    if (!el || !el.parentElement) return;

    const observer = new IntersectionObserver(
      (entries) => { const e = entries[0]; if (e) setIsVisible(e.isIntersecting); },
      { threshold: 0 }
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [pulse]);

  useEffect(() => {
    if (!pulse) return;
    const handle = () => setIsPageVisible(!document.hidden);
    document.addEventListener('visibilitychange', handle);
    return () => document.removeEventListener('visibilitychange', handle);
  }, [pulse]);

  const dotColor = color || 'var(--color-success)';
  const shouldAnimate = pulse && isVisible && isPageVisible && !prefersReducedMotion;

  // Static fallback: reduced-motion, off-screen, hidden tab, or pulse disabled.
  // A faint ring keeps the "live" semantic legible without any motion.
  if (!shouldAnimate) {
    return (
      <div ref={ref} className="relative inline-flex items-center justify-center">
        <span
          className="rounded-full"
          style={{ width: size, height: size, backgroundColor: dotColor }}
        />
      </div>
    );
  }

  return (
    <div ref={ref} className="relative inline-flex items-center justify-center">
      {/* Expanding halo — the gentle "breathing" ring behind the solid core. */}
      <motion.span
        aria-hidden
        className="absolute rounded-full"
        style={{ width: size, height: size, backgroundColor: dotColor }}
        animate={{ scale: [1, 2.2], opacity: [0.45, 0] }}
        transition={{ duration: 1.8, repeat: Infinity, ease: ease.soft, repeatDelay: 0.2 }}
      />
      {/* Solid core — a soft scale/opacity breath, no harsh blink. */}
      <motion.span
        className="relative rounded-full"
        style={{ width: size, height: size, backgroundColor: dotColor }}
        animate={{ scale: [1, 1.12, 1], opacity: [1, 0.82, 1] }}
        transition={{ duration: 1.8, repeat: Infinity, ease: ease.soft, repeatDelay: 0.2 }}
      />
    </div>
  );
}
