import React, { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Input, FormField, useI18n, useToast } from '@deliveryos/ui';
import { PHONE_E164_PATTERN } from '@deliveryos/shared-types';
import { apiClient, ApiError } from '../../lib/index.js';
import { z } from 'zod';

// Brand-new owner entry (O3). The old 9-step wizard is retired: it faked
// "You're live!", seeded a throwaway demo menu, let you "share" before publish,
// and lost progress on reload. The only input a fresh owner genuinely owes us is
// the public identity of their storefront (name · phone · link). This form takes
// those three, creates a *draft* via POST /owner/onboarding/start, then hands off
// to the activation tool (/admin/activation) — where the real menu, gate, live
// preview, and Publish live. Nothing here claims the storefront is live.

const StartResponse = z.object({
  locationId: z.string(),
  slug: z.string(),
}).passthrough();

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/[ë]/g, 'e')
    .replace(/[ç]/g, 'c')
    .replace(/[^a-z0-9\s-]/g, '')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 50);
}

const RESERVED = ['admin', 's', 'api', 'onboarding', 'courier', 'health', 'login', 'orders', 'menu'];

export function OnboardingPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const { showToast } = useToast();

  const [name, setName] = useState('');
  const [phone, setPhone] = useState('');
  const [slug, setSlug] = useState('');
  const [slugError, setSlugError] = useState('');
  const [creating, setCreating] = useState(false);

  // Slug tracks the name until the owner edits it by hand.
  const handleNameChange = useCallback((v: string) => {
    setName(v);
    if (!slug || slug === slugify(name)) {
      const s = slugify(v);
      setSlug(s);
      if (RESERVED.includes(s)) setSlugError(t('admin.reserved_name', 'This name is reserved'));
      else setSlugError('');
    }
  }, [slug, name, t]);

  const handleSlugChange = useCallback((v: string) => {
    const s = v.toLowerCase().replace(/[^a-z0-9-]/g, '').slice(0, 50);
    setSlug(s);
    if (RESERVED.includes(s)) setSlugError(t('admin.reserved_name', 'This name is reserved'));
    else if (s.length < 3) setSlugError(t('admin.too_short', 'Too short (min 3)'));
    else setSlugError('');
  }, [t]);

  const canCreate =
    name.trim().length >= 2 && phone.trim().length >= 8 && slug.length >= 3 && !slugError && !creating;

  const handleCreate = async () => {
    if (!canCreate) return;
    setCreating(true);
    try {
      await apiClient('/owner/onboarding/start', {
        method: 'POST',
        schema: StartResponse,
        body: { name: name.trim(), phone: phone.trim(), slug },
      });
      // Draft created — the activation tool is the owner's real next surface.
      navigate('/admin/activation', { replace: true });
    } catch (err) {
      if (err instanceof ApiError && (err.status === 409 || err.data?.code === 'SLUG_TAKEN')) {
        setSlugError(t('admin.slug_taken', 'That link is already taken — try another.'));
      } else {
        showToast(t('admin.create_failed', 'Could not create your storefront. Please try again.'), 'error');
      }
    } finally {
      setCreating(false);
    }
  };

  const s = {
    card: { background: 'var(--brand-surface)', border: '1px solid var(--brand-border)', borderRadius: 'var(--brand-radius)', padding: '20px' },
    heading: { fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' },
    muted: { color: 'var(--brand-text-muted)', fontSize: '13px' },
  };

  return (
    <div className="min-h-screen bg-[var(--brand-bg)] text-[var(--brand-text)]">
      <div className="max-w-lg mx-auto p-4 md:p-8">
        <form
          style={s.card}
          className="space-y-4"
          onSubmit={(e) => { e.preventDefault(); void handleCreate(); }}
        >
          <div>
            <h2 className="text-xl font-bold" style={s.heading}>{t('admin.create_storefront', 'Create your storefront')}</h2>
            <p className="mt-1" style={s.muted}>
              {t('admin.create_storefront_desc', "Three details to start. You'll add your menu and go live on the next screen — nothing is public until you publish.")}
            </p>
          </div>

          <FormField label={t('admin.restaurant_name', 'Restaurant name')}>
            <Input value={name} onChange={e => handleNameChange((e.target as HTMLInputElement).value)} placeholder="e.g. Pizza Roma" />
          </FormField>

          <FormField label={t('admin.phone_fallback', 'Phone (fallback for customers)')}>
            <Input value={phone} onChange={e => setPhone((e.target as HTMLInputElement).value)} placeholder="+355 69 XXX XXXX" pattern={PHONE_E164_PATTERN} title="+355 followed by 7-14 digits" />
          </FormField>

          <FormField label={t('admin.your_link', 'Your link')}>
            <div className="flex items-center gap-2">
              <Input value={slug} onChange={e => handleSlugChange((e.target as HTMLInputElement).value)} placeholder="sushi-durres" />
              <span className="text-sm whitespace-nowrap" style={s.muted}>.dowiz.org</span>
            </div>
            {slugError && <p className="text-xs mt-1" style={{ color: 'var(--color-danger)' }}>{slugError}</p>}
          </FormField>

          <Button type="submit" disabled={!canCreate} isLoading={creating} className="w-full" size="lg">
            {t('admin.create_continue', 'Create & continue')} →
          </Button>
        </form>
      </div>
    </div>
  );
}
