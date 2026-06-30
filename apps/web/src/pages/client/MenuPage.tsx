import { safeStorage } from '../../lib/safeStorage.js';
import React, { useEffect, useState, useRef, useCallback, useMemo, useLayoutEffect, lazy, Suspense } from 'react';
import type { ProductMedia } from '../../components/media/types';

// Rich product media (ADR-0002) — code-split lazy chunks. They load ONLY when the lazy media
// endpoint returns a non-empty set for the open product (server-gated on MEDIA_RICH_ENABLED +
// business tier), so a storefront with no rich media downloads ~0 KB of these.
const MediaGallery = lazy(() => import('../../components/media/MediaGallery').then(m => ({ default: m.MediaGallery })));
const MediaRenderer = lazy(() => import('../../components/media').then(m => ({ default: m.MediaRenderer })));
import { useParams } from 'react-router-dom';
import { motion, AnimatePresence, useReducedMotion } from 'framer-motion';
import { ProductCard, StateChip, useI18n, useToast, PriceDisplay, getAllergenStyle, ease, SearchInput, computeAllergenSurface, partitionByMacroLens } from '@deliveryos/ui';
import { useSharedCart } from '../../lib/CartProvider.js';
import { MenuComparePanel } from './MenuComparePanel.js';
import type { CompareDish } from './MenuComparePanel.js';
import { DishStats } from '../../components/client/DishStats.js';
import type { DishIngredient } from '../../components/client/DishStats.js';
import { StylizedMap } from '../../components/client/StylizedMap.js';
import { SatelliteMap } from '../../components/client/SatelliteMap.js';
import type { MacroLens } from '@deliveryos/ui';

// Allergen FILTER — gated OFF by default (council menu-characteristics-model FB-C1, recorded human
// decision). The filter PREDICATE is converged onto computeAllergenSurface (declared∪recipe) regardless,
// so a declared-only allergen dish can never be dropped from a "contains X" view; but over near-empty
// coverage a visible "contains X" filter risks a "dishes not shown ⇒ safe" false-read, so the CHIPS stay
// dark until a positive-only human sign-off. Re-enable = flip the flag (predicate is already correct).
const ALLERGEN_FILTER_ENABLED = (import.meta as any).env?.VITE_MENU_ALLERGEN_FILTER === 'true';

// Menu Characteristics layer sub-flags (council menu-characteristics-model, ADR-0014) — all default OFF
// (dark on prod). Each ships behind its own flag, gated by its red→green guardrails. STEP-0 (the allergen
// single-source safety fix) is UNCONDITIONAL and independent of these.
//  - CHARACTERISTICS_ENABLED: L1 taste (already live) + L2 descriptive band (allowlist EMPTY → dormant).
//  - COMPARISON_ENABLED: affordance-only 2-dish compare (no long-press, FB-M4); arrows only price/prep (#11).
//  - FILTER_LENSES_ENABLED: non-allergen sort/filter lenses; no-bom dishes in an explicit "no data" bucket (#15).
const CHARACTERISTICS_ENABLED = (import.meta as any).env?.VITE_MENU_CHARACTERISTICS_ENABLED === 'true';
const COMPARISON_ENABLED = (import.meta as any).env?.VITE_MENU_CHARACTERISTICS_COMPARISON === 'true';
const FILTER_LENSES_ENABLED = (import.meta as any).env?.VITE_MENU_CHARACTERISTICS_FILTER === 'true';

// Allergen DISPLAY freeze (operator directive 2026-06-30): the whole allergen surface is hidden from the
// storefront for now. The computeAllergenSurface single-source library + STEP-0 contract + guardrails #12
// stay intact underneath (dormant) — re-enabling is this one flag, not a rebuild. Default OFF = frozen.
const ALLERGENS_ENABLED = false;

interface ProductModifier {
  id: string;
  name: string;
  price_delta: number;
  available: boolean;
  sort_order: number;
}

type ModifierDisplayType = 'radio' | 'checkbox' | 'select' | 'quantity';

interface ModifierGroup {
  id: string;
  name: string;
  min_select: number;
  max_select: number;
  required: boolean;
  sort_order: number;
  // MENU-AVAILABILITY (additive) · explicit render control. Absent => infer from max_select.
  display_type?: ModifierDisplayType | null;
  modifiers: ProductModifier[];
}

// MENU-AVAILABILITY · resolve the effective control explicitly. Prefer the owner-set
// display_type; fall back to the legacy max_select inference (radio when single-select,
// checkbox otherwise) so unscheduled/un-typed groups render exactly as before.
function resolveDisplayType(g: ModifierGroup): ModifierDisplayType {
  if (g.display_type) return g.display_type;
  return g.max_select === 1 ? 'radio' : 'checkbox';
}

interface Product {
  id: string;
  name: string;
  description?: string;
  price: number;
  prep_time_minutes?: number;
  available: boolean;
  image_key?: string | null;
  imageUrl?: string | null;
  primary_media_id?: string | null;
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
  const { t, locale } = useI18n();
  // Honour the OS reduced-motion preference: framer's spring/stagger entrances become
  // instant crossfades, matching the @media (prefers-reduced-motion) CSS rails.
  const prefersReduced = useReducedMotion();
  const getAttr = (p: Product, key: string): any => {
    if (!p.attributes || typeof p.attributes !== 'object') return undefined;
    return (p.attributes as Record<string, any>)[key];
  };

  const extractLineAllergens = (line: any): string[] => {
    if (Array.isArray(line.allergens)) return line.allergens;
    if (typeof line.allergens === 'string' && line.allergens) return line.allergens.split(',').map((s: string) => s.trim()).filter(Boolean);
    return [];
  };

  // Macro nutrition + ingredients ONLY. Allergens are NOT returned here (STEP-0 single-source contract,
  // council #12): a recipe-only allergen array must never be the basis of a safety read. Recipe-derived
  // allergens are extracted by recipeAllergens() solely to be UNIONED with the owner declaration inside
  // computeAllergenSurface (allergenSurfaceOf), the one allergen source on every storefront surface.
  const bomToNutrition = (p: Product) => {
    const bom = getAttr(p, 'bom');
    if (!Array.isArray(bom) || bom.length === 0) return { kcal: 0, protein: 0, fat: 0, carbs: 0, ingredients: [] };
    let kcal = 0, protein = 0, fat = 0, carbs = 0;
    const ingredients: string[] = [];
    for (const line of bom) {
      if (typeof line.kcal === 'number') kcal += line.kcal;
      if (typeof line.proteinG === 'number') protein += line.proteinG;
      if (typeof line.fatG === 'number') fat += line.fatG;
      if (typeof line.carbsG === 'number') carbs += line.carbsG;
      if (line.supplyName && line.kind !== 'packaging' && line.kind !== 'utensil') ingredients.push(line.supplyName);
    }
    return { kcal: Math.round(kcal), protein: Math.round(protein), fat: Math.round(fat), carbs: Math.round(carbs), ingredients };
  };

  // Recipe-derived allergens — the UNION INPUT only, never a safety output on its own. Its single consumer
  // is allergenSurfaceOf (computeAllergenSurface), which conservatively unions it with the owner's L3
  // declaration so a base-dish allergen can never be dropped by attestation status (council #4-positive).
  const recipeAllergens = (p: Product): string[] => {
    const bom = getAttr(p, 'bom');
    if (!Array.isArray(bom)) return [];
    const set = new Set<string>();
    for (const line of bom) extractLineAllergens(line).forEach((a: string) => set.add(a));
    return Array.from(set);
  };

  // The ONE allergen source on every storefront surface (filter predicate, detail modal, and — when shipped
  // — the card unit + comparison). PRESENCE-only, conservative declared∪recipe union (council STEP-0 / #12).
  const allergenSurfaceOf = (p: Product) => computeAllergenSurface((p as any).attributes, recipeAllergens(p));

  // Per-ingredient breakdown from the BOM (food lines only) for the DishStats viz: name + declared
  // amount (qty/unit, e.g. "Tuna 100 g") + its kcal contribution. Packaging/utensils excluded.
  const dishIngredients = (p: Product): DishIngredient[] => {
    const bom = getAttr(p, 'bom');
    if (!Array.isArray(bom)) return [];
    return bom
      .filter((l: any) => l && l.supplyName && l.kind !== 'packaging' && l.kind !== 'utensil')
      .map((l: any) => ({ name: String(l.supplyName), qty: Number(l.qty) || 0, unit: String(l.unit || 'g'), kcal: Number(l.kcal) || 0 }));
  };

  // Compare input — macros + per-ingredient breakdown for the DishStats viz. Allergens are frozen, so the
  // compare panel carries no allergen data.
  const toCompareInput = (p: Product): CompareDish => {
    const n = bomToNutrition(p);
    return {
      id: p.id,
      name: p.name,
      price: p.price,
      prepTimeMinutes: p.prep_time_minutes ?? null,
      taste: (getAttr(p, 'taste') as Record<string, number>) || null,
      macros: { kcal: n.kcal, protein: n.protein, fat: n.fat, carbs: n.carbs },
      ingredients: dishIngredients(p),
    };
  };
  const toggleCompare = (id: string) => setCompareIds(prev =>
    prev.includes(id) ? prev.filter(x => x !== id) : prev.length >= 2 ? prev : [...prev, id]);

  const CHEF_PICKS_ID = '__chefs_picks__';
  const SORTED_FLAT_ID = '__sorted_flat__';
  const NO_MACRO_DATA_ID = '__no_macro_data__';

  const MIN_SKELETON_DWELL = 300;

  const [menu, setMenu] = useState<MenuResponse | null>(null);
  const [data, setData] = useState<MenuCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [fetchError, setFetchError] = useState(false);
  // 404 (unknown venue slug) is distinct from a transient load failure: retrying a bad slug is
  // futile, so we show a "venue not found" state with an escape, not a retry button.
  const [notFound, setNotFound] = useState(false);
  const [retryCount, setRetryCount] = useState(0);
  const { addItem, bounceCart, reconcileToMenu } = useSharedCart();
  const [detailProduct, setDetailProduct] = useState<Product | null>(null);
  const [selectedModifiers, setSelectedModifiers] = useState<Record<string, string[]>>({});
  const [modifierGroupSelection, setModifierGroupSelection] = useState<Record<string, string[]>>({});
  const [quantity, setQuantity] = useState(1);
  const [imageLoadError, setImageLoadError] = useState(false);
  // Rich media for the open product, lazily fetched on modal open. Empty = fall back to the
  // single image / gradient (today's behaviour). Server returns [] when the feature is gated off.
  const [detailMedia, setDetailMedia] = useState<ProductMedia[]>([]);
  // Compare (council §8.2) — up to TWO dish ids; PERSISTED so a reload keeps the selection.
  const [compareIds, setCompareIds] = useState<string[]>(() => {
    try { const p = JSON.parse(safeStorage.get(`dos_menu_prefs_${slug}`) || 'null'); return Array.isArray(p?.compareIds) ? p.compareIds.filter((x: any) => typeof x === 'string').slice(0, 2) : []; } catch { return []; }
  });
  const [compareOpen, setCompareOpen] = useState(false);
  // Macro filter/sort lens (council §8.3) — PERSISTED. 'none' = off.
  const [macroLens, setMacroLens] = useState<'none' | MacroLens>(() => {
    try { const p = JSON.parse(safeStorage.get(`dos_menu_prefs_${slug}`) || 'null'); return (['none', 'kcal-asc', 'kcal-desc', 'protein-asc', 'protein-desc'] as const).includes(p?.macroLens) ? p.macroLens : 'none'; } catch { return 'none'; }
  });
  const menuPrefsKey = `dos_menu_prefs_${slug}`;
  const [sortBy, setSortBy] = useState<'default' | 'price-asc' | 'price-desc' | 'name'>(() => {
    try {
      const s = safeStorage.get(`dos_menu_prefs_${slug}`);
      const p = s ? JSON.parse(s) : null;
      return (['default', 'price-asc', 'price-desc', 'name'] as const).includes(p?.sortBy) ? p.sortBy : 'default';
    } catch { return 'default'; }
  });
  const [filterAllergen, setFilterAllergen] = useState<string | null>(() => {
    try {
      const s = safeStorage.get(`dos_menu_prefs_${slug}`);
      const p = s ? JSON.parse(s) : null;
      return typeof p?.filterAllergen === 'string' ? p.filterAllergen : null;
    } catch { return null; }
  });
  const [searchQuery, setSearchQuery] = useState<string>(() => {
    try {
      const s = safeStorage.get(`dos_menu_prefs_${slug}`);
      const p = s ? JSON.parse(s) : null;
      return typeof p?.searchQuery === 'string' ? p.searchQuery : '';
    } catch { return ''; }
  });
  // Categories act as a single-select FILTER: tapping a category narrows the menu to it; tapping "All"
  // (or the same category again) clears it. PERSISTED so a reload keeps the chosen category.
  const [selectedCategory, setSelectedCategory] = useState<string | null>(() => {
    try { const p = JSON.parse(safeStorage.get(`dos_menu_prefs_${slug}`) || 'null'); return typeof p?.selectedCategory === 'string' ? p.selectedCategory : null; } catch { return null; }
  });

  // Persist all storefront UI state so a reload restores it (category, sort, filter, search, lens, compare).
  useEffect(() => {
    try { safeStorage.set(menuPrefsKey, JSON.stringify({ sortBy, filterAllergen, searchQuery, selectedCategory, macroLens, compareIds })); } catch {}
  }, [menuPrefsKey, sortBy, filterAllergen, searchQuery, selectedCategory, macroLens, compareIds]);

  // Drop a persisted category that no longer exists once the menu loads (stale pref → show All, not blank).
  useEffect(() => {
    if (selectedCategory && selectedCategory !== CHEF_PICKS_ID && data.length > 0 && !data.some(c => c.id === selectedCategory)) {
      setSelectedCategory(null);
    }
  }, [data, selectedCategory, CHEF_PICKS_ID]);

  const categories = data;

  // Resolve the selected compare ids → products (order preserved). Drops ids that vanished from the menu.
  const compareProducts = useMemo(() => {
    const byId = new Map<string, Product>();
    for (const cat of data) for (const p of cat.products) byId.set(p.id, p);
    return compareIds.map(id => byId.get(id)).filter(Boolean) as Product[];
  }, [data, compareIds]);

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
    // Predicate converged onto the single allergen source (declared∪recipe) so a declared-only "contains X"
    // dish is never dropped (council #12). Only applied when the filter is enabled (flag default off).
    if (ALLERGENS_ENABLED && ALLERGEN_FILTER_ENABLED && filterAllergen) {
      result = result.filter(p => allergenSurfaceOf(p).known.includes(filterAllergen));
    }
    // Category filter (single-select). Chef's Picks is a cross-category overlay, so it
    // filters by the chef_pick attribute rather than a category id.
    if (selectedCategory === CHEF_PICKS_ID) {
      result = result.filter(p => (p as any).attributes?.chef_pick);
    } else if (selectedCategory) {
      result = result.filter(p => p._catId === selectedCategory);
    }
    const macroLensActive = FILTER_LENSES_ENABLED && macroLens !== 'none';
    if (sortBy === 'default' && !searchQuery && !filterAllergen && !selectedCategory && !macroLensActive) return categories;

    // An active search/sort/filter that matches nothing must yield ZERO sections — never a
    // bare "All items" heading over blank space. Returning [] here lets the single empty-state
    // branch below fire for every path (the sorted-flat branch would otherwise emit one empty
    // category, hiding the empty-state).
    if (result.length === 0) return [];

    // Macro lens (council §8.3 / #15): a GLOBAL numeric order over a raw macro, with no-bom dishes pulled
    // into an EXPLICIT "no data" group (never ranked as 0). Takes precedence over the price/name sort.
    if (macroLensActive) {
      const items = result.map(p => {
        const n = bomToNutrition(p);
        const bom = getAttr(p, 'bom');
        return { p, hasData: Array.isArray(bom) && bom.length > 0, kcal: n.kcal, protein: n.protein };
      });
      const { ranked, noData } = partitionByMacroLens(items, macroLens as MacroLens);
      const sections: MenuCategory[] = [{ id: SORTED_FLAT_ID, name: t('client.all_items', 'All items'), sort_order: 0, products: ranked.map(i => i.p) }];
      if (noData.length) sections.push({ id: NO_MACRO_DATA_ID, name: t('filter.no_nutrition_data', 'Nutrition not provided'), sort_order: 1, products: noData.map(i => i.p) });
      return sections;
    }

    // A non-default sort is a GLOBAL order ("cheapest first" etc.). Re-bucketing the
    // sorted list back into categories breaks monotonicity (each category restarts the
    // ordering), so when a sort is active we render ONE flat ungrouped section. Category
    // grouping (and the nav tabs) only make sense for the 'default' order.
    if (sortBy !== 'default') {
      const sorted = sortBy === 'price-asc' ? [...result].sort((a, b) => a.price - b.price)
        : sortBy === 'price-desc' ? [...result].sort((a, b) => b.price - a.price)
        : [...result].sort((a, b) => a.name.localeCompare(b.name));
      return [{ id: SORTED_FLAT_ID, name: t('client.all_items', 'All items'), sort_order: 0, products: sorted }];
    }

    // Chef's Picks filter → one titled section (its products span real categories).
    if (selectedCategory === CHEF_PICKS_ID) {
      return [{ id: CHEF_PICKS_ID, name: t('client.chefs_picks', "Chef's Picks"), sort_order: 0, products: result }];
    }

    // sortBy === 'default' but a search/allergen/category filter is active → keep category grouping.
    const groups: MenuCategory[] = [];
    for (const p of result) {
      const g = groups.find(g => g.id === p._catId);
      if (g) g.products.push(p);
      else groups.push({ id: p._catId, name: p._catName, sort_order: 0, products: [p] });
    }
    return groups;
  }, [categories, sortBy, filterAllergen, searchQuery, selectedCategory, macroLens, t]);

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
    setNotFound(false);
    try {
      const res = await fetch(`/public/locations/${slug}/menu?locale=${encodeURIComponent(locale)}`);
      if (res.status === 404) { setMenu(null); setData([]); setNotFound(true); return; }
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const menuData: MenuResponse = await res.json();
      setMenu(menuData);
      const cats = menuData.categories || [];
      setData(cats);
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
  }, [slug, locale]);

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

  interface LocationInfo { id?: string; lat: number; lng: number; googleRating?: number | null; googleReviewCount?: number | null; isOpen?: boolean; status?: 'open' | 'closed' | 'busy'; closesAt?: string | null; }
  // MENU-AVAILABILITY · venue state (open|closed|busy) decoupled from lat/lng so it
  // surfaces even when geo is absent. `busy` is a distinct eater-facing state.
  const [venueStatus, setVenueStatus] = useState<'open' | 'closed' | 'busy' | null>(null);
  const [locationInfo, setLocationInfo] = useState<LocationInfo | null>(null);
  // Hero background video (self-hosted in R2, served by /media). Per-location convention key; if the
  // object 404s (no video for this venue) the <video> onError flips this off → the stylized map shows.
  const [heroVideoOk, setHeroVideoOk] = useState(true);
  // UX-1 storefront footer links — decoupled from geo so they show even without lat/lng.
  const [storeLinks, setStoreLinks] = useState<{ mapsUrl?: string | null; instagram?: string | null; facebook?: string | null; phone?: string | null }>({});
  const [storeAddress, setStoreAddress] = useState<string | null>(null);
  // Hide the footer in embed/activation-preview contexts (target=_blank is unreliable in iframes).
  const isEmbed = typeof window !== 'undefined' && (new URLSearchParams(window.location.search).get('embed') === 'true' || new URLSearchParams(window.location.search).get('activation') === '1');
  const [deliveryETA, setDeliveryETA] = useState<number | null>(null);
  const [geoStatus, setGeoStatus] = useState<'unknown' | 'granted' | 'denied'>('unknown');
  // When the venue is closed the storefront stays fully browsable, but ordering is
  // blocked: every add-to-cart path checks this flag and the CTA reflects it.
  const isClosed = venueStatus === 'closed';

  useEffect(() => {
    if (!slug) return;
    fetch(`/public/locations/${slug}/info`)
      .then(r => r.ok ? r.json() : null)
      .then((d: any) => {
        if (!d) return;
        if (d.lat && d.lng) setLocationInfo({ id: d.id, lat: d.lat, lng: d.lng, googleRating: d.googleRating, googleReviewCount: d.googleReviewCount, isOpen: d.isOpen, status: d.status, closesAt: d.closesAt ?? null });
        // Derive venue state from the contract status; fall back to the legacy isOpen
        // boolean for older payloads (busy only ever comes from the new `status` field).
        setVenueStatus(d.status ?? (d.isOpen === false ? 'closed' : 'open'));
        setStoreLinks({ mapsUrl: d.googleMapsUrl ?? null, instagram: d.socialInstagram ?? null, facebook: d.socialFacebook ?? null, phone: d.phone ?? null });
        setStoreAddress(d.address ?? null);
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

  // Category tap = single-select filter. Toggling the active one (or "All" = null)
  // clears it. Scroll the menu container back to the top so the narrowed list is
  // visible from its start (avoids landing mid-page after the content shrinks).
  const pickCategory = (id: string | null) => {
    setSelectedCategory(prev => (prev === id ? null : id));
    const container = document.querySelector('.app-shell-main') as HTMLElement | null;
    if (container) container.scrollTo({ top: 0, behavior: prefersReduced ? 'auto' : 'smooth' });
    else window.scrollTo({ top: 0, behavior: prefersReduced ? 'auto' : 'smooth' });
  };

  const handleProductClick = (product: Product) => {
    // Activation tool: when embedded with ?activation=1, tapping an item edits it in
    // the parent tool instead of opening the order detail. Gated by the param + being
    // inside an iframe → zero effect for real customers.
    if (typeof window !== 'undefined' && window.parent !== window &&
        new URLSearchParams(window.location.search).get('activation') === '1') {
      window.parent.postMessage(
        { type: 'dos_activation_edit_product', product: { id: product.id, name: product.name, price: product.price } },
        '*',
      );
      return;
    }
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

  // Lazy-fetch the rich media set when a product modal opens. Gated server-side (returns []
  // when MEDIA_RICH_ENABLED is off or the location isn't business tier) and skipped entirely
  // unless the product carries a primary_media_id — so the dark/default path makes no request
  // and the storefront is byte-identical to today.
  useEffect(() => {
    setDetailMedia([]);
    const pid = detailProduct?.id;
    if (!pid || !detailProduct?.primary_media_id || !slug) return;
    let cancelled = false;
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), 4000); // never block the modal on media
    fetch(`/public/locations/${slug}/products/${pid}/media`, { signal: ctrl.signal })
      .then(r => (r.ok ? r.json() : { media: [] }))
      .then(d => { if (!cancelled) setDetailMedia(Array.isArray(d?.media) ? d.media : []); })
      .catch(() => { /* media is best-effort; fall back to the primary image */ })
      .finally(() => clearTimeout(timer));
    return () => { cancelled = true; ctrl.abort(); clearTimeout(timer); };
  }, [detailProduct?.id, detailProduct?.primary_media_id, slug]);

  // Lock background scroll while the product modal is open. The real scroll
  // container is .app-shell-main (the SPA shell), not <body>, so lock both —
  // otherwise the page behind the bottom-sheet keeps scrolling on touch.
  useEffect(() => {
    if (!detailProduct) return;
    const main = document.querySelector('.app-shell-main') as HTMLElement | null;
    const prevMain = main?.style.overflow ?? '';
    const prevBody = document.body.style.overflow;
    if (main) main.style.overflow = 'hidden';
    document.body.style.overflow = 'hidden';
    return () => {
      if (main) main.style.overflow = prevMain;
      document.body.style.overflow = prevBody;
    };
  }, [detailProduct]);

  // Close the product modal on Escape — the X / grabber / backdrop close it, but a
  // role="dialog" aria-modal sheet should also dismiss on Escape (keyboard a11y).
  useEffect(() => {
    if (!detailProduct) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') closeDetail(); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [detailProduct]);

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

  // F9: reconcile a persisted cart to the freshly-loaded menu. A cart can outlive a
  // price/availability change (localStorage survives across sessions); without this the
  // customer only discovers the drift as a server hard-block after filling out checkout.
  // Re-pricing here means the cart they see is the cart the server will accept.
  useEffect(() => {
    if (!menu) return;
    const products = (menu.categories || []).flatMap(c =>
      (c.products || []).map(p => ({ id: p.id, price: p.price, available: p.available })));
    const summary = reconcileToMenu(menu.menu_version, products);
    if (!summary) return;
    const parts: string[] = [];
    if (summary.repriced.length) parts.push(t('cart.prices_updated', 'Some prices in your cart were updated to the latest menu.'));
    if (summary.removed.length) parts.push(t('cart.items_removed', 'Some items are no longer available and were removed from your cart.'));
    showToast(parts.join(' '), 'info');
    // Keyed on menu_version only: reconcile is idempotent (it early-returns once the cart
    // is stamped to this version), so a re-run from changed identities is harmless.
  }, [menu?.menu_version]);

  // Light haptic tick on add-to-cart (where supported) — part of the tactile
  // ordering loop. Silent no-op on unsupported devices; never blocks the add.
  const tactileAdd = useCallback(() => {
    try { (navigator as any).vibrate?.(12); } catch { /* unsupported */ }
  }, []);

  const handleAddDetail = () => {
    if (!detailProduct || !detailProduct.available || isClosed) return;
    addItem({
      id: makeCartItemId(detailProduct.id, modifierGroupSelection),
      productId: detailProduct.id,
      name: detailProduct.name,
      quantity: quantity,
      price: detailProduct.price + calcModifierDelta(),
      options: modifierGroupSelection,
    });
    bounceCart();
    tactileAdd();
    showToast(t('cart.added_to_cart', 'Added to cart'), 'success');
    closeDetail();
  };

  const canAdd = (): boolean => {
    if (!detailProduct || !detailProduct.available || isClosed) return false;
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

      {/* Vendor info zone — between the header (which already shows the name+logo) and the categories.
          Google rating + reviews link + venue state + closing time, over a stylized-map backdrop (no API).
          The big vendor title was removed (redundant with the header). */}
      <section data-testid="vendor-info" className="relative w-full h-[150px] md:h-[180px] flex items-end overflow-hidden">
        {/* Backdrop: the venue's self-hosted Google video (R2 → /media) when present, else the stylized map.
            The video onError (404 = no video for this venue) reveals the map beneath. */}
        <div className="absolute inset-0"><StylizedMap className="w-full h-full" /></div>
        {locationInfo?.id && heroVideoOk && (
          <video
            className="absolute inset-0 w-full h-full object-cover"
            src={`/media/${locationInfo.id}/hero/video.mp4`}
            autoPlay muted loop playsInline preload="metadata"
            aria-hidden="true"
            onError={() => setHeroVideoOk(false)}
          />
        )}
        {/* Dark scrim so the overlaid white text stays ≥4.5:1 on any tenant palette. */}
        <div className="absolute inset-0" style={{ background: 'linear-gradient(to top, rgba(0,0,0,0.66) 0%, rgba(0,0,0,0.34) 48%, rgba(0,0,0,0.12) 100%)' }} />
        <motion.div
          className="relative z-10 w-full px-5 pb-4 flex flex-col gap-2"
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.45, ease: ease.out }}
        >
          {/* Rating + reviews (owner-entered Google data) */}
          <div className="flex items-center gap-2 flex-wrap text-step-xs font-medium" style={{ color: 'rgba(255,255,255,0.9)', textShadow: '0 1px 2px rgba(0,0,0,0.5)' }}>
            {locationInfo?.googleRating != null && (
              <span className="inline-flex items-center gap-1">
                <span className="inline-flex gap-0.5" style={{ color: 'var(--color-warning)' }}>
                  {[1,2,3,4,5].map(i => <i key={i} className={`ti ${i <= Math.round(locationInfo.googleRating!) ? 'ti-star-filled' : 'ti-star'}`} style={{ fontSize: '0.78rem' }} />)}
                </span>
                <span style={{ color: '#ffffff', fontWeight: 700 }}>{locationInfo.googleRating.toFixed(1)}</span>
                {locationInfo.googleReviewCount != null && <span className="opacity-75">({locationInfo.googleReviewCount})</span>}
              </span>
            )}
            {storeLinks.mapsUrl && (
              <a href={storeLinks.mapsUrl} target="_blank" rel="noopener noreferrer" data-testid="vendor-reviews-link"
                 className="inline-flex items-center gap-1 px-2.5 py-1 rounded-full font-semibold outline-none focus-visible:ring-2 focus-visible:ring-white/70"
                 style={{ background: 'rgba(255,255,255,0.16)', color: '#ffffff', backdropFilter: 'blur(4px)' }}>
                <i className="ti ti-brand-google" style={{ fontSize: '0.8rem' }} aria-hidden="true" />
                {t('client.read_reviews_google', 'Reviews on Google')}
                <i className="ti ti-external-link" style={{ fontSize: '0.7rem' }} aria-hidden="true" />
              </a>
            )}
          </div>
          {/* Venue state + closing time + delivery ETA */}
          <div className="flex items-center gap-2 flex-wrap text-step-xs" style={{ color: 'rgba(255,255,255,0.88)', textShadow: '0 1px 2px rgba(0,0,0,0.5)' }}>
            {venueStatus && <StateChip state={venueStatus} scope="venue" data-testid="venue-state-chip" />}
            {locationInfo?.closesAt && venueStatus !== 'closed' && (
              <span data-testid="vendor-closes-at" className="inline-flex items-center gap-1">
                <i className="ti ti-tools-kitchen-2" style={{ fontSize: '0.72rem' }} aria-hidden="true" />
                {t('client.closes_at', 'closes {{time}}', { time: locationInfo.closesAt })}
              </span>
            )}
            {geoStatus !== 'denied' && deliveryETA != null && (
              <span className="inline-flex items-center gap-1">
                <i className="ti ti-bike" style={{ fontSize: '0.72rem' }} aria-hidden="true" />
                ~{deliveryETA} min
              </span>
            )}
          </div>
        </motion.div>
      </section>

      {/* Unified sticky: Category nav + Search/Sort/Filter — sits below the h-14 header which is outside this scroll container */}
      <div ref={stickyRef} className="sticky top-0 z-40" style={{ background: 'var(--brand-bg)' }}>
        {/* Category nav */}
        <div className="relative border-b" style={{ borderColor: 'var(--brand-border)' }}>
          <nav data-testid="category-nav" className="h-11 overflow-x-auto hide-scrollbar scroll-fade-x flex items-center gap-0.5 px-3 pr-8" aria-label={t('client.categories', 'Categories')}>
            {loading ? (
              <div className="flex gap-4 px-2 h-full items-center">
                <div className="w-14 h-3.5 skeleton-block" />
                <div className="w-14 h-3.5 skeleton-block" />
                <div className="w-14 h-3.5 skeleton-block" />
              </div>
            ) : (
              [
                { id: null as string | null, name: t('client.all', 'All'), count: categories.reduce((n, c) => n + c.products.filter(p => p.available).length, 0), isChef: false },
                ...(chefPicksCategory ? [{ id: chefPicksCategory.id as string | null, name: chefPicksCategory.name, count: chefPicksCategory.products.filter(p => p.available).length, isChef: true }] : []),
                ...categories.map(c => ({ id: c.id as string | null, name: c.name, count: c.products.filter(p => p.available).length, isChef: false })),
              ].map(cat => {
                // Categories filter the menu (single-select). The active one gets the
                // underline + primary text; aria-pressed conveys the toggle state.
                const isActive = selectedCategory === cat.id;
                return (
                  <motion.button
                    key={cat.id ?? '__all__'}
                    whileTap={prefersReduced ? undefined : { scale: 0.97 }}
                    onClick={() => pickCategory(cat.id)}
                    aria-pressed={isActive}
                    className="h-11 flex items-center gap-1 px-3 whitespace-nowrap text-step-xs font-medium border-b-2 shrink-0 outline-none transition-colors duration-150 ease-out rounded-t-md focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-inset"
                    style={{
                      color: isActive ? (cat.isChef ? 'var(--brand-primary-readable)' : 'var(--brand-text)') : 'var(--brand-text-muted)',
                      borderColor: isActive ? 'var(--brand-primary)' : 'transparent',
                    }}
                  >
                    {cat.isChef && <span style={{ fontSize: '0.7rem' }}>✦</span>}
                    {cat.name}
                    <span className="text-step-2xs text-[var(--brand-text-muted)]">({cat.count})</span>
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
          <div className="relative border-b" style={{ borderColor: 'var(--brand-border)' }}>
          <div className="flex items-center gap-2 overflow-x-auto hide-scrollbar scroll-fade-x px-3 py-2 pr-8">
            {/* Compact search pill */}
            <div className="shrink-0" style={{ width: searchQuery ? 140 : 100, transition: 'width var(--motion-base) var(--ease-soft)', minWidth: 100 }}>
              <SearchInput
                size="sm"
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
                placeholder={t('common.search', 'Search')}
                onClear={() => setSearchQuery('')}
                containerClassName="w-full"
              />
            </div>
            <div className="w-px h-4 shrink-0" style={{ background: 'var(--brand-border)' }} />
            {/* Single price-sort toggle: tap cycles unsorted → price low→high → price high→low.
                Replaces the old 4-button row (owner: merge price sorting into one control). */}
            {(() => {
              const priceActive = sortBy === 'price-asc' || sortBy === 'price-desc';
              const next = sortBy === 'price-asc' ? 'price-desc' : sortBy === 'price-desc' ? 'default' : 'price-asc';
              return (
                <motion.button
                  onClick={() => setSortBy(next as typeof sortBy)}
                  whileTap={prefersReduced ? undefined : { scale: 0.95 }}
                  aria-label={t('sort.by_price', 'Sort by price')}
                  aria-pressed={priceActive}
                  className="px-3 h-9 rounded-full text-step-2xs font-medium whitespace-nowrap shrink-0 inline-flex items-center gap-1.5 outline-none transition-colors duration-150 ease-out focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--brand-bg)]"
                  style={{
                    background: priceActive ? 'var(--brand-primary)' : 'var(--brand-surface-raised)',
                    color: priceActive ? 'color-mix(in srgb, var(--brand-bg) 86%, #000)' : 'var(--brand-text-muted)',
                    fontWeight: priceActive ? 700 : 500,
                  }}
                >
                  <i className={`ti ${sortBy === 'price-asc' ? 'ti-sort-ascending-numbers' : sortBy === 'price-desc' ? 'ti-sort-descending-numbers' : 'ti-arrows-sort'}`} style={{ fontSize: '0.8rem' }} aria-hidden="true" />
                  <span>{t('sort.price', 'Price')}</span>
                </motion.button>
              );
            })()}
            {FILTER_LENSES_ENABLED && (
            <span style={{ display: 'contents' }} data-testid="macro-lens">
              {([
                { lens: 'protein-desc' as const, label: t('filter.lens_protein_desc', 'Most protein'), tid: 'macro-lens-protein', icon: 'ti ti-droplet' },
                { lens: 'kcal-asc' as const, label: t('filter.lens_kcal_asc', 'Calories: low to high'), tid: 'macro-lens-kcal', icon: 'ti ti-flame' },
              ]).map(({ lens, label, tid, icon }) => {
                const active = macroLens === lens;
                return (
                  <motion.button
                    key={lens}
                    data-testid={tid}
                    onClick={() => setMacroLens(active ? 'none' : lens)}
                    whileTap={prefersReduced ? undefined : { scale: 0.95 }}
                    aria-pressed={active}
                    className="px-3 h-9 rounded-full text-step-2xs font-medium whitespace-nowrap shrink-0 inline-flex items-center gap-1.5 outline-none transition-colors duration-150 ease-out focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)]"
                    style={{
                      background: active ? 'var(--brand-primary)' : 'var(--brand-surface-raised)',
                      color: active ? 'color-mix(in srgb, var(--brand-bg) 86%, #000)' : 'var(--brand-text-muted)',
                      fontWeight: active ? 700 : 500,
                    }}
                  >
                    <i className={icon} style={{ fontSize: '0.8rem' }} aria-hidden="true" />
                    <span>{label}</span>
                  </motion.button>
                );
              })}
            </span>
            )}
            {ALLERGENS_ENABLED && ALLERGEN_FILTER_ENABLED && (
            // display:contents — the wrapper carries the testid without altering the flex toolbar layout.
            <span style={{ display: 'contents' }} data-testid="allergen-filter-chips">
            {allAllergens.length > 0 && <div className="w-px h-4 shrink-0" style={{ background: 'var(--brand-border)' }} />}
            {allAllergens.map(a => {
              const s = getAllergenStyle(a);
              return (
                <motion.button key={a} onClick={() => setFilterAllergen(filterAllergen === a ? null : a)} whileTap={prefersReduced ? undefined : { scale: 0.95 }}
                  aria-pressed={filterAllergen === a}
                  className="px-3 h-9 rounded-full text-step-2xs font-semibold uppercase whitespace-nowrap shrink-0 flex items-center border outline-none transition-[background-color,color,opacity,border-color] duration-150 ease-out focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--brand-bg)]"
                  style={{
                    background: filterAllergen === a ? 'var(--brand-primary)' : s.bg,
                    color: filterAllergen === a ? 'color-mix(in srgb, var(--brand-bg) 86%, #000)' : s.text,
                    borderColor: filterAllergen === a ? 'var(--brand-primary)' : 'var(--brand-border)',
                    opacity: filterAllergen && filterAllergen !== a ? 0.4 : 1,
                  }}
                >
                  {t(`allergen.${a.toLowerCase()}`, a)}
                </motion.button>
              );
            })}
            </span>
            )}
          </div>
          {/* Right fade hint for horizontal scroll (matches category nav) */}
          <div className="absolute right-0 top-0 bottom-0 w-6 pointer-events-none" style={{ background: 'linear-gradient(to right, transparent, var(--brand-bg))' }} />
          </div>
        )}
      </div>

      {/* Venue state banner — closed vs busy are distinct eater-facing states.
          `busy` (kitchen busy / raised ETA) is NOT closed: ordering stays open. */}
      {venueStatus === 'closed' && (
        <motion.div
          data-testid="venue-closed-banner"
          role="status"
          initial={{ opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          className="mx-4 my-3 px-4 py-3.5 rounded-2xl border-2 flex items-center gap-3 shadow-sm"
          style={{ background: 'color-mix(in srgb, var(--color-danger, #dc2626) 12%, var(--brand-surface))', borderColor: 'color-mix(in srgb, var(--color-danger, #dc2626) 45%, transparent)' }}
        >
          <span className="shrink-0 inline-flex items-center justify-center w-9 h-9 rounded-full" style={{ background: 'color-mix(in srgb, var(--color-danger, #dc2626) 18%, transparent)', color: 'var(--color-danger, #dc2626)' }}>
            <i className="ti ti-clock-off text-xl" />
          </span>
          <div className="min-w-0">
            <p className="text-sm font-bold leading-tight" style={{ color: 'var(--brand-text)' }}>{t('client.closed_title', 'Currently closed')}</p>
            <p className="text-step-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>{t('client.closed_browse_hint', 'You can browse the full menu — ordering reopens during opening hours.')}</p>
          </div>
        </motion.div>
      )}
      {venueStatus === 'busy' && (
        <motion.div
          data-testid="venue-busy-banner"
          initial={{ opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          className="mx-4 my-3 px-4 py-3 rounded-xl border flex items-center gap-3 text-sm font-medium"
          style={{ background: 'color-mix(in srgb, var(--color-warning, #D97706) 10%, transparent)', borderColor: 'var(--color-warning, #D97706)', color: 'var(--color-warning, #D97706)' }}
        >
          <i className="ti ti-flame text-lg shrink-0" />
          <span>{t('client.kitchen_busy', 'The kitchen is busy right now — orders may take a little longer than usual.')}</span>
        </motion.div>
      )}

      {/* Menu Content — min-h prevents layout shifts when filtering/sorting changes product count */}
      <main className="max-w-6xl mx-auto pt-4 min-h-screen">
        {loading ? (
          <div className="px-4 grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
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
            <i className={`text-5xl opacity-40 mb-3 ${notFound ? 'ti ti-map-pin-off' : 'ti ti-tools-kitchen-2'}`} style={{ color: 'var(--brand-primary)' }} />
            <p className="text-base font-semibold" style={{ color: 'var(--brand-text)' }}>
              {notFound
                ? t('client.venue_not_found', 'Restaurant not found')
                : t('client.empty_menu', fetchError ? 'Failed to load menu' : 'Menu unavailable')}
            </p>
            <p className="text-sm mt-1 max-w-xs" style={{ color: 'var(--brand-text-muted)' }}>
              {notFound
                ? t('client.venue_not_found_hint', "We couldn't find a restaurant at this link. Check the address or head back home.")
                : fetchError
                ? t('client.empty_menu_error_hint', "We couldn't load the menu. Please try again in a moment.")
                : t('client.empty_menu_unavailable_hint', "This restaurant hasn't published its menu yet.")}
            </p>
            {notFound ? (
              <a href="/" className="mt-4 inline-flex items-center px-5 py-2 rounded-xl text-sm font-semibold text-[var(--brand-bg)] outline-none transition-[transform,box-shadow] duration-150 ease-out active:scale-95 min-h-11 focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-bg)]" style={{ background: 'var(--brand-primary-strong)' }}>
                <i className="ti ti-home mr-1.5" />{t('client.go_home', 'Back to home')}
              </a>
            ) : fetchError && (
              <motion.button onClick={() => { setRetryCount(c => c + 1); }} whileTap={prefersReduced ? undefined : { scale: 0.97 }} className="mt-4 px-5 py-2 rounded-xl text-sm font-semibold text-[var(--brand-bg)] outline-none transition-[transform,box-shadow] duration-150 ease-out active:scale-95 min-h-11 focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-bg)]" style={{ background: 'var(--brand-primary-strong)' }}>
                <i className="ti ti-refresh mr-1.5" />{t('client.retry', 'Retry')}
              </motion.button>
            )}
          </div>
) : displayCategories.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 px-4 text-center">
            <i className="ti ti-search-off text-5xl opacity-40 mb-3" style={{ color: 'var(--brand-primary)' }} />
            <p className="text-base font-semibold" style={{ color: 'var(--brand-text)' }}>
              {searchQuery
                ? t('client.no_results_query', 'No items match “{{query}}”', { query: searchQuery })
                : t('client.no_results', 'No products match your filters')}
            </p>
            <p className="text-sm mt-1 mb-4 max-w-xs" style={{ color: 'var(--brand-text-muted)' }}>
              {t('client.no_results_hint', 'Try a different search or clear your filters to see the full menu.')}
            </p>
            <motion.button
              onClick={() => { setSortBy('default'); setFilterAllergen(null); setSearchQuery(''); setSelectedCategory(null); }}
              whileTap={prefersReduced ? undefined : { scale: 0.97 }}
              className="px-5 py-2 rounded-xl text-sm font-semibold text-[var(--brand-bg)] outline-none transition-[transform,box-shadow] duration-150 ease-out active:scale-95 min-h-11 focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-bg)]"
              style={{ background: 'var(--brand-primary-strong)' }}
            >
              {t('client.browse_menu', 'Browse full menu')}
            </motion.button>
          </div>
        ) : (
          [
            // Chef's Picks is a curated overlay on the DEFAULT category order; a global
            // sort already reorders everything into one flat list, so prepending picks
            // there would break the monotonic order. Only show it for 'default' AND when
            // no category filter is active (a filter already scopes the list).
            ...(sortBy === 'default' && !selectedCategory && chefPicksCategory ? [chefPicksCategory] : []),
            ...displayCategories,
          ].map(category => {
            const isChefCat = category.id === CHEF_PICKS_ID;
            return (
            <motion.section
              key={category.id}
              id={category.id}
              className="mb-7"
              style={{ scrollMarginTop: scrollOffset + 'px' }}
              initial={prefersReduced ? false : { opacity: 0, transform: 'translateY(6px)' }}
              whileInView={{ opacity: 1, transform: 'translateY(0px)' }}
              viewport={{ once: true, margin: '-60px' }}
              transition={{ duration: prefersReduced ? 0 : 0.22, ease: ease.out }}
            >
              <h2 className="text-lg font-bold px-4 mb-3 flex items-center gap-2" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' }}>
                {isChefCat && <span style={{ color: 'var(--brand-primary)', fontSize: '1rem' }}>✦</span>}
                {category.name}
              </h2>
              <motion.div
                className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3 px-4"
                variants={{ visible: { transition: { staggerChildren: prefersReduced ? 0 : 0.03 } } }}
                initial="hidden"
                whileInView="visible"
                viewport={{ once: true, margin: '-40px' }}
              >
                {category.products.map(product => {
                  const nutrition = bomToNutrition(product);
                  return (
                  <motion.div
                    key={product.id}
                    className="relative"
                    variants={prefersReduced
                      ? { hidden: { opacity: 0 }, visible: { opacity: 1, transition: { duration: 0 } } }
                      : { hidden: { opacity: 0, transform: 'translateY(6px)' }, visible: { opacity: 1, transform: 'translateY(0px)', transition: { duration: 0.18, ease: ease.out } } }}
                  >
                    {COMPARISON_ENABLED && (() => {
                      const picked = compareIds.includes(product.id);
                      const full = compareIds.length >= 2 && !picked;
                      return (
                        <button
                          type="button"
                          data-testid="compare-toggle"
                          onClick={(e) => { e.preventDefault(); e.stopPropagation(); if (!full) toggleCompare(product.id); }}
                          disabled={full}
                          aria-pressed={picked}
                          aria-label={picked ? t('compare.remove', 'Remove from compare') : t('compare.add', 'Add to compare')}
                          title={picked ? t('compare.remove', 'Remove from compare') : t('compare.add', 'Add to compare')}
                          className="absolute top-1.5 left-1.5 z-10 min-w-[30px] min-h-[30px] flex items-center justify-center rounded-full border outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)]"
                          style={{
                            background: picked ? 'var(--brand-primary)' : 'color-mix(in srgb, var(--brand-bg) 82%, transparent)',
                            color: picked ? 'color-mix(in srgb, var(--brand-bg) 86%, #000)' : 'var(--brand-text)',
                            borderColor: picked ? 'var(--brand-primary)' : 'var(--brand-border)',
                            opacity: full ? 0.35 : 1,
                            backdropFilter: 'blur(4px)',
                          }}
                        >
                          <i className={picked ? 'ti ti-check' : 'ti ti-arrows-left-right'} style={{ fontSize: '0.85rem' }} aria-hidden="true" />
                        </button>
                      );
                    })()}
                    <ProductCard product={{
                      id: product.id,
                      name: product.name,
                      description: product.description,
                      price: product.price,
                      prepTimeMinutes: product.prep_time_minutes,
                      image: getImageUrl(product) || undefined,
                      isAvailable: product.available,
                      kcal: nutrition.kcal || undefined,
                      protein: nutrition.protein || undefined,
                      fat: nutrition.fat || undefined,
                      carbs: nutrition.carbs || undefined,
                      ingredients: nutrition.ingredients.length ? nutrition.ingredients : undefined,
                      taste: getAttr(product, 'taste'),
                      chefPick: !!product.attributes?.chef_pick,
                    }}
                    compareGutter={COMPARISON_ENABLED}
                    onClick={() => handleProductClick(product)}
                    onAdd={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      if (!product.available) return;
                      if (isClosed) { showToast(t('client.closed_cannot_order', 'The restaurant is closed — ordering is paused until it reopens.'), 'info'); return; }
                      if (!product.modifier_groups?.length) {
                        addItem({ id: `cart_${product.id}`, productId: product.id, name: product.name, quantity: 1, price: product.price, options: {} });
                        bounceCart();
                        tactileAdd();
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

      {/* Compare — floating selection bar (1–2 chosen) + the panel. Affordance-only (no long-press, FB-M4). */}
      {COMPARISON_ENABLED && compareIds.length > 0 && !compareOpen && !detailProduct && (
        <motion.div
          className="fixed left-1/2 -translate-x-1/2 z-sticky w-[calc(100%-2rem)] max-w-md rounded-full shadow-lg flex items-center gap-2 px-3 py-2"
          style={{ bottom: 'calc(env(safe-area-inset-bottom, 0px) + 5.5rem)', background: 'var(--brand-surface-raised)', border: '1px solid var(--brand-border)' }}
          initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
          transition={{ duration: prefersReduced ? 0 : 0.18 }}
          data-testid="compare-bar"
        >
          <button type="button" onClick={() => setCompareIds([])} className="text-step-2xs font-medium px-2 py-1 shrink-0" style={{ color: 'var(--brand-text-muted)' }} aria-label={t('compare.exit', 'Done')}>
            <i className="ti ti-x" aria-hidden="true" /> {compareIds.length}/2
          </button>
          <span className="text-step-2xs flex-1 min-w-0 truncate text-center font-medium" style={{ color: 'var(--brand-text)' }}>
            {compareProducts.map(p => p.name).join('  vs  ')}
            {compareProducts.length === 1 && (
              <span className="font-normal" style={{ color: 'var(--brand-text-muted)' }}> · {t('compare.pick_one_more', 'pick 1 more')}</span>
            )}
          </span>
          <button
            type="button"
            data-testid="compare-open"
            onClick={() => setCompareOpen(true)}
            disabled={compareIds.length < 2}
            className="text-step-2xs font-bold px-4 h-9 rounded-full disabled:opacity-40 shrink-0 ml-auto"
            style={{ background: 'var(--brand-primary)', color: 'color-mix(in srgb, var(--brand-bg) 86%, #000)' }}
          >
            {t('compare.cta', 'Compare')}
          </button>
        </motion.div>
      )}
      <AnimatePresence>
      {COMPARISON_ENABLED && compareOpen && compareProducts[0] && compareProducts[1] && (
        <MenuComparePanel
          a={toCompareInput(compareProducts[0])}
          b={toCompareInput(compareProducts[1])}
          onClose={() => setCompareOpen(false)}
        />
      )}
      </AnimatePresence>

      {/* Product Detail Modal — z-modal sits above the sticky cart bar (z-sticky)
          so the two never stack; body scroll is locked while it's open. */}
      <AnimatePresence>
      {detailProduct && (
        <motion.div
          key="product-modal"
          className="fixed inset-0 z-modal flex items-end md:items-center justify-center"
          style={{ background: 'color-mix(in srgb, var(--brand-bg) 60%, transparent)', backdropFilter: 'blur(4px)' }}
          role="dialog" aria-modal="true"
          initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: prefersReduced ? 0 : 0.22, ease: ease.out }}
        >
          <button type="button" className="absolute inset-0 cursor-default" aria-label={t('common.close', 'Close')} onClick={closeDetail} />
          <motion.div
            className="relative w-full md:max-w-lg max-h-[85vh] overflow-auto rounded-t-2xl md:rounded-2xl"
            style={{ background: 'var(--brand-bg)', boxShadow: 'var(--elev-4)' }}
            initial={prefersReduced ? { opacity: 0 } : { transform: 'translateY(28px) scale(0.97)', opacity: 0 }}
            animate={{ transform: 'translateY(0px) scale(1)', opacity: 1 }}
            exit={prefersReduced ? { opacity: 0 } : { transform: 'translateY(18px) scale(0.97)', opacity: 0, transition: { duration: 0.18, ease: ease.soft } }}
            transition={prefersReduced ? { duration: 0.15 } : { type: 'spring', stiffness: 340, damping: 32 }}
          >
            {/* STICKY dismiss bar — pinned to the top of the (scrollable) sheet so Close is ALWAYS reachable,
                including after scrolling down (the old in-image button scrolled away — that was the mobile
                "hard to close" pain). Zero-height + transparent; only the controls capture taps. */}
            <div className="sticky top-0 z-30 h-0 pointer-events-none">
              <motion.button
                type="button"
                whileTap={prefersReduced ? undefined : { scale: 0.95 }}
                onClick={closeDetail}
                aria-label={t('common.close', 'Close')}
                className="pointer-events-auto absolute top-3 right-3 min-w-[44px] min-h-[44px] rounded-full flex items-center justify-center shadow-lg outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)]"
                style={{ background: 'var(--brand-bg)', color: 'var(--brand-text)', border: '1px solid var(--brand-border)' }}
              >
                <i className="ti ti-x text-2xl" />
              </motion.button>
              {/* Mobile grabber — tap to close (a familiar bottom-sheet dismiss affordance). */}
              <button
                type="button"
                onClick={closeDetail}
                aria-label={t('common.close', 'Close')}
                className="md:hidden pointer-events-auto absolute top-1.5 left-1/2 -translate-x-1/2 flex items-center justify-center w-24 h-7"
              >
                <span className="block w-10 h-1.5 rounded-full" style={{ background: 'color-mix(in srgb, var(--brand-text) 55%, transparent)' }} />
              </button>
            </div>
            {/* Image — taller hero with more room; the photo is shown in FULL (object-contain) over a
                blurred fill of itself, so nothing is cropped (operator directive: "display full images"). */}
            <div className="relative w-full overflow-hidden" style={{ background: 'var(--brand-surface-raised)' }}>
              {detailMedia.length > 0 ? (
                // Rich media (ADR-0002): gallery for >1, single renderer for 1. Suspense shows
                // the primary image while the code-split chunk loads → no flash. A renderer
                // failure degrades to its poster (handled inside MediaRenderer), never throws.
                <div className="relative w-full aspect-[4/3] md:aspect-[16/10] flex items-center justify-center">
                <Suspense fallback={getImageUrl(detailProduct) ? (
                  <img src={getImageUrl(detailProduct)!} alt={detailProduct.name} className="w-full h-full object-cover" />
                ) : null}>
                  {detailMedia.length > 1 ? (
                    <MediaGallery media={detailMedia} posterFallbackUrl={getImageUrl(detailProduct) || undefined} />
                  ) : (
                    <MediaRenderer media={detailMedia[0]!} active posterFallbackUrl={getImageUrl(detailProduct) || undefined} />
                  )}
                </Suspense>
                </div>
              ) : getImageUrl(detailProduct) && !imageLoadError ? (
                // FULL image — shown at its natural ratio (w-full h-auto), so every side is visible and
                // nothing is cropped or letterboxed (operator directive). Height capped so a tall portrait
                // photo can't dominate the sheet.
                <img
                  src={getImageUrl(detailProduct)!}
                  alt={detailProduct.name}
                  className="block w-full h-auto max-h-[68vh] object-contain"
                  onError={() => setImageLoadError(true)}
                />
              ) : (
                // Crafted on-brand no-photo fallback (mirrors ProductCard): warm
                // brand-tinted gradient + faint dotted texture + a cutlery glyph in
                // a soft medallion. Needs an explicit height → wrapped in an aspect box.
                <div
                  className="relative w-full aspect-[4/3] md:aspect-[16/10] flex flex-col items-center justify-center gap-3 select-none"
                  style={{
                    background:
                      'linear-gradient(135deg, color-mix(in srgb, var(--brand-primary) 16%, var(--brand-surface)) 0%, var(--brand-surface-raised) 55%, color-mix(in srgb, var(--brand-primary) 8%, var(--brand-surface)) 100%)',
                  }}
                >
                  <div
                    className="absolute inset-0"
                    style={{
                      backgroundImage:
                        'radial-gradient(color-mix(in srgb, var(--brand-primary) 30%, transparent) 1px, transparent 1.4px)',
                      backgroundSize: '16px 16px',
                      opacity: 0.35,
                    }}
                  />
                  <span
                    className="relative flex items-center justify-center rounded-full"
                    style={{
                      width: 'clamp(3.5rem, 16%, 5rem)',
                      aspectRatio: '1 / 1',
                      background: 'color-mix(in srgb, var(--brand-surface) 78%, transparent)',
                      border: '1px solid color-mix(in srgb, var(--brand-primary) 28%, transparent)',
                      boxShadow: '0 2px 14px color-mix(in srgb, var(--brand-primary) 18%, transparent)',
                    }}
                  >
                    <i className="ti ti-tools-kitchen-2 leading-none" style={{ fontSize: '1.75rem', color: 'var(--brand-primary)' }} />
                  </span>
                  {/* No dish name here: it already renders once as the modal body heading
                      (the <h2> below). Repeating it on the photoless hero read as a
                      duplicate title. */}
                </div>
              )}
              {/* Close lives in the sticky bar above (always reachable while scrolling). */}
            </div>

            {/* Content — gentle rise after the hero photo morphs into place */}
            <motion.div
              className="p-5 space-y-5"
              initial={prefersReduced ? false : { opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={prefersReduced ? { duration: 0 } : { delay: 0.1, duration: 0.32, ease: ease.out }}
            >
              {/* Name, Description, Price */}
              <div>
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1.5">
                      {detailProduct.attributes?.chef_pick && (
                        <span className="text-step-2xs font-semibold px-2 py-0.5 rounded-full inline-flex items-center gap-1" style={{ background: 'color-mix(in srgb, var(--brand-primary) 10%, transparent)', color: 'var(--brand-primary-readable)' }}>
                          <i className="ti ti-flame" style={{ fontSize: '0.6rem' }} />
                          {t('client.popular', 'Popular')}
                        </span>
                      )}
                    </div>
                    {/* Body font (not the heading serif) to match the menu card title — opened/closed consistency. */}
                    <h2 className="text-xl font-bold leading-tight" style={{ color: 'var(--brand-text)' }}>{detailProduct.name}</h2>
                    {!detailProduct.available && (
                      <span className="inline-block mt-1.5 text-step-2xs font-semibold px-2 py-0.5 rounded" style={{ background: 'rgba(220,38,38,0.1)', color: 'var(--color-danger)' }}>
                        {t('client.unavailable', 'Unavailable')}
                      </span>
                    )}
                  </div>
                  <motion.div
                    className="flex flex-col items-end shrink-0"
                    initial={prefersReduced ? false : { scale: 0.92, opacity: 0 }}
                    animate={{ scale: 1, opacity: 1 }}
                    transition={prefersReduced ? { duration: 0 } : { delay: 0.2, duration: 0.28, ease: ease.out }}
                  >
                    <div className="text-xl font-black whitespace-nowrap" style={{ color: 'var(--brand-primary-readable, var(--brand-text))' }}>
                      <PriceDisplay amount={detailProduct.price + calcModifierDelta()} />
                    </div>
                    {detailProduct.prep_time_minutes != null && (
                      <span
                        className="inline-flex items-center gap-1 text-step-2xs font-medium whitespace-nowrap mt-0.5"
                        style={{ color: 'var(--brand-text-muted)' }}
                        title={t('product.prep_cooking_time', 'Cooking time (not delivery)')}
                        aria-label={t('product.prep_cooking_time', 'Cooking time (not delivery)')}
                      >
                        <i className="ti ti-tools-kitchen-2" style={{ fontSize: '0.75rem' }} aria-hidden="true" />
                        {t('product.prep_minutes', '~{{n}} min', { n: detailProduct.prep_time_minutes })}
                        <span style={{ opacity: 0.7 }}>· {t('product.prep_cooking_label', 'cooking')}</span>
                      </span>
                    )}
                  </motion.div>
                </div>
                {detailProduct.description && (
                  <p className="text-sm mt-2 leading-relaxed" style={{ color: 'var(--brand-text)' }}>{detailProduct.description}</p>
                )}
              </div>

              {/* Taste Section */}
              {(() => {
                const taste = getAttr(detailProduct, 'taste');
                if (!taste || typeof taste !== 'object') return null;
                const icons: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };
                // Skip axes we have no icon for — a hollow ti-circle fallback reads as an
                // empty/broken glyph, so an unmapped axis is dropped rather than rendered blank.
                const entries = Object.entries(taste).filter(([axis, v]) => (v as number) > 0 && icons[axis]);
                if (!entries.length) return null;
                return (
                  <div className="rounded-xl p-4" style={{ background: 'var(--brand-surface)' }}>
                    <h3 className="text-xs font-semibold uppercase tracking-wider mb-3 flex items-center gap-1.5" style={{ color: 'var(--brand-text-muted)' }}>
                      <i className="ti ti-flask" /> {t('common.taste', 'Taste')}
                    </h3>
                    <div className="flex flex-wrap gap-x-4 gap-y-2">
                      {entries.map(([axis, level]) => {
                        const label = t(`admin.taste_${axis}`, axis);
                        return (
                          <span key={axis} className="inline-flex items-center gap-1.5 text-sm" style={{ color: 'var(--brand-text-muted)' }} aria-label={`${label}: ${level}`}>
                            <span className="font-medium">{label}</span>
                            <span className="inline-flex items-center gap-0.5" aria-hidden="true">
                              {Array.from({ length: level as number }).map((_, i) => (
                                <i key={i} className={icons[axis]} style={{ fontSize: '0.7rem' }} />
                              ))}
                            </span>
                          </span>
                        );
                      })}
                    </div>
                  </div>
                );
              })()}

              {/* Nutrition + ingredients — the DishStats data-viz (calorie ring + macro split + per-ingredient
                  bars with declared BOM amounts). Replaces the old 4-number grid and the chip list. */}
              {(() => {
                const n = bomToNutrition(detailProduct);
                return <DishStats variant="full" macros={{ kcal: n.kcal, protein: n.protein, fat: n.fat, carbs: n.carbs }} ingredients={dishIngredients(detailProduct)} />;
              })()}

              {ALLERGENS_ENABLED && (() => {
                // FROZEN (ALLERGENS_ENABLED=false). When re-enabled this is the STEP-0 single-source surface
                // (council #12/#5/#5d): allergenSurfaceOf = computeAllergenSurface (declared∪recipe), PRESENCE
                // only, floor on empty, reliance bound always attached. Architecture preserved, display off.
                const { known } = allergenSurfaceOf(detailProduct);
                return (
                  <div className="rounded-xl p-4" style={{ background: 'rgba(220,38,38,0.06)', borderColor: 'rgba(220,38,38,0.15)', borderWidth: 1 }} data-testid="allergen-surface">
                    <h3 className="text-xs font-semibold uppercase tracking-wider mb-2.5 flex items-center gap-1.5" style={{ color: 'var(--color-danger)' }}>
                      <i className="ti ti-alert-triangle" /> {t('client.allergens', 'Allergens')}
                    </h3>
                    {known.length > 0 ? (
                      <>
                        <p className="text-step-2xs mb-1.5" style={{ color: 'var(--brand-text-muted)' }}>{t('client.allergen_declared_to_contain', 'Declared to contain')}:</p>
                        <div className="flex flex-wrap gap-1.5">
                          {known.map(a => {
                            const s = getAllergenStyle(a);
                            return (
                              <span key={a} className="px-2 py-0.5 rounded font-semibold text-step-2xs uppercase" style={{ background: s.bg, color: s.text }}>
                                {t(`allergen.${a.toLowerCase()}`, a)}
                              </span>
                            );
                          })}
                        </div>
                      </>
                    ) : (
                      <p className="text-step-2xs" data-testid="allergen-no-info" style={{ color: 'var(--brand-text-muted)' }}>{t('client.allergen_info_not_provided', 'Allergen info not provided')}</p>
                    )}
                    <p className="text-step-2xs mt-2 italic" data-testid="allergen-reliance" style={{ color: 'var(--brand-text-muted)' }}>{t('client.allergen_confirm_venue', 'Not a complete allergen list — please confirm with the venue for severe allergies.')}</p>
                  </div>
                );
              })()}

              {/* Modifier Groups */}
              {(detailProduct.modifier_groups || []).length > 0 && (
                <div className="border-t pt-5" style={{ borderColor: 'var(--brand-border)' }}>
                  <h3 className="text-xs font-semibold uppercase tracking-wider mb-3 flex items-center gap-1.5" style={{ color: 'var(--brand-text-muted)' }}>
                    <i className="ti ti-settings" /> {t('client.customize', 'Customize')}
                  </h3>
                  {(detailProduct.modifier_groups || []).map(group => {
                    const displayType = resolveDisplayType(group);
                    return (
                    <div
                      key={group.id}
                      className="mb-4 last:mb-0"
                      data-testid="modifier-group"
                      data-display-type={displayType}
                    >
                      <div className="flex items-center gap-2 mb-2.5">
                        <i
                          className={`ti ${displayType === 'radio' ? 'ti-circle-dot' : displayType === 'checkbox' ? 'ti-checkbox' : displayType === 'quantity' ? 'ti-number' : 'ti-chevron-down'}`}
                          style={{ fontSize: '0.8rem', color: 'var(--brand-text-muted)' }}
                          aria-hidden="true"
                        />
                        <span className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>{group.name}</span>
                        {group.required && (
                          <span className="text-step-2xs px-1.5 py-0.5 rounded font-medium" style={{ background: 'rgba(220,38,38,0.08)', color: 'var(--color-danger)' }}>
                            {t('client.required', 'Required')}
                          </span>
                        )}
                        {group.max_select > 1 && (
                          <span className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>
                            {t('client.up_to', 'up to')} {group.max_select}
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
                              whileTap={prefersReduced ? undefined : { scale: 0.97 }}
                              aria-pressed={isSelected}
                              className={`px-3.5 py-2 text-step-sm font-medium active:scale-[0.97] border min-h-11 outline-none transition-[background-color,color,border-color,transform] duration-150 ease-out focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--brand-bg)] ${
                                isSelected ? 'border-2' : ''
                              }`}
                              style={{
                                borderRadius: 'var(--brand-radius-sm)',
                                background: isSelected ? 'var(--brand-primary-light, var(--brand-surface-raised))' : 'var(--brand-surface)',
                                borderColor: isSelected ? 'var(--brand-primary)' : 'var(--brand-border)',
                                color: isSelected ? 'var(--brand-primary-readable)' : 'var(--brand-text)',
                              }}
                            >
                              {mod.name}
                                  {mod.price_delta > 0 && (
                                <span className="ml-1 text-step-2xs" style={{ color: isSelected ? 'var(--brand-primary-readable)' : 'var(--brand-text-muted)' }}>
                                  +&nbsp;<PriceDisplay amount={mod.price_delta} />
                                </span>
                              )}
                            </motion.button>
                          );
                        })}
                      </div>
                    </div>
                    );
                  })}
                </div>
              )}

              {/* Modifier Summary */}
              {Object.values(modifierGroupSelection).some(s => s.length > 0) && (
                <div className="text-step-2xs leading-relaxed px-1" style={{ color: 'var(--brand-text-muted)' }}>
                  <span className="font-medium" style={{ color: 'var(--brand-text)' }}>{t('menu.selected', 'Selected:')}</span>{' '}
                  {Object.entries(modifierGroupSelection).map(([gid, selectedIds]) => {
                    const group = (detailProduct.modifier_groups || []).find(g => g.id === gid);
                    if (!group || selectedIds.length === 0) return null;
                    return selectedIds.map(sid => group.modifiers.find(m => m.id === sid)?.name).filter(Boolean).join(', ');
                  }).filter(Boolean).join(' · ')}
                </div>
              )}

              {/* Quantity + Add to Cart — always ONE row (stepper + a compact add button). */}
              <div className="flex flex-row items-center gap-2 pt-4 border-t" style={{ borderColor: 'var(--brand-border)' }}>
                <div className="flex items-center shrink-0 rounded-xl p-1" style={{ background: 'var(--brand-surface)' }}>
                  <motion.button
                    onClick={() => setQuantity(q => Math.max(1, q - 1))}
                    whileTap={prefersReduced ? undefined : { scale: 0.92 }}
                    disabled={quantity <= 1}
                    className="min-w-[44px] min-h-[44px] rounded-lg flex items-center justify-center text-base font-medium outline-none transition-[color,transform] duration-150 ease-out [@media(hover:hover)]:hover:text-[var(--brand-primary)] disabled:opacity-30 focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-inset"
                    style={{ color: 'var(--brand-text)' }}
                    aria-label={t('common.decrease_quantity', 'Decrease quantity')}
                  >
                    <i className="ti ti-minus" />
                  </motion.button>
                  <span className="text-base font-semibold w-7 text-center" style={{ color: 'var(--brand-text)' }}>{quantity}</span>
                  <motion.button
                    onClick={() => setQuantity(q => q + 1)}
                    whileTap={prefersReduced ? undefined : { scale: 0.92 }}
                    className="min-w-[44px] min-h-[44px] rounded-lg flex items-center justify-center text-base font-medium outline-none transition-[color,transform] duration-150 ease-out [@media(hover:hover)]:hover:text-[var(--brand-primary)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-inset"
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
                  whileTap={prefersReduced || !canAdd() ? undefined : { scale: 0.97 }}
                  className="flex-1 min-w-0 h-[46px] text-[var(--brand-bg)] font-bold text-step-sm outline-none transition-[transform,opacity] duration-150 ease-out disabled:opacity-40 flex items-center justify-between gap-2 px-4 focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-bg)]"
                  style={{ background: detailProduct.available && !isClosed ? 'var(--brand-primary-strong)' : 'var(--brand-text-muted)', borderRadius: 'var(--brand-radius-btn)' }}
                >
                  {!detailProduct.available ? (
                    <span className="w-full text-center">{t('client.unavailable', 'Unavailable')}</span>
                  ) : isClosed ? (
                    <span className="w-full text-center">{t('client.closed_short', 'Currently closed')}</span>
                  ) : (
                    <>
                      <span className="truncate min-w-0">{t('client.add_to_cart', 'Add to Cart')}</span>
                      <span className="font-extrabold shrink-0"><PriceDisplay amount={(detailProduct.price + calcModifierDelta()) * quantity} /></span>
                    </>
                  )}
                </motion.button>
              </div>
            </motion.div>
          </motion.div>
        </motion.div>
      )}
      </AnimatePresence>

      {/* Storefront footer — always closes the page (outside embed/activation
          preview). Restaurant identity + address, with Google Maps + socials when
          set. Each absent link simply doesn't render. */}
      {!isEmbed && (() => {
        // Two-column footer: LEFT = a stylized (decorative, not a live tile) map with the vendor pin,
        // clickable through to real Maps; RIGHT = identity + address + the contact number (tap-to-call)
        // and the WhatsApp/socials rail. Each link renders only when its datum exists.
        const waDigits = storeLinks.phone ? storeLinks.phone.replace(/\D/g, '') : '';
        const mapsHref = storeLinks.mapsUrl
          || (locationInfo?.lat != null && locationInfo?.lng != null
            ? `https://www.google.com/maps/search/?api=1&query=${locationInfo.lat},${locationInfo.lng}`
            : null);
        const iconCls = "text-lg inline-flex items-center justify-center w-10 h-10 rounded-full outline-none transition-[color,transform] duration-150 ease-out [@media(hover:hover)]:hover:text-[var(--brand-primary)] active:scale-95 focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-bg)]";
        const iconSty = { color: 'var(--brand-text-muted)', background: 'var(--brand-surface)' } as React.CSSProperties;
        return (
        <footer className="mt-12 border-t" style={{ borderColor: 'var(--brand-border)' }}>
          <div className="grid grid-cols-1 sm:grid-cols-2">
            {/* LEFT — stylized map + pin */}
            {mapsHref ? (
              <a href={mapsHref} target="_blank" rel="noopener noreferrer" aria-label={t('client.view_on_maps', 'View on Google Maps')}
                 className="relative block h-40 sm:h-full min-h-[10rem] overflow-hidden group outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-inset">
                {locationInfo?.lat != null && locationInfo?.lng != null ? <SatelliteMap lat={locationInfo.lat} lng={locationInfo.lng} className="w-full h-full" /> : <StylizedMap />}
                <span className="absolute bottom-2 left-2 text-step-2xs font-medium px-2 py-1 rounded-full inline-flex items-center gap-1" style={{ background: 'color-mix(in srgb, var(--brand-bg) 78%, transparent)', color: 'var(--brand-text)' }}>
                  <i className="ti ti-map-pin" aria-hidden="true" /> {t('client.view_on_maps', 'View on Google Maps')}
                </span>
              </a>
            ) : (
              <div className="relative h-40 sm:h-full min-h-[10rem] overflow-hidden">{locationInfo?.lat != null && locationInfo?.lng != null ? <SatelliteMap lat={locationInfo.lat} lng={locationInfo.lng} className="w-full h-full" /> : <StylizedMap />}</div>
            )}
            {/* RIGHT — identity + address + contact */}
            <div className="px-5 py-7 flex flex-col gap-2 justify-center">
              <div className="text-lg font-bold" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' }}>
                {menu?.location_name || t('client.menu', 'Menu')}
              </div>
              {storeAddress && (
                <div className="text-xs leading-relaxed" style={{ color: 'var(--brand-text-muted)' }}>{storeAddress}</div>
              )}
              {storeLinks.phone && (
                <a href={`tel:${storeLinks.phone}`} className="text-sm font-semibold inline-flex items-center gap-1.5 mt-1 w-fit outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] rounded" style={{ color: 'var(--brand-text)' }} aria-label={t('client.call_restaurant', 'Call the restaurant')}>
                  <i className="ti ti-phone" aria-hidden="true" style={{ color: 'var(--brand-primary)' }} /> {storeLinks.phone}
                </a>
              )}
              {(waDigits || storeLinks.instagram || storeLinks.facebook) && (
                <div className="flex items-center gap-2 mt-2">
                  {waDigits && (
                    <a href={`https://wa.me/${waDigits}`} target="_blank" rel="noopener noreferrer" aria-label={t('client.contact_whatsapp', 'Message on WhatsApp')} className={iconCls} style={iconSty}>
                      <i className="ti ti-brand-whatsapp" />
                    </a>
                  )}
                  {storeLinks.instagram && (
                    <a href={storeLinks.instagram} target="_blank" rel="noopener noreferrer" aria-label="Instagram" className={iconCls} style={iconSty}>
                      <i className="ti ti-brand-instagram" />
                    </a>
                  )}
                  {storeLinks.facebook && (
                    <a href={storeLinks.facebook} target="_blank" rel="noopener noreferrer" aria-label="Facebook" className={iconCls} style={iconSty}>
                      <i className="ti ti-brand-facebook" />
                    </a>
                  )}
                </div>
              )}
            </div>
          </div>
        </footer>
        );
      })()}

    </div>
  );
}
