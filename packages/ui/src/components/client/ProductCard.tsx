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
  };
  onAdd: (e: React.MouseEvent) => void;
  onClick?: (e: React.MouseEvent) => void;
}

const TASTE_ICONS: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };
const TASTE_LABELS: Record<string, string> = { spicy: 'Spicy', sweet: 'Sweet', salty: 'Salty', sour: 'Sour', richness: 'Rich' };
export function ProductCard({ product, onAdd, onClick }: ProductCardProps) {
  const { t } = useI18n();
  const [imgError, setImgError] = useState(false);
  const hasAllergens = product.allergens && product.allergens.length > 0;
  const hasIngredients = product.ingredients && product.ingredients.length > 0;
  const hasTaste = product.taste && Object.keys(product.taste).length > 0;
  const hasNutrition = product.kcal != null;
  const allergens = product.allergens || [];
  const ingredients = product.ingredients || [];

  return (
    <motion.article 
      data-testid="menu-item"
      className={`flex flex-col cursor-pointer overflow-hidden border rounded-xl transition-all duration-150 h-full ${
        product.isAvailable ? 'hover:-translate-y-0.5 hover:shadow-lg active:scale-[0.98]' : 'opacity-55'
      }`}
      style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}
      onClick={onClick}
      whileTap={product.isAvailable && onClick ? { scale: 0.97 } : undefined}
    >
      <div 
        className="w-full aspect-[4/3] flex items-center justify-center relative overflow-hidden" 
        style={{ background: 'var(--brand-surface-raised)' }}
      >
        {product.image && !imgError ? (
          <img src={product.image} alt={product.name} className="w-full h-full object-cover transition-transform duration-300 hover:scale-105" loading="lazy" onError={() => setImgError(true)} />
        ) : (
          <div className="flex flex-col items-center gap-1.5" style={{ color: 'var(--brand-text-muted)' }}>
            <i className="ti ti-tools-kitchen-2 text-4xl opacity-25" />
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
              <span className="text-[7px] font-semibold px-1 py-0.5 rounded-sm" style={{ background: 'rgba(220,38,38,0.15)', color: 'var(--color-danger)' }}>
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
        {hasNutrition && (
          <div className="absolute top-1.5 right-1.5 z-10">
              <span className="text-[9px] font-semibold px-1.5 py-0.5 rounded-md flex items-center gap-1" style={{ background: 'rgba(0,0,0,0.6)', color: 'var(--color-on-primary)' }}>
              <i className="ti ti-flame" style={{ fontSize: '0.6rem' }} />
              {product.kcal}
            </span>
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
            className={`shrink-0 min-w-[44px] min-h-[44px] flex items-center justify-center text-white rounded-full transition-all duration-150 ease-in-out mt-0.5 ${
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
            title={t('tooltip.add_to_cart', 'Add to cart')}
            whileTap={product.isAvailable ? { scale: 0.97 } : undefined}
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

        {hasAllergens && (
          <div className="flex gap-0.5 flex-wrap">
            {allergens.map(a => {
              const s = getAllergenStyle(a);
              return (
                <span key={a} className="px-1 py-0 rounded text-[8px] font-semibold uppercase leading-tight" style={{ background: s.bg, color: s.text }}>
                  {t(`allergen.${a.toLowerCase()}`, a)}
                </span>
              );
            })}
          </div>
        )}

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
