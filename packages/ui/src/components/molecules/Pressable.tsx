import { type ReactNode } from 'react';
import { motion, type HTMLMotionProps } from 'framer-motion';
import { scalePress, spring } from '../../lib/motion.js';

interface PressableProps extends HTMLMotionProps<'div'> {
  children: ReactNode;
  /** Disable whileTap scale */
  noTap?: boolean;
  /** Disable whileHover lift (desktop) */
  noHover?: boolean;
}

export function Pressable({ children, noTap, noHover, className = '', ...props }: PressableProps) {
  return (
    <motion.div
      className={className}
      whileTap={noTap ? undefined : { scale: scalePress.scale, transition: spring.press }}
      whileHover={noHover ? undefined : { scale: 1.01, transition: { duration: 0.15 } }}
      {...props}
    >
      {children}
    </motion.div>
  );
}
