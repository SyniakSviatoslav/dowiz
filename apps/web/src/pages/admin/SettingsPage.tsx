import React, { useState, useEffect, useRef } from 'react';
import { Button, Input, EmptyState, MapWithRadius, Toggle, useI18n, LanguageSwitcher } from '@deliveryos/ui';
import type { LngLatLike, Locale } from '@deliveryos/ui';
import { PHONE_E164_REGEX, PHONE_E164_PATTERN } from '@deliveryos/shared-types';
import { apiClient } from '../../lib/index.js';
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
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [success, setSuccess] = useState(false);
  const [error, setError] = useState('');

  // Telegram state
  const [locationId, setLocationId] = useState<string | null>(null);
  const [tgTargets, setTgTargets] = useState<any[]>([]);
  const [tgDeepLink, setTgDeepLink] = useState<string | null>(null);
  const [tgLoading, setTgLoading] = useState(false);
  const [tgTesting, setTgTesting] = useState(false);
  const [tgMessage, setTgMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const [tgQrDataUrl, setTgQrDataUrl] = useState<string | null>(null);

  useEffect(() => {
    if (!tgDeepLink) { setTgQrDataUrl(null); return; }
    QRCode.toDataURL(tgDeepLink, {
      width: 200,
      margin: 1,
      color: { dark: '#000000', light: '#ffffff' },
    }).then(setTgQrDataUrl).catch(() => {});
  }, [tgDeepLink]);

  const fetchSettings = async () => {
    try {
      setLoading(true);
      const data = await apiClient<any>('/owner/settings');
      if (data && data.locationName) {
        setSettings({
          ...data,
          hoursJson: data.hoursJson || DEFAULT_SCHEDULE,
          lat: data.lat || 41.331,
          lng: data.lng || 19.817,
        });
        if (data.id) setLocationId(data.id);
      } else {
        setSettings(MOCK_SETTINGS);
      }
    } catch (err: any) {
      if (err.status === 404) {
        setSettings(MOCK_SETTINGS);
      } else {
        setError('Failed to load settings');
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
  }, [locationId]);

  const fetchTgTargets = async () => {
    if (!locationId) return;
    try {
      const res = await apiClient<any>(`/owner/locations/${locationId}/notifications/targets`);
      setTgTargets(res?.targets || []);
    } catch { /* ignore */ }
  };

  const handleTgConnect = async () => {
    if (!locationId) return;
    setTgLoading(true);
    setTgMessage(null);
    try {
      const res = await apiClient<any>(`/owner/locations/${locationId}/notifications/telegram/connect-init`, { method: 'POST' });
      setTgDeepLink(res.deepLink);
    } catch (err: any) {
      setTgMessage({ type: 'error', text: err.message || 'Failed to initiate connection' });
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
      setTgMessage({ type: 'success', text: 'Test notification sent! Check your Telegram.' });
    } catch (err: any) {
      setTgMessage({ type: 'error', text: err.message || 'Failed to send test' });
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
    } catch { /* ignore */ }
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
      setError('Phone must be in international format (+355...)');
      return;
    }
    setSaving(true);
    setSuccess(false);
    try {
      await apiClient('/owner/settings', {
        method: 'PUT',
        body: settings,
      });
      setSuccess(true);
    } catch (err: any) {
      if (err.status === 404) {
        setSuccess(true);
      } else {
        setError('Failed to save settings');
      }
    } finally {
      setSaving(false);
      if (success || !error) {
        setTimeout(() => setSuccess(false), 3000);
      }
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
    <div className="p-4 space-y-6 max-w-2xl mx-auto">
      <div className="border-b border-[var(--brand-border)] pb-4">
        <h2
          className="text-2xl font-bold"
          style={{ fontFamily: 'var(--brand-font-heading)' }}
        >
          {t('admin.settings')}
        </h2>
      </div>

      {loading ? (
        <div className="animate-pulse space-y-4">
          {[1, 2, 3, 4, 5, 6].map((i) => (
            <div key={i}>
              <div className="h-4 bg-[var(--brand-surface)] rounded w-24 mb-2" />
              <div className="h-10 bg-[var(--brand-surface)] rounded-[var(--brand-radius)] w-full" />
            </div>
          ))}
          <div className="h-10 bg-[var(--brand-surface)] rounded-[var(--brand-radius)] w-32" />
        </div>
      ) : error && settings.locationName === '' ? (
        <EmptyState title={t('common.error')} description={error} />
      ) : (
        <form onSubmit={handleSubmit} className="space-y-5">
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('admin.location_name', 'Location Name')}
                <Input
                  value={settings.locationName}
                  onChange={(e) => handleChange('locationName', e.target.value)}
                  placeholder={t('admin.placeholder_location_name', 'e.g. Downtown Tirana')}
                  required
                />
              </label>
            </div>

            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('common.phone', 'Phone')}
                <Input
                  value={settings.phone}
                  onChange={(e) => handleChange('phone', e.target.value)}
                  placeholder={t('admin.placeholder_phone', '+355...')}
                  pattern={PHONE_E164_PATTERN}
                  title={t('admin.phone_format_hint', '+355 followed by 7-14 digits')}
                  required
                />
              </label>
            </div>

            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('common.address', 'Address')}
                <Input
                  value={settings.address}
                  onChange={(e) => handleChange('address', e.target.value)}
                  placeholder={t('admin.placeholder_address', 'Street, City')}
                  required
                />
              </label>
            </div>
          </div>

          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
              {t('admin.delivery_config', 'Delivery Config')}
            </h3>

            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('cart.delivery_fee', 'Delivery Fee')} (ALL)
                <Input
                  type="number"
                  value={String(settings.deliveryFee)}
                  onChange={(e) => handleChange('deliveryFee', e.target.value)}
                  min="0"
                  required
                />
              </label>
            </div>

            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('admin.min_order', 'Minimum Order')} (ALL)
                <Input
                  type="number"
                  value={String(settings.minOrder)}
                  onChange={(e) => handleChange('minOrder', e.target.value)}
                  min="0"
                  required
                />
              </label>
            </div>

            <div>
              <label className="block text-sm font-medium mb-3" style={labelStyle}>
                {t('admin.delivery_zone', 'Delivery Zone (Radius & Location)')}
              </label>
              <MapWithRadius
                className="h-64 w-full rounded-lg mb-2"
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
          </div>

          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
              {t('admin.working_hours', 'Working Hours')}
            </h3>
            <div className="grid grid-cols-1 sm:grid-cols-[120px_1fr] gap-2">
              {['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'].map(day => {
                const dayData = (settings.hoursJson[day as keyof WeeklySchedule] || DEFAULT_SCHEDULE[day]) as DaySchedule;
                return (
                  <div key={day} className="contents">
                    <div className="flex items-center h-10 px-2 rounded-lg border border-[var(--brand-border)] bg-[var(--brand-bg)] font-medium text-sm capitalize">
                      {t(`admin.days.${day}`, day)}
                    </div>
                    <div className="flex items-center gap-2 p-2 rounded-lg border border-[var(--brand-border)] bg-[var(--brand-bg)]">
                      <Toggle checked={dayData.isOpen} onChange={(v) => handleScheduleChange(day, 'isOpen', v)} />
                      <span className="text-xs w-10 text-[var(--brand-text-muted)] shrink-0">{dayData.isOpen ? t('admin.open', 'Open') : t('admin.closed', 'Closed')}</span>
                      {dayData.isOpen && (
                        <div className="flex items-center gap-2 ml-auto">
                          <Input type="time" value={dayData.open} onChange={(e) => handleScheduleChange(day, 'open', e.target.value)} aria-label={`${t(`admin.days.${day}`, day)} ${t('admin.open', 'open')}`} className="w-20 sm:w-28 text-sm" />
                          <span className="text-xs text-[var(--brand-text-muted)] shrink-0">{t('admin.to', 'to')}</span>
                          <Input type="time" value={dayData.close} onChange={(e) => handleScheduleChange(day, 'close', e.target.value)} aria-label={`${t(`admin.days.${day}`, day)} ${t('admin.close', 'close')}`} className="w-20 sm:w-28 text-sm" />
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* ── Language Preference ── */}
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <h3 className="font-semibold text-sm" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.language', 'Language')}
            </h3>
            <p className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
              {t('admin.language_desc', 'Language for admin panel and Telegram notifications.')}
            </p>
            <LanguageSwitcher variant="full" />
          </div>

          {/* ── Telegram Notifications ── */}
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <div className="flex items-center gap-2">
              <i className="ti ti-brand-telegram text-lg" style={{ color: '#0088cc' }} />
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
                  <div key={tgt.id} className="flex items-center justify-between p-3 rounded-lg border" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)' }}>
                    <div className="flex items-center gap-2">
                      <div className={`w-2 h-2 rounded-full ${tgt.status === 'active' ? 'bg-[var(--color-success)]' : 'bg-[var(--brand-text-muted)]'}`} />
                      <div>
                        <div className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>
                          {tgt.channel === 'telegram' ? 'Telegram' : tgt.channel}
                        </div>
                        <div className="text-[10px] font-mono" style={{ color: 'var(--brand-text-muted)' }}>
                          {tgt.address?.slice(0, 12)}...
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <button onClick={() => handleTgToggle(tgt.id, tgt.status)}
                        aria-label={tgt.status === 'active' ? t('admin.disable', 'Disable') : t('admin.enable', 'Enable')}
                        className={`w-8 h-8 flex items-center justify-center rounded-lg transition-colors ${tgt.status === 'active' ? 'bg-[var(--color-success-light)]' : 'bg-[var(--brand-surface-raised)]'}`}
                        title={tgt.status === 'active' ? t('admin.disable', 'Disable') : t('admin.enable', 'Enable')}>
                        <i className={`ti ti-${tgt.status === 'active' ? 'check' : 'power'}`} style={{ fontSize: '0.85rem', color: tgt.status === 'active' ? 'var(--color-success)' : 'var(--brand-text-muted)' }} aria-hidden="true" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}

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
                      className="text-sm font-mono underline break-all block" style={{ color: 'var(--brand-primary)' }}>
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
              <div className="p-3 rounded-lg text-xs" style={{
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
          </div>

          <div className="flex items-center gap-4">
            <Button type="submit" isLoading={saving} size="lg">
              {t('common.save', 'Save Changes')}
            </Button>
            {success && (
              <span role="status" aria-live="polite" className="text-[var(--color-success)] font-medium text-sm">{t('common.saved', 'Saved successfully!')}</span>
            )}
            {error && settings.locationName !== '' && (
              <span role="alert" className="text-[var(--color-danger)] text-sm">{error}</span>
            )}
          </div>
        </form>
      )}
    </div>
  );
}
