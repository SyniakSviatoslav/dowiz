import React, { useState } from 'react';
<<<<<<< Updated upstream
import { motion } from 'framer-motion';
import { useI18n, PriceDisplay, getAllergenStyle } from '../../index.js';
=======
import { m, useReducedMotion } from 'framer-motion';
import { useI18n, PriceDisplay } from '../../index.js';
>>>>>>> Stashed changes
import { ease, duration } from '../../lib/motion.js';
import { cinematicRevealsEnabled, revealLayoutId } from '../../lib/cinematic.js';

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
<<<<<<< Updated upstream
=======
  // When the parent overlays a compare-toggle on the card's top-left corner, photoless
  // cards (whose title sits at the top-left) must reserve a gutter so the toggle doesn't
  // cover the first characters of the name. Photo cards host the toggle over the image.
  compareGutter?: boolean;
  // P6-3 preview: suppress the "+" add affordance entirely for a never-orderable shadow preview.
  // The card stays fully browsable (tap → detail modal) but advertises no ordering action.
  hideAdd?: boolean;
  // Cinematic reveals (flag: VITE_CINEMATIC_REVEALS): when true, the image/title/price get a
  // shared `layoutId` so they morph into the detail sheet on tap. Defaults to the flag so the
  // card is dark by default and reusable elsewhere without an accidental morph. Reduced-motion
  // is a second gate applied inside — accessibility wins even when this is on.
  sharedLayout?: boolean;
>>>>>>> Stashed changes
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

<<<<<<< Updated upstream
export function ProductCard({ product, onAdd, onClick }: ProductCardProps) {
  const { t } = useI18n();
  const [imgError, setImgError] = useState(false);
  const hasAllergens = product.allergens && product.allergens.length > 0;
  const hasIngredients = product.ingredients && product.ingredients.length > 0;
  const hasTaste = product.taste && Object.keys(product.taste).length > 0;
  const hasNutrition = product.kcal != null;
  const allergens = product.allergens || [];
  const ingredients = product.ingredients || [];
=======
export function ProductCard({ product, onAdd, onClick, compareGutter, hideAdd, sharedLayout }: ProductCardProps) {
  const { t } = useI18n();
  const [imgError, setImgError] = useState(false);
  const prefersReduced = useReducedMotion();
  // Attach the shared-element ids only when the flag is on AND motion is allowed. Under
  // reduced motion we drop the layoutId entirely → no FLIP morph, the detail sheet opens
  // with its opacity-only fallback (same as flag-off). `sharedLayout ?? flag` keeps the
  // card dark by default while letting a call site force it on/off.
  const morph = (sharedLayout ?? cinematicRevealsEnabled()) && !prefersReduced;
>>>>>>> Stashed changes
  const isChefPick = !!product.chefPick;

  return (
    <m.article
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
<<<<<<< Updated upstream
      <div
        className="w-full aspect-[4/3] flex items-center justify-center relative overflow-hidden"
        style={{ background: 'var(--brand-surface-raised)' }}
      >
        {product.image && !imgError ? (
          <motion.img
            layoutId={`product-photo-${product.id}`}
=======
      {/* HIGH-1: photo cards keep the image; photoless items render TEXT-FIRST with no
          fake placeholder slot. Most menu items have no photo, and an identical cutlery
          medallion on every one turned the grid into a sea of placeholders — so without a
          real photo the name, price and description simply take the lead instead. */}
      {hasPhoto && (
        <div
          className="w-full aspect-[4/3] flex items-center justify-center relative overflow-hidden"
          style={{ background: 'var(--brand-surface-raised)' }}
        >
          <m.img
>>>>>>> Stashed changes
            src={product.image}
            alt={product.name}
            className="w-full h-full object-cover"
            loading="lazy"
            onError={() => setImgError(true)}
            variants={product.isAvailable ? imgVariants : undefined}
            layoutId={morph ? revealLayoutId('media', product.id) : undefined}
          />
<<<<<<< Updated upstream
        ) : (
          // Crafted, on-brand no-photo fallback. Not a dead grey box and not a
          // giant monogram: a warm brand-tinted gradient, a faint repeating
          // dotted "tablecloth" texture, and a centred cutlery glyph in a soft
          // brand-coloured medallion. Themed per tenant via --brand-* tokens.
          <div
            className="flex items-center justify-center w-full h-full select-none relative"
            style={{
              background:
                'linear-gradient(135deg, color-mix(in srgb, var(--brand-primary) 14%, var(--brand-surface)) 0%, var(--brand-surface-raised) 55%, color-mix(in srgb, var(--brand-primary) 7%, var(--brand-surface)) 100%)',
            }}
            aria-hidden="true"
          >
            <div
              className="absolute inset-0"
              style={{
                backgroundImage:
                  'radial-gradient(color-mix(in srgb, var(--brand-primary) 30%, transparent) 1px, transparent 1.4px)',
                backgroundSize: '14px 14px',
                opacity: 0.35,
              }}
            />
            <span
              className="relative flex items-center justify-center rounded-full"
              style={{
                width: 'clamp(2.75rem, 22%, 3.75rem)',
                aspectRatio: '1 / 1',
                background: 'color-mix(in srgb, var(--brand-surface) 78%, transparent)',
                border: '1px solid color-mix(in srgb, var(--brand-primary) 28%, transparent)',
                boxShadow: '0 2px 10px color-mix(in srgb, var(--brand-primary) 16%, transparent)',
              }}
            >
              <i
                className="ti ti-tools-kitchen-2 leading-none"
                style={{ fontSize: 'clamp(1.25rem, 9vw, 1.75rem)', color: 'var(--brand-primary)' }}
              />
            </span>
          </div>
        )}
        {hasAllergens && (
          <div className="absolute top-1.5 left-1.5 z-10 flex flex-wrap gap-0.5 max-w-[70%]">
            {allergens.slice(0, 3).map(a => {
              const s = getAllergenStyle(a);
              return (
                <span key={a} className="text-step-2xs font-semibold px-1 py-0.5 rounded-sm leading-tight" style={{ background: s.bg, color: s.text }}>
                  {t(`allergen.${a.toLowerCase()}`, a)}
                </span>
              );
            })}
            {allergens.length > 3 && (
              <span className="text-step-2xs font-semibold px-1 py-0.5 rounded-sm" style={{ background: 'color-mix(in srgb, var(--brand-surface) 84%, #000)', color: 'var(--brand-text)' }}>
                +{allergens.length - 3}
              </span>
            )}
          </div>
        )}
        {/* No "Clean/allergen-free" fallback badge: absence of declared allergen
            data is NOT a safety guarantee. Showing it on every product with no
            allergen info was both misleading (e.g. a salmon roll reading "Clean")
            and visual noise. Allergen scent now appears only when real data exists. */}
        {hasNutrition && !isChefPick && (
          <div className="absolute top-1.5 right-1.5 z-10">
              <span className="text-step-2xs font-semibold px-1.5 py-0.5 rounded-md flex items-center gap-1" style={{ background: 'rgba(0,0,0,0.6)', color: 'var(--color-on-primary)' }}>
              <i className="ti ti-flame" style={{ fontSize: '0.6rem' }} />
              {product.kcal}
            </span>
          </div>
        )}
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
      <div className="p-2.5 flex flex-col flex-1 gap-1 min-h-0">
        <div className="flex items-start justify-between gap-1.5">
          <h3 className="font-semibold text-step-sm leading-tight line-clamp-2 flex-1 min-h-[2.5em]" style={{ color: 'var(--brand-text)' }}>{product.name}</h3>
          <motion.button
=======
          {/* Chef-pick is the one curation cue that stays ON the photo. Allergens,
              nutrition and ingredient chips moved into the detail modal (HIGH-2). */}
          {isChefPick && (
            <div className="absolute top-1.5 right-1.5 z-10">
              <m.span
                className="text-step-2xs font-bold px-1.5 py-0.5 rounded-md flex items-center gap-0.5"
                style={{ background: 'var(--brand-primary)', color: 'color-mix(in srgb, var(--brand-bg) 88%, #000)', boxShadow: '0 2px 8px color-mix(in srgb, var(--brand-primary) 45%, transparent)' }}
                initial={{ scale: 0.85, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                transition={{ type: 'spring', stiffness: 300, damping: 20 }}
              >
                ✦ {t('client.chefs_pick_badge', "Chef's Pick")}
              </m.span>
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
          <m.h3 layoutId={morph ? revealLayoutId('title', product.id) : undefined} className={`font-semibold leading-tight line-clamp-2 flex-1 ${hasPhoto ? 'text-step-sm min-h-[2.5em]' : 'text-step-base'}`} style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{product.name}</m.h3>
          {!hideAdd && (
          <m.button
>>>>>>> Stashed changes
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
<<<<<<< Updated upstream
            <i className="ti ti-plus text-sm leading-none" />
          </motion.button>
=======
            <i className="ti ti-plus text-lg leading-none" />
          </m.button>
          )}
>>>>>>> Stashed changes
        </div>
        {product.description && (
          <p className="text-step-2xs leading-snug line-clamp-2" style={{ color: 'var(--brand-text-muted)' }}>
            {product.description}
          </p>
        )}

        {hasIngredients && (
          <div className="flex gap-0.5 flex-wrap">
            {ingredients.slice(0, 4).map((ing, i) => (
              <span key={i} className="px-1 py-0 rounded text-step-2xs leading-tight" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                {ing}
              </span>
            ))}
            {ingredients.length > 4 && <span className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>+{ingredients.length - 4}</span>}
          </div>
        )}

<<<<<<< Updated upstream
        {/* Allergen scent lives on the image corner badges (top-3 + overflow) and
            the full labelled list is in the detail modal — rendering the loud
            uppercase colour row again here was redundant noise that broke the
            card's palette discipline, so it's intentionally omitted. */}

        <div className="flex items-center justify-between mt-auto pt-1">
          <div className="flex items-baseline gap-1">
            <PriceDisplay amount={product.price} size="md" style={{ color: 'var(--brand-primary)', fontWeight: 800 }} />
            {product.prepTimeMinutes != null && (
              <span className="inline-flex items-center gap-0.5 text-step-2xs font-medium whitespace-nowrap" style={{ color: 'var(--brand-text-muted)' }}>
                <i className="ti ti-clock" style={{ fontSize: '0.7rem' }} aria-hidden="true" />
                {t('product.prep_minutes', '~{{n}} min', { n: product.prepTimeMinutes })}
=======
        {/* Footer: price + meta. flex-wrap so on a narrow 2-col card the meta drops to its own
            line instead of colliding with a wide price (the "1500 ALL~15 min" overlap bug). */}
        <div className="flex flex-wrap items-baseline justify-between gap-x-2 gap-y-1 mt-auto pt-1.5">
          <m.div layoutId={morph ? revealLayoutId('price', product.id) : undefined} className="flex items-baseline">
            <PriceDisplay amount={product.price} size="md" style={{ color: 'var(--brand-primary-readable, var(--brand-text))', fontWeight: 800 }} />
          </m.div>
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
>>>>>>> Stashed changes
              </span>
            )}
            {hasNutrition && (
              <span className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>
                {product.kcal}kcal
                {product.protein != null && <span className="opacity-60"> · P{product.protein}g</span>}
                {product.fat != null && <span className="opacity-60"> · F{product.fat}g</span>}
              </span>
            )}
          </div>
        </div>

        {hasTaste && (
          <div className="flex gap-1.5 flex-wrap">
            {/* Skip axes we have no icon for — a hollow ti-circle fallback reads as an
                empty/broken glyph, so an unmapped axis is dropped rather than rendered blank. */}
            {Object.entries(product.taste!).filter(([axis, v]) => v > 0 && TASTE_ICONS[axis]).map(([axis, level]) => (
              <span key={axis} className="inline-flex items-center gap-0.5" style={{ color: 'color-mix(in srgb, var(--brand-text) 62%, transparent)' }} title={`${TASTE_LABELS[axis] || axis}`}>
                {Array.from({ length: level }).map((_, i) => (
                  <i key={i} className={TASTE_ICONS[axis]} style={{ fontSize: '0.7rem' }} />
                ))}
              </span>
            ))}
          </div>
        )}
      </div>
    </m.article>
  );
}
