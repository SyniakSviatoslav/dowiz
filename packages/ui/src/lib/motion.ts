import { type Transition, type Variants } from 'framer-motion';

/*
 * THE motion source of truth (Emil-calibrated). Import from here / the @deliveryos/ui barrel —
 * never inline `[0.16,1,0.3,1]` or a raw cubic-bezier in a component. These mirror the CSS tokens
 * in theme/tokens.css (--ease-out / --ease-soft / --ease-in-out / --motion-*). Conventions:
 *  - entrances/exits use ease.out (strong expo — "instant feedback"); on-screen movement ease.inOut.
 *  - UI durations stay < 0.3s (fast/base); modals/drawers may use slow. Exit faster than enter.
 *  - never animate from scale(0) (variants start at 0.95+); :active press = scalePress (0.97).
 *  - bounce is reserved for rare playful moments (NOT default card/list entry — Emil: avoid bounce in UI).
 *  - every consumer must honor prefers-reduced-motion (useReducedMotion) — these variants are motion;
 *    gate or swap to opacity-only under reduced motion at the call site.
 */

/* ── Easing curves ── */
export const ease = {
  out:   [0.16, 1, 0.3, 1] as const,
  inOut: [0.65, 0, 0.35, 1] as const,
  soft:  [0.4, 0, 0.2, 1] as const,
  bounce: [0.34, 1.56, 0.64, 1] as const,
};

/* ── Spring presets ── */
export const spring = {
  press:  { type: 'spring' as const, stiffness: 400, damping: 25 },
  enter:  { type: 'spring' as const, stiffness: 260, damping: 24 },
  bounce: { type: 'spring' as const, stiffness: 500, damping: 18 },
  gentle: { type: 'spring' as const, stiffness: 180, damping: 22 },
};

/* ── Duration presets (ms) ── */
export const duration = {
  instant: 0.08,
  fast:    0.15,
  base:    0.24,
  slow:    0.4,
};

/* ── Default transition ── */
export const defaultTransition: Transition = {
  duration: duration.base,
  ease: ease.out,
};

/* ── Page transitions — for AnimatePresence ── */
export const pageTransition: Variants = {
  initial: { opacity: 0, y: 8, scale: 0.99 },
  animate: { opacity: 1, y: 0, scale: 1, transition: { duration: duration.slow, ease: ease.out } },
  exit:    { opacity: 0, y: -8, scale: 0.99, transition: { duration: duration.fast, ease: ease.soft } },
};

export const pageTransitionSlide: Variants = {
  initial: { opacity: 0, x: 16 },
  animate: { opacity: 1, x: 0, transition: { duration: duration.slow, ease: ease.out } },
  exit:    { opacity: 0, x: -16, transition: { duration: duration.fast, ease: ease.soft } },
};

/* ── Entry variants ── */
export const fadeIn: Variants = {
  hidden: { opacity: 0 },
  visible: { opacity: 1, transition: defaultTransition },
};

export const scaleIn: Variants = {
  hidden: { opacity: 0, scale: 0.95 },
  visible: { opacity: 1, scale: 1, transition: defaultTransition },
};

export const slideUp: Variants = {
  hidden: { opacity: 0, y: 16 },
  visible: { opacity: 1, y: 0, transition: defaultTransition },
};

export const slideDown: Variants = {
  hidden: { opacity: 0, y: -16 },
  visible: { opacity: 1, y: 0, transition: defaultTransition },
};

export const slideLeft: Variants = {
  hidden: { opacity: 0, x: 20 },
  visible: { opacity: 1, x: 0, transition: defaultTransition },
};

export const slideRight: Variants = {
  hidden: { opacity: 0, x: -20 },
  visible: { opacity: 1, x: 0, transition: defaultTransition },
};

/* ── Exit-only variants (for items leaving DOM) ── */
export const fadeOut: Variants = {
  exit: { opacity: 0, transition: { duration: duration.fast, ease: ease.soft } },
};

export const scaleOut: Variants = {
  exit: { opacity: 0, scale: 0.95, transition: { duration: duration.fast, ease: ease.soft } },
};

export const slideOutDown: Variants = {
  exit: { opacity: 0, y: 16, transition: { duration: duration.fast, ease: ease.soft } },
};

/* ── List / stagger variants ── */
export const listItem: Variants = {
  hidden: { opacity: 0, y: 10, scale: 0.98 },
  visible: { opacity: 1, y: 0, scale: 1, transition: { duration: duration.base, ease: ease.out } },
};

export const listItemFast: Variants = {
  hidden: { opacity: 0, y: 6 },
  visible: { opacity: 1, y: 0, transition: { duration: duration.fast, ease: ease.out } },
};

export const staggerChildren: Variants = {
  hidden: { transition: { staggerChildren: 0.04, delayChildren: 0.05 } },
  visible: { transition: { staggerChildren: 0.04, delayChildren: 0.05 } },
};

export const staggerFast: Variants = {
  hidden: { transition: { staggerChildren: 0.02, delayChildren: 0.02 } },
  visible: { transition: { staggerChildren: 0.02, delayChildren: 0.02 } },
};

/* ── Micro-interaction transforms (use directly on motion.div) ── */
export const scalePress = { scale: 0.97 };
export const scaleLift = { scale: 1.02 };
export const scaleTap = { scale: 0.95 };

/* ── Card entry — for items appearing in a grid ── */
export const cardEntry: Variants = {
  // Emil-calibration: cards in a grid are seen often → crisp expo ease-out, NOT bounce
  // (bounce on frequent entries reads as attention-seeking). Bounce stays for rare delight only.
  hidden: { opacity: 0, y: 16, scale: 0.97 },
  visible: { opacity: 1, y: 0, scale: 1, transition: { duration: duration.base, ease: ease.out } },
  exit: { opacity: 0, y: -8, scale: 0.97, transition: { duration: duration.fast, ease: ease.soft } },
};

/* ── Badge / dot pulse ── */
export const pulseDot: Variants = {
  idle: { scale: 1, opacity: 1 },
  pulse: {
    scale: [1, 0.85, 1],
    opacity: [1, 0.4, 1],
    transition: { duration: 2, repeat: Infinity, ease: ease.soft },
  },
};

/* ── Notification slide-in (for toasts) ── */
export const toastIn: Variants = {
  hidden: { opacity: 0, x: 60, scale: 0.95 },
  visible: { opacity: 1, x: 0, scale: 1, transition: { type: 'spring', stiffness: 300, damping: 24 } },
  exit: { opacity: 0, x: 60, scale: 0.95, transition: { duration: duration.fast, ease: ease.soft } },
};

/* ── Modal overlay ── */
export const overlayIn: Variants = {
  hidden: { opacity: 0 },
  visible: { opacity: 1, transition: { duration: duration.fast } },
  exit: { opacity: 0, transition: { duration: duration.fast } },
};

export const modalIn: Variants = {
  hidden: { opacity: 0, y: 40, scale: 0.96 },
  visible: { opacity: 1, y: 0, scale: 1, transition: { type: 'spring', stiffness: 300, damping: 28 } },
  exit: { opacity: 0, y: 30, scale: 0.96, transition: { duration: duration.fast, ease: ease.soft } },
};
