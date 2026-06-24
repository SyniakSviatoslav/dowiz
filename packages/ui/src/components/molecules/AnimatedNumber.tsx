import { useEffect, useRef, useState } from 'react';
import { useReducedMotion } from 'framer-motion';

interface AnimatedNumberProps {
  value: number;
  duration?: number;
  className?: string;
  formatter?: (v: number) => string;
}

export function AnimatedNumber({ value, duration = 240, className = '', formatter }: AnimatedNumberProps) {
  const [display, setDisplay] = useState(value);
  const reduceMotion = useReducedMotion();
  const rafRef = useRef<number>(0);
  const startRef = useRef<{ value: number; time: number }>({ value, time: Date.now() });

  useEffect(() => {
    if (value === display) return;
    if (reduceMotion) { setDisplay(value); return; }
    startRef.current = { value: display, time: Date.now() };

    const animate = () => {
      const elapsed = Date.now() - startRef.current.time;
      const progress = Math.min(elapsed / duration, 1);
      const eased = 1 - Math.pow(1 - progress, 3);
      const current = Math.round(startRef.current.value + (value - startRef.current.value) * eased);
      setDisplay(current);
      if (progress < 1) {
        rafRef.current = requestAnimationFrame(animate);
      }
    };
    rafRef.current = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(rafRef.current);
  }, [value, duration, reduceMotion]);

  return (
    <span className={className} style={{ fontVariantNumeric: 'tabular-nums' }}>
      {formatter ? formatter(display) : display}
    </span>
  );
}
