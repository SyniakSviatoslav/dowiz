import React, { useState, useEffect, useRef } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { Button, Input, EmptyState, MapWithRadius, Toggle, useI18n, LanguageSwitcher, useToast, ease, duration } from '@deliveryos/ui';
import type { LngLatLike, Locale } from '@deliveryos/ui';
import { PHONE_E164_REGEX, PHONE_E164_PATTERN } from '@deliveryos/shared-types';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

// VITE_TG_CATEGORY_GATING (default off): mirrors the server TG_CATEGORY_GATING flag so
// the category preference-centre stays dark until the dispatcher gates by category.
const CATEGORY_GATING_ENABLED = import.meta.env.VITE_TG_CATEGORY_GATING === 'true';

const OwnerSettingsResponse = z.object({
  id: z.string().optional(),
  name: z.string().optional(),
  locationName: z.string().optional(),
  slug: z.string().optional(),
  phone: z.string().optional(),
  address: z.string().optional(),
}).passthrough();

const NotificationTargetsResponse = z.object({
  targets: z.array(z.any()),
}).passthrough();

const TelegramConnectResponse = z.object({
  deepLink: z.string().optional(),
}).passthrough();

const FallbackConfigResponse = z.object({
  phone: z.string().nullable().optional(),
  showPhoneOnError: z.boolean().optional(),
  showPhoneOnOffline: z.boolean().optional(),
  wsRetryMax: z.number().optional(),
  wsRetryBaseMs: z.number().optional(),
}).passthrough();
import QRCode from 'qrcode';

interface DaySchedule {
  isOpen: boolean;
  open: string;
  close: string;
}
type WeeklySchedule = Record<string, DaySchedule>;

const DEFAULT_SCHEDULE: WeeklySchedule = {
  monday: { isOpen: true, open: '09:00', close: '22:00' },
  tuesday: { isOpen: true, open: '09:00', close: '22:00' },
  wednesday: { isOpen: true, open: '09:00', close: '22:00' },
  thursday: { isOpen: true, open: '09:00', close: '22:00' },
  friday: { isOpen: true, open: '09:00', close: '23:00' },
  saturday: { isOpen: true, open: '10:00', close: '23:00' },
  sunday: { isOpen: true, open: '10:00', close: '22:00' },
};

interface LocationSettings {
  locationName: string;
  phone: string;
  address: string;
  deliveryFee: number;
  minOrder: number;
  radiusKm: number;
  lat: number;
  lng: number;
  hoursJson: WeeklySchedule;
}

const MOCK_SETTINGS: LocationSettings = {
  locationName: 'Downtown Tirana',
  phone: '+35542345678',
  address: 'Rruga Ismail Qemali 45, Tirana',
  deliveryFee: 120,
  minOrder: 500,
  radiusKm: 8,
  lat: 41.331,
  lng: 19.817,
  hoursJson: DEFAULT_SCHEDULE,
};

export function SettingsPage() {
  const { t, locale, setLocale } = useI18n();
  const { showToast } = useToast();
  const reduceMotion = useReducedMotion();
  // Soft-UI section reveal: ease-out fade+rise, collapsed to a crossfade under reduced-motion.
  const sectionVariants = {
    hidden: { opacity: 0, y: reduceMotion ? 0 : 12 },
    visible: { opacity: 1, y: 0, transition: { duration: reduceMotion ? 0.01 : duration.slow, ease: ease.out } },
  };
  // Shared soft-UI card classes: 1px border + elev-1 (no ghost-card — border is 1px), token radius/transition.
  const cardClass = 'card-section bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4 shadow-[var(--elev-1)] transition-shadow duration-[var(--motion-fast)] ease-[var(--ease-soft)]';
  const [settings, setSettings] = useState<LocationSettings>({
    locationName: '',
    phone: '',
    address: '',
    deliveryFee: 0,
    minOrder: 0,
    radiusKm: 0,
    lat: 41.331,
    lng: 19.817,
    hoursJson: DEFAULT_SCHEDULE,
  });
  const [deliveryPaused, setDeliveryPaused] = useState(false);
  const [togglingDelivery, setTogglingDelivery] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [justSaved, setJustSaved] = useState(false);
  const [error, setError] = useState('');

  // Telegram state
  const [locationId, setLocationId] = useState<string | null>(null);
  const [tgTargets, setTgTargets] = useState<any[]>([]);
  const [tgDeepLink, setTgDeepLink] = useState<string | null>(null);
  const [tgLoading, setTgLoading] = useState(false);
  const [tgTesting, setTgTesting] = useState(false);
  const [tgMessage, setTgMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const [tgQrDataUrl, setTgQrDataUrl] = useState<string | null>(null);

  // Fallback phone state
  const [fallbackPhone, setFallbackPhone] = useState('');
  const [fallbackPhoneError, setFallbackPhoneError] = useState('');
  const [fallbackSaving, setFallbackSaving] = useState(false);
  // Preserve the other fallback flags the PUT contract requires sending back.
  const fallbackFlagsRef = useRef<{ showPhoneOnError: boolean; showPhoneOnOffline: boolean }>({
    showPhoneOnError: true,
    showPhoneOnOffline: true,
  });

  useEffect(() => {
    if (!tgDeepLink) { setTgQrDataUrl(null); return; }
    QRCode.toDataURL(tgDeepLink, {
      width: 200,
      margin: 1,
      color: { dark: getComputedStyle(document.documentElement).getPropertyValue('--brand-text').trim() || '#000000', light: getComputedStyle(document.documentElement).getPropertyValue('--brand-bg').trim() || '#ffffff' },
    }).then(setTgQrDataUrl).catch(() => {});
  }, [tgDeepLink]);

  const fetchSettings = async () => {
    try {
      setLoading(true);
      const data = await apiClient<typeof OwnerSettingsResponse>('/owner/settings', { schema: OwnerSettingsResponse });
      if (data && (data.name || (data as any).locationName)) {
        setSettings({
          ...(data as any),
          hoursJson: (data as any).hoursJson || DEFAULT_SCHEDULE,
          lat: (data as any).lat || 41.331,
          lng: (data as any).lng || 19.817,
        });
        if (data.id) setLocationId(data.id);
        setDeliveryPaused((data as any).deliveryPaused ?? false);
      } else {
        setSettings(MOCK_SETTINGS);
      }
    } catch (err: any) {
      if (err.status === 404) {
        setSettings(MOCK_SETTINGS);
      } else {
        // Keep the raw server detail in the console; show a localized message.
        console.error('[SettingsPage] fetch settings failed:', err);
        setError(t('admin.settings_load_error', 'Failed to load settings'));
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchSettings();
  }, []);

  useEffect(() => {
    if (!locationId) return;
    fetchTgTargets();
    fetchFallbackConfig();
  }, [locationId]);

  // Notification language follows the admin dashboard language (admin.language_desc):
  // whenever the dashboard locale changes — or targets load with a stale locale — push
  // it to the owner's active notification targets so notifications arrive in the same
  // language as the panel. Guarded by the staleness check, so it settles (no loop).
  useEffect(() => {
    if (!locationId || tgTargets.length === 0) return;
    const stale = tgTargets.filter((tg: any) => tg.status === 'active' && tg.locale !== locale);
    if (stale.length === 0) return;
    (async () => {
      await Promise.all(stale.map((tg: any) =>
        apiClient(`/owner/locations/${locationId}/notifications/targets/${tg.id}`, {
          method: 'PUT',
          body: { locale },
        }).catch((err) => console.warn('[SettingsPage] locale sync failed:', err)),
      ));
      fetchTgTargets();
    })();
  }, [locale, tgTargets, locationId]);

  const fetchFallbackConfig = async () => {
    if (!locationId) return;
    try {
      const res = await apiClient<typeof FallbackConfigResponse>(`/owner/locations/${locationId}/settings/fallback`, { schema: FallbackConfigResponse });
      setFallbackPhone(res?.phone || '');
      fallbackFlagsRef.current = {
        showPhoneOnError: res?.showPhoneOnError ?? true,
        showPhoneOnOffline: res?.showPhoneOnOffline ?? true,
      };
    } catch (err) { console.warn('[SettingsPage] fetch fallback config failed:', err); }
  };

  const handleFallbackSave = async () => {
    if (!locationId) return;
    const trimmed = fallbackPhone.trim();
    if (trimmed && !PHONE_E164_REGEX.test(trimmed)) {
      setFallbackPhoneError(t('admin.phone_format_hint', '+355 followed by 7-14 digits'));
      return;
    }
    setFallbackPhoneError('');
    setFallbackSaving(true);
    try {
      await apiClient(`/owner/locations/${locationId}/settings/fallback`, {
        method: 'PUT',
        body: {
          phone: trimmed,
          showPhoneOnError: fallbackFlagsRef.current.showPhoneOnError,
          showPhoneOnOffline: fallbackFlagsRef.current.showPhoneOnOffline,
        },
      });
      showToast(t('common.saved', 'Settings saved'), 'success');
    } catch (err: any) {
      showToast(err?.message || t('common.error', 'Failed to save settings'), 'error');
    } finally {
      setFallbackSaving(false);
    }
  };

  const fetchTgTargets = async () => {
    if (!locationId) return;
    try {
      const res = await apiClient<typeof NotificationTargetsResponse>(`/owner/locations/${locationId}/notifications/targets`, { schema: NotificationTargetsResponse });
      setTgTargets(res?.targets || []);
    } catch (err) { console.warn('[SettingsPage] fetch tg targets failed:', err); }
  };

  const handleTgConnect = async () => {
    if (!locationId) return;
    setTgLoading(true);
    setTgMessage(null);
    try {
      const res = await apiClient<typeof TelegramConnectResponse>(`/owner/locations/${locationId}/notifications/telegram/connect-init`, { method: 'POST', schema: TelegramConnectResponse });
      setTgDeepLink(res.deepLink ?? null);
    } catch (err: any) {
      setTgMessage({ type: 'error', text: err.message || t('admin.tg_connection_error', 'Failed to initiate connection') });
    } finally {
      setTgLoading(false);
    }
  };

  const handleTgTest = async () => {
    if (!locationId) return;
    setTgTesting(true);
    setTgMessage(null);
    try {
      await apiClient(`/owner/locations/${locationId}/notifications/test`, { method: 'POST' });
      setTgMessage({ type: 'success', text: t('admin.tg_test_sent', 'Test notification sent! Check your Telegram.') });
    } catch (err: any) {
      setTgMessage({ type: 'error', text: err.message || t('admin.tg_test_error', 'Failed to send test') });
    } finally {
      setTgTesting(false);
    }
  };

  const handleTgToggle = async (targetId: string, currentStatus: string) => {
    if (!locationId) return;
    const newStatus = currentStatus === 'active' ? 'disabled' : 'active';
    try {
      await apiClient(`/owner/locations/${locationId}/notifications/targets/${targetId}`, {
        method: 'PUT',
        body: { status: newStatus }
      });
      await fetchTgTargets();
    } catch (err) { console.warn('[SettingsPage] toggle tg target failed:', err); }
  };

  const handleCategoryToggle = async (targetId: string, category: 'operational' | 'quality', newValue: boolean) => {
    if (!locationId) return;
    try {
      await apiClient(`/owner/locations/${locationId}/notifications/targets/${targetId}`, {
        method: 'PUT',
        body: { prefs: { [category]: newValue } },
      });
      await fetchTgTargets();
    } catch (err) { console.warn('[SettingsPage] category toggle failed:', err); }
  };

  const handleChange = (field: keyof LocationSettings, value: any) => {
    setSettings((prev) => ({
      ...prev,
      [field]:
        field === 'deliveryFee' || field === 'minOrder' || field === 'radiusKm'
          ? Number(value) || 0
          : value,
    }));
  };

  const handleScheduleChange = (day: string, field: keyof DaySchedule, value: any) => {
    setSettings((prev) => {
      const currentDay = prev.hoursJson[day as keyof WeeklySchedule] || DEFAULT_SCHEDULE[day];
      const newSchedule = {
        ...prev.hoursJson,
        [day]: {
          ...currentDay,
          [field]: value
        }
      } as WeeklySchedule;
      return {
        ...prev,
        hoursJson: newSchedule
      };
    });
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (settings.phone && !PHONE_E164_REGEX.test(settings.phone)) {
      setError(t('admin.phone_format_error', 'Phone must be in international format (+355...)'));
      return;
    }
    setSaving(true);
    setError('');
    try {
      await apiClient('/owner/settings', {
        method: 'PUT',
        body: { ...settings, deliveryPaused },
      });
      showToast(t('common.saved', 'Settings saved'), 'success');
      setJustSaved(true);
      setTimeout(() => setJustSaved(false), 2400);
    } catch (err: any) {
      if (err.status === 404) {
        showToast(t('common.saved', 'Settings saved'), 'success');
        setJustSaved(true);
        setTimeout(() => setJustSaved(false), 2400);
      } else {
        showToast(t('common.error', 'Failed to save settings'), 'error');
        setError(t('admin.settings_save_error', 'Failed to save settings'));
      }
    } finally {
      setSaving(false);
    }
  };

  const fieldStyle: React.CSSProperties = {
    backgroundColor: 'var(--brand-surface)',
    borderColor: 'var(--brand-border)',
    borderRadius: 'var(--brand-radius)',
    color: 'var(--brand-text)',
  };

  const labelStyle: React.CSSProperties = {
    color: 'var(--brand-text-muted)',
  };

  return (
    <div className="p-4 pb-12 space-y-6 max-w-2xl mx-auto">
      <div className="border-b border-[var(--brand-border)] pb-4">
        <h2
          className="text-2xl font-bold text-[var(--brand-text)]"
          style={{ fontFamily: 'var(--brand-font-heading)' }}
        >
          {t('admin.settings')}
        </h2>
        <p className="text-sm mt-1" style={{ color: 'var(--brand-text-muted)' }}>
          {t('admin.settings_subtitle', 'Manage your store details, delivery zone, hours and notifications.')}
        </p>
      </div>

      {loading ? (
        <div className="space-y-5" aria-busy="true" aria-label={t('admin.settings_loading', 'Loading settings')}>
          {[1, 2, 3].map((card) => (
            <div key={card} className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4 shadow-[var(--elev-1)]">
              <div className="h-4 w-32 rounded-md shimmer" />
              {[1, 2].map((row) => (
                <div key={row} className="space-y-2">
                  <div className="h-3.5 w-24 rounded-md shimmer" />
                  <div className="h-10 w-full rounded-[var(--brand-radius)] shimmer" />
                </div>
              ))}
            </div>
          ))}
          <div className="h-11 w-36 rounded-[var(--brand-radius-btn,12px)] shimmer" />
        </div>
      ) : error && settings.locationName === '' ? (
        <EmptyState
          title={t('common.error')}
          description={error}
          action={
            <Button variant="outline" size="sm" onClick={fetchSettings}>
              <i className="ti ti-refresh" /> {t('common.retry', 'Provo përsëri')}
            </Button>
          }
        />
      ) : (
        <motion.form
          onSubmit={handleSubmit}
          className="space-y-5"
          initial="hidden"
          animate="visible"
          variants={{ visible: { transition: { staggerChildren: reduceMotion ? 0 : 0.05 } } }}
        >
          <motion.div variants={sectionVariants} className={cardClass}>
            <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
              {t('admin.store_details', 'Store Details')}
            </h3>
            <div>
              <label htmlFor="settings-locationName" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('admin.location_name', 'Location Name')}
              </label>
              <Input
                id="settings-locationName"
                value={settings.locationName}
                onChange={(e) => handleChange('locationName', e.target.value)}
                placeholder={t('admin.placeholder_location_name', 'e.g. Downtown Tirana')}
                required
              />
            </div>

            <div>
              <label htmlFor="settings-phone" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('common.phone', 'Phone')}
              </label>
              <Input
                id="settings-phone"
                value={settings.phone}
                onChange={(e) => handleChange('phone', e.target.value)}
                placeholder={t('admin.placeholder_phone', '+355...')}
                pattern={PHONE_E164_PATTERN}
                title={t('admin.phone_format_hint', '+355 followed by 7-14 digits')}
                required
              />
            </div>

            <div>
              <label htmlFor="settings-address" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('common.address', 'Address')}
              </label>
              <Input
                id="settings-address"
                value={settings.address}
                onChange={(e) => handleChange('address', e.target.value)}
                placeholder={t('admin.placeholder_address', 'Street, City')}
                required
              />
            </div>
          </motion.div>

          <motion.div variants={sectionVariants} className={cardClass}>
            <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
              {t('admin.delivery_config', 'Delivery Config')}
            </h3>

            <div>
              <label htmlFor="settings-deliveryFee" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('cart.delivery_fee', 'Delivery Fee')} (ALL)
              </label>
              <Input
                id="settings-deliveryFee"
                type="number"
                value={String(settings.deliveryFee)}
                onChange={(e) => handleChange('deliveryFee', e.target.value)}
                min="0"
                required
              />
            </div>

            <div>
              <label htmlFor="settings-minOrder" className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('admin.min_order', 'Minimum Order')} (ALL)
              </label>
              <Input
                id="settings-minOrder"
                type="number"
                value={String(settings.minOrder)}
                onChange={(e) => handleChange('minOrder', e.target.value)}
                min="0"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-3" style={labelStyle}>
                {t('admin.delivery_zone', 'Delivery Zone (Radius & Location)')}
              </label>
              <MapWithRadius
                className="h-56 sm:h-64 w-full min-w-0 rounded-[var(--brand-radius)] mb-2 overflow-hidden"
                initialCenter={[settings.lng, settings.lat]}
                initialRadiusKm={settings.radiusKm}
                onRadiusChange={(c, r) => {
                  handleChange('lng', c[0]);
                  handleChange('lat', c[1]);
                  handleChange('radiusKm', r);
                }}
              />
              <p className="text-xs" style={labelStyle}>{t('admin.map_hint', 'Drag the pin to update location, adjust radius for delivery zone.')}</p>
            </div>
          </motion.div>

          <motion.div variants={sectionVariants} className={cardClass}>
            <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
              {t('admin.working_hours', 'Working Hours')}
            </h3>
            <div className="grid grid-cols-1 sm:grid-cols-[120px_1fr] gap-2">
              {['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'].map(day => {
                const dayData = (settings.hoursJson[day as keyof WeeklySchedule] || DEFAULT_SCHEDULE[day]) as DaySchedule;
                return (
                  <div key={day} className="contents">
                    <div className="flex items-center min-h-[44px] px-3 rounded-[var(--brand-radius-sm,8px)] border border-[var(--brand-border)] bg-[var(--brand-bg)] font-medium text-sm capitalize text-[var(--brand-text)] truncate">
                      {t(`admin.days.${day}`, day)}
                    </div>
                    <div
                      className="flex items-center gap-2 p-2 min-w-0 rounded-[var(--brand-radius-sm,8px)] border bg-[var(--brand-bg)] transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)]"
                      style={{ borderColor: dayData.isOpen ? 'var(--brand-primary)' : 'var(--brand-border)' }}
                    >
                      <Toggle checked={dayData.isOpen} onChange={(v) => handleScheduleChange(day, 'isOpen', v)} aria-label={`${t(`admin.days.${day}`, day)} ${dayData.isOpen ? t('admin.open', 'Open') : t('admin.closed', 'Closed')}`} />
                      <span className="text-xs w-12 shrink-0" style={{ color: dayData.isOpen ? 'var(--color-success)' : 'var(--brand-text-muted)' }}>{dayData.isOpen ? t('admin.open', 'Open') : t('admin.closed', 'Closed')}</span>
                      {dayData.isOpen && (
                        <div className="flex items-center gap-1.5 ml-auto min-w-0">
                          <Input type="time" value={dayData.open} onChange={(e) => handleScheduleChange(day, 'open', e.target.value)} aria-label={`${t(`admin.days.${day}`, day)} ${t('admin.open', 'open')}`} className="w-[5.5rem] sm:w-28 text-sm min-w-0" />
                          <span className="text-xs text-[var(--brand-text-muted)] shrink-0">{t('admin.to', 'to')}</span>
                          <Input type="time" value={dayData.close} onChange={(e) => handleScheduleChange(day, 'close', e.target.value)} aria-label={`${t(`admin.days.${day}`, day)} ${t('admin.close', 'close')}`} className="w-[5.5rem] sm:w-28 text-sm min-w-0" />
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </motion.div>

          {/* ── Language Preference ── */}
          <motion.div variants={sectionVariants} className={cardClass}>
            <h3 className="font-semibold text-sm" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.language', 'Language')}
            </h3>
            <p className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.language_desc', 'Language for admin panel and Telegram notifications.')}
            </p>
            <LanguageSwitcher variant="full" />
          </motion.div>

          {/* ── Telegram Notifications ── */}
          <motion.div variants={sectionVariants} className={cardClass}>
            <div className="flex items-center gap-2">
              <i className="ti ti-brand-telegram text-lg" style={{ color: 'var(--color-info)' }} aria-hidden="true" />
              <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
                {t('admin.telegram_notifications', 'Telegram Notifications')}
              </h3>
            </div>

            <p className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.telegram_desc', 'Receive order alerts and manage your restaurant directly from Telegram.')}
            </p>

            {/* Connected targets */}
            {tgTargets.length > 0 && (
              <div className="space-y-2">
                {tgTargets.map((tgt: any) => (
                  <div key={tgt.id} className="flex items-center justify-between gap-2 p-3 rounded-[var(--brand-radius-sm,8px)] border" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
                    <div className="flex items-center gap-2 min-w-0">
                      <div className={`w-2 h-2 rounded-full shrink-0 ${tgt.status === 'active' ? 'bg-[var(--color-success)]' : 'bg-[var(--brand-text-muted)]'}`} />
                      <div className="min-w-0">
                        <div className="text-sm font-medium truncate" style={{ color: 'var(--brand-text)' }}>
                          {tgt.channel === 'telegram' ? 'Telegram' : tgt.channel}
                        </div>
                        <div className="text-[10px] font-mono truncate" style={{ color: 'var(--brand-text-muted)' }}>
                          {tgt.address?.slice(0, 12)}...
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2 shrink-0">
                      <button onClick={() => handleTgToggle(tgt.id, tgt.status)}
                        aria-label={tgt.status === 'active' ? t('admin.disable', 'Disable') : t('admin.enable', 'Enable')}
                        aria-pressed={tgt.status === 'active'}
                        className={`w-9 h-9 flex items-center justify-center rounded-[var(--brand-radius-sm,8px)] transition-[background-color,transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:[@media(hover:hover)]:-translate-y-0.5 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-bg)] ${tgt.status === 'active' ? 'bg-[var(--color-success-light)]' : 'bg-[var(--brand-surface-raised)]'}`}
                        title={tgt.status === 'active' ? t('admin.disable', 'Disable') : t('admin.enable', 'Enable')}>
                        <i className={`ti ti-${tgt.status === 'active' ? 'check' : 'power'}`} style={{ fontSize: '0.85rem', color: tgt.status === 'active' ? 'var(--color-success)' : 'var(--brand-text-muted)' }} aria-hidden="true" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Notification category preference-centre (dark until VITE_TG_CATEGORY_GATING) */}
            {CATEGORY_GATING_ENABLED && (() => {
              const primary = tgTargets.find((tg: any) => tg.channel === 'telegram' && tg.status === 'active');
              if (!primary) return null;
              const opOn = primary.prefs?.operational !== false; // default ON
              const qOn = primary.prefs?.quality === true;        // default OFF
              return (
                <div data-testid="notif-categories" className="mt-4 space-y-2">
                  <div className="text-sm font-semibold" style={{ color: 'var(--brand-text)' }}>
                    {t('admin.notif_categories', 'Notification categories')}
                  </div>
                  <div className="flex items-center justify-between p-3 rounded-lg border" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
                    <div>
                      <div className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>🔴 {t('admin.notif_transactional', 'Transactional')}</div>
                      <div className="text-[11px]" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.notif_transactional_desc', 'New orders, failures — cannot be turned off')}</div>
                    </div>
                    <span data-testid="notif-cat-transactional" className="text-[11px] font-medium px-2 py-1 rounded" style={{ color: 'var(--color-success)', background: 'var(--color-success-light)' }}>
                      {t('admin.notif_always_on', 'Always on')}
                    </span>
                  </div>
                  <div className="flex items-center justify-between p-3 rounded-lg border" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
                    <div>
                      <div className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>🟠 {t('admin.notif_operational', 'Operational')}</div>
                      <div className="text-[11px]" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.notif_operational_desc', 'Shift changes, storefront open/close')}</div>
                    </div>
                    <span data-testid="notif-cat-operational">
                      <Toggle checked={opOn} onChange={(v) => handleCategoryToggle(primary.id, 'operational', v)} aria-label={t('admin.notif_operational', 'Operational')} />
                    </span>
                  </div>
                  <div className="flex items-center justify-between p-3 rounded-lg border" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
                    <div>
                      <div className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>🟡 {t('admin.notif_quality', 'Quality & analytics')}</div>
                      <div className="text-[11px]" style={{ color: 'var(--brand-text-muted)' }}>{t('admin.notif_quality_desc', 'Low ratings, digests')}</div>
                    </div>
                    <span data-testid="notif-cat-quality">
                      <Toggle checked={qOn} onChange={(v) => handleCategoryToggle(primary.id, 'quality', v)} aria-label={t('admin.notif_quality', 'Quality & analytics')} />
                    </span>
                  </div>
                </div>
              );
            })()}

            {/* Deep link flow + QR */}
            {tgDeepLink && (
              <div className="p-3 rounded-lg border" style={{ background: 'var(--brand-primary-light)', borderColor: 'var(--brand-primary)' }}>
                <div className="flex items-start gap-4">
                  {tgQrDataUrl && (
                    <div className="shrink-0">
                      <img src={tgQrDataUrl} alt="QR Code" className="w-24 h-24 rounded-lg" />
                    </div>
                  )}
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-semibold mb-1.5" style={{ color: 'var(--brand-primary)' }}>
                      {t('admin.tg_step1', '1. Scan QR or open this link in Telegram:')}
                    </div>
                    <a href={tgDeepLink} target="_blank" rel="noopener noreferrer"
                      className="text-sm font-mono underline break-all block rounded-[var(--brand-radius-sm,8px)] transition-opacity duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:opacity-80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-primary-light)]" style={{ color: 'var(--brand-primary)' }}>
                      {tgDeepLink}
                    </a>
                    <div className="text-[10px] mt-2" style={{ color: 'var(--brand-text-muted)' }}>
                      {t('admin.tg_step2', '2. Click Start in the bot. Your Telegram will be connected automatically.')}
                    </div>
                  </div>
                </div>
                <div className="flex gap-2 mt-2">
                  <Button onClick={() => { setTgDeepLink(null); handleTgConnect(); }} variant="ghost" size="sm">
                    <i className="ti ti-refresh" /> {t('common.refresh', 'Refresh')}
                  </Button>
                </div>
              </div>
            )}

            {/* Messages */}
            {tgMessage && (
              <div role="alert" aria-live="polite" className="p-3 rounded-lg text-xs" style={{
                background: tgMessage.type === 'success' ? 'var(--color-success-light)' : 'var(--color-danger-light)',
                color: tgMessage.type === 'success' ? 'var(--color-success)' : 'var(--color-danger)'
              }}>
                {tgMessage.text}
              </div>
            )}

            {/* Actions */}
            <div className="flex gap-2">
              <Button onClick={handleTgConnect} isLoading={tgLoading} variant="ghost" size="sm">
                <i className="ti ti-brand-telegram" /> {tgTargets.length > 0 ? t('admin.tg_add_another', 'Add Another') : t('admin.tg_connect', 'Connect Telegram')}
              </Button>
              {tgTargets.length > 0 && (
                <Button onClick={handleTgTest} isLoading={tgTesting} variant="ghost" size="sm">
                  <i className="ti ti-send" /> {t('admin.tg_test', 'Send Test')}
                </Button>
              )}
            </div>
          </motion.div>

          {/* ── Fallback Phone ── */}
          {locationId && (
            <motion.div variants={sectionVariants} className={cardClass}>
              <div className="flex items-center gap-2">
                <i className="ti ti-phone-call text-lg" style={{ color: 'var(--color-info)' }} aria-hidden="true" />
                <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
                  {t('admin.fallback_phone', 'Fallback Phone')}
                </h3>
              </div>
              <p className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
                {t('admin.fallback_phone_desc', 'Shown to customers if notifications fail or the connection drops, so they can reach you directly. Leave empty to disable.')}
              </p>
              <div>
                <label htmlFor="settings-fallbackPhone" className="block text-sm font-medium mb-1" style={labelStyle}>
                  {t('admin.fallback_phone_label', 'Contact number')}
                </label>
                <div className="flex flex-col sm:flex-row gap-2">
                  <Input
                    id="settings-fallbackPhone"
                    type="tel"
                    inputMode="tel"
                    autoComplete="tel"
                    value={fallbackPhone}
                    onChange={(e) => { setFallbackPhone(e.target.value); if (fallbackPhoneError) setFallbackPhoneError(''); }}
                    placeholder={t('admin.placeholder_phone', '+355...')}
                    pattern={PHONE_E164_PATTERN}
                    title={t('admin.phone_format_hint', '+355 followed by 7-14 digits')}
                    aria-invalid={fallbackPhoneError ? true : undefined}
                    aria-describedby={fallbackPhoneError ? 'settings-fallbackPhone-error' : undefined}
                    className="flex-1 min-h-[44px]"
                  />
                  <Button onClick={handleFallbackSave} isLoading={fallbackSaving} variant="ghost" className="min-h-[44px]">
                    {t('common.save', 'Save')}
                  </Button>
                </div>
                {fallbackPhoneError && (
                  <span id="settings-fallbackPhone-error" role="alert" className="block mt-1 text-[var(--color-danger)] text-sm">
                    {fallbackPhoneError}
                  </span>
                )}
              </div>
            </motion.div>
          )}

          <motion.div variants={sectionVariants} className={cardClass.replace(' space-y-4', '')}>
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <h3 className="font-semibold text-sm text-[var(--brand-text)]">{t('admin.delivery_status', 'Delivery Status')}</h3>
                <p className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>
                  {deliveryPaused
                    ? t('admin.delivery_paused_hint', 'Delivery is paused. Customers see a "Closed" message.')
                    : t('admin.delivery_open_hint', 'Delivery is open based on your hours schedule.')}
                </p>
              </div>
              <div className="shrink-0 transition-opacity duration-[var(--motion-fast)] ease-[var(--ease-soft)]" style={{ opacity: togglingDelivery ? 0.5 : 1, pointerEvents: togglingDelivery ? 'none' : 'auto' }}>
                <Toggle
                  checked={!deliveryPaused}
                  onChange={async (v) => {
                    const newPaused = !v;
                    setTogglingDelivery(true);
                    try {
                      await apiClient('/owner/settings', { method: 'PUT', body: { deliveryPaused: newPaused } });
                      setDeliveryPaused(newPaused);
                      showToast(newPaused ? t('admin.delivery_paused', 'Delivery paused') : t('admin.delivery_resumed', 'Delivery resumed'), 'success');
                    } catch { showToast(t('common.error', 'Failed to update delivery status'), 'error'); }
                    finally { setTogglingDelivery(false); }
                  }}
                  label={deliveryPaused ? t('admin.delivery_closed_label', 'Closed') : t('admin.delivery_open_label', 'Open')}
                />
              </div>
            </div>
          </motion.div>

          <motion.div variants={sectionVariants} className="flex flex-wrap items-center gap-3">
            <Button type="submit" isLoading={saving} size="lg">
              {t('common.save', 'Save Changes')}
            </Button>
            {justSaved && !saving && (
              <motion.span
                role="status"
                aria-live="polite"
                initial={{ opacity: 0, scale: reduceMotion ? 1 : 0.9 }}
                animate={{ opacity: 1, scale: 1, transition: { duration: reduceMotion ? 0.01 : duration.base, ease: ease.out } }}
                className="inline-flex items-center gap-1.5 text-sm font-medium"
                style={{ color: 'var(--color-success)' }}
              >
                <i className="ti ti-circle-check" aria-hidden="true" /> {t('common.saved', 'Settings saved')}
              </motion.span>
            )}
            {error && settings.locationName !== '' && (
              <span role="alert" className="text-[var(--color-danger)] text-sm">{error}</span>
            )}
          </motion.div>
        </motion.form>
      )}
    </div>
  );
}
