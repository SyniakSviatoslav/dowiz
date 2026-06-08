import React, { useEffect, useState, useRef, useCallback } from 'react';
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

  const categories = data;

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
        const mockCategories: MenuCategory[] = [
          {
            id: 'cat-sushi', name: 'Sushi', sort_order: 0, products: [
              { id: 'p1', name: 'Spicy Tuna Roll', description: 'Fresh tuna, spicy mayo, scallions, sesame.', price: 320, available: false, modifier_groups: [] },
              { id: 'p2', name: 'Salmon Avocado Roll', description: 'Fresh salmon, avocado, cream cheese.', price: 380, available: true, modifier_groups: [] },
              { id: 'p3', name: 'Dragon Roll', description: 'Eel, cucumber, avocado, eel sauce.', price: 420, available: true, modifier_groups: [] },
              { id: 'p4', name: 'Shrimp Tempura Roll', description: 'Crispy shrimp, avocado, spicy mayo.', price: 390, available: true, modifier_groups: [] },
              { id: 'p7', name: 'Crunchy Roll', description: 'Tempura bits, crab, cream cheese.', price: 360, available: true, modifier_groups: [] },
            ]
          },
          {
            id: 'cat-ramen', name: 'Ramen', sort_order: 1, products: [
              { id: 'p5', name: 'Tonkotsu Ramen', description: 'Rich pork broth, chashu, soft egg, noodles.', price: 350, available: true, modifier_groups: [] },
              { id: 'p6', name: 'Spicy Miso Ramen', description: 'Miso broth, spicy ground pork, corn, scallions.', price: 330, available: true, modifier_groups: [] },
            ]
          }
        ];
        setMenu(null);
        setData(mockCategories);
        if (mockCategories[0]) setActiveTab(mockCategories[0].id);
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
    return `https://cdn.dowiz.org/${product.image_key}`;
  };

  return (
    <div className="relative min-h-screen pb-20">

      {/* Hero Section */}
      <section className="relative w-full h-[240px] flex items-end overflow-hidden" style={{ background: 'linear-gradient(160deg, var(--brand-surface-raised) 0%, var(--brand-accent) 60%, var(--brand-primary) 100%)' }}>
        <div className="absolute inset-0" style={{ background: 'linear-gradient(to top, rgba(0,0,0,0.85) 0%, rgba(0,0,0,0.4) 50%, rgba(0,0,0,0.1) 100%)' }} />
        <div className="relative z-10 w-full px-5 pb-5">
          <div className="flex items-center gap-1 text-[13px] font-medium mb-2" style={{ color: 'var(--brand-text-muted)' }}>
            <span className="inline-flex gap-0.5" style={{ color: 'var(--color-warning)' }}>
              {[1,2,3,4,5].map(i => <i key={i} className="ti ti-star-filled" style={{ fontSize: '0.8rem' }} />)}
            </span>
            <span style={{ color: 'var(--brand-text)' }}>4.8</span>
            <span>({t('client.reviews_count', '124 reviews')})</span>
          </div>
          <h1 className="text-[32px] font-bold text-white" style={{ fontFamily: 'var(--brand-font-heading)', textShadow: '0 2px 12px rgba(0,0,0,0.5)' }}>Dubin & Sushi</h1>
          <p className="text-[14px] font-medium mt-1" style={{ color: 'rgba(255,255,255,0.8)' }}>{t('client.menu_subtitle', 'Sushi & Noodles &middot; Delivery from 30 min')}</p>
        </div>
      </section>

      {/* Category Nav */}
      <nav className="sticky top-[56px] z-40 h-[48px] border-b w-full" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
        <div className="h-full overflow-x-auto hide-scrollbar flex items-center text-[14px]">
          {loading ? (
            <div className="flex gap-4 px-4 h-full items-center">
              <div className="w-16 h-4 skeleton-block" />
              <div className="w-16 h-4 skeleton-block" />
              <div className="w-16 h-4 skeleton-block" />
            </div>
          ) : (
            categories.map(cat => (
              <button 
                key={cat.id}
                onClick={() => handleScrollTo(cat.id)}
                role="tab"
                aria-selected={activeTab === cat.id}
                className="h-full flex items-center px-4 whitespace-nowrap font-medium transition-colors border-b-2"
                style={{ 
                  color: activeTab === cat.id ? 'var(--brand-primary)' : 'var(--brand-text-muted)',
                  borderColor: activeTab === cat.id ? 'var(--brand-primary)' : 'transparent',
                  fontWeight: activeTab === cat.id ? 600 : 500
                }}
              >
                {cat.name}
              </button>
            ))
          )}
        </div>
      </nav>

      {/* Menu Content */}
      <main className="max-w-7xl mx-auto pt-4">
        {loading ? (
          <div className="px-4 mt-4 grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-3">
            {[1,2,3,4,5,6].map(i => (
              <div key={i} className="rounded-[12px] overflow-hidden border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
                <div className="w-full aspect-[4/3] skeleton-block rounded-none" />
                <div className="p-3">
                  <div className="h-3 w-3/4 skeleton-block mb-3" />
                  <div className="h-2 w-full skeleton-block mb-1.5" />
                  <div className="h-2 w-4/5 skeleton-block mb-4" />
                  <div className="flex justify-between items-center pt-2">
                    <div className="h-4 w-16 skeleton-block" />
                    <div className="w-8 h-8 rounded-full skeleton-block" />
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          categories.map(category => (
            <section 
              key={category.id} 
              id={category.id} 
              ref={el => { sectionRefs.current[category.id] = el }}
              className="mb-10 scroll-mt-[120px]"
            >
              <h2 className="text-[22px] font-bold px-4 mb-4" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' }}>
                {category.name}
              </h2>
              <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-3 px-4">
                {category.products.map(product => (
                  <div key={product.id} onClick={() => handleProductClick(product)}>
                    <ProductCard product={{
                      id: product.id,
                      name: product.name,
                      description: product.description,
                      price: product.price,
                      image: getImageUrl(product) || undefined,
                      isAvailable: product.available,
                    }}                     onAdd={(e) => {
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
                ))}
              </div>
            </section>
          ))
        )}
      </main>

      {/* Product Detail Modal */}
      {detailProduct && (
        <div className="fixed inset-0 z-50 flex items-end md:items-center justify-center" style={{ background: 'rgba(0,0,0,0.6)' }} onClick={closeDetail}>
          <div 
            className="w-full md:max-w-lg max-h-[85vh] overflow-auto rounded-t-2xl md:rounded-2xl" 
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
                  <i className="ti ti-tools-kitchen-2 text-4xl opacity-40" />
                  <span className="text-sm font-medium">{detailProduct.name}</span>
                </div>
              )}
              <button 
                className="absolute top-3 right-3 w-8 h-8 rounded-full flex items-center justify-center"
                style={{ background: 'rgba(0,0,0,0.5)', color: 'white' }}
                onClick={closeDetail}
              >
                <i className="ti ti-x text-lg" />
              </button>
            </div>

            {/* Content */}
            <div className="p-5 space-y-5">
              {/* Name & Price */}
              <div className="flex items-start justify-between gap-3">
                <div className="flex-1">
                  <h2 className="text-xl font-bold" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{detailProduct.name}</h2>
                  {!detailProduct.available && (
                    <span className="inline-block mt-1 text-[11px] font-semibold px-2 py-0.5 rounded" style={{ background: 'var(--color-danger-light, rgba(239,68,68,0.1))', color: 'var(--color-danger)' }}>
                      {t('client.unavailable', 'Unavailable')}
                    </span>
                  )}
                  {detailProduct.description && (
                    <p className="text-sm mt-1.5" style={{ color: 'var(--brand-text-muted)' }}>{detailProduct.description}</p>
                  )}
                </div>
                <div className="text-xl font-black whitespace-nowrap" style={{ color: 'var(--brand-primary)' }}>
                  {(detailProduct.price + calcModifierDelta()).toLocaleString()} ALL
                </div>
              </div>

              {/* Attributes */}
              {detailProduct.attributes && typeof detailProduct.attributes === 'object' && Object.keys(detailProduct.attributes).length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                  {Object.entries(detailProduct.attributes).map(([key, val]) => (
                    <span key={key} className="text-[11px] px-2 py-0.5 rounded-full" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                      {key.replace(/_/g, ' ')}: {String(val)}
                    </span>
                  ))}
                </div>
              )}

              {/* Modifier Groups */}
              {(detailProduct.modifier_groups || []).map(group => (
                <div key={group.id}>
                  <div className="flex items-center gap-2 mb-2">
                    <span className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>{group.name}</span>
                    {group.required && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded" style={{ background: 'var(--color-danger-light, rgba(239,68,68,0.1))', color: 'var(--color-danger)' }}>
                        {t('client.required', 'Required')}
                      </span>
                    )}
                    {group.max_select > 1 && (
                      <span className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>
                        {t('client.up_to', 'Up to')} {group.max_select}
                      </span>
                    )}
                  </div>
                  <div className="space-y-1">
                    {group.modifiers.filter(m => m.available).map(mod => {
                      const selected = modifierGroupSelection[group.id] || [];
                      const isSelected = selected.includes(mod.id);
                      return (
                        <button
                          key={mod.id}
                          onClick={() => toggleModifier(group.id, mod.id, group)}
                          className={`w-full flex items-center justify-between px-3 py-2.5 rounded-lg text-sm transition-all ${
                            isSelected ? 'ring-2' : ''
                          }`}
                          style={{
                            background: isSelected ? 'var(--brand-primary-light, var(--brand-surface-raised))' : 'var(--brand-surface)',
                            borderColor: isSelected ? 'var(--brand-primary)' : 'var(--brand-border)',
                            color: 'var(--brand-text)',
                            border: '1px solid',
                          }}
                        >
                          <span>{mod.name}</span>
                          {mod.price_delta > 0 ? (
                            <span className="text-xs font-medium" style={{ color: 'var(--brand-primary)' }}>+{mod.price_delta.toLocaleString()} ALL</span>
                          ) : (
                            <span className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('client.included', 'Included')}</span>
                          )}
                        </button>
                      );
                    })}
                  </div>
                  {group.modifiers.filter(m => !m.available).length > 0 && (
                    <div className="mt-1 text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>
                      {group.modifiers.filter(m => !m.available).length} {t('client.unavailable_options', 'options unavailable')}
                    </div>
                  )}
                </div>
              ))}

              {/* Quantity + Add to Cart */}
              <div className="flex items-center gap-4 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
                <div className="flex items-center gap-3">
                  <button 
                    onClick={() => setQuantity(q => Math.max(1, q - 1))}
                    className="w-10 h-10 rounded-full flex items-center justify-center text-lg font-medium transition-colors active:scale-95"
                    style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}
                  >
                    -
                  </button>
                  <span className="text-lg font-semibold w-8 text-center" style={{ color: 'var(--brand-text)' }}>{quantity}</span>
                  <button 
                    onClick={() => setQuantity(q => q + 1)}
                    className="w-10 h-10 rounded-full flex items-center justify-center text-lg font-medium transition-colors active:scale-95"
                    style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}
                  >
                    +
                  </button>
                </div>
                <button
                  onClick={handleAddDetail}
                  disabled={!canAdd()}
                  className="flex-1 h-12 rounded-xl text-white font-semibold text-sm transition-all active:scale-[0.98] disabled:opacity-40"
                  style={{ background: detailProduct.available ? 'var(--brand-primary)' : 'var(--brand-text-muted)' }}
                >
                  {detailProduct.available
                    ? `${t('client.add_to_cart', 'Add to Cart')} · ${((detailProduct.price + calcModifierDelta()) * quantity).toLocaleString()} ${getCurrency(menu)}`
                    : t('client.unavailable', 'Unavailable')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

    </div>
  );
}
