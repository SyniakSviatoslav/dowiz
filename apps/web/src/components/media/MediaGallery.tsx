// MediaGallery — cinematic product-media slide orchestrator.
//
// How MenuPage integrates this: the lead lazy-fetches `media[]` on product-modal
// open (only when MEDIA_RICH_ENABLED + business tier), then — where today a single
// hero <img> sits — renders <MediaGallery media={media} posterFallbackUrl={…}/> when
// media.length > 1. The whole component is React.lazy code-split by the lead, so the
// gallery chunk (and any video/spin renderer chunk it pulls in) only loads when a
// rich gallery is actually shown. media.length === 1 → one zero-chrome hero; 0 →
// null (caller paints its gradient placeholder).
//
// Invariants enforced here (Phase-2 contract §MediaGallery):
//  • ONE heavy decode at a time — only the ACTIVE slide mounts a MediaRenderer with
//    active=true; neighbours are poster-prefetch only (a bare <link rel=prefetch>),
//    never a second heavy decode.
//  • ZERO CLS — every slide shares one fixed hero box. The container owns a stable
//    aspect-ratio (derived from the primary slide); slides are absolutely positioned
//    inside it, so switching slides never reflows.
//  • ZERO auto-advance. Navigation is user-driven only (buttons / arrow keys / dots).
//  • Teardown on slide change + unmount is driven purely off the `active` prop: the
//    previous slide's MediaRenderer receives active=false and tears its decoder down
//    (its own effect cleanup), so no decoder survives a slide change.

import React, { useCallback, useId, useMemo, useRef, useState } from 'react';
import { MediaRenderer } from './MediaRenderer';
import type { ProductMedia } from './types';

interface MediaGalleryProps {
  media: ProductMedia[];
  posterFallbackUrl?: string;
}

/** Aspect-ratio of the primary slide → the stable hero box (CLS = 0). 4/3 default. */
function aspectRatioOf(item: ProductMedia | undefined): string {
  if (item?.width && item.height && item.width > 0 && item.height > 0) {
    return `${item.width} / ${item.height}`;
  }
  return '4 / 3';
}

/** Best raster URL to prefetch for a neighbour without a heavy decode. */
function prefetchUrl(item: ProductMedia, fallback?: string): string | undefined {
  if (item.kind === 'image') return item.url;
  return item.posterUrl ?? fallback ?? undefined;
}

export function MediaGallery({ media, posterFallbackUrl }: MediaGalleryProps) {
  const [index, setIndex] = useState(0);
  const liveId = useId();
  const trackRef = useRef<HTMLDivElement | null>(null);

  // Clamp index defensively if the media array shrinks between renders.
  const safeIndex = media.length > 0 ? Math.min(index, media.length - 1) : 0;
  const aspectRatio = useMemo(() => aspectRatioOf(media[0]), [media]);

  const go = useCallback(
    (next: number) => {
      if (media.length === 0) return;
      const wrapped = (next + media.length) % media.length;
      setIndex(wrapped);
    },
    [media.length],
  );


  // 0 → caller shows its own gradient placeholder.
  if (media.length === 0) return null;

  // 1 → single zero-chrome hero (no buttons, no dots, no live region).
  const only = media[0];
  if (media.length === 1 && only) {
    return (
      <div className="dz-media-hero" style={{ position: 'relative', aspectRatio }}>
        <MediaRenderer media={only} active posterFallbackUrl={posterFallbackUrl} />
      </div>
    );
  }

  const current = media[safeIndex];
  const neighbours = [media[safeIndex - 1], media[safeIndex + 1]].filter(Boolean) as ProductMedia[];

  return (
    // Carousel region. Keyboard navigation is via the prev/next <button>s below (real buttons,
    // fully keyboard-accessible) + the dot controls — so the container itself stays a plain,
    // non-focusable group (no roving tabindex needed).
    <div
      className="dz-media-gallery"
      role="group"
      aria-roledescription="carousel"
      aria-label="Product media"
      ref={trackRef}
      style={{ position: 'relative', width: '100%', height: '100%' }}
    >
      {/* Stable hero box — fills a fixed-aspect parent (the modal hero) when present; falls
          back to its own aspect-ratio standalone (CLS = 0). Slides absolutely positioned. */}
      <div className="dz-media-stage" style={{ position: 'relative', width: '100%', height: '100%', aspectRatio, overflow: 'hidden' }}>
        {media.map((item, i) => (
          <div
            key={item.id}
            className="dz-media-slide"
            aria-hidden={i !== safeIndex}
            style={{
              position: 'absolute',
              inset: 0,
              opacity: i === safeIndex ? 1 : 0,
              transition: 'opacity 180ms ease',
              pointerEvents: i === safeIndex ? 'auto' : 'none',
            }}
          >
            {/* Only the active slide mounts a renderer → one heavy decode at a time.
                When the index changes, the previous slide unmounts → its MediaRenderer
                teardown fires (decoder released). active prop drives this entirely. */}
            {i === safeIndex ? (
              <MediaRenderer media={item} active posterFallbackUrl={posterFallbackUrl} />
            ) : null}
          </div>
        ))}
      </div>

      {/* Neighbour poster prefetch — raster only, never a heavy decode. */}
      {neighbours.map((item) => {
        const href = prefetchUrl(item, posterFallbackUrl);
        return href ? <link key={item.id} rel="prefetch" as="image" href={href} /> : null;
      })}

      <button
        type="button"
        className="dz-media-prev"
        aria-label="Previous media"
        onClick={() => go(safeIndex - 1)}
        style={navBtnStyle('left')}
      >
        <span aria-hidden="true">‹</span>
      </button>
      <button
        type="button"
        className="dz-media-next"
        aria-label="Next media"
        onClick={() => go(safeIndex + 1)}
        style={navBtnStyle('right')}
      >
        <span aria-hidden="true">›</span>
      </button>

      {/* Dot indicators. */}
      <div className="dz-media-dots" role="tablist" aria-label="Choose media" style={dotsRowStyle}>
        {media.map((item, i) => (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={i === safeIndex}
            aria-label={`Show media ${i + 1} of ${media.length}`}
            onClick={() => go(i)}
            style={dotStyle(i === safeIndex)}
          />
        ))}
      </div>

      {/* Polite announcer — "N of M" + the current slide's alt text. */}
      <div id={liveId} aria-live="polite" className="dz-sr-only" style={srOnlyStyle}>
        {`${safeIndex + 1} of ${media.length}`}
        {current?.alt ? `: ${current.alt}` : ''}
      </div>
    </div>
  );
}

// ---- inline styles (no new deps; keep chrome self-contained) -------------------

function navBtnStyle(side: 'left' | 'right'): React.CSSProperties {
  return {
    position: 'absolute',
    top: '50%',
    [side]: 8,
    transform: 'translateY(-50%)',
    width: 36,
    height: 36,
    borderRadius: '50%',
    border: 'none',
    background: 'rgba(0,0,0,0.45)',
    color: '#fff',
    fontSize: 22,
    lineHeight: 1,
    cursor: 'pointer',
    zIndex: 2,
  };
}

const dotsRowStyle: React.CSSProperties = {
  position: 'absolute',
  bottom: 8,
  left: 0,
  right: 0,
  display: 'flex',
  justifyContent: 'center',
  gap: 6,
  zIndex: 2,
};

function dotStyle(active: boolean): React.CSSProperties {
  return {
    width: active ? 18 : 8,
    height: 8,
    borderRadius: 4,
    border: 'none',
    padding: 0,
    background: active ? '#fff' : 'rgba(255,255,255,0.5)',
    cursor: 'pointer',
    transition: 'width 160ms ease',
  };
}

const srOnlyStyle: React.CSSProperties = {
  position: 'absolute',
  width: 1,
  height: 1,
  padding: 0,
  margin: -1,
  overflow: 'hidden',
  clip: 'rect(0,0,0,0)',
  whiteSpace: 'nowrap',
  border: 0,
};

export default MediaGallery;
