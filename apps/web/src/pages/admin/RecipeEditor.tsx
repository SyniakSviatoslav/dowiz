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
        <label className="text-xs font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.recipe_bom', 'Recipe (BOM) — per serving')}</label>
        <button type="button" onClick={() => setShowPicker(!showPicker)}
          className="flex items-center gap-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-colors hover:bg-[var(--brand-surface-raised)] active:scale-95"
          style={{ color: 'var(--brand-primary)', border: '1px solid var(--brand-primary)' }}>
          <i className="ti ti-plus" style={{ fontSize: '0.7rem' }} /> {t('admin.add_supply', 'Add supply')}
        </button>
      </div>

      {showPicker && (
        <div className="p-2 rounded-lg border" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)' }}>
          <div className="flex gap-1 mb-2">
            {kinds.map(k => (
              <button key={k} type="button" onClick={() => { setActiveKind(k); setSearch(''); }}
                className={`flex items-center gap-1 px-2 py-1 rounded text-[10px] font-medium transition-all ${activeKind === k ? 'text-white' : 'text-[var(--brand-text-muted)]'}`}
                style={{ background: activeKind === k ? 'var(--brand-primary)' : 'var(--brand-surface)' }}>
                <i className={KIND_ICONS[k] || 'ti ti-circle'} style={{ fontSize: '0.65rem' }} />
                {k === 'food_ingredient' ? t('supply.food') : k === 'condiment' ? t('supply.sauces') : k === 'packaging' ? t('supply.packaging') : t('supply.utensils')}
              </button>
            ))}
          </div>
          <input value={search} onChange={e => setSearch(e.target.value)} placeholder={t('common.search')}
            autoFocus className="w-full h-8 px-2 mb-1 rounded text-xs outline-none border"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          <div className="max-h-36 overflow-y-auto space-y-0.5 mb-1">
            {filteredSupplies.map(s => {
              const isSelected = selectedIds.has(s.id);
              const isAlready = lines.some(l => l.supplyId === s.id);
              return (
                <button key={s.id} type="button" onClick={() => { if (!isAlready) toggleSelect(s.id); }}
                  disabled={isAlready}
                  className="w-full flex items-center gap-2 px-2 py-1.5 rounded text-xs text-left transition-colors hover:bg-[var(--brand-surface)] disabled:opacity-30"
                  style={{ color: 'var(--brand-text)', background: isSelected ? 'var(--brand-primary-light)' : 'transparent' }}>
                  <input type="checkbox" checked={isSelected || isAlready} readOnly className="w-3 h-3 accent-[var(--brand-primary)]" />
                  <span className="flex-1 truncate">{s.name}</span>
                  <span className="text-[10px] opacity-40">{s.baseUnit}</span>
                  {s.kcalPer100 != null && <span className="text-[10px] opacity-30">{s.kcalPer100}kcal</span>}
                </button>
              );
            })}
            {filteredSupplies.length === 0 && (
              <p className="text-[10px] p-2 text-center" style={{ color: 'var(--brand-text-muted)' }}>
                {search ? t('admin.no_matches', 'No matches.') : t('admin.no_supplies_add_first', 'No supplies here. Add in Supplies first.')}
              </p>
            )}
          </div>
          {selectedIds.size > 0 && (
            <button type="button" onClick={addSelectedSupplies}
              className="w-full py-1.5 rounded text-[11px] font-medium text-white active:scale-95 transition-transform"
              style={{ background: 'var(--brand-primary)' }}>
              {t('common.add', 'Add')} {selectedIds.size} {t('common.selected', 'selected')}
            </button>
          )}
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
                <i className={KIND_ICONS[line.kind] || 'ti ti-circle'} style={{ fontSize: '0.65rem', color: s && hasNutrition ? 'var(--color-success)' : 'var(--brand-text-muted)' }} />
                <span className="text-xs flex-1 truncate">{line.supplyName}</span>
                {!hasNutrition && (
                  <span className="text-[9px] px-1 rounded" style={{ color: 'var(--color-warning)', background: 'var(--color-warning-light)' }}>{t('common.no_data', 'no data')}</span>
                )}
                <div className="flex items-center gap-1">
                  <button type="button" onClick={() => updateQty(i, line.qty - step(line.unit))}
                    className="w-5 h-5 rounded flex items-center justify-center text-xs hover:bg-[var(--brand-surface)] transition-colors"
                    style={{ color: 'var(--brand-text-muted)' }}>-</button>
                  <input type="number" value={line.qty}
                    onChange={e => updateQty(i, parseInt(e.target.value) || 0)}
                    className="w-12 h-5 text-center rounded text-[10px] outline-none border"
                    style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  <span className="text-[9px] w-8 text-center" style={{ color: 'var(--brand-text-muted)' }}>{line.unit}</span>
                </div>
                <button type="button" onClick={() => onChange(lines.filter((_, j) => j !== i))}
                  className="w-5 h-5 rounded flex items-center justify-center hover:bg-[var(--color-danger-light)] transition-colors">
                  <i className="ti ti-x" style={{ fontSize: '0.6rem', color: 'var(--brand-text-muted)' }} />
                </button>
              </div>
            );
          })}
        </div>
      )}

      {lines.length > 0 && (
        <div className="p-3 rounded-lg border" style={{
          background: nutrition.complete ? 'rgba(5,150,105,0.05)' : 'var(--brand-surface-raised)',
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
                  style={{ background: 'rgba(217,119,6,0.1)', color: 'var(--color-warning)' }}>{t(`allergen.${a}`, a)}</span>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
