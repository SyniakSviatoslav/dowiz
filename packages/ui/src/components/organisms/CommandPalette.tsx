import { useEffect, useMemo, useRef, useState } from 'react';
import { ResponsiveDialog } from '../molecules/ResponsiveDialog.js';
import { useI18n } from '../../lib/I18nProvider.js';
import { formatKeychord, isMacPlatform } from '../../hooks/use-keyboard-shortcuts.js';

// CommandPalette — ⌘K-style fuzzy launcher. Presentation-only: the caller owns
// the command list (nav + safe quick-actions) and open state; ResponsiveDialog
// supplies the a11y shell (focus trap, Escape, focus restore, portal).

export interface PaletteCommand {
  id: string;
  label: string;
  /** Tabler icon class, e.g. "ti ti-clipboard-list". */
  icon?: string;
  /** Muted right-of-label context, e.g. the route. */
  hint?: string;
  /** Extra corpus for fuzzy matching (not displayed). */
  keywords?: string;
  /** Display-only keychord spec (e.g. "g o") rendered as <kbd>. */
  shortcut?: string;
  perform: () => void;
}

/**
 * Pure fuzzy scorer: -1 = no match, higher = better. Exact substring beats
 * subsequence; earlier/word-start/adjacent hits rank higher. Exported for tests.
 */
export function fuzzyScore(query: string, text: string): number {
  const q = query.trim().toLowerCase();
  const t = text.toLowerCase();
  if (!q) return 0;
  const idx = t.indexOf(q);
  if (idx >= 0) return 1000 - idx;
  let score = 0;
  let ti = 0;
  let prev = -1;
  for (const ch of q) {
    let found = -1;
    while (ti < t.length) {
      if (t[ti] === ch) { found = ti; break; }
      ti++;
    }
    if (found === -1) return -1;
    score += 10;
    if (found === prev + 1) score += 5; // adjacent run
    if (found === 0 || t[found - 1] === ' ' || t[found - 1] === '/') score += 8; // word start
    prev = found;
    ti = found + 1;
  }
  return score - Math.floor(prev / 4); // light spread penalty
}

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  commands: PaletteCommand[];
  placeholder?: string;
}

export function CommandPalette({ open, onClose, commands, placeholder }: CommandPaletteProps) {
  const { t } = useI18n();
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);
  const isMac = useMemo(() => isMacPlatform(), []);

  // Fresh state every time the palette opens.
  useEffect(() => {
    if (open) {
      setQuery('');
      setSelected(0);
    }
  }, [open]);

  const filtered = useMemo(() => {
    if (!query.trim()) return commands;
    return commands
      .map((cmd) => ({ cmd, score: fuzzyScore(query, `${cmd.label} ${cmd.keywords ?? ''}`) }))
      .filter((x) => x.score >= 0)
      .sort((a, b) => b.score - a.score)
      .map((x) => x.cmd);
  }, [commands, query]);

  // Keep the selection valid + visible as the filter narrows.
  useEffect(() => {
    setSelected((s) => Math.min(s, Math.max(0, filtered.length - 1)));
  }, [filtered.length]);
  useEffect(() => {
    listRef.current
      ?.querySelector('[aria-selected="true"]')
      ?.scrollIntoView({ block: 'nearest' });
  }, [selected, filtered]);

  const run = (cmd: PaletteCommand) => {
    onClose();
    cmd.perform();
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, filtered.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelected((s) => Math.max(s - 1, 0));
    } else if (e.key === 'Home' && !query) {
      e.preventDefault();
      setSelected(0);
    } else if (e.key === 'End' && !query) {
      e.preventDefault();
      setSelected(Math.max(0, filtered.length - 1));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const cmd = filtered[selected];
      if (cmd) run(cmd);
    }
  };

  const activeId = filtered[selected] ? `palette-opt-${filtered[selected]!.id}` : undefined;

  return (
    <ResponsiveDialog open={open} onClose={onClose} title={t('palette.title', 'Command palette')}>
      <div data-testid="command-palette" className="space-y-3">
        {/* First focusable in the dialog → ResponsiveDialog auto-focuses it. */}
        <input
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="command-palette-list"
          aria-activedescendant={activeId}
          aria-autocomplete="list"
          aria-label={t('palette.title', 'Command palette')}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={placeholder ?? t('palette.placeholder', 'Type a command or search…')}
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          className="w-full px-3 py-2.5 rounded-[var(--brand-radius-sm)] bg-[var(--brand-surface)] border border-[var(--brand-border)] text-sm text-[var(--brand-text)] placeholder:text-[var(--brand-text-muted)] focus:outline-none focus:ring-2 focus:ring-[var(--brand-primary)]"
        />
        {filtered.length === 0 ? (
          <div className="px-3 py-6 text-center text-sm text-[var(--brand-text-muted)]">
            {t('palette.empty', 'No matching commands')}
          </div>
        ) : (
          <ul
            id="command-palette-list"
            role="listbox"
            aria-label={t('palette.title', 'Command palette')}
            ref={listRef}
            className="max-h-[50vh] overflow-y-auto -mx-1 px-1 space-y-0.5"
          >
            {filtered.map((cmd, i) => (
              // eslint-disable-next-line jsx-a11y/click-events-have-key-events -- listbox option; keyboard nav is on the combobox input via aria-activedescendant, not per-option handlers
              <li
                key={cmd.id}
                id={`palette-opt-${cmd.id}`}
                role="option"
                aria-selected={i === selected}
                onMouseEnter={() => setSelected(i)}
                onClick={() => run(cmd)}
                className={`flex items-center gap-3 px-3 py-2.5 rounded-[var(--brand-radius-sm)] cursor-pointer text-sm transition-colors duration-[var(--motion-fast)] ${
                  i === selected
                    ? 'bg-[var(--brand-surface-raised)] text-[var(--brand-text)]'
                    : 'text-[var(--brand-text-muted)]'
                }`}
              >
                {cmd.icon && <i className={`${cmd.icon} text-step-lg shrink-0`} aria-hidden="true" />}
                <span className="truncate flex-1 text-[var(--brand-text)]">{cmd.label}</span>
                {cmd.hint && (
                  <span className="text-xs text-[var(--brand-text-muted)] truncate max-w-[8rem]">{cmd.hint}</span>
                )}
                {cmd.shortcut && (
                  <span className="flex items-center gap-1 shrink-0" aria-hidden="true">
                    {formatKeychord(cmd.shortcut, isMac).map((part, pi) => (
                      <kbd
                        key={pi}
                        className="min-w-[1.4rem] px-1.5 py-0.5 text-center text-step-2xs font-semibold rounded-md border border-[var(--brand-border)] bg-[var(--brand-surface)] text-[var(--brand-text-muted)]"
                      >
                        {part}
                      </kbd>
                    ))}
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </ResponsiveDialog>
  );
}
