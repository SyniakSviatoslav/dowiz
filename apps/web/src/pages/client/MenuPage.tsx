import React, { useEffect, useState, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ProductCard, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { useSharedCart } from '../../lib/CartProvider.js';

interface MenuCategory {
  id: string;
  name: string;
  items: any[];
}

export function MenuPage() {
  const { slug } = useParams<{ slug: string }>();
  const { t } = useI18n();
  const [categories, setCategories] = useState<MenuCategory[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<string>('');
  const { addItem, bounceCart } = useSharedCart();
  const navigate = useNavigate();

  // Fetch menu data
  useEffect(() => {
    async function load() {
      try {
        const data = await apiClient<any>(`/public/menu/${slug}`);
        const cats = Array.isArray(data) ? data : [];
        setCategories(cats);
        if (cats[0]) setActiveTab(cats[0].id);
        setLoading(false);
      } catch (err) {
        // Fallback mock data
        const mockCategories: MenuCategory[] = [
          {
            id: 'cat-sushi', name: 'Sushi', items: [
              { id: 'p1', name: 'Spicy Tuna Roll', description: 'Fresh tuna, spicy mayo, scallions, sesame.', price: 320, isAvailable: true, tags: ['gluten', 'dairy'] },
              { id: 'p2', name: 'Salmon Avocado Roll', description: 'Fresh salmon, avocado, cream cheese.', price: 380, isAvailable: true, tags: ['gluten', 'dairy'] },
              { id: 'p3', name: 'Dragon Roll', description: 'Eel, cucumber, avocado, eel sauce.', price: 420, isAvailable: false },
              { id: 'p4', name: 'Shrimp Tempura Roll', description: 'Crispy shrimp, avocado, spicy mayo.', price: 390, isAvailable: true },
            ]
          },
          {
            id: 'cat-ramen', name: 'Ramen', items: [
              { id: 'p5', name: 'Tonkotsu Ramen', description: 'Rich pork broth, chashu, soft egg, noodles.', price: 350, isAvailable: true },
              { id: 'p6', name: 'Spicy Miso Ramen', description: 'Miso broth, spicy ground pork, corn, scallions.', price: 330, isAvailable: true },
            ]
          }
        ];
        setCategories(mockCategories);
        if (mockCategories[0]) setActiveTab(mockCategories[0].id);
        setLoading(false);
      }
    }
    load();
  }, [slug]);

  // Intersection Observer for sticky nav
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

  const handleAdd = (e: React.MouseEvent, product: any) => {
    e.preventDefault();
    e.stopPropagation();
    addItem({ ...product, productId: product.id, quantity: 1, options: [] });
    bounceCart();
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
                {category.items.map(product => (
                  <ProductCard key={product.id} product={product} onAdd={(e) => handleAdd(e, product)} />
                ))}
              </div>
            </section>
          ))
        )}
      </main>

    </div>
  );
}
