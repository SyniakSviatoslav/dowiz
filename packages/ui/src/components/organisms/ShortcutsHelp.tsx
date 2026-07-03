import { useMemo } from 'react';
import { ResponsiveDialog } from '../molecules/ResponsiveDialog.js';
import { useI18n } from '../../lib/I18nProvider.js';
import { formatKeychord, isMacPlatform } from '../../hooks/use-keyboard-shortcuts.js';

// ShortcutsHelp — the discoverable "?" overlay listing the ACTIVE shortcuts.
// Presentation-only: the caller passes exactly what it registered, so this
// sheet can never drift from reality by hardcoding its own list.

export interface ShortcutHelpItem {
  /** Keychord spec as registered (e.g. "mod+k", "g o", "escape"). */
  keys: string;
  label: string;
}

interface ShortcutsHelpProps {
  open: boolean;
  onClose: () => void;
  shortcuts: ShortcutHelpItem[];
}

export function ShortcutsHelp({ open, onClose, shortcuts }: ShortcutsHelpProps) {
  const { t } = useI18n();
  const isMac = useMemo(() => isMacPlatform(), []);

  return (
    <ResponsiveDialog open={open} onClose={onClose} title={t('shortcuts.title', 'Keyboard shortcuts')}>
      <ul data-testid="shortcuts-help" className="space-y-0.5">
        {shortcuts.map((s) => (
          <li
            key={s.keys}
            className="flex items-center justify-between gap-4 px-2 py-2 rounded-[var(--brand-radius-sm)] hover:bg-[var(--brand-surface-raised)]"
          >
            <span className="text-sm text-[var(--brand-text)]">{s.label}</span>
            <span className="flex items-center gap-1 shrink-0">
              {formatKeychord(s.keys, isMac).map((part, i) => (
                <kbd
                  key={i}
                  className="min-w-[1.6rem] px-1.5 py-0.5 text-center text-xs font-semibold rounded-md border border-[var(--brand-border)] bg-[var(--brand-surface)] text-[var(--brand-text-muted)]"
                >
                  {part}
                </kbd>
              ))}
            </span>
          </li>
        ))}
      </ul>
    </ResponsiveDialog>
  );
}
