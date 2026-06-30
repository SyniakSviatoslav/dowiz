import { useEffect, useState, useRef, useCallback } from 'react';
import { EmptyState, SkeletonBase, useI18n, PriceDisplay, AnimatedNumber, Button } from '@deliveryos/ui';
import Map, { Marker } from 'react-map-gl/maplibre';
import 'maplibre-gl/dist/maplibre-gl.css';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

const AnalyticsOverviewResponse = z.custom<AnalyticsData>();

const ProductOrdersResponse = z.array(z.object({
  id: z.string(),
  total: z.number(),
  currency_code: z.string(),
  created_at: z.string(),
  status: z.string(),
  customer_name: z.string(),
  quantity: z.number(),
  price: z.number(),
}));
import { exportCSV, exportJSON } from '../../lib/exportCSV.js';

interface AnalyticsData {
  revenue: { today: number; trend: string };
  orders: { today: number; trend: string };
  avgOrderValue: { value: number; trend: string };
  deliveryTime: { avg: number; trend: string };
  chart: Array<{ day: string; revenue: number }>;
  topProducts: Array<{ name: string; orders: number; revenue: number; imageUrl?: string }>;
  geoLocations?: Array<{ lat: number; lng: number }>;
  heatmap?: Array<{ day: string; hours: number[]; products: string[][] }>;
}

interface ProductOrder {
  id: string;
  total: number;
  currency_code: string;
  created_at: string;
  status: string;
  customer_name: string;
  quantity: number;
  price: number;
}

const HOUR_LABELS = ['0-3', '4-7', '8-11', '12-15', '16-19', '20-23'];

function SimpleBar({ value, maxValue, label, dayLabel, delay }: { value: number; maxValue: number; label: string; dayLabel: string; delay: number }) {
  const [height, setHeight] = useState(0);

  useEffect(() => {
    const timer = setTimeout(() => setHeight((value / maxValue) * 100), delay);
    return () => clearTimeout(timer);
  }, [value, maxValue, delay]);

  return (
    <div className="flex-1 flex flex-col items-center gap-1 h-full justify-end">
      <span className="text-step-2xs font-medium tabular-nums" style={{ color: 'var(--brand-text-muted)' }}>
        {label}
      </span>
      <div
        className="w-full rounded-t-md transition-[height,width,background-color] duration-500 ease-out"
        style={{
          height: `${height}%`,
          minHeight: 4,
          background: 'linear-gradient(to top, var(--brand-primary), var(--brand-primary-hover))',
        }}
      />
      <span className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>{dayLabel}</span>
    </div>
  );
}

export function AnalyticsPage() {
  const { t } = useI18n();
  const [data, setData] = useState<AnalyticsData | null>(null);
  const [loading, setLoading] = useState(true);
  const [period, setPeriod] = useState<'7d' | '30d'>('7d');
  const [copied, setCopied] = useState(false);

  const [error, setError] = useState(false);
  const CONSUMPTION_DATA = [
    { name: 'Salmon fillet', consumed: 12.5, unit: 'kg', ordered: 8, pct: 85 },
    { name: 'Sushi rice', consumed: 28, unit: 'kg', ordered: 4, pct: 65 },
    { name: 'Nori sheets', consumed: 240, unit: 'pcs', ordered: 60, pct: 50 },
    { name: 'Avocado', consumed: 35, unit: 'pcs', ordered: 10, pct: 40 },
    { name: 'Cream cheese', consumed: 6.2, unit: 'kg', ordered: 3, pct: 70 },
    { name: 'Spicy mayo', consumed: 4.5, unit: 'L', ordered: 2, pct: 30 },
    { name: 'Takeout boxes', consumed: 126, unit: 'pcs', ordered: 126, pct: 100 },
    { name: 'Chopsticks', consumed: 252, unit: 'pcs', ordered: 126, pct: 100 },
  ];
  const [expandedProduct, setExpandedProduct] = useState<string | null>(null);
  const [productOrders, setProductOrders] = useState<ProductOrder[]>([]);
  const [productOrdersLoading, setProductOrdersLoading] = useState(false);

  useEffect(() => {
    setLoading(true);
    setError(false);
    apiClient<typeof AnalyticsOverviewResponse>(`/owner/analytics?period=${period}`, { schema: AnalyticsOverviewResponse })
      .then(d => { setData(d); setLoading(false); })
      .catch(() => {
        setError(true);
        setLoading(false);
      });
  }, [period]);

  const toggleProduct = async (name: string) => {
    if (expandedProduct === name) {
      setExpandedProduct(null);
      setProductOrders([]);
      return;
    }
    setExpandedProduct(name);
    setProductOrdersLoading(true);
    try {
      const data = await apiClient<typeof ProductOrdersResponse>(`/owner/analytics/product-orders?name=${encodeURIComponent(name)}`, { schema: ProductOrdersResponse });
      setProductOrders(Array.isArray(data) ? data : []);
    } catch (err) {
      console.error('[AnalyticsPage] fetch product orders failed:', err);
      setProductOrders([]);
    } finally {
      setProductOrdersLoading(false);
    }
  };

  const handleCopyReorder = useCallback(() => {
    const list = data?.topProducts?.map(p => p.name).join('\n') || '';
    navigator.clipboard.writeText(list).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    });
  }, [data]);

  if (loading) return (
    <div className="p-4 md:p-6 max-w-7xl mx-auto space-y-6">
      <div className="flex items-center justify-between gap-3">
        <div className="space-y-2">
          <SkeletonBase className="h-7 w-32" />
          <SkeletonBase className="h-3 w-48" />
        </div>
        <SkeletonBase className="h-8 w-24 rounded-lg shrink-0" />
      </div>
      <div className="flex items-center justify-between">
        <SkeletonBase className="h-4 w-20" />
        <SkeletonBase className="h-6 w-24 rounded-md" />
      </div>
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        {[1, 2, 3, 4].map(i => <SkeletonBase key={i} className="h-24 rounded-xl" />)}
      </div>
      <SkeletonBase className="h-64 rounded-xl" />
    </div>
  );

  // Distinguish a failed request from a genuine "no orders" result. A caught error leaves
  // `data` null; without this branch the user would see the "no orders" copy even though the
  // request failed — which contradicts a dashboard that shows live orders.
  if (error) return (
    <div className="p-4 md:p-6 max-w-7xl mx-auto">
      <EmptyState
        title={t('admin.analytics_unavailable', 'Analytics unavailable')}
        description={t('admin.analytics_error_hint', "We couldn't load analytics right now. Your orders are safe — please try again.")}
        icon={<i className="ti ti-alert-triangle text-4xl opacity-30" />}
        action={
          <button
            onClick={() => setPeriod(p => p)}
            className="px-3 py-1.5 text-xs font-medium rounded-md bg-[var(--brand-primary)] text-[var(--brand-bg)] transition-[background-color,transform,box-shadow] duration-200 active:scale-[0.97] hover:bg-[var(--brand-primary-hover)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-bg)]"
          >
            {t('common.retry', 'Retry')}
          </button>
        }
      />
    </div>
  );

  // Honest empty copy: analytics is period-scoped (last 7 days), so older orders won't appear
  // here even when the all-time dashboard shows them. Surface the period selector so the user
  // understands the window and can switch it.
  if (!data) return (
    <div className="p-4 md:p-6 max-w-7xl mx-auto space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-2xl font-bold truncate" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.analytics', 'Analytics')}</h2>
          <p className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.analytics_period_note', 'Showing the selected period only — older orders are not counted here.')}</p>
        </div>
        <div className="flex rounded-lg p-0.5 shrink-0" style={{ background: 'var(--brand-surface-raised)' }}>
          {(['7d', '30d'] as const).map(p => (
            <button
              key={p}
              onClick={() => setPeriod(p)}
              className={`px-3 py-1.5 text-xs font-medium rounded-md transition-[background-color,color,transform,box-shadow] duration-200 active:scale-[0.97] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface-raised)] ${period === p ? 'bg-[var(--brand-primary)] text-[var(--brand-bg)] shadow-sm' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}
            >
              {p === '7d' ? t('admin.7_days', '7 days') : t('admin.30_days', '30 days')}
            </button>
          ))}
        </div>
      </div>
      <EmptyState title={t('admin.no_data', 'No data')} description={t('admin.analytics_empty_period_hint', 'No orders in this period. Try a wider range, or check back once new orders come in.')} icon={<i className="ti ti-chart-bar text-4xl opacity-30" />} />
    </div>
  );

  const maxRevenue = Math.max(...data.chart.map(c => c.revenue), 1);
  const heatmapData = data.heatmap || [
    { day: 'Mon', hours: [0,0,0,0,0,0], products: [[],[],[],[],[],[]] },
    { day: 'Tue', hours: [0,0,0,0,0,0], products: [[],[],[],[],[],[]] },
    { day: 'Wed', hours: [0,0,0,0,0,0], products: [[],[],[],[],[],[]] },
    { day: 'Thu', hours: [0,0,0,0,0,0], products: [[],[],[],[],[],[]] },
    { day: 'Fri', hours: [0,0,0,0,0,0], products: [[],[],[],[],[],[]] },
    { day: 'Sat', hours: [0,0,0,0,0,0], products: [[],[],[],[],[],[]] },
    { day: 'Sun', hours: [0,0,0,0,0,0], products: [[],[],[],[],[],[]] },
  ];
  const heatmapMax = Math.max(...heatmapData.flatMap(d => d.hours), 1);

  const compactFmt = new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 });
  const statCards = [
    { label: t('admin.revenue', 'Revenue'), num: Math.round(data.revenue.today), format: (v: number) => compactFmt.format(v), value: compactFmt.format(Math.round(data.revenue.today)), trend: data.revenue.trend, icon: 'ti ti-wallet', colorVar: '--color-success', noData: false },
    { label: t('admin.orders', 'Orders'), num: data.orders.today, format: (v: number) => v.toString(), value: data.orders.today.toString(), trend: data.orders.trend, icon: 'ti ti-shopping-cart', colorVar: '--color-info', noData: false },
    { label: t('admin.avg_order', 'Avg Order'), num: data.avgOrderValue.value, format: (v: number) => String(v), value: String(data.avgOrderValue.value), trend: data.avgOrderValue.trend, icon: 'ti ti-receipt', colorVar: '--status-scheduled', noData: false },
    { label: t('admin.delivery_time', 'Delivery'), num: data.deliveryTime.avg, format: (v: number) => `${v} ${t('admin.min', 'min')}`, value: `${data.deliveryTime.avg} ${t('admin.min', 'min')}`, trend: data.deliveryTime.trend, icon: 'ti ti-truck-delivery', colorVar: '--color-warning',
      // A 0-minute average delivery is impossible — it means no completed deliveries yet.
      // Show a "no data" state instead of "0 min" + a meaningless −x% delta off a zero baseline.
      noData: data.deliveryTime.avg === 0 },
  ];

  return (
    <div className="p-4 md:p-6 max-w-7xl mx-auto space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-2xl font-bold truncate" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.analytics', 'Analytics')}</h2>
          <p className="text-xs mt-0.5 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.analytics_desc', 'Performance overview for your restaurant')}</p>
        </div>
        <div className="flex rounded-lg p-0.5 shrink-0" style={{ background: 'var(--brand-surface-raised)' }}>
          {(['7d', '30d'] as const).map(p => (
            <button
              key={p}
              onClick={() => setPeriod(p)}
              className={`px-3 py-1.5 text-xs font-medium rounded-md transition-[background-color,color,transform,box-shadow] duration-200 active:scale-[0.97] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface-raised)] ${period === p ? 'bg-[var(--brand-primary)] text-[var(--brand-bg)] shadow-sm' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}
            >
              {p === '7d' ? t('admin.7_days', '7 days') : t('admin.30_days', '30 days')}
            </button>
          ))}
        </div>
      </div>

      {/* Stat cards */}
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>{t('admin.overview', 'Overview')}</h3>
        <div className="flex items-center gap-2">
          <Button variant="secondary" size="sm" onClick={() => exportCSV(statCards.map(c => ({ Metric: c.label, Value: c.value, Trend: c.trend })), 'analytics-stats.csv')}>
            <i className="ti ti-download" /> {t('admin.export_csv', 'Export CSV')}
          </Button>
          <Button variant="secondary" size="sm" onClick={() => exportJSON(statCards.map(c => ({ Metric: c.label, Value: c.value, Trend: c.trend })), 'analytics-stats.json')} title={t('tooltip.export_json', 'Export as JSON')}>
            <i className="ti ti-braces" /> {t('admin.export_json', 'Export JSON')}
          </Button>
        </div>
      </div>
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 stagger-children">
        {statCards.map((card, i) => (
          <div
            key={i}
            data-testid="kpi-card"
            className="p-4 rounded-xl card-lift breathe min-w-0 transition-shadow"
            style={{ background: 'var(--brand-surface)', boxShadow: 'var(--elev-1)', animationDelay: `${i * 0.3}s`, transitionDuration: 'var(--motion-fast)', transitionTimingFunction: 'var(--ease-soft)' }}
          >
            <div className="flex items-center justify-between gap-2 mb-3">
              <span className="text-xs font-medium truncate" style={{ color: 'var(--brand-text-muted)' }}>{card.label}</span>
              <i className={`${card.icon} text-lg shrink-0`} style={{ color: `var(${card.colorVar})` }} />
            </div>
            <div data-testid="kpi-value" className="text-xl font-bold mb-1 tabular-nums truncate" style={{ color: card.noData ? 'var(--brand-text-muted)' : 'var(--brand-text)' }}>
              {card.noData ? '—' : <AnimatedNumber value={card.num} formatter={card.format} />}
            </div>
            <span className="text-xs font-medium tabular-nums" style={{ color: card.noData ? 'var(--brand-text-muted)' : card.trend.startsWith('+') ? 'var(--color-success)' : card.trend.startsWith('-') ? 'var(--color-danger)' : 'var(--brand-text-muted)' }}>
              {card.noData ? t('admin.no_data_yet', 'No data yet') : `${card.trend} ${t('admin.vs_last_period', 'vs last period')}`}
            </span>
          </div>
        ))}
      </div>

      {/* Revenue chart */}
      <div className="p-5 rounded-xl border border-glow" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
        <div className="flex items-center justify-between gap-3 mb-4 flex-wrap">
          <h3 className="text-sm font-semibold shrink-0" style={{ color: 'var(--brand-text)' }}>{t('admin.revenue_trend', 'Revenue Trend')}</h3>
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-step-2xs px-2 py-0.5 rounded-full tabular-nums whitespace-nowrap" style={{ background: 'var(--brand-primary-light)', color: 'var(--brand-primary-readable)' }}>
              {t('admin.total', 'Total:')} <PriceDisplay amount={data.chart.reduce((s, c) => s + c.revenue, 0)} />
            </span>
            <span className="text-step-2xs px-2 py-0.5 rounded-full tabular-nums whitespace-nowrap" style={{ background: 'var(--color-success-light)', color: 'var(--color-success)' }}>
              {t('admin.avg', 'Avg:')} <PriceDisplay amount={Math.round(data.chart.reduce((s, c) => s + c.revenue, 0) / data.chart.length)} />
            </span>
          </div>
        </div>
        <div className="flex items-end gap-2 h-48">
          {data.chart.map((item, idx) => (
            <SimpleBar
              key={item.day}
              value={item.revenue}
              maxValue={maxRevenue}
              label={`${(item.revenue / 1000).toFixed(0)}k`}
              dayLabel={item.day}
              delay={idx * 60}
            />
          ))}
        </div>
      </div>

      {/* Two-column layout for Top Products + Consumption */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Top products */}
        <div className="p-5 rounded-xl border border-glow" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>{t('admin.top_products', 'Top Products')}</h3>
            <div className="flex items-center gap-2">
              <Button variant="secondary" size="sm" onClick={() => exportCSV(data.topProducts.map(p => ({ Product: p.name, Orders: p.orders, Revenue: p.revenue })), 'top-products.csv')}>
                <i className="ti ti-download" /> {t('admin.export_csv', 'Export CSV')}
              </Button>
              <Button variant="secondary" size="sm" onClick={() => exportJSON(data.topProducts.map(p => ({ Product: p.name, Orders: p.orders, Revenue: p.revenue })), 'top-products.json')} title={t('tooltip.export_json', 'Export as JSON')}>
                <i className="ti ti-braces" /> {t('admin.export_json', 'Export JSON')}
              </Button>
            </div>
          </div>
          <div className="space-y-1">
            {data.topProducts.map((p, i) => {
              const firstRevenue = data.topProducts[0]?.revenue ?? p.revenue;
              const barPct = firstRevenue > 0 ? Math.round((p.revenue / firstRevenue) * 100) : 100;
              const isExpanded = expandedProduct === p.name;
              return (
                <div key={p.name}>
                  <button
                    type="button"
                    className="flex items-center gap-3 py-2 px-3 rounded-lg hover:bg-[var(--brand-surface-raised)] transition-colors slide-in-right cursor-pointer w-full text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
                    style={{ animationDelay: `${i * 50}ms` }}
                    onClick={() => toggleProduct(p.name)}
                  >
                    <div className="w-10 h-10 rounded-lg flex items-center justify-center shrink-0 overflow-hidden" style={{ background: 'var(--brand-primary-light)' }}>
                      {p.imageUrl ? (
                        <img src={p.imageUrl} alt="" className="w-full h-full object-cover" />
                      ) : i === 0 ? (
                        <i className="ti ti-crown" style={{ color: 'var(--color-warning)' }} />
                      ) : (
                        <i className="ti ti-tools-kitchen-2" style={{ color: 'var(--brand-primary)' }} />
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <div className="text-sm font-medium truncate">{p.name}</div>
                        {i === 0 && <span className="text-step-2xs px-1.5 py-0.5 rounded font-mono" style={{ background: 'var(--brand-primary-light)', color: 'var(--brand-primary-readable)' }}>#1</span>}
                      </div>
                      <div className="h-1 rounded-full" style={{ background: 'var(--brand-border)' }}>
                        <div className="h-full rounded-full progress-animate" style={{ width: `${barPct}%`, background: 'var(--brand-primary)', opacity: 0.3 + (barPct / 100) * 0.7 }} />
                      </div>
                      <div className="flex justify-between mt-1">
                        <span className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>{p.orders} {t('admin.orders', 'orders').toLowerCase()}</span>
                      </div>
                    </div>
                    <div className="flex items-center gap-2 shrink-0">
                      <div className="text-sm font-semibold text-right" style={{ color: 'var(--brand-primary-readable)' }}>
                        <PriceDisplay amount={p.revenue} />
                      </div>
                      <i className={`ti ${isExpanded ? 'ti-chevron-up' : 'ti-chevron-down'} text-xs text-[var(--brand-text-muted)]`} />
                    </div>
                  </button>
                  {isExpanded && (
                    <div className="px-3 pb-2">
                      {productOrdersLoading ? (
                        <div className="animate-pulse space-y-2 py-2">
                          {[1,2,3].map(j => <div key={j} className="h-8 bg-[var(--brand-surface)] rounded" />)}
                        </div>
                      ) : productOrders.length === 0 ? (
                        <div className="text-xs py-2 text-center" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.no_orders', 'No orders found')}</div>
                      ) : (
                        <div className="max-h-48 overflow-auto space-y-1 pt-1">
                          {productOrders.map(o => (
                            <div key={o.id} className="flex items-center justify-between px-3 py-2 rounded-lg text-xs" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)' }}>
                              <div className="flex items-center gap-2 min-w-0">
                                <span className="font-mono text-step-2xs text-[var(--brand-text-muted)] shrink-0">{o.id.slice(0, 8)}</span>
                                <span className="truncate">{o.customer_name}</span>
                                <span className="text-step-2xs px-1 py-0.5 rounded tabular-nums shrink-0" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>x{o.quantity}</span>
                              </div>
                              <div className="flex items-center gap-2 shrink-0">
                                <span className="font-medium"><PriceDisplay amount={o.price} /></span>
                                <span data-dynamic className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>{new Date(o.created_at).toLocaleDateString()}</span>
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Consumption report */}
        <div className="p-5 rounded-xl border border-glow" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>
              {t('admin.ingredient_consumption', 'Ingredient Consumption')} <span className="text-step-2xs font-normal opacity-50">({t('admin.derived', 'derived')})</span>
            </h3>
            <div className="flex items-center gap-2">
              <Button variant="secondary" size="sm" onClick={() => exportCSV(CONSUMPTION_DATA, 'consumption.csv')}>
                <i className="ti ti-download" /> {t('admin.export_csv', 'Export CSV')}
              </Button>
              <Button variant="secondary" size="sm" onClick={() => exportJSON(CONSUMPTION_DATA, 'consumption.json')} title={t('tooltip.export_json', 'Export as JSON')}>
                <i className="ti ti-braces" /> {t('admin.export_json', 'Export JSON')}
              </Button>
            </div>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            {CONSUMPTION_DATA.map((item, i) => (
              <div
                key={item.name}
                className="flex items-center justify-between p-3 rounded-lg border slide-in-up"
                style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface-raised)', animationDelay: `${i * 60}ms` }}
              >
                <div className="min-w-0 flex-1">
                  <div className="text-xs font-medium truncate">{item.name}</div>
                  <div className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>{item.consumed} {item.unit}</div>
                  <div className="w-full h-1 rounded-full mt-1.5" style={{ background: 'var(--brand-border)' }}>
                    <div
                      className="h-full rounded-full progress-animate"
                      style={{
                        width: `${item.pct}%`,
                        background: item.pct > 80 ? 'var(--color-warning)' : 'var(--color-success)',
                        transitionDelay: `${i * 80}ms`,
                      }}
                    />
                  </div>
                </div>
                {item.pct > 80 && (
                  <span className="text-step-2xs ml-2 px-1.5 py-0.5 rounded font-medium shrink-0 animate-pulse" style={{ background: 'var(--color-warning-light, color-mix(in srgb, var(--color-warning) 15%, transparent))', color: 'var(--color-warning)' }}>
                    {t('admin.reorder', 'Reorder')}
                  </span>
                )}
              </div>
            ))}
          </div>
          <p className="text-step-2xs mt-3" style={{ color: 'var(--brand-text-muted)' }}>
            {t('admin.consumption_hint', 'Based on today\'s orders x recipe quantities. Estimates only.')}
          </p>
          <button
            onClick={handleCopyReorder}
            className="flex items-center gap-1.5 mt-2 px-3 py-1.5 text-step-2xs font-medium rounded-lg border transition-[background-color,transform,box-shadow] duration-200 hover:bg-[var(--brand-surface-raised)] active:scale-[0.97] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
            style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-primary-readable)' }}
          >
            {copied ? (
              <><i className="ti ti-check" /> {t('common.copied', 'Copied!')}</>
            ) : (
              <><i className="ti ti-clipboard" /> {t('admin.copy_reorder', 'Copy Reorder List')}</>
            )}
          </button>
        </div>
      </div>

      {/* Order Heatmap */}
      <div className="p-5 rounded-xl border border-glow" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>{t('admin.order_heatmap', 'Order Heatmap')}</h3>
          <div className="flex items-center gap-2">
            <span className="flex items-center gap-1 text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>
              <span className="w-2 h-2 rounded-sm" style={{ background: 'var(--brand-surface-raised)' }} /> {t('admin.low', 'Low')}
            </span>
            <span className="flex items-center gap-1 text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>
              <span className="w-2 h-2 rounded-sm" style={{ background: 'var(--brand-primary)' }} /> {t('admin.peak', 'Peak')}
            </span>
          </div>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-xs">
            <thead>
              <tr>
                <th className="text-left font-medium py-1 pr-3" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.day', 'Day')}</th>
                {HOUR_LABELS.map(label => (
                  <th key={label} className="text-center font-medium py-1 px-1" style={{ color: 'var(--brand-text-muted)' }}>{label}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {heatmapData.map(row => (
                <tr key={row.day}>
                  <td className="text-left font-medium py-1 pr-3" style={{ color: 'var(--brand-text)' }}>{row.day}</td>
                  {row.hours.map((count, ci) => {
                    const products = row.products?.[ci] || [];
                    const productList = products.length > 0 ? products.slice(0, 5).join(', ') + (products.length > 5 ? ` +${products.length - 5} more` : '') : '';
                    return (
                      <td key={ci} className="p-0.5 relative group">
                        <div
                          className="rounded-sm transition-transform duration-300 [@media(hover:hover)]:hover:scale-110 cursor-default"
                          title={productList ? `${row.day} ${HOUR_LABELS[ci]}: ${count} orders\nProducts: ${productList}` : `${row.day} ${HOUR_LABELS[ci]}: ${count} orders`}
                          style={{
                            minHeight: 28,
                            minWidth: 32,
                            background: count === heatmapMax
                              ? 'color-mix(in srgb, var(--brand-primary) 90%, transparent)'
                              : count > 0
                                ? `color-mix(in srgb, var(--brand-primary) ${Math.round((0.1 + (count / heatmapMax) * 0.8) * 100)}%, transparent)`
                                : 'var(--brand-surface-raised)',
                            ...(count === heatmapMax ? { border: '1px solid var(--brand-primary)', boxShadow: '0 0 8px color-mix(in srgb, var(--brand-primary) 30%, transparent)' } : {}),
                          }}
                        >
                          {count > 0 && (
                            <div className="hidden group-hover:block absolute bottom-full left-1/2 -translate-x-1/2 mb-1 px-2 py-1 rounded text-step-2xs whitespace-nowrap z-10 pointer-events-none shadow-lg"
                              style={{ background: 'var(--brand-bg)', border: '1px solid var(--brand-border)', color: 'var(--brand-text)' }}>
                              <div className="font-semibold tabular-nums">{count} {count === 1 ? t('admin.order_one', 'order') : t('admin.order_other', 'orders')}</div>
                              {productList && <div className="text-[var(--brand-text-muted)] max-w-[200px] truncate">{productList}</div>}
                            </div>
                          )}
                        </div>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Geo-Analytics */}
      <div className="p-5 rounded-xl border border-glow" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
        <h3 className="text-sm font-semibold mb-4" style={{ color: 'var(--brand-text)' }}>{t('admin.delivery_heatmap', 'Delivery Heatmap (Last 7 Days)')}</h3>
        <div className="h-64 rounded-xl overflow-hidden">
          <Map
            initialViewState={{
              longitude: 19.8187,
              latitude: 41.3275,
              zoom: 12
            }}
            mapStyle="https://basemaps.cartocdn.com/gl/positron-gl-style/style.json"
            interactive={true}
          >
            {data.geoLocations?.map((loc, i) => (
              <Marker key={i} longitude={loc.lng} latitude={loc.lat}>
                <div className="w-3 h-3 rounded-full bg-[var(--brand-primary)] opacity-60 shadow-lg" />
              </Marker>
            ))}
          </Map>
        </div>
      </div>
    </div>
  );
}
