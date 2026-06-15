import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Button, BottomSheet, useI18n, PriceDisplay } from '../../index.js';

export interface CartItem {
  id: string;
  productId: string;
  name: string;
  quantity: number;
  price: number;
  options?: Record<string, string[]>;
}

interface CartDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  items: CartItem[];
  onUpdateQuantity: (id: string, qty: number) => void;
  onCheckout: () => void;
  title?: string;
  emptyText?: string;
  totalLabel?: string;
  checkoutLabel?: string;
  clearLabel?: string;
}
export function CartDrawer({ isOpen, onClose, items, onUpdateQuantity, onCheckout, title, emptyText, totalLabel, checkoutLabel }: CartDrawerProps) {
  const { t } = useI18n();
  const total = items.reduce((sum, item) => sum + item.price * item.quantity, 0);

  return (
    <BottomSheet isOpen={isOpen} onClose={onClose} title={title || t('cart.title', 'Cart')}>
      <div className="flex flex-col h-[60vh] max-h-[500px]">
        <div className="flex-1 overflow-y-auto pb-4">
          {items.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full gap-3" style={{ color: 'var(--brand-text-muted)' }}>
              <i className="ti ti-shopping-cart text-3xl opacity-30" />
              <span className="text-sm">{emptyText || t('cart.empty', 'Cart is empty')}</span>
            </div>
          ) : (
            <div className="space-y-3">
              <AnimatePresence initial={false}>
                {items.map((item, idx) => (
                  <motion.div
                    key={item.id}
                    layout
                    initial={{ opacity: 0, x: -16, scale: 0.96 }}
                    animate={{ opacity: 1, x: 0, scale: 1, transition: { duration: 0.22, delay: idx * 0.04 } }}
                    exit={{ opacity: 0, x: 20, scale: 0.94, transition: { duration: 0.18 } }}
                    className="flex items-center justify-between py-1"
                  >
                    <div className="flex-1 min-w-0">
                      <div className="text-[var(--brand-text)] font-medium truncate">{item.name}</div>
                      <div className="text-[var(--brand-text-muted)] text-sm"><PriceDisplay amount={item.price} size="sm" /></div>
                    </div>
                    <div className="flex items-center gap-2 shrink-0">
                      <motion.button
                        type="button"
                        onClick={() => onUpdateQuantity(item.id, item.quantity - 1)}
                        className="min-w-[40px] min-h-[40px] rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] flex items-center justify-center"
                        whileTap={{ scale: 0.82 }}
                        whileHover={{ backgroundColor: 'var(--brand-border)' }}
                      >-</motion.button>
                      <motion.span
                        key={item.quantity}
                        initial={{ scale: 1.3 }}
                        animate={{ scale: 1 }}
                        transition={{ type: 'spring', stiffness: 300, damping: 20 }}
                        className="text-[var(--brand-text)] font-bold w-5 text-center"
                      >{item.quantity}</motion.span>
                      <motion.button
                        type="button"
                        onClick={() => onUpdateQuantity(item.id, item.quantity + 1)}
                        className="min-w-[40px] min-h-[40px] rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] flex items-center justify-center"
                        whileTap={{ scale: 0.82 }}
                        whileHover={{ backgroundColor: 'var(--brand-border)' }}
                      >+</motion.button>
                    </div>
                  </motion.div>
                ))}
              </AnimatePresence>
            </div>
          )}
        </div>
        
        {items.length > 0 && (
          <div className="pt-4 border-t border-[var(--brand-border)]">
            <div className="flex justify-between font-bold text-lg mb-4 text-[var(--brand-text)]">
              <span>{totalLabel || t('cart.total', 'Total')}</span>
              <PriceDisplay amount={total} size="lg" />
            </div>
            <Button data-testid="cart-checkout" className="w-full" size="lg" onClick={onCheckout}>
              {checkoutLabel || t('checkout.place_order', 'Checkout')}
            </Button>
          </div>
        )}
      </div>
    </BottomSheet>
  );
}
