import { useI18n } from '../../lib/I18nProvider.js';

// Presentational undo/redo pair for client-draft editors (pairs with the
// useHistoryStack hook, but takes plain props so any history source works).
// Styled to match the admin toolbar icon-button pattern (FilterMenu trigger):
// bordered, brand-surface, focus ring on --brand-primary.

export interface UndoRedoButtonsProps {
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
  /** Overrides for the aria-labels/tooltips (default from i18n common.undo / common.redo). */
  undoLabel?: string;
  redoLabel?: string;
  className?: string;
}

const BTN_CLASS =
  'flex items-center justify-center w-9 h-9 rounded-lg border text-sm outline-none transition-colors ' +
  'disabled:opacity-40 disabled:pointer-events-none ' +
  '[@media(hover:hover)]:hover:bg-[var(--brand-surface-raised)] ' +
  'focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1';

const BTN_STYLE = {
  background: 'var(--brand-surface)',
  borderColor: 'var(--brand-border)',
  color: 'var(--brand-text)',
} as const;

export function UndoRedoButtons({ canUndo, canRedo, onUndo, onRedo, undoLabel, redoLabel, className = '' }: UndoRedoButtonsProps) {
  const { t } = useI18n();
  const undoText = undoLabel ?? t('common.undo', 'Undo');
  const redoText = redoLabel ?? t('common.redo', 'Redo');
  return (
    <div role="group" aria-label={t('common.edit_history', 'Edit history')} className={`inline-flex items-center gap-1 ${className}`}>
      <button
        type="button"
        data-testid="undo-button"
        onClick={onUndo}
        disabled={!canUndo}
        aria-label={undoText}
        title={undoText}
        className={BTN_CLASS}
        style={BTN_STYLE}
      >
        <i className="ti ti-arrow-back-up" aria-hidden="true" />
      </button>
      <button
        type="button"
        data-testid="redo-button"
        onClick={onRedo}
        disabled={!canRedo}
        aria-label={redoText}
        title={redoText}
        className={BTN_CLASS}
        style={BTN_STYLE}
      >
        <i className="ti ti-arrow-forward-up" aria-hidden="true" />
      </button>
    </div>
  );
}
