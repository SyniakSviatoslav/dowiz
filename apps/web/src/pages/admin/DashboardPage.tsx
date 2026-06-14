import React, { useEffect, useState, useMemo, useCallback, useRef } from 'react';
import { motion } from 'framer-motion';
import { OrderCard, EmptyState, CourierLiveMap, HintCard, useI18n, MobilePicker, useIsMobile, AnimatedNumber, LiveDot, WSStatusDot, useToast, useConfirm, useHaptics, useSoundPrefs, ResponsiveDialog, PriceDisplay } from '@deliveryos/ui';
import type { AdminOrder, CourierOnMap, LngLatLike, PickerOption } from '@deliveryos/ui';
import type { ThemeConfig } from '@deliveryos/ui';
import { apiClient, useWebSocket, useSound } from '../../lib/index.js';
import { z } from 'zod';
import { LocationResponse, CourierListResponse, CategoryResponse } from '@deliveryos/shared-types';

const AnyResponse = z.any();

const NotificationStatusResponse = z.object({
  telegramConnected: z.boolean().optional(),
}).passthrough();

const OrdersListResponse = z.any();
import { exportCSV } from '../../lib/exportCSV.js';
import { mergeDelta } from './dashboard-utils.js';

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
  const [readiness, setReadiness] = useState<{ menu: boolean; phone: boolean; address: boolean; couriers: boolean; branding: boolean; placeOrder: boolean; telegram: boolean }>({ menu: false, phone: false, address: false, couriers: false, branding: false, placeOrder: false, telegram: false });

  const { play: playPing } = useSound('/sounds/ping.mp3');
  const { trigger: haptic } = useHaptics();
  const { alertSoundEnabled } = useSoundPrefs();
  const [tenantId, setTenantId] = useState('');
  const [expandedMessages, setExpandedMessages] = useState<Set<string>>(new Set());
  const [messagesByOrder, setMessagesByOrder] = useState<Record<string, any[]>>({});
  const [detailOrder, setDetailOrder] = useState<any | null>(null);

  const fetchOrders = async () => {
    try {
      setLoading(true);
      const data = await apiClient<typeof OrdersListResponse>('/owner/orders', { schema: OrdersListResponse });
      setOrders(Array.isArray(data) ? data : []);
    } catch (err: any) {
      setOrders([]);
      setError('Failed to load orders');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchOrders();
    apiClient<typeof LocationResponse>('/owner/settings', { schema: LocationResponse }).then(res => {
      if (res.id) setTenantId(res.id);
      if ((res as any).locationName || res.name) {
        const generated = ((res as any).locationName || res.name).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '').slice(0, 50);
        setClientSlug(generated);
      }
      // Compute readiness from actual data
      const r = { menu: false, phone: false, address: false, couriers: false, branding: false, placeOrder: false, telegram: false };
      r.phone = !!(res.phone && res.phone.length > 5);
      r.address = !!(res.address && res.address.length > 5);
      r.branding = !!((res as any).locationName || res.name) && ((res as any).locationName || res.name).length > 2;
      setReadiness(r);
      // Check notification status (lightweight)
      if (res.id) {
        apiClient<typeof NotificationStatusResponse>(`/owner/locations/${res.id}/notifications/status`, { schema: NotificationStatusResponse }).then(status => {
          setReadiness(prev => ({ ...prev, telegram: status?.telegramConnected || false }));
        }).catch(() => {});
      }
    }).catch(() => {});
    // Check menu + couriers in parallel
    Promise.all([
      apiClient('/owner/menu/categories', { schema: z.array(CategoryResponse) }).catch(() => []),
      apiClient<typeof CourierListResponse>('/owner/couriers', { schema: CourierListResponse }).catch(() => ({ couriers: [] })),
    ]).then(([cats, couriers]) => {
      setReadiness(prev => ({
        ...prev,
        menu: Array.isArray(cats) && cats.length > 0,
        couriers: Array.isArray(couriers) && couriers.length > 0,
      }));
    }).catch(() => {});
  }, []);

  const isFirstConnect = useRef(true);
  const { status: connectionStatus } = useWebSocket({
    room: `location:${tenantId}:dashboard`,
    enabled: true,
    onMessage: (msg) => {
      const envelope = msg?.data || msg;
      const payload = envelope.data || envelope;
      if (envelope.type === 'order.created') {
        setOrders(prev => mergeDelta(prev, payload, true));
        if (alertSoundEnabled) playPing();
        haptic('tap');
      }
      else if (envelope.type === 'order.status') {
        setOrders(prev => mergeDelta(prev, payload, false));
      }
      else if (envelope.type === 'courier_position') { setCourierPositions(prev => ({ ...prev, [payload.courierId]: [payload.lng, payload.lat] })); }
    },
    onReconnect: () => {
      if (isFirstConnect.current) { isFirstConnect.current = false; return; }
      fetchOrders();
    },
  });

  // Separate subscription for courier channel (position updates, shift status)
  useWebSocket({
    room: `location:${tenantId}:couriers`,
    enabled: !!tenantId,
    onMessage: (msg) => {
      const envelope = msg?.data || msg;
      const p = envelope.payload;
      if (!p) return;
      if (envelope.type === 'courier.position_updated' && p.position) {
        setCourierPositions(prev => ({ ...prev, [p.courierId]: [p.position.lng, p.position.lat] }));
      } else if (envelope.type === 'courier.shift_updated') {
        setCourierPositions(prev => {
          if (p.status === 'offline') {
            const next = { ...prev };
            delete next[p.courierId];
            return next;
          }
          return prev;
        });
      }
    },
  });

  const fetchMessages = async (orderId: string) => {
    if (messagesByOrder[orderId]) return;
    try {
      const data = await apiClient<typeof AnyResponse>(`/orders/${orderId}/messages`, { schema: AnyResponse });
      setMessagesByOrder(prev => ({ ...prev, [orderId]: Array.isArray(data) ? data : [] }));
    } catch (err) { console.debug('[DashboardPage] fetch messages failed:', err); }
  };

  const handleSendMessage = async (orderId: string, presetKey: string, params?: Record<string, unknown>) => {
    try {
      const msg = await apiClient(`/orders/${orderId}/messages`, { method: 'POST', body: { presetKey, params } });
      setMessagesByOrder(prev => ({ ...prev, [orderId]: [...(prev[orderId] || []), msg] }));
    } catch (err) { console.warn('[DashboardPage] send message failed:', err); }
  };

  const { showToast } = useToast();
  const { confirm, dialog } = useConfirm();

  const handleToggleMessages = (orderId: string) => {
    setExpandedMessages(prev => {
      const next = new Set(prev);
      if (next.has(orderId)) next.delete(orderId);
      else { next.add(orderId); fetchMessages(orderId); }
      return next;
    });
  };

  const handleUpdateStatus = async (id: string, newStatus: string) => {
    if (newStatus === 'REJECTED' || newStatus === 'CANCELLED') {
      const confirmed = await confirm({
        title: newStatus === 'REJECTED' ? t('admin.reject_order', 'Reject Order') : t('admin.cancel_order', 'Cancel Order'),
        message: newStatus === 'REJECTED'
          ? t('admin.reject_confirm_msg', 'Are you sure you want to reject this order? This cannot be undone.')
          : t('admin.cancel_confirm_msg', 'Are you sure you want to cancel this order? This cannot be undone.'),
        confirmLabel: newStatus === 'REJECTED' ? t('admin.reject', 'Reject') : t('admin.cancel', 'Cancel'),
        variant: 'danger',
      });
      if (!confirmed) return;
    }
    setOrders(prev => {
      const existing = prev.find(o => o.id === id);
      if (!existing) return prev;
      if (existing.status === newStatus) return prev;
      return prev.map(o => o.id === id ? { ...o, status: newStatus as AdminOrder['status'] } as AdminOrder : o);
    });
    try {
      await apiClient(`/orders/${id}/status`, { method: 'PATCH', body: { status: newStatus } });
      const statusLabels: Record<string, string> = {
        CONFIRMED: t('admin.order_confirmed_toast', 'Order confirmed!'),
        PREPARING: t('admin.order_preparing_toast', 'Order marked as preparing!'),
        READY: t('admin.order_ready_toast', 'Order is ready!'),
        IN_DELIVERY: t('admin.order_assigned_toast', 'Order assigned to courier!'),
        DELIVERED: t('admin.order_delivered_toast', 'Order delivered!'),
        REJECTED: t('admin.order_rejected_toast', 'Order rejected.'),
        CANCELLED: t('admin.order_cancelled_toast', 'Order cancelled.'),
      };
      if (statusLabels[newStatus]) {
        showToast(statusLabels[newStatus], newStatus === 'REJECTED' || newStatus === 'CANCELLED' ? 'error' : 'success');
      }
    } catch (err) {
      console.error('[DashboardPage] update status failed:', err);
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
    { label: t('admin.telegram_notifications', 'Telegram'), done: readiness.telegram, icon: 'ti ti-brand-telegram' },
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
    <div className="p-4 md:p-6 max-w-7xl mx-auto space-y-4" role="region" aria-live="polite" aria-label={t('admin.live_orders', 'Live orders')}>
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
      <div className="grid grid-cols-3 sm:grid-cols-5 gap-3 stagger-children">
        {[
          { label: t('order.pending', 'Pending'), value: stats.pending, color: 'var(--status-pending)', isCurrency: false, tooltip: t('tooltip.pending_orders', 'Orders awaiting confirmation') },
          { label: t('order.preparing', 'Preparing'), value: stats.inProgress, color: 'var(--status-preparing)', isCurrency: false, tooltip: t('tooltip.active_orders', 'Orders being prepared') },
          { label: t('order.ready', 'Ready'), value: stats.ready, color: 'var(--status-ready)', isCurrency: false, tooltip: t('tooltip.ready_orders', 'Orders ready for pickup or delivery') },
          { label: t('order.in_delivery', 'Delivery'), value: stats.inDelivery, color: 'var(--status-in-delivery)', isCurrency: false, tooltip: '' },
          { label: t('cart.total', 'Revenue'), value: stats.revenue, color: 'var(--brand-primary)', isCurrency: true, tooltip: t('tooltip.revenue_today', "Today's total revenue") },
        ].map((stat, i) => (
          <div
            key={stat.label}
            title={stat.tooltip || undefined}
            className="text-center p-3 rounded-xl fade-in card-base" style={{ animationDelay: `${i * 40}ms` }}
          >
            <div className="text-2xl font-bold mb-0.5" style={{ color: stat.color }}>
              {stat.isCurrency ? (
                <><AnimatedNumber value={Math.round(stats.revenue / 1000)} />k</>
              ) : (
                <AnimatedNumber value={stat.value} />
              )}
            </div>
            <div className="text-[10px] font-medium" style={{ color: 'var(--brand-text-muted)' }}>{stat.label}</div>
          </div>
        ))}
      </div>

      {/* Header */}
        {/* Sticky header: title + view toggles + search */}
        <div className="sticky top-0 z-10 pb-2 space-y-3" style={{ background: 'var(--brand-bg)' }}>
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div className="flex items-center gap-4 shrink-0">
              <div>
                <div className="flex items-center gap-2">
                  <h2 className="text-xl sm:text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{viewMode === 'live' ? t('admin.live_orders', 'Live Orders') : t('courier.history', 'Order History')}</h2>
                  <div data-testid="ws-status-dot" data-connected={connectionStatus === 'connected' ? 'true' : 'false'}><WSStatusDot status={connectionStatus === 'disabled' ? 'disconnected' : connectionStatus} /></div>
                </div>
                <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{filteredOrders.length}</p>
              </div>
              <div className="flex bg-[var(--brand-surface)] border rounded-lg overflow-hidden p-0.5 shrink-0" role="tablist" aria-label={t('admin.view_mode', 'View mode')} style={{ borderColor: 'var(--brand-border)' }}>
                <motion.button role="tab" whileTap={{ scale: 0.97 }} aria-selected={viewMode === 'live'}
                  onClick={() => { setViewMode('live'); setStatusFilter('all'); }}
                  className={`w-16 sm:w-20 px-3 py-1 text-sm font-medium rounded-md whitespace-nowrap transition-colors text-center ${viewMode === 'live' ? 'bg-[var(--brand-primary-light)] text-[var(--brand-text)] border border-[var(--brand-primary)]' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)] border border-transparent'}`}
                >{t('admin.live', 'Live')}</motion.button>
                <motion.button role="tab" whileTap={{ scale: 0.97 }} aria-selected={viewMode === 'history'}
                  onClick={() => { setViewMode('history'); setStatusFilter('all'); }}
                  className={`w-16 sm:w-20 px-3 py-1 text-sm font-medium rounded-md whitespace-nowrap transition-colors text-center ${viewMode === 'history' ? 'bg-[var(--brand-primary-light)] text-[var(--brand-text)] border border-[var(--brand-primary)]' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)] border border-transparent'}`}
                >{t('courier.history', 'History')}</motion.button>
              </div>
              {clientSlug && (
                <div className="hidden md:flex items-center gap-2 px-3 py-1.5 rounded-lg border text-xs" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
                  <i className="ti ti-link text-[var(--brand-primary)]" />
                  <span className="font-mono truncate max-w-[140px]">{clientSlug}.dowiz.org</span>
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
            </div>

            <div className="flex items-center gap-2">
              <div className="relative flex-1 sm:flex-none">
                <i className="ti ti-search absolute left-3 top-1/2 -translate-y-1/2 text-sm" style={{ color: 'var(--brand-text-muted)' }} />
                <input
                  value={search}
                  onChange={e => setSearch(e.target.value)}
                  placeholder={t('common.search', 'Search...')}
                  aria-label="Search orders by name or ID"
                  className="pl-9 pr-4 py-2 rounded-lg border text-sm outline-none transition-all duration-200 focus:border-[var(--brand-primary)] focus:ring-2 focus:ring-[var(--brand-primary-light)] w-full sm:w-48"
                  style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
                />
              </div>
              <motion.button onClick={() => exportCSV(filteredOrders, 'orders.csv')} whileTap={{ scale: 0.97 }} title={t('tooltip.export_csv', 'Export as CSV')} className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg border transition-all duration-200 hover:bg-[var(--brand-surface-raised)] active:scale-[0.97] shrink-0" style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
                <i className="ti ti-download"></i> {t('admin.export_csv', 'Export CSV')}
              </motion.button>
            </div>
          </div>

          {/* Filters row: statuses + sort — stacked on mobile, side-by-side on desktop */}
          <div className="flex flex-col sm:flex-row items-start sm:items-center gap-2">
            <div className="flex overflow-x-auto hide-scrollbar gap-1 pb-1 snap-x snap-mandatory flex-1 w-full sm:w-auto" role="group" aria-label={t('admin.status_filter', 'Order status filter')}>
              {STATUSES.map(s => (
                  <motion.button
                  key={s}
                  onClick={() => setStatusFilter(s)}
                  whileTap={{ scale: 0.97 }}
                  aria-pressed={statusFilter === s}
                  title={t('tooltip.filter_status', 'Filter by status')}
                  className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all duration-200 capitalize snap-start shrink-0 whitespace-nowrap ${statusFilter === s ? 'bg-[var(--brand-primary-light)] text-[var(--brand-text)] shadow-sm border border-[var(--brand-primary)]' : 'bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] hover:text-[var(--brand-text)] border border-transparent'}`}
                  style={{ minHeight: 'var(--tap-min)' }}
                >
                  {s === 'all' ? t('common.all', 'All') : t(`order.${s.toLowerCase()}`, s.replace('_', ' ').toLowerCase())}
                </motion.button>
              ))}
            </div>
            <div className="relative self-end sm:self-auto">
              <motion.button
                onClick={() => setSortPickerOpen(true)}
                whileTap={{ scale: 0.97 }}
                className="flex items-center gap-1.5 px-3 py-2 rounded-lg border text-xs font-medium shrink-0"
                style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)', minHeight: 'var(--tap-min)' }}
              >
                <i className="ti ti-arrows-sort text-sm" />
                {sortBy === 'newest' ? t('admin.newest_first', 'Newest') : sortBy === 'oldest' ? t('admin.oldest_first', 'Oldest') : t('admin.highest_total', 'Highest')}
              </motion.button>
              {isMobile ? (
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
              ) : (
                sortPickerOpen && (
                  <>
                    <div className="fixed inset-0 z-40" onClick={() => setSortPickerOpen(false)} />
                    <div className="absolute right-0 bottom-full mb-2 z-50 rounded-lg shadow-elevation-3 py-1 min-w-[150px] max-h-[60vh] overflow-y-auto scale-in" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)' }}>
                      {[
                        { value: 'newest', label: t('admin.newest_first', 'Newest first'), icon: 'ti ti-sort-descending' },
                        { value: 'oldest', label: t('admin.oldest_first', 'Oldest first'), icon: 'ti ti-sort-ascending' },
                        { value: 'highest', label: t('admin.highest_total', 'Highest total'), icon: 'ti ti-coin' },
                      ].map(opt => (
                        <motion.button key={opt.value} onClick={() => { setSortBy(opt.value as any); setSortPickerOpen(false); }} whileTap={{ scale: 0.97 }}
                          className={`flex items-center gap-2 w-full px-3 py-2 text-xs transition-colors hover:bg-[var(--brand-surface-raised)] ${sortBy === opt.value ? 'font-semibold' : ''}`}
                          style={{ color: sortBy === opt.value ? 'var(--brand-primary)' : 'var(--brand-text)' }}>
                          <i className={opt.icon} style={{ fontSize: '0.8rem' }} />
                          <span className="flex-1">{opt.label}</span>
                          {sortBy === opt.value && <i className="ti ti-check" style={{ color: 'var(--brand-primary)', fontSize: '0.7rem' }} />}
                        </motion.button>
                      ))}
                    </div>
                  </>
                )
              )}
            </div>
          </div>
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
          title={t('admin.no_orders_yet', 'No orders yet')}
          description={t('admin.no_orders_hint', 'Share your menu link with customers to start receiving orders.')}
          icon={<i className="ti ti-inbox text-4xl" style={{ color: 'var(--brand-text-muted)', opacity: 0.4 }} />}
        />
      ) : (
        <motion.div
          className="grid grid-cols-1 md:grid-cols-2 gap-4"
          variants={{ visible: { transition: { staggerChildren: 0.025 } } }}
          initial="hidden"
          whileInView="visible"
          viewport={{ once: true, margin: '-40px' }}
          aria-live="polite" aria-atomic="true"
        >
          {filteredOrders.map(order => (
            <motion.div
              key={order.id}
              variants={{ hidden: { opacity: 0, y: 10 }, visible: { opacity: 1, y: 0, transition: { duration: 0.2, ease: [0.16, 1, 0.3, 1] } } }}
              className="order-card-container" data-testid={`order-card-${order.id}`} data-status={order.status}>
              <OrderCard
                order={order}
                onUpdateStatus={handleUpdateStatus}
                showMessages={expandedMessages.has(order.id)}
                onToggleMessages={handleToggleMessages}
                messages={messagesByOrder[order.id]}
                onSendMessage={handleSendMessage}
                onViewDetail={(id) => setDetailOrder(orders.find(o => o.id === id) || null)}
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

      {/* Notification Banner */}
      {!readiness.telegram && filteredOrders.some(o => o.status !== 'DELIVERED' && o.status !== 'CANCELLED') && (
        <div className="p-4 rounded-xl border" style={{ background: 'var(--color-warning-light, rgba(217,119,6,0.1))', borderColor: 'var(--color-warning, #D97706)' }}>
          <div className="flex items-start gap-3">
            <i className="ti ti-bell-ringing text-lg shrink-0" style={{ color: 'var(--color-warning, #D97706)' }} />
            <div className="flex-1 min-w-0">
              <div className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>
                {t('admin.notification_banner_title', 'Notifications not set up')}
              </div>
              <div className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>
                {t('admin.notification_banner_desc', 'You won\'t receive alerts about new orders. Connect Telegram to stay informed.')}
              </div>
            </div>
            <a
              href="/admin/settings"
              className="shrink-0 px-3 py-1.5 text-xs font-semibold rounded-full text-white transition-all hover:opacity-90"
              style={{ background: 'var(--brand-primary)' }}
            >
              <i className="ti ti-brand-telegram mr-1" />
              {t('admin.tg_connect', 'Connect')}
            </a>
          </div>
        </div>
      )}

      {/* Readiness Checklist */}
      <div className="card-base p-5">
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
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-2.5">
          {readinessItems.map(item => (
            <div
              key={item.label}
              className="flex items-center gap-2.5 p-2.5 rounded-lg transition-all duration-200"
              style={{ background: item.done ? 'color-mix(in srgb, var(--color-success) 5%, transparent)' : 'var(--brand-surface-raised)' }}
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

      {/* Order detail modal */}
      <ResponsiveDialog open={!!detailOrder} onClose={() => setDetailOrder(null)} title={detailOrder ? t('order.number') + ' ' + detailOrder.shortId : ''}>
        {detailOrder && (
          <div className="space-y-4 text-sm">
            <div className="grid grid-cols-2 gap-3">
              <div><span className="text-[var(--brand-text-muted)]">{t('order.status')}:</span> <strong>{detailOrder.status}</strong></div>
              <div><span className="text-[var(--brand-text-muted)]">{t('checkout.total')}:</span> <PriceDisplay amount={detailOrder.total} size="lg" /></div>
              <div className="col-span-2"><span className="text-[var(--brand-text-muted)]">{t('admin.customer')}:</span> {detailOrder.customerName}</div>
              <div className="col-span-2"><span className="text-[var(--brand-text-muted)]">{t('checkout.delivery')}:</span> {detailOrder.deliveryAddress || '—'}</div>
              <div className="col-span-2"><span className="text-[var(--brand-text-muted)]">{t('checkout.payment')}:</span> {detailOrder.paymentMethod || '—'}</div>
              {detailOrder.courierName && <div className="col-span-2"><span className="text-[var(--brand-text-muted)]">{t('courier.title')}:</span> {detailOrder.courierName}</div>}
            </div>
            {detailOrder.items && detailOrder.items.length > 0 && (
              <div>
                <h4 className="font-semibold mb-2 text-[var(--brand-text)]">{t('order.items')}:</h4>
                <div className="space-y-1">
                  {detailOrder.items.map((item: any, i: number) => (
                    <div key={i} className="flex justify-between py-1 border-b" style={{ borderColor: 'var(--brand-border)' }}>
                      <span>{item.name} x{item.qty}</span>
                      <PriceDisplay amount={item.price * item.qty} />
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </ResponsiveDialog>
      {dialog}
    </div>
  );
}
