import { useState, useEffect } from 'react';
import { motion, AnimatePresence, useReducedMotion } from 'framer-motion';
import { Button, EmptyState, SkeletonBase, useI18n, useConfirm, Toggle, Select, Textarea, ease, duration, useToast, ResponsiveDialog } from '@deliveryos/ui';
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
  bare = false,
}: {
  initial?: Promotion | null;
  onSave: (data: CreatePromotion) => Promise<boolean> | void;
  onCancel: () => void;
  // S5 fix: when rendered inside ResponsiveDialog (which already supplies its own
  // card chrome + scroll region + padding), skip this component's own outer
  // bg/rounded/shadow wrapper and sticky footer so we don't nest a card in a card.
  // The standalone (non-modal) "create" usage keeps the original full wrapper.
  bare?: boolean;
}) {
  const { t } = useI18n();
  const prefersReducedMotion = useReducedMotion();
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
  const [saveError, setSaveError] = useState<string | null>(null);
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

  const handleSave = async () => {
    if (!validate()) return;
    setSaving(true);
    setSaveError(null);
    const payload: CreatePromotion = {
      ...form,
      valid_from: new Date(form.valid_from).toISOString(),
      valid_until: form.valid_until ? new Date(form.valid_until).toISOString() : null,
      description: form.description?.trim() || null,
    };
    const ok = await onSave(payload);
    if (ok === false) {
      setSaveError(t('promotions.save_error', 'Could not save this promotion. Please check the details and try again.'));
      setSaving(false);
      return;
    }
    setSaving(false);
  };

  const set = <K extends keyof CreatePromotion>(key: K, value: CreatePromotion[K]) => {
    setForm(prev => ({ ...prev, [key]: value }));
    if (errors[key as string]) setErrors(prev => { const { [key as string]: _, ...rest } = prev; return rest; });
  };

  const inputClass = 'w-full h-11 px-3 rounded-[var(--brand-radius-sm)] border text-sm outline-none transition-[border-color,box-shadow] duration-[var(--motion-fast,150ms)] ease-[var(--ease-soft,ease)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--brand-surface)] focus:border-[var(--brand-primary)]';
  const labelClass = 'text-xs font-medium block mb-1';

  const titleBlock = (
    <h3 className="text-base font-semibold" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' }}>
      {initial ? `${t('common.edit')}: ${form.code}` : t('promotions.create', 'Create Promotion')}
    </h3>
  );

  const errorBlock = saveError && (
    <div className="flex items-start gap-2 px-3 py-2 rounded-[var(--brand-radius-sm)] text-xs" role="alert"
      style={{ background: 'var(--color-danger-light)', color: 'var(--color-danger)' }}>
      <i className="ti ti-alert-circle shrink-0" style={{ fontSize: '0.9rem', marginTop: '1px' }} />
      <span className="min-w-0">{saveError}</span>
    </div>
  );

  const fields = (
    <>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div className="min-w-0">
            <label className={labelClass} style={{ color: 'var(--brand-text)' }}>{t('promotions.code', 'Code')} *</label>
            <input value={form.code} onChange={e => set('code', e.target.value.toUpperCase())} placeholder="SUMMER20"
              aria-invalid={!!errors.code}
              className={`${inputClass} font-mono uppercase`}
              style={{ background: 'var(--brand-surface-raised)', borderColor: errors.code ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
            {errors.code && <span className="text-xs mt-1 block" style={{ color: 'var(--color-danger)' }}>{errors.code}</span>}
          </div>
          <div className="min-w-0">
            <label className={labelClass} style={{ color: 'var(--brand-text)' }}>{t('promotions.type', 'Type')} *</label>
            <Select value={form.type} onChange={e => set('type', e.target.value as any)}>
              <option value="percentage">{t('promotions.percentage', 'Percentage')}</option>
              <option value="fixed">{t('promotions.fixed', 'Fixed amount')}</option>
              <option value="free_delivery">{t('promotions.free_delivery', 'Free delivery')}</option>
            </Select>
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div className="min-w-0">
            <label className={labelClass} style={{ color: 'var(--brand-text)' }}>
              {form.type === 'percentage' ? t('promotions.discount_percentage', 'Discount (%)') : form.type === 'fixed' ? t('promotions.discount_amount', 'Discount amount (ALL)') : t('promotions.discount_value', 'Discount value')} *
            </label>
            <input value={form.discount_value || ''} onChange={e => set('discount_value', parseInt(e.target.value) || 0)} type="number" min={0}
              aria-invalid={!!errors.discount_value}
              className={inputClass}
              style={{ background: 'var(--brand-surface-raised)', borderColor: errors.discount_value ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
            {errors.discount_value && <span className="text-xs mt-1 block" style={{ color: 'var(--color-danger)' }}>{errors.discount_value}</span>}
          </div>
          <div className="min-w-0">
            <label className={labelClass} style={{ color: 'var(--brand-text)' }}>{t('promotions.min_order', 'Min. order (ALL)')}</label>
            <input value={form.min_order_amount || ''} onChange={e => set('min_order_amount', parseInt(e.target.value) || 0)} type="number" min={0}
              className={inputClass}
              style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div className="min-w-0">
            <label className={labelClass} style={{ color: 'var(--brand-text)' }}>{t('promotions.valid_from', 'Valid from')} *</label>
            <input value={form.valid_from} onChange={e => set('valid_from', e.target.value)} type="datetime-local"
              aria-invalid={!!errors.valid_from}
              className={inputClass}
              style={{ background: 'var(--brand-surface-raised)', borderColor: errors.valid_from ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
            {errors.valid_from && <span className="text-xs mt-1 block" style={{ color: 'var(--color-danger)' }}>{errors.valid_from}</span>}
          </div>
          <div className="min-w-0">
            <label className={labelClass} style={{ color: 'var(--brand-text)' }}>{t('promotions.valid_until', 'Valid until')}</label>
            <input value={form.valid_until || ''} onChange={e => set('valid_until', e.target.value || null)} type="datetime-local"
              className={inputClass}
              style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          </div>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div className="min-w-0">
            <label className={labelClass} style={{ color: 'var(--brand-text)' }}>{t('promotions.max_uses', 'Max uses')}</label>
            <input value={form.max_uses ?? ''} onChange={e => set('max_uses', e.target.value ? parseInt(e.target.value) : null)} type="number" min={0} placeholder={t('common.unlimited', 'Unlimited')}
              className={inputClass}
              style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
          </div>
        </div>

        <div className="min-w-0">
          <label className={labelClass} style={{ color: 'var(--brand-text)' }}>{t('promotions.description', 'Description')}</label>
          <Textarea value={form.description ?? ''} onChange={e => set('description', e.target.value)} rows={3} maxLength={500}
            placeholder={t('promotions.description_placeholder', 'e.g. Summer special - 20% off all sushi rolls')} />
        </div>
    </>
  );

  const footer = (
    <>
      <Button variant="ghost" size="sm" onClick={onCancel}>{t('common.cancel')}</Button>
      <Button size="sm" loading={saving} onClick={handleSave}>{t('common.save')}</Button>
    </>
  );

  // S5 fix: `bare` (used when this form is the child of ResponsiveDialog, which
  // already renders its own card chrome + scroll region) skips this component's own
  // outer bg/rounded/shadow wrapper and sticky border-t footer — everything else
  // (title text, field markup, button handlers/order) is identical to the
  // standalone "create" rendering below.
  if (bare) {
    return (
      <div className="space-y-4">
        {titleBlock}
        {errorBlock}
        {fields}
        <div className="flex justify-end gap-2 pt-2">{footer}</div>
      </div>
    );
  }

  return (
    <div className="bg-[var(--brand-surface)] rounded-[var(--brand-radius)] flex flex-col max-h-[85vh] shadow-[var(--elev-4)]">
      <div className="overflow-y-auto p-5 space-y-4">
        {titleBlock}
        {errorBlock}
        {fields}
      </div>
      <div className="flex justify-end gap-2 p-5 pt-3 border-t shrink-0" style={{ borderColor: 'var(--brand-border)', background: 'var(--brand-surface)' }}>
        {footer}
      </div>
    </div>
  );
}

function formatDateTime(iso: string): string {
  try { return new Date(iso).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' }); } catch { return iso; }
}

export function PromotionsPage() {
  const { t } = useI18n();
  const prefersReducedMotion = useReducedMotion();
  const { confirm: rmConfirm, dialog: rmDialog } = useConfirm();
  const { showToast } = useToast();

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

  // S5 fix: body-scroll-lock while the edit modal is open now lives inside
  // ResponsiveDialog (keyed off its own `open` prop) — this page no longer needs
  // its own copy of that effect.

  const handleCreate = async (data: CreatePromotion): Promise<boolean> => {
    try {
      await apiClient('/owner/promotions', { method: 'POST', body: data });
      setShowCreate(false);
      await load();
      return true;
    } catch (err: any) {
      console.error('[Promotions] create failed:', err);
      return false;
    }
  };

  const handleEdit = async (data: CreatePromotion): Promise<boolean> => {
    if (!editing) return false;
    try {
      await apiClient(`/owner/promotions/${editing.id}`, { method: 'PATCH', body: data });
      setEditing(null);
      await load();
      return true;
    } catch (err: any) {
      console.error('[Promotions] edit failed:', err);
      return false;
    }
  };

  const handleToggleActive = async (p: Promotion) => {
    try {
      await apiClient(`/owner/promotions/${p.id}`, { method: 'PATCH', body: { is_active: !p.is_active } });
      // Not optimistic — state only flips after the PATCH resolves, so there is
      // nothing to roll back on failure below.
      setPromotions(prev => prev.map(x => x.id === p.id ? { ...x, is_active: !x.is_active } : x));
    } catch (err: any) {
      // S4 fix: this used to fail silently (console-only) while the Toggle still
      // visually showed the pre-toggle state with no error — the owner had no way
      // to tell the request had failed.
      console.error('[Promotions] toggle failed:', err);
      showToast(t('promotions.toggle_error', 'Could not update promotion status. Please try again.'), 'error');
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
      // Not optimistic — the row is only removed from the list after the DELETE
      // resolves, so there is nothing to restore on failure below.
      setPromotions(prev => prev.filter(x => x.id !== p.id));
    } catch (err: any) {
      // S4 fix: this used to fail silently (console-only) — the confirm dialog
      // had already closed, so the owner saw nothing and could believe the
      // promotion was deleted when the row was still there on next reload.
      console.error('[Promotions] delete failed:', err);
      showToast(t('common.error_delete', 'Failed to delete.'), 'error');
    }
  };

  const now = new Date();
  const activeCount = promotions.filter(p => p.is_active && (!p.valid_until || new Date(p.valid_until) > now)).length;
  const totalCount = promotions.length;

  return (
    <>
      <div className="p-4 md:p-6 max-w-6xl mx-auto space-y-5">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div className="min-w-0">
            <h2 className="text-2xl font-bold truncate" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' }}>{t('admin.promotions', 'Promotions')}</h2>
            {!loading && !error && totalCount > 0 && (
              <p className="text-xs mt-0.5" style={{ color: 'var(--brand-text)' }}>
                {activeCount} {t('promotions.active_of', 'active')} {t('common.of')} {totalCount}
              </p>
            )}
          </div>
          <Button className="shrink-0" onClick={() => { setShowCreate(true); setEditing(null); }}>
            <i className="ti ti-plus" /> {t('promotions.create', 'Create Promotion')}
          </Button>
        </div>

        <AnimatePresence initial={false}>
          {showCreate && (
            <motion.div
              initial={{ opacity: 0, y: prefersReducedMotion ? 0 : -8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: prefersReducedMotion ? 0 : -8 }}
              transition={{ duration: prefersReducedMotion ? 0 : duration.base, ease: ease.out }}>
              <PromotionForm onSave={handleCreate} onCancel={() => setShowCreate(false)} />
            </motion.div>
          )}
        </AnimatePresence>

        {loading ? (
          <div className="space-y-2">{[1,2,3,4,5].map(i => <SkeletonBase key={i} className="h-24 w-full rounded-[var(--brand-radius)]" />)}</div>
        ) : error ? (
          <div className="flex flex-col items-center justify-center py-16 gap-4 text-center">
            <i className="ti ti-alert-circle text-4xl" style={{ color: 'var(--color-danger)', opacity: 0.5 }} />
            <p className="text-sm max-w-sm" style={{ color: 'var(--brand-text)' }}>{error}</p>
            <Button variant="outline" size="sm" onClick={load}><i className="ti ti-refresh" /> {t('common.retry')}</Button>
          </div>
        ) : promotions.length === 0 ? (
          <div data-testid="empty-state">
            <EmptyState
              title={t('promotions.empty_title', 'No promotions yet')}
              description={t('promotions.empty_desc', 'Create your first promotion to start offering discounts to your customers.')}
              icon={<i className="ti ti-ticket text-4xl" style={{ opacity: 0.3 }} />}
              action={
                <Button onClick={() => { setShowCreate(true); setEditing(null); }}>
                  <i className="ti ti-plus" /> {t('promotions.create', 'Create Promotion')}
                </Button>
              }
            />
          </div>
        ) : (
          <motion.div className="space-y-2" data-testid="promotions-list"
            variants={{ visible: { transition: { staggerChildren: prefersReducedMotion ? 0 : 0.03 } } }}
            initial="hidden" animate="visible">
            {promotions.map((p, i) => {
              const meta = typeMeta[p.type] || { label: '', icon: 'ti ti-ticket' };
              const isExpired = p.valid_until && new Date(p.valid_until) < now;
              const usedRatio = p.max_uses ? Math.round((p.current_uses / p.max_uses) * 100) : null;
              return (
                <motion.div key={p.id} data-testid="promotion-card"
                  variants={{ hidden: { opacity: 0, y: prefersReducedMotion ? 0 : 8 }, visible: { opacity: 1, y: 0, transition: { duration: prefersReducedMotion ? 0 : duration.base, ease: ease.out } } }}
                  whileHover={prefersReducedMotion ? undefined : { y: -2 }}
                  className={`flex items-start gap-4 p-4 rounded-[var(--brand-radius)] border transition-[box-shadow] duration-[var(--motion-fast,150ms)] ease-[var(--ease-soft,ease)] shadow-[var(--elev-1)] [@media(hover:hover)]:hover:shadow-[var(--elev-2)] ${!p.is_active || isExpired ? 'opacity-60' : ''}`}
                  style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
                  <div className="w-11 h-11 rounded-[var(--brand-radius-sm)] flex items-center justify-center shrink-0" style={{ background: p.is_active && !isExpired ? 'var(--brand-primary-light)' : 'var(--brand-surface-raised)' }}>
                    <i className={meta.icon} style={{ fontSize: '1.1rem', color: p.is_active && !isExpired ? 'var(--brand-primary)' : 'var(--brand-text-muted)' }} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="text-sm font-mono font-bold tracking-wider truncate max-w-[12rem]" style={{ color: 'var(--brand-text)' }}>{p.code}</span>
                      <span className={`text-xs px-1.5 py-0.5 rounded-[var(--brand-radius-sm)] font-medium whitespace-nowrap ${p.type === 'free_delivery' ? 'bg-[var(--color-info-light)] text-[var(--color-info)]' : p.type === 'percentage' ? 'bg-[var(--color-success-light)] text-[var(--color-success)]' : 'bg-[var(--color-warning-light)] text-[var(--color-warning)]'}`}>
                        {p.type === 'percentage' ? `${p.discount_value}%` : p.type === 'fixed' ? `${(p.discount_value / 100).toFixed(0)} ALL` : t('promotions.free_delivery_short', 'Free delivery')}
                      </span>
                      {isExpired && (
                        <span className="text-xs px-1.5 py-0.5 rounded-[var(--brand-radius-sm)] font-medium whitespace-nowrap" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                          {t('promotions.expired', 'Expired')}
                        </span>
                      )}
                      {usedRatio !== null && (
                        <span className="text-xs px-1.5 py-0.5 rounded-[var(--brand-radius-sm)] font-medium whitespace-nowrap" style={{ background: usedRatio >= 90 ? 'var(--color-danger-light)' : 'var(--brand-surface-raised)', color: usedRatio >= 90 ? 'var(--color-danger)' : 'var(--brand-text-muted)' }}>
                          {p.current_uses}/{p.max_uses}
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-xs mt-1 min-w-0" style={{ color: 'var(--brand-text-muted)' }}>
                      <i className="ti ti-calendar shrink-0" style={{ fontSize: '0.75rem' }} />
                      <span className="truncate">{t('promotions.from', 'From')} {formatDateTime(p.valid_from)}{p.valid_until ? ` · ${t('promotions.until', 'until')} ${formatDateTime(p.valid_until)}` : ''}</span>
                    </div>
                    {p.min_order_amount > 0 && (
                      <div className="flex items-center gap-1 text-xs mt-0.5 min-w-0" style={{ color: 'var(--brand-text-muted)' }}>
                        <i className="ti ti-shopping-cart shrink-0" style={{ fontSize: '0.75rem' }} />
                        <span className="truncate">{t('promotions.min_order', 'Min. order')}: {(p.min_order_amount / 100).toFixed(0)} ALL</span>
                      </div>
                    )}
                    {p.description && (
                      <p className="text-xs mt-1 line-clamp-1" style={{ color: 'var(--brand-text-muted)' }}>{p.description}</p>
                    )}
                  </div>
                  <div className="flex flex-col items-center gap-2 shrink-0">
                    <Toggle
                      checked={p.is_active}
                      onChange={() => handleToggleActive(p)}
                      aria-label={t('promotions.toggle_active', 'Toggle active')}
                    />
                    <div className="flex items-center gap-1">
                      <button onClick={() => setEditing(p)} className="w-11 h-11 flex items-center justify-center rounded-[var(--brand-radius-sm)] transition-[background-color,transform] duration-[var(--motion-fast,150ms)] ease-[var(--ease-soft,ease)] [@media(hover:hover)]:hover:bg-[var(--brand-surface-raised)] active:scale-[0.95] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--brand-surface)]" title={t('common.edit', 'Edit')} aria-label={t('common.edit', 'Edit')}>
                        <i className="ti ti-edit" style={{ fontSize: '0.95rem', color: 'var(--brand-text-muted)' }} />
                      </button>
                      <button onClick={() => handleDelete(p)} className="w-11 h-11 flex items-center justify-center rounded-[var(--brand-radius-sm)] transition-[background-color,transform] duration-[var(--motion-fast,150ms)] ease-[var(--ease-soft,ease)] [@media(hover:hover)]:hover:bg-[var(--color-danger-light)] active:scale-[0.95] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-danger)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--brand-surface)]" title={t('common.delete', 'Delete')} aria-label={t('common.delete', 'Delete')}>
                        <i className="ti ti-trash" style={{ fontSize: '0.95rem', color: 'var(--brand-text-muted)' }} />
                      </button>
                    </div>
                  </div>
                </motion.div>
              );
            })}
          </motion.div>
        )}
      </div>

      {/* S5 fix: migrated off the hand-rolled fixed/inset-0-overlay backdrop+dialog
          (no focus trap, no Escape handling) onto the shared ResponsiveDialog
          primitive, which now provides a real focus trap (initial focus lands on
          the Code input — the first focusable element — and Tab/Shift+Tab cycle
          confined to the dialog), Escape-to-close, backdrop click-to-close, and
          body-scroll-lock. `bare` keeps PromotionForm's own title/fields/footer
          content and handlers unchanged, only dropping its now-redundant outer
          card chrome since ResponsiveDialog supplies that. */}
      <ResponsiveDialog open={!!editing} onClose={() => setEditing(null)} className="max-w-lg">
        {editing && <PromotionForm initial={editing} onSave={handleEdit} onCancel={() => setEditing(null)} bare />}
      </ResponsiveDialog>
      {rmDialog}
    </>
  );
}
