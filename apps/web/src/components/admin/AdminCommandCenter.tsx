import { useMemo, useState } from 'react';
import { CommandPalette, ShortcutsHelp, useKeyboardShortcuts, useI18n } from '@deliveryos/ui';
import type { PaletteCommand, ShortcutDef, ShortcutHelpItem } from '@deliveryos/ui';

// AdminCommandCenter — power-user layer for the owner dashboard: the ⌘K/Ctrl+K
// command palette (fuzzy nav + safe quick-actions), "g"-sequences for section
// nav, and the "?" shortcuts help sheet. Mounted once in AdminLayout; purely
// additive (nav-only actions — no mutations, no API calls).
//
// Feature flag: VITE_KEYBOARD_SHORTCUTS_ENABLED — default ON; set to the
// literal string "false" at build time to disable the whole layer.
const KEYBOARD_SHORTCUTS_ENABLED = import.meta.env.VITE_KEYBOARD_SHORTCUTS_ENABLED !== 'false';

export interface AdminNavItem {
  key: string; // i18n key, e.g. 'admin.orders'
  href: string;
  icon: string;
}

// g-sequence per section (spec → href). Kept to the high-traffic five so the
// mnemonic space stays memorable; everything else is reachable via the palette.
const NAV_SHORTCUTS: Record<string, string> = {
  '/admin': 'g o', // orders (home)
  '/admin/menu': 'g m',
  '/admin/couriers': 'g c',
  '/admin/analytics': 'g a',
  '/admin/settings': 'g s',
};

interface AdminCommandCenterProps {
  navItems: AdminNavItem[];
  /** The layout's navigate helper (preserves the ?dev=true suffix). */
  navTo: (href: string) => void;
}

export function AdminCommandCenter({ navItems, navTo }: AdminCommandCenterProps) {
  const { t } = useI18n();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);

  const go = (href: string) => {
    setPaletteOpen(false);
    setHelpOpen(false);
    navTo(href);
  };

  const commands: PaletteCommand[] = useMemo(
    () => [
      ...navItems.map((item) => ({
        id: `nav:${item.href}`,
        label: t(item.key),
        icon: item.icon,
        hint: item.href,
        keywords: item.href.replace('/admin', '').replace('/', ' ') || 'orders',
        shortcut: NAV_SHORTCUTS[item.href],
        perform: () => go(item.href),
      })),
      // Safe quick-actions only (nav/local UI — "New menu item" has no URL
      // trigger today, so it stays out until MenuManager exposes one).
      {
        id: 'action:shortcuts-help',
        label: t('shortcuts.title', 'Keyboard shortcuts'),
        icon: 'ti ti-keyboard',
        shortcut: '?',
        perform: () => setHelpOpen(true),
      },
    ],
    [navItems, t, navTo],
  );

  const shortcuts: ShortcutDef[] = [
    {
      keys: 'mod+k',
      // ⌘K must work even while typing in a search box — it is not "typing".
      allowInEditable: true,
      description: t('shortcuts.open_palette', 'Open command palette'),
      onMatch: () => {
        setHelpOpen(false);
        setPaletteOpen((o) => !o);
      },
    },
    {
      keys: '?',
      description: t('shortcuts.show_help', 'Show keyboard shortcuts'),
      onMatch: () => {
        setPaletteOpen(false);
        setHelpOpen((o) => !o);
      },
    },
    ...navItems
      .filter((item) => NAV_SHORTCUTS[item.href])
      .map((item) => ({
        keys: NAV_SHORTCUTS[item.href]!,
        description: `${t('shortcuts.go_to', 'Go to')} ${t(item.key)}`,
        onMatch: () => go(item.href),
      })),
  ];

  useKeyboardShortcuts(shortcuts, { enabled: KEYBOARD_SHORTCUTS_ENABLED });

  const helpItems: ShortcutHelpItem[] = [
    ...shortcuts.map((s) => ({ keys: s.keys, label: s.description ?? s.keys })),
    // Escape is handled by every ResponsiveDialog, not registered here — but it
    // belongs on the sheet so the behavior is discoverable.
    { keys: 'escape', label: t('shortcuts.close_dialogs', 'Close dialogs') },
  ];

  if (!KEYBOARD_SHORTCUTS_ENABLED) return null;

  return (
    <>
      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} commands={commands} />
      <ShortcutsHelp open={helpOpen} onClose={() => setHelpOpen(false)} shortcuts={helpItems} />
    </>
  );
}
