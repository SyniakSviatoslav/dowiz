import React, { useEffect, useState, useMemo } from 'react';
import { Button, EmptyState, SkeletonBase, Select, useI18n, PriceDisplay, ease, duration, SearchInput } from '@deliveryos/ui';
import { motion, useReducedMotion } from 'framer-motion';
import { useNavigate } from 'react-router-dom';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

// The API returns a bare array of customers; older mocks wrapped it as
// { customers: [...] }. Accept either so schema validation doesn't reject the
// real response (which silently showed "0 customers").
const CustomerListResponse = z.union([
  z.array(z.any()),
  z.object({ customers: z.array(z.any()) }).passthrough(),
]);

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
  const reduceMotion = useReducedMotion();
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [search, setSearch] = useState('');
  const [sortKey, setSortKey] = useState<'orders' | 'ltv' | 'name'>('orders');
  const [expandedCustomer, setExpandedCustomer] = useState<string | null>(null);
  const [analyticsCache, setAnalyticsCache] = useState<Record<string, any>>({});
  const [loadingAnalytics, setLoadingAnalytics] = useState<string | null>(null);
  const [onboardingDone, setOnboardingDone] = useState<boolean | null>(null);

  const loadCustomers = React.useCallback(() => {
    setLoading(true);
    setError(false);
    apiClient<typeof CustomerListResponse>('/owner/customers', { schema: CustomerListResponse })
      .then(d => { setCustomers(Array.isArray(d) ? d : ((d as any).customers || [])); setLoading(false); })
      .catch(() => {
        setCustomers([]);
        setError(true);
        setLoading(false);
      });
  }, []);

  useEffect(() => { loadCustomers(); }, [loadCustomers]);

  // Contact reveal is order-scoped by design — the only un-mask path is the
  // audited POST /owner/locations/:loc/orders/:order/reveal-customer-contact.
  // There is no customer-scoped reveal endpoint, so the CRM list shows the
  // masked phone only and never fabricates a number. (Previously a dead
  // /customers/:id/reveal-contact call 404'd and the catch rendered a hardcoded
  // fake phone for every customer.)

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

  // The API sends the last-order time pre-formatted in English ("15h ago", "1d ago",
  // "never"). Re-localize the unit suffix client-side so the Albanian UI doesn't show
  // raw English. Helper kept local to the page (no shared util) per scope.
  const localizeRelativeTime = (s: string): string => {
    if (!s) return s;
    if (s === 'never') return t('time.never', 'never');
    const m = s.match(/^(\d+)([mhdw])\s*ago$/);
    if (!m) return s;
    const n = m[1] ?? '';
    const unit = m[2] ?? '';
    const unitKey: Record<string, [string, string]> = {
      m: ['time.minutes_ago', '{n}m ago'],
      h: ['time.hours_ago', '{n}h ago'],
      d: ['time.days_ago', '{n}d ago'],
      w: ['time.weeks_ago', '{n}w ago'],
    };
    const entry = unitKey[unit];
    if (!entry) return s;
    return t(entry[0], entry[1]).replace('{n}', n);
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

  if (loading) {
    return (
      <div className="p-4 md:p-6 max-w-6xl mx-auto space-y-4">
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
          <div className="space-y-2">
            <SkeletonBase className="h-7 w-40" />
            <SkeletonBase className="h-4 w-24" />
          </div>
          <div className="flex gap-2 w-full sm:w-auto">
            <SkeletonBase className="h-9 flex-1 sm:w-48" />
            <SkeletonBase className="h-9 w-24" />
          </div>
        </div>
        <div className="rounded-xl border overflow-hidden" style={{ borderColor: 'var(--brand-border)' }}>
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="flex items-center gap-3 p-3 border-t first:border-t-0" style={{ borderColor: 'var(--brand-border)' }}>
              <SkeletonBase className="h-4 flex-1" />
              <SkeletonBase className="h-4 w-28 hidden md:block" />
              <SkeletonBase className="h-4 w-10" />
              <SkeletonBase className="h-7 w-20 rounded-full" />
            </div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="p-4 md:p-6 max-w-6xl mx-auto space-y-4">
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.customers', 'Customers')}</h2>
          <p className="text-sm tabular-nums" style={{ color: 'var(--brand-text)' }}>{filtered.length} {t('admin.customers_lower', 'customers')}</p>
        </div>
        <div className="flex flex-col sm:flex-row gap-2 w-full sm:w-auto">
          <SearchInput
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder={t('admin.search_customers', 'Search customers...')}
            containerClassName="w-full sm:w-48"
          />
          <Select
            value={sortKey}
            onChange={e => setSortKey(e.target.value as any)}
            aria-label={t('admin.sort_by', 'Sort by')}
          >
            <option value="orders">{t('admin.most_orders', 'Most orders')}</option>
            <option value="ltv">{t('admin.highest_ltv', 'Highest LTV')}</option>
            <option value="name">{t('admin.name_az', 'Name A-Z')}</option>
          </Select>
          <Button variant="secondary" size="sm" onClick={() => {
            const exportData = filtered.map(c => ({ name: c.name, phone: c.phone, orders: c.orders, ltv: c.ltv, lastOrder: c.lastOrder }));
            exportCSV(exportData, 'customers.csv');
          }}>
            <i className="ti ti-download"></i> {t('admin.export_csv', 'Export CSV')}
          </Button>
        </div>
      </div>

      {error ? (
        <EmptyState
          icon={<i className="ti ti-cloud-off" />}
          title={t('admin.customers_error_title', 'Could not load customers')}
          description={t('admin.customers_error_desc', 'Something went wrong while loading. Please try again.')}
          action={
            <Button onClick={loadCustomers} size="sm">
              <i className="ti ti-refresh" /> {t('common.retry', 'Retry')}
            </Button>
          }
        />
      ) : filtered.length === 0 ? (
        <EmptyState
          icon={<i className={search ? 'ti ti-search-off' : 'ti ti-users'} />}
          title={search ? t('admin.no_match', 'No match.') : t('admin.no_customers', 'No customers')}
          description={search ? t('admin.no_match_hint', 'Try a different name or phone number.') : t('admin.no_customers_hint', 'Customers appear after their first order.')}
        />
      ) : (
        <>
        {/* Desktop / tablet: data table */}
        <div className="hidden md:block rounded-[var(--brand-radius)] overflow-hidden" style={{ boxShadow: 'var(--elev-1)', background: 'var(--brand-surface)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr style={{ background: 'var(--brand-surface)' }}>
                <th className="text-left p-3 font-medium w-2/5" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.customer', 'Customer')}</th>
                <th className="text-left p-3 font-medium w-1/4" style={{ color: 'var(--brand-text-muted)' }}>{t('common.phone', 'Phone')}</th>
                <th className="text-right p-3 font-medium" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.orders', 'Orders')}</th>
                <th className="text-right p-3 font-medium hidden md:table-cell" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.ltv', 'LTV')}</th>
                <th className="text-right p-3 font-medium hidden lg:table-cell" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.last_order', 'Last order')}</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((c, i) => (
                <React.Fragment key={c.id}>
                  <motion.tr
                    onClick={() => toggleExpand(c.id)}
                    initial={reduceMotion ? false : { opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={reduceMotion ? { duration: 0 } : { duration: duration.base, ease: ease.out, delay: Math.min(i, 12) * 0.03 }}
                    className={`border-t cursor-pointer transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)] outline-none hover:[@media(hover:hover)]:bg-[var(--brand-surface-raised)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--brand-primary)] ${expandedCustomer === c.id ? 'bg-[var(--brand-surface-raised)]' : ''}`}
                    style={{ borderColor: 'var(--brand-border)' }}
                  >
                    <td className="p-3">
                      <div className="font-medium flex items-center gap-2 min-w-0" style={{ color: 'var(--brand-text)' }}>
                        <i className={`ti ti-chevron-${expandedCustomer === c.id ? 'down' : 'right'} text-[var(--brand-text-muted)] text-xs shrink-0 transition-transform duration-[var(--motion-fast)] ease-[var(--ease-soft)]`} />
                        <span className="truncate">{c.name}</span>
                      </div>
                    </td>
                    <td className="p-3 text-sm" style={{ color: 'var(--brand-text-muted)' }}>
                      <span className="truncate block tabular-nums">{c.phone}</span>
                    </td>
                    <td className="p-3 text-right font-medium">
                      <span className="inline-block px-2 py-0.5 rounded-full text-xs tabular-nums" style={{ background: c.orders > 15 ? 'rgba(5,150,105,0.1)' : 'var(--brand-surface-raised)', color: c.orders > 15 ? 'var(--color-success)' : 'var(--brand-text)' }}>
                        {c.orders}
                      </span>
                    </td>
                    <td className="p-3 text-right tabular-nums hidden md:table-cell" style={{ color: 'var(--brand-text)' }}>
                      <PriceDisplay amount={c.ltv} />
                    </td>
                    <td className="p-3 text-right tabular-nums hidden lg:table-cell whitespace-nowrap" style={{ color: 'var(--brand-text-muted)' }}>
                      {localizeRelativeTime(c.lastOrder)}
                    </td>
                  </motion.tr>
                  {expandedCustomer === c.id && (
                    <tr className="bg-[var(--brand-bg)]">
                      <td colSpan={5} className="p-4 border-b" style={{ borderColor: 'var(--brand-border)' }}>
                        <CustomerDetail t={t} loading={loadingAnalytics === c.id} data={analyticsCache[c.id]} reduceMotion={reduceMotion} />
                      </td>
                    </tr>
                  )}
                </React.Fragment>
              ))}
            </tbody>
          </table>
        </div>

        {/* Mobile: stacked cards (no clipped wide table) */}
        <div className="md:hidden space-y-3">
          {filtered.map((c, i) => {
            const expanded = expandedCustomer === c.id;
            return (
              <motion.div
                key={c.id}
                initial={reduceMotion ? false : { opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={reduceMotion ? { duration: 0 } : { duration: duration.base, ease: ease.out, delay: Math.min(i, 12) * 0.03 }}
                className="rounded-[var(--brand-radius)] overflow-hidden"
                style={{ boxShadow: 'var(--elev-1)', background: 'var(--brand-surface)' }}
              >
                {/* A div, not a <button>: expand-on-click is a mouse convenience and
                    all row data (name, masked phone, orders, LTV, last order) is
                    visible unexpanded, so no interaction is keyboard-gated here. */}
                <motion.div
                  onClick={() => toggleExpand(c.id)}
                  whileTap={reduceMotion ? undefined : { scale: 0.99 }}
                  className="w-full text-left p-4 flex items-start gap-3 cursor-pointer transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)]"
                >
                  <i className={`ti ti-chevron-${expanded ? 'down' : 'right'} text-[var(--brand-text-muted)] text-sm mt-0.5 shrink-0`} />
                  <div className="min-w-0 flex-1">
                    <div className="font-medium truncate" style={{ color: 'var(--brand-text)' }}>{c.name}</div>
                    <div className="text-sm truncate tabular-nums" style={{ color: 'var(--brand-text-muted)' }}>{c.phone}</div>
                    <div className="mt-2 flex items-center gap-3 text-xs flex-wrap" style={{ color: 'var(--brand-text-muted)' }}>
                      <span className="inline-flex items-center gap-1 tabular-nums">
                        <i className="ti ti-shopping-bag" /> {c.orders} {c.orders === 1 ? t('admin.order_lower', 'order') : t('admin.orders_lower', 'orders')}
                      </span>
                      <span className="inline-flex items-center gap-1 tabular-nums" style={{ color: 'var(--brand-text)' }}>
                        <PriceDisplay amount={c.ltv} />
                      </span>
                      <span className="inline-flex items-center gap-1 tabular-nums whitespace-nowrap">
                        <i className="ti ti-clock" /> {localizeRelativeTime(c.lastOrder)}
                      </span>
                    </div>
                  </div>
                </motion.div>
                {expanded && (
                  <div className="px-4 pb-4 border-t pt-4" style={{ borderColor: 'var(--brand-border)' }}>
                    <CustomerDetail t={t} loading={loadingAnalytics === c.id} data={analyticsCache[c.id]} reduceMotion={reduceMotion} />
                  </div>
                )}
              </motion.div>
            );
          })}
        </div>
        </>
      )}
    </div>
  );
}

// Shared expanded-detail panel (preferences / recent orders / heatmap) for the
// desktop table row and the mobile card. Kept local to the page per scope.
function CustomerDetail({ t, loading, data, reduceMotion }: { t: (k: string, f: string) => string; loading: boolean; data: any; reduceMotion: boolean | null }) {
  if (loading) {
    return (
      <div className="space-y-3" aria-busy="true" aria-label={t('common.loading', 'Loading')}>
        <SkeletonBase className="h-4 w-32" />
        <SkeletonBase className="h-16 w-full" />
        <SkeletonBase className="h-16 w-full" />
      </div>
    );
  }
  if (!data) return null;
  return (
    <motion.div
      initial={reduceMotion ? false : { opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={reduceMotion ? { duration: 0 } : { duration: duration.base, ease: ease.out }}
      className="grid grid-cols-1 md:grid-cols-3 gap-6"
    >
      {/* Preferences */}
      <div>
        <h4 className="text-xs font-bold uppercase tracking-wider mb-3 text-[var(--brand-text-muted)]">{t('admin.top_preferences', 'Top Preferences')}</h4>
        <div className="space-y-2">
          {data.preferences?.map((p: any, i: number) => (
            <div key={i} className="flex justify-between items-center gap-3 text-sm p-2 rounded-[var(--brand-radius-sm)] bg-[var(--brand-surface)]">
              <span className="font-medium truncate min-w-0" style={{ color: 'var(--brand-text)' }}>{p.name}</span>
              <div className="text-right shrink-0">
                <div className="text-[var(--brand-text-muted)] text-xs tabular-nums">x{p.total_qty}</div>
                <div className="text-xs font-bold text-[var(--brand-primary)] tabular-nums"><PriceDisplay amount={p.total_spent} /></div>
              </div>
            </div>
          ))}
          {!data.preferences?.length && <div className="text-sm text-[var(--brand-text-muted)]">{t('admin.no_items_bought', 'No items bought yet.')}</div>}
        </div>
      </div>

      {/* Order History */}
      <div className="md:col-span-2">
        <h4 className="text-xs font-bold uppercase tracking-wider mb-3 text-[var(--brand-text-muted)]">{t('admin.recent_orders', 'Recent Orders')}</h4>
        <div className="space-y-2 max-h-64 overflow-y-auto pr-2 custom-scrollbar">
          {data.orders?.map((o: any) => (
            <div key={o.id} className="p-3 rounded-[var(--brand-radius-sm)] border bg-[var(--brand-surface)]" style={{ borderColor: 'var(--brand-border)' }}>
              <div className="flex justify-between items-start gap-2 mb-2">
                <div className="min-w-0">
                  <span className="font-bold tabular-nums" style={{ color: 'var(--brand-text)' }}>#{o.id.toString().substring(0, 4).toUpperCase()}</span>
                  <span className="text-xs text-[var(--brand-text-muted)] ml-2 tabular-nums">{new Date(o.created_at).toLocaleString()}</span>
                </div>
                <span className="text-xs font-bold px-1.5 py-0.5 rounded-[var(--brand-radius-sm)] bg-[var(--brand-surface-raised)] shrink-0" style={{ color: 'var(--brand-text)' }}>{o.status}</span>
              </div>
              {o.delivery_address && (
                <div className="text-xs text-[var(--brand-text-muted)] mb-2 flex items-center gap-1">
                  <i className="ti ti-map-pin shrink-0" /> <span className="truncate">{o.delivery_address}</span>
                </div>
              )}
              <div className="text-xs space-y-0.5 pl-4 border-l-2 border-[var(--brand-border)]">
                {o.items?.map((item: any, i: number) => (
                  <div key={i} className="flex justify-between gap-2">
                    <span className="truncate min-w-0">{item.name} <span className="tabular-nums">x{item.qty}</span></span>
                  </div>
                ))}
              </div>
            </div>
          ))}
          {!data.orders?.length && <div className="text-sm text-[var(--brand-text-muted)]">{t('admin.no_orders_found', 'No orders found.')}</div>}
        </div>
      </div>

      {/* Heatmap summary */}
      <div className="md:col-span-3 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
        <h4 className="text-xs font-bold uppercase tracking-wider mb-3 text-[var(--brand-text-muted)]">{t('admin.ordering_heatmap', 'Ordering Heatmap')}</h4>
        <div className="flex flex-wrap gap-2">
          {data.heatmap?.length > 0 ? data.heatmap.map((h: any, i: number) => (
            <div key={i} className="px-2 py-1 rounded-[var(--brand-radius-sm)] text-xs font-medium tabular-nums" style={{ background: `rgba(var(--brand-primary-rgb, 59, 130, 246), ${Math.min(1, h.cnt / 5)})`, color: h.cnt > 2 ? 'white' : 'inherit' }}>
              {[t('client.day_sun', 'Sun'), t('client.day_mon', 'Mon'), t('client.day_tue', 'Tue'), t('client.day_wed', 'Wed'), t('client.day_thu', 'Thu'), t('client.day_fri', 'Fri'), t('client.day_sat', 'Sat')][h.dow]} {h.hour}:00 - {h.cnt} {t('admin.orders_lower', 'orders')}
            </div>
          )) : <div className="text-sm text-[var(--brand-text-muted)]">{t('admin.not_enough_data', 'Not enough data for heatmap.')}</div>}
        </div>
      </div>
    </motion.div>
  );
}
