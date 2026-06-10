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
    allergens?: string[];
    kcal?: number | null;
    protein?: number | null;
    fat?: number | null;
    carbs?: number | null;
    ingredients?: string[];
  };
  onAdd: (e: React.MouseEvent) => void;
  onClick?: (e: React.MouseEvent) => void;
}

const TASTE_ICONS: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };
const TASTE_LABELS: Record<string, string> = { spicy: 'Spicy', sweet: 'Sweet', salty: 'Salty', sour: 'Sour', richness: 'Rich' };
const ALLERGEN_COLORS: Record<string, { bg: string; text: string }> = {
  gluten: { bg: 'rgba(234,179,8,0.12)', text: '#a16207' },
  dairy: { bg: 'rgba(59,130,246,0.12)', text: '#1d4ed8' },
  eggs: { bg: 'rgba(234,179,8,0.12)', text: '#a16207' },
  soy: { bg: 'rgba(34,197,94,0.12)', text: '#15803d' },
  nuts: { bg: 'rgba(249,115,22,0.12)', text: '#c2410c' },
  peanuts: { bg: 'rgba(249,115,22,0.12)', text: '#c2410c' },
  shellfish: { bg: 'rgba(239,68,68,0.12)', text: '#b91c1c' },
  fish: { bg: 'rgba(6,182,212,0.12)', text: '#0e7490' },
  sesame: { bg: 'rgba(168,85,247,0.12)', text: '#7e22ce' },
};

function getAllergenStyle(allergen: string) {
  const key = allergen.toLowerCase();
  return ALLERGEN_COLORS[key] || { bg: 'rgba(107,114,128,0.12)', text: '#374151' };
}

export function ProductCard({ product, onAdd, onClick }: ProductCardProps) {
  const hasAllergens = product.allergens && product.allergens.length > 0;
  const hasIngredients = product.ingredients && product.ingredients.length > 0;
  const hasTaste = product.taste && Object.keys(product.taste).length > 0;
  const hasNutrition = product.kcal != null;
  const allergens = product.allergens || [];
  const ingredients = product.ingredients || [];

  return (
    <article 
      className={`product-card rounded-xl flex flex-col cursor-pointer overflow-hidden border transition-all duration-150 ease-in-out h-full ${
        product.isAvailable ? 'hover:-translate-y-0.5 hover:shadow-lg active:scale-[0.98]' : 'opacity-55'
      }`}
      style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}
      onClick={onClick}
    >
      <div 
        className="w-full aspect-[4/3] flex items-center justify-center relative overflow-hidden" 
        style={{ background: 'var(--brand-surface-raised)' }}
      >
        {product.image ? (
          <img src={product.image} alt={product.name} className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105" loading="lazy" />
        ) : (
          <div className="flex flex-col items-center gap-1.5" style={{ color: 'var(--brand-text-muted)' }}>
            <i className="ti ti-tools-kitchen-2 text-4xl opacity-25" />
          </div>
        )}
        {hasAllergens && (
          <div className="absolute top-1.5 left-1.5 z-10 flex gap-0.5">
            <span className="text-[8px] font-semibold px-1.5 py-0.5 rounded-md flex items-center gap-0.5" style={{ background: 'rgba(220,38,38,0.15)', color: 'var(--color-danger)' }}>
              <i className="ti ti-alert-triangle" style={{ fontSize: '0.55rem' }} />
              {allergens.length === 1 ? allergens[0] : `${allergens.length}`}
            </span>
          </div>
        )}
        {!hasAllergens && product.isAvailable && (
          <div className="absolute top-1.5 left-1.5 z-10">
            <span className="text-[8px] font-medium px-1.5 py-0.5 rounded-md flex items-center gap-0.5" style={{ background: 'rgba(5,150,105,0.12)', color: 'var(--color-success)' }}>
              <i className="ti ti-circle-check" style={{ fontSize: '0.55rem' }} />
              Clean
            </span>
          </div>
        )}
        {hasNutrition && (
          <div className="absolute top-1.5 right-1.5 z-10">
            <span className="text-[9px] font-semibold px-1.5 py-0.5 rounded-md flex items-center gap-1" style={{ background: 'rgba(0,0,0,0.6)', color: '#fff' }}>
              <i className="ti ti-flame" style={{ fontSize: '0.6rem' }} />
              {product.kcal}
            </span>
          </div>
        )}
        {!product.isAvailable && (
          <>
            <div className="absolute inset-0 z-10" style={{ background: 'linear-gradient(to top, rgba(0,0,0,0.7) 0%, rgba(0,0,0,0.3) 100%)' }} />
            <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-20">
              <span className="text-[10px] font-semibold px-2.5 py-1 rounded-md" style={{ background: 'var(--color-danger)', color: '#fff' }}>
                Unavailable
              </span>
            </div>
          </>
        )}
      </div>
      <div className="p-2.5 flex flex-col flex-1 gap-1 min-h-0">
        <div className="flex items-start justify-between gap-1.5">
          <h3 className="font-semibold text-[13px] leading-tight line-clamp-2 flex-1" style={{ color: 'var(--brand-text)' }}>{product.name}</h3>
          <button 
            className={`shrink-0 w-[32px] h-[32px] flex items-center justify-center text-white rounded-full transition-all duration-150 ease-in-out mt-0.5 ${
              product.isAvailable 
                ? 'hover:brightness-110 hover:scale-110 active:scale-[0.88]' 
                : 'opacity-30 cursor-not-allowed'
            }`}
            style={{ background: 'var(--brand-primary)', boxShadow: '0 2px 8px rgba(0,0,0,0.12)' }}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              if (product.isAvailable) onAdd(e);
            }}
            disabled={!product.isAvailable}
            aria-label="Add to cart"
          >
            <i className="ti ti-plus text-sm leading-none" />
          </button>
        </div>
        {product.description && (
          <p className="text-[10px] leading-snug line-clamp-2" style={{ color: 'var(--brand-text-muted)' }}>
            {product.description}
          </p>
        )}

        {hasIngredients && (
          <div className="flex gap-0.5 flex-wrap">
            {ingredients.slice(0, 4).map((ing, i) => (
              <span key={i} className="px-1 py-0 rounded text-[9px] leading-tight" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                {ing}
              </span>
            ))}
            {ingredients.length > 4 && <span className="text-[9px]" style={{ color: 'var(--brand-text-muted)' }}>+{ingredients.length - 4}</span>}
          </div>
        )}

        {hasAllergens && (
          <div className="flex gap-0.5 flex-wrap">
            {allergens.map(a => {
              const s = getAllergenStyle(a);
              return (
                <span key={a} className="px-1 py-0 rounded text-[8px] font-semibold uppercase leading-tight" style={{ background: s.bg, color: s.text }}>
                  {a}
                </span>
              );
            })}
          </div>
        )}

        <div className="flex items-center justify-between mt-auto pt-1">
          <div className="flex items-baseline gap-1">
            <span className="font-bold text-sm" style={{ color: 'var(--brand-primary)' }}>{formatALL(product.price)}</span>
            {hasNutrition && (
              <span className="text-[8px]" style={{ color: 'var(--brand-text-muted)' }}>
                {product.kcal}kcal
                {product.protein != null && <span className="opacity-60"> · P{product.protein}g</span>}
                {product.fat != null && <span className="opacity-60"> · F{product.fat}g</span>}
              </span>
            )}
          </div>
        </div>

        {hasTaste && (
          <div className="flex gap-1">
            {Object.entries(product.taste!).filter(([, v]) => v > 0).map(([axis, level]) => (
              <span key={axis} className="inline-flex items-center gap-0.5 text-[9px] leading-tight" style={{ color: 'var(--brand-text-muted)' }} title={`${TASTE_LABELS[axis] || axis}: ${level}/3`}>
                <i className={TASTE_ICONS[axis] || 'ti ti-circle'} style={{ fontSize: '0.6rem' }} />
                {level}
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
              maxLength={6}
              value={code} 
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setCode(e.target.value)} 
              placeholder="000000"
              className="text-center text-2xl tracking-widest"
              error={!!error}
            />
            {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}
            <Button className="w-full" onClick={handleVerify} isLoading={loading} disabled={code.length !== 6}>Verify & Complete Order</Button>
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
