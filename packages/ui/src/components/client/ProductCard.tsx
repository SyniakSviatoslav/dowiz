import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { useI18n, PriceDisplay } from '../../index.js';
import { ease, duration } from '../../lib/motion.js';

interface ProductCardProps {
  product: {
    id: string; name: string; description?: string; price: number; image?: string;
    prepTimeMinutes?: number;
    isAvailable: boolean; tags?: string[];
    taste?: Record<string, number>;
    allergens?: string[];
    kcal?: number | null;
    protein?: number | null;
    fat?: number | null;
    carbs?: number | null;
    ingredients?: string[];
    chefPick?: boolean;
  };
  onAdd: (e: React.MouseEvent) => void;
  onClick?: (e: React.MouseEvent) => void;
  // When the parent overlays a compare-toggle on the card's top-left corner, photoless
  // cards (whose title sits at the top-left) must reserve a gutter so the toggle doesn't
  // cover the first characters of the name. Photo cards host the toggle over the image.
  compareGutter?: boolean;
  // P6-3 preview: suppress the "+" add affordance entirely for a never-orderable shadow preview.
  // The card stays fully browsable (tap → detail modal) but advertises no ordering action.
  hideAdd?: boolean;
}

const TASTE_ICONS: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };
const TASTE_LABELS: Record<string, string> = { spicy: 'Spicy', sweet: 'Sweet', salty: 'Salty', sour: 'Sour', richness: 'Rich' };

const cardVariants = {
  rest: { y: 0, boxShadow: '0 1px 4px rgba(0,0,0,0.06)', scale: 1 },
  hover: { y: -2, boxShadow: '0 8px 22px rgba(0,0,0,0.11)', scale: 1.005, transition: { duration: duration.fast, ease: ease.out } },
  tap: { scale: 0.98, y: -1, transition: { duration: duration.instant, ease: ease.out } },
};
const imgVariants = {
  rest: { scale: 1 },
  hover: { scale: 1.04, transition: { duration: duration.slow, ease: ease.out } },
};
const addBtnVariants = {
  rest: { scale: 1 },
  hover: { scale: 1.06, boxShadow: '0 4px 12px rgba(0,0,0,0.18)', transition: { duration: duration.fast, ease: ease.out } },
  tap: { scale: 0.96, transition: { duration: duration.instant } },
};

// Hover lift is for pointer devices only — on touch the "hover" state sticks after a
// tap, which reads as a stuck/janky card. Gate the lift behind a hover-capable pointer.
const canHover = typeof window !== 'undefined' && window.matchMedia?.('(hover: hover)').matches;

export function ProductCard({ product, onAdd, onClick, compareGutter, hideAdd }: ProductCardProps) {
  const { t } = useI18n();
  const [imgError, setImgError] = useState(false);
  const isChefPick = !!product.chefPick;
  // HIGH-1: photoless items render text-first (no fake placeholder slot), so the card
  // only reserves a photo area when there is a real photo to show.
  const hasPhoto = !!product.image && !imgError;
  // Photoless cards put the title at the top-left, exactly where the parent overlays the
  // compare-toggle — reserve a left gutter on the leading row so the name isn't clipped.
  const reserveGutter = !!compareGutter && !hasPhoto;
  // HIGH-2: the card carries ESSENTIALS only — name, price, short description, and at
  // most ONE taste cue (the dominant axis). The full taste profile, allergens, nutrition
  // and ingredients all live in the detail modal, so the grid stays scannable instead of
  // a wall of chips on every card.
  const dominantTaste = product.taste
    ? Object.entries(product.taste)
        .filter(([axis, v]) => v > 0 && TASTE_ICONS[axis])
        .sort((a, b) => b[1] - a[1])[0]
    : undefined;

  return (
    <motion.article
      data-testid="menu-item"
      className={`flex flex-col cursor-pointer overflow-hidden border rounded-xl h-full ${
        product.isAvailable ? '' : 'opacity-55'
      }`}
      style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}
      onClick={onClick}
      variants={product.isAvailable ? cardVariants : undefined}
      initial="rest"
      whileHover={product.isAvailable && canHover ? 'hover' : undefined}
      whileTap={product.isAvailable && onClick ? 'tap' : undefined}
    >
      {/* HIGH-1: photo cards keep the image; photoless items render TEXT-FIRST with no
          fake placeholder slot. Most menu items have no photo, and an identical cutlery
          medallion on every one turned the grid into a sea of placeholders — so without a
          real photo the name, price and description simply take the lead instead. */}
      {hasPhoto && (
        <div
          className="w-full aspect-[4/3] flex items-center justify-center relative overflow-hidden"
          style={{ background: 'var(--brand-surface-raised)' }}
        >
          <motion.img
            src={product.image}
            alt={product.name}
            className="w-full h-full object-cover"
            loading="lazy"
            onError={() => setImgError(true)}
            variants={product.isAvailable ? imgVariants : undefined}
          />
          {/* Chef-pick is the one curation cue that stays ON the photo. Allergens,
              nutrition and ingredient chips moved into the detail modal (HIGH-2). */}
          {isChefPick && (
            <div className="absolute top-1.5 right-1.5 z-10">
              <motion.span
                className="text-step-2xs font-bold px-1.5 py-0.5 rounded-md flex items-center gap-0.5"
                style={{ background: 'var(--brand-primary)', color: 'color-mix(in srgb, var(--brand-bg) 88%, #000)', boxShadow: '0 2px 8px color-mix(in srgb, var(--brand-primary) 45%, transparent)' }}
                initial={{ scale: 0.85, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                transition={{ type: 'spring', stiffness: 300, damping: 20 }}
              >
                ✦ {t('client.chefs_pick_badge', "Chef's Pick")}
              </motion.span>
            </div>
          )}
          {/* No sold-out overlay/chip: read_public_menu only returns is_available=true products,
              so the storefront hides unavailable items rather than greying them. */}
        </div>
      )}
      <div className={`flex flex-col flex-1 gap-1 min-h-0 ${hasPhoto ? 'p-2.5' : 'p-3.5'}`}>
        {/* Photoless cards have no image corner to host the chef-pick cue, so it surfaces inline. */}
        {!hasPhoto && isChefPick && (
          <span className={`self-start text-step-2xs font-bold px-1.5 py-0.5 rounded-md inline-flex items-center gap-0.5 mb-0.5 ${reserveGutter ? 'ml-7' : ''}`} style={{ background: 'color-mix(in srgb, var(--brand-primary) 14%, transparent)', color: 'var(--brand-primary-readable)' }}>
            ✦ {t('client.chefs_pick_badge', "Chef's Pick")}
          </span>
        )}
        <div className={`flex items-start justify-between gap-1.5 ${reserveGutter && !isChefPick ? 'pl-7' : ''}`}>
          <h3 className={`font-semibold leading-tight line-clamp-2 flex-1 ${hasPhoto ? 'text-step-sm min-h-[2.5em]' : 'text-step-base'}`} style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{product.name}</h3>
          {!hideAdd && (
          <motion.button
            data-testid="menu-item-add"
            className={`shrink-0 min-w-[44px] min-h-[44px] flex items-center justify-center text-[var(--brand-bg)] rounded-full mt-0.5 ${
              product.isAvailable ? 'cursor-pointer' : 'opacity-30 cursor-not-allowed'
            }`}
            style={{ background: 'var(--brand-primary)', boxShadow: '0 2px 8px rgba(0,0,0,0.12)' }}
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              if (product.isAvailable) onAdd(e);
            }}
            disabled={!product.isAvailable}
            aria-label={t('tooltip.add_to_cart', 'Add to cart')}
            title={t('tooltip.add_to_cart', 'Add to cart')}
            variants={product.isAvailable ? addBtnVariants : undefined}
            whileTap={product.isAvailable ? 'tap' : undefined}
          >
            <i className="ti ti-plus text-lg leading-none" />
          </motion.button>
          )}
        </div>
        {product.description && (
          // Photoless cards earn an extra description line since they don't spend height on an image.
          <p className={`text-step-2xs leading-snug ${hasPhoto ? 'line-clamp-2' : 'line-clamp-3'}`} style={{ color: 'var(--brand-text-muted)', fontFamily: 'var(--brand-font-body)' }}>
            {product.description}
          </p>
        )}

        {/* HIGH-2: full ingredients / allergen / nutrition detail intentionally omitted from
            the card — it all lives one tap away in the detail modal. */}

        {/* Footer: price + meta. flex-wrap so on a narrow 2-col card the meta drops to its own
            line instead of colliding with a wide price (the "1500 ALL~15 min" overlap bug). */}
        <div className="flex flex-wrap items-baseline justify-between gap-x-2 gap-y-1 mt-auto pt-1.5">
          <PriceDisplay amount={product.price} size="md" style={{ color: 'var(--brand-primary-readable, var(--brand-text))', fontWeight: 800 }} />
          {/* Meta rail: dominant taste cue + COOKING-time (not delivery). shrink-0 + nowrap keeps
              it intact; when it can't fit beside the price, flex-wrap moves the whole rail below. */}
          <div className="flex items-center gap-1.5 shrink-0 whitespace-nowrap">
            {/* HIGH-2: a single dominant taste cue — the full taste profile is in the modal. */}
            {dominantTaste && (
              <span
                className="inline-flex items-center"
                style={{ color: 'color-mix(in srgb, var(--brand-text) 62%, transparent)' }}
                title={TASTE_LABELS[dominantTaste[0]] || dominantTaste[0]}
                aria-label={TASTE_LABELS[dominantTaste[0]] || dominantTaste[0]}
              >
                <i className={TASTE_ICONS[dominantTaste[0]]} style={{ fontSize: '0.85rem' }} />
              </span>
            )}
            {product.prepTimeMinutes != null && (
              <span
                className="inline-flex items-center gap-0.5 text-step-2xs font-medium whitespace-nowrap"
                style={{ color: 'var(--brand-text-muted)' }}
                title={t('product.prep_cooking_time', 'Cooking time (not delivery)')}
                aria-label={t('product.prep_cooking_time', 'Cooking time (not delivery)')}
              >
                <i className="ti ti-tools-kitchen-2" style={{ fontSize: '0.7rem' }} aria-hidden="true" />
                {t('product.prep_minutes', '~{{n}} min', { n: product.prepTimeMinutes })}
              </span>
            )}
          </div>
        </div>
      </div>
    </motion.article>
  );
}
