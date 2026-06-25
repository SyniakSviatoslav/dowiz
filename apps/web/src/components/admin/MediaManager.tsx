/**
 * MediaManager — owner-facing rich-media manager for the cinematic product-media
 * seam (ADR-0002, Phase-2 contract §Admin media manager). Mounted inside the
 * product editor in MenuManagerPage, only when editing an existing product.
 *
 * Flow (matches apps/api/src/routes/owner/product-media.ts — the authoritative
 * server contract):
 *   1. presign  POST /owner/menu/products/:id/media/presign
 *                 body { kind, items:[{ mimeType, bytes, sha256 }] }
 *                 → { uploads:[{ key, url, sha256 }], expiresIn }   (413 on budget/size)
 *   2. PUT each file straight to its presigned URL (raw bytes).
 *   3. confirm  POST /owner/menu/products/:id/media/confirm
 *                 body { kind, storageKey, mimeType, bytes, width?, height? }
 *                 → { id, sortOrder }
 * Then set-primary / reorder / available-toggle, all per the same contract.
 *
 * Listing: there is no owner-scoped list endpoint, so we reuse the PUBLIC lazy
 * endpoint GET /public/locations/:slug/products/:id/media for the owner's own
 * location. NOTE: that endpoint filters available=true, so media toggled OFF
 * drop out of the list (re-enabling requires the public payload to surface it).
 *
 * Gating: the whole component is DARK by default. It renders nothing unless the
 * caller passes enabled (MEDIA_RICH_ENABLED + business tier hint) — read via
 * import.meta.env.VITE_MEDIA_RICH_ENABLED, default hidden.
 */
import React, { useState, useEffect, useCallback } from 'react';
import { motion } from 'framer-motion';
import { useI18n, useToast, useConfirm } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

export type MediaKind = 'image' | 'video' | 'spin' | 'model';

export interface MediaItem {
  id: string;
  kind: MediaKind;
  url: string;
  posterUrl?: string | null;
  mimeType: string;
  alt?: string | null;
  sortOrder: number;
}

interface MediaManagerProps {
  productId: string;
  /** Whether the rich-media feature is enabled for this owner (flag + business tier). */
  enabled: boolean;
  /** Owner's location slug — used for the public lazy-media list endpoint. */
  slug?: string | null;
  /** Notifies the parent when the primary media changes (so it can refresh thumbnails). */
  onPrimaryChange?: () => void;
}

const ACCEPT = 'image/webp,image/jpeg,video/mp4';
const ALLOWED_MIME = new Set(['image/webp', 'image/jpeg', 'video/mp4']);
const MAX_BYTES = 25 * 1024 * 1024; // generous client guard; server enforces the real ceiling

function kindForMime(mime: string): MediaKind {
  return mime.startsWith('video/') ? 'video' : 'image';
}

// SHA-256 of a file via Web Crypto → lowercase hex (matches the server's /^[0-9a-f]{64}$/).
async function sha256Hex(buf: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', buf);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

export function MediaManager({ productId, enabled, slug, onPrimaryChange }: MediaManagerProps) {
  const { t } = useI18n();
  const { showToast } = useToast();
  const { confirm, dialog: confirmDialog } = useConfirm();

  const [media, setMedia] = useState<MediaItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [progress, setProgress] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  const loadMedia = useCallback(async () => {
    if (!enabled || !slug || !productId) return;
    setLoading(true);
    try {
      const res = await apiClient<any>(`/public/locations/${slug}/products/${productId}/media`);
      const list: MediaItem[] = Array.isArray(res?.media) ? res.media : [];
      list.sort((a, b) => a.sortOrder - b.sortOrder);
      setMedia(list);
    } catch (err) {
      console.debug('[MediaManager] load media failed:', err);
    } finally {
      setLoading(false);
    }
  }, [enabled, slug, productId]);

  useEffect(() => { loadMedia(); }, [loadMedia]);

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = ''; // allow re-selecting the same file
    if (!file) return;
    if (!ALLOWED_MIME.has(file.type)) {
      showToast(t('admin.media_bad_type', 'Only WebP, JPEG or MP4 files are allowed.'), 'error');
      return;
    }
    if (file.size > MAX_BYTES) {
      showToast(t('admin.media_too_large', 'File is too large.'), 'error');
      return;
    }

    setUploading(true);
    try {
      const kind = kindForMime(file.type);
      const buf = await file.arrayBuffer();
      setProgress(t('admin.media_hashing', 'Preparing…'));
      const sha256 = await sha256Hex(buf);

      // 1. presign
      setProgress(t('admin.media_requesting', 'Requesting upload…'));
      let presign: any;
      try {
        presign = await apiClient<any>(`/owner/menu/products/${productId}/media/presign`, {
          method: 'POST',
          body: { kind, items: [{ mimeType: file.type, bytes: file.size, sha256 }] },
        });
      } catch (err: any) {
        if (err?.status === 413) {
          showToast(t('admin.media_over_budget', 'Storage budget exceeded — remove some media first.'), 'error');
        } else if (err?.status === 400) {
          showToast(t('admin.media_rejected', 'This file was rejected. Use WebP, JPEG or MP4.'), 'error');
        } else {
          showToast(t('admin.media_upload_failed', 'Upload failed. Please try again.'), 'error');
        }
        return;
      }
      const upload = presign?.uploads?.[0];
      if (!upload?.url || !upload?.key) {
        showToast(t('admin.media_upload_failed', 'Upload failed. Please try again.'), 'error');
        return;
      }

      // 2. PUT raw bytes to the presigned URL (direct to object storage, no apiClient).
      setProgress(t('admin.media_uploading', 'Uploading…'));
      const putRes = await fetch(upload.url, {
        method: 'PUT',
        headers: { 'Content-Type': file.type },
        body: buf,
      });
      if (!putRes.ok) {
        showToast(t('admin.media_upload_failed', 'Upload failed. Please try again.'), 'error');
        return;
      }

      // 3. confirm — persists one product_media row.
      setProgress(t('admin.media_finalizing', 'Finalizing…'));
      await apiClient<any>(`/owner/menu/products/${productId}/media/confirm`, {
        method: 'POST',
        body: { kind, storageKey: upload.key, mimeType: file.type, bytes: file.size },
      });

      showToast(t('admin.media_added', 'Media added'), 'success');
      await loadMedia();
    } catch (err) {
      console.error('[MediaManager] upload failed:', err);
      showToast(t('admin.media_upload_failed', 'Upload failed. Please try again.'), 'error');
    } finally {
      setUploading(false);
      setProgress(null);
    }
  };

  const handleSetPrimary = async (mediaId: string) => {
    setBusyId(mediaId);
    try {
      await apiClient<any>(`/owner/menu/products/${productId}/media/${mediaId}/set-primary`, { method: 'POST' });
      showToast(t('admin.media_primary_set', 'Primary media updated'), 'success');
      onPrimaryChange?.();
    } catch (err) {
      console.error('[MediaManager] set-primary failed:', err);
      showToast(t('common.error_save', 'Failed to save.'), 'error');
    } finally {
      setBusyId(null);
    }
  };

  const handleToggleAvailable = async (item: MediaItem, available: boolean) => {
    if (!available) {
      const ok = await confirm({
        title: t('admin.media_hide_title', 'Hide media'),
        message: t('admin.media_hide_confirm', 'Hidden media disappears from this list until re-enabled on the storefront. Continue?'),
        confirmLabel: t('common.hide', 'Hide'),
        variant: 'danger',
      });
      if (!ok) return;
    }
    setBusyId(item.id);
    try {
      await apiClient<any>(`/owner/menu/products/${productId}/media/${item.id}`, {
        method: 'PATCH',
        body: { available },
      });
      await loadMedia();
    } catch (err) {
      console.error('[MediaManager] toggle available failed:', err);
      showToast(t('common.error_save', 'Failed to save.'), 'error');
    } finally {
      setBusyId(null);
    }
  };

  // Reorder via up/down buttons (NO dnd-kit). Optimistic local swap, then persist the
  // full order in one call (server replays it as sort_order = index).
  const handleMove = async (index: number, dir: -1 | 1) => {
    const target = index + dir;
    if (target < 0 || target >= media.length) return;
    const next = [...media];
    const a = next[index];
    const b = next[target];
    if (!a || !b) return;
    next[index] = b;
    next[target] = a;
    setMedia(next);
    try {
      await apiClient<any>(`/owner/menu/products/${productId}/media/reorder`, {
        method: 'POST',
        body: { order: next.map((m) => m.id) },
      });
    } catch (err) {
      console.error('[MediaManager] reorder failed:', err);
      showToast(t('common.error_save', 'Failed to save.'), 'error');
      await loadMedia(); // resync on failure
    }
  };

  if (!enabled) return null;

  const kindBadge = (kind: MediaKind) => {
    const map: Record<MediaKind, { icon: string; label: string }> = {
      image: { icon: 'ti ti-photo', label: t('admin.media_kind_image', 'Image') },
      video: { icon: 'ti ti-video', label: t('admin.media_kind_video', 'Video') },
      spin: { icon: 'ti ti-rotate-360', label: t('admin.media_kind_spin', '360°') },
      model: { icon: 'ti ti-cube', label: t('admin.media_kind_model', '3D') },
    };
    return map[kind] || map.image;
  };

  return (
    <div className="pt-2 border-t" style={{ borderColor: 'var(--brand-border)' }}>
      <div className="flex items-center justify-between mb-2">
        <label className="text-xs font-medium" style={{ color: 'var(--brand-text-muted)' }}>
          {t('admin.media_gallery', 'Media gallery')}
        </label>
        <label className={`flex items-center gap-1 px-3 py-1.5 text-xs font-medium rounded-lg border cursor-pointer ${uploading ? 'opacity-60 pointer-events-none' : ''}`}
          style={{ background: 'var(--brand-primary-light)', borderColor: 'var(--brand-primary)', color: 'var(--brand-text)' }}>
          <i className={uploading ? 'ti ti-loader animate-spin' : 'ti ti-upload'} />
          {uploading ? (progress || t('admin.media_uploading', 'Uploading…')) : t('admin.media_add', 'Add media')}
          <input type="file" accept={ACCEPT} onChange={handleUpload} className="hidden" disabled={uploading} />
        </label>
      </div>

      <p className="text-step-2xs mb-2" style={{ color: 'var(--brand-text-muted)' }}>
        {t('admin.media_hint', 'WebP/JPEG images or MP4 video. The first item is the storefront hero.')}
      </p>

      {loading ? (
        <div className="flex justify-center py-4">
          <i className="ti ti-loader animate-spin text-lg" style={{ color: 'var(--brand-primary)' }} />
        </div>
      ) : media.length === 0 ? (
        <p className="text-xs py-3 text-center" style={{ color: 'var(--brand-text-muted)' }}>
          {t('admin.media_empty', 'No extra media yet.')}
        </p>
      ) : (
        <div className="space-y-2">
          {media.map((item, index) => {
            const badge = kindBadge(item.kind);
            const isBusy = busyId === item.id;
            return (
              <div key={item.id} className="flex items-center gap-2 p-2 rounded-lg border"
                style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', opacity: isBusy ? 0.6 : 1 }}>
                {/* Thumbnail */}
                <div className="w-12 h-12 rounded-md overflow-hidden shrink-0 flex items-center justify-center"
                  style={{ background: 'var(--brand-primary-light)' }}>
                  {item.kind === 'image' && item.url
                    ? <img src={item.url} alt={item.alt || ''} className="w-full h-full object-cover" loading="lazy" />
                    : item.posterUrl
                      ? <img src={item.posterUrl} alt={item.alt || ''} className="w-full h-full object-cover" loading="lazy" />
                      : <i className={badge.icon} style={{ color: 'var(--brand-primary)', fontSize: '1.25rem' }} />}
                </div>

                {/* Kind badge + primary marker */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-1.5">
                    <i className={badge.icon} style={{ color: 'var(--brand-primary)', fontSize: '0.7rem' }} />
                    <span className="text-xs font-medium" style={{ color: 'var(--brand-text)' }}>{badge.label}</span>
                    {index === 0 && (
                      <span className="text-[9px] font-semibold px-1.5 py-0.5 rounded-full"
                        style={{ background: 'var(--brand-primary-light)', color: 'var(--brand-primary)' }}>
                        {t('admin.media_hero', 'Hero')}
                      </span>
                    )}
                  </div>
                </div>

                {/* Reorder up/down */}
                <div className="flex flex-col">
                  <motion.button type="button" onClick={() => handleMove(index, -1)} disabled={index === 0 || isBusy} whileTap={{ scale: 0.9 }}
                    className="w-6 h-5 flex items-center justify-center rounded hover:bg-[var(--brand-surface)] disabled:opacity-30"
                    title={t('admin.media_move_up', 'Move up')} aria-label={t('admin.media_move_up', 'Move up')}>
                    <i className="ti ti-chevron-up" style={{ fontSize: '0.7rem', color: 'var(--brand-text-muted)' }} />
                  </motion.button>
                  <motion.button type="button" onClick={() => handleMove(index, 1)} disabled={index === media.length - 1 || isBusy} whileTap={{ scale: 0.9 }}
                    className="w-6 h-5 flex items-center justify-center rounded hover:bg-[var(--brand-surface)] disabled:opacity-30"
                    title={t('admin.media_move_down', 'Move down')} aria-label={t('admin.media_move_down', 'Move down')}>
                    <i className="ti ti-chevron-down" style={{ fontSize: '0.7rem', color: 'var(--brand-text-muted)' }} />
                  </motion.button>
                </div>

                {/* Set primary */}
                <motion.button type="button" onClick={() => handleSetPrimary(item.id)} disabled={isBusy} whileTap={{ scale: 0.97 }}
                  className="px-2 py-1 text-step-2xs font-medium rounded-md border hover:bg-[var(--brand-surface)]"
                  style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}
                  title={t('admin.media_set_primary', 'Set as primary')}>
                  {t('admin.media_set_primary', 'Set primary')}
                </motion.button>

                {/* Hide (available → false) */}
                <motion.button type="button" onClick={() => handleToggleAvailable(item, false)} disabled={isBusy} whileTap={{ scale: 0.97 }}
                  className="w-7 h-7 flex items-center justify-center rounded-md hover:bg-[var(--color-danger-light)]"
                  title={t('admin.media_hide', 'Hide')} aria-label={t('admin.media_hide', 'Hide')}>
                  <i className="ti ti-eye-off" style={{ fontSize: '0.75rem', color: 'var(--brand-text-muted)' }} />
                </motion.button>
              </div>
            );
          })}
        </div>
      )}
      {confirmDialog}
    </div>
  );
}
