// The MicFab — a persistent mic glyph wrapped in a bloom of concentric "halo" rings
// (docs/design/voice-control/VOICE-UI-REFERENCE.md §2). Idle is STATIC (no idle pulse —
// anti-surveillance, ui-spec §1); motion begins only after a tap. Pure CSS transform/opacity on
// pseudo-elements (`.mic-fab[data-state]::before/::after`, theme/tokens.css) + ONE
// requestAnimationFrame loop writing a single `--amp` custom property for amplitude reactivity —
// no WebGL/Lottie/new dep.

import { useEffect, useRef } from 'react';
import { useReducedMotion } from 'framer-motion';
import { useI18n } from '../lib/I18nProvider.js';
import { shouldAnimateHalo, smoothAmplitude, type VoicePhase } from './state-machine.js';
import { FAB_POSITION_STYLE } from './layout.js';
import type { VoiceAmplitudeSource } from './types.js';

export interface MicFabProps {
  phase: VoicePhase;
  onTap: () => void;
  /**
   * The render predicate (WebGPU probe / runtime config / build flag / user pref / secure
   * context) is computed at the MOUNT site, not here (ui-spec §1) — this prop is the single
   * boolean gate. Defaults to `true` so a consumer may also simply not render `<MicFab>` at all;
   * both forms satisfy "unmounted while any modal is open."
   */
  visible?: boolean;
  /** Optional live amplitude source (e.g. an AnalyserNode-backed RMS reader owned by the engine).
   *  Without one the halo still blooms (the calm "breath" loop) but `--amp` stays 0. */
  amplitudeSource?: VoiceAmplitudeSource;
  className?: string;
}

const GLYPH_BY_PHASE: Record<VoicePhase['type'], string> = {
  idle: 'ti-microphone',
  disclosure: 'ti-microphone',
  'permission-request': 'ti-microphone',
  listening: 'ti-microphone-filled',
  transcribing: 'ti-microphone-filled',
  confirming: 'ti-microphone-filled',
  applied: 'ti-check',
  disambiguating: 'ti-microphone-filled',
  error: 'ti-microphone',
};

function ariaKeyForPhase(phase: VoicePhase): string {
  switch (phase.type) {
    case 'listening':
      return 'voice.listening';
    case 'transcribing':
      return 'voice.transcribing';
    case 'applied':
      return 'voice.applied';
    case 'error':
      return phase.kind === 'mic_denied' ? 'voice.err.mic_denied' : 'voice.fab_label';
    default:
      return 'voice.fab_label';
  }
}

export function MicFab({ phase, onTap, visible = true, amplitudeSource, className = '' }: MicFabProps) {
  const { t } = useI18n();
  const prefersReducedMotion = !!useReducedMotion();
  const rootRef = useRef<HTMLButtonElement>(null);
  const rafRef = useRef<number | null>(null);
  const ampRef = useRef(0);

  const animateHalo = shouldAnimateHalo(phase.type, prefersReducedMotion);

  // The ONE requestAnimationFrame loop (VOICE-UI-REFERENCE.md §3) — writes a single `--amp`
  // custom property directly on the DOM node (bypassing React re-renders, which is the whole
  // point at 60fps) so the CSS halo ring can react to live mic amplitude. Runs only while
  // 'listening' and motion is not reduced; stops and clears --amp otherwise.
  useEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    if (!animateHalo) {
      ampRef.current = 0;
      el.style.setProperty('--amp', '0');
      return;
    }
    const tick = () => {
      const raw = amplitudeSource?.getAmplitude() ?? 0;
      ampRef.current = smoothAmplitude(ampRef.current, raw);
      el.style.setProperty('--amp', ampRef.current.toFixed(3));
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    };
  }, [animateHalo, amplitudeSource]);

  if (!visible) return null;

  const label = t(ariaKeyForPhase(phase));
  const glyph = GLYPH_BY_PHASE[phase.type];
  const showSpinner = phase.type === 'transcribing' && !prefersReducedMotion;

  return (
    <button
      ref={rootRef}
      type="button"
      data-testid="voice-mic-fab"
      data-state={phase.type}
      onClick={onTap}
      aria-label={label}
      className={`mic-fab z-sticky rounded-full flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 ${className}`}
      style={{
        ...FAB_POSITION_STYLE,
        background: 'var(--brand-primary)',
        color: 'var(--color-on-primary)',
        boxShadow: 'var(--elev-3)',
      }}
    >
      {showSpinner ? (
        <span className="mic-fab__spinner" aria-hidden="true" />
      ) : (
        <i className={`ti ${glyph}`} aria-hidden="true" style={{ fontSize: '1.5rem' }} />
      )}
    </button>
  );
}

