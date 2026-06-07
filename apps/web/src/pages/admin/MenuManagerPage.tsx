import React, { useState, useEffect, useMemo } from 'react';
import { Button, Input, EmptyState, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { RecipeEditor } from './RecipeEditor.js';
import { AllergenEditor, ReadinessIndicator } from './AllergenEditor.js';

interface Ingredient {
  id: string;
  name: string;
  unit: string;
  stock: number;
  minStock: number;
}

interface Product {
  id: string;
  name: string;
  price: number;
  description?: string;
  available: boolean;
  categoryId: string;
  imageUrl?: string;
  ingredients?: string[];
  stockCount?: number;
  taste?: { spicy?: number; sweet?: number; salty?: number; sour?: number; richness?: number };
  allergenStatus?: 'unset' | 'none' | 'listed';
  allergensList?: string[];
}

interface Category {
  id: string;
  name: string;
  products?: Product[];
}

const MOCK_INGREDIENTS: Ingredient[] = [
  { id: 'i1', name: 'Salmon fillet', unit: 'kg', stock: 4.5, minStock: 2 },
  { id: 'i2', name: 'Tuna fillet', unit: 'kg', stock: 3.2, minStock: 2 },
  { id: 'i3', name: 'Sushi rice', unit: 'kg', stock: 12, minStock: 5 },
  { id: 'i4', name: 'Nori sheets', unit: 'pcs', stock: 80, minStock: 50 },
  { id: 'i5', name: 'Avocado', unit: 'pcs', stock: 15, minStock: 10 },
  { id: 'i6', name: 'Cream cheese', unit: 'kg', stock: 2.5, minStock: 1.5 },
  { id: 'i7', name: 'Shrimp', unit: 'kg', stock: 3.0, minStock: 1.5 },
  { id: 'i8', name: 'Cucumber', unit: 'pcs', stock: 20, minStock: 8 },
  { id: 'i9', name: 'Sesame seeds', unit: 'kg', stock: 1.2, minStock: 0.5 },
  { id: 'i10', name: 'Spicy mayo', unit: 'L', stock: 2.0, minStock: 0.5 },
  { id: 'i11', name: 'Eel sauce', unit: 'L', stock: 0.8, minStock: 0.5 },
  { id: 'i12', name: 'Soy sauce', unit: 'L', stock: 5.0, minStock: 2 },
  { id: 'i13', name: 'Wasabi', unit: 'kg', stock: 0.4, minStock: 0.2 },
  { id: 'i14', name: 'Pickled ginger', unit: 'kg', stock: 1.0, minStock: 0.5 },
  { id: 'i15', name: 'Tempura batter', unit: 'kg', stock: 2.0, minStock: 1 },
];

export function MenuManagerPage() {
  const [categories, setCategories] = useState<Category[]>([]);
  const [ingredients, setIngredients] = useState<Ingredient[]>(MOCK_INGREDIENTS);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [newCategoryName, setNewCategoryName] = useState('');
  const [expandedCat, setExpandedCat] = useState<string | null>(null);
  const [previewProduct, setPreviewProduct] = useState<Product | null>(null);

  // Form state
  const [showForm, setShowForm] = useState(false);
  const [editingProduct, setEditingProduct] = useState<Product | null>(null);
  const [formName, setFormName] = useState('');
  const [formPrice, setFormPrice] = useState('');
  const [formDesc, setFormDesc] = useState('');
  const [formAvailable, setFormAvailable] = useState(true);
  const [formImage, setFormImage] = useState<string | null>(null);
  const [formStock, setFormStock] = useState('');
  const [formIngredients, setFormIngredients] = useState<string[]>([]);
  const [newIngredient, setNewIngredient] = useState('');
  const [saving, setSaving] = useState(false);
  const [showIngredients, setShowIngredients] = useState(false);
  // Taste profile (5 axes ├Ч 3 levels)
  const TASTE_AXES = ['spicy', 'sweet', 'salty', 'sour', 'richness'] as const;
  const TASTE_LABELS: Record<string, string> = { spicy: 'Spicy', sweet: 'Sweet', salty: 'Salty', sour: 'Sour', richness: 'Richness' };
  const TASTE_ICONS: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };
  const [formTaste, setFormTaste] = useState<Record<string, number>>({});
  const [formAllergenStatus, setFormAllergenStatus] = useState<'unset' | 'none' | 'listed'>('unset');
  const [formAllergensList, setFormAllergensList] = useState<string[]>([]);
  const [formRecipeLines, setFormRecipeLines] = useState<Array<{supplyId: string; supplyName: string; qty: number; unit: string; kind: string; kcal: number | null; proteinG: number | null; fatG: number | null; carbsG: number | null; allergens: string[]}>>([]);

  // Filter/sort state
  const [searchQuery, setSearchQuery] = useState('');
  const [sortBy, setSortBy] = useState<'name' | 'price-asc' | 'price-desc'>('name');
  const [filterAvailable, setFilterAvailable] = useState<'all' | 'available' | 'unavailable'>('all');

  // Ingredient inventory state
  const [editIngredientId, setEditIngredientId] = useState<string | null>(null);
  const [editStock, setEditStock] = useState('');

  const fetchCategories = async () => {
    try {
      setLoading(true);
      const data = await apiClient<any>('/owner/menu/categories');
      setCategories(Array.isArray(data) ? data : []);
      setError('');
    } catch (err: any) {
      if (err.status === 404 || err.status === 403) {
        setCategories([
          { id: 'c1', name: 'Sushi Rolls' },
          { id: 'c2', name: 'Nigiri & Sashimi' },
          { id: 'c3', name: 'Hot Dishes' },
          { id: 'c4', name: 'Drinks' },
          { id: 'c5', name: 'Desserts' },
        ]);
      } else {
        setError('Failed to load menu. Add ?dev=true to the URL for mock mode.');
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { fetchCategories(); }, []);

  const openAddForm = (categoryId: string) => {
    setEditingProduct(null);
    setShowForm(true);
    setFormName('');
    setFormPrice('');
    setFormDesc('');
    setFormAvailable(true);
    setFormImage(null);
    setFormStock('');
    setFormIngredients([]);
    setFormTaste({});
    setFormAllergenStatus('unset');
    setFormAllergensList([]);
    setExpandedCat(categoryId);
  };

  const openEditForm = (product: Product) => {
    setEditingProduct(product);
    setShowForm(true);
    setFormName(product.name);
    setFormPrice(String(product.price));
    setFormDesc(product.description || '');
    setFormAvailable(product.available);
    setFormImage(product.imageUrl || null);
    setFormStock(product.stockCount != null ? String(product.stockCount) : '');
    setFormIngredients(product.ingredients || []);
    setFormTaste(product.taste || {});
    setFormAllergenStatus(product.allergenStatus || 'unset');
    setFormAllergensList(product.allergensList || []);
  };

  const closeForm = () => {
    setShowForm(false);
    setEditingProduct(null);
    setFormName(''); setFormPrice(''); setFormDesc('');
    setFormImage(null); setFormStock(''); setFormIngredients([]);
    setSaving(false);
  };

  const handleImageUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (!file.type.startsWith('image/')) { alert('Only image files (JPG, PNG, WebP)'); return; }
    if (file.size > 5 * 1024 * 1024) { alert('Max 5 MB'); return; }
    const reader = new FileReader();
    reader.onload = () => setFormImage(reader.result as string);
    reader.readAsDataURL(file);
  };

  const handleSaveProduct = async () => {
    if (!formName.trim() || !formPrice || !expandedCat) return;
    const price = parseInt(formPrice);
    if (isNaN(price) || price <= 0) return;
    const stock = formStock ? parseInt(formStock) : undefined;
    if (stock !== undefined && (isNaN(stock) || stock < 0)) return;

    setSaving(true);
    const product = {
      id: editingProduct?.id || `p_${Date.now()}`,
      name: formName, price,
      description: formDesc,
      available: formAvailable,
      imageUrl: formImage || undefined,
      ingredients: formIngredients,
      stockCount: stock,
      taste: Object.keys(formTaste).length > 0 ? formTaste : undefined,
      allergenStatus: formAllergenStatus,
      allergensList: formAllergenStatus === 'listed' ? formAllergensList : undefined,
      categoryId: expandedCat,
    };

    try {
      if (editingProduct) {
        await apiClient(`/owner/menu/products/${editingProduct.id}`, { method: 'PATCH', body: product });
      } else {
        await apiClient('/owner/menu/products', { method: 'POST', body: product });
      }
      setCategories(prev => prev.map(c => {
        if (c.id !== expandedCat) return c;
        const prods = c.products || [];
        if (editingProduct) return { ...c, products: prods.map(p => p.id === editingProduct.id ? { ...p, ...product } : p) };
        return { ...c, products: [...prods, product] };
      }));
      closeForm();
    } catch { closeForm(); }
  };

  const handleDeleteProduct = async (catId: string, productId: string) => {
    try { await apiClient(`/owner/menu/products/${productId}`, { method: 'DELETE' }); } catch {}
    setCategories(prev => prev.map(c =>
      c.id === catId ? { ...c, products: (c.products || []).filter(p => p.id !== productId) } : c
    ));
  };

  const handleToggleAvailable = async (catId: string, product: Product) => {
    const updated = { ...product, available: !product.available };
    setCategories(prev => prev.map(c => {
      if (c.id !== catId) return c;
      return { ...c, products: (c.products || []).map(p => p.id === product.id ? updated : p) };
    }));
    try { await apiClient(`/owner/menu/products/${product.id}`, { method: 'PATCH', body: { available: updated.available } }); } catch {}
  };

  const handleAddCategory = async () => {
    const name = newCategoryName.trim();
    if (!name) return;
    setCategories(prev => [...prev, { id: `c_${Date.now()}`, name, products: [] }]);
    setNewCategoryName('');
    try { await apiClient('/owner/menu/categories', { method: 'POST', body: { name } }); } catch {}
  };

  const toggleExpand = async (catId: string) => {
    if (expandedCat === catId) { setExpandedCat(null); return; }
    setExpandedCat(catId);
    const cat = categories.find(c => c.id === catId);
    if (cat && cat.products === undefined) {
      try {
        const prods = await apiClient<any>(`/owner/menu/products?category_id=${catId}`);
        setCategories(prev => prev.map(c => c.id === catId ? { ...c, products: Array.isArray(prods) ? prods : [] } : c));
      } catch {
        setCategories(prev => prev.map(c => c.id === catId ? { ...c, products: [] } : c));
      }
    }
  };

  // тФАтФА Ingredient inventory management тФАтФА
  const handleUpdateStock = (id: string, newStock: number) => {
    setIngredients(prev => prev.map(i => i.id === id ? { ...i, stock: newStock } : i));
    setEditIngredientId(null);
  };

  // тФАтФА Filtered/sorted products тФАтФА
  const getAllProducts = (catId: string): Product[] => {
    const cat = categories.find(c => c.id === catId);
    if (!cat?.products) return [];
    let result = [...cat.products];
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(p => p.name.toLowerCase().includes(q) || p.description?.toLowerCase().includes(q));
    }
    if (filterAvailable === 'available') result = result.filter(p => p.available);
    if (filterAvailable === 'unavailable') result = result.filter(p => !p.available);
    result.sort((a, b) => {
      if (sortBy === 'name') return a.name.localeCompare(b.name);
      if (sortBy === 'price-asc') return a.price - b.price;
      return b.price - a.price;
    });
    return result;
  };

  // тФАтФА Stock summary тФАтФА
  const totalDishes = useMemo(() => {
    let count = 0;
    categories.forEach(c => {
      if (c.products) count += c.products.filter(p => p.stockCount != null && p.stockCount > 0).length;
    });
    return count;
  }, [categories]);

  const lowStockIngredients = ingredients.filter(i => i.stock <= i.minStock);

  return (
    <div className="p-4 md:p-6 max-w-5xl mx-auto space-y-5">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Menu Manager</h2>
          <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>
            {categories.length} categories ┬╖ {totalDishes} dishes in stock
            {lowStockIngredients.length > 0 && (
              <span className="ml-2 px-1.5 py-0.5 rounded text-xs" style={{ background: 'rgba(217,119,6,0.15)', color: 'var(--color-warning)' }}>
                {lowStockIngredients.length} low stock
              </span>
            )}
          </p>
        </div>
      </div>

      {error && (
        <div className="p-3 rounded-lg text-sm flex items-center justify-between" style={{ background: 'rgba(220,38,38,0.1)', color: 'var(--color-danger)' }}>
          {error}
          <button onClick={fetchCategories} className="underline ml-3 shrink-0">Retry</button>
        </div>
      )}

      {/* Toolbar: search, filter, sort, add category */}
      <div className="flex flex-col sm:flex-row gap-2">
        <div className="relative flex-1">
          <i className="ti ti-search absolute left-3 top-1/2 -translate-y-1/2 text-sm" style={{ color: 'var(--brand-text-muted)' }} />
          <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)} placeholder="Search products..."
            className="w-full pl-9 pr-4 py-2 rounded-lg border text-sm outline-none"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
        </div>
        <select value={sortBy} onChange={e => setSortBy(e.target.value as any)}
          className="px-3 py-2 rounded-lg border text-sm outline-none"
          style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
          <option value="name">Name A-Z</option>
          <option value="price-asc">Price тЖС</option>
          <option value="price-desc">Price тЖУ</option>
        </select>
        <select value={filterAvailable} onChange={e => setFilterAvailable(e.target.value as any)}
          className="px-3 py-2 rounded-lg border text-sm outline-none"
          style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
          <option value="all">All items</option>
          <option value="available">Available</option>
          <option value="unavailable">Stop-listed</option>
        </select>
        <Button onClick={() => setShowIngredients(!showIngredients)} variant="ghost" size="sm">
          <i className="ti ti-flask" /> {showIngredients ? 'Hide' : 'Ingredients'}
        </Button>
      </div>

      {/* Ingredient Inventory Panel */}
      {showIngredients && (
        <div className="rounded-xl border p-4 space-y-3 slide-in-up" style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface)' }}>
          <div className="flex items-center justify-between">
            <h3 className="font-semibold text-sm">Ingredient Inventory</h3>
            <span className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{ingredients.length} items</span>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
            {ingredients.map(ing => {
              const low = ing.stock <= ing.minStock;
              const pct = Math.min(100, (ing.stock / Math.max(ing.minStock * 2, 1)) * 100);
              return (
                <div key={ing.id} className="flex items-center gap-3 p-3 rounded-lg border" style={{ borderColor: low ? 'rgba(217,119,6,0.3)' : 'var(--brand-border)', background: low ? 'rgba(217,119,6,0.05)' : 'var(--brand-surface-raised)' }}>
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium truncate">{ing.name}</div>
                    <div className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
                      {editIngredientId === ing.id ? (
                        <span className="flex items-center gap-1">
                          <input value={editStock} onChange={e => setEditStock(e.target.value)} type="number"
                            className="w-16 px-1 py-0.5 text-xs rounded border" autoFocus
                            style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                          <span>{ing.unit}</span>
                          <button onClick={() => handleUpdateStock(ing.id, Number(editStock) || ing.stock)} className="text-[var(--color-success)]">тЬУ</button>
                        </span>
                      ) : (
                        <span onClick={() => { setEditIngredientId(ing.id); setEditStock(String(ing.stock)); }} className="cursor-pointer hover:underline">
                          {ing.stock} {ing.unit} / min {ing.minStock} {ing.unit}
                        </span>
                      )}
                    </div>
                  </div>
                  {low && <span className="text-[10px] px-1.5 py-0.5 rounded-full font-medium" style={{ background: 'var(--color-warning)', color: '#fff' }}>LOW</span>}
                  <div className="w-12 h-1.5 rounded-full overflow-hidden" style={{ background: 'var(--brand-border)' }}>
                    <div className="h-full rounded-full transition-all" style={{ width: `${pct}%`, background: low ? 'var(--color-warning)' : 'var(--color-success)' }} />
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Add Category */}
      <div className="flex gap-2">
        <Input placeholder="New category..." value={newCategoryName} onChange={e => setNewCategoryName(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleAddCategory()} />
        <Button onClick={handleAddCategory}>Add Category</Button>
      </div>

      {/* Categories & Products */}
      {loading ? (
        <div className="space-y-3">{ [1,2,3].map(i => <div key={i} className="h-12 shimmer rounded-lg" />) }</div>
      ) : categories.length === 0 ? (
        <EmptyState title="No categories" description="Add a category above to start." />
      ) : (
        <div className="space-y-2">
          {categories.map(cat => {
            const products = getAllProducts(cat.id);
            return (
              <div key={cat.id} className="rounded-xl border overflow-hidden" style={{ borderColor: 'var(--brand-border)' }}>
                <button onClick={() => toggleExpand(cat.id)} className="w-full p-4 flex items-center justify-between hover:bg-[var(--brand-surface)] transition-colors text-left">
                  <div className="flex items-center gap-3">
                    <i className={`ti text-sm transition-transform ${expandedCat === cat.id ? 'ti-chevron-down' : 'ti-chevron-right'}`} style={{ color: 'var(--brand-text-muted)' }} />
                    <span className="font-medium">{cat.name}</span>
                    <span className="text-xs px-2 py-0.5 rounded-full" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                      {cat.products?.length ?? '...'}
                    </span>
                  </div>
                  <Button size="sm" variant="ghost" onClick={(e) => { e.stopPropagation(); openAddForm(cat.id); }}>
                    <i className="ti ti-plus" /> Add
                  </Button>
                </button>

                {expandedCat === cat.id && (
                  <div className="border-t" style={{ borderColor: 'var(--brand-border)' }}>
                    {cat.products === undefined ? (
                      <div className="p-4 text-center text-sm" style={{ color: 'var(--brand-text-muted)' }}>Loading...</div>
                    ) : products.length === 0 ? (
                      <div className="p-4 text-center text-sm" style={{ color: 'var(--brand-text-muted)' }}>
                        {searchQuery ? 'No matching products.' : 'No items yet. Click Add to create one.'}
                      </div>
                    ) : (
                      products.map((product, idx) => (
                        <div key={product.id} className="flex items-center gap-3 p-3 border-t hover:bg-[var(--brand-surface)] transition-colors slide-in-up"
                          style={{ borderColor: 'var(--brand-border)', animationDelay: `${idx * 20}ms` }}>
                          {/* Image or placeholder */}
                          <div className="w-12 h-12 rounded-lg overflow-hidden shrink-0 cursor-pointer"
                            style={{ background: 'var(--brand-surface-raised)' }}
                            onClick={() => setPreviewProduct(product)}>
                            {product.imageUrl
                              ? <img src={product.imageUrl} alt="" className="w-full h-full object-cover" />
                              : <div className="w-full h-full flex items-center justify-center"><i className="ti ti-photo text-lg" style={{ color: 'var(--brand-border)' }} /></div>
                            }
                          </div>

                          {/* Availability toggle */}
                          <button onClick={() => handleToggleAvailable(cat.id, product)}
                            className={`w-5 h-5 rounded border-2 flex items-center justify-center shrink-0 transition-colors ${product.available ? 'bg-[var(--brand-primary)] border-[var(--brand-primary)]' : 'border-[var(--brand-border)]'}`}
                            title={product.available ? 'Available тАФ click to stop-list' : 'Stop-listed тАФ click to enable'}>
                            {product.available && <i className="ti ti-check text-[10px] text-white" />}
                          </button>

                          {/* Info */}
                          <div className="flex-1 min-w-0 cursor-pointer" onClick={() => setPreviewProduct(product)}>
                            <div className="font-medium text-sm truncate">{product.name}</div>
                            <div className="flex items-center gap-3 text-xs" style={{ color: 'var(--brand-text-muted)' }}>
                              {product.description && <span className="truncate">{product.description}</span>}
                              {product.ingredients && product.ingredients.length > 0 && (
                                <span className="text-[10px] px-1.5 py-0.5 rounded" style={{ background: 'var(--brand-surface-raised)' }}>
                                  {product.ingredients.length} ingredients
                                </span>
                              )}
                              {product.stockCount != null && (
                                <span className={product.stockCount > 0 ? '' : 'text-[var(--color-danger)]'}>
                                  {product.stockCount > 0 ? `${product.stockCount} in stock` : 'Out of stock'}
                                </span>
                              )}
                            </div>
                          </div>

                          {/* Price */}
                          <span className="text-sm font-semibold shrink-0" style={{ color: 'var(--brand-primary)' }}>{product.price} ALL</span>

                          {/* Actions */}
                          <button onClick={() => openEditForm(product)} className="w-8 h-8 flex items-center justify-center rounded-md hover:bg-[var(--brand-surface-raised)]" title="Edit">
                            <i className="ti ti-pencil text-sm" style={{ color: 'var(--brand-text-muted)' }} />
                          </button>
                          <button onClick={() => handleDeleteProduct(cat.id, product.id)} className="w-8 h-8 flex items-center justify-center rounded-md hover:bg-red-500/10" title="Delete">
                            <i className="ti ti-trash text-sm" style={{ color: 'var(--color-danger)' }} />
                          </button>
                        </div>
                      ))
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* Product Preview Card */}
      {previewProduct && (
        <div className="fixed inset-0 z-50 flex items-center justify-center fade-in" onClick={() => setPreviewProduct(null)}>
          <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" />
          <div className="relative w-[320px] bg-[var(--brand-surface)] rounded-2xl overflow-hidden shadow-2xl z-10 scale-in" onClick={e => e.stopPropagation()}>
            <div className="aspect-[4/3] relative" style={{ background: 'var(--brand-surface-raised)' }}>
              {previewProduct.imageUrl
                ? <img src={previewProduct.imageUrl} alt="" className="w-full h-full object-cover" />
                : <div className="w-full h-full flex items-center justify-center"><i className="ti ti-photo text-4xl" style={{ color: 'var(--brand-border)' }} /></div>
              }
              <button onClick={() => setPreviewProduct(null)} className="absolute top-3 right-3 w-8 h-8 rounded-full bg-black/40 text-white flex items-center justify-center">
                <i className="ti ti-x" />
              </button>
              {!previewProduct.available && (
                <div className="absolute inset-0 bg-black/40 flex items-center justify-center">
                  <span className="px-3 py-1 rounded-lg text-sm font-medium bg-white/90 text-black">Stop-listed</span>
                </div>
              )}
            </div>
            <div className="p-4 space-y-3">
              <div className="flex items-start justify-between">
                <div>
                  <h3 className="text-lg font-bold">{previewProduct.name}</h3>
                  {previewProduct.description && <p className="text-sm mt-1" style={{ color: 'var(--brand-text-muted)' }}>{previewProduct.description}</p>}
                </div>
                <span className="text-xl font-black shrink-0" style={{ color: 'var(--brand-primary)' }}>{previewProduct.price} ALL</span>
              </div>

              {previewProduct.ingredients && previewProduct.ingredients.length > 0 && (
                <div>
                  <div className="text-xs font-medium mb-1.5" style={{ color: 'var(--brand-text-muted)' }}>Ingredients</div>
                  <div className="flex flex-wrap gap-1">
                    {previewProduct.ingredients.map(ing => {
                      const inv = ingredients.find(i => i.name.toLowerCase() === ing.toLowerCase());
                      const low = inv && inv.stock <= inv.minStock;
                      return (
                        <span key={ing} className={`text-[10px] px-2 py-0.5 rounded-full ${low ? 'bg-[var(--color-warning)]/10 text-[var(--color-warning)]' : ''}`}
                          style={!low ? { background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' } : undefined}
                          title={inv ? `${inv.stock} ${inv.unit} in stock (min ${inv.minStock})` : ing}>
                          {ing} {low && 'тЪа'}
                        </span>
                      );
                    })}
                  </div>
                </div>
              )}

              {previewProduct.stockCount != null && (
                <div className="flex items-center gap-2">
                  <span className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>In stock:</span>
                  <span className={`text-sm font-bold ${previewProduct.stockCount === 0 ? 'text-[var(--color-danger)]' : 'text-[var(--color-success)]'}`}>
                    {previewProduct.stockCount === 0 ? 'Out of stock' : `${previewProduct.stockCount} available today`}
                  </span>
                </div>
              )}

              <div className="flex gap-2 pt-2">
                <Button onClick={() => { setPreviewProduct(null); openEditForm(previewProduct); }} size="sm" className="flex-1">
                  <i className="ti ti-pencil" /> Edit
                </Button>
                <Button onClick={() => setPreviewProduct(null)} variant="ghost" size="sm">Close</Button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Add/Edit Form Modal */}
      {showForm && (
        <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center fade-in" onClick={closeForm}>
          <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" />
          <div className="relative w-full max-w-md bg-[var(--brand-surface)] rounded-t-2xl sm:rounded-2xl p-6 space-y-4 z-10 slide-in-up max-h-[90vh] overflow-auto" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{editingProduct ? 'Edit Item' : 'Add Item'}</h3>
              <button onClick={closeForm} className="w-8 h-8 flex items-center justify-center rounded-md hover:bg-[var(--brand-surface-raised)]">
                <i className="ti ti-x" style={{ color: 'var(--brand-text-muted)' }} />
              </button>
            </div>

            {/* Photo */}
            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>Photo</label>
              <div className="flex items-start gap-3">
                <label className="flex flex-col items-center justify-center w-20 h-20 rounded-lg border-2 border-dashed cursor-pointer hover:border-[var(--brand-primary)] transition-colors shrink-0"
                  style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface-raised)' }}>
                  {formImage ? <img src={formImage} alt="" className="w-full h-full object-cover rounded-lg" />
                    : <div className="text-center"><i className="ti ti-camera text-lg" style={{ color: 'var(--brand-text-muted)' }} /><span className="text-[9px] block">JPG/PNG</span></div>}
                  <input type="file" accept="image/jpeg,image/png,image/webp" onChange={handleImageUpload} className="hidden" />
                </label>
                <div className="text-xs space-y-1" style={{ color: 'var(--brand-text-muted)' }}>
                  <p>4:3 ratio, max 5 MB</p>
                  <p>JPG, PNG, WebP</p>
                  {formImage && <button onClick={() => setFormImage(null)} className="text-[var(--color-danger)] underline">Remove</button>}
                </div>
              </div>
            </div>

            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>Name *</label>
              <Input value={formName} onChange={e => setFormName(e.target.value)} placeholder="e.g. Margherita Pizza" autoFocus />
            </div>

            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>Price (ALL) *</label>
              <Input value={formPrice} onChange={e => setFormPrice(e.target.value)} placeholder="600" type="number" />
            </div>

            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>Description</label>
              <Input value={formDesc} onChange={e => setFormDesc(e.target.value)} placeholder="Short description..." />
            </div>

            {/* BOM Recipe */}
            <RecipeEditor lines={formRecipeLines} onChange={setFormRecipeLines} />

            {/* Allergen Attestation (replaces inline) */}
            <AllergenEditor
              status={formAllergenStatus}
              declaredAllergens={formAllergensList}
              bomAllergens={[...new Set(formRecipeLines.flatMap(l => l.allergens))]}
              onStatusChange={setFormAllergenStatus}
              onAllergensChange={setFormAllergensList}
            />

            {/* Stock count */}
            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>Available today (pieces)</label>
              <Input value={formStock} onChange={e => setFormStock(e.target.value)} placeholder="Leave empty for unlimited" type="number" />
            </div>

            {/* Ingredients */}
            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>Ingredients</label>
              <div className="flex flex-wrap gap-1 mb-2">
                {formIngredients.map((ing, i) => (
                  <span key={i} className="text-xs px-2 py-0.5 rounded-full flex items-center gap-1" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text)' }}>
                    {ing}
                    <button onClick={() => setFormIngredients(prev => prev.filter((_, j) => j !== i))} className="hover:text-[var(--color-danger)]">├Ч</button>
                  </span>
                ))}
              </div>
              <div className="flex gap-1">
                <select value={newIngredient} onChange={e => setNewIngredient(e.target.value)}
                  className="flex-1 px-2 py-1.5 text-xs rounded-lg border outline-none"
                  style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
                  <option value="">Select from inventory...</option>
                  {ingredients.filter(i => !formIngredients.includes(i.name)).map(i => (
                    <option key={i.id} value={i.name}>{i.name} ({i.stock} {i.unit})</option>
                  ))}
                </select>
                <Button size="sm" onClick={() => {
                  if (newIngredient && !formIngredients.includes(newIngredient)) {
                    setFormIngredients(prev => [...prev, newIngredient]);
                    setNewIngredient('');
                  }
                }}>+</Button>
              </div>
            </div>

            {/* Taste Profile */}
            <div>
              <label className="text-xs font-medium mb-2 block" style={{ color: 'var(--brand-text-muted)' }}>Taste Profile <span className="opacity-50">(optional)</span></label>
              <div className="space-y-1.5">
                {TASTE_AXES.map(axis => (
                  <div key={axis} className="flex items-center gap-2">
                    <span className="text-xs w-20 shrink-0 inline-flex items-center gap-1" style={{ color: 'var(--brand-text-muted)' }}>
                      <i className={TASTE_ICONS[axis] || 'ti ti-circle'} style={{ fontSize: '0.65rem' }} />
                      {TASTE_LABELS[axis]}
                    </span>
                    <div className="flex gap-0.5 flex-1">
                      {[1, 2, 3].map(level => {
                        const active = formTaste[axis] === level;
                        return (
                          <button
                            key={level}
                            type="button"
                            onClick={() => setFormTaste(prev => prev[axis] === level ? Object.fromEntries(Object.entries(prev).filter(([k]) => k !== axis)) : { ...prev, [axis]: level })}
                            className={`flex-1 h-6 rounded text-[10px] font-medium transition-all ${
                              active ? 'text-white scale-105' : 'hover:bg-[var(--brand-surface-raised)]'
                            }`}
                            style={{
                              background: active ? `hsl(${axis === 'spicy' ? 10 : axis === 'sweet' ? 35 : axis === 'salty' ? 200 : axis === 'sour' ? 70 : 30}, ${level * 25}%, ${60 - level * 5}%)` : 'var(--brand-surface-raised)',
                              color: active ? '#fff' : 'var(--brand-text-muted)',
                            }}
                            title={`${TASTE_LABELS[axis] || axis}: ${level === 1 ? 'Low' : level === 2 ? 'Medium' : 'High'}`}
                          >
                            {level === 1 ? 'Low' : level === 2 ? 'Med' : 'High'}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Readiness indicator */}
            <ReadinessIndicator checks={[
              { label: 'Name set', pass: !!formName.trim() },
              { label: 'Price set', pass: !!formPrice && parseInt(formPrice) > 0 },
              { label: 'Allergens declared', pass: formAllergenStatus !== 'unset' },
            ]} />

            <label className="flex items-center gap-3 cursor-pointer">
              <input type="checkbox" checked={formAvailable} onChange={e => setFormAvailable(e.target.checked)} className="w-4 h-4 rounded accent-[var(--brand-primary)]" />
              <span className="text-sm">Available for order</span>
            </label>

            <div className="flex gap-3 pt-2">
              <Button onClick={closeForm} variant="ghost" className="flex-1">Cancel</Button>
              <Button onClick={handleSaveProduct} isLoading={saving} disabled={!formName.trim() || !formPrice} className="flex-1">
                {editingProduct ? 'Save Changes' : 'Add Item'}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
