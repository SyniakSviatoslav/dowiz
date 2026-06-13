import { useState, useEffect, useMemo } from 'react';
import { Button, EmptyState, SkeletonBase, HintCard, useI18n, useConfirm } from '@deliveryos/ui';

type SupplyKind = 'food_ingredient' | 'condiment' | 'packaging' | 'utensil';

interface SupplyItem {
  id: string;
  name: string;
  kind: SupplyKind;
  category: string;
  baseUnit: string;
  kcalPer100: number | null;
  proteinMgPer100: number | null;
  fatMgPer100: number | null;
  carbMgPer100: number | null;
  allergens: string[];
  reorderThreshold: number | null;
  nutritionConfirmedAt: string | null;
  active: boolean;
  createdAt: string;
}

const STORAGE_KEY = 'dos_supplies';

function defaultSupplies(): SupplyItem[] {
  return [
    { id: 's1', name: 'Salmon fillet', kind: 'food_ingredient', category: 'Fish', baseUnit: 'g', kcalPer100: 208, proteinMgPer100: 20000, fatMgPer100: 13000, carbMgPer100: 0, allergens: ['fish'], reorderThreshold: 5000, nutritionConfirmedAt: '2026-06-01', active: true, createdAt: new Date().toISOString() },
    { id: 's2', name: 'Sushi rice', kind: 'food_ingredient', category: 'Grains', baseUnit: 'g', kcalPer100: 130, proteinMgPer100: 2700, fatMgPer100: 300, carbMgPer100: 28000, allergens: [], reorderThreshold: 10000, nutritionConfirmedAt: '2026-06-01', active: true, createdAt: new Date().toISOString() },
    { id: 's3', name: 'Nori sheets', kind: 'food_ingredient', category: 'Seaweed', baseUnit: 'unit', kcalPer100: 35, proteinMgPer100: 5800, fatMgPer100: 400, carbMgPer100: 5100, allergens: [], reorderThreshold: 100, nutritionConfirmedAt: null, active: true, createdAt: new Date().toISOString() },
    { id: 's4', name: 'Avocado', kind: 'food_ingredient', category: 'Vegetables', baseUnit: 'g', kcalPer100: 160, proteinMgPer100: 2000, fatMgPer100: 15000, carbMgPer100: 9000, allergens: [], reorderThreshold: 3000, nutritionConfirmedAt: '2026-06-01', active: true, createdAt: new Date().toISOString() },
    { id: 's5', name: 'Cream cheese', kind: 'food_ingredient', category: 'Dairy', baseUnit: 'g', kcalPer100: 342, proteinMgPer100: 6000, fatMgPer100: 34000, carbMgPer100: 4000, allergens: ['milk'], reorderThreshold: 2000, nutritionConfirmedAt: '2026-06-02', active: true, createdAt: new Date().toISOString() },
    { id: 's6', name: 'Shrimp', kind: 'food_ingredient', category: 'Seafood', baseUnit: 'g', kcalPer100: 85, proteinMgPer100: 20000, fatMgPer100: 700, carbMgPer100: 0, allergens: ['shellfish'], reorderThreshold: 2000, nutritionConfirmedAt: '2026-06-01', active: true, createdAt: new Date().toISOString() },
    { id: 's7', name: 'Spicy mayo', kind: 'condiment', category: 'Sauces', baseUnit: 'ml', kcalPer100: 500, proteinMgPer100: 1000, fatMgPer100: 55000, carbMgPer100: 2000, allergens: ['eggs'], reorderThreshold: 1000, nutritionConfirmedAt: null, active: true, createdAt: new Date().toISOString() },
    { id: 's8', name: 'Soy sauce', kind: 'condiment', category: 'Sauces', baseUnit: 'ml', kcalPer100: 53, proteinMgPer100: 8000, fatMgPer100: 100, carbMgPer100: 4900, allergens: ['soy', 'gluten'], reorderThreshold: 500, nutritionConfirmedAt: '2026-06-01', active: true, createdAt: new Date().toISOString() },
    { id: 's9', name: 'Sesame seeds', kind: 'food_ingredient', category: 'Seeds', baseUnit: 'g', kcalPer100: 573, proteinMgPer100: 17000, fatMgPer100: 50000, carbMgPer100: 23000, allergens: ['sesame'], reorderThreshold: 500, nutritionConfirmedAt: null, active: true, createdAt: new Date().toISOString() },
    { id: 's10', name: 'Eel sauce', kind: 'condiment', category: 'Sauces', baseUnit: 'ml', kcalPer100: 290, proteinMgPer100: 3000, fatMgPer100: 0, carbMgPer100: 68000, allergens: ['soy', 'gluten'], reorderThreshold: 500, nutritionConfirmedAt: null, active: true, createdAt: new Date().toISOString() },
    { id: 's11', name: 'Cucumber', kind: 'food_ingredient', category: 'Vegetables', baseUnit: 'g', kcalPer100: 15, proteinMgPer100: 700, fatMgPer100: 100, carbMgPer100: 3600, allergens: [], reorderThreshold: 3000, nutritionConfirmedAt: null, active: true, createdAt: new Date().toISOString() },
    { id: 's12', name: 'Wasabi', kind: 'condiment', category: 'Sauces', baseUnit: 'g', kcalPer100: 292, proteinMgPer100: 2000, fatMgPer100: 9000, carbMgPer100: 46000, allergens: [], reorderThreshold: 500, nutritionConfirmedAt: null, active: true, createdAt: new Date().toISOString() },
    { id: 's13', name: 'Takeout box (large)', kind: 'packaging', category: 'Containers', baseUnit: 'unit', kcalPer100: null, proteinMgPer100: null, fatMgPer100: null, carbMgPer100: null, allergens: [], reorderThreshold: 100, nutritionConfirmedAt: null, active: true, createdAt: new Date().toISOString() },
    { id: 's14', name: 'Chopsticks', kind: 'utensil', category: 'Utensils', baseUnit: 'unit', kcalPer100: null, proteinMgPer100: null, fatMgPer100: null, carbMgPer100: null, allergens: [], reorderThreshold: 200, nutritionConfirmedAt: null, active: true, createdAt: new Date().toISOString() },
    { id: 's15', name: 'Pickled ginger', kind: 'food_ingredient', category: 'Vegetables', baseUnit: 'g', kcalPer100: 60, proteinMgPer100: 300, fatMgPer100: 100, carbMgPer100: 14000, allergens: [], reorderThreshold: 500, nutritionConfirmedAt: null, active: true, createdAt: new Date().toISOString() },
  ];
}

// Shared utility to load/save from localStorage — used by both SupplyLibrary and RecipeEditor
export function loadSupplies(): SupplyItem[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return saveSupplies(defaultSupplies());
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    return saveSupplies(defaultSupplies());
  } catch {
    return saveSupplies(defaultSupplies());
  }
}

export function saveSupplies(supplies: SupplyItem[]): SupplyItem[] {
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(supplies)); } catch {
    console.debug('[SupplyLibrary] localStorage write failed');
  }
  return supplies;
}

export function getSupplyById(id: string): SupplyItem | undefined {
  return loadSupplies().find(s => s.id === id);
}

export function getActiveSupplies(): SupplyItem[] {
  return loadSupplies().filter(s => s.active);
}

// ── SupplyLibraryPage ──

const SupplyForm = ({
  initial,
  onSave,
  onCancel,
}: {
  initial?: SupplyItem | null;
  onSave: (item: SupplyItem) => void;
  onCancel: () => void;
}) => {
  const { t } = useI18n();
  const [name, setName] = useState(initial?.name || '');
  const [kind, setKind] = useState<SupplyKind>(initial?.kind || 'food_ingredient');
  const [category, setCategory] = useState(initial?.category || '');
  const [baseUnit, setBaseUnit] = useState(initial?.baseUnit || 'g');
  const [kcal, setKcal] = useState(initial?.kcalPer100?.toString() || '');
  const [protein, setProtein] = useState(initial?.proteinMgPer100 ? (initial.proteinMgPer100 / 1000).toString() : '');
  const [fat, setFat] = useState(initial?.fatMgPer100 ? (initial.fatMgPer100 / 1000).toString() : '');
  const [carbs, setCarbs] = useState(initial?.carbMgPer100 ? (initial.carbMgPer100 / 1000).toString() : '');
  const [allergens, setAllergens] = useState<string[]>(initial?.allergens || []);
  const [threshold, setThreshold] = useState(initial?.reorderThreshold?.toString() || '');
  const [saving, setSaving] = useState(false);
  const isFood = kind === 'food_ingredient' || kind === 'condiment';

  const ALL_ALLERGENS = ['gluten', 'shellfish', 'eggs', 'fish', 'peanuts', 'soy', 'milk', 'nuts', 'celery', 'mustard', 'sesame', 'sulphites', 'lupin', 'molluscs'];

  const handleSave = () => {
    if (!name.trim() || !category.trim()) return;
    setSaving(true);
    const item: SupplyItem = {
      id: initial?.id || `s_${Date.now()}`,
      name: name.trim(),
      kind,
      category: category.trim(),
      baseUnit,
      kcalPer100: isFood && kcal ? parseInt(kcal) : null,
      proteinMgPer100: isFood && protein ? Math.round(parseFloat(protein) * 1000) : null,
      fatMgPer100: isFood && fat ? Math.round(parseFloat(fat) * 1000) : null,
      carbMgPer100: isFood && carbs ? Math.round(parseFloat(carbs) * 1000) : null,
      allergens: isFood ? allergens : [],
      reorderThreshold: threshold ? parseInt(threshold) : null,
      nutritionConfirmedAt: initial?.nutritionConfirmedAt || null,
      active: initial?.active ?? true,
      createdAt: initial?.createdAt || new Date().toISOString(),
    };
    onSave(item);
    setTimeout(() => setSaving(false), 200);
  };

  const kindIcons: Record<SupplyKind, string> = {
    food_ingredient: 'ti ti-meat',
    condiment: 'ti ti-bottle',
    packaging: 'ti ti-box',
    utensil: 'ti ti-tool',
  };

  return (
    <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl p-5 space-y-4 slide-in-up">
      <h3 className="text-sm font-semibold" style={{ fontFamily: 'var(--brand-font-heading)' }}>
        {initial ? `${t('common.edit')}: ${name}` : t('menu.add') + ' ' + t('admin.supplies')}
      </h3>
      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.name', 'Name')}</label>
          <input value={name} onChange={e => setName(e.target.value)} placeholder={t('admin.eg_salmon', 'e.g. Salmon fillet')} className="w-full h-10 px-3 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
        </div>
        <div>
          <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.category', 'Category')}</label>
          <input value={category} onChange={e => setCategory(e.target.value)} placeholder={t('admin.eg_category', 'e.g. Fish, Dairy')} className="w-full h-10 px-3 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
        </div>
      </div>
      <div className="flex flex-wrap gap-1">
        <label className="text-[11px] font-medium block w-full mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.type', 'Type')}</label>
        {(['food_ingredient', 'condiment', 'packaging', 'utensil'] as SupplyKind[]).map(k => (
          <button key={k} type="button" onClick={() => setKind(k)}
            className={`flex items-center gap-1 px-2.5 py-1.5 rounded-md text-[10px] font-medium transition-all ${kind === k ? 'text-white' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}
            style={{ background: kind === k ? 'var(--brand-primary)' : 'var(--brand-surface-raised)' }}>
            <i className={kindIcons[k]} style={{ fontSize: '0.7rem' }} />
            {k === 'food_ingredient' ? t('supply.ingredient', 'Ingredient') : k === 'condiment' ? t('supply.sauce', 'Sauce') : k === 'packaging' ? t('supply.packaging', 'Packaging') : t('supply.utensil', 'Utensil')}
          </button>
        ))}
        <div className="w-full sm:w-auto sm:ml-2">
          <label className="text-[10px] block mb-0.5" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.unit', 'Unit')}</label>
          <select value={baseUnit} onChange={e => setBaseUnit(e.target.value)} className="h-8 px-2 rounded-md border text-[10px] outline-none" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
            <option value="g">g</option><option value="ml">ml</option><option value="unit">unit</option>
          </select>
        </div>
      </div>
      {isFood && (
        <div className="border-t pt-3 space-y-3" style={{ borderColor: 'var(--brand-border)' }}>
          <label className="text-[11px] font-medium block" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.nutrition_per_100', 'Nutrition per 100')}{baseUnit}</label>
          <div className="grid grid-cols-4 gap-2">
            {[{ label: t('admin.kcal', 'Kcal'), val: kcal, set: setKcal, ph: 'kcal' }, { label: t('admin.protein_g', 'Protein (g)'), val: protein, set: setProtein, ph: '0' }, { label: t('admin.fat_g', 'Fat (g)'), val: fat, set: setFat, ph: '0' }, { label: t('admin.carbs_g', 'Carbs (g)'), val: carbs, set: setCarbs, ph: '0' }].map(f => (
              <div key={f.label}>
                <input value={f.val} onChange={e => f.set(e.target.value)} type="number" placeholder={f.ph} className="w-full h-9 px-2 rounded-lg border text-xs outline-none focus:border-[var(--brand-primary)] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                <span className="text-[9px] block mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>{f.label}</span>
              </div>
            ))}
          </div>
          <label className="text-[11px] font-medium block" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.allergens', 'Allergens')}</label>
          <div className="flex flex-wrap gap-1">
            {ALL_ALLERGENS.map(a => {
              const active = allergens.includes(a);
              return (
                  <button key={a} type="button" onClick={() => setAllergens(prev => active ? prev.filter(x => x !== a) : [...prev, a])}
                    className={`px-2 py-1 rounded-full text-[10px] font-medium transition-all ${active ? 'text-white' : 'text-[var(--brand-text-muted)] hover:bg-[var(--brand-surface-raised)]'}`}
                    style={{ background: active ? 'var(--color-warning)' : 'var(--brand-border)' }}>{t(`allergen.${a.toLowerCase()}`, a)}</button>
              );
            })}
          </div>
        </div>
      )}
      <div className="border-t pt-3" style={{ borderColor: 'var(--brand-border)' }}>
        <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.reorder_threshold', 'Reorder threshold')} ({baseUnit})</label>
        <input value={threshold} onChange={e => setThreshold(e.target.value)} type="number" className="w-full sm:w-48 h-9 px-3 rounded-lg border text-xs outline-none focus:border-[var(--brand-primary)] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
      </div>
      <div className="flex justify-end gap-2 pt-2">
        <Button variant="ghost" size="sm" onClick={onCancel}>{t('common.cancel')}</Button>
        <Button size="sm" loading={saving} onClick={handleSave}>{t('common.save')}</Button>
      </div>
    </div>
  );
};

export function SupplyLibraryPage() {
  const [supplies, setSupplies] = useState<SupplyItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [kindFilter, setKindFilter] = useState<'all' | SupplyKind>('all');
  const [sortBy, setSortBy] = useState<'name' | 'category'>('name');
  const [sortOpen, setSortOpen] = useState(false);
  const [editing, setEditing] = useState<SupplyItem | null>(null);
  const [adding, setAdding] = useState(false);
  const { t } = useI18n();
  const { confirm: supplyConfirm, dialog: supplyConfirmDialog } = useConfirm();

  useEffect(() => {
    const s = loadSupplies();
    setSupplies(s);
    setLoading(false);
  }, []);

  const persistAndSet = (newSupplies: SupplyItem[]) => {
    saveSupplies(newSupplies);
    setSupplies(newSupplies);
  };

  const filtered = useMemo(() => {
    let result = supplies.filter(s => s.active);
    if (search) {
      const q = search.toLowerCase();
      result = result.filter(s => s.name.toLowerCase().includes(q) || s.category.toLowerCase().includes(q));
    }
    if (kindFilter !== 'all') result = result.filter(s => s.kind === kindFilter);
    result.sort((a, b) => {
      if (sortBy === 'name') return a.name.localeCompare(b.name);
      return a.category.localeCompare(b.category);
    });
    return result;
  }, [supplies, search, kindFilter, sortBy]);

  const handleSave = (item: SupplyItem) => {
    if (editing) {
      persistAndSet(supplies.map(s => s.id === item.id ? item : s));
      setEditing(null);
    } else {
      persistAndSet([item, ...supplies]);
      setAdding(false);
    }
  };

  const handleToggleActive = (id: string) => {
    const updated = supplies.map(s => s.id === id ? { ...s, active: !s.active } : s);
    persistAndSet(updated);
  };

  const handleDelete = async (id: string) => {
    const ok = await supplyConfirm({ title: t('admin.confirm_delete_supply_title', 'Delete supply'), message: t('admin.confirm_delete_supply', 'Are you sure you want to delete this supply item?'), confirmLabel: t('common.delete', 'Delete'), variant: 'danger' });
    if (!ok) return;
    persistAndSet(supplies.filter(s => s.id !== id));
  };

  const foodCount = supplies.filter(s => s.kind === 'food_ingredient').length;
  const condCount = supplies.filter(s => s.kind === 'condiment').length;
  const pkgCount = supplies.filter(s => s.kind === 'packaging').length;
  const utCount = supplies.filter(s => s.kind === 'utensil').length;

  const kindIcons: Record<string, string> = {
    all: 'ti ti-packages', food_ingredient: 'ti ti-meat', condiment: 'ti ti-bottle', packaging: 'ti ti-box', utensil: 'ti ti-tool',
  };
  const KINDS: Array<{ key: 'all' | SupplyKind; label: string }> = [
    { key: 'all', label: t('common.all') },
    { key: 'food_ingredient', label: t('admin.ingredients', 'Ingredients') },
    { key: 'condiment', label: t('admin.sauces', 'Sauces') },
    { key: 'packaging', label: t('admin.packaging', 'Packaging') },
    { key: 'utensil', label: t('admin.utensils', 'Utensils') },
  ];

  return (
    <>
    <div className="p-4 md:p-6 max-w-6xl mx-auto space-y-5">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.supplies')}</h2>
          <p className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>
            {foodCount} {t('supply.ingredient_short', 'ing')}, {condCount} {t('supply.sauces_short', 'sauces')}, {pkgCount} {t('supply.packaging_short', 'pkg')}, {utCount} {t('supply.utensils_short', 'utensils')}
          </p>
        </div>
        <Button onClick={() => { setAdding(true); setEditing(null); }}>
          <i className="ti ti-plus" /> {t('admin.add_supply', 'Add Supply')}
        </Button>
      </div>

      <HintCard title={t('admin.supplies')} description={t('admin.supplies_hint', 'Ingredients added here appear in product recipe editor. Define once, use everywhere.')} icon="ti ti-info-circle" />

      {adding && <SupplyForm onSave={handleSave} onCancel={() => setAdding(false)} />}

      <div className="flex gap-2">
        <div className="relative flex-1 sm:flex-none sm:w-64">
          <i className="ti ti-search absolute left-3 top-1/2 -translate-y-1/2 text-sm" style={{ color: 'var(--brand-text-muted)' }} />
          <input value={search} onChange={e => setSearch(e.target.value)} placeholder={t('common.search')}
            className="pl-9 pr-4 py-2 w-full rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
        </div>
        <div className="relative">
          <button onClick={() => setSortOpen(!sortOpen)}
            className="flex items-center gap-1.5 px-3 py-2 rounded-lg border text-sm outline-none"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
            <i className="ti ti-arrows-sort text-base" />
          </button>
          {sortOpen && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setSortOpen(false)} />
              <div className="absolute right-0 top-full mt-1 z-50 rounded-lg shadow-elevation-3 py-1 min-w-[140px] scale-in" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)' }}>
                {[
                  { value: 'name', label: t('admin.name_az', 'Name A-Z'), icon: 'ti ti-sort-az' },
                  { value: 'category', label: t('admin.category', 'Category'), icon: 'ti ti-folder' },
                ].map(opt => (
                  <button key={opt.value} onClick={() => { setSortBy(opt.value as any); setSortOpen(false); }}
                    className={`flex items-center gap-2 w-full px-3 py-2 text-xs transition-colors hover:bg-[var(--brand-surface-raised)] ${sortBy === opt.value ? 'font-semibold' : ''}`}
                    style={{ color: sortBy === opt.value ? 'var(--brand-primary)' : 'var(--brand-text)' }}>
                    <i className={opt.icon} style={{ fontSize: '0.8rem' }} />
                    <span className="flex-1">{opt.label}</span>
                    {sortBy === opt.value && <i className="ti ti-check" style={{ color: 'var(--brand-primary)', fontSize: '0.7rem' }} />}
                  </button>
                ))}
              </div>
            </>
          )}
        </div>
        <div className="flex overflow-x-auto hide-scrollbar gap-1 pb-1 snap-x snap-mandatory flex-1" style={{ background: 'var(--brand-bg)' }}>
          {KINDS.map(k => (
            <button key={k.key} onClick={() => setKindFilter(k.key)}
              className={`flex items-center gap-1 px-3 py-1.5 text-[11px] font-medium rounded-md transition-all snap-start shrink-0 whitespace-nowrap ${kindFilter === k.key ? 'bg-[var(--brand-primary)] text-white shadow-sm' : 'bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}>
              <i className={kindIcons[k.key]} style={{ fontSize: '0.8rem' }} />{k.label}
            </button>
          ))}
        </div>
      </div>

      {loading ? (
        <div className="space-y-2">{[1,2,3,4,5].map(i => <SkeletonBase key={i} className="h-16 w-full rounded-xl" />)}</div>
      ) : filtered.length === 0 ? (
        <EmptyState title={t('common.no_data')} description={search ? t('admin.no_supplies_match', 'No supplies match.') : t('admin.add_first_supply', 'Add your first supply to start.')} icon={<i className="ti ti-packages text-4xl" style={{ opacity: 0.3 }} />} />
      ) : (
        <div className="space-y-1">
          {editing && (
            <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center fade-in" onClick={() => setEditing(null)}>
              <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" />
              <div className="relative w-full max-w-lg mx-4 mb-0 sm:mb-auto max-h-[85vh] overflow-auto rounded-t-2xl sm:rounded-2xl" onClick={e => e.stopPropagation()}>
                <SupplyForm initial={editing} onSave={handleSave} onCancel={() => setEditing(null)} />
              </div>
            </div>
          )}
          {filtered.map((supply, i) => {
            const ico = kindIcons[supply.kind] || 'ti ti-circle';
            const icoColor = supply.kind === 'food_ingredient' ? 'var(--color-success)' : supply.kind === 'condiment' ? 'var(--color-warning)' : supply.kind === 'packaging' ? 'var(--color-info)' : 'var(--brand-text-muted)';
            return (
              <div key={supply.id} className={`flex items-center gap-3 p-3 rounded-xl border transition-all duration-200 hover:bg-[var(--brand-surface)] slide-in-up`}
                style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', animationDelay: `${i * 30}ms` }}>
                <div className="w-10 h-10 rounded-lg flex items-center justify-center shrink-0" style={{ background: 'var(--brand-primary-light)' }}>
                  <i className={ico} style={{ fontSize: '1rem', color: icoColor }} />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium truncate" style={{ color: 'var(--brand-text)' }}>{supply.name}</span>
                    <span className="text-[10px] px-1.5 py-0.5 rounded font-mono uppercase" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                      {supply.kind === 'food_ingredient' ? t('supply.ingredient_short') : supply.kind === 'condiment' ? t('supply.sauces_short') : supply.kind === 'packaging' ? t('supply.packaging_short') : t('supply.utensils_short')}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 text-[11px] mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>
                    <span>{supply.category}</span><span>·</span><span>{supply.baseUnit}</span>
                    {supply.kcalPer100 && <><span>·</span><span>{supply.kcalPer100} kcal/100{supply.baseUnit}</span></>}
                    {!supply.nutritionConfirmedAt && (supply.kind === 'food_ingredient' || supply.kind === 'condiment') && (
                      <span className="px-1.5 py-0.5 rounded text-[10px]" style={{ background: 'var(--color-warning-light)', color: 'var(--color-warning)' }}>{t('admin.unconfirmed', 'unconfirmed')}</span>
                    )}
                  </div>
                  {supply.allergens.length > 0 && (
                    <div className="flex gap-1 mt-1 flex-wrap">{supply.allergens.map(a => (
                      <span key={a} className="px-1.5 py-0.5 rounded-full text-[9px] font-medium" style={{ background: 'rgba(217,119,6,0.1)', color: 'var(--color-warning)' }}>{t(`allergen.${a}`, a)}</span>
                    ))}</div>
                  )}
                </div>
                <div className="flex items-center gap-1 shrink-0">
                  <button onClick={() => setEditing(supply)} className="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-[var(--brand-surface-raised)] transition-colors" title={t('common.edit', 'Edit')}>
                    <i className="ti ti-edit" style={{ fontSize: '0.85rem', color: 'var(--brand-text-muted)' }} />
                  </button>
                  <button onClick={() => handleDelete(supply.id)} className="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-[var(--color-danger-light)] transition-colors" title={t('common.delete', 'Delete')}>
                    <i className="ti ti-trash" style={{ fontSize: '0.85rem', color: 'var(--brand-text-muted)' }} />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
      {supplyConfirmDialog}
    </>
  );
}
