import React, { useEffect, useState, useRef, useCallback, useMemo, useLayoutEffect } from 'react';
import { useParams } from 'react-router-dom';
import { motion } from 'framer-motion';
import { ProductCard, useI18n, useToast, PriceDisplay, getAllergenStyle } from '@deliveryos/ui';
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
  imageUrl?: string | null;
  modifier_groups?: ModifierGroup[];
  attributes?: any;
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

  const CHEF_PICKS_ID = '__chefs_picks__';

  const MIN_SKELETON_DWELL = 300;

  const [menu, setMenu] = useState<MenuResponse | null>(null);
  const [data, setData] = useState<MenuCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [fetchError, setFetchError] = useState(false);
  const [retryCount, setRetryCount] = useState(0);
  const [activeTab, setActiveTab] = useState<string>('');
  const { addItem, bounceCart } = useSharedCart();
  const [detailProduct, setDetailProduct] = useState<Product | null>(null);
  const [selectedModifiers, setSelectedModifiers] = useState<Record<string, string[]>>({});
  const [modifierGroupSelection, setModifierGroupSelection] = useState<Record<string, string[]>>({});
  const [quantity, setQuantity] = useState(1);
  const [imageLoadError, setImageLoadError] = useState(false);
  const menuPrefsKey = `dos_menu_prefs_${slug}`;
  const [sortBy, setSortBy] = useState<'default' | 'price-asc' | 'price-desc' | 'name'>(() => {
    try {
      const s = localStorage.getItem(`dos_menu_prefs_${slug}`);
      const p = s ? JSON.parse(s) : null;
      return (['default', 'price-asc', 'price-desc', 'name'] as const).includes(p?.sortBy) ? p.sortBy : 'default';
    } catch { return 'default'; }
  });
  const [filterAllergen, setFilterAllergen] = useState<string | null>(() => {
    try {
      const s = localStorage.getItem(`dos_menu_prefs_${slug}`);
      const p = s ? JSON.parse(s) : null;
      return typeof p?.filterAllergen === 'string' ? p.filterAllergen : null;
    } catch { return null; }
  });
  const [searchQuery, setSearchQuery] = useState<string>(() => {
    try {
      const s = localStorage.getItem(`dos_menu_prefs_${slug}`);
      const p = s ? JSON.parse(s) : null;
      return typeof p?.searchQuery === 'string' ? p.searchQuery : '';
    } catch { return ''; }
  });

  useEffect(() => {
    try { localStorage.setItem(menuPrefsKey, JSON.stringify({ sortBy, filterAllergen, searchQuery })); } catch {}
  }, [menuPrefsKey, sortBy, filterAllergen, searchQuery]);

  const categories = data;

  const chefPicksCategory = useMemo((): MenuCategory | null => {
    const picks: Product[] = [];
    for (const cat of data) {
      for (const p of cat.products) {
        if (p.attributes?.chef_pick) picks.push(p);
      }
    }
    if (picks.length === 0) return null;
    return { id: CHEF_PICKS_ID, name: t('client.chefs_picks', "Chef's Picks"), sort_order: -1, products: picks };
  }, [data]);

  const displayCategories = useMemo(() => {
    const all: (Product & { _catId: string; _catName: string })[] = [];
    for (const cat of categories) {
      for (const p of cat.products) {
        all.push({ ...p, _catId: cat.id, _catName: cat.name });
      }
    }
    let result = all;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(p => p.name.toLowerCase().includes(q) || p.description?.toLowerCase().includes(q));
    }
    if (filterAllergen) {
      result = result.filter(p => bomToNutrition(p).allergens.includes(filterAllergen));
    }
    if (sortBy === 'default' && !searchQuery && !filterAllergen) return categories;
    const sorted = sortBy === 'default' ? result
      : sortBy === 'price-asc' ? [...result].sort((a, b) => a.price - b.price)
      : sortBy === 'price-desc' ? [...result].sort((a, b) => b.price - a.price)
      : [...result].sort((a, b) => a.name.localeCompare(b.name));
    const groups: MenuCategory[] = [];
    for (const p of sorted) {
      const g = groups.find(g => g.id === p._catId);
      if (g) g.products.push(p);
      else groups.push({ id: p._catId, name: p._catName, sort_order: 0, products: [p] });
    }
    return groups;
  }, [categories, sortBy, filterAllergen, searchQuery]);

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

  const loadMenu = useCallback(async () => {
    const start = Date.now();
    setLoading(true);
    setFetchError(false);
    try {
      const res = await fetch(`/public/locations/${slug}/menu`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const menuData: MenuResponse = await res.json();
      setMenu(menuData);
      const cats = menuData.categories || [];
      setData(cats);
      const hasChefPicks = cats.some(c => c.products.some(p => p.attributes?.chef_pick));
      if (hasChefPicks) setActiveTab(CHEF_PICKS_ID);
      else if (cats[0]) setActiveTab(cats[0].id);
    } catch (err) {
      console.error('[MenuPage] Failed to load menu:', err);
      setMenu(null);
      setData([]);
      setFetchError(true);
    } finally {
      const elapsed = Date.now() - start;
      if (elapsed < MIN_SKELETON_DWELL) {
        await new Promise(r => setTimeout(r, MIN_SKELETON_DWELL - elapsed));
      }
      setLoading(false);
    }
  }, [slug]);

  useEffect(() => {
    loadMenu();
  }, [loadMenu, retryCount]);

  const stickyRef = useRef<HTMLDivElement>(null);
  const [stickyHeight, setStickyHeight] = useState(44);

  useLayoutEffect(() => {
    const update = () => {
      if (stickyRef.current) setStickyHeight(stickyRef.current.offsetHeight);
    };
    update();
    const ro = new ResizeObserver(update);
    if (stickyRef.current) ro.observe(stickyRef.current);
    return () => ro.disconnect();
  }, [loading]);

  const HEADER_H = 56;
  const scrollOffset = HEADER_H + stickyHeight + 8;

  interface LocationInfo { lat: number; lng: number; googleRating?: number | null; googleReviewCount?: number | null; isOpen?: boolean; }
  const [locationInfo, setLocationInfo] = useState<LocationInfo | null>(null);
  const [deliveryETA, setDeliveryETA] = useState<number | null>(null);
  const [geoStatus, setGeoStatus] = useState<'unknown' | 'granted' | 'denied'>('unknown');

  useEffect(() => {
    if (!slug) return;
    fetch(`/public/locations/${slug}/info`)
      .then(r => r.ok ? r.json() : null)
      .then((d: any) => {
        if (d?.lat && d?.lng) setLocationInfo({ lat: d.lat, lng: d.lng, googleRating: d.googleRating, googleReviewCount: d.googleReviewCount, isOpen: d.isOpen });
      })
      .catch(() => {});
  }, [slug]);

  useEffect(() => {
    if (!locationInfo?.lat || !locationInfo?.lng) return;
    if (!('geolocation' in navigator)) { setGeoStatus('denied'); return; }
    navigator.geolocation.getCurrentPosition(
      (pos) => {
        setGeoStatus('granted');
        const { latitude: uLat, longitude: uLng } = pos.coords;
        const url = `https://router.project-osrm.org/route/v1/driving/${locationInfo.lng},${locationInfo.lat};${uLng},${uLat}?overview=false`;
        fetch(url).then(r => r.ok ? r.json() : null)
          .then((data: any) => {
            const secs = data?.routes?.[0]?.duration;
            if (typeof secs === 'number') setDeliveryETA(Math.ceil(secs / 60));
          }).catch(() => {});
      },
      () => setGeoStatus('denied'),
      { timeout: 8000, maximumAge: 300_000 }
    );
  }, [locationInfo]);

  const sectionRefs = useRef<Record<string, HTMLElement | null>>({});
  useEffect(() => {
    if (loading) return;
    const container = document.querySelector('.app-shell-main') as HTMLElement | null;
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          setActiveTab(entry.target.id);
        }
      }
    }, {
      root: container || null,
      rootMargin: `-${stickyHeight + 8}px 0px -60% 0px`,
    });

    Object.values(sectionRefs.current).forEach(el => {
      if (el) observer.observe(el);
    });
    return () => observer.disconnect();
  }, [loading, categories, chefPicksCategory, stickyHeight]);

  const handleScrollTo = (id: string) => {
    const el = document.getElementById(id);
    if (!el) return;
    const container = document.querySelector('.app-shell-main') as HTMLElement | null;
    if (container) {
      const top = container.scrollTop + el.getBoundingClientRect().top - container.getBoundingClientRect().top - stickyHeight - 8;
      container.scrollTo({ top, behavior: 'smooth' });
    } else {
      window.scrollTo({ top: el.getBoundingClientRect().top + window.scrollY - scrollOffset, behavior: 'smooth' });
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

  const { showToast } = useToast();

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
    showToast(t('cart.added_to_cart', 'Added to cart'), 'success');
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
    if (product.imageUrl) return product.imageUrl;
    if (!product.image_key) return null;
    if (product.image_key.startsWith('data:') || product.image_key.startsWith('http://') || product.image_key.startsWith('https://')) {
      return product.image_key;
    }
    const base = typeof window !== 'undefined'
      ? window.location.origin
      : (import.meta.env?.VITE_API_BASE_URL || '');
    const cleanKey = product.image_key.startsWith('/') ? product.image_key.slice(1) : product.image_key;
    return `${base}/images/${cleanKey}`;
  };
  const attrEntries = (p: Product): [string, any][] => {
    if (!p.attributes || typeof p.attributes !== 'object') return [];
    return Object.entries(p.attributes as Record<string, any>).filter(([k]) => !['kcal', 'protein', 'fat', 'carbs', 'allergens', 'tags', 'taste', 'bom', 'stock_count'].includes(k));
  };

  return (
    <div className="relative min-h-screen pb-28">

      {/* Hero Section */}
      <section className="relative w-full h-[160px] md:h-[200px] flex items-end overflow-hidden" style={{ background: 'linear-gradient(160deg, var(--brand-surface-raised) 0%, var(--brand-accent) 50%, var(--brand-primary) 100%)' }}>
        <div className="absolute inset-0" style={{ background: 'linear-gradient(to top, color-mix(in srgb, var(--brand-bg) 80%, transparent) 0%, color-mix(in srgb, var(--brand-bg) 40%, transparent) 50%, color-mix(in srgb, var(--brand-bg) 5%, transparent) 100%)' }} />
        <div className="absolute inset-0 opacity-[0.06]" style={{ background: 'radial-gradient(ellipse at 30% 50%, color-mix(in srgb, var(--brand-primary) 20%, transparent) 0%, transparent 60%)' }} />
        <div className="absolute inset-0 opacity-[0.03]" style={{ background: 'radial-gradient(ellipse at 70% 30%, color-mix(in srgb, var(--brand-text) 15%, transparent) 0%, transparent 50%)' }} />
        <motion.div
          className="relative z-10 w-full px-5 pb-5"
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.45, ease: [0.16, 1, 0.3, 1] }}
        >
          <motion.div
            className="flex items-center gap-1.5 text-[12px] font-medium mb-1.5"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.4, delay: 0.15 }}
            style={{ color: 'color-mix(in srgb, var(--color-on-primary) 70%, transparent)' }}
          >
            {locationInfo?.googleRating != null ? (
              <>
                <span className="inline-flex gap-0.5" style={{ color: 'var(--color-warning)' }}>
                  {[1,2,3,4,5].map(i => <i key={i} className={`ti ${i <= Math.round(locationInfo.googleRating!) ? 'ti-star-filled' : 'ti-star'}`} style={{ fontSize: '0.7rem' }} />)}
                </span>
                <span style={{ color: 'var(--color-on-primary)', fontWeight: 600 }}>{locationInfo.googleRating.toFixed(1)}</span>
                {locationInfo.googleReviewCount != null && <span className="opacity-70">({locationInfo.googleReviewCount})</span>}
              </>
            ) : null}
            {geoStatus !== 'denied' && deliveryETA != null && (
              <>
                {locationInfo?.googleRating != null && <span className="mx-1.5 opacity-40">·</span>}
                <i className="ti ti-clock" style={{ fontSize: '0.7rem' }} />
                <span>~{deliveryETA} min</span>
              </>
            )}
          </motion.div>
          <h1 className="text-[22px] md:text-[26px] font-bold leading-tight" style={{ color: 'var(--color-on-primary)', fontFamily: 'var(--brand-font-heading)', textShadow: '0 2px 16px color-mix(in srgb, var(--brand-bg) 40%, transparent)' }}>
            {menu?.location_name || t('client.menu', 'Menu')}
          </h1>
        </motion.div>
      </section>

      {/* Unified sticky: Category nav + Search/Sort/Filter — sits below the h-14 header which is outside this scroll container */}
      <div ref={stickyRef} className="sticky top-0 z-40" style={{ background: 'var(--brand-bg)' }}>
        {/* Category nav */}
        <div className="relative border-b" style={{ borderColor: 'var(--brand-border)' }}>
          <nav className="h-[44px] overflow-x-auto hide-scrollbar flex items-center gap-0.5 px-2" aria-label={t('client.categories', 'Categories')}>
            {loading ? (
              <div className="flex gap-4 px-2 h-full items-center">
                <div className="w-14 h-3.5 skeleton-block" />
                <div className="w-14 h-3.5 skeleton-block" />
                <div className="w-14 h-3.5 skeleton-block" />
              </div>
            ) : (
              [
                ...(chefPicksCategory ? [chefPicksCategory] : []),
                ...categories,
              ].map(cat => {
                const count = cat.products.filter(p => p.available).length;
                const isChefCat = cat.id === CHEF_PICKS_ID;
                return (
                  <motion.button
                    key={cat.id}
                    whileTap={{ scale: 0.97 }}
                    onClick={() => handleScrollTo(cat.id)}
                    role="tab"
                    aria-selected={activeTab === cat.id}
                    className="h-[44px] flex items-center gap-1 px-3 whitespace-nowrap text-[12px] font-medium transition-all border-b-2 shrink-0"
                    style={{
                      color: activeTab === cat.id ? (isChefCat ? '#f59e0b' : 'var(--brand-text)') : 'var(--brand-text-muted)',
                      borderColor: activeTab === cat.id ? (isChefCat ? '#f59e0b' : 'var(--brand-primary)') : 'transparent',
                    }}
                  >
                    {isChefCat && <span style={{ fontSize: '0.7rem' }}>✦</span>}
                    {cat.name}
                    <span className="text-[10px] opacity-40">({count})</span>
                  </motion.button>
                );
              })
            )}
          </nav>
          {/* Right fade hint for horizontal scroll */}
          <div className="absolute right-0 top-0 bottom-0 w-6 pointer-events-none" style={{ background: 'linear-gradient(to right, transparent, var(--brand-bg))' }} />
        </div>

        {/* Search + Sort + Allergen — single compact scrollable row */}
        {!loading && categories.length > 0 && (
          <div className="flex items-center gap-2 overflow-x-auto hide-scrollbar px-3 py-1.5 border-b" style={{ borderColor: 'var(--brand-border)' }}>
            {/* Compact search pill */}
            <div className="relative shrink-0" style={{ width: searchQuery ? 130 : 90, transition: 'width 0.2s', minWidth: 90 }}>
              <i className="ti ti-search absolute left-2 top-1/2 -translate-y-1/2 text-[10px]" style={{ color: 'var(--brand-text-muted)' }} />
              <input
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
                placeholder={t('common.search', 'Search')}
                className="w-full pl-6 pr-5 h-7 rounded-full text-[11px] outline-none"
                style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}
              />
              {searchQuery && (
                <button onClick={() => setSearchQuery('')} className="absolute right-1.5 top-1/2 -translate-y-1/2 w-4 h-4 flex items-center justify-center">
                  <i className="ti ti-x text-[9px]" style={{ color: 'var(--brand-text-muted)' }} />
                </button>
              )}
            </div>
            <div className="w-px h-4 shrink-0" style={{ background: 'var(--brand-border)' }} />
            {(['default', 'price-asc', 'price-desc', 'name'] as const).map(mode => (
              <motion.button key={mode} onClick={() => setSortBy(mode)} whileTap={{ scale: 0.95 }}
                className="px-2 h-7 rounded-full text-[10px] font-medium transition-all whitespace-nowrap shrink-0 flex items-center"
                style={{
                  background: sortBy === mode ? 'var(--brand-primary)' : 'var(--brand-surface-raised)',
                  color: sortBy === mode ? '#fff' : 'var(--brand-text-muted)',
                }}
              >
                {mode === 'default' ? <i className="ti ti-layout-list" style={{ fontSize: '0.65rem' }} /> : mode === 'price-asc' ? '↑ $' : mode === 'price-desc' ? '↓ $' : 'A–Z'}
              </motion.button>
            ))}
            {allAllergens.length > 0 && <div className="w-px h-4 shrink-0" style={{ background: 'var(--brand-border)' }} />}
            {allAllergens.map(a => {
              const s = getAllergenStyle(a);
              return (
                <motion.button key={a} onClick={() => setFilterAllergen(filterAllergen === a ? null : a)} whileTap={{ scale: 0.95 }}
                  className="px-2 h-7 rounded-full text-[9px] font-semibold uppercase whitespace-nowrap shrink-0 transition-all"
                  style={{
                    background: filterAllergen === a ? s.text : s.bg,
                    color: filterAllergen === a ? '#fff' : s.text,
                    opacity: filterAllergen && filterAllergen !== a ? 0.35 : 1,
                  }}
                >
                  {t(`allergen.${a.toLowerCase()}`, a)}
                </motion.button>
              );
            })}
          </div>
        )}
      </div>

      {/* Delivery closed banner */}
      {locationInfo?.isOpen === false && (
        <motion.div
          initial={{ opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          className="mx-4 my-3 px-4 py-3 rounded-xl border flex items-center gap-3 text-sm font-medium"
          style={{ background: 'var(--color-warning-light, rgba(217,119,6,0.08))', borderColor: 'var(--color-warning, #D97706)', color: 'var(--color-warning, #D97706)' }}
        >
          <i className="ti ti-clock-off text-lg shrink-0" />
          <span>{t('client.delivery_closed', 'We are currently closed. Check back during opening hours.')}</span>
        </motion.div>
      )}

      {/* Menu Content — min-h prevents layout shifts when filtering/sorting changes product count */}
      <main className="max-w-5xl mx-auto pt-4 min-h-screen">
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
            <p className="text-sm font-medium" style={{ color: 'var(--brand-text-muted)' }}>
              {t('client.empty_menu', fetchError ? 'Failed to load menu' : 'Menu unavailable')}
            </p>
            {fetchError && (
              <motion.button onClick={() => { setRetryCount(c => c + 1); }} whileTap={{ scale: 0.97 }} className="mt-4 px-5 py-2 rounded-xl text-sm font-semibold text-white transition-all active:scale-95 min-h-11" style={{ background: 'var(--brand-primary)' }}>
                <i className="ti ti-refresh mr-1.5" />{t('client.retry', 'Retry')}
              </motion.button>
            )}
          </div>
) : displayCategories.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 px-4 text-center">
            <i className="ti ti-search-off text-5xl opacity-20 mb-3" style={{ color: 'var(--brand-text-muted)' }} />
            <p className="text-sm font-medium mb-4" style={{ color: 'var(--brand-text-muted)' }}>
              {t('client.no_results', 'No products match your filters')}
            </p>
            <motion.button
              onClick={() => { setSortBy('default'); setFilterAllergen(null); setSearchQuery(''); }}
              whileTap={{ scale: 0.97 }}
              className="px-5 py-2 rounded-xl text-sm font-semibold text-white transition-all active:scale-95 min-h-11"
              style={{ background: 'var(--brand-primary)' }}
            >
              {t('client.browse_menu', 'Browse full menu')}
            </motion.button>
          </div>
        ) : (
          [
            ...(chefPicksCategory ? [chefPicksCategory] : []),
            ...displayCategories,
          ].map(category => {
            const isChefCat = category.id === CHEF_PICKS_ID;
            return (
            <motion.section
              key={category.id}
              id={category.id}
              ref={el => { sectionRefs.current[category.id] = el }}
              className="mb-7"
              style={{ scrollMarginTop: scrollOffset + 'px' }}
              initial={{ opacity: 0, y: 16 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: '-60px' }}
              transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
            >
              <h2 className="text-lg font-bold px-4 mb-3 flex items-center gap-2" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' }}>
                {isChefCat && <span style={{ color: '#f59e0b', fontSize: '1rem' }}>✦</span>}
                {category.name}
              </h2>
              <motion.div
                className="grid grid-cols-2 md:grid-cols-3 gap-3 px-4"
                variants={{ visible: { transition: { staggerChildren: 0.03 } } }}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true, margin: '-40px' }}
              >
                {category.products.map(product => {
                  const nutrition = bomToNutrition(product);
                  return (
                  <motion.div
                    key={product.id}
                    variants={{ hidden: { opacity: 0, y: 12 }, visible: { opacity: 1, y: 0, transition: { duration: 0.2, ease: [0.16, 1, 0.3, 1] } } }}
                  >
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
                      chefPick: !!product.attributes?.chef_pick,
                    }}
                    onClick={() => handleProductClick(product)}
                    onAdd={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      if (!product.available) return;
                      if (!product.modifier_groups?.length) {
                        addItem({ id: `cart_${product.id}`, productId: product.id, name: product.name, quantity: 1, price: product.price, options: {} });
                        bounceCart();
                        showToast(t('cart.added_to_cart', 'Added to cart'), 'success');
                      } else {
                        handleProductClick(product);
                      }
                    }} />
                  </motion.div>
                  );
                })}
              </motion.div>
            </motion.section>
            );
          })
        )}
       </main>

      {/* Product Detail Modal */}
      {detailProduct && (
        <div className="fixed inset-0 z-50 flex items-end md:items-center justify-center transition-opacity duration-300" style={{ background: 'color-mix(in srgb, var(--brand-bg) 60%, transparent)', backdropFilter: 'blur(4px)' }} role="dialog" aria-modal="true">
          <button type="button" className="absolute inset-0 cursor-default" aria-label={t('common.close', 'Close')} onClick={closeDetail} />
          <div
            className="relative w-full md:max-w-lg max-h-[85vh] overflow-auto rounded-t-2xl md:rounded-2xl shadow-2xl animate-slide-up"
            style={{ background: 'var(--brand-bg)' }}
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
              <motion.button 
                whileTap={{ scale: 0.95 }}
                className="absolute top-4 right-4 min-w-[44px] min-h-[44px] rounded-full flex items-center justify-center backdrop-blur-md active:scale-[0.95] transition-transform"
                style={{ background: 'color-mix(in srgb, var(--brand-bg) 50%, transparent)', color: 'var(--color-on-primary)' }}
                onClick={closeDetail}
                aria-label={t('common.close', 'Close')}
              >
                <i className="ti ti-x text-xl" />
              </motion.button>
              {detailProduct.available && bomToNutrition(detailProduct).kcal > 0 && (
                <div className="absolute bottom-3 left-3 z-10">
                  <span className="text-[10px] font-medium px-2 py-1 rounded-md flex items-center gap-1.5" style={{ background: 'color-mix(in srgb, var(--brand-bg) 60%, transparent)', color: 'var(--color-on-primary)' }}>
                    <i className="ti ti-flame" style={{ fontSize: '0.7rem' }} />
                    {bomToNutrition(detailProduct).kcal} {t('nutrition.calories', 'kcal')}
                    {bomToNutrition(detailProduct).protein > 0 && <span className="opacity-70">· {t('nutrition.protein', 'P')}{bomToNutrition(detailProduct).protein}g</span>}
                    {bomToNutrition(detailProduct).fat > 0 && <span className="opacity-70">· {t('nutrition.fat', 'F')}{bomToNutrition(detailProduct).fat}g</span>}
                    {bomToNutrition(detailProduct).carbs > 0 && <span className="opacity-70">· {t('nutrition.carbs', 'C')}{bomToNutrition(detailProduct).carbs}g</span>}
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
                        <span className="text-[10px] font-semibold px-2 py-0.5 rounded-full inline-flex items-center gap-1" style={{ background: 'color-mix(in srgb, var(--brand-primary) 10%, transparent)', color: 'var(--brand-primary)' }}>
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
                    <PriceDisplay amount={detailProduct.price + calcModifierDelta()} />
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
                        { key: 'nutrition.calories', value: bomToNutrition(detailProduct).kcal, icon: 'ti ti-flame' },
                        { key: 'nutrition.protein', value: bomToNutrition(detailProduct).protein, icon: 'ti ti-droplet' },
                        { key: 'nutrition.fat', value: bomToNutrition(detailProduct).fat, icon: 'ti ti-droplet-half' },
                        { key: 'nutrition.carbs', value: bomToNutrition(detailProduct).carbs, icon: 'ti ti-droplet-filled' },
                      ].map(n => n.value > 0 && (
                        <div key={n.key} className="flex flex-col items-center gap-1">
                          <i className={n.icon} style={{ fontSize: '1rem', color: 'var(--brand-text-muted)' }} />
                          <span className="text-sm font-bold" style={{ color: 'var(--brand-text)' }}>{n.value}</span>
                          <span className="text-[9px] font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t(n.key)}</span>
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
                          {t(`allergen.${a.toLowerCase()}`, a)}
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
                            <motion.button
                              key={mod.id}
                              data-testid="modifier-option"
                              onClick={() => toggleModifier(group.id, mod.id, group)}
                              whileTap={{ scale: 0.97 }}
                              className={`px-3.5 py-2 rounded-[10px] text-[13px] font-medium transition-all active:scale-[0.97] border min-h-11 ${
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
                                  +<PriceDisplay amount={mod.price_delta} currency="ALL" />
                                </span>
                              )}
                            </motion.button>
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
                  <motion.button 
                    onClick={() => setQuantity(q => Math.max(1, q - 1))}
                    whileTap={{ scale: 0.9 }}
                    className="min-w-[44px] min-h-[44px] rounded-lg flex items-center justify-center text-base font-medium transition-colors hover:opacity-80 active:scale-90"
                    style={{ color: 'var(--brand-text)' }}
                    aria-label={t('common.decrease_quantity', 'Decrease quantity')}
                  >
                    <i className="ti ti-minus" />
                  </motion.button>
                  <span className="text-base font-semibold w-8 text-center" style={{ color: 'var(--brand-text)' }}>{quantity}</span>
                  <motion.button 
                    onClick={() => setQuantity(q => q + 1)}
                    whileTap={{ scale: 0.9 }}
                    className="min-w-[44px] min-h-[44px] rounded-lg flex items-center justify-center text-base font-medium transition-colors hover:opacity-80 active:scale-90"
                    style={{ color: 'var(--brand-text)' }}
                    aria-label={t('common.increase_quantity', 'Increase quantity')}
                  >
                    <i className="ti ti-plus" />
                  </motion.button>
                </div>
                <motion.button
                  data-testid="product-detail-confirm"
                  onClick={handleAddDetail}
                  disabled={!canAdd()}
                  whileTap={{ scale: 0.95 }}
                  className="flex-1 h-[48px] rounded-xl text-white font-bold text-[15px] transition-all active:scale-[0.95] disabled:opacity-40 flex items-center justify-center gap-2"
                  style={{ background: detailProduct.available ? 'var(--brand-primary)' : 'var(--brand-text-muted)', borderRadius: 'var(--brand-radius-btn)' }}
                >
                  {detailProduct.available ? (
                    <>
                      <span>{t('client.add_to_cart', 'Add to Cart')}</span>
                      <span className="opacity-50 font-normal">·</span>
                      <PriceDisplay amount={(detailProduct.price + calcModifierDelta()) * quantity} />
                    </>
                  ) : (
                    t('client.unavailable', 'Unavailable')
                  )}
                </motion.button>
              </div>
            </div>
          </div>
        </div>
      )}

    </div>
  );
}
