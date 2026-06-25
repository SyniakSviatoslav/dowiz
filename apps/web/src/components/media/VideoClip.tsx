// Native <video> clip — muted, looping, playsInline. No video library.
//
// Data-saver guard: if useSaveData() is true we render the poster ONLY and never
// set src / fetch the video. prefers-reduced-motion → start paused on the poster.
// WCAG 2.2.2: a visible pause/play control is shown whenever the clip is playing
// (and to resume). When `active` goes false, or on unmount, we pause() and clear
// src='' so the decoder is released (no leaked decoders across many slides).
// onError degrades to the poster.

import { useEffect, useRef, useState } from 'react';
import type { ProductMedia } from './types';
import { useReducedMotion, useSaveData } from './hooks';

export default function VideoClip({ media, active = true }: { media: ProductMedia; active?: boolean }) {
  const reduced = useReducedMotion();
  const saveData = useSaveData();
  const ref = useRef<HTMLVideoElement | null>(null);
  const [playing, setPlaying] = useState(false);
  const [failed, setFailed] = useState(false);
  const poster = media.posterUrl || media.url;

  // Drive playback from `active` + reduced-motion. Releasing src on deactivate /
  // unmount frees the decoder.
  useEffect(() => {
    const v = ref.current;
    if (!v || saveData || failed) return;
    if (active && !reduced) {
      if (v.getAttribute('src') !== media.url) v.setAttribute('src', media.url);
      v.play().then(() => setPlaying(true)).catch(() => { /* autoplay blocked → poster + control */ });
    } else {
      v.pause();
      setPlaying(false);
      if (!active) { v.removeAttribute('src'); v.load(); } // release decoder when offscreen
    }
    return () => {
      v.pause();
      v.removeAttribute('src');
      v.load();
      setPlaying(false);
    };
  }, [active, reduced, saveData, failed, media.url]);

  const toggle = () => {
    const v = ref.current;
    if (!v) return;
    if (v.paused) {
      if (v.getAttribute('src') !== media.url) v.setAttribute('src', media.url);
      v.play().then(() => setPlaying(true)).catch(() => setPlaying(false));
    } else {
      v.pause();
      setPlaying(false);
    }
  };

  // Data-saver or load failure → poster only, no video element / fetch at all.
  if (saveData || failed) {
    return (
      <img src={poster} alt={media.alt ?? ''} style={{ width: '100%', height: '100%', objectFit: 'contain', display: 'block' }} />
    );
  }

  return (
    <div style={{ position: 'relative', width: '100%', height: '100%' }}>
      <video
        ref={ref}
        muted
        loop
        playsInline
        poster={poster}
        preload="none"
        onPlaying={() => setPlaying(true)}
        onPause={() => setPlaying(false)}
        onError={() => setFailed(true)}
        aria-label={media.alt ?? undefined}
        style={{ width: '100%', height: '100%', objectFit: 'contain', display: 'block' }}
      />
      <button
        type="button"
        onClick={toggle}
        aria-label={playing ? 'Pause' : 'Play'}
        style={{
          position: 'absolute', bottom: 8, right: 8, width: 'var(--tap-min, 44px)', height: 'var(--tap-min, 44px)',
          display: 'grid', placeItems: 'center', borderRadius: '50%', border: 'none', cursor: 'pointer',
          background: 'rgba(0,0,0,0.55)', color: '#fff',
        }}
      >
        <i className={playing ? 'ti ti-player-pause' : 'ti ti-player-play'} aria-hidden="true" />
      </button>
    </div>
  );
}
