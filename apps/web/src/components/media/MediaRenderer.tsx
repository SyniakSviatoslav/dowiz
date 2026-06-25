// MediaRenderer — kind-dispatch registry for cinematic product media.
//
// How MenuPage integrates this (wiring owned by the lead, NOT this lane):
//   On product-modal open, MenuPage lazy-fetches the gallery via
//   GET /api/public/locations/:slug/products/:productId/media → { media: ProductMedia[] }.
//   It then renders the active item through <MediaRenderer media={item} active /> and sets
//   `active` ONLY for the currently-visible slide (one heavy decode at a time). When the
//   response is empty (feature dark, free tier, or no rows) the parent falls back to the
//   product's existing image_key/gradient — MediaRenderer is never mounted in that case.
//   `posterFallbackUrl` lets the parent pass that same image_key as a last-resort still.
//
// Code-splitting: image renders inline (no chunk). video/spin are React.lazy chunks
// loaded only when that kind is present AND `active` is true. Each lazy renderer is
// wrapped in <Suspense> (poster fallback) and an error boundary, so a renderer crash
// degrades to the poster and never throws into MenuPage.

import React, { Component, Suspense } from 'react';
import type { ReactNode } from 'react';
import type { ProductMedia } from './types';

const VideoClip = React.lazy(() => import('./VideoClip'));
const SpinViewer = React.lazy(() => import('./SpinViewer'));

const fill: React.CSSProperties = { width: '100%', height: '100%', objectFit: 'contain', display: 'block' };

function Poster({ media, posterFallbackUrl }: { media: ProductMedia; posterFallbackUrl?: string }) {
  const src = media.posterUrl || (media.kind === 'image' ? media.url : undefined) || posterFallbackUrl;
  if (!src) return null;
  return <img src={src} alt={media.alt ?? ''} style={fill} />;
}

// Tiny error boundary: any throw from a heavy renderer falls back to the poster.
class MediaErrorBoundary extends Component<{ fallback: ReactNode; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  render() { return this.state.failed ? this.props.fallback : this.props.children; }
}

export function MediaRenderer({
  media,
  active,
  posterFallbackUrl,
}: {
  media: ProductMedia;
  active: boolean;
  posterFallbackUrl?: string;
}) {
  const poster = <Poster media={media} posterFallbackUrl={posterFallbackUrl} />;

  switch (media.kind) {
    case 'image':
      // Inline, never code-split — the common case must not pay a chunk round-trip.
      return <img src={media.url} alt={media.alt ?? ''} style={fill} />;

    case 'video':
      if (!active) return poster;
      return (
        <MediaErrorBoundary fallback={poster}>
          <Suspense fallback={poster}>
            <VideoClip media={media} active={active} />
          </Suspense>
        </MediaErrorBoundary>
      );

    case 'spin':
      if (!active) return poster;
      return (
        <MediaErrorBoundary fallback={poster}>
          <Suspense fallback={poster}>
            <SpinViewer media={media} />
          </Suspense>
        </MediaErrorBoundary>
      );

    case 'model':
    default:
      // No client renderer (no WebGL/model-viewer dep) → poster → posterFallback → nothing.
      return poster;
  }
}

export default MediaRenderer;
