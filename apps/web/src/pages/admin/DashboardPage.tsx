import React, { useEffect, useState, useMemo } from 'react';
import { OrderCard, EmptyState, CourierLiveMap, HintCard } from '@deliveryos/ui';
import type { AdminOrder, CourierOnMap, LngLatLike } from '@deliveryos/ui';
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
  const [showHint, setShowHint] = useState(() => localStorage.getItem('dos_dash_hint_dismissed') !== '1');

  const { play: playPing } = useSound('/sounds/ping.mp3');
  const tenantId = 't1';

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

  useEffect(() => { fetchOrders(); }, []);

  useWebSocket({
    room: `admin:${tenantId}`,
    enabled: false,
    onMessage: (msg) => {
      if (msg.type === 'order_created') { setOrders(prev => [msg.payload, ...prev]); playPing(); }
      else if (msg.type === 'order_updated') { setOrders(prev => prev.map(o => o.id === msg.payload.id ? { ...o, ...msg.payload } : o)); }
      else if (msg.type === 'courier_position') { setCourierPositions(prev => ({ ...prev, [msg.payload.courierId]: [msg.payload.lng, msg.payload.lat] })); }
    },
    onReconnect: () => { fetchOrders(); },
  });

  const handleUpdateStatus = async (id: string, newStatus: string) => {
    setOrders(prev => prev.map(o => o.id === id ? { ...o, status: newStatus as AdminOrder['status'] } : o));
    try {
      await apiClient(`/owner/orders/${id}/status`, { method: 'PATCH', body: { status: newStatus } });
    } catch {
      fetchOrders();
    }
  };

  const couriersOnMap: CourierOnMap[] = useMemo(() => [
    { id: 'cu1', name: 'Ardit', initials: 'AK', lngLat: courierPositions['cu1'] || [19.820, 41.333], status: 'busy' },
    { id: 'cu2', name: 'Blerim', initials: 'BH', lngLat: courierPositions['cu2'] || [19.810, 41.329], status: 'online' },
  ], [courierPositions]);

  const filteredOrders = useMemo(() => {
    let result = orders.filter(o => o.status !== 'DELIVERED' && o.status !== 'CANCELLED');
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
  }, [orders, search, statusFilter, sortBy]);

  const STATUSES = ['all', 'PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY'];

  const readinessItems = [
    { label: 'Menu items', done: true, icon: 'ti ti-tools-kitchen-2' },
    { label: 'Phone set', done: true, icon: 'ti ti-phone' },
    { label: 'Delivery zone', done: true, icon: 'ti ti-map-pin' },
    { label: 'Courier setup', done: false, icon: 'ti ti-motorbike' },
    { label: 'Branding', done: true, icon: 'ti ti-palette' },
    { label: 'Allergens declared', done: false, icon: 'ti ti-alert-triangle' },
    { label: 'Test order placed', done: false, icon: 'ti ti-shopping-cart' },
    { label: 'Payment setup', done: true, icon: 'ti ti-cash' },
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
    <div className="p-4 md:p-6 max-w-7xl mx-auto space-y-6">
      {/* Welcome Hint */}
      {showHint && (
        <HintCard
          title="Welcome to your Dashboard"
          description="Here you can manage incoming orders, track couriers, and monitor your store readiness. Use the sidebar to navigate between sections."
          icon="ti ti-info-circle"
          onDismiss={() => { setShowHint(false); localStorage.setItem('dos_dash_hint_dismissed', '1'); }}
        />
      )}

      {/* Quick Stats Row */}
      <div className="grid grid-cols-5 gap-2 stagger-children">
        {[
          { label: 'Pending', value: stats.pending, color: 'var(--status-pending)' },
          { label: 'Preparing', value: stats.inProgress, color: 'var(--status-preparing)' },
          { label: 'Ready', value: stats.ready, color: 'var(--status-ready)' },
          { label: 'Delivery', value: stats.inDelivery, color: 'var(--status-in-delivery)' },
          { label: 'Revenue', value: `${(stats.revenue / 1000).toFixed(0)}k`, color: 'var(--brand-primary)' },
        ].map((stat, i) => (
          <div key={stat.label} className="text-center p-3 rounded-xl card-lift" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', border: '1px solid var(--brand-border)' }}>
            <div className="text-2xl font-bold mb-0.5" style={{ color: stat.color }}>{stat.value}</div>
            <div className="text-[10px] font-medium" style={{ color: 'var(--brand-text-muted)' }}>{stat.label}</div>
          </div>
        ))}
      </div>

      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Live Orders</h2>
          <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{filteredOrders.length} active</p>
        </div>

        {/* Search */}
        <div className="flex items-center gap-2">
          <div className="relative">
            <i className="ti ti-search absolute left-3 top-1/2 -translate-y-1/2 text-sm" style={{ color: 'var(--brand-text-muted)' }} />
            <input
              value={search}
              onChange={e => setSearch(e.target.value)}
              placeholder="Search orders..."
              aria-label="Search orders by name or ID"
              className="pl-9 pr-4 py-2 rounded-lg border text-sm outline-none transition-all duration-200 focus:border-[var(--brand-primary)] focus:ring-2 focus:ring-[var(--brand-primary-light)] w-full sm:w-56"
              style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
            />
          </div>
          <button onClick={() => exportCSV(filteredOrders, 'orders.csv')} className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg border transition-all duration-200 hover:bg-[var(--brand-surface-raised)] active:scale-[0.97]" style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
            <i className="ti ti-download"></i> Export CSV
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-center gap-2">
        <div className="flex rounded-lg p-0.5" style={{ background: 'var(--brand-surface-raised)' }}>
          {STATUSES.map(s => (
            <button
              key={s}
              onClick={() => setStatusFilter(s)}
              aria-pressed={statusFilter === s}
              className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all duration-200 capitalize ${statusFilter === s ? 'bg-[var(--brand-primary)] text-white shadow-sm' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}
            >
              {s === 'all' ? 'All' : s.replace('_', ' ').toLowerCase()}
            </button>
          ))}
        </div>
        <div className="flex-1" />
        <select
          value={sortBy}
          onChange={e => setSortBy(e.target.value as any)}
          className="px-3 py-1.5 text-xs rounded-lg border outline-none transition-colors"
          style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
        >
          <option value="newest">Newest first</option>
          <option value="oldest">Oldest first</option>
          <option value="highest">Highest total</option>
        </select>
      </div>

      {/* Orders grid */}
      {error ? (
        <EmptyState title="Error" description={error} />
      ) : loading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {[1, 2, 3, 4].map(i => (
            <div key={i} className="h-36 rounded-xl shimmer" />
          ))}
        </div>
      ) : filteredOrders.length === 0 ? (
        <EmptyState
          title="No active orders"
          description={search ? 'No orders match your search.' : 'Waiting for incoming orders. Orders will appear here in real time.'}
          icon={<i className="ti ti-inbox text-4xl" style={{ color: 'var(--brand-text-muted)', opacity: 0.4 }} />}
        />
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 stagger-children">
          {filteredOrders.map(order => (
            <OrderCard key={order.id} order={order} onUpdateStatus={handleUpdateStatus} />
          ))}
        </div>
      )}

      {/* Live Courier Map */}
      <div>
        <div className="flex items-center gap-2 mb-3">
          <i className="ti ti-map-2 text-lg" style={{ color: 'var(--brand-primary)' }} />
          <h3 className="text-lg font-semibold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Couriers Live</h3>
          <span className="w-2 h-2 rounded-full dot-pulse" style={{ backgroundColor: 'var(--color-success)' }} />
        </div>
        <CourierLiveMap className="h-72 w-full rounded-xl border border-glow" couriers={couriersOnMap} center={[19.817, 41.331]} zoom={13} />
      </div>

      {/* Readiness Checklist */}
      <div className="p-5 rounded-xl border border-glow" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
        <div className="flex items-center gap-2 mb-4">
          <i className="ti ti-clipboard-check text-lg" style={{ color: 'var(--brand-primary)' }} />
          <h3 className="text-sm font-semibold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Store Readiness</h3>
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
  );
}
