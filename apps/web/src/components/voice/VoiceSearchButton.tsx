// Push-to-talk voice SEARCH for the storefront menu (Phase A, plan §8 / ADR-0015).
//
// WHAT IT REUSES (no STT/NLU reimplemented here):
//   @deliveryos/voice  →  TransformersTranscriber (whisper-base, on-device) + WhisperProvider
//                         (audio→ASR→matchIntent → IntentProposal) + ConfirmationGate (the sole sink).
//   This component only adds the two genuinely-new pieces: (1) browser mic capture → 16 kHz mono
//   Float32 PCM (the transcriber's PcmAudio contract), and (2) the MicFab UI + a11y wiring.
//
// SAFETY / SCOPE (this increment is READ-ONLY, search-only):
//   - The engine holds zero write capability; the ConfirmationGate is the only sink. We wire ONLY the
//     setSearch handler to real work — every other handler is a no-op, so no non-search intent can act,
//     and ADD_TO_CART (the one STATEFUL intent) is held pending and never confirmed here.
//   - SET_SEARCH is READ_ONLY → the gate auto-applies it → onSetSearch(query). No cart, no money, no
//     confirm chip, no VAD, no TTS (those are later phases C/D/E).
//   - Zero egress: transcription is on-device (whisper via transformers.js); audio never leaves the tab.
//   - True-dark: @deliveryos/voice (and the ML dep behind it) is dynamic-import()ed only after the flag
//     is on AND the user actually presses to talk — with VITE_VOICE_CONTROL off this file's render
//     predicate returns null and the engine is never imported.

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useReducedMotion } from 'framer-motion';
import { useI18n } from '@deliveryos/ui';
import type { ConfirmationGate, Transcriber } from '@deliveryos/voice';
import {
  intentToSearchQuery,
  voiceSearchAvailability,
  type Locale,
  type MenuContext,
} from './voiceSearchIntent';

const FLAG_ENABLED = import.meta.env.VITE_VOICE_CONTROL === 'true';

type VoiceState = 'idle' | 'listening' | 'transcribing' | 'error';

export interface VoiceSearchButtonProps {
  /** The storefront search setter — the SINGLE write surface this component may touch. */
  readonly onSetSearch: (query: string) => void;
  /** Public menu vocabulary (products + categories) for the matcher's slot-fill. */
  readonly menu: MenuContext;
  /** Override the active locale (defaults to the app i18n locale). */
  readonly locale?: Locale;
  readonly className?: string;
}

/** Downsampled to whatever rate the AudioContext runs at (we request 16 kHz — whisper's contract). */
const TARGET_SAMPLE_RATE = 16000;

/**
 * The push-to-talk MicFab. Renders nothing when voice cannot work (flag off / insecure context / no
 * mic) — voice is strictly additive, so an unusable control is absent, never greyed (plan §6).
 */
export function VoiceSearchButton({ onSetSearch, menu, locale, className }: VoiceSearchButtonProps) {
  const { t, locale: appLocale } = useI18n();
  const prefersReduced = useReducedMotion();
  const effectiveLocale = (locale ?? appLocale) as Locale;

  const availability = useMemo(
    () =>
      voiceSearchAvailability({
        flagEnabled: FLAG_ENABLED,
        secureContext: typeof window !== 'undefined' && window.isSecureContext === true,
        hasMediaDevices:
          typeof navigator !== 'undefined' &&
          typeof navigator.mediaDevices?.getUserMedia === 'function',
      }),
    [],
  );

  const [state, setState] = useState<VoiceState>('idle');
  const [status, setStatus] = useState<string>('');

  // Live refs so async capture callbacks never read stale props/values.
  const onSetSearchRef = useRef(onSetSearch);
  onSetSearchRef.current = onSetSearch;
  const menuRef = useRef(menu);
  menuRef.current = menu;

  // Warm engine handles cached across presses so the ~130 MB whisper model loads once, not per tap.
  const transcriberRef = useRef<Transcriber | null>(null);
  const gateRef = useRef<ConfirmationGate | null>(null);

  // Capture graph refs.
  const streamRef = useRef<MediaStream | null>(null);
  const audioCtxRef = useRef<AudioContext | null>(null);
  const processorRef = useRef<ScriptProcessorNode | null>(null);
  const sourceRef = useRef<MediaStreamAudioSourceNode | null>(null);
  const chunksRef = useRef<Float32Array[]>([]);
  const holdingRef = useRef(false);

  const announce = useCallback((msg: string) => setStatus(msg), []);

  const teardownCapture = useCallback(() => {
    try {
      processorRef.current?.disconnect();
      sourceRef.current?.disconnect();
    } catch {
      /* ignore */
    }
    processorRef.current = null;
    sourceRef.current = null;
    if (audioCtxRef.current) {
      void audioCtxRef.current.close().catch(() => {});
      audioCtxRef.current = null;
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((tr) => tr.stop()); // release the mic immediately
      streamRef.current = null;
    }
  }, []);

  useEffect(() => () => teardownCapture(), [teardownCapture]);

  const startCapture = useCallback(async () => {
    if (holdingRef.current || state === 'transcribing') return;
    holdingRef.current = true;
    chunksRef.current = [];
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      if (!holdingRef.current) {
        // Released before permission resolved — drop it.
        stream.getTracks().forEach((tr) => tr.stop());
        return;
      }
      streamRef.current = stream;
      const Ctx: typeof AudioContext =
        window.AudioContext ?? (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      const ctx = new Ctx({ sampleRate: TARGET_SAMPLE_RATE });
      audioCtxRef.current = ctx;
      const source = ctx.createMediaStreamSource(stream);
      sourceRef.current = source;
      const processor = ctx.createScriptProcessor(4096, 1, 1);
      processorRef.current = processor;
      processor.onaudioprocess = (e) => {
        if (!holdingRef.current) return;
        chunksRef.current.push(new Float32Array(e.inputBuffer.getChannelData(0)));
      };
      source.connect(processor);
      processor.connect(ctx.destination);
      setState('listening');
      announce(t('voice.search.listening', 'Listening…'));
    } catch {
      holdingRef.current = false;
      teardownCapture();
      setState('error');
      announce(t('voice.search.mic_denied', 'Microphone unavailable'));
    }
  }, [state, t, announce, teardownCapture]);

  const stopCapture = useCallback(async () => {
    if (!holdingRef.current) return;
    holdingRef.current = false;
    const chunks = chunksRef.current;
    chunksRef.current = [];
    teardownCapture();

    const total = chunks.reduce((n, c) => n + c.length, 0);
    if (total === 0) {
      setState('idle');
      announce('');
      return;
    }
    const pcm = new Float32Array(total);
    let off = 0;
    for (const c of chunks) {
      pcm.set(c, off);
      off += c.length;
    }

    setState('transcribing');
    announce(t('voice.search.thinking', 'One moment…'));
    try {
      // TRUE-DARK: the engine (and its ML dep) enters the graph only here, on a real press.
      const voice = await import('@deliveryos/voice');
      if (!transcriberRef.current) {
        transcriberRef.current = new voice.TransformersTranscriber(effectiveLocale, {
          device: 'webgpu',
          dtype: 'q8',
        });
      }
      if (!gateRef.current) {
        const noop = () => {};
        gateRef.current = new voice.ConfirmationGate({
          addToCart: noop,
          setSort: noop,
          setMacroLens: noop,
          selectCategory: noop,
          // The ONE real handler: SET_SEARCH → the storefront search box. Read-only, auto-applied.
          setSearch: (args) => {
            const q = intentToSearchQuery({ kind: 'SET_SEARCH', args, transcript: '', confidence: 1 });
            if (q) onSetSearchRef.current(q);
          },
          toggleCompare: noop,
          readOrder: noop,
          navigateCheckout: noop,
        });
      }
      const provider = new voice.WhisperProvider(transcriberRef.current, effectiveLocale, menuRef.current);
      const { proposal } = await provider.once(pcm);
      const query = intentToSearchQuery(proposal);
      if (proposal) gateRef.current.submit(proposal); // gate auto-applies SET_SEARCH via setSearch handler
      setState('idle');
      announce(
        query
          ? t('voice.search.applied', 'Searching for {{query}}', { query })
          : t('voice.search.not_understood', 'Sorry, I did not catch that'),
      );
    } catch {
      setState('error');
      announce(t('voice.search.engine_error', 'Voice search is unavailable right now'));
    }
  }, [effectiveLocale, t, announce, teardownCapture]);

  if (!availability.render) return null;

  const listening = state === 'listening';
  const busy = state === 'transcribing';
  const label = listening
    ? t('voice.search.stop_label', 'Release to search')
    : t('voice.search.label', 'Search by voice');

  return (
    <>
      <button
        type="button"
        aria-label={label}
        aria-pressed={listening}
        disabled={busy}
        title={label}
        className={className}
        // Push-to-talk: hold (pointer or keyboard) to capture, release to search.
        onPointerDown={(e) => {
          e.preventDefault();
          void startCapture();
        }}
        onPointerUp={() => void stopCapture()}
        onPointerLeave={() => {
          if (holdingRef.current) void stopCapture();
        }}
        onKeyDown={(e) => {
          if ((e.key === ' ' || e.key === 'Enter') && !e.repeat) {
            e.preventDefault();
            void startCapture();
          }
        }}
        onKeyUp={(e) => {
          if (e.key === ' ' || e.key === 'Enter') {
            e.preventDefault();
            void stopCapture();
          }
        }}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: 44,
          height: 44,
          borderRadius: '9999px',
          border: `1px solid var(--brand-primary, #4f46e5)`,
          background: listening ? 'var(--brand-primary, #4f46e5)' : 'transparent',
          color: listening ? '#fff' : 'var(--brand-primary, #4f46e5)',
          cursor: busy ? 'wait' : 'pointer',
          // reduced-motion: a static ring instead of the listening pulse (plan §6).
          animation: listening && !prefersReduced ? 'voiceSearchPulse 1.2s ease-in-out infinite' : 'none',
          transition: 'background var(--motion-base, 160ms) ease, color var(--motion-base, 160ms) ease',
          touchAction: 'none',
        }}
      >
        <i className={busy ? 'ti ti-loader-2' : 'ti ti-microphone'} aria-hidden="true" />
        <style>{`@keyframes voiceSearchPulse{0%,100%{box-shadow:0 0 0 0 var(--brand-primary,#4f46e5)}50%{box-shadow:0 0 0 6px rgba(79,70,229,0.18)}}`}</style>
      </button>
      {/* Visually-hidden live region: every state change is announced for screen readers. */}
      <span
        aria-live={state === 'error' ? 'assertive' : 'polite'}
        style={{
          position: 'absolute',
          width: 1,
          height: 1,
          padding: 0,
          margin: -1,
          overflow: 'hidden',
          clip: 'rect(0 0 0 0)',
          whiteSpace: 'nowrap',
          border: 0,
        }}
      >
        {status}
      </span>
    </>
  );
}

export default VoiceSearchButton;
