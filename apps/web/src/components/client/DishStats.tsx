import type { ReactNode } from 'react';
import { useI18n } from '@deliveryos/ui';

// ─────────────────────────────────────────────────────────────────────────────
// DishStats — a reusable, "data-art"-quality nutrition + ingredients visual.
// Used in the dish detail modal ('full') and the 2-dish compare panel ('compact').
// Pure SVG + divs + brand CSS variables; no new dependencies, no hardcoded colors.
// The calorie ring's arc segments ENCODE the macro energy split (protein/carbs
// 4 kcal/g, fat 9 kcal/g), so the ring itself reads as the macro breakdown.
// ─────────────────────────────────────────────────────────────────────────────

export interface DishMacros {
  kcal: number;
  protein: number;
  fat: number;
  carbs: number;
}

export interface DishIngredient {
  name: string;
  qty: number;
  unit: string;
  kcal: number;
}

export interface DishStatsProps {
  macros: DishMacros;
  ingredients?: DishIngredient[];
  /** 'full' = detail modal (default); 'compact' = a single compare column. */
  variant?: 'full' | 'compact';
  className?: string;
}

type MacroKey = 'protein' | 'fat' | 'carbs';

interface MacroDef {
  key: MacroKey;
  labelKey: string;
  icon: string;
  kcalPerG: number;
  /** on-brand shade derived from the single brand primary */
  color: string;
}

// protein = strong primary · carbs = ~55% primary · fat = ~30% primary.
const MACROS: readonly MacroDef[] = [
  { key: 'protein', labelKey: 'nutrition.protein', icon: 'ti ti-droplet', kcalPerG: 4, color: 'var(--brand-primary)' },
  { key: 'fat', labelKey: 'nutrition.fat', icon: 'ti ti-droplet-half', kcalPerG: 9, color: 'color-mix(in srgb, var(--brand-primary) 30%, var(--brand-surface))' },
  { key: 'carbs', labelKey: 'nutrition.carbs', icon: 'ti ti-droplet-filled', kcalPerG: 4, color: 'color-mix(in srgb, var(--brand-primary) 55%, var(--brand-surface))' },
];

const INGREDIENT_BAR_COLOR = 'color-mix(in srgb, var(--brand-primary) 70%, var(--brand-surface))';

const prefersReduced =
  typeof window !== 'undefined' &&
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Coerce a possibly-missing numeric field to a non-negative finite number. */
function safe(v: number | undefined | null): number {
  return typeof v === 'number' && Number.isFinite(v) && v > 0 ? v : 0;
}

/** Round to at most one decimal so "12.5 g" stays clean and integers stay integers. */
function fmtNum(v: number): number {
  return Math.round(v * 10) / 10;
}

const COUNT_UNITS = new Set(['', 'unit', 'units', 'pc', 'pcs', 'piece', 'pieces', 'x', '×', 'count']);

/**
 * Format an ingredient's declared amount: "100 g" for mass/volume units,
 * "×2" for count-like units (sheets/pieces). Built as a plain string so it
 * carries no visible JSX text literal of its own.
 */
function formatAmount(qty: number, unit: string): string {
  const u = (unit || '').trim();
  const lower = u.toLowerCase();
  const q = fmtNum(safe(qty));
  if (COUNT_UNITS.has(lower)) return `×${q}`;
  return `${q} ${u}`;
}

// ── Calorie ring (SVG donut whose arc encodes the macro energy split) ─────────

interface RingSegment {
  key: string;
  val: number;
  color: string;
}

function CalorieRing({
  size,
  stroke,
  displayKcal,
  segments,
  label,
  caloriesLabel,
}: {
  size: number;
  stroke: number;
  displayKcal: number;
  segments: RingSegment[];
  label: string;
  caloriesLabel: string;
}): JSX.Element {
  const center = size / 2;
  const r = (size - stroke) / 2;
  const circumference = 2 * Math.PI * r;
  const total = segments.reduce((sum, s) => sum + s.val, 0);
  const visible = segments.filter((s) => s.val > 0);

  let offset = 0;
  const arcs = visible.map((s) => {
    const len = (s.val / total) * circumference;
    const node = (
      <circle
        key={s.key}
        cx={center}
        cy={center}
        r={r}
        fill="none"
        stroke={s.color}
        strokeWidth={stroke}
        strokeLinecap="butt"
        strokeDasharray={`${len} ${circumference - len}`}
        strokeDashoffset={-offset}
        transform={`rotate(-90 ${center} ${center})`}
        style={{ transition: prefersReduced ? 'none' : 'stroke-dasharray 0.5s ease' }}
      />
    );
    offset += len;
    return node;
  });

  return (
    <div className="relative shrink-0" style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={label}>
        {/* track */}
        <circle cx={center} cy={center} r={r} fill="none" stroke="var(--brand-border)" strokeWidth={stroke} />
        {/* macro-energy arcs, or a single solid ring when no macro breakdown exists */}
        {total > 0 ? (
          arcs
        ) : (
          <circle
            cx={center}
            cy={center}
            r={r}
            fill="none"
            stroke="var(--brand-primary)"
            strokeWidth={stroke}
            transform={`rotate(-90 ${center} ${center})`}
          />
        )}
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center" aria-hidden="true">
        <i className="ti ti-flame" style={{ color: 'var(--brand-primary-readable)', fontSize: size * 0.16, lineHeight: 1 }} />
        <span
          style={{
            fontFamily: 'var(--brand-font-heading)',
            color: 'var(--brand-text)',
            fontSize: size * 0.26,
            fontWeight: 800,
            lineHeight: 1.05,
          }}
        >
          {displayKcal}
        </span>
        <span className="text-step-2xs font-medium uppercase tracking-wider" style={{ color: 'var(--brand-text-muted)' }}>
          {caloriesLabel}
        </span>
      </div>
    </div>
  );
}

// ── Macro rows (label + grams + proportional bar + subtle % of energy) ────────

function MacroRow({
  def,
  grams,
  gramsShare,
  energyShare,
  label,
}: {
  def: MacroDef;
  grams: number;
  gramsShare: number;
  energyShare: number;
  label: string;
}): JSX.Element {
  return (
    <div className="w-full" aria-label={`${label}: ${grams} g, ${energyShare}%`}>
      <div className="flex items-center justify-between gap-2 mb-1">
        <span className="inline-flex items-center gap-1.5 text-step-2xs font-medium min-w-0" style={{ color: 'var(--brand-text-muted)' }}>
          <i className={def.icon} aria-hidden="true" style={{ color: def.color, fontSize: '0.85rem' }} />
          <span className="truncate">{label}</span>
        </span>
        <span className="text-step-2xs font-semibold shrink-0 tabular-nums" style={{ color: 'var(--brand-text)' }}>
          {grams}
          <span style={{ color: 'var(--brand-text-muted)', fontWeight: 400 }}> g</span>
          {energyShare > 0 && <span style={{ color: 'var(--brand-text-muted)', fontWeight: 400 }}> · {energyShare}%</span>}
        </span>
      </div>
      <div className="w-full h-1.5 rounded-full overflow-hidden" style={{ background: 'var(--brand-surface-raised)' }}>
        <div
          className="h-full rounded-full"
          style={{
            width: `${gramsShare}%`,
            background: def.color,
            transition: prefersReduced ? 'none' : 'width 0.5s ease',
          }}
        />
      </div>
    </div>
  );
}

// ── Ingredient bars (share of total ingredient kcal, fallback to qty) ─────────

function IngredientRow({
  name,
  amount,
  share,
  ariaLabel,
}: {
  name: string;
  amount: string;
  share: number;
  ariaLabel: string;
}): JSX.Element {
  return (
    <div className="w-full" aria-label={ariaLabel}>
      <div className="flex items-center justify-between gap-2 mb-0.5">
        <span className="text-step-2xs font-medium truncate" style={{ color: 'var(--brand-text)' }}>
          {name}
        </span>
        <span className="text-step-2xs shrink-0 tabular-nums" style={{ color: 'var(--brand-text-muted)' }}>
          {amount}
        </span>
      </div>
      <div className="w-full h-1.5 rounded-full overflow-hidden" style={{ background: 'var(--brand-surface-raised)' }}>
        <div
          className="h-full rounded-full"
          style={{
            width: `${share}%`,
            background: INGREDIENT_BAR_COLOR,
            transition: prefersReduced ? 'none' : 'width 0.5s ease',
          }}
        />
      </div>
    </div>
  );
}

function SectionHeading({ icon, children }: { icon: string; children: ReactNode }): JSX.Element {
  return (
    <h3 className="text-xs font-semibold uppercase tracking-wider flex items-center gap-1.5" style={{ color: 'var(--brand-text-muted)' }}>
      <i className={icon} aria-hidden="true" /> {children}
    </h3>
  );
}

export function DishStats({ macros, ingredients, variant = 'full', className }: DishStatsProps): JSX.Element | null {
  const { t } = useI18n();

  const vals: Record<MacroKey, number> = {
    protein: safe(macros?.protein),
    fat: safe(macros?.fat),
    carbs: safe(macros?.carbs),
  };
  const kcal = safe(macros?.kcal);

  const ingr = (ingredients ?? []).filter((i): i is DishIngredient => !!i && typeof i.name === 'string' && i.name.length > 0);

  // Guard: nothing meaningful to draw.
  if (kcal <= 0 && ingr.length === 0) return null;

  const compact = variant === 'compact';

  // Macro energy split — this is what the ring arcs encode.
  const energies: Record<MacroKey, number> = {
    protein: vals.protein * 4,
    fat: vals.fat * 9,
    carbs: vals.carbs * 4,
  };
  const totalMacroE = energies.protein + energies.fat + energies.carbs;
  const totalGrams = vals.protein + vals.fat + vals.carbs;
  const hasMacros = totalGrams > 0;

  const displayKcal = kcal > 0 ? Math.round(kcal) : Math.round(totalMacroE);
  const showRing = displayKcal > 0 || totalMacroE > 0;

  const segments: RingSegment[] = MACROS.map((m) => ({ key: m.key, val: energies[m.key], color: m.color }));

  const ringLabel = `${t('client.nutrition', 'Nutrition')}: ${displayKcal} ${t('nutrition.calories', 'kcal')}` +
    (hasMacros
      ? ` — ${MACROS.map((m) => `${t(m.labelKey, m.key)} ${fmtNum(vals[m.key])} g`).join(', ')}`
      : '');

  // Ingredient bar metric: share of total kcal, or share of qty if all kcal are 0.
  const useKcal = ingr.some((i) => safe(i.kcal) > 0);
  const metricOf = (i: DishIngredient): number => (useKcal ? safe(i.kcal) : safe(i.qty));
  const sorted = [...ingr].sort((a, b) => metricOf(b) - metricOf(a));
  const maxMetric = Math.max(1, ...sorted.map(metricOf));
  const ingrList = compact ? sorted.slice(0, 5) : sorted;
  const hasIngredients = ingrList.length > 0;

  const ringSize = compact ? 84 : 120;
  const ringStroke = compact ? 9 : 13;

  const macroRows = (
    <div className={`w-full flex flex-col ${compact ? 'gap-2' : 'gap-2.5'}`}>
      {MACROS.filter((m) => vals[m.key] > 0).map((m) => (
        <MacroRow
          key={m.key}
          def={m}
          grams={fmtNum(vals[m.key])}
          gramsShare={totalGrams > 0 ? (vals[m.key] / totalGrams) * 100 : 0}
          energyShare={totalMacroE > 0 ? Math.round((energies[m.key] / totalMacroE) * 100) : 0}
          label={t(m.labelKey, m.key)}
        />
      ))}
    </div>
  );

  const ingredientBars = (
    <div className="w-full flex flex-col gap-1.5">
      {ingrList.map((i, idx) => {
        const amount = formatAmount(i.qty, i.unit);
        const kcalSuffix = safe(i.kcal) > 0 ? `, ${Math.round(safe(i.kcal))} ${t('nutrition.calories', 'kcal')}` : '';
        return (
          <IngredientRow
            key={`${i.name}-${idx}`}
            name={i.name}
            amount={amount}
            share={(metricOf(i) / maxMetric) * 100}
            ariaLabel={`${i.name} · ${amount}${kcalSuffix}`}
          />
        );
      })}
    </div>
  );

  // ── COMPACT (compare column): tight, single-column, full-width for grid alignment.
  if (compact) {
    return (
      <div data-testid="dish-stats" data-variant="compact" className={`w-full flex flex-col gap-3 ${className ?? ''}`}>
        {showRing && (
          <div className="w-full flex flex-col items-center gap-2.5">
            <CalorieRing
              size={ringSize}
              stroke={ringStroke}
              displayKcal={displayKcal}
              segments={segments}
              label={ringLabel}
              caloriesLabel={t('nutrition.calories', 'Calories')}
            />
            {hasMacros && macroRows}
          </div>
        )}
        {hasIngredients && (
          <div className="w-full flex flex-col gap-1.5">
            <SectionHeading icon="ti ti-list-check">{t('client.ingredients', 'Ingredients')}</SectionHeading>
            {ingredientBars}
          </div>
        )}
      </div>
    );
  }

  // ── FULL (detail modal): roomy, brand cards.
  return (
    <div data-testid="dish-stats" data-variant="full" className={`w-full flex flex-col gap-3 ${className ?? ''}`}>
      {(showRing || hasMacros) && (
        <div className="rounded-xl p-4 flex flex-col gap-4" style={{ background: 'var(--brand-surface)' }}>
          <SectionHeading icon="ti ti-report-analytics">{t('client.nutrition', 'Nutrition')}</SectionHeading>
          <div className="flex flex-col sm:flex-row sm:items-center gap-4">
            {showRing && (
              <CalorieRing
                size={ringSize}
                stroke={ringStroke}
                displayKcal={displayKcal}
                segments={segments}
                label={ringLabel}
                caloriesLabel={t('nutrition.calories', 'Calories')}
              />
            )}
            {hasMacros && <div className="flex-1 min-w-0">{macroRows}</div>}
          </div>
        </div>
      )}
      {hasIngredients && (
        <div className="rounded-xl p-4 flex flex-col gap-3" style={{ background: 'var(--brand-surface)' }}>
          <SectionHeading icon="ti ti-list-check">{t('client.ingredients', 'Ingredients')}</SectionHeading>
          {ingredientBars}
        </div>
      )}
    </div>
  );
}
