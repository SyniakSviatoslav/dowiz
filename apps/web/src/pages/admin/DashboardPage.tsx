import React, { useEffect, useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import { OrderCard, EmptyState, CourierLiveMap, HintCard, useI18n, MobilePicker, useIsMobile, AnimatedNumber, LiveDot, useHaptics, useSoundPrefs, PullToRefresh } from '@deliveryos/ui';
import type { AdminOrder, CourierOnMap, LngLatLike, PickerOption } from '@deliveryos/ui';
import { apiClient, useWebSocket, useSound } from '../../lib/index.js';
import { exportCSV } from '../../lib/exportCSV.js';

export function DashboardPage() {
  const [orders, setOrders] = useState<AdminOrder[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [courierPositions, setCourierPositions] = useState<Record<string, LngLatLike>>({});
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [sortBy, setSortBy] = useState<'newest' | 'oldest' | 'highest'>('newest');
  const [viewMode, setViewMode] = useState<'live' | 'history'>('live');
  const [showHint, setShowHint] = useState(() => localStorage.getItem('dos_dash_hint_dismissed') !== '1');
  const { t } = useI18n();
  const isMobile = useIsMobile();
  const [clientSlug, setClientSlug] = useState('');
  const [sortPickerOpen, setSortPickerOpen] = useState(false);
  const [copiedLink, setCopiedLink] = useState(false);
  const [readiness, setReadiness] = useState<{ menu: boolean; phone: boolean; address: boolean; couriers: boolean; branding: boolean; placeOrder: boolean }>({ menu: false, phone: false, address: false, couriers: false, branding: false, placeOrder: false });

  const { play: playPing } = useSound('/sounds/ping.mp3');
  const { trigger: haptic } = useHaptics();
  const { alertSoundEnabled } = useSoundPrefs();
  const [tenantId, setTenantId] = useState('');
  const [expandedMessages, setExpandedMessages] = useState<Set<string>>(new Set());
  const [messagesByOrder, setMessagesByOrder] = useState<Record<string, any[]>>({});

  const fetchOrders = async () => {
    try {
      setLoading(true);
      const data = await apiClient<any>('/owner/orders');
      setOrders(Array.isArray(data) ? data : []);
    } catch (err: any) {
      if (err.status === 404) {
        setOrders([
          { id: 'o_1', status: 'PENDING', createdAt: new Date().toISOString(), items: [{ name: 'Burger', quantity: 2 }], total: 130000, customerName: 'Sara', shortId: '#2301', itemCount: 2, itemsSummary: 'Burger x2', customerPhone: '+355...', etaMinutes: null, elapsedSeconds: 60, courierName: null },
          { id: 'o_2', status: 'PREPARING', createdAt: new Date(Date.now() - 600000).toISOString(), items: [{ name: 'Pizza', quantity: 1 }], total: 85000, customerName: 'Alina', shortId: '#2300', itemCount: 1, itemsSummary: 'Pizza x1', customerPhone: '+355...', etaMinutes: null, elapsedSeconds: 600, courierName: 'Ardit' },
          { id: 'o_3', status: 'IN_DELIVERY', createdAt: new Date(Date.now() - 1200000).toISOString(), items: [{ name: 'Sushi', quantity: 3 }], total: 210000, customerName: 'Bled', shortId: '#2299', itemCount: 3, itemsSummary: 'Sushi x3', customerPhone: '+355...', etaMinutes: 14, elapsedSeconds: 1200, courierName: 'Ardit' },
        ]);
      } else {
        setError('Failed to load active orders');
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchOrders();
    apiClient<any>('/owner/settings').then(res => {
      if (res.id) setTenantId(res.id);
      if (res.locationName) {
        const generated = res.locationName.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 50);
        setClientSlug(generated);
      }
      // Compute readiness from actual data
      const r = { menu: false, phone: false, address: false, couriers: false, branding: false, placeOrder: false };
      r.phone = !!(res.phone && res.phone.length > 5);
      r.address = !!(res.address && res.address.length > 5);
      r.branding = !!(res.locationName && res.locationName.length > 2);
      setReadiness(r);
    }).catch(() => {});
    // Check menu + couriers in parallel
    Promise.all([
      apiClient<any>('/owner/menu/categories').catch(() => []),
      apiClient<any>('/owner/couriers').catch(() => []),
    ]).then(([cats, couriers]) => {
      setReadiness(prev => ({
        ...prev,
        menu: Array.isArray(cats) && cats.length > 0,
        couriers: Array.isArray(couriers) && couriers.length > 0,
      }));
    }).catch(() => {});
  }, []);

  useWebSocket({
    room: `location:${tenantId}:dashboard`,
    enabled: true,
    onMessage: (msg) => {
      const inner = msg?.data?.data || msg?.data || msg;
      if (inner.type === 'order.created') {
        setOrders(prev => [inner.data || inner, ...prev]);
        if (alertSoundEnabled) playPing();
        haptic('tap');
      }
      else if (inner.type === 'order.status') { setOrders(prev => prev.map(o => o.id === (inner.data?.orderId || inner.orderId) ? { ...o, ...(inner.data || inner) } : o)); }
      else if (inner.type === 'courier_position') { setCourierPositions(prev => ({ ...prev, [inner.courierId]: [inner.lng, inner.lat] })); }
    },
    onReconnect: () => { fetchOrders(); },
  });

  const fetchMessages = async (orderId: string) => {
    if (messagesByOrder[orderId]) return;
    try {
      const data = await apiClient<any>(`/orders/${orderId}/messages`);
      setMessagesByOrder(prev => ({ ...prev, [orderId]: Array.isArray(data) ? data : [] }));
    } catch { /* ignore */ }
  };

  const handleSendMessage = async (orderId: string, presetKey: string, params?: Record<string, unknown>) => {
    try {
      const msg = await apiClient(`/orders/${orderId}/messages`, { method: 'POST', body: { presetKey, params } });
      setMessagesByOrder(prev => ({ ...prev, [orderId]: [...(prev[orderId] || []), msg] }));
    } catch { /* ignore */ }
  };

  const handleToggleMessages = (orderId: string) => {
    setExpandedMessages(prev => {
      const next = new Set(prev);
      if (next.has(orderId)) next.delete(orderId);
      else { next.add(orderId); fetchMessages(orderId); }
      return next;
    });
  };

  const handleUpdateStatus = async (id: string, newStatus: string) => {
    setOrders(prev => prev.map(o => o.id === id ? { ...o, status: newStatus as AdminOrder['status'] } : o));
    try {
      await apiClient(`/orders/${id}/status`, { method: 'PATCH', body: { status: newStatus } });
    } catch {
      fetchOrders();
    }
  };

  const couriersOnMap: CourierOnMap[] = useMemo(() => [
    { id: 'cu1', name: 'Ardit', initials: 'AK', lngLat: courierPositions['cu1'] || [19.820, 41.333], status: 'busy' },
    { id: 'cu2', name: 'Blerim', initials: 'BH', lngLat: courierPositions['cu2'] || [19.810, 41.329], status: 'online' },
  ], [courierPositions]);

  const filteredOrders = useMemo(() => {
    let result = orders;
    if (viewMode === 'live') {
      result = result.filter(o => o.status !== 'DELIVERED' && o.status !== 'CANCELLED');
    } else {
      result = result.filter(o => o.status === 'DELIVERED' || o.status === 'CANCELLED');
    }

    if (search) {
      const q = search.toLowerCase();
      result = result.filter(o => o.customerName?.toLowerCase().includes(q) || o.shortId?.toLowerCase().includes(q) || o.itemsSummary?.toLowerCase().includes(q));
    }
    if (statusFilter !== 'all') {
      result = result.filter(o => o.status === statusFilter);
    }
    result.sort((a, b) => {
      if (sortBy === 'newest') return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
      if (sortBy === 'oldest') return new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
      return (b.total || 0) - (a.total || 0);
    });
    return result;
  }, [orders, search, statusFilter, sortBy, viewMode]);

  const STATUSES = viewMode === 'live' 
    ? ['all', 'PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY']
    : ['all', 'DELIVERED', 'CANCELLED'];

  const readinessItems = [
    { label: t('admin.menu', 'Menu'), done: readiness.menu, icon: 'ti ti-tools-kitchen-2' },
    { label: t('auth.phone', 'Phone number'), done: readiness.phone, icon: 'ti ti-phone' },
    { label: t('checkout.delivery_address', 'Delivery address'), done: readiness.address, icon: 'ti ti-map-pin' },
    { label: t('admin.couriers', 'Couriers'), done: readiness.couriers, icon: 'ti ti-motorbike' },
    { label: t('admin.branding', 'Branding'), done: readiness.branding, icon: 'ti ti-palette' },
    { label: t('checkout.place_order', 'Place order'), done: orders.length > 0, icon: 'ti ti-shopping-cart' },
    { label: t('checkout.payment_method', 'Payment method'), done: true, icon: 'ti ti-cash' },
  ];
  const doneCount = readinessItems.filter(r => r.done).length;
  const totalChecks = readinessItems.length;

  const stats = {
    pending: filteredOrders.filter(o => o.status === 'PENDING').length,
    inProgress: filteredOrders.filter(o => o.status === 'PREPARING' || o.status === 'CONFIRMED').length,
    ready: filteredOrders.filter(o => o.status === 'READY').length,
    inDelivery: filteredOrders.filter(o => o.status === 'IN_DELIVERY').length,
    revenue: filteredOrders.reduce((sum, o) => sum + (o.total || 0), 0),
  };

  return (
    <PullToRefresh onRefresh={fetchOrders}>
    <div className="p-4 md:p-6 max-w-7xl mx-auto space-y-6">
      {/* Welcome Hint */}
      {showHint && (
        <HintCard
          title={t('admin.welcome_dashboard', 'Welcome to your Dashboard')}
          description={t('admin.dashboard_hint', 'Here you can manage incoming orders, track couriers, and monitor your store readiness. Use the sidebar to navigate between sections.')}
          icon="ti ti-info-circle"
          onDismiss={() => { setShowHint(false); localStorage.setItem('dos_dash_hint_dismissed', '1'); }}
        />
      )}

      {/* Quick Stats Row */}
      <div className="grid grid-cols-3 sm:grid-cols-5 gap-2 stagger-children">
        {[
          { label: t('order.pending', 'Pending'), value: stats.pending, color: 'var(--status-pending)', isCurrency: false },
          { label: t('order.preparing', 'Preparing'), value: stats.inProgress, color: 'var(--status-preparing)', isCurrency: false },
          { label: t('order.ready', 'Ready'), value: stats.ready, color: 'var(--status-ready)', isCurrency: false },
          { label: t('order.in_delivery', 'Delivery'), value: stats.inDelivery, color: 'var(--status-in-delivery)', isCurrency: false },
          { label: t('cart.total', 'Revenue'), value: stats.revenue, color: 'var(--brand-primary)', isCurrency: true },
        ].map((stat, i) => (
          <motion.div
            key={stat.label}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.04, duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
            className="text-center p-3 rounded-xl" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)' }}
          >
            <div className="text-2xl font-bold mb-0.5" style={{ color: stat.color }}>
              {stat.isCurrency ? (
                <><AnimatedNumber value={Math.round(stats.revenue / 1000)} />k</>
              ) : (
                <AnimatedNumber value={stat.value} />
              )}
            </div>
            <div className="text-[10px] font-medium" style={{ color: 'var(--brand-text-muted)' }}>{stat.label}</div>
          </motion.div>
        ))}
      </div>

      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div className="flex items-center gap-4">
          <div>
            <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{viewMode === 'live' ? t('admin.live_orders', 'Live Orders') : t('courier.history', 'Order History')}</h2>
            <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{filteredOrders.length}</p>
          </div>
          {clientSlug && (
            <div className="hidden sm:flex items-center gap-2 px-3 py-1.5 rounded-lg border text-xs" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
              <i className="ti ti-link text-[var(--brand-primary)]" />
              <span className="font-mono truncate max-w-[180px]">{clientSlug}.dowiz.org</span>
              <button
                onClick={() => {
                  navigator.clipboard.writeText(`https://${clientSlug}.dowiz.org`);
                  setCopiedLink(true);
                  setTimeout(() => setCopiedLink(false), 2000);
                }}
                className="text-[var(--brand-primary)] hover:underline font-medium shrink-0"
              >
                {copiedLink ? t('common.copied', 'Copied!') : t('common.copy', 'Copy')}
              </button>
            </div>
          )}
          <div className="flex bg-[var(--brand-surface)] border rounded-lg overflow-hidden p-0.5" role="tablist" aria-label={t('admin.view_mode', 'View mode')} style={{ borderColor: 'var(--brand-border)' }}>
            <button role="tab" aria-selected={viewMode === 'live'}
              onClick={() => { setViewMode('live'); setStatusFilter('all'); }}
              className={`px-3 py-1 text-sm font-medium rounded-md transition-colors ${viewMode === 'live' ? 'bg-[var(--brand-primary)] text-white' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}
            >{t('admin.live', 'Live')}</button>
            <button role="tab" aria-selected={viewMode === 'history'}
              onClick={() => { setViewMode('history'); setStatusFilter('all'); }}
              className={`px-3 py-1 text-sm font-medium rounded-md transition-colors ${viewMode === 'history' ? 'bg-[var(--brand-primary)] text-white' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}
            >{t('courier.history', 'History')}</button>
          </div>
        </div>

        {/* Search */}
        <div className="flex items-center gap-2">
          <div className="relative">
            <i className="ti ti-search absolute left-3 top-1/2 -translate-y-1/2 text-sm" style={{ color: 'var(--brand-text-muted)' }} />
            <input
              value={search}
              onChange={e => setSearch(e.target.value)}
              placeholder={t('common.search', 'Search...')}
              aria-label="Search orders by name or ID"
              className="pl-9 pr-4 py-2 rounded-lg border text-sm outline-none transition-all duration-200 focus:border-[var(--brand-primary)] focus:ring-2 focus:ring-[var(--brand-primary-light)] w-full sm:w-56"
              style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
            />
          </div>
          <button onClick={() => exportCSV(filteredOrders, 'orders.csv')} className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg border transition-all duration-200 hover:bg-[var(--brand-surface-raised)] active:scale-[0.97]" style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
            <i className="ti ti-download"></i> {t('admin.export_csv', 'Export CSV')}
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-2">
        <div className="flex overflow-x-auto hide-scrollbar gap-1 pb-1 snap-x snap-mandatory flex-1 -mx-1 px-1" role="group" aria-label={t('admin.status_filter', 'Order status filter')}>
          {STATUSES.map(s => (
            <button
              key={s}
              onClick={() => setStatusFilter(s)}
              aria-pressed={statusFilter === s}
              className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all duration-200 capitalize snap-start shrink-0 ${statusFilter === s ? 'bg-[var(--brand-primary)] text-white shadow-sm' : 'bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}
              style={{ minHeight: 'var(--tap-min)' }}
            >
              {s === 'all' ? t('common.all', 'All') : t(`order.${s.toLowerCase()}`, s.replace('_', ' ').toLowerCase())}
            </button>
          ))}
        </div>
        {isMobile ? (
          <>
            <button
              onClick={() => setSortPickerOpen(true)}
              className="flex items-center gap-1.5 px-3 py-2 rounded-lg border text-xs font-medium shrink-0"
              style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)', minHeight: 'var(--tap-min)' }}
            >
              <i className="ti ti-arrows-sort text-sm" />
              {sortBy === 'newest' ? t('admin.newest_first', 'Newest') : sortBy === 'oldest' ? t('admin.oldest_first', 'Oldest') : t('admin.highest_total', 'Highest')}
            </button>
            <MobilePicker
              open={sortPickerOpen}
              onClose={() => setSortPickerOpen(false)}
              title={t('admin.sort_orders', 'Sort orders')}
              options={[
                { value: 'newest', label: t('admin.newest_first', 'Newest first'), icon: 'ti ti-sort-descending' },
                { value: 'oldest', label: t('admin.oldest_first', 'Oldest first'), icon: 'ti ti-sort-ascending' },
                { value: 'highest', label: t('admin.highest_total', 'Highest total'), icon: 'ti ti-coin' },
              ]}
              selectedValue={sortBy}
              onSelect={(opt) => setSortBy(opt.value as any)}
            />
          </>
        ) : (
          <select
            value={sortBy}
            onChange={e => setSortBy(e.target.value as any)}
            aria-label={t('admin.sort_orders', 'Sort orders')}
            className="px-3 py-1.5 text-xs rounded-lg border outline-none transition-colors shrink-0"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
          >
            <option value="newest">{t('admin.newest_first', 'Newest first')}</option>
            <option value="oldest">{t('admin.oldest_first', 'Oldest first')}</option>
            <option value="highest">{t('admin.highest_total', 'Highest total')}</option>
          </select>
        )}
      </div>

      {/* Orders grid */}
      {error ? (
        <EmptyState title={t('common.error', 'Error')} description={error} />
      ) : loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {[1, 2, 3, 4].map(i => (
            <div key={i} className="h-36 rounded-xl shimmer" />
          ))}
        </div>
      ) : filteredOrders.length === 0 ? (
        <EmptyState
          title={t('common.no_data', 'No data')}
          description={t('common.no_data', 'No data')}
          icon={<i className="ti ti-inbox text-4xl" style={{ color: 'var(--brand-text-muted)', opacity: 0.4 }} />}
        />
      ) : (
        <motion.div
          className="grid grid-cols-1 md:grid-cols-2 gap-4"
          variants={{ visible: { transition: { staggerChildren: 0.04 } } }}
          initial="hidden"
          animate="visible"
        >
          {filteredOrders.map(order => (
            <motion.div
              key={order.id}
              variants={{ hidden: { opacity: 0, y: 8 }, visible: { opacity: 1, y: 0, transition: { duration: 0.2 } } }}
            >
              <OrderCard
                order={order}
                onUpdateStatus={handleUpdateStatus}
                showMessages={expandedMessages.has(order.id)}
                onToggleMessages={handleToggleMessages}
                messages={messagesByOrder[order.id]}
                onSendMessage={handleSendMessage}
              />
            </motion.div>
          ))}
        </motion.div>
      )}

      {/* Live Courier Map */}
      <div>
        <div className="flex items-center gap-2 mb-3">
          <i className="ti ti-map-2 text-lg" style={{ color: 'var(--brand-primary)' }} />
          <h3 className="text-lg font-semibold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.couriers_live', 'Couriers Live')}</h3>
          <LiveDot size={8} color="var(--color-success)" />
        </div>
        <CourierLiveMap className="h-48 md:h-72 w-full rounded-xl border border-glow" couriers={couriersOnMap} center={[19.817, 41.331]} zoom={13} />
      </div>

      {/* Readiness Checklist */}
      <div className="p-5 rounded-xl border border-glow" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
        <div className="flex items-center gap-2 mb-4">
          <i className="ti ti-clipboard-check text-lg" style={{ color: 'var(--brand-primary)' }} />
          <h3 className="text-sm font-semibold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.store_readiness', 'Store Readiness')}</h3>
          <span className="ml-auto text-xs font-medium px-2 py-0.5 rounded-full" style={{ background: 'var(--brand-primary-light)', color: 'var(--brand-primary)' }}>
            {doneCount}/{totalChecks}
          </span>
        </div>
        <div className="w-full h-1.5 rounded-full mb-4" style={{ background: 'var(--brand-border)' }}>
          <div
            className="h-full rounded-full progress-animate"
            style={{
              width: `${(doneCount / totalChecks) * 100}%`,
              background: doneCount === totalChecks ? 'var(--color-success)' : 'var(--brand-primary)',
            }}
          />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-2">
          {readinessItems.map(item => (
            <div
              key={item.label}
              className="flex items-center gap-2.5 p-2.5 rounded-lg transition-all duration-200 hover:bg-[var(--brand-surface-raised)]"
              style={{ background: 'var(--brand-surface-raised)' }}
            >
              <i
                className={`${item.icon} text-sm`}
                style={{ color: item.done ? 'var(--color-success)' : 'var(--brand-text-muted)' }}
              />
              <span className="text-xs flex-1" style={{ color: item.done ? 'var(--brand-text)' : 'var(--brand-text-muted)' }}>
                {item.label}
              </span>
              {item.done ? (
                <i className="ti ti-circle-check-filled text-sm" style={{ color: 'var(--color-success)' }} />
              ) : (
                <i className="ti ti-circle-dashed text-sm" style={{ color: 'var(--brand-text-muted)', opacity: 0.5 }} />
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
    </PullToRefresh>
  );
}
