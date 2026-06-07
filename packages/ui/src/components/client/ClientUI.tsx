import React, { useState, useEffect } from 'react';
import { Button, Input, BottomSheet, Modal } from '../../index.js';
import { formatALL } from '@deliveryos/shared-types';

export interface CartItem {
  id: string;
  productId: string;
  name: string;
  quantity: number;
  price: number;
  options?: Record<string, string[]>;
}

// --- ProductCard ---
interface ProductCardProps {
  product: {
    id: string; name: string; description?: string; price: number; image?: string;
    isAvailable: boolean; tags?: string[];
    taste?: Record<string, number>;
    allergenStatus?: string;
    kcal?: number | null;
    protein?: number | null;
    fat?: number | null;
    carbs?: number | null;
  };
  onAdd: (e: React.MouseEvent) => void;
}

const TASTE_ICONS: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };

export function ProductCard({ product, onAdd }: ProductCardProps) {
  return (
    <article 
      className={`product-card rounded-[12px] flex flex-col cursor-pointer overflow-hidden border ${!product.isAvailable ? 'opacity-60' : ''}`}
      style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}
    >
      <div 
        className="w-full aspect-[4/3] flex items-center justify-center relative" 
        style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-border)' }}
      >
        {product.image ? (
          <img src={product.image} alt={product.name} className="w-full h-full object-cover" />
        ) : (
          <span className="text-[32px] font-bold">Img</span>
        )}
        {!product.isAvailable && (
          <>
            <div className="absolute inset-0 z-10" style={{ background: 'rgba(0,0,0,0.5)' }}></div>
            <div className="absolute inset-0 flex items-center justify-center z-20">
              <span className="text-[11px] px-2 py-1 rounded-[6px] font-medium" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}>
                Unavailable
              </span>
            </div>
          </>
        )}
      </div>
      <div className="p-3 flex flex-col flex-1">
        <h3 className="font-medium text-[14px] mb-1" style={{ color: 'var(--brand-text)' }}>{product.name}</h3>
        {product.description && (
          <p className="text-[12px] mb-2 line-clamp-2 overflow-hidden" style={{ color: 'var(--brand-text-muted)' }}>
            {product.description}
          </p>
        )}
        <div className="flex items-center justify-between mt-auto pt-2">
          <div className="flex items-baseline gap-2">
            <span className="font-bold text-[15px]" style={{ color: 'var(--brand-primary)' }}>{formatALL(product.price)}</span>
            {product.kcal != null && (
              <span className="text-[10px] font-medium" style={{ color: 'var(--brand-text-muted)' }}>
                ~{product.kcal} kcal
                {product.protein != null && <span className="ml-1 opacity-60">P:{product.protein}g</span>}
                {product.fat != null && <span className="ml-1 opacity-60">F:{product.fat}g</span>}
                {product.carbs != null && <span className="ml-1 opacity-60">C:{product.carbs}g</span>}
              </span>
            )}
          </div>
          <button 
            className={`min-w-[44px] min-h-[44px] flex items-center justify-center text-white rounded-[var(--brand-radius-btn)] ${product.isAvailable ? 'active:scale-[0.97]' : 'opacity-30 cursor-not-allowed'}`}
            style={{ background: 'var(--brand-primary)' }}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              if (product.isAvailable) onAdd(e);
            }}
            disabled={!product.isAvailable}
            aria-label="Add"
          >
            <span className="text-xl leading-none mb-1">+</span>
          </button>
        </div>
        {product.tags && product.tags.length > 0 && (
          <div className="flex gap-1 mt-1.5 flex-wrap">
            {product.tags.map(tag => (
              <span key={tag} className="px-1.5 py-0.5 rounded-[6px] text-[10px]" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                {tag}
              </span>
            ))}
          </div>
        )}
        {/* Allergen badge */}
        {product.allergenStatus === 'none' && (
          <div className="mt-1.5 flex items-center gap-1 text-[10px]" style={{ color: 'var(--color-success)' }}>✓ No allergens</div>
        )}
        {product.allergenStatus === 'listed' && product.tags && product.tags.length > 0 && (
          <div className="mt-1.5 flex items-center gap-1 text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>
            <span className="font-medium">Allergens:</span>
            {product.tags.slice(0, 4).map((tag: string) => (
              <span key={tag} className="px-1 py-0.5 rounded-[3px]" style={{ background: 'var(--brand-surface-raised)' }}>{tag}</span>
            ))}
            {product.tags.length > 4 && <span>+{product.tags.length - 4}</span>}
          </div>
        )}
        {/* Taste indicators */}
        {product.taste && Object.keys(product.taste).length > 0 && (
          <div className="flex gap-0.5 mt-1.5">
            {Object.entries(product.taste).map(([axis, level]) => (
              <span key={axis} className="inline-flex items-center gap-0.5 text-[10px] opacity-70" title={`${axis}: ${level}/3`}>
                <i className={TASTE_ICONS[axis] || 'ti ti-circle'} style={{ fontSize: '0.7rem' }} />{level}
              </span>
            ))}
          </div>
        )}
      </div>
    </article>
  );
}

// --- CartDrawer ---
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
  const total = items.reduce((sum, item) => sum + item.price * item.quantity, 0);

  return (
    <BottomSheet isOpen={isOpen} onClose={onClose} title={title || 'Cart'}>
      <div className="flex flex-col h-[60vh] max-h-[500px]">
        <div className="flex-1 overflow-y-auto pb-4">
          {items.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full gap-3" style={{ color: 'var(--brand-text-muted)' }}>
              <i className="ti ti-shopping-cart text-3xl opacity-30" />
              <span className="text-sm">{emptyText || 'Cart is empty'}</span>
            </div>
          ) : (
            <div className="space-y-4">
              {items.map(item => (
                <div key={item.id} className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="text-[var(--brand-text)] font-medium truncate">{item.name}</div>
                    <div className="text-[var(--brand-text-muted)] text-sm">{formatALL(item.price)}</div>
                  </div>
                  <div className="flex items-center gap-3 shrink-0">
                    <button onClick={() => onUpdateQuantity(item.id, item.quantity - 1)} className="w-8 h-8 rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)] transition-colors active:scale-95">-</button>
                    <span className="text-[var(--brand-text)] font-medium w-4 text-center">{item.quantity}</span>
                    <button onClick={() => onUpdateQuantity(item.id, item.quantity + 1)} className="w-8 h-8 rounded-full bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)] transition-colors active:scale-95">+</button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
        
        {items.length > 0 && (
          <div className="pt-4 border-t border-[var(--brand-border)]">
            <div className="flex justify-between font-bold text-lg mb-4 text-[var(--brand-text)]">
              <span>{totalLabel || 'Total'}</span>
              <span>{formatALL(total)}</span>
            </div>
            <Button className="w-full" size="lg" onClick={onCheckout}>
              {checkoutLabel || 'Checkout'}
            </Button>
          </div>
        )}
      </div>
    </BottomSheet>
  );
}

// --- CartFAB ---
export function CartFAB({ itemsCount, total, onClick, isBouncing = false }: { itemsCount: number; total: number; onClick: () => void; isBouncing?: boolean }) {
  if (itemsCount === 0) return null;
  return (
    <div className="fixed bottom-[80px] right-[20px] z-[100] embed-hidden">
      <button 
        id="cartFabBtn" 
        aria-label={`Cart: ${itemsCount} items, ${total} ALL`}
        className={`h-[48px] px-5 text-white text-[14px] font-medium flex items-center justify-center gap-1 ${isBouncing ? 'cart-bounce' : ''}`}
        style={{ 
          background: 'var(--brand-primary)', 
          borderRadius: 'var(--brand-radius-btn)', 
          boxShadow: '0 4px 12px color-mix(in srgb, var(--brand-primary) 40%, transparent)' 
        }} 
        onClick={onClick}
      >
        <i className="ti ti-shopping-cart text-lg leading-none" />
        <span className="mx-1 opacity-40">|</span>
        <span>{itemsCount}</span>
        <span className="mx-1 opacity-40">|</span>
        <span>{total}</span> ALL
      </button>
    </div>
  );
}

// --- OTPModal ---
interface OTPModalProps {
  isOpen: boolean;
  onClose: () => void;
  phone: string;
  onSendOTP: (phone: string) => Promise<void>;
  onVerifyOTP: (code: string) => Promise<void>;
}
export function OTPModal({ isOpen, onClose, phone, onSendOTP, onVerifyOTP }: OTPModalProps) {
  const [step, setStep] = useState<'phone' | 'code'>('phone');
  const [currentPhone, setCurrentPhone] = useState(phone);
  const [code, setCode] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (isOpen) {
      setStep('phone');
      setCurrentPhone(phone);
      setCode('');
      setError('');
    }
  }, [isOpen, phone]);

  const handleSend = async () => {
    try {
      setLoading(true);
      setError('');
      await onSendOTP(currentPhone);
      setStep('code');
    } catch (err: any) {
      setError(err.message || 'Failed to send OTP');
    } finally {
      setLoading(false);
    }
  };

  const handleVerify = async () => {
    try {
      setLoading(true);
      setError('');
      await onVerifyOTP(code);
      onClose();
    } catch (err: any) {
      setError(err.message || 'Invalid code');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={step === 'phone' ? 'Verify your phone' : 'Enter confirmation code'}>
      <div className="space-y-4">
        {step === 'phone' ? (
          <>
            <p className="text-sm text-[var(--brand-text-muted)]">We need to verify your phone number to proceed with the order.</p>
            <Input 
              type="tel" 
              value={currentPhone} 
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setCurrentPhone(e.target.value)} 
              placeholder="+355 6X XXX XXXX"
              error={!!error}
            />
            {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}
            <Button className="w-full" onClick={handleSend} isLoading={loading}>Send Code</Button>
          </>
        ) : (
          <>
            <p className="text-sm text-[var(--brand-text-muted)]">Code sent to {currentPhone}. <button className="text-[var(--brand-primary)] underline" onClick={() => setStep('phone')}>Edit</button></p>
            <Input 
              type="text" 
              maxLength={4}
              value={code} 
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setCode(e.target.value)} 
              placeholder="0000"
              className="text-center text-2xl tracking-widest"
              error={!!error}
            />
            {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}
            <Button className="w-full" onClick={handleVerify} isLoading={loading} disabled={code.length !== 4}>Verify & Complete Order</Button>
          </>
        )}
      </div>
    </Modal>
  );
}

// --- OrderProgress ---
export function OrderProgress({ status }: { status: string }) {
  const steps = [
    { key: 'PENDING', label: 'Received' },
    { key: 'PREPARING', label: 'Preparing' },
    { key: 'READY', label: 'Ready' },
    { key: 'IN_DELIVERY', label: 'On the way' },
    { key: 'DELIVERED', label: 'Delivered' },
  ];

  const currentIndex = steps.findIndex(s => s.key === status) >= 0 ? steps.findIndex(s => s.key === status) : 0;

  return (
    <div className="relative py-4">
      <div className="absolute top-1/2 left-0 right-0 h-1 bg-[var(--brand-surface-raised)] -translate-y-1/2 z-0" />
      <div 
        className="absolute top-1/2 left-0 h-1 bg-[var(--brand-primary)] -translate-y-1/2 z-0 transition-all duration-500" 
        style={{ width: `${(currentIndex / (steps.length - 1)) * 100}%` }}
      />
      <div className="relative z-10 flex justify-between">
        {steps.map((step, i) => {
          const isActive = i <= currentIndex;
          return (
            <div key={step.key} className="flex flex-col items-center">
              <div className={`w-4 h-4 rounded-full border-2 ${isActive ? 'bg-[var(--brand-primary)] border-[var(--brand-primary)]' : 'bg-[var(--brand-surface)] border-[var(--brand-border)]'} transition-colors duration-500`} />
              <span className={`text-[10px] mt-1 ${isActive ? 'text-[var(--brand-text)] font-semibold' : 'text-[var(--brand-text-muted)]'}`}>{step.label}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
