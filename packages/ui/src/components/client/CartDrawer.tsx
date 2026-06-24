import React, { useEffect } from 'react';
import { createPortal } from 'react-dom';
import { motion, AnimatePresence, useReducedMotion } from 'framer-motion';
import { Button, useI18n, PriceDisplay } from '../../index.js';

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
  const reduce = useReducedMotion();
  const total = items.reduce((sum, item) => sum + item.price * item.quantity, 0);

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };
    document.addEventListener('keydown', onKey);
    document.body.style.overflow = 'hidden';
    return () => {
      document.removeEventListener('keydown', onKey);
      document.body.style.overflow = '';
    };
  }, [isOpen, onClose]);

  if (typeof document === 'undefined') return null;

  return createPortal(
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0 z-modal-backdrop" role="dialog" aria-modal="true" aria-label={title || t('cart.title', 'Cart')}>
          <motion.div
            className="absolute inset-0 bg-black/60 backdrop-blur-sm"
            role="button"
            tabIndex={0}
            aria-label={t('common.close', 'Close')}
            onClick={onClose}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.18, ease: [0.4, 0, 0.2, 1] }}
          />
          <motion.div
            className="absolute bottom-0 left-0 right-0 max-h-[85vh] flex flex-col bg-[var(--brand-surface)] rounded-t-2xl"
            style={{ boxShadow: 'var(--elev-4)', paddingBottom: 'var(--safe-bottom)' }}
            initial={reduce ? { opacity: 0 } : { y: '100%' }}
            animate={reduce ? { opacity: 1 } : { y: 0 }}
            exit={reduce ? { opacity: 0 } : { y: '100%' }}
            transition={reduce ? { duration: 0.18 } : { duration: 0.32, ease: [0.16, 1, 0.3, 1] }}
          >
            <div className="flex items-center justify-center pt-2 pb-1 shrink-0">
              <div className="w-10 h-1 rounded-full bg-[var(--brand-border)]" />
            </div>
            <div className="flex items-center justify-between px-5 pt-2 pb-3 shrink-0">
              <h2 className="text-lg font-heading font-semibold text-[var(--brand-text)] truncate">{title || t('cart.title', 'Cart')}</h2>
              <button
                type="button"
                onClick={onClose}
                aria-label={t('common.close', 'Close')}
                className="w-9 h-9 shrink-0 flex items-center justify-center rounded-full text-[var(--brand-text-muted)] transition-colors duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none hover:bg-[var(--brand-surface-raised)] hover:text-[var(--brand-text)] active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
              >
                <i className="ti ti-x" />
              </button>
            </div>

            <div className="flex-1 min-h-0 overflow-y-auto px-5">
              {items.length === 0 ? (
                <div className="flex flex-col items-center justify-center text-center py-16 gap-3">
                  <div className="w-16 h-16 rounded-full flex items-center justify-center bg-[var(--brand-surface-raised)]" style={{ boxShadow: 'var(--elev-1)' }}>
                    <i className="ti ti-shopping-cart text-2xl text-[var(--brand-text-muted)]" />
                  </div>
                  <div className="text-[var(--brand-text)] font-medium">{emptyText || t('cart.empty', 'Your cart is empty')}</div>
                  <p className="text-sm text-[var(--brand-text-muted)] max-w-[240px]">{t('cart.empty_hint', 'Add a few tasty items to get started.')}</p>
                </div>
              ) : (
                <ul className="space-y-2 py-1">
                  <AnimatePresence initial={false}>
                    {items.map((item, idx) => (
                      <motion.li
                        key={item.id}
                        layout={!reduce}
                        initial={reduce ? { opacity: 0 } : { opacity: 0, x: -12 }}
                        animate={reduce ? { opacity: 1 } : { opacity: 1, x: 0, transition: { duration: 0.22, ease: [0.16, 1, 0.3, 1], delay: Math.min(idx, 6) * 0.03 } }}
                        exit={reduce ? { opacity: 0 } : { opacity: 0, x: 16, height: 0, marginTop: 0, transition: { duration: 0.18, ease: [0.4, 0, 0.2, 1] } }}
                        className="flex items-center justify-between gap-3 py-2"
                      >
                        <div className="flex-1 min-w-0">
                          <div className="text-[var(--brand-text)] font-medium truncate">{item.name}</div>
                          <div className="text-[var(--brand-text-muted)] text-sm tabular-nums"><PriceDisplay amount={item.price} size="sm" /></div>
                        </div>
                        <div className="flex items-center gap-2 shrink-0">
                          <button
                            type="button"
                            aria-label={t('cart.decrease', 'Decrease quantity')}
                            onClick={() => onUpdateQuantity(item.id, item.quantity - 1)}
                            className="w-10 h-10 rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] flex items-center justify-center text-lg transition-[transform,background-color] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none hover:bg-[var(--brand-border)] active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                          >&minus;</button>
                          <motion.span
                            key={item.quantity}
                            initial={reduce ? false : { scale: 1.25 }}
                            animate={{ scale: 1 }}
                            transition={reduce ? { duration: 0 } : { type: 'spring', stiffness: 320, damping: 18 }}
                            className="text-[var(--brand-text)] font-bold w-6 text-center tabular-nums"
                          >{item.quantity}</motion.span>
                          <button
                            type="button"
                            aria-label={t('cart.increase', 'Increase quantity')}
                            onClick={() => onUpdateQuantity(item.id, item.quantity + 1)}
                            className="w-10 h-10 rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] flex items-center justify-center text-lg transition-[transform,background-color] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none hover:bg-[var(--brand-border)] active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                          >+</button>
                        </div>
                      </motion.li>
                    ))}
                  </AnimatePresence>
                </ul>
              )}
            </div>

            {items.length > 0 && (
              <div className="shrink-0 px-5 pt-3 pb-5 bg-[var(--brand-surface)]" style={{ boxShadow: '0 -8px 24px rgba(0,0,0,.06)' }}>
                <div className="flex items-center justify-between gap-3 font-bold text-lg mb-3 text-[var(--brand-text)]">
                  <span className="min-w-0 truncate">{totalLabel || t('cart.total', 'Total')}</span>
                  <span className="shrink-0 tabular-nums"><PriceDisplay amount={total} size="lg" /></span>
                </div>
                <Button data-testid="cart-checkout" className="w-full min-w-0" size="lg" onClick={onCheckout}>
                  <span className="truncate">{checkoutLabel || t('checkout.place_order', 'Checkout')}</span>
                </Button>
              </div>
            )}
          </motion.div>
        </div>
      )}
    </AnimatePresence>,
    document.body,
  );
}
