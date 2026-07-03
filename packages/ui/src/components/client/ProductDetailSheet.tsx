import React from 'react';
import { AnimatePresence, m, useReducedMotion } from 'framer-motion';
import { useI18n } from '../../lib/I18nProvider.js';
import { PriceDisplay } from '../atoms/PriceDisplay.js';
import { ease } from '../../lib/motion.js';
import { cinematicRevealsEnabled, revealLayoutId } from '../../lib/cinematic.js';

/*
 * ProductDetailSheet — the card→detail shared-element (`layoutId`) reveal, extracted as a
 * reusable UI atom (the detail modal currently lives inline in apps/web MenuPage.tsx).
 *
 * The INCOMING half of the morph: the hero image, title and price carry the SAME `layoutId`s
 * as their ProductCard counterparts (via `revealLayoutId`), so the tapped card's image/title/
 * price visibly FLY into the sheet instead of a disconnected bottom-sheet pop.
 *
 * FLAG-DARK (VITE_CINEMATIC_REVEALS, default OFF) + reduced-motion:
 *   - flag off / reduced motion → `layoutId` is undefined on every node → no shared-layout
 *     projection → identical to the existing opacity + translateY bottom-sheet behaviour.
 *   - reduced motion additionally zeroes the enter transform (opacity-only), matching the
 *     `prefersReduced` branch the inline modal already ships.
 *
 * WIRING (lead, MenuPage.tsx — I do NOT edit that hotspot from this lane): replace the inline
 * `<AnimatePresence>{detailProduct && (…sheet…)}` block's OUTER chrome + hero/title/price header
 * with this component, and pass the remaining detail body (taste / nutrition / modifiers / the
 * sticky confirm bar) as `children`. Pass `sharedLayout` so the flag governs both halves in one
 * place, and ensure ProductCard receives the same `sharedLayout` so the ids match.
 */

export interface ProductDetailSheetProduct {
  id: string;
  name: string;
  /** Total price to show in the header (base + any modifier delta the caller already applied). */
  price: number;
  /** Primary poster/image url; the morph targets this node. Omit for the no-photo fallback. */
  image?: string;
  available?: boolean;
}

export interface ProductDetailSheetProps {
  product: ProductDetailSheetProduct | null;
  open: boolean;
  onClose: () => void;
  /** Force the shared-element morph on/off. Defaults to the VITE_CINEMATIC_REVEALS flag. */
  sharedLayout?: boolean;
  /** Optional slot rendered in the hero instead of the default <img> (e.g. lazy MediaGallery). */
  hero?: React.ReactNode;
  /** The detail body: description, taste, nutrition, modifiers, sticky confirm bar, etc. */
  children?: React.ReactNode;
}

export function ProductDetailSheet({ product, open, onClose, sharedLayout, hero, children }: ProductDetailSheetProps) {
  const { t } = useI18n();
  const prefersReduced = useReducedMotion();
  const morph = (sharedLayout ?? cinematicRevealsEnabled()) && !prefersReduced;
  const id = product?.id;
  // layoutDependency pins re-measurement to open/close (the product id), never every render —
  // the hotspot guard from the plan so the morph nodes don't re-project on unrelated renders.
  const layoutDep = id;

  return (
    <AnimatePresence>
      {open && product && (
        <m.div
          key="product-detail-sheet"
          data-testid="product-detail-sheet"
          className="fixed inset-0 z-modal flex items-end md:items-center justify-center"
          style={{ background: 'color-mix(in srgb, var(--brand-bg) 60%, transparent)', backdropFilter: 'blur(4px)' }}
          role="dialog"
          aria-modal="true"
          aria-label={product.name}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: prefersReduced ? 0 : 0.22, ease: ease.out }}
        >
          <button type="button" className="absolute inset-0 cursor-default" aria-label={t('common.close', 'Close')} onClick={onClose} />
          <m.div
            className="relative w-full md:max-w-lg max-h-[85vh] overflow-auto rounded-t-2xl md:rounded-2xl"
            style={{ background: 'var(--brand-bg)', boxShadow: 'var(--elev-4)' }}
            // When the shared-element morph is on, framer drives the geometry from the card, so
            // we skip the translateY/scale enter (it would fight the projection). Off/reduced →
            // keep the existing bottom-sheet rise verbatim.
            initial={morph ? { opacity: 0 } : prefersReduced ? { opacity: 0 } : { transform: 'translateY(28px) scale(0.97)', opacity: 0 }}
            animate={morph ? { opacity: 1 } : { transform: 'translateY(0px) scale(1)', opacity: 1 }}
            exit={prefersReduced || morph ? { opacity: 0 } : { transform: 'translateY(18px) scale(0.97)', opacity: 0, transition: { duration: 0.18, ease: ease.soft } }}
            transition={prefersReduced ? { duration: 0.15 } : { type: 'spring', stiffness: 340, damping: 32 }}
          >
            {/* Sticky dismiss bar — always reachable while the sheet scrolls. */}
            <div className="sticky top-0 z-30 h-0 pointer-events-none">
              <m.button
                type="button"
                whileTap={prefersReduced ? undefined : { scale: 0.95 }}
                onClick={onClose}
                aria-label={t('common.close', 'Close')}
                data-testid="product-detail-close"
                className="pointer-events-auto absolute top-3 right-3 min-w-[56px] min-h-[56px] rounded-full flex items-center justify-center outline-none backdrop-blur-sm focus-visible:ring-2 focus-visible:ring-white/80 focus-visible:ring-offset-2"
                style={{ background: 'rgba(0,0,0,0.68)', color: '#ffffff', border: '1.5px solid rgba(255,255,255,0.42)', boxShadow: '0 6px 20px rgba(0,0,0,0.5)' }}
              >
                <i className="ti ti-x text-3xl" />
              </m.button>
            </div>

            {/* Hero — the shared-element morph target. `hero` slot wins (rich media); else the
                poster image (with layoutId) or the on-brand no-photo medallion fallback. */}
            <div data-testid="product-detail-hero" className="relative w-full overflow-hidden" style={{ background: 'var(--brand-surface-raised)' }}>
              {hero ? (
                hero
              ) : product.image ? (
                <m.img
                  layoutId={morph && id ? revealLayoutId('media', id) : undefined}
                  layoutDependency={layoutDep}
                  src={product.image}
                  alt={product.name}
                  className="block w-full h-auto max-h-[68vh] object-contain"
                />
              ) : (
                <div
                  className="relative w-full aspect-[4/3] md:aspect-[16/10] flex flex-col items-center justify-center gap-3 select-none"
                  style={{ background: 'linear-gradient(135deg, color-mix(in srgb, var(--brand-primary) 16%, var(--brand-surface)) 0%, var(--brand-surface-raised) 55%, color-mix(in srgb, var(--brand-primary) 8%, var(--brand-surface)) 100%)' }}
                >
                  <span
                    className="relative flex items-center justify-center rounded-full"
                    style={{ width: 'clamp(3.5rem, 16%, 5rem)', aspectRatio: '1 / 1', background: 'color-mix(in srgb, var(--brand-surface) 78%, transparent)', border: '1px solid color-mix(in srgb, var(--brand-primary) 28%, transparent)', boxShadow: '0 2px 14px color-mix(in srgb, var(--brand-primary) 18%, transparent)' }}
                  >
                    <i className="ti ti-tools-kitchen-2 leading-none" style={{ fontSize: '1.75rem', color: 'var(--brand-primary)' }} />
                  </span>
                </div>
              )}
            </div>

            {/* Header: title + price carry the matching shared ids. Content rises after the hero. */}
            <m.div
              className="p-5 space-y-5"
              initial={prefersReduced ? false : { opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={prefersReduced ? { duration: 0 } : { delay: 0.1, duration: 0.32, ease: ease.out }}
            >
              <div className="flex items-start justify-between gap-3">
                <m.h2
                  layoutId={morph && id ? revealLayoutId('title', id) : undefined}
                  layoutDependency={layoutDep}
                  className="text-xl font-bold leading-tight flex-1 min-w-0"
                  style={{ color: 'var(--brand-text)' }}
                >
                  {product.name}
                </m.h2>
                <m.div
                  layoutId={morph && id ? revealLayoutId('price', id) : undefined}
                  layoutDependency={layoutDep}
                  className="text-xl font-black whitespace-nowrap shrink-0"
                  style={{ color: 'var(--brand-primary-readable, var(--brand-text))' }}
                >
                  <PriceDisplay amount={product.price} />
                </m.div>
              </div>
              {children}
            </m.div>
          </m.div>
        </m.div>
      )}
    </AnimatePresence>
  );
}
