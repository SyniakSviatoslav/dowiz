import React, { useState, useEffect, useMemo } from 'react';
import { Button, Input, EmptyState, CourierLiveMap, useI18n } from '@deliveryos/ui';
import type { CourierOnMap, LngLatLike } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

import { exportCSV } from '../../lib/exportCSV.js';

interface Courier {
  id: string;
  name: string;
  phone: string;
  status: 'online' | 'busy' | 'offline';
  deliveriesCompleted: number;
  rating: number;
}

const MOCK_COURIERS: Courier[] = [
  { id: 'cu1', name: 'Ardit Kola', phone: '+355691234567', status: 'busy', deliveriesCompleted: 342, rating: 4.8 },
  { id: 'cu2', name: 'Blerim Hoxha', phone: '+355692345678', status: 'online', deliveriesCompleted: 189, rating: 4.5 },
  { id: 'cu3', name: 'Elira Shehu', phone: '+355693456789', status: 'online', deliveriesCompleted: 76, rating: 4.2 },
  { id: 'cu4', name: 'Genti Mema', phone: '+355694567890', status: 'offline', deliveriesCompleted: 515, rating: 4.9 },
  { id: 'cu5', name: 'Denisa Leka', phone: '+355695678901', status: 'busy', deliveriesCompleted: 231, rating: 4.6 },
];

const MOCK_POSITIONS: Record<string, LngLatLike> = {
  cu1: [19.820, 41.333],
  cu2: [19.810, 41.329],
  cu3: [19.815, 41.336],
  cu4: [19.805, 41.325],
  cu5: [19.825, 41.338],
};

const STATUS_COLORS: Record<string, string> = {
  online: 'var(--color-success, #22c55e)',
  busy: 'var(--color-warning, #f59e0b)',
  offline: 'var(--brand-text-muted, #a8a8a8)',
};

export function CouriersPage() {
  const { t } = useI18n();
  const [couriers, setCouriers] = useState<Courier[]>([]);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [courierPositions, setCourierPositions] = useState<Record<string, LngLatLike>>({});
  const [showAddForm, setShowAddForm] = useState(false);
  const [newCourierEmail, setNewCourierEmail] = useState('');
  const [newCourierRole, setNewCourierRole] = useState('courier');
  const [inviteResult, setInviteResult] = useState('');

  const tenantId = 't1';

  const handleAddCourier = async () => {
    if (!newCourierEmail) return;
    try {
      const res = await apiClient<any>(`/owner/locations/${tenantId}/courier-invites`, { 
        method: 'POST', 
        body: { role: newCourierRole, email: newCourierEmail, ttl_hours: 48 } 
      });
      setInviteResult(res?.deepLink || res?.link || 'Invite created');
      setNewCourierEmail('');
      setTimeout(() => setInviteResult(''), 5000);
      fetchCouriers();
    } catch { setInviteResult('Failed to create invite'); }
  };

  const fetchCouriers = async () => {
    try {
      setLoading(true);
      const data = await apiClient<any>('/owner/couriers');
      if (Array.isArray(data) && data.length > 0) {
        setCouriers(data);
      } else {
        setCouriers(MOCK_COURIERS);
      }
    } catch (err: any) {
      if (err.status === 404) {
        setCouriers(MOCK_COURIERS);
        setCourierPositions(MOCK_POSITIONS);
      } else {
        setError('Failed to load couriers');
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchCouriers();
  }, []);

  const filtered = couriers.filter(
    (c) =>
      c.name.toLowerCase().includes(search.toLowerCase()) ||
      c.phone.includes(search)
  );

  const couriersOnMap: CourierOnMap[] = useMemo(() => {
    return filtered.map((c) => ({
      id: c.id,
      name: c.name,
      initials: c.name
        .split(' ')
        .map((n) => n[0])
        .join(''),
      lngLat: courierPositions[c.id] || MOCK_POSITIONS[c.id] || [19.817, 41.331],
      status: c.status === 'offline' ? 'offline' : c.status === 'busy' ? 'busy' : 'online',
    }));
  }, [filtered, courierPositions]);

  const onlineCount = couriers.filter((c) => c.status !== 'offline').length;

  return (
    <div className="p-4 space-y-6 max-w-4xl mx-auto">
      <div className="flex flex-col sm:flex-row sm:justify-between sm:items-center gap-3 border-b border-[var(--brand-border)] pb-4">
        <h2
          className="text-2xl font-bold"
          style={{ fontFamily: 'var(--brand-font-heading)' }}
        >
          {t('admin.couriers', 'Couriers')}
        </h2>
        <div className="flex items-center gap-3">
          <div className="bg-[var(--brand-surface-raised)] px-3 py-1 rounded-full text-sm font-medium">
            {onlineCount} {t('admin.online', 'online')}
          </div>
          <button onClick={() => exportCSV(filtered, 'couriers.csv')} className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors hover:bg-[var(--brand-surface-raised)]" style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
            <i className="ti ti-download"></i> {t('admin.export_csv', 'Export CSV')}
          </button>
          <Button onClick={() => setShowAddForm(!showAddForm)}>+ {t('admin.add_courier', 'Add Courier')}</Button>
        </div>
      </div>

      {showAddForm && (
        <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl p-4 flex gap-3 items-end flex-wrap">
          <div className="flex-1 min-w-[200px]">
            <label className="text-xs font-medium mb-1 block text-[var(--brand-text-muted)]">{t('admin.courier_email', 'Courier Email')}</label>
            <Input type="email" value={newCourierEmail} onChange={e => setNewCourierEmail(e.target.value)} placeholder="courier@example.com" />
          </div>
          <div className="w-32">
            <label className="text-xs font-medium mb-1 block text-[var(--brand-text-muted)]">{t('admin.role', 'Role')}</label>
            <select value={newCourierRole} onChange={e => setNewCourierRole(e.target.value)}
              className="w-full h-10 px-3 rounded-lg border text-sm outline-none bg-[var(--brand-surface)]"
              style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
              <option value="courier">{t('admin.courier_role', 'Courier')}</option>
              <option value="dispatcher">{t('admin.dispatcher', 'Dispatcher')}</option>
            </select>
          </div>
          <Button onClick={handleAddCourier} variant="primary">{t('admin.send_invite', 'Send Invite')}</Button>
          {inviteResult && <span className="text-sm w-full mt-2 font-medium" style={{ color: inviteResult === 'Failed to create invite' ? 'var(--color-danger)' : 'var(--color-success)' }}>
            {inviteResult.startsWith('http') ? (
              <a href={inviteResult} target="_blank" rel="noreferrer" className="underline break-all">{inviteResult}</a>
            ) : inviteResult}
          </span>}
        </div>
      )}

      <div className="max-w-sm">
        <Input
          placeholder={t('admin.search_couriers', 'Search couriers...')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>

      {error ? (
        <EmptyState title={t('common.error', 'Error')} description={error} />
      ) : loading ? (
        <div className="animate-pulse space-y-3">
          {[1, 2, 3].map((i) => (
            <div
              key={i}
              className="h-16 bg-[var(--brand-surface)] rounded-[var(--brand-radius)] w-full"
            />
          ))}
        </div>
      ) : filtered.length === 0 ? (
        <EmptyState title={t('admin.no_couriers', 'No couriers')} description={t('admin.no_couriers_match', 'No couriers match your search.')} />
      ) : (
        <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] overflow-hidden">
          {filtered.map((c) => (
            <div
              key={c.id}
              className="p-4 border-b border-[var(--brand-border)] last:border-b-0 flex items-center justify-between gap-3"
            >
              <div className="flex items-center gap-3 min-w-0">
                <div
                  className="w-10 h-10 rounded-full flex items-center justify-center text-white font-bold text-sm shrink-0"
                  style={{
                    backgroundColor:
                      c.status === 'offline'
                        ? 'var(--brand-text-muted, #a8a8a8)'
                        : 'var(--brand-primary)',
                  }}
                >
                  {c.name
                    .split(' ')
                    .map((n) => n[0])
                    .join('')}
                </div>
                <div className="min-w-0">
                  <div className="font-medium truncate">{c.name}</div>
                  <div className="text-xs text-[var(--brand-text-muted)]">{c.phone}</div>
                </div>
              </div>
              <div className="flex items-center gap-3 shrink-0">
                <div className="text-right hidden sm:block">
                  <div className="text-sm font-medium">{c.deliveriesCompleted}</div>
                  <div className="text-xs text-[var(--brand-text-muted)]">{t('admin.deliveries', 'deliveries')}</div>
                </div>
                <span
                  className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium capitalize"
                  style={{ backgroundColor: `${STATUS_COLORS[c.status]}20`, color: STATUS_COLORS[c.status] }}
                >
                  {c.status}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="mt-6">
        <h3
          className="text-lg font-semibold mb-3 text-[var(--brand-text)]"
          style={{ fontFamily: 'var(--brand-font-heading)' }}
        >
          {t('admin.live_map', 'Live Map')}
        </h3>
        <CourierLiveMap
          className="h-72 w-full rounded-lg"
          couriers={couriersOnMap}
          center={[19.817, 41.331]}
          zoom={13}
        />
      </div>
    </div>
  );
}
