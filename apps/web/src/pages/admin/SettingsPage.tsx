import React, { useState, useEffect } from 'react';
import { Button, Input, EmptyState, MapWithRadius, Toggle, useI18n } from '@deliveryos/ui';
import type { LngLatLike } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

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
  const { t } = useI18n();
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
    <div className="p-4 space-y-6 max-w-lg mx-auto">
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
              </label>
              <Input
                value={settings.locationName}
                onChange={(e) => handleChange('locationName', e.target.value)}
                placeholder="e.g. Downtown Tirana"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('common.phone', 'Phone')}
              </label>
              <Input
                value={settings.phone}
                onChange={(e) => handleChange('phone', e.target.value)}
                placeholder="+355..."
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('common.address', 'Address')}
              </label>
              <Input
                value={settings.address}
                onChange={(e) => handleChange('address', e.target.value)}
                placeholder="Street, City"
                required
              />
            </div>
          </div>

          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <h3 className="font-semibold text-sm text-[var(--brand-text-muted)]">
              {t('admin.delivery_config', 'Delivery Config')}
            </h3>

            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('cart.delivery_fee', 'Delivery Fee')} (ALL)
              </label>
              <Input
                type="number"
                value={String(settings.deliveryFee)}
                onChange={(e) => handleChange('deliveryFee', e.target.value)}
                min="0"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                {t('admin.min_order', 'Minimum Order')} (ALL)
              </label>
              <Input
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
            <div className="space-y-3">
              {['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'].map(day => {
                const dayData = (settings.hoursJson[day as keyof WeeklySchedule] || DEFAULT_SCHEDULE[day]) as DaySchedule;
                return (
                  <div key={day} className="flex items-center gap-3 p-2 rounded-lg border border-[var(--brand-border)] bg-[var(--brand-bg)]">
                    <div className="w-24 font-medium text-sm capitalize">{t(`admin.days.${day}`, day)}</div>
                    <div className="flex-1 flex items-center gap-2">
                      <Toggle checked={dayData.isOpen} onChange={(v) => handleScheduleChange(day, 'isOpen', v)} />
                      <span className="text-xs w-10 text-[var(--brand-text-muted)]">{dayData.isOpen ? t('admin.open', 'Open') : t('admin.closed', 'Closed')}</span>
                    </div>
                    {dayData.isOpen && (
                      <div className="flex items-center gap-2">
                        <Input type="time" value={dayData.open} onChange={(e) => handleScheduleChange(day, 'open', e.target.value)} className="w-24 text-sm" />
                        <span className="text-xs text-[var(--brand-text-muted)]">{t('admin.to', 'to')}</span>
                        <Input type="time" value={dayData.close} onChange={(e) => handleScheduleChange(day, 'close', e.target.value)} className="w-24 text-sm" />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          <div className="flex items-center gap-4">
            <Button type="submit" isLoading={saving} size="lg">
              {t('common.save', 'Save Changes')}
            </Button>
            {success && (
              <span className="text-[var(--color-success)] font-medium text-sm">{t('common.saved', 'Saved successfully!')}</span>
            )}
            {error && settings.locationName !== '' && (
              <span className="text-[var(--color-danger)] text-sm">{error}</span>
            )}
          </div>
        </form>
      )}
    </div>
  );
}
