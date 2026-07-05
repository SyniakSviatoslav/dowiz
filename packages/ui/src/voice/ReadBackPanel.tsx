// The "read my order" READ_ONLY checkout surface (docs/design/voice-control/ui-spec.md §4). Reads
// back the user's OWN client-side cart lines + total — no new network call, no new PII egress, no
// cross-tenant surface. The mount site owns the actual cart data (this module has no CartProvider
// import) and passes already-formatted lines/total in.
//
// Wrapped in `aria-live="polite"` + `role="status"` so a screen reader actually ANNOUNCES the
// lines + total when voice triggers it — the announcement is the point, the panel is the mirror.

import { useI18n } from '../lib/I18nProvider.js';
import { ResponsiveDialog } from '../components/molecules/ResponsiveDialog.js';

export interface ReadBackLine {
  readonly qty: number;
  readonly name: string;
  /** Already money-formatted (e.g. via the app's `formatMoney` + active currency) — this module
   *  does not know about currencies/locale-formatting. */
  readonly amount: string;
}

export interface ReadBackPanelProps {
  open: boolean;
  lines: readonly ReadBackLine[];
  total: string;
  onClose: () => void;
}

export function ReadBackPanel({ open, lines, total, onClose }: ReadBackPanelProps) {
  const { t } = useI18n();
  return (
    <ResponsiveDialog open={open} onClose={onClose} title={t('voice.read_order_title')}>
      <div data-testid="voice-read-back-panel" role="status" aria-live="polite" className="flex flex-col gap-2">
        <ul className="flex flex-col gap-1.5">
          {lines.map((line, i) => (
            <li key={i} className="flex justify-between gap-3 text-sm text-[var(--brand-text)]">
              <span>
                {line.qty}× {line.name}
              </span>
              <span className="tabular-nums text-[var(--brand-text-muted)]">{line.amount}</span>
            </li>
          ))}
        </ul>
        <div className="flex justify-between gap-3 pt-2 border-t border-[var(--brand-border)] text-sm font-semibold text-[var(--brand-text)]">
          <span>{t('voice.read_order_total')}</span>
          <span className="tabular-nums">{total}</span>
        </div>
      </div>
    </ResponsiveDialog>
  );
}
