import React, { useState, useEffect, useMemo } from 'react';
import { Button, Input, EmptyState, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { RecipeEditor } from './RecipeEditor.js';


interface Product {
  id: string;
  name: string;
  price: number;
  description?: string;
  available: boolean;
  categoryId: string;
  imageUrl?: string;

  stockCount?: number;
  taste?: { spicy?: number; sweet?: number; salty?: number; sour?: number; richness?: number };
}

interface Category {
  id: string;
  name: string;
  product_count?: number;
  products?: Product[];
}



export function MenuManagerPage() {
  const { t } = useI18n();
  const [categories, setCategories] = useState<Category[]>([]);
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

  const [saving, setSaving] = useState(false);

  // Import state
  const [showImport, setShowImport] = useState(false);
  const [importFile, setImportFile] = useState<File | null>(null);
  const [importDragOver, setImportDragOver] = useState(false);
  const [importLoading, setImportLoading] = useState(false);
  const [importStep, setImportStep] = useState<'upload' | 'preview' | 'done'>('upload');
  const [importSessionId, setImportSessionId] = useState<string | null>(null);
  const [importPreview, setImportPreview] = useState<any>(null);
  const [importMode, setImportMode] = useState<'merge' | 'add_only' | 'replace'>('merge');
  const [importError, setImportError] = useState('');
  const [importResult, setImportResult] = useState<any>(null);

  // Taste profile (5 axes ├Ч 3 levels)
  const TASTE_AXES = ['spicy', 'sweet', 'salty', 'sour', 'richness'] as const;
  const TASTE_LABELS: Record<string, string> = { spicy: 'Spicy', sweet: 'Sweet', salty: 'Salty', sour: 'Sour', richness: 'Richness' };
  const TASTE_ICONS: Record<string, string> = { spicy: 'ti ti-pepper', sweet: 'ti ti-candy', salty: 'ti ti-salt', sour: 'ti ti-lemon-2', richness: 'ti ti-flame' };
  const [formTaste, setFormTaste] = useState<Record<string, number>>({});
  const [formRecipeLines, setFormRecipeLines] = useState<Array<{supplyId: string; supplyName: string; qty: number; unit: string; kind: string; kcal: number | null; proteinG: number | null; fatG: number | null; carbsG: number | null; allergens: string[]}>>([]);

  // Filter/sort state
  const [searchQuery, setSearchQuery] = useState('');
  const [sortBy, setSortBy] = useState<'name' | 'price-asc' | 'price-desc'>('name');
  const [filterAvailable, setFilterAvailable] = useState<'all' | 'available' | 'unavailable'>('all');


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

    setFormTaste({});
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

    setFormTaste(product.taste || {});
  };

  const closeForm = () => {
    setShowForm(false);
    setEditingProduct(null);
    setFormName(''); setFormPrice(''); setFormDesc('');
    setFormImage(null); setFormStock('');
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
      name: formName, price,
      description: formDesc,
      available: formAvailable,
      imageUrl: formImage || undefined,
      categoryId: expandedCat,
    };

    try {
      if (editingProduct) {
        await apiClient(`/owner/menu/products/${editingProduct.id}`, { method: 'PATCH', body: product });
      } else {
        await apiClient('/owner/menu/products', { method: 'POST', body: product });
      }
      closeForm();
      await fetchCategories();
      // Also expand the category and refetch products
      const prods = await apiClient<any>(`/owner/menu/products?category_id=${expandedCat}`);
      setCategories(prev => prev.map(c => c.id === expandedCat ? { ...c, products: Array.isArray(prods) ? prods : [] } : c));
    } catch {
      alert(t('common.error_save', 'Failed to save product.'));
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteProduct = async (catId: string, productId: string) => {
    try { 
      await apiClient(`/owner/menu/products/${productId}`, { method: 'DELETE' }); 
      setCategories(prev => prev.map(c =>
        c.id === catId ? { ...c, products: (c.products || []).filter(p => p.id !== productId) } : c
      ));
    } catch {
      alert(t('common.error_delete', 'Failed to delete.'));
    }
  };

  const handleToggleAvailable = async (catId: string, product: Product) => {
    const updated = { ...product, available: !product.available };
    setCategories(prev => prev.map(c => {
      if (c.id !== catId) return c;
      return { ...c, products: (c.products || []).map(p => p.id === product.id ? updated : p) };
    }));
    try { await apiClient(`/owner/menu/products/${product.id}`, { method: 'PATCH', body: { available: updated.available } }); } catch {
      console.debug('[MenuManager] failed to toggle product availability');
    }
  };

  const handleAddCategory = async () => {
    const name = newCategoryName.trim();
    if (!name) return;
    setNewCategoryName('');
    try { 
      await apiClient('/owner/menu/categories', { method: 'POST', body: { name } }); 
      await fetchCategories();
    } catch {
      alert(t('common.error_save', 'Failed to save category.'));
    }
  };

  const MAX_IMPORT_SIZE = 10 * 1024 * 1024; // 10MB

  // ── PDF Menu Import ──
  const handleImportUpload = async () => {
    if (!importFile) return;
    if (importFile.size > MAX_IMPORT_SIZE) {
      setImportError(`File too large (max ${MAX_IMPORT_SIZE / 1024 / 1024}MB).`);
      setImportLoading(false);
      return;
    }
    setImportLoading(true);
    setImportError('');
    try {
      const formData = new FormData();
      formData.append('file', importFile);
      formData.append('mode', importMode);
      const res = await apiClient<any>('/owner/menu/import/preview', { method: 'POST', body: formData, timeout: 120000 });
      setImportSessionId(res.import_session_id);
      setImportPreview(res);
      setImportStep('preview');
    } catch (err: any) {
      setImportError(err.message || 'Failed to analyze menu. Make sure the file is a PDF or image.');
    } finally {
      setImportLoading(false);
    }
  };

  const handleImportCommit = async () => {
    if (!importSessionId) return;
    setImportLoading(true);
    setImportError('');
    try {
      const res = await apiClient<any>('/owner/menu/import/commit', {
        method: 'POST',
        body: { import_session_id: importSessionId, force: true }
      });
      setImportResult(res);
      setImportStep('done');
      await fetchCategories();
    } catch (err: any) {
      setImportError(err.message || 'Failed to import menu.');
    } finally {
      setImportLoading(false);
    }
  };

  const resetImport = () => {
    setShowImport(false);
    setImportFile(null);
    setImportStep('upload');
    setImportSessionId(null);
    setImportPreview(null);
    setImportMode('merge');
    setImportError('');
    setImportResult(null);
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


  return (
    <div className="p-4 md:p-6 max-w-5xl mx-auto space-y-5">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.menu_manager', 'Menu Manager')}</h2>
          <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>
            {categories.length} {t('admin.categories', 'categories')} ┬╖ {totalDishes} {t('admin.dishes_in_stock', 'dishes in stock')}
          </p>
        </div>
        <Button onClick={() => setShowImport(true)} variant="ghost" size="sm">
          <i className="ti ti-file-import" /> {t('admin.import_pdf', 'Import PDF')}
        </Button>
      </div>

      {error && (
        <div className="p-3 rounded-lg text-sm flex items-center justify-between" style={{ background: 'var(--color-danger-light)', color: 'var(--color-danger)' }}>
          {error}
          <button onClick={fetchCategories} className="underline ml-3 shrink-0">{t('common.retry', 'Retry')}</button>
        </div>
      )}

      {/* Toolbar: search, filter, sort, add category */}
      <div className="flex flex-col sm:flex-row gap-2">
        <div className="relative flex-1">
          <i className="ti ti-search absolute left-3 top-1/2 -translate-y-1/2 text-sm" style={{ color: 'var(--brand-text-muted)' }} />
          <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)} placeholder={t('admin.search_products', 'Search products...')}
            className="w-full pl-9 pr-4 py-2 rounded-lg border text-sm outline-none"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
        </div>
        <select value={sortBy} onChange={e => setSortBy(e.target.value as any)}
          className="px-3 py-2 rounded-lg border text-sm outline-none"
          style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
          <option value="name">{t('admin.name_az', 'Name A-Z')}</option>
          <option value="price-asc">{t('admin.price_asc', 'Price тЖС')}</option>
          <option value="price-desc">{t('admin.price_desc', 'Price тЖУ')}</option>
        </select>
        <select value={filterAvailable} onChange={e => setFilterAvailable(e.target.value as any)}
          className="px-3 py-2 rounded-lg border text-sm outline-none"
          style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
          <option value="all">{t('admin.all_items', 'All items')}</option>
          <option value="available">{t('menu.available', 'Available')}</option>
          <option value="unavailable">{t('menu.stop_listed', 'Stop-listed')}</option>
        </select>
      </div>

      {/* Add Category */}
      <div className="flex gap-2">
        <Input placeholder={t('admin.new_category', 'New category...')} value={newCategoryName} onChange={e => setNewCategoryName(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleAddCategory()} />
        <Button onClick={handleAddCategory}>{t('admin.add_category', 'Add Category')}</Button>
      </div>

      {/* Categories & Products */}
      {loading ? (
        <div className="space-y-3">{ [1,2,3].map(i => <div key={i} className="h-12 shimmer rounded-lg" />) }</div>
      ) : categories.length === 0 ? (
        <EmptyState title={t('admin.no_categories', 'No categories')} description={t('admin.add_category_desc', 'Add a category above to start.')} />
      ) : (
        <div className="rounded-xl border overflow-hidden" style={{ borderColor: 'var(--brand-border)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr style={{ background: 'var(--brand-surface)' }}>
                <th className="text-left p-3 font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.product', 'Product')}</th>
                <th className="text-left p-3 font-medium hidden sm:table-cell" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.category', 'Category')}</th>
                <th className="text-right p-3 font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.price_all', 'Price (ALL)')}</th>
                <th className="text-center p-3 font-medium hidden md:table-cell" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.stock', 'Stock')}</th>
                <th className="text-center p-3 font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('menu.available', 'Available')}</th>
                <th className="p-3" />
              </tr>
            </thead>
            <tbody>
              {categories.map(cat => {
                const products = getAllProducts(cat.id);
                if (products.length === 0 && searchQuery) return null;
                return (
                  <React.Fragment key={cat.id}>
                    {/* Category header row */}
                    <tr style={{ background: 'var(--brand-surface-raised)' }}>
                      <td colSpan={6} className="px-3 py-2">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <i className={`ti text-sm cursor-pointer transition-transform ${expandedCat === cat.id ? 'ti-chevron-down' : 'ti-chevron-right'}`}
                              style={{ color: 'var(--brand-text-muted)' }}
                              onClick={() => toggleExpand(cat.id)} />
                            <span className="font-semibold text-sm" style={{ color: 'var(--brand-text)' }}>{cat.name}</span>
                            <span className="text-[10px] px-1.5 py-0.5 rounded-full font-medium" style={{ background: 'var(--brand-surface)', color: 'var(--brand-text-muted)' }}>
                              {cat.product_count ?? cat.products?.length ?? 0}
                            </span>
                          </div>
                          <Button size="sm" variant="ghost" onClick={() => openAddForm(cat.id)}>
                            <i className="ti ti-plus" /> {t('common.add', 'Add')}
                          </Button>
                        </div>
                      </td>
                    </tr>
                    {/* Product rows */}
                    {(expandedCat === cat.id || searchQuery) && (
                      cat.products === undefined ? (
                        <tr><td colSpan={6} className="p-4 text-center text-sm" style={{ color: 'var(--brand-text-muted)' }}>{t('common.loading', 'Loading...')}</td></tr>
                      ) : products.length === 0 ? (
                        <tr><td colSpan={6} className="p-4 text-center text-sm" style={{ color: 'var(--brand-text-muted)' }}>
                          {searchQuery ? t('admin.no_matching_products', 'No matching products.') : t('admin.no_items_yet', 'No items yet. Click Add to create one.')}
                        </td></tr>
                      ) : products.map((product, idx) => (
                        <tr key={product.id}
                          className="border-t transition-colors hover:bg-[var(--brand-surface-raised)] cursor-pointer"
                          style={{ borderColor: 'var(--brand-border)', animationDelay: `${idx * 30}ms` }}
                          onClick={() => setPreviewProduct(product)}>
                          <td className="p-3">
                            <div className="flex items-center gap-3">
                              <div className="w-10 h-10 rounded-lg overflow-hidden shrink-0 flex items-center justify-center"
                                style={{ background: 'var(--brand-primary-light)' }}>
                                {product.imageUrl
                                  ? <img src={product.imageUrl} alt="" className="w-full h-full object-cover" />
                                  : <i className="ti ti-photo" style={{ color: 'var(--brand-primary)' }} />
                                }
                              </div>
                              <div className="min-w-0">
                                <div className="flex items-center gap-2">
                                  <span className="font-medium truncate" style={{ color: 'var(--brand-text)' }}>{product.name}</span>
                                  <span className="text-[10px] px-1.5 py-0.5 rounded font-mono" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                                    PRD
                                  </span>
                                </div>
                                {product.description && <div className="text-[11px] truncate" style={{ color: 'var(--brand-text-muted)' }}>{product.description}</div>}
                              </div>
                            </div>
                          </td>
                          <td className="p-3 hidden sm:table-cell" style={{ color: 'var(--brand-text-muted)' }}>
                            {cat.name}
                          </td>
                          <td className="p-3 text-right font-semibold" style={{ color: 'var(--brand-primary)' }}>
                            {product.price} ALL
                          </td>
                          <td className="p-3 text-center hidden md:table-cell">
                            {product.stockCount != null ? (
                              <span className={`text-xs font-medium px-2 py-0.5 rounded-full ${product.stockCount > 0 ? 'text-[var(--color-success)]' : 'text-[var(--color-danger)]'}`}
                                style={{ background: product.stockCount > 0 ? 'var(--color-success-light)' : 'var(--color-danger-light)' }}>
                                {product.stockCount}
                              </span>
                            ) : (
                              <span className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>—</span>
                            )}
                          </td>
                          <td className="p-3 text-center">
                            <button onClick={(e) => { e.stopPropagation(); handleToggleAvailable(cat.id, product); }}
                              className={`w-5 h-5 rounded border-2 flex items-center justify-center mx-auto transition-colors ${product.available ? 'bg-[var(--brand-primary)] border-[var(--brand-primary)]' : 'border-[var(--brand-border)]'}`}
                              title={product.available ? t('menu.available', 'Available') : t('menu.stop_listed', 'Stop-listed')}>
                              {product.available && <i className="ti ti-check text-[10px] text-white" />}
                            </button>
                          </td>
                          <td className="p-3 text-right">
                            <div className="flex items-center justify-end gap-1">
                              <button onClick={(e) => { e.stopPropagation(); openEditForm(product); }}
                                className="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-[var(--brand-surface)] transition-colors"
                                title={t('common.edit', 'Edit')}>
                                <i className="ti ti-edit" style={{ fontSize: '0.85rem', color: 'var(--brand-text-muted)' }} />
                              </button>
                              <button onClick={(e) => { e.stopPropagation(); handleDeleteProduct(cat.id, product.id); }}
                                className="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-[var(--color-danger-light)] transition-colors"
                                title={t('common.delete', 'Delete')}>
                                <i className="ti ti-trash" style={{ fontSize: '0.85rem', color: 'var(--brand-text-muted)' }} />
                              </button>
                            </div>
                          </td>
                        </tr>
                      ))
                    )}
                  </React.Fragment>
                );
              })}
            </tbody>
          </table>
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
                  <span className="px-3 py-1 rounded-lg text-sm font-medium bg-white/90 text-black">{t('menu.stop_listed', 'Stop-listed')}</span>
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



              {previewProduct.stockCount != null && (
                <div className="flex items-center gap-2">
                  <span className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.in_stock_label', 'In stock:')}</span>
                  <span className={`text-sm font-bold ${previewProduct.stockCount === 0 ? 'text-[var(--color-danger)]' : 'text-[var(--color-success)]'}`}>
                    {previewProduct.stockCount === 0 ? t('menu.out_of_stock', 'Out of stock') : `${previewProduct.stockCount} ${t('admin.available_today', 'available today')}`}
                  </span>
                </div>
              )}

              <div className="flex gap-2 pt-2">
                <Button onClick={() => { setPreviewProduct(null); openEditForm(previewProduct); }} size="sm" className="flex-1">
                  <i className="ti ti-pencil" /> {t('common.edit', 'Edit')}
                </Button>
                <Button onClick={() => setPreviewProduct(null)} variant="ghost" size="sm">{t('common.close', 'Close')}</Button>
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
              <h3 className="text-lg font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{editingProduct ? t('admin.edit_item', 'Edit Item') : t('admin.add_item', 'Add Item')}</h3>
              <button onClick={closeForm} className="w-8 h-8 flex items-center justify-center rounded-md hover:bg-[var(--brand-surface-raised)]">
                <i className="ti ti-x" style={{ color: 'var(--brand-text-muted)' }} />
              </button>
            </div>

            {/* Photo */}
            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.photo', 'Photo')}</label>
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
                  {formImage && <button onClick={() => setFormImage(null)} className="text-[var(--color-danger)] underline">{t('common.remove', 'Remove')}</button>}
                </div>
              </div>
            </div>

            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.name', 'Name')} *</label>
              <Input value={formName} onChange={e => setFormName(e.target.value)} placeholder="e.g. Margherita Pizza" autoFocus />
            </div>

            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.price_all', 'Price (ALL)')} *</label>
              <Input value={formPrice} onChange={e => setFormPrice(e.target.value)} placeholder="600" type="number" />
            </div>

            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.description', 'Description')}</label>
              <Input value={formDesc} onChange={e => setFormDesc(e.target.value)} placeholder={t('admin.short_description', 'Short description...')} />
            </div>

            {/* BOM Recipe */}
            <RecipeEditor lines={formRecipeLines} onChange={setFormRecipeLines}
            />

            {/* Stock count */}
            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.available_today_pieces', 'Available today (pieces)')}</label>
              <Input value={formStock} onChange={e => setFormStock(e.target.value)} placeholder={t('admin.leave_empty_unlimited', 'Leave empty for unlimited')} type="number" />
            </div>

            {/* Taste Profile */}
            <div>
              <label className="text-xs font-medium mb-2 block" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.taste_profile', 'Taste Profile')} <span className="opacity-50">({t('admin.optional', 'optional')})</span></label>
              <div className="space-y-1.5">
                {TASTE_AXES.map(axis => (
                  <div key={axis} className="flex items-center gap-2">
                    <span className="text-xs w-20 shrink-0 inline-flex items-center gap-1" style={{ color: 'var(--brand-text-muted)' }}>
                      <i className={TASTE_ICONS[axis] || 'ti ti-circle'} style={{ fontSize: '0.65rem' }} />
                      {t(`admin.taste_${axis}`, TASTE_LABELS[axis])}
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
                              color: active ? 'var(--color-on-primary)' : 'var(--brand-text-muted)',
                            }}
                            title={`${TASTE_LABELS[axis] || axis}: ${level === 1 ? t('admin.taste_low', 'Low') : level === 2 ? t('admin.taste_medium', 'Medium') : t('admin.taste_high', 'High')}`}
                          >
                            {level === 1 ? t('admin.taste_low', 'Low') : level === 2 ? t('admin.taste_med', 'Med') : t('admin.taste_high', 'High')}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <label className="flex items-center gap-3 cursor-pointer">
              <input type="checkbox" checked={formAvailable} onChange={e => setFormAvailable(e.target.checked)} className="w-4 h-4 rounded accent-[var(--brand-primary)]" />
              <span className="text-sm">{t('admin.available_for_order', 'Available for order')}</span>
            </label>

            <div className="flex gap-3 pt-2">
              <Button onClick={closeForm} variant="ghost" className="flex-1">{t('common.cancel', 'Cancel')}</Button>
              <Button onClick={handleSaveProduct} isLoading={saving} disabled={!formName.trim() || !formPrice} className="flex-1">
                {editingProduct ? t('common.save', 'Save Changes') : t('admin.add_item', 'Add Item')}
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* ── PDF Import Modal ── */}
      {showImport && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4" style={{ background: 'rgba(0,0,0,0.5)' }}>
          <div className="w-full max-w-lg rounded-2xl border shadow-xl overflow-hidden" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b" style={{ borderColor: 'var(--brand-border)' }}>
              <div className="flex items-center gap-2">
                <i className="ti ti-file-import text-lg" style={{ color: 'var(--brand-primary)' }} />
                <h3 className="font-bold">{t('admin.import_menu', 'Import Menu from PDF')}</h3>
              </div>
              <button onClick={resetImport} className="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-[var(--brand-surface)] transition-colors">
                <i className="ti ti-x" style={{ color: 'var(--brand-text-muted)' }} />
              </button>
            </div>

            <div className="p-4 space-y-4">
              {/* Step: Upload */}
              {importStep === 'upload' && (
                <>
                  {/* Mode selector */}
                  <div>
                    <label className="text-[11px] font-medium block mb-1.5" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.import_mode', 'Import Mode')}</label>
                    <div className="flex gap-2">
                      {[
                        { value: 'merge', label: t('admin.import_merge', 'Merge'), desc: t('admin.import_merge_desc', 'Update existing, add new') },
                        { value: 'add_only', label: t('admin.import_add_only', 'Add Only'), desc: t('admin.import_add_only_desc', 'Never overwrite') },
                        { value: 'replace', label: t('admin.import_replace', 'Replace'), desc: t('admin.import_replace_desc', 'Delete & recreate') },
                      ].map(opt => (
                        <button key={opt.value} onClick={() => setImportMode(opt.value as any)}
                          className={`flex-1 p-2 rounded-lg border text-left transition-colors ${importMode === opt.value ? 'ring-2 ring-[var(--brand-primary)]' : ''}`}
                          style={{ background: importMode === opt.value ? 'var(--brand-primary-light)' : 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
                          <div className="text-xs font-semibold" style={{ color: 'var(--brand-text)' }}>{opt.label}</div>
                          <div className="text-[10px] mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>{opt.desc}</div>
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* Drop zone */}
                  <div
                    onDragOver={e => { e.preventDefault(); setImportDragOver(true); }}
                    onDragLeave={() => setImportDragOver(false)}
                    onDrop={e => { e.preventDefault(); setImportDragOver(false); const f = e.dataTransfer.files?.[0]; if (f) setImportFile(f); }}
                    className={`border-2 border-dashed rounded-xl p-8 text-center transition-colors cursor-pointer ${importDragOver ? 'border-[var(--brand-primary)] bg-[var(--brand-primary-light)]' : 'border-[var(--brand-border)]'}`}
                    onClick={() => document.getElementById('import-file-input')?.click()}>
                    <input id="import-file-input" type="file" accept=".pdf,image/*" className="hidden"
                      onChange={e => { const f = e.target.files?.[0]; if (f) setImportFile(f); }} />
                    {importFile ? (
                      <div className="space-y-2">
                        <i className="ti ti-file-check text-3xl" style={{ color: 'var(--color-success)' }} />
                        <div className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>{importFile.name}</div>
                        <div className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{(importFile.size / 1024).toFixed(0)} KB</div>
                        <button onClick={e => { e.stopPropagation(); setImportFile(null); }} className="text-xs underline" style={{ color: 'var(--brand-text-muted)' }}>
                          {t('common.remove', 'Remove')}
                        </button>
                      </div>
                    ) : (
                      <div className="space-y-2">
                        <i className="ti ti-upload text-3xl" style={{ color: 'var(--brand-text-muted)' }} />
                        <div className="text-sm" style={{ color: 'var(--brand-text)' }}>
                          {t('admin.import_drop', 'Drop your PDF or menu image here')}
                        </div>
                        <div className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
                          {t('admin.import_hint', 'AI will analyze and create categories, items & supplies. Review before saving.')}
                        </div>
                      </div>
                    )}
                  </div>

                  {importError && (
                    <div className="p-3 rounded-lg text-xs" style={{ background: 'var(--color-danger-light)', color: 'var(--color-danger)' }}>
                      {importError}
                    </div>
                  )}

                  <Button onClick={handleImportUpload} isLoading={importLoading} disabled={!importFile} className="w-full">
                    <i className="ti ti-brain" /> {t('admin.analyze_menu', 'Analyze with AI')}
                  </Button>
                </>
              )}

              {/* Step: Preview */}
              {importStep === 'preview' && importPreview && (
                <>
                  <div className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>
                    {t('admin.import_preview_title', 'AI parsed your menu. Review before saving:')}
                  </div>

                  {/* Summary */}
                  <div className="grid grid-cols-3 gap-2">
                    {[
                      { label: t('admin.categories', 'Categories'), value: importPreview.draft_preview?.categories?.length ?? 0, icon: 'ti ti-folder' },
                      { label: t('admin.products', 'Products'), value: importPreview.draft_preview?.products?.length ?? 0, icon: 'ti ti utensils-crossed' },
                      { label: t('admin.issues', 'Issues'), value: importPreview.issues?.length ?? 0, icon: 'ti ti-alert-triangle', danger: (importPreview.issues?.length ?? 0) > 0 },
                    ].map((s, i) => (
                      <div key={i} className="p-3 rounded-lg border text-center" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
                        <i className={`${s.icon} text-lg`} style={{ color: s.danger ? 'var(--color-danger)' : 'var(--brand-primary)' }} />
                        <div className="text-lg font-bold" style={{ color: 'var(--brand-text)' }}>{s.value}</div>
                        <div className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>{s.label}</div>
                      </div>
                    ))}
                  </div>

                  {/* Categories preview */}
                  {importPreview.draft_preview?.categories?.length > 0 && (
                    <div className="space-y-1.5">
                      <div className="text-[11px] font-semibold" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.categories', 'Categories')}</div>
                      {importPreview.draft_preview.categories.map((c: any, i: number) => (
                        <div key={i} className="flex items-center gap-2 text-sm p-2 rounded-lg" style={{ background: 'var(--brand-surface)' }}>
                          <i className="ti ti-folder text-xs" style={{ color: 'var(--brand-primary)' }} />
                          <span style={{ color: 'var(--brand-text)' }}>{c.name}</span>
                        </div>
                      ))}
                    </div>
                  )}

                  {/* Products preview */}
                  {importPreview.draft_preview?.products?.length > 0 && (
                    <div className="space-y-1.5 max-h-48 overflow-y-auto custom-scrollbar">
                      <div className="text-[11px] font-semibold" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.products', 'Products')}</div>
                      {importPreview.draft_preview.products.map((p: any, i: number) => (
                        <div key={i} className="flex items-center justify-between text-sm p-2 rounded-lg" style={{ background: 'var(--brand-surface)' }}>
                          <div className="flex items-center gap-2">
                            <i className="ti ti-package text-xs" style={{ color: 'var(--brand-primary)' }} />
                            <span style={{ color: 'var(--brand-text)' }}>{p.name}</span>
                            {p.categoryKey && (
                              <span className="text-[10px] px-1.5 py-0.5 rounded" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                                {p.categoryKey}
                              </span>
                            )}
                          </div>
                          <span className="font-semibold text-xs" style={{ color: 'var(--brand-primary)' }}>{p.price} ALL</span>
                        </div>
                      ))}
                    </div>
                  )}

                  {/* Issues */}
                  {importPreview.issues?.length > 0 && (
                    <div className="space-y-1 max-h-32 overflow-y-auto custom-scrollbar">
                      <div className="text-[11px] font-semibold" style={{ color: 'var(--color-danger)' }}>{t('admin.issues', 'Issues')}</div>
                      {importPreview.issues.map((iss: any, i: number) => (
                        <div key={i} className="text-xs p-2 rounded" style={{ background: 'var(--color-danger-light)', color: 'var(--color-danger)' }}>
                          {iss.message}
                        </div>
                      ))}
                    </div>
                  )}

                  {importError && (
                    <div className="p-3 rounded-lg text-xs" style={{ background: 'var(--color-danger-light)', color: 'var(--color-danger)' }}>
                      {importError}
                    </div>
                  )}

                  <div className="flex gap-3">
                    <Button onClick={resetImport} variant="ghost" className="flex-1">{t('common.cancel', 'Cancel')}</Button>
                    <Button onClick={handleImportCommit} isLoading={importLoading} className="flex-1">
                      <i className="ti ti-check" /> {t('admin.import_commit', 'Import Menu')}
                    </Button>
                  </div>
                </>
              )}

              {/* Step: Done */}
              {importStep === 'done' && importResult && (
                <div className="text-center space-y-4 py-4">
                  <i className="ti ti-circle-check-filled text-5xl" style={{ color: 'var(--color-success)' }} />
                  <div>
                    <div className="text-lg font-bold" style={{ color: 'var(--brand-text)' }}>{t('admin.import_success', 'Menu Imported!')}</div>
                    <div className="text-sm mt-1" style={{ color: 'var(--brand-text-muted)' }}>
                      {t('admin.import_result', '{{categories}} categories, {{products}} products created.', {
                        categories: importResult.counts?.categories ?? 0,
                        products: importResult.counts?.products ?? 0,
                      })}
                    </div>
                  </div>
                  <Button onClick={resetImport} className="px-8">{t('common.done', 'Done')}</Button>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
