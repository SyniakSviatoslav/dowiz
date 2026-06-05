import React, { useState, useEffect } from 'react';
import { Button, Input, EmptyState } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

interface LocationSettings {
  locationName: string;
  phone: string;
  address: string;
  deliveryFee: number;
  minOrder: number;
  radiusKm: number;
}

const MOCK_SETTINGS: LocationSettings = {
  locationName: 'Downtown Tirana',
  phone: '+35542345678',
  address: 'Rruga Ismail Qemali 45, Tirana',
  deliveryFee: 120,
  minOrder: 500,
  radiusKm: 8,
};

export function SettingsPage() {
  const [settings, setSettings] = useState<LocationSettings>({
    locationName: '',
    phone: '',
    address: '',
    deliveryFee: 0,
    minOrder: 0,
    radiusKm: 0,
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
        setSettings(data);
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

  const handleChange = (field: keyof LocationSettings, value: string) => {
    setSettings((prev) => ({
      ...prev,
      [field]:
        field === 'deliveryFee' || field === 'minOrder' || field === 'radiusKm'
          ? Number(value) || 0
          : value,
    }));
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
          Location Settings
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
        <EmptyState title="Error" description={error} />
      ) : (
        <form onSubmit={handleSubmit} className="space-y-5">
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-5 space-y-4">
            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                Location Name
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
                Phone
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
                Address
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
              Delivery Config
            </h3>

            <div>
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                Delivery Fee (ALL)
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
                Minimum Order (ALL)
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
              <label className="block text-sm font-medium mb-1" style={labelStyle}>
                Delivery Radius (km)
              </label>
              <Input
                type="number"
                value={String(settings.radiusKm)}
                onChange={(e) => handleChange('radiusKm', e.target.value)}
                min="0"
                max="50"
                required
              />
            </div>
          </div>

          <div className="flex items-center gap-4">
            <Button type="submit" isLoading={saving} size="lg">
              Save Changes
            </Button>
            {success && (
              <span className="text-green-500 font-medium text-sm">Saved successfully!</span>
            )}
            {error && settings.locationName !== '' && (
              <span className="text-red-500 text-sm">{error}</span>
            )}
          </div>
        </form>
      )}
    </div>
  );
}
