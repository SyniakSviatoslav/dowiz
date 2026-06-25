import { type ReactNode } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import { ease, duration } from '../../lib/motion.js';

interface CrossfadeOnLoadProps {
  isLoading: boolean;
  children: ReactNode;
  skeleton?: ReactNode;
}

export function CrossfadeOnLoad({ isLoading, children, skeleton }: CrossfadeOnLoadProps) {
  const reduce = useReducedMotion();
  return (
    <AnimatePresence mode="wait">
      {isLoading ? (
        <motion.div
          key="skeleton"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={reduce ? { opacity: 0 } : { opacity: 0, scale: 0.97 }}
          transition={{ duration: duration.fast, ease: ease.soft }}
        >
          {skeleton}
        </motion.div>
      ) : (
        <motion.div
          key="content"
          initial={reduce ? { opacity: 0 } : { opacity: 0, scale: 0.98 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: duration.base, ease: ease.out }}
        >
          {children}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
