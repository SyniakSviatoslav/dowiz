// The confirmation affordance for the STATEFUL add-to-cart proposal (safety-critical —
// docs/design/voice-control/ui-spec.md §3, C-2/STOP-2). A non-modal chip anchored above the
// MicFab echoing the PARSED intent before any write. This is the ONLY path from a STATEFUL
// proposal to the injected gate's `confirm()` — nothing else in this module calls it.
//
// Equal affordance weight is a HARD assertion (STOP-2/C-2): Confirm and Cancel share the exact
// same button class/style constant (layout.ts) — identical size/background/border/color/padding.
// A glyph may differ; nothing else may. 12s timeout / Esc / outside-tap all resolve to Cancel
// (fail-safe default = no write).

import { useEffect, useRef } from 'react';
import { useI18n } from '../lib/I18nProvider.js';
import { ANCHOR_ABOVE_FAB_STYLE, EQUAL_AFFORDANCE_BUTTON_CLASSNAME, EQUAL_AFFORDANCE_BUTTON_STYLE } from './layout.js';
import { extractAddToCartLabel } from './state-machine.js';
import type { VoiceProposal } from './types.js';

const DEFAULT_TIMEOUT_MS = 12_000;

export interface ConfirmChipProps {
  proposal: VoiceProposal;
  onConfirm: () => void;
  onCancel: () => void;
  timeoutMs?: number;
}

export function ConfirmChip({ proposal, onConfirm, onCancel, timeoutMs = DEFAULT_TIMEOUT_MS }: ConfirmChipProps) {
  const { t } = useI18n();
  const containerRef = useRef<HTMLDivElement>(null);
  const { qty, item } = extractAddToCartLabel(proposal);
  const message = t('voice.confirm_add', undefined, { qty, item });

  // Focus the chip on appear (a keyboard/SR user must land on the decision, ui-spec §7). Neither
  // button is a default-Enter primary — a STATEFUL write must be a deliberate Tab+activate choice.
  useEffect(() => {
    containerRef.current?.focus();
  }, []);

  // Fail-safe timeout: no decision within `timeoutMs` ⇒ Cancel (no write). A safety surface
  // defaults to not acting.
  useEffect(() => {
    const timer = setTimeout(onCancel, timeoutMs);
    return () => clearTimeout(timer);
  }, [onCancel, timeoutMs]);

  // Esc = Cancel (explicit, never a silent confirm).
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onCancel]);

  // Outside-tap = Cancel.
  useEffect(() => {
    const handler = (e: PointerEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) onCancel();
    };
    document.addEventListener('pointerdown', handler);
    return () => document.removeEventListener('pointerdown', handler);
  }, [onCancel]);

  return (
    <div
      ref={containerRef}
      tabIndex={-1}
      role="group"
      aria-live="polite"
      aria-label={message}
      data-testid="voice-confirm-chip"
      className="rounded-[var(--brand-radius)] p-3 flex flex-col gap-3 outline-none"
      style={{
        ...ANCHOR_ABOVE_FAB_STYLE,
        background: 'var(--brand-surface)',
        border: '1px solid var(--brand-border)',
        boxShadow: 'var(--elev-4)',
      }}
    >
      <p className="text-sm font-medium text-[var(--brand-text)]">{message}</p>
      <div className="flex gap-2">
        <button
          type="button"
          data-testid="voice-confirm-cancel"
          onClick={onCancel}
          className={EQUAL_AFFORDANCE_BUTTON_CLASSNAME}
          style={EQUAL_AFFORDANCE_BUTTON_STYLE}
        >
          <i className="ti ti-x" aria-hidden="true" />
          {t('voice.cancel')}
        </button>
        <button
          type="button"
          data-testid="voice-confirm-confirm"
          onClick={onConfirm}
          className={EQUAL_AFFORDANCE_BUTTON_CLASSNAME}
          style={EQUAL_AFFORDANCE_BUTTON_STYLE}
        >
          <i className="ti ti-check" aria-hidden="true" />
          {t('voice.confirm')}
        </button>
      </div>
    </div>
  );
}
