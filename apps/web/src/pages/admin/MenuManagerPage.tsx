import React, { useState, useEffect, useMemo } from 'react';
import { motion } from 'framer-motion';
import { Button, Input, EmptyState, useI18n, useConfirm, MobilePicker, useIsMobile, PriceDisplay, getAllergenStyle, useToast } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';
import { CategoryResponse, ProductResponse } from '@deliveryos/shared-types';

const CategoryArraySchema = z.array(CategoryResponse);
const ProductArraySchema = z.array(ProductResponse);
const AnySchema = z.any();

const MenuImportPreviewResponse = z.object({
  import_session_id: z.string().optional(),
  draft_preview: z.any().optional(),
  issues: z.array(z.any()).optional(),
}).passthrough();

const MenuImportCommitResponse = z.object({
  counts: z.object({
    categories: z.number().optional(),
    products: z.number().optional(),
  }).optional(),
}).passthrough();
import { RecipeEditor } from './RecipeEditor.js';

function getProductAllergens(product: Product): string[] {
  const set = new Set<string>();
  if (product.recipeLines) {
    for (const line of product.recipeLines) {
      if (Array.isArray(line.allergens)) line.allergens.forEach(a => set.add(a));
    }
  }
  if (product.attributes && typeof product.attributes === 'object') {
    const bom = (product.attributes as any).bom;
    if (Array.isArray(bom)) {
      for (const line of bom) {
        if (Array.isArray(line.allergens)) line.allergens.forEach((a: string) => set.add(a));
      }
    }
  }
  return Array.from(set).sort();
}


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
  recipeLines?: Array<{ supplyId: string; supplyName: string; qty: number; unit: string; kind: string; kcal: number | null; proteinG: number | null; fatG: number | null; carbsG: number | null; allergens: string[] }>;
  attributes?: Record<string, unknown>;
}

interface Category {
  id: string;
  name: string;
  product_count?: number;
  products?: Product[];
}



export function MenuManagerPage() {
  const { t } = useI18n();
  const { showToast } = useToast();
  const { confirm, dialog: confirmDialog } = useConfirm();
  const isMobile = useIsMobile();
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
  const [pendingImageFile, setPendingImageFile] = useState<File | null>(null);
  const [productsLoading, setProductsLoading] = useState(false);

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
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<'name' | 'price-asc' | 'price-desc'>('name');
  const [filterAvailable, setFilterAvailable] = useState<'all' | 'available' | 'unavailable'>('all');
  const [sortOpen, setSortOpen] = useState(false);
  const [availOpen, setAvailOpen] = useState(false);


  const fetchCategories = async () => {
    try {
      setLoading(true);
      const data = await apiClient<typeof CategoryArraySchema>('/owner/menu/categories', { schema: CategoryArraySchema });
      setCategories(Array.isArray(data) ? data : []);
      setError('');
    } catch (err: any) {
      setCategories([]);
      setError('Failed to load menu');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { fetchCategories(); }, []);

  // Load all products when "All" category is selected
  useEffect(() => {
    if (selectedCategory === null && categories.length > 0) {
      loadAllProducts();
    }
  }, [selectedCategory, categories.length]);

  const loadAllProducts = async () => {
    setProductsLoading(true);
    try {
      const prods = await apiClient<typeof ProductArraySchema>('/owner/menu/products', { schema: ProductArraySchema });
      if (Array.isArray(prods)) {
        setCategories(prev => prev.map(cat => ({
          ...cat,
          products: prods.filter(p => p.categoryId === cat.id) as any,
        })));
      }
    } catch (err) {
      console.debug('[MenuManagerPage] load all products failed:', err);
    } finally {
      setProductsLoading(false);
    }
  };

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
    setFormRecipeLines(product.recipeLines || []);
  };

  const closeForm = () => {
    setShowForm(false);
    setEditingProduct(null);
    setFormName(''); setFormPrice(''); setFormDesc('');
    setFormImage(null); setFormStock(''); setPendingImageFile(null);
    setSaving(false);
  };

  const handleSaveProduct = async () => {
    if (!formName.trim() || !formPrice || !expandedCat) return;
    const price = parseInt(formPrice);
    if (isNaN(price) || price <= 0) return;
    const stock = formStock ? parseInt(formStock) : undefined;
    if (stock !== undefined && (isNaN(stock) || stock < 0)) return;

    setSaving(true);
    const hasTaste = Object.keys(formTaste).length > 0;
    const hasRecipeLines = formRecipeLines.length > 0;
    const product: Record<string, any> = {
      name: formName.trim(), price,
      description: formDesc || undefined,
      available: formAvailable,
      categoryId: expandedCat,
      taste: hasTaste ? formTaste : undefined,
      stockCount: stock,
      recipeLines: hasRecipeLines ? formRecipeLines : undefined,
    };

    try {
      let saved: any;
      let productId: string;
      if (editingProduct) {
        saved = await apiClient(`/owner/menu/products/${editingProduct.id}`, { method: 'PATCH', body: product });
        productId = editingProduct.id;
      } else {
        saved = await apiClient('/owner/menu/products', { method: 'POST', body: product });
        productId = saved.id;
      }
      if (pendingImageFile) {
        const formData = new FormData();
        formData.append('file', pendingImageFile);
        try {
          await apiClient(`/owner/menu/products/${productId}/image`, { method: 'POST', body: formData, timeout: 30000 });
          setPendingImageFile(null);
        } catch (err) {
          console.warn('[MenuManagerPage] image upload failed:', err);
          showToast(t('admin.image_upload_failed', 'Image upload failed. You can try again.'), 'warning');
        }
      }
      closeForm();
      await fetchCategories();
      const prods = await apiClient<typeof ProductArraySchema>(`/owner/menu/products?category_id=${expandedCat}`, { schema: ProductArraySchema });
      setCategories(prev => prev.map(c => c.id === expandedCat ? { ...c, products: Array.isArray(prods) ? (prods as any) : [] } : c));
      showToast(t('admin.product_saved', 'Product saved'), 'success');
    } catch (err) {
      console.error('[MenuManagerPage] save product failed:', err);
      showToast(t('common.error_save', 'Failed to save product.'), 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleImageSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    if (!file.type.startsWith('image/')) { showToast(t('admin.error_image_only', 'Only image files (JPG, PNG, WebP)'), 'error'); return; }
    if (file.size > 5 * 1024 * 1024) { showToast(t('admin.error_max_size', 'Max 5 MB'), 'error'); return; }
    setFormImage(URL.createObjectURL(file));
    setPendingImageFile(file);
  };

  const handleDeleteProduct = async (catId: string, productId: string) => {
    const ok = await confirm({ title: t('admin.confirm_delete_product_title', 'Delete product'), message: t('admin.confirm_delete_product', 'Are you sure you want to delete this product?'), confirmLabel: t('common.delete', 'Delete'), variant: 'danger' });
    if (!ok) return;
    try { 
      await apiClient(`/owner/menu/products/${productId}`, { method: 'DELETE' }); 
      setCategories(prev => prev.map(c =>
        c.id === catId ? { ...c, products: (c.products || []).filter(p => p.id !== productId) } : c
      ));
      showToast(t('admin.product_deleted', 'Product deleted'), 'success');
    } catch (err) {
      console.error('[MenuManagerPage] delete product failed:', err);
      showToast(t('common.error_delete', 'Failed to delete.'), 'error');
    }
  };

  const handleToggleAvailable = async (catId: string, product: Product) => {
    const updated = { ...product, available: !product.available };
    setCategories(prev => prev.map(c => {
      if (c.id !== catId) return c;
      return { ...c, products: (c.products || []).map(p => p.id === product.id ? updated : p) };
    }));
    try { await apiClient(`/owner/menu/products/${product.id}`, { method: 'PATCH', body: { available: updated.available } }); } catch (err) {
      console.debug('[MenuManager] failed to toggle product availability:', err);
    }
  };

  const handleAddCategory = async () => {
    const name = newCategoryName.trim();
    if (!name) return;
    setNewCategoryName('');
    try { 
      await apiClient('/owner/menu/categories', { method: 'POST', body: { name } }); 
      await fetchCategories();
      showToast(t('admin.category_saved', 'Category created'), 'success');
    } catch (err) {
      console.error('[MenuManagerPage] add category failed:', err);
      showToast(t('common.error_save', 'Failed to save category.'), 'error');
    }
  };

  const handleDeleteCategory = async (categoryId: string) => {
    const ok = await confirm({ title: t('admin.confirm_delete_category_title', 'Delete category'), message: t('admin.confirm_delete_category', 'Delete this category? Products in it will need to be moved or deleted first.'), confirmLabel: t('common.delete', 'Delete'), variant: 'danger' });
    if (!ok) return;
    try {
      await apiClient(`/owner/menu/categories/${categoryId}`, { method: 'DELETE' });
      setCategories(prev => prev.filter(c => c.id !== categoryId));
      showToast(t('admin.category_deleted', 'Category deleted'), 'success');
    } catch (err: any) {
      showToast(err.message || t('common.error_delete', 'Failed to delete.'), 'error');
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
      const res = await apiClient<typeof MenuImportPreviewResponse>('/owner/menu/import/preview', { method: 'POST', body: formData, timeout: 120000, schema: MenuImportPreviewResponse });
      setImportSessionId(res.import_session_id ?? null);
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
      const res = await apiClient<typeof MenuImportCommitResponse>('/owner/menu/import/commit', {
        method: 'POST',
        body: { import_session_id: importSessionId, force: true },
        schema: MenuImportCommitResponse,
      });
      setImportResult(res);
      setImportStep('done');
      showToast(t('admin.import_success', 'Menu imported'), 'success');
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

  const loadCategoryProducts = async (catId: string) => {
    setProductsLoading(true);
    try {
      const prods = await apiClient<typeof ProductArraySchema>(`/owner/menu/products?category_id=${catId}`, { schema: ProductArraySchema });
      setCategories(prev => prev.map(c => c.id === catId ? { ...c, products: Array.isArray(prods) ? (prods as any) : [] } : c));
    } catch (err) {
      console.debug('[MenuManagerPage] load category products failed:', err);
      setCategories(prev => prev.map(c => c.id === catId ? { ...c, products: [] } : c));
    } finally {
      setProductsLoading(false);
    }
  };

  const toggleExpand = async (catId: string) => {
    if (expandedCat === catId) { setExpandedCat(null); return; }
    setExpandedCat(catId);
    const cat = categories.find(c => c.id === catId);
    if (cat && cat.products === undefined) {
      await loadCategoryProducts(catId);
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
          <motion.button onClick={fetchCategories} whileTap={{ scale: 0.97 }} className="underline ml-3 shrink-0">{t('common.retry', 'Retry')}</motion.button>
        </div>
      )}

      {/* Toolbar: search, filter, sort, add category */}
      <div className="flex flex-wrap gap-2">
        <div className="relative flex-1 min-w-[200px]">
          <i className="ti ti-search absolute left-3 top-1/2 -translate-y-1/2 text-sm" style={{ color: 'var(--brand-text-muted)' }} />
          <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)} placeholder={t('admin.search_products', 'Search products...')}
            className="w-full pl-9 pr-4 py-2 rounded-lg border text-sm outline-none"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
        </div>

        {/* Sort */}
        <div className="relative">
          <motion.button onClick={() => setSortOpen(true)} whileTap={{ scale: 0.97 }}
            className="flex items-center gap-1.5 px-3 py-2 rounded-lg border text-sm outline-none"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
            <i className="ti ti-arrows-sort text-base" />
            <span className="hidden sm:inline text-xs">{sortBy === 'name' ? t('admin.name_az', 'Name A-Z') : sortBy === 'price-asc' ? t('admin.price_asc', 'Price \u2191') : t('admin.price_desc', 'Price \u2193')}</span>
          </motion.button>
          {isMobile ? (
            <MobilePicker
              open={sortOpen}
              onClose={() => setSortOpen(false)}
              title={t('admin.sort_products', 'Sort products')}
              options={[
                { value: 'name', label: t('admin.name_az', 'Name A-Z'), icon: 'ti ti-sort-az' },
                { value: 'price-asc', label: t('admin.price_asc', 'Price ArrowUp'), icon: 'ti ti-sort-ascending' },
                { value: 'price-desc', label: t('admin.price_desc', 'Price ArrowDown'), icon: 'ti ti-sort-descending' },
              ]}
              selectedValue={sortBy}
              onSelect={(opt) => { setSortBy(opt.value as any); setSortOpen(false); }}
            />
          ) : sortOpen && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setSortOpen(false)} />
              <div className="absolute right-0 top-full mt-1 z-50 rounded-lg shadow-elevation-3 py-1 min-w-[140px] scale-in" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)' }}>
                {[
                  { value: 'name', label: t('admin.name_az', 'Name A-Z'), icon: 'ti ti-sort-az' },
                  { value: 'price-asc', label: t('admin.price_asc', 'Price ArrowUp'), icon: 'ti ti-sort-ascending' },
                  { value: 'price-desc', label: t('admin.price_desc', 'Price ArrowDown'), icon: 'ti ti-sort-descending' },
                ].map(opt => (
                  <motion.button key={opt.value} onClick={() => { setSortBy(opt.value as any); setSortOpen(false); }} whileTap={{ scale: 0.97 }}
                    className={`flex items-center gap-2 w-full px-3 py-2 text-xs transition-colors hover:bg-[var(--brand-surface-raised)] ${sortBy === opt.value ? 'font-semibold' : ''}`}
                    style={{ color: sortBy === opt.value ? 'var(--brand-primary)' : 'var(--brand-text)' }}>
                    <i className={opt.icon} style={{ fontSize: '0.8rem' }} />
                    <span className="flex-1">{opt.label}</span>
                    {sortBy === opt.value && <i className="ti ti-check" style={{ color: 'var(--brand-primary)', fontSize: '0.7rem' }} />}
                  </motion.button>
                ))}
              </div>
            </>
          )}
        </div>

        {/* Availability filter */}
        <div className="relative">
          <motion.button onClick={() => setAvailOpen(true)} whileTap={{ scale: 0.97 }}
            className="flex items-center gap-1.5 px-3 py-2 rounded-lg border text-sm outline-none"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: filterAvailable !== 'all' ? 'var(--brand-primary)' : 'var(--brand-text)' }}>
            <i className="ti ti-filter text-base" />
          </motion.button>
          {isMobile ? (
            <MobilePicker
              open={availOpen}
              onClose={() => setAvailOpen(false)}
              title={t('admin.filter_availability', 'Filter by availability')}
              options={[
                { value: 'all', label: t('admin.all_items', 'All items'), icon: 'ti ti-list' },
                { value: 'available', label: t('menu.available', 'Available'), icon: 'ti ti-circle-check' },
                { value: 'unavailable', label: t('menu.stop_listed', 'Stop-listed'), icon: 'ti ti-circle-x' },
              ]}
              selectedValue={filterAvailable}
              onSelect={(opt) => { setFilterAvailable(opt.value as any); setAvailOpen(false); }}
            />
          ) : availOpen && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setAvailOpen(false)} />
              <div className="absolute right-0 top-full mt-1 z-50 rounded-lg shadow-elevation-3 py-1 min-w-[150px] scale-in" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)' }}>
                {[
                  { value: 'all', label: t('admin.all_items', 'All items'), icon: 'ti ti-list' },
                  { value: 'available', label: t('menu.available', 'Available'), icon: 'ti ti-circle-check' },
                  { value: 'unavailable', label: t('menu.stop_listed', 'Stop-listed'), icon: 'ti ti-circle-x' },
                ].map(opt => (
                  <motion.button key={opt.value} onClick={() => { setFilterAvailable(opt.value as any); setAvailOpen(false); }} whileTap={{ scale: 0.97 }}
                    className={`flex items-center gap-2 w-full px-3 py-2 text-xs transition-colors hover:bg-[var(--brand-surface-raised)] ${filterAvailable === opt.value ? 'font-semibold' : ''}`}
                    style={{ color: filterAvailable === opt.value ? 'var(--brand-primary)' : 'var(--brand-text)' }}>
                    <i className={opt.icon} style={{ fontSize: '0.8rem' }} />
                    <span className="flex-1">{opt.label}</span>
                    {filterAvailable === opt.value && <i className="ti ti-check" style={{ color: 'var(--brand-primary)', fontSize: '0.7rem' }} />}
                  </motion.button>
                ))}
              </div>
            </>
          )}
        </div>

        <div className="flex gap-2 items-center">
          <input value={newCategoryName} onChange={e => setNewCategoryName(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && handleAddCategory()}
            placeholder={t('admin.new_category', 'New category...')}
            className="w-32 sm:w-40 h-10 px-3 rounded-lg border text-sm outline-none"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          <motion.button onClick={handleAddCategory} whileTap={{ scale: 0.97 }}
            className="flex items-center gap-1 px-3 py-2 rounded-lg text-sm font-medium shrink-0 border border-[var(--brand-primary)]"
            style={{ background: 'var(--brand-primary-light)', color: 'var(--brand-text)' }}>
            <i className="ti ti-plus text-sm" />
          </motion.button>
        </div>
      </div>

      {/* Category tabs */}
      {!loading && categories.length > 0 && (
        <div className="flex overflow-x-auto hide-scrollbar gap-1 pb-1 snap-x snap-mandatory sticky top-0 z-10" style={{ background: 'var(--brand-bg)' }}>
          <motion.button onClick={() => setSelectedCategory(null)} whileTap={{ scale: 0.97 }}
            className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all snap-start shrink-0 whitespace-nowrap ${selectedCategory === null ? 'bg-[var(--brand-primary-light)] text-[var(--brand-text)] shadow-sm border border-[var(--brand-primary)]' : 'bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] hover:text-[var(--brand-text)] border border-transparent'}`}>
            {t('common.all', 'All')}
          </motion.button>
          {categories.map(cat => (
            <motion.button key={cat.id} onClick={async () => { setSelectedCategory(cat.id); await toggleExpand(cat.id); }} whileTap={{ scale: 0.97 }}
              className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all snap-start shrink-0 whitespace-nowrap ${selectedCategory === cat.id ? 'bg-[var(--brand-primary-light)] text-[var(--brand-text)] shadow-sm border border-[var(--brand-primary)]' : 'bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] hover:text-[var(--brand-text)] border border-transparent'}`}>
              {cat.name} <span className="text-[10px] opacity-70">({cat.product_count ?? cat.products?.length ?? 0})</span>
            </motion.button>
          ))}
        </div>
      )}

      {/* Products grid */}
      {loading ? (
        <div className="space-y-3">{ [1,2,3].map(i => <div key={i} className="h-12 shimmer rounded-lg" />) }</div>
      ) : categories.length === 0 ? (
        <EmptyState title={t('admin.no_categories', 'No categories')} description={t('admin.add_category_desc', 'Add a category above to start.')} />
      ) : (
        <div className="space-y-2">
          {/* Active category header with add button */}
          <div className="flex items-center justify-between">
            <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>
              {(() => {
                const catsToShow = selectedCategory ? categories.filter(c => c.id === selectedCategory) : categories;
                let count = 0;
                catsToShow.forEach(c => { count += getAllProducts(c.id).length; });
                return `${count} ${t('admin.products', 'products')}`;
              })()}
            </p>
            {selectedCategory && (
              <motion.button onClick={() => openAddForm(selectedCategory)} whileTap={{ scale: 0.97 }}
                className="flex items-center gap-1 px-3 py-1.5 text-xs font-medium rounded-lg border border-[var(--brand-primary)]"
                style={{ background: 'var(--brand-primary-light)', color: 'var(--brand-text)' }}>
                <i className="ti ti-plus text-xs" /> {t('common.add', 'Add')}
              </motion.button>
            )}
          </div>
          {productsLoading && selectedCategory ? (
            <div className="col-span-full flex justify-center py-8">
              <i className="ti ti-loader animate-spin text-xl" style={{ color: 'var(--brand-primary)' }} />
            </div>
          ) : (
          <motion.div
            className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3"
            variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.03, delayChildren: 0.05 } } }}
            initial="hidden"
            animate="visible"
          >
            {(selectedCategory ? categories.filter(c => c.id === selectedCategory) : categories).map(cat => {
              if (cat.products === undefined && selectedCategory === cat.id) return null;
              const products = getAllProducts(cat.id);
              if (products.length === 0 && searchQuery) return null;
              return products.map((product) => (
                <motion.div key={product.id}
                  variants={{ hidden: { opacity: 0, y: 12, scale: 0.97 }, visible: { opacity: 1, y: 0, scale: 1, transition: { type: 'spring', stiffness: 260, damping: 24 } } }}
                  whileTap={{ scale: 0.97 }}
                  className="p-3 rounded-xl border transition-all hover:bg-[var(--brand-surface-raised)] cursor-pointer fade-in"
                  style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}
                  onClick={() => setPreviewProduct(product)}>
                  <div className="flex items-start gap-3">
                    <div className="w-14 h-14 rounded-lg overflow-hidden shrink-0 flex items-center justify-center"
                      style={{ background: 'var(--brand-primary-light)' }}>
                      {product.imageUrl
                        ? <img src={product.imageUrl} alt="" className="w-full h-full object-cover" loading="lazy"
                            onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; (e.target as HTMLImageElement).parentElement!.innerHTML = '<i class=\\"ti ti-photo\\"></i>'; }} />
                        : <i className="ti ti-photo" style={{ color: 'var(--brand-primary)' }} />
                      }
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between gap-1">
                        <span className="font-medium text-sm truncate" style={{ color: 'var(--brand-text)' }}>{product.name}</span>
                        <span className="text-sm font-bold shrink-0" style={{ color: 'var(--brand-primary)' }}><PriceDisplay amount={product.price} /></span>
                      </div>
                      {product.description && <div className="text-[11px] truncate mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>{product.description}</div>}
                      <div className="flex items-center gap-2 mt-1.5">
                        <motion.button onClick={(e) => { e.stopPropagation(); handleToggleAvailable(cat.id, product); }} whileTap={{ scale: 0.97 }}
                          className={`relative w-8 h-4 rounded-full transition-colors duration-200 ${product.available ? 'bg-[var(--brand-primary)]' : 'bg-[var(--brand-border)]'}`}
                          title={product.available ? t('menu.available', 'Available') : t('menu.stop_listed', 'Stop-listed')}
                          role="switch" aria-checked={product.available}>
                          <div className={`absolute top-0.5 w-3 h-3 rounded-full bg-white transition-transform duration-200 shadow-sm ${product.available ? 'left-[18px]' : 'left-0.5'}`} />
                        </motion.button>
                        <span className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>
                          {product.available ? t('menu.available', 'Available') : t('menu.stop_listed', 'Stop-listed')}
                        </span>
                        {product.stockCount != null && (
                          <span className={`text-[10px] font-medium px-1.5 py-0.5 rounded-full ${product.stockCount > 0 ? 'text-[var(--color-success)]' : 'text-[var(--color-danger)]'}`}
                            style={{ background: product.stockCount > 0 ? 'var(--color-success-light)' : 'var(--color-danger-light)' }}>
                            {product.stockCount}
                          </span>
                        )}
                      </div>
                      {getProductAllergens(product).length > 0 && (
                        <div className="flex flex-wrap gap-1 mt-1">
                          {getProductAllergens(product).map(a => {
                            const s = getAllergenStyle(a);
                            return (
                              <span key={a} className="text-[8px] font-semibold px-1 py-0.5 rounded-full leading-tight"
                                style={{ background: s.bg, color: s.text }}>
                                {t(`allergen.${a.toLowerCase()}`, a)}
                              </span>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center justify-end gap-1 mt-2 pt-2 border-t" style={{ borderColor: 'var(--brand-border)' }}>
                    <motion.button onClick={(e) => { e.stopPropagation(); openEditForm(product); }} whileTap={{ scale: 0.97 }}
                      className="w-7 h-7 flex items-center justify-center rounded-md hover:bg-[var(--brand-surface)] transition-colors"
                      title={t('common.edit', 'Edit')}>
                      <i className="ti ti-edit" style={{ fontSize: '0.75rem', color: 'var(--brand-text-muted)' }} />
                    </motion.button>
                    <motion.button onClick={(e) => { e.stopPropagation(); handleDeleteProduct(cat.id, product.id); }} whileTap={{ scale: 0.97 }}
                      className="w-7 h-7 flex items-center justify-center rounded-md hover:bg-[var(--color-danger-light)] transition-colors"
                      title={t('common.delete', 'Delete')}>
                      <i className="ti ti-trash" style={{ fontSize: '0.75rem', color: 'var(--brand-text-muted)' }} />
                    </motion.button>
                  </div>
                </motion.div>
              ));
            })}
          </motion.div>
          )}
          {selectedCategory === null && categories.every(cat => getAllProducts(cat.id).length === 0) && searchQuery && (
            <EmptyState title={t('admin.no_matching_products', 'No matching products.')} description="" />
          )}
        </div>
      )}

      {/* Product Preview Card */}
      {previewProduct && (
        <div className="fixed inset-0 z-50 flex items-center justify-center fade-in" onClick={() => setPreviewProduct(null)}>
          <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" />
          <div className="relative w-[320px] bg-[var(--brand-surface)] rounded-2xl overflow-hidden shadow-2xl z-10 scale-in" onClick={e => e.stopPropagation()}>
            <div className="aspect-[4/3] relative" style={{ background: 'var(--brand-surface-raised)' }}>
              {previewProduct.imageUrl
                ? <img src={previewProduct.imageUrl} alt="" className="w-full h-full object-cover" loading="lazy"
                    onError={(e) => { const t = e.target as HTMLImageElement; t.style.display = 'none'; const p = t.parentElement; if (p) { const i = document.createElement('i'); i.className = 'ti ti-photo text-4xl'; i.style.cssText = 'color: var(--brand-border)'; p.appendChild(i); } }} />
                : <div className="w-full h-full flex items-center justify-center"><i className="ti ti-photo text-4xl" style={{ color: 'var(--brand-border)' }} /></div>
              }
              <motion.button onClick={() => setPreviewProduct(null)} whileTap={{ scale: 0.97 }} className="absolute top-3 right-3 w-8 h-8 rounded-full bg-black/40 text-white flex items-center justify-center">
                <i className="ti ti-x" />
              </motion.button>
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
                <span className="text-xl font-black shrink-0" style={{ color: 'var(--brand-primary)' }}><PriceDisplay amount={previewProduct.price} /></span>
              </div>



              {previewProduct.stockCount != null && (
                <div className="flex items-center gap-2">
                  <span className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.in_stock_label', 'In stock:')}</span>
                  <span className={`text-sm font-bold ${previewProduct.stockCount === 0 ? 'text-[var(--color-danger)]' : 'text-[var(--color-success)]'}`}>
                    {previewProduct.stockCount === 0 ? t('menu.out_of_stock', 'Out of stock') : `${previewProduct.stockCount} ${t('admin.available_today', 'available today')}`}
                  </span>
                </div>
              )}

              {getProductAllergens(previewProduct).length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                  {getProductAllergens(previewProduct).map(a => {
                    const s = getAllergenStyle(a);
                    return (
                      <span key={a} className="text-[10px] font-bold px-2 py-0.5 rounded-full"
                        style={{ background: s.bg, color: s.text }}>
                        {t(`allergen.${a.toLowerCase()}`, a)}
                      </span>
                    );
                  })}
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
          <div className="relative w-full max-w-md bg-[var(--brand-surface)] rounded-t-2xl sm:rounded-2xl p-6 space-y-4 z-10 slide-in-up max-h-[85vh] overflow-auto pb-20 sm:pb-6" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{editingProduct ? t('admin.edit_item', 'Edit Item') : t('admin.add_item', 'Add Item')}</h3>
              <motion.button onClick={closeForm} whileTap={{ scale: 0.97 }} className="w-8 h-8 flex items-center justify-center rounded-md hover:bg-[var(--brand-surface-raised)]">
                <i className="ti ti-x" style={{ color: 'var(--brand-text-muted)' }} />
              </motion.button>
            </div>

            {/* Photo */}
            <div>
              <label className="text-xs font-medium mb-1 block" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.photo', 'Photo')}</label>
              <div className="flex items-start gap-3">
                <label className="flex flex-col items-center justify-center w-20 h-20 rounded-lg border-2 border-dashed cursor-pointer hover:border-[var(--brand-primary)] transition-colors shrink-0"
                  style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface-raised)' }}>
                  {formImage ? <img src={formImage} alt="" className="w-full h-full object-cover rounded-lg" />
                    : <div className="text-center"><i className="ti ti-camera text-lg" style={{ color: 'var(--brand-text-muted)' }} /><span className="text-[9px] block">JPG/PNG</span></div>}
                  <input type="file" accept="image/jpeg,image/png,image/webp" onChange={handleImageSelect} className="hidden" />
                </label>
                <div className="text-xs space-y-1" style={{ color: 'var(--brand-text-muted)' }}>
                  <p>4:3 ratio, max 5 MB</p>
                  <p>JPG, PNG, WebP</p>
                  {formImage && <button onClick={async () => { const ok = await confirm({ title: t('admin.confirm_remove_image_title', 'Remove image'), message: t('admin.confirm_remove_image', 'Are you sure you want to remove the image?'), variant: 'danger' }); if (ok) { setFormImage(null); setPendingImageFile(null); } }} className="text-[var(--color-danger)] underline">{t('common.remove', 'Remove')}</button>}
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
                          <motion.button
                            key={level}
                            type="button"
                            whileTap={{ scale: 0.97 }}
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
                          </motion.button>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <label className="flex items-center gap-3 cursor-pointer select-none" onClick={() => setFormAvailable(!formAvailable)}>
              <div className={`relative w-11 h-6 rounded-full transition-colors duration-200 ${formAvailable ? 'bg-[var(--brand-primary)]' : 'bg-[var(--brand-border)]'}`}>
                <div className={`absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white transition-transform duration-200 shadow-sm ${formAvailable ? 'translate-x-5' : 'translate-x-0'}`} />
              </div>
              <span className="text-sm font-medium" style={{ color: formAvailable ? 'var(--brand-text)' : 'var(--brand-text-muted)' }}>
                {t('admin.available_for_order', 'Available for order')}
              </span>
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
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4"><div className="absolute inset-0 bg-black/50 backdrop-blur-sm" />
          <div className="w-full max-w-lg rounded-2xl border shadow-xl overflow-hidden" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b" style={{ borderColor: 'var(--brand-border)' }}>
              <div className="flex items-center gap-2">
                <i className="ti ti-file-import text-lg" style={{ color: 'var(--brand-primary)' }} />
                <h3 className="font-bold">{t('admin.import_menu', 'Import Menu from PDF')}</h3>
              </div>
              <motion.button onClick={resetImport} whileTap={{ scale: 0.97 }} className="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-[var(--brand-surface)] transition-colors">
                <i className="ti ti-x" style={{ color: 'var(--brand-text-muted)' }} />
              </motion.button>
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
                        <motion.button key={opt.value} onClick={() => setImportMode(opt.value as any)} whileTap={{ scale: 0.97 }}
                          className={`flex-1 p-2 rounded-lg border text-left transition-colors ${importMode === opt.value ? 'ring-2 ring-[var(--brand-primary)]' : ''}`}
                          style={{ background: importMode === opt.value ? 'var(--brand-primary-light)' : 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
                          <div className="text-xs font-semibold" style={{ color: 'var(--brand-text)' }}>{opt.label}</div>
                          <div className="text-[10px] mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>{opt.desc}</div>
                        </motion.button>
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
                        <motion.button onClick={e => { e.stopPropagation(); setImportFile(null); }} whileTap={{ scale: 0.97 }} className="text-xs underline" style={{ color: 'var(--brand-text-muted)' }}>
                          {t('common.remove', 'Remove')}
                        </motion.button>
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
                          <span className="font-semibold text-xs" style={{ color: 'var(--brand-primary)' }}><PriceDisplay amount={p.price} /></span>
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
      {confirmDialog}
    </div>
  );
}
