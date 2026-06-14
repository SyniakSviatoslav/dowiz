import React from 'react';
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
            <div className="space-y-4">
              {items.map(item => (
                <div key={item.id} className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="text-[var(--brand-text)] font-medium truncate">{item.name}</div>
                    <div className="text-[var(--brand-text-muted)] text-sm"><PriceDisplay amount={item.price} size="sm" /></div>
                  </div>
                  <div className="flex items-center gap-3 shrink-0">
                    <button onClick={() => onUpdateQuantity(item.id, item.quantity - 1)} className="min-w-[44px] min-h-[44px] rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)] transition-colors active:scale-95">-</button>
                    <span className="text-[var(--brand-text)] font-medium w-4 text-center">{item.quantity}</span>
                    <button onClick={() => onUpdateQuantity(item.id, item.quantity + 1)} className="min-w-[44px] min-h-[44px] rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)] transition-colors active:scale-95">+</button>
                  </div>
                </div>
              ))}
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
