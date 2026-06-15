import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Button, Input, EmptyState, CourierLiveMap, useI18n, PriceDisplay, useToast } from '@deliveryos/ui';
import type { CourierOnMap, LngLatLike } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

const CourierDetailsResponse = z.object({
  shifts: z.array(z.object({ id: z.string(), status: z.string(), started_at: z.string(), ended_at: z.string().nullable() })),
  earnings: z.object({ today: z.number(), week: z.number(), month: z.number(), today_deliveries: z.number(), month_deliveries: z.number() }),
  history: z.array(z.any()),
}).passthrough();

const CourierInviteResponse = z.object({
  deepLink: z.string(),
  link: z.string(),
  code: z.string(),
}).passthrough();

import { exportCSV } from '../../lib/exportCSV.js';

interface Courier {
  id: string;
  name: string;
  phone: string;
  status: 'online' | 'busy' | 'offline';
  deliveriesCompleted: number;
  rating: number;
}

interface HistoryItem {
  id: string; order_id: string; status: string;
  assigned_at: string; accepted_at: string | null; picked_up_at: string | null; delivered_at: string | null;
  cash_amount: number; total: number; currency_code: string; delivery_address: string | null;
  customer_name: string; customer_phone: string | null;
}

interface CourierDetails {
  shifts: Array<{ id: string; status: string; started_at: string; ended_at: string | null }>;
  earnings: { today: number; week: number; month: number; today_deliveries: number; month_deliveries: number };
  history: HistoryItem[];
}

const STATUS_COLORS: Record<string, string> = {
  online: 'var(--color-success)',
  busy: 'var(--color-warning)',
  offline: 'var(--brand-text-muted)',
};

export function CouriersPage() {
  const { t } = useI18n();
  const { showToast } = useToast();
  const [couriers, setCouriers] = useState<Courier[]>([]);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [courierPositions, setCourierPositions] = useState<Record<string, LngLatLike>>({});
  const [showAddForm, setShowAddForm] = useState(false);
  const [newCourierEmail, setNewCourierEmail] = useState('');
  const [newCourierRole, setNewCourierRole] = useState('courier');
  const [inviteResult, setInviteResult] = useState<{ link: string; code: string } | null>(null);
  const [inviteError, setInviteError] = useState('');
  const [locationId, setLocationId] = useState('');
  const [selectedCourier, setSelectedCourier] = useState<string | null>(null);
  const [courierDetails, setCourierDetails] = useState<CourierDetails | null>(null);
  const [detailsLoading, setDetailsLoading] = useState(false);
  const [selectedOrderDetail, setSelectedOrderDetail] = useState<HistoryItem | null>(null);

  useEffect(() => {
    apiClient<any>('/owner/settings').then((res: any) => {
      if (res.id) setLocationId(res.id);
    }).catch((err) => console.debug('[CouriersPage] failed to load settings:', err));
  }, []);

  const fetchDetails = async (courierId: string) => {
    if (selectedCourier === courierId) {
      setSelectedCourier(null);
      setCourierDetails(null);
      return;
    }
    setSelectedCourier(courierId);
    setDetailsLoading(true);
    try {
      const data = await apiClient<typeof CourierDetailsResponse>(`/owner/locations/${locationId}/couriers/${courierId}/details`, { schema: CourierDetailsResponse });
      setCourierDetails(data);
    } catch (err) {
      console.error('[CouriersPage] failed to fetch courier details:', err);
      setCourierDetails(null);
    } finally {
      setDetailsLoading(false);
    }
  };

  const handleAddCourier = async () => {
    if (!newCourierEmail || !locationId) return;
    setInviteError('');
    setInviteResult(null);
    try {
      const res = await apiClient<typeof CourierInviteResponse>(`/owner/locations/${locationId}/courier-invites`, { 
        method: 'POST', 
        body: { role: newCourierRole, email: newCourierEmail, ttl_hours: 48 },
        schema: CourierInviteResponse,
      });
      if (res?.deepLink || res?.link) {
        setInviteResult({
          link: (res.deepLink || res.link) as string,
          code: res.code || '',
        });
        showToast(t('admin.invite_created', 'Invite created!'), 'success');
        setNewCourierEmail('');
        fetchCouriers();
      } else {
        setInviteError('Failed to create invite');
      }
    } catch (err) { 
      console.error('[CouriersPage] failed to create invite:', err);
      setInviteError('Failed to create invite'); 
    }
  };

  const handleCopyInvite = () => {
    if (!inviteResult) return;
    const text = `Ftesë për Korrier / Courier Invite:\nLink: ${inviteResult.link}\nCode: ${inviteResult.code}`;
    navigator.clipboard.writeText(text);
    showToast(t('admin.invite_copied', 'Invite link copied!'), 'success');
  };

  const fetchCouriers = useCallback(async () => {
    if (!locationId) return;
    try {
      setLoading(true);
      const data = await apiClient<any>(`/owner/locations/${locationId}/couriers`);
      const list = data?.couriers;
      if (Array.isArray(list) && list.length > 0) {
        setCouriers(list.map((c: any) => ({
          id: c.id,
          name: c.full_name || c.name || 'Unknown',
          phone: c.masked_phone || c.maskedPhone || '',
          status: c.status === 'active' || c.status === 'available' ? 'online' : c.status === 'on_delivery' ? 'busy' : 'offline',
          deliveriesCompleted: c.deliveries_completed || c.ordersToday || 0,
          rating: c.rating || 0,
        })));
      } else {
        setCouriers([]);
      }
      setError('');
    } catch (err) {
      console.error('[CouriersPage] failed to fetch couriers:', err);
      setError('Failed to load couriers');
    } finally {
      setLoading(false);
    }
  }, [locationId]);

  useEffect(() => {
    if (locationId) fetchCouriers();
  }, [locationId, fetchCouriers]);

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
      lngLat: courierPositions[c.id] || [19.817, 41.331],
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
          <motion.button onClick={() => exportCSV(filtered, 'couriers.csv')} whileTap={{ scale: 0.97 }} className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors hover:bg-[var(--brand-surface-raised)]" style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
            <i className="ti ti-download"></i> {t('admin.export_csv', 'Export CSV')}
          </motion.button>
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

          {inviteError && (
            <div className="w-full mt-2 text-sm font-medium px-3 py-2 rounded-lg border border-[var(--status-cancelled-border)] bg-[var(--status-cancelled-light)] text-[var(--color-danger)]">
              {inviteError}
            </div>
          )}

          {inviteResult && (
            <div className="w-full mt-3 p-4 rounded-xl border border-[var(--status-confirmed-border)] bg-[var(--status-confirmed-light)] space-y-3">
              <div>
                <p className="text-xs font-semibold uppercase tracking-wider text-[var(--color-success)] mb-1">
                  Ftesa u Krijua / Invite Created
                </p>
                <p className="text-xs text-[var(--brand-text-muted)]">
                  Dërgoji këtë link dhe kod korrierit. Kodi nuk shfaqet më kurrë / Send the link and code to the courier. The code is never shown again.
                </p>
              </div>

              <div>
                <p className="text-[10px] font-bold uppercase tracking-widest text-[var(--brand-text-muted)] block mb-1">Link</p>
                <div className="flex gap-2 items-center">
                  <a 
                    href={inviteResult.link} 
                    target="_blank" 
                    rel="noreferrer" 
                    className="flex-1 px-3 py-2 text-xs font-mono break-all rounded-lg border border-[var(--brand-border)] bg-[var(--brand-surface)] text-[var(--brand-text)] hover:bg-[var(--brand-surface-raised)] truncate"
                  >
                    {inviteResult.link}
                  </a>
                </div>
              </div>

              <div>
                <p className="text-[10px] font-bold uppercase tracking-widest text-[var(--brand-text-muted)] block mb-1">
                  Kodi i Sigurisë / Security Code (16 chars)
                </p>
                <code className="block px-3 py-2 text-lg font-mono tracking-widest text-center font-bold rounded-lg border border-[var(--brand-border)] bg-[var(--brand-surface)] text-[var(--brand-text)] select-all">
                  {inviteResult.code}
                </code>
              </div>

              <Button
                onClick={handleCopyInvite}
                variant="primary"
                className="w-full"
                size="sm"
              >
                {'📋 Kopjo Detajet / Copy Link & Code'}
              </Button>
            </div>
          )}
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
        <EmptyState title={t('admin.no_couriers', 'No couriers')} description={search ? t('admin.no_couriers_match', 'No couriers match your search.') : t('admin.no_couriers_hint', 'Send an invite link to add your first courier.')} />
      ) : (
        <motion.div
          className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] overflow-hidden"
          variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.04, delayChildren: 0.05 } } }}
          initial="hidden"
          animate="visible"
        >
          {filtered.map((c) => (
            <motion.div
              key={c.id}
              variants={{ hidden: { opacity: 0, y: 8 }, visible: { opacity: 1, y: 0, transition: { type: 'spring', stiffness: 260, damping: 24 } } }}
            >
              <motion.div
                whileTap={{ scale: 0.99 }}
                className="p-4 border-b border-[var(--brand-border)] flex items-center justify-between gap-3 cursor-pointer hover:bg-[var(--brand-surface-raised)] transition-colors"
                onClick={() => fetchDetails(c.id)}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div
                    className="w-10 h-10 rounded-full flex items-center justify-center text-white font-bold text-sm shrink-0"
                    style={{
                      backgroundColor:
                        c.status === 'offline'
                          ? 'var(--brand-text-muted)'
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
                  <i className={`ti ${selectedCourier === c.id ? 'ti-chevron-up' : 'ti-chevron-down'} text-sm text-[var(--brand-text-muted)]`} />
                </div>
              </motion.div>
              {selectedCourier === c.id && (
                <div className="p-4 border-b border-[var(--brand-border)] bg-[var(--brand-surface-raised)]/50">
                  {detailsLoading ? (
                    <div className="animate-pulse space-y-2">
                      <div className="h-4 bg-[var(--brand-surface)] rounded w-1/3" />
                      <div className="h-4 bg-[var(--brand-surface)] rounded w-1/2" />
                    </div>
                  ) : courierDetails ? (
                    <div className="space-y-4">
                      <div className="grid grid-cols-3 gap-3">
                        <div className="p-3 rounded-lg bg-[var(--brand-surface)] border border-[var(--brand-border)] text-center">
                          <div className="text-lg font-bold" style={{ color: 'var(--brand-primary)' }}><PriceDisplay amount={courierDetails.earnings?.today ?? 0} /></div>
                          <div className="text-xs text-[var(--brand-text-muted)]">{t('admin.earnings_today', 'Today')} ({courierDetails.earnings?.today_deliveries ?? 0} {t('admin.deliveries', 'deliveries')})</div>
                        </div>
                        <div className="p-3 rounded-lg bg-[var(--brand-surface)] border border-[var(--brand-border)] text-center">
                          <div className="text-lg font-bold"><PriceDisplay amount={courierDetails.earnings?.week ?? 0} /></div>
                          <div className="text-xs text-[var(--brand-text-muted)]">{t('admin.earnings_week', 'This Week')}</div>
                        </div>
                        <div className="p-3 rounded-lg bg-[var(--brand-surface)] border border-[var(--brand-border)] text-center">
                          <div className="text-lg font-bold"><PriceDisplay amount={courierDetails.earnings?.month ?? 0} /></div>
                          <div className="text-xs text-[var(--brand-text-muted)]">{t('admin.earnings_month', 'This Month')} ({courierDetails.earnings?.month_deliveries ?? 0} {t('admin.deliveries', 'deliveries')})</div>
                        </div>
                      </div>

                      {courierDetails.history.length > 0 && (
                        <div>
                          <h4 className="text-sm font-semibold mb-2">{t('admin.recent_deliveries', 'Recent Deliveries')}</h4>
                          <div className="space-y-2 max-h-64 overflow-auto">
                            {courierDetails.history.map(h => (
                              <div key={h.id} className="px-3 py-2 rounded-lg bg-[var(--brand-surface)] border border-[var(--brand-border)] text-sm">
                                <div className="flex items-center justify-between gap-2 mb-1">
                                  <motion.button onClick={(e) => { e.stopPropagation(); setSelectedOrderDetail(h); }} whileTap={{ scale: 0.97 }}
                                    className="text-xs font-mono text-[var(--brand-primary)] hover:underline font-medium">#{h.order_id.slice(0, 8)}</motion.button>
                                  <span className="font-medium"><PriceDisplay amount={h.total || 0} /></span>
                                </div>
                                <div className="flex items-center gap-2 text-xs text-[var(--brand-text-muted)]">
                                  <i className="ti ti-user" /> {h.customer_name}
                                  {h.customer_phone && <><span>·</span><i className="ti ti-phone" /> {h.customer_phone}</>}
                                </div>
                                {h.delivery_address && (
                                  <div className="flex items-center gap-1 text-xs text-[var(--brand-text-muted)] mt-0.5">
                                    <i className="ti ti-map-pin" /> {h.delivery_address}
                                  </div>
                                )}
                                <div className="flex items-center gap-2 text-[11px] text-[var(--brand-text-muted)] mt-1.5 pt-1.5 border-t border-[var(--brand-border)]">
                                  <span className="font-medium" style={{ color: 'var(--brand-text)' }}>{t('admin.timing', 'Timing')}:</span>
                                  <span>{t('admin.assigned', 'Assigned')} {new Date(h.assigned_at).toLocaleString()}</span>
                                  {h.accepted_at && <><span>·</span><span>{t('admin.accepted', 'Accepted')} {new Date(h.accepted_at).toLocaleString()}</span></>}
                                  {h.picked_up_at && <><span>·</span><span>{t('admin.picked_up', 'Picked up')} {new Date(h.picked_up_at).toLocaleString()}</span></>}
                                  {h.delivered_at && <><span>·</span><span>{t('admin.delivered', 'Delivered')} {new Date(h.delivered_at).toLocaleString()}</span></>}
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                      {courierDetails.shifts.length > 0 && (
                        <div>
                          <h4 className="text-sm font-semibold mb-2">{t('admin.recent_shifts', 'Recent Shifts')}</h4>
                          <div className="flex flex-wrap gap-2">
                            {courierDetails.shifts.slice(0, 5).map(s => (
                              <span key={s.id} className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium border"
                                style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface)' }}>
                                <span className={`w-1.5 h-1.5 rounded-full ${s.status === 'available' ? 'bg-[var(--color-success)]' : s.status === 'on_delivery' ? 'bg-[var(--color-warning)]' : 'bg-[var(--brand-text-muted)]'}`} />
                                {s.status} {s.started_at ? new Date(s.started_at).toLocaleDateString() : ''}
                              </span>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  ) : (
                    <div className="text-sm text-[var(--brand-text-muted)]">{t('common.error', 'Failed to load details')}                    </div>
                  )}
                </div>
              )}
            </motion.div>
          ))}
        </motion.div>
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

      {/* Order Detail Modal */}
      {selectedOrderDetail && (
        <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center fade-in" role="dialog" aria-modal="true">
          <button type="button" className="absolute inset-0 bg-black/50 backdrop-blur-sm cursor-default" aria-label="Close" onClick={() => setSelectedOrderDetail(null)} />
          <div className="relative w-full max-w-md bg-[var(--brand-surface)] rounded-t-2xl sm:rounded-2xl p-6 space-y-4 z-10 slide-in-up">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>
                {t('admin.order_details', 'Order Details')} #{selectedOrderDetail.order_id.slice(0, 8)}
              </h3>
              <motion.button onClick={() => setSelectedOrderDetail(null)} whileTap={{ scale: 0.97 }} className="w-8 h-8 flex items-center justify-center rounded-md hover:bg-[var(--brand-surface-raised)]">
                <i className="ti ti-x" style={{ color: 'var(--brand-text-muted)' }} />
              </motion.button>
            </div>

            <div className="space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <div className="p-3 rounded-lg border text-center" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
                  <div className="text-lg font-bold" style={{ color: 'var(--brand-primary)' }}><PriceDisplay amount={selectedOrderDetail.total || 0} /></div>
                  <div className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('cart.total', 'Total')}</div>
                </div>
                <div className="p-3 rounded-lg border text-center" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
                  <div className="text-lg font-bold" style={{ color: 'var(--color-success)' }}><PriceDisplay amount={selectedOrderDetail.cash_amount || 0} /></div>
                  <div className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('cart.delivery_fee', 'Delivery Fee')}</div>
                </div>
              </div>

              <div className="p-3 rounded-lg border" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
                <h4 className="text-xs font-semibold mb-2" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.status_timeline', 'Status Timeline')}</h4>
                <div className="space-y-2">
                  {[
                    { label: t('admin.assigned', 'Assigned'), time: selectedOrderDetail.assigned_at, icon: 'ti ti-user-plus' },
                    { label: t('admin.accepted', 'Accepted'), time: selectedOrderDetail.accepted_at, icon: 'ti ti-circle-check' },
                    { label: t('admin.picked_up', 'Picked up'), time: selectedOrderDetail.picked_up_at, icon: 'ti ti-package' },
                    { label: t('admin.delivered', 'Delivered'), time: selectedOrderDetail.delivered_at, icon: 'ti ti-map-pin-check' },
                  ].filter(s => s.time).map((s, i, arr) => (
                    <div key={s.label} className="flex items-center gap-3">
                      <div className="flex flex-col items-center">
                        <div className="w-6 h-6 rounded-full flex items-center justify-center" style={{ background: 'var(--brand-primary-light)' }}>
                          <i className={s.icon} style={{ fontSize: '0.7rem', color: 'var(--brand-primary)' }} />
                        </div>
                        {i < arr.length - 1 && <div className="w-px h-4" style={{ background: 'var(--brand-border)' }} />}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>{s.label}</div>
                        <div className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{new Date(s.time!).toLocaleString()}</div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              <div className="text-xs space-y-1" style={{ color: 'var(--brand-text-muted)' }}>
                <div className="flex items-center gap-1"><i className="ti ti-user" /> {selectedOrderDetail.customer_name}</div>
                {selectedOrderDetail.customer_phone && <div className="flex items-center gap-1"><i className="ti ti-phone" /> {selectedOrderDetail.customer_phone}</div>}
                {selectedOrderDetail.delivery_address && <div className="flex items-center gap-1"><i className="ti ti-map-pin" /> {selectedOrderDetail.delivery_address}</div>}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
