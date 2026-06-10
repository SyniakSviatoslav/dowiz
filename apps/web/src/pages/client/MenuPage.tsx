import React, { useEffect, useState, useRef, useCallback, useMemo } from 'react';
import { useParams } from 'react-router-dom';
import { ProductCard, useI18n } from '@deliveryos/ui';
import { useSharedCart } from '../../lib/CartProvider.js';

interface ProductModifier {
  id: string;
  name: string;
  price_delta: number;
  available: boolean;
  sort_order: number;
}

interface ModifierGroup {
  id: string;
  name: string;
  min_select: number;
  max_select: number;
  required: boolean;
  sort_order: number;
  modifiers: ProductModifier[];
}

interface Product {
  id: string;
  name: string;
  description?: string;
  price: number;
  available: boolean;
  image_key?: string | null;
  attributes?: any;
  modifier_groups?: ModifierGroup[];
}

interface MenuCategory {
  id: string;
  name: string;
  sort_order: number;
  products: Product[];
}

interface MenuResponse {
  menu_version: number;
  default_locale: string;
  supported_locales: string[];
  currency: { code: string; minor_unit: number } | string;
  location_name?: string;
  categories: MenuCategory[];
}

const getCurrency = (m: MenuResponse | null): string => {
  if (!m) return 'ALL';
  if (typeof m.currency === 'string') return m.currency;
  return m.currency.code;
};

export function MenuPage() {
  const { slug } = useParams<{ slug: string }>();
  const { t } = useI18n();
  const getAttr = (p: Product, key: string): any => {
    if (!p.attributes || typeof p.attributes !== 'object') return undefined;
    return (p.attributes as Record<string, any>)[key];
  };

  const extractLineAllergens = (line: any): string[] => {
    if (Array.isArray(line.allergens)) return line.allergens;
    if (typeof line.allergens === 'string' && line.allergens) return line.allergens.split(',').map((s: string) => s.trim()).filter(Boolean);
    return [];
  };

  const bomToNutrition = (p: Product) => {
    const bom = getAttr(p, 'bom');
    if (!Array.isArray(bom) || bom.length === 0) return { kcal: 0, protein: 0, fat: 0, carbs: 0, allergens: [], ingredients: [] };
    let kcal = 0, protein = 0, fat = 0, carbs = 0;
    const allergens = new Set<string>();
    const ingredients: string[] = [];
    for (const line of bom) {
      if (typeof line.kcal === 'number') kcal += line.kcal;
      if (typeof line.proteinG === 'number') protein += line.proteinG;
      if (typeof line.fatG === 'number') fat += line.fatG;
      if (typeof line.carbsG === 'number') carbs += line.carbsG;
      extractLineAllergens(line).forEach((a: string) => allergens.add(a));
      if (line.supplyName && line.kind !== 'packaging' && line.kind !== 'utensil') ingredients.push(line.supplyName);
    }
    return { kcal: Math.round(kcal), protein: Math.round(protein), fat: Math.round(fat), carbs: Math.round(carbs), allergens: Array.from(allergens), ingredients };
  };

  const [menu, setMenu] = useState<MenuResponse | null>(null);
  const [data, setData] = useState<MenuCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<string>('');
  const { addItem, bounceCart } = useSharedCart();
  const [detailProduct, setDetailProduct] = useState<Product | null>(null);
  const [selectedModifiers, setSelectedModifiers] = useState<Record<string, string[]>>({});
  const [modifierGroupSelection, setModifierGroupSelection] = useState<Record<string, string[]>>({});
  const [quantity, setQuantity] = useState(1);
  const [imageLoadError, setImageLoadError] = useState(false);
  const [sortBy, setSortBy] = useState<'default' | 'price-asc' | 'price-desc' | 'name'>('default');
  const [filterAllergen, setFilterAllergen] = useState<string | null>(null);

  const categories = data;

  const displayCategories = useMemo(() => {
    if (sortBy === 'default' && !filterAllergen) return categories;
    const all: (Product & { _catId: string; _catName: string })[] = [];
    for (const cat of categories) {
      for (const p of cat.products) {
        all.push({ ...p, _catId: cat.id, _catName: cat.name });
      }
    }
    const filtered = filterAllergen
      ? all.filter(p => bomToNutrition(p).allergens.includes(filterAllergen))
      : all;
    const sorted = sortBy === 'default' ? filtered
      : sortBy === 'price-asc' ? [...filtered].sort((a, b) => a.price - b.price)
      : sortBy === 'price-desc' ? [...filtered].sort((a, b) => b.price - a.price)
      : [...filtered].sort((a, b) => a.name.localeCompare(b.name));
    const groups: MenuCategory[] = [];
    for (const p of sorted) {
      const g = groups.find(g => g.id === p._catId);
      if (g) g.products.push(p);
      else groups.push({ id: p._catId, name: p._catName, sort_order: 0, products: [p] });
    }
    return groups;
  }, [categories, sortBy, filterAllergen]);

  const allAllergens = useMemo(() => {
    const set = new Set<string>();
    for (const cat of data) for (const p of cat.products) {
      const bom = getAttr(p, 'bom');
      if (Array.isArray(bom)) for (const line of bom) {
        extractLineAllergens(line).forEach(a => set.add(a));
      }
    }
    return Array.from(set).sort();
  }, [data]);

  const allTasteAxes = useMemo(() => {
    const set = new Set<string>();
    for (const cat of data) for (const p of cat.products) {
      const taste = getAttr(p, 'taste');
      if (taste && typeof taste === 'object') for (const k of Object.keys(taste)) if ((taste as any)[k] > 0) set.add(k);
    }
    return Array.from(set);
  }, [data]);

  useEffect(() => {
    async function load() {
      try {
        const res = await fetch(`/public/locations/${slug}/menu`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const menuData: MenuResponse = await res.json();
        setMenu(menuData);
        const cats = menuData.categories || [];
        setData(cats);
        if (cats[0]) setActiveTab(cats[0].id);
        setLoading(false);
      } catch (err) {
        console.error('[MenuPage] Failed to load menu:', err);
        setMenu(null);
        setData([]);
        setLoading(false);
      }
    }
    load();
  }, [slug]);

  const sectionRefs = useRef<Record<string, HTMLElement | null>>({});
  useEffect(() => {
    if (loading) return;
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          setActiveTab(entry.target.id);
        }
      }
    }, { rootMargin: '-120px 0px -60% 0px' });

    Object.values(sectionRefs.current).forEach(el => {
      if (el) observer.observe(el);
    });
    return () => observer.disconnect();
  }, [loading, categories]);

  const handleScrollTo = (id: string) => {
    const el = document.getElementById(id);
    if (el) {
      const top = el.getBoundingClientRect().top + window.scrollY - 110;
      window.scrollTo({ top, behavior: 'smooth' });
    }
  };

  const handleProductClick = (product: Product) => {
    setDetailProduct(product);
    setQuantity(1);
    setImageLoadError(false);
    const initial: Record<string, string[]> = {};
    const groups = product.modifier_groups || [];
    for (const g of groups) {
      initial[g.id] = [];
      if (g.required && g.min_select > 0 && g.modifiers.length > 0) {
        const firstAvailable = g.modifiers.find(m => m.available);
        if (firstAvailable) initial[g.id] = [firstAvailable.id];
      }
    }
    setModifierGroupSelection(initial);
  };

  const closeDetail = () => {
    setDetailProduct(null);
    setModifierGroupSelection({});
    setQuantity(1);
  };

  const toggleModifier = (groupId: string, modifierId: string, group: ModifierGroup) => {
    setModifierGroupSelection(prev => {
      const current = [...(prev[groupId] || [])];
      if (group.max_select === 1) {
        if (current[0] === modifierId) return prev;
        return { ...prev, [groupId]: [modifierId] };
      }
      const idx = current.indexOf(modifierId);
      if (idx >= 0) {
        const filtered = current.filter(m => m !== modifierId);
        if (filtered.length < group.min_select) return prev;
        return { ...prev, [groupId]: filtered };
      }
      if (current.length >= group.max_select) return prev;
      return { ...prev, [groupId]: [...current, modifierId] };
    });
  };

  const calcModifierDelta = useCallback((): number => {
    if (!detailProduct) return 0;
    const groups = detailProduct.modifier_groups || [];
    let delta = 0;
    for (const g of groups) {
      const selected = modifierGroupSelection[g.id] || [];
      for (const selId of selected) {
        const mod = g.modifiers.find(m => m.id === selId);
        if (mod) delta += mod.price_delta;
      }
    }
    return delta;
  }, [detailProduct, modifierGroupSelection]);

  const makeCartItemId = (productId: string, options: Record<string, string[]>) => {
    const opts = JSON.stringify(options);
    let hash = 0;
    for (let i = 0; i < opts.length; i++) { hash = ((hash << 5) - hash + opts.charCodeAt(i)) | 0; }
    return `${productId}_${Math.abs(hash).toString(36)}`;
  };

  const handleAddDetail = () => {
    if (!detailProduct || !detailProduct.available) return;
    addItem({
      id: makeCartItemId(detailProduct.id, modifierGroupSelection),
      productId: detailProduct.id,
      name: detailProduct.name,
      quantity: quantity,
      price: detailProduct.price + calcModifierDelta(),
      options: modifierGroupSelection,
    });
    bounceCart();
    closeDetail();
  };

  const canAdd = (): boolean => {
    if (!detailProduct || !detailProduct.available) return false;
    const groups = detailProduct.modifier_groups || [];
    for (const g of groups) {
      if (!g.required) continue;
      const selected = modifierGroupSelection[g.id] || [];
      if (selected.length < g.min_select) return false;
    }
    return true;
  };

  const getImageUrl = (product: Product): string | null => {
    if (!product.image_key) return null;
    if (product.image_key.startsWith('data:') || product.image_key.startsWith('http://') || product.image_key.startsWith('https://')) {
      return product.image_key;
    }
    return `https://cdn.dowiz.org/${product.image_key}`;
  };

  const ALLERGEN_COLORS: Record<string, { bg: string; text: string }> = {
    gluten: { bg: 'rgba(234,179,8,0.12)', text: '#a16207' },
    dairy: { bg: 'rgba(59,130,246,0.12)', text: '#1d4ed8' },
    eggs: { bg: 'rgba(234,179,8,0.12)', text: '#a16207' },
    soy: { bg: 'rgba(34,197,94,0.12)', text: '#15803d' },
    nuts: { bg: 'rgba(249,115,22,0.12)', text: '#c2410c' },
    peanuts: { bg: 'rgba(249,115,22,0.12)', text: '#c2410c' },
    shellfish: { bg: 'rgba(239,68,68,0.12)', text: '#b91c1c' },
    fish: { bg: 'rgba(6,182,212,0.12)', text: '#0e7490' },
    sesame: { bg: 'rgba(168,85,247,0.12)', text: '#7e22ce' },
  };

  function getAllergenStyle(allergen: string) {
    const key = allergen.toLowerCase();
    return ALLERGEN_COLORS[key] || { bg: 'rgba(107,114,128,0.12)', text: '#374151' };
  };

  const attrEntries = (p: Product): [string, any][] => {
    if (!p.attributes || typeof p.attributes !== 'object') return [];
    return Object.entries(p.attributes as Record<string, any>).filter(([k]) => !['kcal', 'protein', 'fat', 'carbs', 'allergens', 'tags', 'taste', 'bom', 'stock_count'].includes(k));
  };

  return (
    <div className="relative min-h-screen pb-28">

      {/* Hero Section */}
      <section className="relative w-full h-[200px] flex items-end overflow-hidden" style={{ background: 'linear-gradient(160deg, var(--brand-surface-raised) 0%, var(--brand-accent) 60%, var(--brand-primary) 100%)' }}>
        <div className="absolute inset-0" style={{ background: 'linear-gradient(to top, rgba(0,0,0,0.75) 0%, rgba(0,0,0,0.3) 45%, rgba(0,0,0,0.05) 100%)' }} />
        <div className="absolute inset-0 opacity-[0.04] bg-[radial-gradient(circle_at_30%_50%,#fff_0%,transparent_60%)]" />
        <div className="relative z-10 w-full px-5 pb-4">
          <div className="flex items-center gap-1.5 text-[12px] font-medium mb-1.5" style={{ color: 'rgba(255,255,255,0.65)' }}>
            <span className="inline-flex gap-0.5" style={{ color: 'var(--color-warning)' }}>
              {[1,2,3,4,5].map(i => <i key={i} className="ti ti-star-filled" style={{ fontSize: '0.7rem' }} />)}
            </span>
            <span style={{ color: '#fff', fontWeight: 600 }}>4.8</span>
            <span className="opacity-70">(124)</span>
            <span className="mx-1.5 opacity-40">·</span>
            <i className="ti ti-clock" style={{ fontSize: '0.7rem' }} />
            <span>30 min</span>
          </div>
          <h1 className="text-[26px] font-bold leading-tight text-white" style={{ fontFamily: 'var(--brand-font-heading)', textShadow: '0 2px 16px rgba(0,0,0,0.4)' }}>
            {menu?.location_name || t('client.menu', 'Menu')}
          </h1>
        </div>
      </section>

      {/* Category Nav */}
      <nav className="sticky top-0 z-40 h-[44px] border-b w-full" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
        <div className="h-full overflow-x-auto hide-scrollbar flex items-center gap-1 px-2 text-[13px]">
          {loading ? (
            <div className="flex gap-4 px-2 h-full items-center">
              <div className="w-14 h-3.5 skeleton-block" />
              <div className="w-14 h-3.5 skeleton-block" />
              <div className="w-14 h-3.5 skeleton-block" />
            </div>
          ) : (
            categories.map(cat => {
              const count = cat.products.filter(p => p.available).length;
              return (
                <button 
                  key={cat.id}
                  onClick={() => handleScrollTo(cat.id)}
                  role="tab"
                  aria-selected={activeTab === cat.id}
                  className="h-full flex items-center gap-1.5 px-3.5 whitespace-nowrap font-medium transition-all border-b-2"
                  style={{ 
                    color: activeTab === cat.id ? 'var(--brand-primary)' : 'var(--brand-text-muted)',
                    borderColor: activeTab === cat.id ? 'var(--brand-primary)' : 'transparent',
                  }}
                >
                  {cat.name}
                  <span className="text-[10px] opacity-50">({count})</span>
                </button>
              );
            })
          )}
        </div>
      </nav>

      {/* Sort & Filter Bar */}
      {!loading && (allAllergens.length > 0 || sortBy !== 'default') && (
        <div className="sticky top-[44px] z-30 border-b px-4 py-2 flex items-center gap-2 overflow-x-auto hide-scrollbar" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
          <div className="flex items-center gap-1.5 shrink-0">
            <span className="text-[10px] font-semibold uppercase tracking-wider" style={{ color: 'var(--brand-text-muted)' }}>
              <i className="ti ti-arrows-sort" style={{ fontSize: '0.7rem' }} />
            </span>
            {(['default', 'price-asc', 'price-desc', 'name'] as const).map(mode => (
              <button key={mode} onClick={() => setSortBy(mode)}
                className="px-2 py-0.5 rounded-md text-[10px] font-medium transition-all whitespace-nowrap"
                style={{
                  background: sortBy === mode ? 'var(--brand-primary)' : 'var(--brand-surface-raised)',
                  color: sortBy === mode ? '#fff' : 'var(--brand-text-muted)',
                }}
              >
                {mode === 'default' ? '·' : mode === 'price-asc' ? '↑ Price' : mode === 'price-desc' ? '↓ Price' : 'A-Z'}
              </button>
            ))}
          </div>
          {allAllergens.length > 0 && (
            <div className="h-4 w-px shrink-0" style={{ background: 'var(--brand-border)' }} />
          )}
          <div className="flex items-center gap-1 overflow-x-auto hide-scrollbar">
            {allAllergens.map(a => {
              const s = getAllergenStyle(a);
              return (
                <button key={a} onClick={() => setFilterAllergen(filterAllergen === a ? null : a)}
                  className="px-1.5 py-0.5 rounded text-[9px] font-semibold uppercase whitespace-nowrap transition-all"
                  style={{
                    background: filterAllergen === a ? s.text : s.bg,
                    color: filterAllergen === a ? '#fff' : s.text,
                    opacity: filterAllergen && filterAllergen !== a ? 0.3 : 1,
                  }}
                >
                  {a}
                </button>
              );
            })}
          </div>
        </div>
      )}

      {/* Menu Content */}
      <main className="max-w-5xl mx-auto pt-4">
        {loading ? (
          <div className="px-4 grid grid-cols-2 md:grid-cols-3 gap-3">
            {[1,2,3,4,5,6].map(i => (
              <div key={i} className="rounded-xl overflow-hidden border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
                <div className="w-full aspect-[4/3] skeleton-block rounded-none" />
                <div className="p-3">
                  <div className="h-3 w-3/4 skeleton-block mb-3" />
                  <div className="h-2 w-full skeleton-block mb-1.5" />
                  <div className="h-2 w-4/5 skeleton-block mb-4" />
                  <div className="flex justify-between items-center pt-2">
                    <div className="h-4 w-16 skeleton-block" />
                    <div className="w-9 h-9 rounded-full skeleton-block" />
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : !categories.length ? (
          <div className="flex flex-col items-center justify-center py-20 px-4 text-center">
            <i className="ti ti-tools-kitchen-2 text-5xl opacity-20 mb-3" style={{ color: 'var(--brand-text-muted)' }} />
            <p className="text-sm font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('client.empty_menu', 'Menu unavailable')}</p>
          </div>
) : (
          displayCategories.map(category => (
            <section 
              key={category.id} 
              id={category.id} 
              ref={el => { sectionRefs.current[category.id] = el }}
              className="mb-7 scroll-mt-[100px]"
            >
              <h2 className="text-lg font-bold px-4 mb-3" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' }}>
                {category.name}
              </h2>
              <div className="grid grid-cols-2 md:grid-cols-3 gap-3 px-4">
                {category.products.map(product => {
                  const nutrition = bomToNutrition(product);
                  return (
                  <div key={product.id}>
                    <ProductCard product={{
                      id: product.id,
                      name: product.name,
                      description: product.description,
                      price: product.price,
                      image: getImageUrl(product) || undefined,
                      isAvailable: product.available,
                      kcal: nutrition.kcal || undefined,
                      protein: nutrition.protein || undefined,
                      fat: nutrition.fat || undefined,
                      carbs: nutrition.carbs || undefined,
                      allergens: nutrition.allergens.length ? nutrition.allergens : undefined,
                      ingredients: nutrition.ingredients.length ? nutrition.ingredients : undefined,
                      taste: getAttr(product, 'taste'),
                    }}
                    onClick={() => handleProductClick(product)}
                    onAdd={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      if (!product.available) return;
                      if (!product.modifier_groups?.length) {
                        addItem({ id: `cart_${product.id}`, productId: product.id, name: product.name, quantity: 1, price: product.price, options: {} });
                        bounceCart();
                      } else {
                        handleProductClick(product);
                      }
                    }} />
                  </div>
                  );
                })}
              </div>
            </section>
          ))
        )}
       </main>

      {/* Product Detail Modal */}
      {detailProduct && (
        <div className="fixed inset-0 z-50 flex items-end md:items-center justify-center backdrop-blur-sm transition-opacity duration-300" style={{ background: 'rgba(0,0,0,0.6)' }} onClick={closeDetail}>
          <div 
            className="w-full md:max-w-lg max-h-[85vh] overflow-auto rounded-t-2xl md:rounded-2xl shadow-2xl animate-slide-up" 
            style={{ background: 'var(--brand-bg)' }}
            onClick={e => e.stopPropagation()}
          >
            {/* Image */}
            <div className="relative w-full aspect-[16/9] md:aspect-[2/1] flex items-center justify-center overflow-hidden" style={{ background: 'var(--brand-surface-raised)' }}>
              {getImageUrl(detailProduct) && !imageLoadError ? (
                <img 
                  src={getImageUrl(detailProduct)!} 
                  alt={detailProduct.name} 
                  className="w-full h-full object-cover"
                  onError={() => setImageLoadError(true)}
                />
              ) : (
                <div className="flex flex-col items-center gap-2" style={{ color: 'var(--brand-text-muted)' }}>
                  <i className="ti ti-tools-kitchen-2 text-5xl opacity-30" />
                  <span className="text-sm font-medium opacity-60">{detailProduct.name}</span>
                </div>
              )}
              <button 
                className="absolute top-4 right-4 min-w-[44px] min-h-[44px] rounded-full flex items-center justify-center backdrop-blur-md active:scale-[0.95] transition-transform"
                style={{ background: 'rgba(0,0,0,0.5)', color: 'white' }}
                onClick={closeDetail}
                aria-label="Close"
              >
                <i className="ti ti-x text-xl" />
              </button>
              {detailProduct.available && bomToNutrition(detailProduct).kcal > 0 && (
                <div className="absolute bottom-3 left-3 z-10">
                  <span className="text-[10px] font-medium px-2 py-1 rounded-md flex items-center gap-1.5" style={{ background: 'rgba(0,0,0,0.6)', color: '#fff' }}>
                    <i className="ti ti-flame" style={{ fontSize: '0.7rem' }} />
                    {bomToNutrition(detailProduct).kcal} kcal
                    {bomToNutrition(detailProduct).protein > 0 && <span className="opacity-70">· P{bomToNutrition(detailProduct).protein}g</span>}
                    {bomToNutrition(detailProduct).fat > 0 && <span className="opacity-70">· F{bomToNutrition(detailProduct).fat}g</span>}
                    {bomToNutrition(detailProduct).carbs > 0 && <span className="opacity-70">· C{bomToNutrition(detailProduct).carbs}g</span>}
                  </span>
                </div>
              )}
            </div>

            {/* Content */}
            <div className="p-5 space-y-5">
              {/* Name, Description, Price */}
              <div>
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1.5">
                      {t('client.recommended', '') && (
                        <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full inline-flex items-center gap-1" style={{ background: 'rgba(234,79,22,0.1)', color: 'var(--brand-primary)' }}>
                          <i className="ti ti-flame" style={{ fontSize: '0.6rem' }} />
                          {t('client.popular', 'Popular')}
                        </span>
                      )}
                    </div>
                    <h2 className="text-xl font-bold leading-tight" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{detailProduct.name}</h2>
                    {!detailProduct.available && (
                      <span className="inline-block mt-1.5 text-[10px] font-semibold px-2 py-0.5 rounded" style={{ background: 'rgba(220,38,38,0.1)', color: 'var(--color-danger)' }}>
                        {t('client.unavailable', 'Unavailable')}
                      </span>
                    )}
                  </div>
                  <div className="text-xl font-black whitespace-nowrap shrink-0" style={{ color: 'var(--brand-primary)' }}>
                    {(detailProduct.price + calcModifierDelta()).toLocaleString()} {getCurrency(menu)}
                  </div>
                </div>
                {detailProduct.description && (
                  <p className="text-sm mt-2 leading-relaxed" style={{ color: 'var(--brand-text-muted)' }}>{detailProduct.description}</p>
                )}
              </div>

              {/* Taste Section */}
              {(() => {
                const taste = getAttr(detailProduct, 'taste');
                if (!taste || typeof taste !== 'object') return null;
                const entries = Object.entries(taste).filter(([, v]) => (v as number) > 0);
                if (!entries.length) return null;
                const icons: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };
                return (
                  <div className="rounded-xl p-4" style={{ background: 'var(--brand-surface)' }}>
                    <h3 className="text-xs font-semibold uppercase tracking-wider mb-3 flex items-center gap-1.5" style={{ color: 'var(--brand-text-muted)' }}>
                      <i className="ti ti-flask" /> {t('common.taste', 'Taste')}
                    </h3>
                    <div className="flex flex-wrap gap-x-4 gap-y-2">
                      {entries.map(([axis, level]) => (
                        <span key={axis} className="inline-flex items-center gap-1 text-sm" style={{ color: 'var(--brand-text-muted)' }}>
                          <i className={icons[axis] || 'ti ti-circle'} style={{ fontSize: '0.75rem' }} />
                          {Array.from({ length: level as number }).map((_, i) => (
                            <i key={i} className={icons[axis] || 'ti ti-circle'} style={{ fontSize: '0.65rem' }} />
                          ))}
                        </span>
                      ))}
                    </div>
                  </div>
                );
              })()}

              {/* Nutrition Section */}
              {(bomToNutrition(detailProduct).kcal > 0) && (
                <div className="rounded-xl p-4" style={{ background: 'var(--brand-surface)' }}>
                  <h3 className="text-xs font-semibold uppercase tracking-wider mb-3 flex items-center gap-1.5" style={{ color: 'var(--brand-text-muted)' }}>
                    <i className="ti ti-report-analytics" /> {t('client.nutrition', 'Nutrition')}
                  </h3>
                  <div className="grid grid-cols-4 gap-3 text-center">
                    {[
                      { label: 'Calories', value: bomToNutrition(detailProduct).kcal, unit: 'kcal', icon: 'ti ti-flame' },
                      { label: 'Protein', value: bomToNutrition(detailProduct).protein, unit: 'g', icon: 'ti ti-droplet' },
                      { label: 'Fat', value: bomToNutrition(detailProduct).fat, unit: 'g', icon: 'ti ti-droplet-half' },
                      { label: 'Carbs', value: bomToNutrition(detailProduct).carbs, unit: 'g', icon: 'ti ti-droplet-filled' },
                    ].map(n => n.value > 0 && (
                      <div key={n.label} className="flex flex-col items-center gap-1">
                        <i className={n.icon} style={{ fontSize: '1rem', color: 'var(--brand-text-muted)' }} />
                        <span className="text-sm font-bold" style={{ color: 'var(--brand-text)' }}>{n.value}</span>
                        <span className="text-[9px] font-medium" style={{ color: 'var(--brand-text-muted)' }}>{n.label}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {(bomToNutrition(detailProduct).allergens.length > 0) && (
                <div className="rounded-xl p-4" style={{ background: 'rgba(220,38,38,0.06)', borderColor: 'rgba(220,38,38,0.15)', borderWidth: 1 }}>
                  <h3 className="text-xs font-semibold uppercase tracking-wider mb-2.5 flex items-center gap-1.5" style={{ color: 'var(--color-danger)' }}>
                    <i className="ti ti-alert-triangle" /> {t('client.allergens', 'Allergens')}
                  </h3>
                  <div className="flex flex-wrap gap-1.5">
                    {bomToNutrition(detailProduct).allergens.map(a => {
                      const s = getAllergenStyle(a);
                      return (
                        <span key={a} className="px-2 py-0.5 rounded font-semibold text-[10px] uppercase" style={{ background: s.bg, color: s.text }}>
                          {a}
                        </span>
                      );
                    })}
                  </div>
                </div>
              )}

              {(bomToNutrition(detailProduct).ingredients.length > 0) && (
                <div className="rounded-xl p-4" style={{ background: 'var(--brand-surface)' }}>
                  <h3 className="text-xs font-semibold uppercase tracking-wider mb-2.5 flex items-center gap-1.5" style={{ color: 'var(--brand-text-muted)' }}>
                    <i className="ti ti-list-check" /> {t('client.ingredients', 'Ingredients')}
                  </h3>
                  <div className="flex flex-wrap gap-1.5">
                    {bomToNutrition(detailProduct).ingredients.map((ing, i) => (
                      <span key={i} className="text-[11px] px-2 py-0.5 rounded-md" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}>
                        {ing}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {/* Modifier Groups */}
              {(detailProduct.modifier_groups || []).length > 0 && (
                <div className="border-t pt-5" style={{ borderColor: 'var(--brand-border)' }}>
                  <h3 className="text-xs font-semibold uppercase tracking-wider mb-3 flex items-center gap-1.5" style={{ color: 'var(--brand-text-muted)' }}>
                    <i className="ti ti-settings" /> {t('client.customize', 'Customize')}
                  </h3>
                  {(detailProduct.modifier_groups || []).map(group => (
                    <div key={group.id} className="mb-4 last:mb-0">
                      <div className="flex items-center gap-2 mb-2.5">
                        <span className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>{group.name}</span>
                        {group.required && (
                          <span className="text-[9px] px-1.5 py-0.5 rounded font-medium" style={{ background: 'rgba(220,38,38,0.08)', color: 'var(--color-danger)' }}>
                            {t('client.required', 'Required')}
                          </span>
                        )}
                        {group.max_select > 1 && (
                          <span className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>
                            up to {group.max_select}
                          </span>
                        )}
                      </div>
                      <div className="flex flex-wrap gap-2">
                        {group.modifiers.filter(m => m.available).map(mod => {
                          const selected = modifierGroupSelection[group.id] || [];
                          const isSelected = selected.includes(mod.id);
                          return (
                            <button
                              key={mod.id}
                              onClick={() => toggleModifier(group.id, mod.id, group)}
                              className={`px-3.5 py-2 rounded-[10px] text-[13px] font-medium transition-all active:scale-[0.97] border ${
                                isSelected ? 'border-2' : ''
                              }`}
                              style={{
                                background: isSelected ? 'var(--brand-primary-light, var(--brand-surface-raised))' : 'var(--brand-surface)',
                                borderColor: isSelected ? 'var(--brand-primary)' : 'var(--brand-border)',
                                color: isSelected ? 'var(--brand-primary)' : 'var(--brand-text)',
                              }}
                            >
                              {mod.name}
                              {mod.price_delta > 0 && (
                                <span className="ml-1 text-[11px]" style={{ color: isSelected ? 'var(--brand-primary)' : 'var(--brand-text-muted)' }}>
                                  +{mod.price_delta.toLocaleString()}
                                </span>
                              )}
                            </button>
                          );
                        })}
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {/* Modifier Summary */}
              {Object.values(modifierGroupSelection).some(s => s.length > 0) && (
                <div className="text-[11px] leading-relaxed px-1" style={{ color: 'var(--brand-text-muted)' }}>
                  <span className="font-medium" style={{ color: 'var(--brand-text)' }}>Selected:</span>{' '}
                  {Object.entries(modifierGroupSelection).map(([gid, selectedIds]) => {
                    const group = (detailProduct.modifier_groups || []).find(g => g.id === gid);
                    if (!group || selectedIds.length === 0) return null;
                    return selectedIds.map(sid => group.modifiers.find(m => m.id === sid)?.name).filter(Boolean).join(', ');
                  }).filter(Boolean).join(' · ')}
                </div>
              )}

              {/* Quantity + Add to Cart */}
              <div className="flex items-center gap-3 pt-4 border-t" style={{ borderColor: 'var(--brand-border)' }}>
                <div className="flex items-center gap-2 rounded-xl p-1" style={{ background: 'var(--brand-surface)' }}>
                  <button 
                    onClick={() => setQuantity(q => Math.max(1, q - 1))}
                    className="min-w-[44px] min-h-[44px] rounded-lg flex items-center justify-center text-base font-medium transition-colors hover:opacity-80 active:scale-90"
                    style={{ color: 'var(--brand-text)' }}
                    aria-label="Decrease quantity"
                  >
                    <i className="ti ti-minus" />
                  </button>
                  <span className="text-base font-semibold w-8 text-center" style={{ color: 'var(--brand-text)' }}>{quantity}</span>
                  <button 
                    onClick={() => setQuantity(q => q + 1)}
                    className="min-w-[44px] min-h-[44px] rounded-lg flex items-center justify-center text-base font-medium transition-colors hover:opacity-80 active:scale-90"
                    style={{ color: 'var(--brand-text)' }}
                    aria-label="Increase quantity"
                  >
                    <i className="ti ti-plus" />
                  </button>
                </div>
                <button
                  onClick={handleAddDetail}
                  disabled={!canAdd()}
                  className="flex-1 h-[48px] rounded-xl text-white font-bold text-[15px] transition-all active:scale-[0.95] disabled:opacity-40 flex items-center justify-center gap-2"
                  style={{ background: detailProduct.available ? 'var(--brand-primary)' : 'var(--brand-text-muted)', borderRadius: 'var(--brand-radius-btn)' }}
                >
                  {detailProduct.available ? (
                    <>
                      <span>{t('client.add_to_cart', 'Add to Cart')}</span>
                      <span className="opacity-50 font-normal">·</span>
                      <span className="font-bold">{(detailProduct.price + calcModifierDelta()) * quantity} {getCurrency(menu)}</span>
                    </>
                  ) : (
                    t('client.unavailable', 'Unavailable')
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

    </div>
  );
}
