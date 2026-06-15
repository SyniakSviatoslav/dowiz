import { useState, useEffect } from 'react';
import { apiClient } from '../lib/index.js';
import { z } from 'zod';
import { CategoryResponse, ProductResponse } from '@deliveryos/shared-types';

const CategoryArraySchema = z.array(CategoryResponse);
const ProductArraySchema = z.array(ProductResponse);

export interface Product {
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

export interface Category {
  id: string;
  name: string;
  productCount?: number;
  products?: Product[];
}

export function useMenuData() {
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [productsLoading, setProductsLoading] = useState(false);

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

  const loadProducts = async (categoryId?: string) => {
    setProductsLoading(true);
    try {
      const url = categoryId
        ? `/owner/menu/products?category_id=${categoryId}`
        : '/owner/menu/products';
      const prods = await apiClient<typeof ProductArraySchema>(url, { schema: ProductArraySchema });
      if (Array.isArray(prods)) {
        if (categoryId) {
          setCategories(prev => prev.map(c =>
            c.id === categoryId ? { ...c, products: prods as any } : c
          ));
        } else {
          setCategories(prev => prev.map(cat => ({
            ...cat,
            products: (prods as any[]).filter((p: any) => p.categoryId === cat.id),
          })));
        }
      }
      return prods;
    } catch (err) {
      console.debug('[useMenuData] load products failed:', err);
      return [];
    } finally {
      setProductsLoading(false);
    }
  };

  return {
    categories,
    setCategories,
    loading,
    error,
    productsLoading,
    refresh: fetchCategories,
    loadProducts,
  };
}
