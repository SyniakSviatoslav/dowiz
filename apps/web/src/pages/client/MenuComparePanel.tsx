import { motion } from 'framer-motion';
import { useI18n, PriceDisplay, getAllergenStyle, ease, compareDishes } from '@deliveryos/ui';
import type { CompareDishInput } from '@deliveryos/ui';

// Two-dish comparison panel (council menu-characteristics-model §8.2 / FB-M4). Affordance-only entry
// (no long-press). It reuses the SAME single derivation as every other surface — compareDishes() calls
// computeAllergenSurface() for both dishes' allergens, so a card chip and a compare cell can never disagree.
//
// Facts, never a verdict (#11): directional "lower" markers appear ONLY on price + prep-time (neutral
// facts — cheaper / faster). There is NO global winner, NO macro arrow, NO "healthier". Taste is
// side-by-side. Allergens render BOTH surfaces explicitly (#8): a no-data dish shows the floor + the
// reliance bound, NEVER a blank that would read "free-from" by contrast.

const TASTE_ICONS: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };

interface MenuComparePanelProps {
  a: CompareDishInput;
  b: CompareDishInput;
  onClose: () => void;
}

export function MenuComparePanel({ a, b, onClose }: MenuComparePanelProps) {
  const { t } = useI18n();
  const c = compareDishes(a, b);

  // A down-arrow marker on the "lower wins" cell (cheaper / faster). Neutral fact, not a verdict.
  const lowerMark = (axis: 'a' | 'b', side: 'a' | 'b') =>
    axis === side ? (
      <i className="ti ti-arrow-down ml-1" aria-label={t('compare.lower_marker', 'lower')} style={{ color: 'var(--brand-primary-readable, var(--brand-text))', fontSize: '0.8rem' }} />
    ) : null;

  const tasteChips = (taste: Record<string, number>) => {
    const axes = Object.entries(taste).filter(([axis, v]) => v > 0 && TASTE_ICONS[axis]).sort((x, y) => y[1] - x[1]);
    if (axes.length === 0) return <span className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>—</span>;
    return (
      <div className="flex flex-wrap gap-1">
        {axes.map(([axis, v]) => (
          <span key={axis} className="inline-flex items-center gap-0.5 text-step-2xs px-1.5 py-0.5 rounded-md" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}>
            <i className={TASTE_ICONS[axis]} aria-hidden="true" style={{ fontSize: '0.75rem' }} />{v}
          </span>
        ))}
      </div>
    );
  };

  const allergenCell = (surface: { known: string[]; hasInfo: boolean }) => (
    surface.hasInfo ? (
      <div className="flex flex-wrap gap-1">
        {surface.known.map(al => {
          const s = getAllergenStyle(al);
          return <span key={al} className="px-1.5 py-0.5 rounded font-semibold text-step-2xs uppercase" style={{ background: s.bg, color: s.text }}>{t(`allergen.${al.toLowerCase()}`, al)}</span>;
        })}
      </div>
    ) : (
      <span className="text-step-2xs" data-testid="compare-allergen-no-info" style={{ color: 'var(--brand-text-muted)' }}>{t('client.allergen_info_not_provided', 'Allergen info not provided')}</span>
    )
  );

  const Row = ({ label, a: ca, b: cb }: { label: string; a: React.ReactNode; b: React.ReactNode }) => (
    <>
      <div className="text-step-2xs font-semibold uppercase tracking-wider pt-3" style={{ color: 'var(--brand-text-muted)' }}>{label}</div>
      <div className="pt-3 pr-2" style={{ color: 'var(--brand-text)' }}>{ca}</div>
      <div className="pt-3" style={{ color: 'var(--brand-text)' }}>{cb}</div>
    </>
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
        className="w-full max-w-lg rounded-t-2xl p-4 pb-6 max-h-[85vh] overflow-y-auto"
        style={{ background: 'var(--brand-bg)' }}
        initial={{ y: '100%' }} animate={{ y: 0 }} exit={{ y: '100%' }}
        transition={{ duration: 0.24, ease: ease.out }}
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-1">
          <h2 className="text-base font-bold" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' }}>{t('compare.title', 'Compare dishes')}</h2>
          <button onClick={onClose} className="text-step-2xs font-semibold px-3 h-9 rounded-full" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}>{t('compare.exit', 'Done')}</button>
        </div>

        {/* Header row: the two dish names */}
        <div className="grid grid-cols-[max-content_1fr_1fr] gap-x-3 items-end">
          <div />
          <div className="text-sm font-bold pb-1 pr-2" style={{ color: 'var(--brand-text)' }}>{a.name}</div>
          <div className="text-sm font-bold pb-1" style={{ color: 'var(--brand-text)' }}>{b.name}</div>

          <Row
            label={t('sort.price', 'Price')}
            a={<span className="inline-flex items-center font-bold"><PriceDisplay amount={c.price.a ?? 0} size="sm" />{lowerMark(c.price.lower as 'a' | 'b', 'a')}</span>}
            b={<span className="inline-flex items-center font-bold"><PriceDisplay amount={c.price.b ?? 0} size="sm" />{lowerMark(c.price.lower as 'a' | 'b', 'b')}</span>}
          />
          <Row
            label={t('compare.prep_time', 'Prep time')}
            a={c.prepTime.a != null ? <span className="inline-flex items-center">{t('product.prep_minutes', '~{{n}} min', { n: c.prepTime.a })}{lowerMark(c.prepTime.lower as 'a' | 'b', 'a')}</span> : <span style={{ color: 'var(--brand-text-muted)' }}>—</span>}
            b={c.prepTime.b != null ? <span className="inline-flex items-center">{t('product.prep_minutes', '~{{n}} min', { n: c.prepTime.b })}{lowerMark(c.prepTime.lower as 'a' | 'b', 'b')}</span> : <span style={{ color: 'var(--brand-text-muted)' }}>—</span>}
          />
          <Row label={t('compare.taste', 'Taste')} a={tasteChips(c.taste.a)} b={tasteChips(c.taste.b)} />
          <Row label={t('client.allergens', 'Allergens')} a={allergenCell(c.allergens.a)} b={allergenCell(c.allergens.b)} />
        </div>

        {/* Reliance bound — always attached wherever the allergen surface renders (#5d/#8). */}
        <p className="text-step-2xs mt-4 italic" data-testid="compare-allergen-reliance" style={{ color: 'var(--brand-text-muted)' }}>{t('client.allergen_confirm_venue', 'Not a complete allergen list — please confirm with the venue for severe allergies.')}</p>
      </motion.div>
    </motion.div>
  );
}
