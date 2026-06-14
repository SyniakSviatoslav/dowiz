import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { PriceDisplay } from '../../index.js';

export function CartFAB({ itemsCount, total, onClick, isBouncing = false }: { itemsCount: number; total: number; onClick: () => void; isBouncing?: boolean }) {
  return (
    <AnimatePresence>
      {itemsCount > 0 && (
        <motion.div
          className="fixed bottom-[80px] right-[20px] z-[100] embed-hidden"
          initial={{ scale: 0, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          exit={{ scale: 0, opacity: 0 }}
          transition={{ type: 'spring', stiffness: 260, damping: 24 }}
        >
          <motion.button 
            id="cartFabBtn" 
            aria-label={`Cart: ${itemsCount} items`}
            className={`h-[48px] px-5 text-white text-[14px] font-medium flex items-center justify-center gap-1 ${isBouncing ? 'cart-bounce' : ''}`}
            style={{ 
              background: 'var(--brand-primary)', 
              borderRadius: 'var(--brand-radius-btn)', 
              boxShadow: '0 4px 12px color-mix(in srgb, var(--brand-primary) 40%, transparent)' 
            }} 
            onClick={onClick}
            whileTap={{ scale: 0.95 }}
          >
            <span className="relative inline-flex">
              <i className="ti ti-shopping-cart text-lg leading-none" />
              <span className="absolute -top-2 -right-2 min-w-[18px] h-[18px] rounded-full bg-[var(--color-danger)] text-white text-[10px] font-bold flex items-center justify-center leading-none px-1 shadow-md">
                {itemsCount > 99 ? '99+' : itemsCount}
              </span>
            </span>
            <span className="mx-1 opacity-40">|</span>
            <PriceDisplay amount={total} />
          </motion.button>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
