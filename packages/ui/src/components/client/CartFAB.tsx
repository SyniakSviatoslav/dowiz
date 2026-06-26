import React from 'react';
import { motion, AnimatePresence, useReducedMotion } from 'framer-motion';
import { PriceDisplay } from '../../index.js';

export function CartFAB({ itemsCount, total, onClick, isBouncing = false }: { itemsCount: number; total: number; onClick: () => void; isBouncing?: boolean }) {
  const reduce = useReducedMotion();

  return (
    <AnimatePresence>
      {itemsCount > 0 && (
        <motion.div
          className="fixed right-[20px] z-[100] embed-hidden"
          style={{ bottom: 'calc(80px + var(--safe-bottom))' }}
          initial={reduce ? { opacity: 0 } : { transform: 'translateY(12px) scale(0.92)', opacity: 0 }}
          animate={reduce ? { opacity: 1 } : { transform: 'translateY(0px) scale(1)', opacity: 1 }}
          exit={reduce ? { opacity: 0 } : { transform: 'translateY(12px) scale(0.92)', opacity: 0 }}
          transition={reduce ? { duration: 0.15 } : { type: 'spring', stiffness: 320, damping: 26 }}
        >
          <motion.button
            id="cartFabBtn"
            aria-label={`Cart: ${itemsCount} items`}
            className="h-[48px] px-5 text-white text-step-sm font-medium flex items-center justify-center gap-1 transition-[box-shadow] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-primary)]"
            style={{
              background: 'var(--brand-primary)',
              borderRadius: 'var(--brand-radius-btn)',
              boxShadow: 'var(--elev-3)',
            }}
            onClick={onClick}
            whileHover={reduce ? undefined : { transform: 'translateY(-2px)', boxShadow: 'var(--elev-4)' }}
            whileTap={reduce ? undefined : { transform: 'scale(0.96)' }}
          >
            <span className="relative inline-flex">
              <i className="ti ti-shopping-cart text-lg leading-none" />
              <AnimatePresence mode="popLayout">
                <motion.span
                  key={itemsCount > 99 ? '99+' : itemsCount}
                  initial={reduce ? { opacity: 0 } : { transform: 'scale(0.6)', opacity: 0 }}
                  animate={reduce ? { opacity: 1 } : { transform: 'scale(1)', opacity: 1 }}
                  exit={reduce ? { opacity: 0 } : { transform: 'scale(0.6)', opacity: 0 }}
                  transition={reduce ? { duration: 0.1 } : { type: 'spring', stiffness: 500, damping: 22 }}
                  className="absolute -top-2 -right-2 min-w-[18px] h-[18px] rounded-full bg-[var(--color-danger-strong)] text-white text-step-2xs font-bold flex items-center justify-center leading-none px-1 tabular-nums"
                  style={{ boxShadow: 'var(--elev-2)' }}
                >
                  {itemsCount > 99 ? '99+' : itemsCount}
                </motion.span>
              </AnimatePresence>
            </span>
            <span className="mx-1 opacity-40" aria-hidden>|</span>
            <span className="tabular-nums"><PriceDisplay amount={total} /></span>
          </motion.button>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
