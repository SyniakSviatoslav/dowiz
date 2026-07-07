import React, { useEffect, useRef, useState } from 'react';
import { useI18n } from '@deliveryos/ui';

/**
 * CityPopRadio — a subtle, discoverable easter egg on the entry page.
 * A dim "88.2 FM" chip (bottom-left) most people will scroll past; hover glows it,
 * click (or press "R") tunes in an actual Japanese city-pop live-radio stream.
 *
 * Audio source is a YouTube live-radio embed — licensing handled by the host
 * platform (we embed, we do not rehost). SWAP `CITY_POP.youtubeId` for your
 * preferred free, embeddable city-pop live stream / playlist. If the embed is
 * blocked (CSP frame-src) or offline, the panel shows a dry-wit "signal lost"
 * state instead of breaking. Play/stop = mount/unmount the iframe (no JS API).
 *
 * NOTE (operator): confirm the stream id you want, and allow
 * `frame-src https://www.youtube.com` in the staging/prod CSP for it to play there.
 */
const CITY_POP = {
  // Empty until a verified live/embeddable city-pop stream id is set (see NOTE above).
  // With no id, the panel opens but shows the dry-wit "signal lost" state (never broken).
  youtubeId: '',
  station: 'DOWIZ FM',
  freq: '88.2',
};

// Whether we have a plausibly-real id configured (guards the placeholder).
const HAS_STREAM = /^[A-Za-z0-9_-]{11}$/.test(CITY_POP.youtubeId);

export function CityPopRadio() {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [failed, setFailed] = useState(false);
  const failTimer = useRef<number | null>(null);

  // Discoverability #2: press "R" to tune in (ignored while typing in a field).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement | null;
      if (el && /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName)) return;
      if (e.key.toLowerCase() === 'r' && !e.metaKey && !e.ctrlKey && !e.altKey) {
        setOpen((o) => !o);
      }
      if (e.key === 'Escape') setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const tuneIn = () => {
    setOpen(true);
    setPlaying(true);
    setFailed(false);
    // If the iframe never loads (blocked/offline), fall to "signal lost".
    if (failTimer.current) window.clearTimeout(failTimer.current);
    failTimer.current = window.setTimeout(() => setFailed(true), 6000);
  };
  const stop = () => {
    setPlaying(false);
    if (failTimer.current) window.clearTimeout(failTimer.current);
  };

  const src =
    `https://www.youtube-nocookie.com/embed/${CITY_POP.youtubeId}` +
    `?autoplay=1&loop=1&playlist=${CITY_POP.youtubeId}&modestbranding=1&playsinline=1`;

  return (
    <div className="lp-radio" data-open={open ? 'true' : 'false'}>
      {/* Collapsed chip — the easter egg. Dim set-dressing until you notice it. */}
      {!open && (
        <button
          className="lp-radio__chip"
          onClick={() => setOpen(true)}
          aria-label={t('lp.radio_hint', 'Tune in — city pop radio')}
          title={t('lp.radio_hint', 'Tune in — city pop radio')}
        >
          <span className="lp-radio__dot" />
          <span className="lp-mono">{CITY_POP.freq} FM</span>
        </button>
      )}

      {/* Expanded cassette/CRT panel */}
      {open && (
        <div className="lp-radio__panel" role="dialog" aria-label={t('lp.radio_station', 'DOWIZ FM')}>
          <div className="lp-radio__head">
            <span className="lp-eyebrow" style={{ fontSize: 11 }}>{CITY_POP.station} · {CITY_POP.freq}</span>
            <button className="lp-radio__x" onClick={() => { setOpen(false); stop(); }} aria-label={t('lp.radio_close', 'Close')}>✕</button>
          </div>

          <div className="lp-radio__body">
            <div className={`lp-radio__eq${playing && !failed ? ' is-live' : ''}`} aria-hidden="true">
              <span /><span /><span /><span /><span />
            </div>
            <div className="lp-radio__meta">
              <div className="lp-radio__now lp-mono">
                {failed || !HAS_STREAM
                  ? t('lp.radio_off', 'Signal lost — the night shift is on a smoke break.')
                  : playing
                    ? t('lp.radio_now', 'Now playing — Japanese city pop')
                    : t('lp.radio_paused', 'Paused. The night is still out there.')}
              </div>
              <div className="lp-radio__genre">{t('lp.radio_genre', 'City pop // for the 2 a.m. shift')}</div>
            </div>
          </div>

          <div className="lp-radio__ctrls">
            {playing
              ? <button className="lp-radio__btn" onClick={stop}>{t('lp.radio_stop', 'Stop')}</button>
              : <button className="lp-radio__btn lp-radio__btn--on" onClick={tuneIn} disabled={!HAS_STREAM}>{t('lp.radio_play', 'Tune in')}</button>}
          </div>

          {/* Hidden audio-bearing iframe (present, offscreen — display:none kills YT audio). */}
          {playing && HAS_STREAM && !failed && (
            <iframe
              title="citypop"
              className="lp-radio__frame"
              src={src}
              allow="autoplay; encrypted-media"
              onLoad={() => { if (failTimer.current) window.clearTimeout(failTimer.current); }}
            />
          )}
        </div>
      )}
    </div>
  );
}
