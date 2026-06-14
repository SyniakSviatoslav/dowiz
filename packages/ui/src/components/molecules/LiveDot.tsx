import { useEffect, useRef, useState } from 'react';
import { motion } from 'framer-motion';

interface LiveDotProps {
  color?: string;
  size?: number;
  pulse?: boolean;
}

export function LiveDot({ color, size = 8, pulse = true }: LiveDotProps) {
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

  const shouldAnimate = pulse && isVisible && isPageVisible;

  if (!shouldAnimate) {
    return (
      <div ref={ref} className="inline-flex items-center justify-center">
        <span
          className="rounded-full"
          style={{ width: size, height: size, backgroundColor: color || 'var(--color-success)' }}
        />
      </div>
    );
  }

  return (
    <div ref={ref} className="inline-flex items-center justify-center">
      <motion.span
        className="rounded-full"
        style={{ width: size, height: size, backgroundColor: color || 'var(--color-success)' }}
        animate={{
          scale: [1, 1.3, 1],
          opacity: [1, 0.6, 1],
        }}
        transition={{
          duration: 2,
          repeat: Infinity,
          ease: 'easeInOut',
        }}
      />
    </div>
  );
}
