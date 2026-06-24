import { useState, useEffect } from 'react';
import { Button, EmptyState, SkeletonBase, useI18n, useConfirm, Toggle, ResponsiveDialog } from '@deliveryos/ui';
import { PromotionSchema, PromotionListResponse, CreatePromotionSchema } from '@deliveryos/shared-types';
import { apiClient } from '../../lib/index.js';
import type { z } from 'zod';

type Promotion = z.infer<typeof PromotionSchema>;
type CreatePromotion = z.infer<typeof CreatePromotionSchema>;

const emptyForm: CreatePromotion = {
  code: '',
  type: 'percentage',
  discount_value: 0,
  min_order_amount: 0,
  valid_from: '',
  valid_until: null,
  max_uses: null,
  description: null,
  applicable_product_ids: [],
  max_uses_per_customer: 1,
  is_active: true,
};

const typeMeta: Record<string, { label: string; icon: string }> = {
  percentage: { label: '%', icon: 'ti ti-percentage' },
  fixed: { label: 'ALL', icon: 'ti ti-currency-cent' },
  free_delivery: { label: '', icon: 'ti ti-truck-delivery' },
};

function PromotionForm({
  initial,
  onSave,
  onCancel,
}: {
  initial?: Promotion | null;
  onSave: (data: CreatePromotion) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [form, setForm] = useState<CreatePromotion>(() => {
    if (!initial) return { ...emptyForm };
    return {
      code: initial.code,
      type: initial.type,
      discount_value: initial.discount_value,
      min_order_amount: initial.min_order_amount,
      valid_from: initial.valid_from.slice(0, 16),
      valid_until: initial.valid_until ? initial.valid_until.slice(0, 16) : '',
      max_uses: initial.max_uses ?? null,
      description: initial.description ?? '',
      applicable_product_ids: initial.applicable_product_ids,
      max_uses_per_customer: initial.max_uses_per_customer,
      is_active: initial.is_active,
    };
  });
  const [saving, setSaving] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validate = (): boolean => {
    const e: Record<string, string> = {};
    if (!form.code.trim()) e.code = t('promotions.code_required', 'Code is required');
    if (form.discount_value <= 0) e.discount_value = t('promotions.discount_required', 'Discount must be greater than 0');
    if (!form.valid_from) e.valid_from = t('promotions.valid_from_required', 'Start date is required');
    if (form.type === 'percentage' && form.discount_value > 100)
      e.discount_value = t('promotions.percentage_max', 'Percentage cannot exceed 100');
    setErrors(e);
    return Object.keys(e).length === 0;
  };

  const handleSave = () => {
    if (!validate()) return;
    setSaving(true);
    const payload: CreatePromotion = {
      ...form,
      valid_from: new Date(form.valid_from).toISOString(),
      valid_until: form.valid_until ? new Date(form.valid_until).toISOString() : null,
      description: form.description?.trim() || null,
    };
    onSave(payload);
    setTimeout(() => setSaving(false), 200);
  };

  const set = <K extends keyof CreatePromotion>(key: K, value: CreatePromotion[K]) => {
    setForm(prev => ({ ...prev, [key]: value }));
    if (errors[key as string]) setErrors(prev => { const { [key as string]: _, ...rest } = prev; return rest; });
  };

  return (
    <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-xl flex flex-col max-h-[85vh]">
      <div className="overflow-y-auto p-5 space-y-4 slide-in-up">
        <h3 className="text-sm font-semibold" style={{ fontFamily: 'var(--brand-font-heading)' }}>
          {initial ? `${t('common.edit')}: ${form.code}` : t('promotions.create', 'Create Promotion')}
        </h3>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('promotions.code', 'Code')} *</label>
            <input value={form.code} onChange={e => set('code', e.target.value.toUpperCase())} placeholder="SUMMER20"
              className="w-full h-10 px-3 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors font-mono uppercase"
              style={{ background: 'var(--brand-surface-raised)', borderColor: errors.code ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
            {errors.code && <span className="text-[10px] mt-0.5 block" style={{ color: 'var(--color-danger)' }}>{errors.code}</span>}
          </div>
          <div>
            <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('promotions.type', 'Type')} *</label>
            <select value={form.type} onChange={e => set('type', e.target.value as any)}
              className="w-full h-10 px-3 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors"
              style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
              <option value="percentage">{t('promotions.percentage', 'Percentage')}</option>
              <option value="fixed">{t('promotions.fixed', 'Fixed amount')}</option>
              <option value="free_delivery">{t('promotions.free_delivery', 'Free delivery')}</option>
            </select>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>
              {form.type === 'percentage' ? t('promotions.discount_percentage', 'Discount (%)') : form.type === 'fixed' ? t('promotions.discount_amount', 'Discount amount (ALL)') : t('promotions.discount_value', 'Discount value')} *
            </label>
            <input value={form.discount_value || ''} onChange={e => set('discount_value', parseInt(e.target.value) || 0)} type="number" min={0}
              className="w-full h-10 px-3 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors"
              style={{ background: 'var(--brand-surface-raised)', borderColor: errors.discount_value ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
            {errors.discount_value && <span className="text-[10px] mt-0.5 block" style={{ color: 'var(--color-danger)' }}>{errors.discount_value}</span>}
          </div>
          <div>
            <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('promotions.min_order', 'Min. order (ALL)')}</label>
            <input value={form.min_order_amount || ''} onChange={e => set('min_order_amount', parseInt(e.target.value) || 0)} type="number" min={0}
              className="w-full h-10 px-3 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors"
              style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('promotions.valid_from', 'Valid from')} *</label>
            <input value={form.valid_from} onChange={e => set('valid_from', e.target.value)} type="datetime-local"
              className="w-full h-10 px-3 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors"
              style={{ background: 'var(--brand-surface-raised)', borderColor: errors.valid_from ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
            {errors.valid_from && <span className="text-[10px] mt-0.5 block" style={{ color: 'var(--color-danger)' }}>{errors.valid_from}</span>}
          </div>
          <div>
            <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('promotions.valid_until', 'Valid until')}</label>
            <input value={form.valid_until || ''} onChange={e => set('valid_until', e.target.value || null)} type="datetime-local"
              className="w-full h-10 px-3 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors"
              style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('promotions.max_uses', 'Max uses')}</label>
            <input value={form.max_uses ?? ''} onChange={e => set('max_uses', e.target.value ? parseInt(e.target.value) : null)} type="number" min={0} placeholder={t('common.unlimited', 'Unlimited')}
              className="w-full h-10 px-3 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors"
              style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          </div>
        </div>

        <div>
          <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('promotions.description', 'Description')}</label>
          <textarea value={form.description ?? ''} onChange={e => set('description', e.target.value)} rows={3} maxLength={500}
            placeholder={t('promotions.description_placeholder', 'e.g. Summer special - 20% off all sushi rolls')}
            className="w-full px-3 py-2 rounded-lg border text-sm outline-none focus:border-[var(--brand-primary)] transition-colors resize-none"
            style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
        </div>
      </div>
      <div className="flex justify-end gap-2 p-5 pt-3 border-t shrink-0" style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface)' }}>
        <Button variant="ghost" size="sm" onClick={onCancel}>{t('common.cancel')}</Button>
        <Button size="sm" loading={saving} onClick={handleSave}>{t('common.save')}</Button>
      </div>
    </div>
  );
}

function formatDateTime(iso: string): string {
  try { return new Date(iso).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' }); } catch { return iso; }
}

export function PromotionsPage() {
  const { t } = useI18n();
  const { confirm: rmConfirm, dialog: rmDialog } = useConfirm();

  const [promotions, setPromotions] = useState<Promotion[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState<Promotion | null>(null);
  const [showCreate, setShowCreate] = useState(false);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiClient<typeof PromotionListResponse>('/owner/promotions', { schema: PromotionListResponse });
      setPromotions(data.promotions);
    } catch (err: any) {
      // Keep the raw server detail in the console only — never surface a raw string
      // like "Internal server error" to the owner.
      console.error('[Promotions] load failed:', err);
      setError(t('promotions.load_error', 'Could not load promotions. Please try again.'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  useEffect(() => {
    if (editing) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => { document.body.style.overflow = ''; };
  }, [editing]);

  const handleCreate = async (data: CreatePromotion) => {
    try {
      await apiClient('/owner/promotions', { method: 'POST', body: data });
      setShowCreate(false);
      await load();
    } catch (err: any) {
      console.error('[Promotions] create failed:', err);
    }
  };

  const handleEdit = async (data: CreatePromotion) => {
    if (!editing) return;
    try {
      await apiClient(`/owner/promotions/${editing.id}`, { method: 'PATCH', body: data });
      setEditing(null);
      await load();
    } catch (err: any) {
      console.error('[Promotions] edit failed:', err);
    }
  };

  const handleToggleActive = async (p: Promotion) => {
    try {
      await apiClient(`/owner/promotions/${p.id}`, { method: 'PATCH', body: { is_active: !p.is_active } });
      setPromotions(prev => prev.map(x => x.id === p.id ? { ...x, is_active: !x.is_active } : x));
    } catch (err: any) {
      console.error('[Promotions] toggle failed:', err);
    }
  };

  const handleDelete = async (p: Promotion) => {
    const ok = await rmConfirm({
      title: t('promotions.confirm_delete_title', 'Delete promotion'),
      message: t('promotions.confirm_delete', 'Are you sure you want to delete {code}? This action cannot be undone.').replace('{code}', p.code),
      confirmLabel: t('common.delete', 'Delete'),
      variant: 'danger',
    });
    if (!ok) return;
    try {
      await apiClient(`/owner/promotions/${p.id}`, { method: 'DELETE' });
      setPromotions(prev => prev.filter(x => x.id !== p.id));
    } catch (err: any) {
      console.error('[Promotions] delete failed:', err);
    }
  };

  const now = new Date();
  const activeCount = promotions.filter(p => p.is_active && (!p.valid_until || new Date(p.valid_until) > now)).length;
  const totalCount = promotions.length;

  return (
    <>
      <div className="p-4 md:p-6 max-w-6xl mx-auto space-y-5">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('admin.promotions', 'Promotions')}</h2>
            {!loading && (
              <p className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>
                {activeCount} {t('promotions.active_of', 'active')} {t('common.of')} {totalCount}
              </p>
            )}
          </div>
          <Button onClick={() => { setShowCreate(true); setEditing(null); }}>
            <i className="ti ti-plus" /> {t('promotions.create', 'Create Promotion')}
          </Button>
        </div>

        {showCreate && (
          <PromotionForm onSave={handleCreate} onCancel={() => setShowCreate(false)} />
        )}

        {loading ? (
          <div className="space-y-2">{[1,2,3,4,5].map(i => <SkeletonBase key={i} className="h-24 w-full rounded-xl" />)}</div>
        ) : error ? (
          <div className="flex flex-col items-center justify-center py-16 gap-4">
            <i className="ti ti-alert-circle text-4xl" style={{ color: 'var(--color-danger)', opacity: 0.5 }} />
            <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{error}</p>
            <Button variant="outline" size="sm" onClick={load}><i className="ti ti-refresh" /> {t('common.retry')}</Button>
          </div>
        ) : promotions.length === 0 ? (
          <div data-testid="empty-state">
            <EmptyState
              title={t('promotions.empty_title', 'No promotions yet')}
              description={t('promotions.empty_desc', 'Create your first promotion to start offering discounts to your customers.')}
              icon={<i className="ti ti-ticket text-4xl" style={{ opacity: 0.3 }} />}
            />
          </div>
        ) : (
          <div className="space-y-2" data-testid="promotions-list">
            {editing && (
              <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center fade-in" role="dialog" aria-modal="true">
                <button type="button" className="absolute inset-0 bg-black/50 backdrop-blur-sm cursor-default" aria-label={t('common.close', 'Close')} onClick={() => setEditing(null)} />
                <div className="relative w-full max-w-lg mx-4 mb-0 sm:mb-auto rounded-t-2xl sm:rounded-2xl">
                  <PromotionForm initial={editing} onSave={handleEdit} onCancel={() => setEditing(null)} />
                </div>
              </div>
            )}
            {promotions.map((p, i) => {
              const meta = typeMeta[p.type] || { label: '', icon: 'ti ti-ticket' };
              const isExpired = p.valid_until && new Date(p.valid_until) < now;
              const usedRatio = p.max_uses ? Math.round((p.current_uses / p.max_uses) * 100) : null;
              return (
                <div key={p.id} data-testid="promotion-card"
                  className={`flex items-start gap-4 p-4 rounded-xl border transition-all duration-200 hover:bg-[var(--brand-surface)] slide-in-up ${!p.is_active || isExpired ? 'opacity-60' : ''}`}
                  style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', animationDelay: `${i * 30}ms` }}>
                  <div className="w-11 h-11 rounded-xl flex items-center justify-center shrink-0" style={{ background: p.is_active && !isExpired ? 'var(--brand-primary-light)' : 'var(--brand-surface-raised)' }}>
                    <i className={meta.icon} style={{ fontSize: '1.1rem', color: p.is_active && !isExpired ? 'var(--brand-primary)' : 'var(--brand-text-muted)' }} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-sm font-mono font-bold tracking-wider" style={{ color: 'var(--brand-text)' }}>{p.code}</span>
                      <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${p.type === 'free_delivery' ? 'bg-[var(--color-info-light)] text-[var(--color-info)]' : p.type === 'percentage' ? 'bg-[var(--color-success-light)] text-[var(--color-success)]' : 'bg-[var(--color-warning-light)] text-[var(--color-warning)]'}`}>
                        {p.type === 'percentage' ? `${p.discount_value}%` : p.type === 'fixed' ? `${(p.discount_value / 100).toFixed(0)} ALL` : t('promotions.free_delivery_short', 'Free delivery')}
                      </span>
                      {isExpired && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded font-medium" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                          {t('promotions.expired', 'Expired')}
                        </span>
                      )}
                      {usedRatio !== null && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded font-medium" style={{ background: usedRatio >= 90 ? 'rgba(239,68,68,0.1)' : 'var(--brand-surface-raised)', color: usedRatio >= 90 ? 'var(--color-danger)' : 'var(--brand-text-muted)' }}>
                          {p.current_uses}/{p.max_uses}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-[11px] mt-1" style={{ color: 'var(--brand-text-muted)' }}>
                      <i className="ti ti-calendar" style={{ fontSize: '0.7rem' }} />
                      <span>{t('promotions.from', 'From')} {formatDateTime(p.valid_from)}</span>
                      {p.valid_until && <><span>·</span><span>{t('promotions.until', 'until')} {formatDateTime(p.valid_until)}</span></>}
                    </div>
                    {p.min_order_amount > 0 && (
                      <div className="flex items-center gap-1 text-[11px] mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>
                        <i className="ti ti-shopping-cart" style={{ fontSize: '0.7rem' }} />
                        <span>{t('promotions.min_order', 'Min. order')}: {(p.min_order_amount / 100).toFixed(0)} ALL</span>
                      </div>
                    )}
                    {p.description && (
                      <p className="text-[11px] mt-1 line-clamp-1" style={{ color: 'var(--brand-text-muted)' }}>{p.description}</p>
                    )}
                  </div>
                  <div className="flex flex-col items-center gap-2 shrink-0">
                    <Toggle
                      checked={p.is_active}
                      onChange={() => handleToggleActive(p)}
                      aria-label={t('promotions.toggle_active', 'Toggle active')}
                    />
                    <div className="flex items-center gap-1">
                      <button onClick={() => setEditing(p)} className="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-[var(--brand-surface-raised)] transition-colors" title={t('common.edit', 'Edit')}>
                        <i className="ti ti-edit" style={{ fontSize: '0.85rem', color: 'var(--brand-text-muted)' }} />
                      </button>
                      <button onClick={() => handleDelete(p)} className="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-[var(--color-danger-light)] transition-colors" title={t('common.delete', 'Delete')}>
                        <i className="ti ti-trash" style={{ fontSize: '0.85rem', color: 'var(--brand-text-muted)' }} />
                      </button>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
      {rmDialog}
    </>
  );
}
