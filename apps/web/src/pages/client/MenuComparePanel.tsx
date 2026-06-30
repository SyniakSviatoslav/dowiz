import { motion } from 'framer-motion';
import { useI18n, PriceDisplay, ease } from '@deliveryos/ui';
import { DishStats } from '../../components/client/DishStats.js';
import type { DishMacros, DishIngredient } from '../../components/client/DishStats.js';

// Two-dish comparison panel (council menu-characteristics-model §8.2 / FB-M4). Affordance-only entry.
// Clean 2-column grid: each dish is a self-contained column (name, price, prep, then the DishStats viz —
// calorie ring + macro split + ingredient bars), so columns align by construction. Directional markers are
// NEUTRAL facts on price (cheaper) + prep (faster) ONLY — never a macro/global "winner" (#11). Allergens are
// FROZEN (operator directive) so no allergen cell / reliance bound renders here.

export interface CompareDish {
  id: string;
  name: string;
  price: number;
  prepTimeMinutes?: number | null;
  taste?: Record<string, number> | null;
  macros: DishMacros;
  ingredients: DishIngredient[];
}
interface MenuComparePanelProps {
  a: CompareDish;
  b: CompareDish;
  onClose: () => void;
}

const TASTE_ICONS: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };

export function MenuComparePanel({ a, b, onClose }: MenuComparePanelProps) {
  const { t } = useI18n();

  const lower = (x?: number | null, y?: number | null): 'a' | 'b' | null => {
    if (typeof x !== 'number' || typeof y !== 'number') return null;
    return x < y ? 'a' : y < x ? 'b' : null;
  };
  const priceLower = lower(a.price, b.price);
  const prepLower = lower(a.prepTimeMinutes ?? null, b.prepTimeMinutes ?? null);

  const marker = (side: 'a' | 'b', winner: 'a' | 'b' | null, label: string) =>
    winner === side ? (
      <span className="inline-flex items-center gap-0.5 text-step-2xs font-semibold px-1.5 py-0.5 rounded-full" style={{ background: 'color-mix(in srgb, var(--brand-primary) 16%, transparent)', color: 'var(--brand-primary-readable)' }}>
        <i className="ti ti-arrow-down" aria-hidden="true" style={{ fontSize: '0.7rem' }} />{label}
      </span>
    ) : null;

  const tasteRow = (taste?: Record<string, number> | null) => {
    const axes = Object.entries(taste || {}).filter(([axis, v]) => v > 0 && TASTE_ICONS[axis]).sort((x, y) => y[1] - x[1]);
    if (axes.length === 0) return <span className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>—</span>;
    return (
      <div className="flex flex-wrap gap-1">
        {axes.map(([axis, v]) => (
          <span key={axis} className="inline-flex items-center gap-0.5 text-step-2xs px-1.5 py-0.5 rounded-md" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}>
            <i className={TASTE_ICONS[axis]} aria-hidden="true" style={{ fontSize: '0.72rem' }} />{v}
          </span>
        ))}
      </div>
    );
  };

  const column = (d: CompareDish, side: 'a' | 'b') => (
    <div className="flex flex-col gap-2 min-w-0">
      <h3 className="text-sm font-bold leading-tight line-clamp-2 min-h-[2.4em]" style={{ color: 'var(--brand-text)' }}>{d.name}</h3>
      <div className="flex items-center gap-2 flex-wrap">
        <span className="text-base font-black" style={{ color: 'var(--brand-primary-readable, var(--brand-text))' }}><PriceDisplay amount={d.price} size="sm" /></span>
        {marker(side, priceLower, t('compare.cheaper', 'cheaper'))}
      </div>
      <div className="flex items-center gap-2 flex-wrap text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>
        {d.prepTimeMinutes != null ? (
          <span className="inline-flex items-center gap-1" title={t('product.prep_cooking_time', 'Cooking time (not delivery)')}>
            <i className="ti ti-tools-kitchen-2" aria-hidden="true" style={{ fontSize: '0.72rem' }} />
            {t('product.prep_minutes', '~{{n}} min', { n: d.prepTimeMinutes })}
          </span>
        ) : <span>—</span>}
        {marker(side, prepLower, t('compare.faster', 'faster'))}
      </div>
      <div className="pt-0.5">
        <div className="text-step-2xs font-semibold uppercase tracking-wider mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('compare.taste', 'Taste')}</div>
        {tasteRow(d.taste)}
      </div>
      <DishStats variant="compact" macros={d.macros} ingredients={d.ingredients} className="mt-1" />
    </div>
  );

  return (
    <motion.div
      className="fixed inset-0 z-modal flex items-end justify-center"
      style={{ background: 'rgba(0,0,0,0.45)' }}
      initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
      onClick={onClose}
      role="dialog" aria-modal="true" aria-label={t('compare.title', 'Compare dishes')}
      data-testid="compare-panel"
    >
      <motion.div
        className="w-full max-w-lg rounded-t-2xl p-4 pb-6 max-h-[88vh] overflow-y-auto"
        style={{ background: 'var(--brand-bg)' }}
        initial={{ y: '100%' }} animate={{ y: 0 }} exit={{ y: '100%' }}
        transition={{ duration: 0.24, ease: ease.out }}
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-base font-bold" style={{ color: 'var(--brand-text)' }}>{t('compare.title', 'Compare dishes')}</h2>
          <button onClick={onClose} className="text-step-2xs font-semibold px-3 h-9 rounded-full" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}>{t('compare.exit', 'Done')}</button>
        </div>
        <div className="grid grid-cols-2 gap-4 items-start">
          {column(a, 'a')}
          {column(b, 'b')}
        </div>
      </motion.div>
    </motion.div>
  );
}
