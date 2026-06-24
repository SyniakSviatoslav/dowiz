import { motion } from 'framer-motion';
import { ease } from '../../lib/motion.js';

interface AnimatedCheckProps {
  size?: number;
  strokeWidth?: number;
  className?: string;
  color?: string;
}

export function AnimatedCheck({ size = 48, strokeWidth = 3, className = '', color }: AnimatedCheckProps) {
  const strokeColor = color || 'var(--color-success)';
  return (
    <motion.svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      initial={{ opacity: 0, scale: 0.85 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.2, ease: ease.out }}
    >
      <motion.circle
        cx="12" cy="12" r="11"
        stroke={strokeColor}
        strokeWidth={strokeWidth}
        fill="none"
        initial={{ pathLength: 0 }}
        animate={{ pathLength: 1 }}
        transition={{ duration: 0.3, ease: ease.out }}
      />
      <motion.path
        d="M7 12.5l3 3 7-7"
        stroke={strokeColor}
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeLinejoin="round"
        fill="none"
        initial={{ pathLength: 0 }}
        animate={{ pathLength: 1 }}
        transition={{ duration: 0.25, delay: 0.15, ease: ease.out }}
      />
    </motion.svg>
  );
}
