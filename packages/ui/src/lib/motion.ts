import { type Transition, type Variants } from 'framer-motion';

/* ── Spring presets ── */
export const spring = {
  press:  { type: 'spring' as const, stiffness: 400, damping: 25 },
  enter:  { type: 'spring' as const, stiffness: 260, damping: 24 },
  bounce: { type: 'spring' as const, stiffness: 500, damping: 18 },
};

/* ── Default transition ── */
export const defaultTransition: Transition = {
  duration: 0.24,
  ease: [0.16, 1, 0.3, 1],
};

/* ── Variants ── */
export const fadeIn: Variants = {
  hidden: { opacity: 0 },
  visible: { opacity: 1, transition: defaultTransition },
};

export const scaleIn: Variants = {
  hidden: { opacity: 0, scale: 0.95 },
  visible: { opacity: 1, scale: 1, transition: defaultTransition },
};

export const slideUp: Variants = {
  hidden: { opacity: 0, y: 12 },
  visible: { opacity: 1, y: 0, transition: defaultTransition },
};

export const slideDown: Variants = {
  hidden: { opacity: 0, y: -12 },
  visible: { opacity: 1, y: 0, transition: defaultTransition },
};

export const slideLeft: Variants = {
  hidden: { opacity: 0, x: 20 },
  visible: { opacity: 1, x: 0, transition: defaultTransition },
};

export const listItem: Variants = {
  hidden: { opacity: 0, y: 8, scale: 0.98 },
  visible: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.2, ease: [0.16, 1, 0.3, 1] } },
};

export const staggerChildren: Variants = {
  hidden: { transition: { staggerChildren: 0.04, delayChildren: 0.05 } },
  visible: { transition: { staggerChildren: 0.04, delayChildren: 0.05 } },
};

export const scalePress = { scale: 0.97 };
export const scaleLift = { scale: 1.02 };
