// The first-mic-tap one-time privacy disclosure (docs/design/voice-control/ui-spec.md §5).
// Shown before the very first permission-request. "Not now" is the no-op default — it MUST NOT
// import the engine, request the mic, or fetch the model (G11 guardrail). This file's import list
// is the proof: React + i18n + ResponsiveDialog + this module's own layout/types only — no
// `@deliveryos/voice`, no apps/web adapter, nothing engine-shaped anywhere in this file.
//
// Equal affordance (C-2, same rule as the confirm chip): "Use voice" and "Not now" share the exact
// same button class/style constant as ConfirmChip's Confirm/Cancel — neither is a small grey ghost
// against a bright primary.

import { useEffect, useRef } from 'react';
import { useI18n } from '../lib/I18nProvider.js';
import { ResponsiveDialog } from '../components/molecules/ResponsiveDialog.js';
import { EQUAL_AFFORDANCE_BUTTON_CLASSNAME, EQUAL_AFFORDANCE_BUTTON_STYLE } from './layout.js';

export interface DisclosureSheetProps {
  open: boolean;
  onUse: () => void;
  onDecline: () => void;
}

export function DisclosureSheet({ open, onUse, onDecline }: DisclosureSheetProps) {
  const { t } = useI18n();
  // ResponsiveDialog (this repo's shared modal-shell primitive) does not itself restore focus on
  // close — do it here so dismissing the sheet (any path) returns focus to whatever opened it
  // (the MicFab), never stranding focus on <body> (ui-spec §7).
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (open) {
      previouslyFocusedRef.current = document.activeElement as HTMLElement | null;
    } else {
      previouslyFocusedRef.current?.focus?.();
    }
  }, [open]);

  return (
    <ResponsiveDialog open={open} onClose={onDecline} title={t('voice.setting_label')}>
      <div data-testid="voice-disclosure-sheet" className="flex flex-col gap-4">
        <p className="text-sm text-[var(--brand-text)] leading-relaxed">{t('voice.disclosure_body')}</p>
        <div className="flex gap-2">
          <button
            type="button"
            data-testid="voice-disclosure-decline"
            onClick={onDecline}
            className={EQUAL_AFFORDANCE_BUTTON_CLASSNAME}
            style={EQUAL_AFFORDANCE_BUTTON_STYLE}
          >
            {t('voice.disclosure_decline')}
          </button>
          <button
            type="button"
            data-testid="voice-disclosure-use"
            onClick={onUse}
            className={EQUAL_AFFORDANCE_BUTTON_CLASSNAME}
            style={EQUAL_AFFORDANCE_BUTTON_STYLE}
          >
            {t('voice.disclosure_use')}
          </button>
        </div>
      </div>
    </ResponsiveDialog>
  );
}
