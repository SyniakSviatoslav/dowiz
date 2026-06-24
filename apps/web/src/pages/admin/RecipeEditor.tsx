import { useState, useMemo } from 'react';
import { loadSupplies } from './SupplyLibraryPage.js';
import { useI18n } from '@deliveryos/ui';

const KIND_ICONS: Record<string, string> = {
  food_ingredient: 'ti ti-meat',
  condiment: 'ti ti-bottle',
  packaging: 'ti ti-box',
  utensil: 'ti ti-tool',
};

interface RecipeLine {
  supplyId: string;
  supplyName: string;
  qty: number;
  unit: string;
  kind: string;
  kcal: number | null;
  proteinG: number | null;
  fatG: number | null;
  carbsG: number | null;
  allergens: string[];
}

interface RecipeEditorProps {
  lines: RecipeLine[];
  onChange: (lines: RecipeLine[]) => void;
  onBomAllergensChange?: (allergens: string[]) => void;
}

export function RecipeEditor({ lines, onChange, onBomAllergensChange }: RecipeEditorProps) {
  const { t } = useI18n();
  const [search, setSearch] = useState('');
  const [showPicker, setShowPicker] = useState(false);
  const [activeKind, setActiveKind] = useState<string>('food_ingredient');
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  const allSupplies = useMemo(() => loadSupplies().filter(s => s.active), []);

  const filteredSupplies = useMemo(() => {
    let result = allSupplies.filter(s => s.kind === activeKind);
    if (search) {
      const q = search.toLowerCase();
      result = result.filter(s =>
        s.name.toLowerCase().includes(q) ||
        (s.category || '').toLowerCase().includes(q)
      );
    }
    return result;
  }, [search, activeKind, allSupplies]);

  const nutrition = useMemo(() => {
    let kcal = 0, protein = 0, fat = 0, carbs = 0;
    let complete = lines.length > 0;
    const bomAllergens = new Set<string>();
    for (const line of lines) {
      if (line.kcal != null) kcal += line.kcal; else complete = false;
      if (line.proteinG != null) protein += line.proteinG;
      if (line.fatG != null) fat += line.fatG;
      if (line.carbsG != null) carbs += line.carbsG;
      line.allergens.forEach(a => bomAllergens.add(a));
    }
    const allergens = [...bomAllergens];
    // Notify parent about BOM allergens
    if (onBomAllergensChange) onBomAllergensChange(allergens);
    return { kcal: Math.round(kcal), protein: Math.round(protein), fat: Math.round(fat), carbs: Math.round(carbs), complete, bomAllergens: allergens };
  }, [lines, onBomAllergensChange]);

  const toggleSelect = (supplyId: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(supplyId)) next.delete(supplyId); else next.add(supplyId);
      return next;
    });
  };

  const addSelectedSupplies = () => {
    const toAdd = allSupplies.filter(s => selectedIds.has(s.id) && !lines.some(l => l.supplyId === s.id));
    if (!toAdd.length) return;
    const newLines = [...lines];
    for (const supply of toAdd) {
      const qty = supply.baseUnit === 'g' || supply.baseUnit === 'ml' ? 100 : 1;
      const ratio = qty / 100;
      newLines.push({
        supplyId: supply.id,
        supplyName: supply.name,
        qty,
        unit: supply.baseUnit,
        kind: supply.kind,
        kcal: supply.kcalPer100 != null ? Math.round(supply.kcalPer100 * ratio) : null,
        proteinG: supply.proteinMgPer100 != null ? Math.round((supply.proteinMgPer100 / 1000) * ratio) : null,
        fatG: supply.fatMgPer100 != null ? Math.round((supply.fatMgPer100 / 1000) * ratio) : null,
        carbsG: supply.carbMgPer100 != null ? Math.round((supply.carbMgPer100 / 1000) * ratio) : null,
        allergens: supply.allergens,
      });
    }
    onChange(newLines);
    setSelectedIds(new Set());
    setSearch('');
  };

  const updateQty = (index: number, newQty: number) => {
    if (newQty <= 0) { onChange(lines.filter((_, i) => i !== index)); return; }
    const updated = [...lines];
    const line = updated[index];
    if (!line) return;
    const supply = allSupplies.find(s => s.id === line.supplyId);
    if (!supply) return;
    const ratio = newQty / 100;
    updated[index] = {
      ...line, qty: newQty,
      kcal: supply.kcalPer100 != null ? Math.round(supply.kcalPer100 * ratio) : null,
      proteinG: supply.proteinMgPer100 != null ? Math.round((supply.proteinMgPer100 / 1000) * ratio) : null,
      fatG: supply.fatMgPer100 != null ? Math.round((supply.fatMgPer100 / 1000) * ratio) : null,
      carbsG: supply.carbMgPer100 != null ? Math.round((supply.carbMgPer100 / 1000) * ratio) : null,
    };
    onChange(updated);
  };

  const step = (unit: string) => unit === 'unit' ? 1 : 10;
  const kinds = ['food_ingredient', 'condiment', 'packaging', 'utensil'];

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <label className="text-xs font-medium min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.recipe_bom', 'Recipe (BOM) — per serving')}</label>
        <button type="button" onClick={() => setShowPicker(!showPicker)} aria-expanded={showPicker}
          className="shrink-0 flex items-center gap-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-[background,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] [@media(hover:hover)]:hover:bg-[var(--brand-surface-raised)] active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1"
          style={{ color: 'var(--brand-primary)', border: '1px solid var(--brand-primary)' }}>
          <i className="ti ti-plus" style={{ fontSize: '0.7rem' }} /> {t('admin.add_supply', 'Add supply')}
        </button>
      </div>

      {showPicker && (
        <div className="p-2 rounded-lg border slide-in-up" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)' }}>
          <div className="flex gap-1 mb-2 overflow-x-auto hide-scrollbar -mx-2 px-2">
            {kinds.map(k => {
              const tabActive = activeKind === k;
              return (
              <button key={k} type="button" aria-pressed={tabActive} onClick={() => { setActiveKind(k); setSearch(''); }}
                className={`shrink-0 flex items-center gap-1 px-2 py-1 rounded text-[10px] font-medium transition-[background,color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 ${tabActive ? 'text-white' : 'text-[var(--brand-text-muted)] [@media(hover:hover)]:hover:bg-[var(--brand-surface)]'}`}
                style={{ background: tabActive ? 'var(--brand-primary)' : 'var(--brand-surface)' }}>
                <i className={KIND_ICONS[k] || 'ti ti-circle'} style={{ fontSize: '0.65rem' }} />
                {k === 'food_ingredient' ? t('supply.food') : k === 'condiment' ? t('supply.sauces') : k === 'packaging' ? t('supply.packaging') : t('supply.utensils')}
              </button>
              );
            })}
          </div>
          {/* eslint-disable jsx-a11y/no-autofocus */}
          <input value={search} onChange={e => setSearch(e.target.value)} placeholder={t('common.search')}
            autoFocus className="w-full h-8 px-2 mb-1 rounded text-xs outline-none border transition-shadow duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          {/* eslint-enable jsx-a11y/no-autofocus */}
          <div className="max-h-36 overflow-y-auto space-y-0.5 mb-1">
            {filteredSupplies.map(s => {
              const isSelected = selectedIds.has(s.id);
              const isAlready = lines.some(l => l.supplyId === s.id);
              return (
                <button key={s.id} type="button" onClick={() => { if (!isAlready) toggleSelect(s.id); }}
                  disabled={isAlready} aria-pressed={isSelected}
                  className="w-full flex items-center gap-2 px-2 py-1.5 rounded text-xs text-left transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)] [@media(hover:hover)]:hover:bg-[var(--brand-surface)] disabled:opacity-30 disabled:cursor-not-allowed focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1"
                  style={{ color: 'var(--brand-text)', background: isSelected ? 'var(--brand-primary-light)' : 'transparent' }}>
                  <input type="checkbox" checked={isSelected || isAlready} readOnly tabIndex={-1} className="w-3 h-3 shrink-0 accent-[var(--brand-primary)]" />
                  <span className="flex-1 min-w-0 truncate">{s.name}</span>
                  <span className="shrink-0 text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>{s.baseUnit}</span>
                  {s.kcalPer100 != null && <span className="shrink-0 text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>{s.kcalPer100}kcal</span>}
                </button>
              );
            })}
            {filteredSupplies.length === 0 && (
              <div className="flex flex-col items-center gap-1 px-2 py-4 text-center">
                <i className={`${search ? 'ti ti-search-off' : 'ti ti-package-off'}`} style={{ fontSize: '1.1rem', color: 'var(--brand-text-muted)' }} />
                <p className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>
                  {search ? t('admin.no_matches', 'No matches.') : t('admin.no_supplies_add_first', 'No supplies here. Add in Supplies first.')}
                </p>
              </div>
            )}
          </div>
          {selectedIds.size > 0 && (
            <button type="button" onClick={addSelectedSupplies}
              className="w-full py-1.5 rounded text-[11px] font-medium text-white active:scale-95 transition-transform duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1"
              style={{ background: 'var(--brand-primary)' }}>
              {t('common.add', 'Add')} {selectedIds.size} {t('common.selected', 'selected')}
            </button>
          )}
        </div>
      )}

      {lines.length === 0 && !showPicker && (
        <div className="flex flex-col items-center gap-1.5 px-4 py-6 rounded-lg border border-dashed text-center" style={{ borderColor: 'var(--brand-border)' }}>
          <i className="ti ti-bowl-spoon" style={{ fontSize: '1.4rem', color: 'var(--brand-text-muted)' }} />
          <p className="text-xs font-medium" style={{ color: 'var(--brand-text)' }}>{t('admin.recipe_empty_title', 'No ingredients yet')}</p>
          <p className="text-[11px] leading-snug max-w-[36ch]" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.recipe_empty_help', 'Add supplies to auto-calculate nutrition and flag allergens.')}</p>
        </div>
      )}

      {lines.length > 0 && (
        <div className="space-y-1.5">
          {lines.map((line, i) => {
            const s = allSupplies.find(x => x.id === line.supplyId);
            const hasNutrition = s && (s.kind === 'food_ingredient' || s.kind === 'condiment') && s.kcalPer100 != null;
            return (
              <div key={line.supplyId} className="flex items-center gap-2 p-2 rounded-lg slide-in-up"
                style={{ background: 'var(--brand-surface-raised)', animationDelay: `${i * 50}ms` }}>
                <i className={`${KIND_ICONS[line.kind] || 'ti ti-circle'} shrink-0`} style={{ fontSize: '0.65rem', color: s && hasNutrition ? 'var(--color-success)' : 'var(--brand-text-muted)' }} />
                <span className="text-xs flex-1 min-w-0 truncate" style={{ color: 'var(--brand-text)' }}>{line.supplyName}</span>
                {!hasNutrition && (
                  <span className="shrink-0 text-[9px] px-1 rounded" style={{ color: 'var(--color-warning)', background: 'var(--color-warning-light)' }}>{t('common.no_data', 'no data')}</span>
                )}
                <div className="flex items-center gap-1 shrink-0">
                  <button type="button" aria-label={t('common.decrease_quantity', 'Decrease quantity')} onClick={() => updateQty(i, line.qty - step(line.unit))}
                    className="w-6 h-6 rounded flex items-center justify-center text-xs transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)] [@media(hover:hover)]:hover:bg-[var(--brand-surface)] active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1"
                    style={{ color: 'var(--brand-text-muted)' }}>-</button>
                  <input type="number" value={line.qty} aria-label={`${line.supplyName} ${t('admin.qty', 'quantity')}`}
                    onChange={e => updateQty(i, parseInt(e.target.value) || 0)}
                    className="w-12 h-6 text-center rounded text-[10px] outline-none border transition-shadow duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1"
                    style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  <span className="text-[9px] w-8 text-center" style={{ color: 'var(--brand-text-muted)' }}>{line.unit}</span>
                </div>
                <button type="button" aria-label={t('common.remove', 'Remove')} onClick={() => onChange(lines.filter((_, j) => j !== i))}
                  className="shrink-0 w-6 h-6 rounded flex items-center justify-center transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)] [@media(hover:hover)]:hover:bg-[var(--color-danger-light)] active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-danger)] focus-visible:ring-offset-1">
                  <i className="ti ti-x" style={{ fontSize: '0.6rem', color: 'var(--brand-text-muted)' }} />
                </button>
              </div>
            );
          })}
        </div>
      )}

      {lines.length > 0 && (
        <div className="p-3 rounded-lg border" style={{
          background: nutrition.complete ? 'var(--color-success-light)' : 'var(--brand-surface-raised)',
          borderColor: nutrition.complete ? 'var(--color-success)' : 'var(--brand-border)'
        }}>
          <div className="flex items-center gap-1.5 mb-2">
            <i className="ti ti-chart-donut text-xs" style={{ color: nutrition.complete ? 'var(--color-success)' : 'var(--brand-text-muted)' }} />
            <span className="text-[10px] font-semibold" style={{ color: nutrition.complete ? 'var(--color-success)' : 'var(--brand-text-muted)' }}>
              {nutrition.complete ? t('admin.nutrition_per_serving', 'Nutrition per serving') : t('admin.incomplete_nutrition', 'Incomplete — some supplies lack nutrition data')}
            </span>
          </div>
          <div className="grid grid-cols-4 gap-2 text-center">
            {[
              { label: t('admin.kcal', 'Kcal'), value: nutrition.kcal || '—', color: 'var(--brand-primary)' },
              { label: t('admin.protein', 'Protein'), value: nutrition.protein ? `${nutrition.protein}g` : '—', color: 'var(--color-info)' },
              { label: t('admin.fat', 'Fat'), value: nutrition.fat ? `${nutrition.fat}g` : '—', color: 'var(--color-warning)' },
              { label: t('admin.carbs', 'Carbs'), value: nutrition.carbs ? `${nutrition.carbs}g` : '—', color: 'var(--color-success)' },
            ].map(n => (
              <div key={n.label} className="p-1.5 rounded" style={{ background: 'var(--brand-surface)' }}>
                <div className="text-sm font-bold" style={{ color: n.color }}>{n.value}</div>
                <div className="text-[9px]" style={{ color: 'var(--brand-text-muted)' }}>{n.label}</div>
              </div>
            ))}
          </div>
          {nutrition.bomAllergens.length > 0 && (
            <div className="flex items-center gap-1 mt-2 flex-wrap">
              <span className="text-[9px]" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.bom_label', 'BOM:')}</span>
              {nutrition.bomAllergens.map(a => (
                <span key={a} className="px-1.5 py-0.5 rounded-full text-[9px] font-medium"
                  style={{ background: 'var(--color-warning-light)', color: 'var(--color-warning)' }}>{t(`allergen.${a}`, a)}</span>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
