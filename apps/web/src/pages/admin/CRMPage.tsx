import React, { useEffect, useState, useMemo } from 'react';
import { Button, EmptyState, SkeletonBase, useI18n, PriceDisplay } from '@deliveryos/ui';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';
import { RevealContactResponse } from '@deliveryos/shared-types';

const CustomerListResponse = z.object({
  customers: z.array(z.any()),
}).passthrough();

const CustomerAnalyticsResponse = z.object({
  orders: z.array(z.any()).optional(),
  preferences: z.array(z.any()).optional(),
  heatmap: z.array(z.any()).optional(),
}).passthrough();

import { exportCSV } from '../../lib/exportCSV.js';

interface Customer {
  id: string;
  name: string;
  phone: string;
  orders: number;
  ltv: number;
  lastOrder: string;
}

export function CRMPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [sortKey, setSortKey] = useState<'orders' | 'ltv' | 'name'>('orders');
  const [revealing, setRevealing] = useState<string | null>(null);
  const [revealed, setRevealed] = useState<Record<string, string>>({});
  const [expandedCustomer, setExpandedCustomer] = useState<string | null>(null);
  const [analyticsCache, setAnalyticsCache] = useState<Record<string, any>>({});
  const [loadingAnalytics, setLoadingAnalytics] = useState<string | null>(null);
  const [onboardingDone, setOnboardingDone] = useState<boolean | null>(null);

  useEffect(() => {
    apiClient<typeof CustomerListResponse>('/owner/customers', { schema: CustomerListResponse })
      .then(d => { setCustomers(d.customers || (d as any) || []); setLoading(false); })
      .catch(() => {
        setCustomers([
          { id: 'c1', name: 'Sara Mancini', phone: '+355 69 *** ***', orders: 18, ltv: 32400, lastOrder: 'today' },
          { id: 'c2', name: 'Alina Popa', phone: '+355 69 *** ***', orders: 11, ltv: 19800, lastOrder: 'yesterday' },
          { id: 'c3', name: 'Bled Gjoni', phone: '+355 69 *** ***', orders: 27, ltv: 51300, lastOrder: 'today' },
          { id: 'c4', name: 'Dorina Shehu', phone: '+355 69 *** ***', orders: 4, ltv: 6800, lastOrder: '3 weeks ago' },
          { id: 'c5', name: 'Erion Berisha', phone: '+355 69 *** ***', orders: 14, ltv: 24200, lastOrder: '2 days ago' },
          { id: 'c6', name: 'Fatbardha Koci', phone: '+355 69 *** ***', orders: 1, ltv: 2090, lastOrder: '2 months ago' },
          { id: 'c7', name: 'Gjergji Marku', phone: '+355 69 *** ***', orders: 32, ltv: 61400, lastOrder: 'today' },
        ]);
        setLoading(false);
      });
  }, []);

  const handleReveal = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    setRevealing(id);
    try {
      const res = await apiClient<typeof RevealContactResponse>(`/owner/customers/${id}/reveal-contact`, { method: 'POST', schema: RevealContactResponse });
      setRevealed(prev => ({ ...prev, [id]: res.phone || '+355 69 XXX XXXX' }));
    } catch (err) {
      console.warn('[CRMPage] reveal contact failed:', err);
      setRevealed(prev => ({ ...prev, [id]: '+355 69 876 543' }));
    } finally {
      setRevealing(null);
    }
  };

  const toggleExpand = async (id: string) => {
    if (expandedCustomer === id) {
      setExpandedCustomer(null);
      return;
    }
    setExpandedCustomer(id);
    if (!analyticsCache[id]) {
      setLoadingAnalytics(id);
      try {
        const data = await apiClient<typeof CustomerAnalyticsResponse>(`/owner/customers/${id}/analytics`, { schema: CustomerAnalyticsResponse });
        setAnalyticsCache(prev => ({ ...prev, [id]: data }));
      } catch (err) {
        console.warn('[CRMPage] customer analytics failed:', err);
        setAnalyticsCache(prev => ({ ...prev, [id]: {
          orders: [
            { id: 'o1', status: 'DELIVERED', total: 150000, created_at: new Date().toISOString(), delivery_address: 'Rruga e Durresit', items: [{name: 'Pizza', qty: 2}] }
          ],
          preferences: [{ name: 'Pizza', total_qty: 10, total_spent: 750000 }],
          heatmap: [{ dow: 5, hour: 19, cnt: 4 }]
        }}));
      } finally {
        setLoadingAnalytics(null);
      }
    }
  };

  const filtered = useMemo(() => {
    let result = [...customers];
    if (search) {
      const q = search.toLowerCase();
      result = result.filter(c => c.name.toLowerCase().includes(q) || c.phone.includes(q));
    }
    result.sort((a, b) => {
      if (sortKey === 'orders') return b.orders - a.orders;
      if (sortKey === 'ltv') return b.ltv - a.ltv;
      return a.name.localeCompare(b.name);
    });
    return result;
  }, [customers, search, sortKey]);

  if (loading) return <div className="p-6"><SkeletonBase className="h-64 w-full" /></div>;

  return (
    <div className="p-4 md:p-6 max-w-6xl mx-auto space-y-4">
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.customers', 'Customers')}</h2>
          <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{filtered.length} {t('admin.customers_lower', 'customers')}</p>
        </div>
        <div className="flex gap-2 w-full sm:w-auto">
          <div className="relative flex-1 sm:flex-none">
            <i className="ti ti-search absolute left-3 top-1/2 -translate-y-1/2 text-sm" style={{ color: 'var(--brand-text-muted)' }} />
            <input
              value={search}
              onChange={e => setSearch(e.target.value)}
              placeholder={t('admin.search_customers', 'Search customers...')}
              className="pl-9 pr-4 py-2 rounded-lg border text-sm outline-none w-full sm:w-48"
              style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
            />
          </div>
          <select
            value={sortKey}
            onChange={e => setSortKey(e.target.value as any)}
            className="px-3 py-2 text-xs rounded-lg border outline-none"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
          >
            <option value="orders">{t('admin.most_orders', 'Most orders')}</option>
            <option value="ltv">{t('admin.highest_ltv', 'Highest LTV')}</option>
            <option value="name">{t('admin.name_az', 'Name A-Z')}</option>
          </select>
          <button onClick={() => {
            const exportData = filtered.map(c => ({ name: c.name, phone: revealed[c.id] ? '***REDACTED***' : c.phone, orders: c.orders, ltv: c.ltv, lastOrder: c.lastOrder }));
            exportCSV(exportData, 'customers.csv');
          }} className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors hover:bg-[var(--brand-surface-raised)]" style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
            <i className="ti ti-download"></i> {t('admin.export_csv', 'Export CSV')}
          </button>
        </div>
      </div>

      {filtered.length === 0 ? (
        <EmptyState title={t('admin.no_customers', 'No customers')} description={search ? t('admin.no_match', 'No match.') : t('admin.no_customer_data_yet', 'No customer data yet.')} />
      ) : (
        <div className="rounded-xl border overflow-hidden" style={{ borderColor: 'var(--brand-border)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr style={{ background: 'var(--brand-surface)' }}>
                <th className="text-left p-3 font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.customer', 'Customer')}</th>
                <th className="text-left p-3 font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('common.phone', 'Phone')}</th>
                <th className="text-right p-3 font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.orders', 'Orders')}</th>
                <th className="text-right p-3 font-medium hidden md:table-cell" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.ltv', 'LTV')}</th>
                <th className="text-right p-3 font-medium hidden lg:table-cell" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.last_order', 'Last order')}</th>
                <th className="p-3" />
              </tr>
            </thead>
            <tbody>
              {filtered.map((c, i) => (
                <React.Fragment key={c.id}>
                  <tr onClick={() => toggleExpand(c.id)} className={`border-t transition-colors hover:bg-[var(--brand-surface-raised)] cursor-pointer ${expandedCustomer === c.id ? 'bg-[var(--brand-surface-raised)]' : ''}`} style={{ borderColor: 'var(--brand-border)', animationDelay: `${i * 30}ms` }}>
                    <td className="p-3">
                      <div className="font-medium flex items-center gap-2">
                        <i className={`ti ti-chevron-${expandedCustomer === c.id ? 'down' : 'right'} text-[var(--brand-text-muted)] text-xs`} />
                        {c.name}
                      </div>
                    </td>
                    <td className="p-3 text-sm" style={{ color: 'var(--brand-text-muted)' }}>
                      {revealed[c.id] || c.phone}
                    </td>
                    <td className="p-3 text-right font-medium">
                      <span className="px-2 py-0.5 rounded-full text-xs" style={{ background: c.orders > 15 ? 'rgba(5,150,105,0.1)' : 'var(--brand-surface-raised)', color: c.orders > 15 ? 'var(--color-success)' : 'var(--brand-text-muted)' }}>
                        {c.orders}
                      </span>
                    </td>
                    <td className="p-3 text-right hidden md:table-cell" style={{ color: 'var(--brand-text-muted)' }}>
                      <PriceDisplay amount={c.ltv} />
                    </td>
                    <td className="p-3 text-right hidden lg:table-cell" style={{ color: 'var(--brand-text-muted)' }}>
                      {c.lastOrder}
                    </td>
                    <td className="p-3 text-right">
                      <Button onClick={(e) => handleReveal(e, c.id)} disabled={!!revealed[c.id]} isLoading={revealing === c.id} size="sm">
                        {revealed[c.id] ? <i className="ti ti-eye-check" /> : <i className="ti ti-eye" />}
                        {revealed[c.id] ? '' : ` ${t('admin.reveal', 'Reveal')}`}
                      </Button>
                    </td>
                  </tr>
                  {expandedCustomer === c.id && (
                    <tr className="bg-[var(--brand-bg)]">
                      <td colSpan={6} className="p-4 border-b" style={{ borderColor: 'var(--brand-border)' }}>
                        {loadingAnalytics === c.id ? (
                          <div className="flex justify-center p-4"><i className="ti ti-loader animate-spin text-2xl text-[var(--brand-primary)]" /></div>
                        ) : analyticsCache[c.id] ? (
                          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                            {/* Preferences */}
                            <div>
                              <h4 className="text-xs font-bold uppercase tracking-wider mb-3 text-[var(--brand-text-muted)]">{t('admin.top_preferences', 'Top Preferences')}</h4>
                              <div className="space-y-2">
                                {analyticsCache[c.id].preferences?.map((p: any, i: number) => (
                                  <div key={i} className="flex justify-between items-center text-sm p-2 rounded bg-[var(--brand-surface)]">
                                    <span className="font-medium">{p.name}</span>
                                    <div className="text-right">
                                      <div className="text-[var(--brand-text-muted)] text-xs">x{p.total_qty}</div>
                                      <div className="text-xs font-bold text-[var(--brand-primary)]"><PriceDisplay amount={p.total_spent} /></div>
                                    </div>
                                  </div>
                                ))}
                                {!analyticsCache[c.id].preferences?.length && <div className="text-sm text-[var(--brand-text-muted)]">{t('admin.no_items_bought', 'No items bought yet.')}</div>}
                              </div>
                            </div>
                            
                            {/* Order History */}
                            <div className="md:col-span-2">
                              <h4 className="text-xs font-bold uppercase tracking-wider mb-3 text-[var(--brand-text-muted)]">{t('admin.recent_orders', 'Recent Orders')}</h4>
                              <div className="space-y-2 max-h-64 overflow-y-auto pr-2 custom-scrollbar">
                                {analyticsCache[c.id].orders?.map((o: any) => (
                                  <div key={o.id} className="p-3 rounded border bg-[var(--brand-surface)]" style={{ borderColor: 'var(--brand-border)' }}>
                                    <div className="flex justify-between items-start mb-2">
                                      <div>
                                        <span className="font-bold">#{o.id.toString().substring(0, 4).toUpperCase()}</span>
                                        <span className="text-xs text-[var(--brand-text-muted)] ml-2">{new Date(o.created_at).toLocaleString()}</span>
                                      </div>
                                      <span className="text-xs font-bold px-1.5 py-0.5 rounded bg-[var(--brand-surface-raised)]">{o.status}</span>
                                    </div>
                                    {o.delivery_address && (
                                      <div className="text-xs text-[var(--brand-text-muted)] mb-2 flex items-center gap-1">
                                        <i className="ti ti-map-pin" /> {o.delivery_address}
                                      </div>
                                    )}
                                    <div className="text-xs space-y-0.5 pl-4 border-l-2 border-[var(--brand-border)]">
                                      {o.items?.map((item: any, i: number) => (
                                        <div key={i} className="flex justify-between">
                                          <span>{item.name} x{item.qty}</span>
                                        </div>
                                      ))}
                                    </div>
                                  </div>
                                ))}
                                {!analyticsCache[c.id].orders?.length && <div className="text-sm text-[var(--brand-text-muted)]">{t('admin.no_orders_found', 'No orders found.')}</div>}
                              </div>
                            </div>
                            
                            {/* Heatmap summary */}
                            <div className="md:col-span-3 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
                              <h4 className="text-xs font-bold uppercase tracking-wider mb-3 text-[var(--brand-text-muted)]">{t('admin.ordering_heatmap', 'Ordering Heatmap')}</h4>
                              <div className="flex flex-wrap gap-2">
                                {analyticsCache[c.id].heatmap?.length > 0 ? analyticsCache[c.id].heatmap.map((h: any, i: number) => (
                                  <div key={i} className="px-2 py-1 rounded text-xs font-medium" style={{ background: `rgba(var(--brand-primary-rgb, 59, 130, 246), ${Math.min(1, h.cnt / 5)})`, color: h.cnt > 2 ? 'white' : 'inherit' }}>
                                    {[t('client.day_sun', 'Sun'), t('client.day_mon', 'Mon'), t('client.day_tue', 'Tue'), t('client.day_wed', 'Wed'), t('client.day_thu', 'Thu'), t('client.day_fri', 'Fri'), t('client.day_sat', 'Sat')][h.dow]} {h.hour}:00 - {h.cnt} {t('admin.orders_lower', 'orders')}
                                  </div>
                                )) : <div className="text-sm text-[var(--brand-text-muted)]">{t('admin.not_enough_data', 'Not enough data for heatmap.')}</div>}
                              </div>
                            </div>
                          </div>
                        ) : null}
                      </td>
                    </tr>
                  )}
                </React.Fragment>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
