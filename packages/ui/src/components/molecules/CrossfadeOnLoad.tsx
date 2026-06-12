import { type ReactNode, useState, useEffect } from 'react';
import { AnimatePresence, motion } from 'framer-motion';

interface CrossfadeOnLoadProps {
  isLoading: boolean;
  children: ReactNode;
  skeleton?: ReactNode;
}

export function CrossfadeOnLoad({ isLoading, children, skeleton }: CrossfadeOnLoadProps) {
  return (
    <AnimatePresence mode="wait">
      {isLoading ? (
        <motion.div
          key="skeleton"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0, scale: 0.97 }}
          transition={{ duration: 0.15, ease: [0.16, 1, 0.3, 1] }}
        >
          {skeleton}
        </motion.div>
      ) : (
        <motion.div
          key="content"
          initial={{ opacity: 0, scale: 0.98 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
        >
          {children}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
