import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { useI18n, PriceDisplay, getAllergenStyle } from '../../index.js';

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
    chefPick?: boolean;
  };
  onAdd: (e: React.MouseEvent) => void;
  onClick?: (e: React.MouseEvent) => void;
}

const TASTE_ICONS: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };
const TASTE_LABELS: Record<string, string> = { spicy: 'Spicy', sweet: 'Sweet', salty: 'Salty', sour: 'Sour', richness: 'Rich' };
const SPRING_OUT = [0.16, 1, 0.3, 1] as [number, number, number, number];

const cardVariants = {
  rest: { y: 0, boxShadow: '0 1px 4px rgba(0,0,0,0.06)', scale: 1 },
  hover: { y: -4, boxShadow: '0 12px 32px rgba(0,0,0,0.13)', scale: 1.01, transition: { duration: 0.18, ease: SPRING_OUT } },
  tap: { scale: 0.975, y: -1, transition: { duration: 0.08 } },
};
const imgVariants = {
  rest: { scale: 1 },
  hover: { scale: 1.06, transition: { duration: 0.35, ease: SPRING_OUT } },
};
const addBtnVariants = {
  rest: { scale: 1, rotate: 0 },
  hover: { scale: 1.14, rotate: 8, boxShadow: '0 6px 18px rgba(0,0,0,0.22)', transition: { duration: 0.18, ease: SPRING_OUT } },
  tap: { scale: 0.82, rotate: 0, transition: { duration: 0.08 } },
};

export function ProductCard({ product, onAdd, onClick }: ProductCardProps) {
  const { t } = useI18n();
  const [imgError, setImgError] = useState(false);
  const hasAllergens = product.allergens && product.allergens.length > 0;
  const hasIngredients = product.ingredients && product.ingredients.length > 0;
  const hasTaste = product.taste && Object.keys(product.taste).length > 0;
  const hasNutrition = product.kcal != null;
  const allergens = product.allergens || [];
  const ingredients = product.ingredients || [];
  const isChefPick = !!product.chefPick;

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
      whileHover={product.isAvailable ? 'hover' : undefined}
      whileTap={product.isAvailable && onClick ? 'tap' : undefined}
    >
      <div
        className="w-full aspect-[4/3] flex items-center justify-center relative overflow-hidden"
        style={{ background: 'var(--brand-surface-raised)' }}
      >
        {product.image && !imgError ? (
          <motion.img
            src={product.image}
            alt={product.name}
            className="w-full h-full object-cover"
            loading="lazy"
            onError={() => setImgError(true)}
            variants={product.isAvailable ? imgVariants : undefined}
          />
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
                <span key={a} className="text-[7px] font-semibold px-1 py-0.5 rounded-sm leading-tight" style={{ background: s.bg, color: s.text }}>
                  {t(`allergen.${a.toLowerCase()}`, a)}
                </span>
              );
            })}
            {allergens.length > 3 && (
              <span className="text-[7px] font-semibold px-1 py-0.5 rounded-sm" style={{ background: '#fee2e2', color: '#991b1b' }}>
                +{allergens.length - 3}
              </span>
            )}
          </div>
        )}
        {!hasAllergens && product.isAvailable && (
          <div className="absolute top-1.5 left-1.5 z-10">
            <span className="text-[8px] font-medium px-1.5 py-0.5 rounded-md flex items-center gap-0.5" style={{ background: 'rgba(5,150,105,0.12)', color: 'var(--color-success)' }}>
              <i className="ti ti-circle-check" style={{ fontSize: '0.55rem' }} />
              {t('common.clean', 'Clean')}
            </span>
          </div>
        )}
        {hasNutrition && !isChefPick && (
          <div className="absolute top-1.5 right-1.5 z-10">
              <span className="text-[9px] font-semibold px-1.5 py-0.5 rounded-md flex items-center gap-1" style={{ background: 'rgba(0,0,0,0.6)', color: 'var(--color-on-primary)' }}>
              <i className="ti ti-flame" style={{ fontSize: '0.6rem' }} />
              {product.kcal}
            </span>
          </div>
        )}
        {isChefPick && (
          <div className="absolute top-1.5 right-1.5 z-10">
            <motion.span
              className="text-[8px] font-bold px-1.5 py-0.5 rounded-md flex items-center gap-0.5"
              style={{ background: 'linear-gradient(135deg, #f59e0b, #f97316)', color: '#fff', boxShadow: '0 2px 6px rgba(245,158,11,0.45)' }}
              initial={{ scale: 0.8, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              transition={{ type: 'spring', stiffness: 300, damping: 20 }}
            >
              ✦ {t('client.chefs_pick_badge', "Chef's Pick")}
            </motion.span>
          </div>
        )}
        {!product.isAvailable && (
          <>
            <div className="absolute inset-0 z-10" style={{ background: 'linear-gradient(to top, rgba(0,0,0,0.7) 0%, rgba(0,0,0,0.3) 100%)' }} />
            <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-20">
              <span className="text-[10px] font-semibold px-2.5 py-1 rounded-md" style={{ background: 'var(--color-danger)', color: 'var(--color-on-primary)' }}>
                {t('client.unavailable', 'Unavailable')}
              </span>
            </div>
          </>
        )}
      </div>
      <div className="p-2.5 flex flex-col flex-1 gap-1 min-h-0">
        <div className="flex items-start justify-between gap-1.5">
          <h3 className="font-semibold text-[13px] leading-tight line-clamp-2 flex-1" style={{ color: 'var(--brand-text)' }}>{product.name}</h3>
          <motion.button
            data-testid="menu-item-add"
            className={`shrink-0 min-w-[44px] min-h-[44px] flex items-center justify-center text-white rounded-full mt-0.5 ${
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
            <i className="ti ti-plus text-sm leading-none" />
          </motion.button>
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

        {/* Allergen scent lives on the image corner badges (top-3 + overflow) and
            the full labelled list is in the detail modal — rendering the loud
            uppercase colour row again here was redundant noise that broke the
            card's palette discipline, so it's intentionally omitted. */}

        <div className="flex items-center justify-between mt-auto pt-1">
          <div className="flex items-baseline gap-1">
            <PriceDisplay amount={product.price} size="sm" />
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
          <div className="flex gap-1.5 flex-wrap">
            {Object.entries(product.taste!).filter(([, v]) => v > 0).map(([axis, level]) => (
              <span key={axis} className="inline-flex items-center gap-0.5" style={{ color: 'var(--brand-text-muted)' }} title={`${TASTE_LABELS[axis] || axis}`}>
                {Array.from({ length: level }).map((_, i) => (
                  <i key={i} className={TASTE_ICONS[axis] || 'ti ti-circle'} style={{ fontSize: '0.6rem' }} />
                ))}
              </span>
            ))}
          </div>
        )}
      </div>
    </motion.article>
  );
}
