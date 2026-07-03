import { useEffect, useRef } from 'react';

// use-keyboard-shortcuts — global power-user shortcuts for the admin shell.
//
// Two layers:
//   1. A PURE keychord matcher (parse/match/sequence/format) with zero DOM
//      dependencies — unit-testable under `tsx --test` (node:test, no jsdom).
//   2. A thin React hook that wires the matcher to `window` keydown, with
//      sequence support ("g o"), a reset timeout, and editable-target guards.
//
// Spec grammar: chords are `+`-joined tokens ("mod+k", "shift+/", "?"), a
// sequence is space-separated chords ("g o"). `mod` is the PLATFORM modifier:
// ⌘ on macOS/iOS, Ctrl everywhere else.

export interface KeyChord {
  /** Normalized `event.key` to match (lowercased; 'esc' → 'escape'). */
  key: string;
  ctrl: boolean;
  meta: boolean;
  alt: boolean;
  shift: boolean;
  /** Platform modifier: resolves to meta on mac, ctrl elsewhere. */
  mod: boolean;
}

/** The minimal slice of KeyboardEvent the pure matcher needs. */
export interface ChordEventLike {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

type ModField = 'ctrl' | 'meta' | 'alt' | 'shift' | 'mod';

const MOD_TOKENS: Record<string, ModField> = {
  mod: 'mod',
  cmd: 'meta',
  command: 'meta',
  meta: 'meta',
  win: 'meta',
  ctrl: 'ctrl',
  control: 'ctrl',
  alt: 'alt',
  option: 'alt',
  shift: 'shift',
};

const KEY_ALIASES: Record<string, string> = {
  esc: 'escape',
  return: 'enter',
  space: ' ',
  spacebar: ' ',
  up: 'arrowup',
  down: 'arrowdown',
  left: 'arrowleft',
  right: 'arrowright',
};

function normalizeKey(key: string): string {
  const k = key.toLowerCase();
  return KEY_ALIASES[k] ?? k;
}

/** Parse a single chord spec like "mod+k", "shift+/", "g" or "?". */
export function parseKeychord(spec: string): KeyChord {
  const chord: KeyChord = { key: '', ctrl: false, meta: false, alt: false, shift: false, mod: false };
  const tokens = spec.trim().split('+').filter(Boolean);
  for (let i = 0; i < tokens.length; i++) {
    const raw = tokens[i]!.toLowerCase();
    const mod = MOD_TOKENS[raw];
    // A modifier name in the LAST position is the key itself (e.g. spec "shift").
    if (mod && i < tokens.length - 1) chord[mod] = true;
    else chord.key = normalizeKey(raw);
  }
  return chord;
}

/** Parse a spec into a chord sequence: "g o" → [g, o]; "mod+k" → [mod+k]. */
export function parseSequence(spec: string): KeyChord[] {
  return spec.trim().split(/\s+/).filter(Boolean).map(parseKeychord);
}

/**
 * Does a keydown event satisfy a chord? Modifiers must match EXACTLY (a plain
 * "g" chord never fires on Ctrl+G), with one deliberate exception: for shifted
 * characters like "?" the shift state is ignored — shift is what PRODUCES the
 * character, so requiring shift:false would make "?" unmatchable.
 */
export function chordMatches(ev: ChordEventLike, chord: KeyChord, isMac: boolean): boolean {
  if (!chord.key || normalizeKey(ev.key) !== chord.key) return false;
  const wantMeta = chord.meta || (chord.mod && isMac);
  const wantCtrl = chord.ctrl || (chord.mod && !isMac);
  if (!!ev.metaKey !== wantMeta) return false;
  if (!!ev.ctrlKey !== wantCtrl) return false;
  if (!!ev.altKey !== chord.alt) return false;
  const shiftAgnostic = chord.key.length === 1 && !/^[a-z0-9]$/.test(chord.key);
  if (!shiftAgnostic && !!ev.shiftKey !== chord.shift) return false;
  return true;
}

/**
 * Advance a sequence state machine by one keydown. `progress` is how many
 * chords of `seq` already matched. Returns the new progress and whether the
 * full sequence just completed (progress resets to 0 on completion). A
 * mismatching key re-anchors on the first chord ("g g o" still ends in "g o")
 * or resets to 0.
 */
export function advanceSequence(
  progress: number,
  ev: ChordEventLike,
  seq: KeyChord[],
  isMac: boolean,
): { progress: number; matched: boolean } {
  if (seq.length === 0) return { progress: 0, matched: false };
  const at = progress >= 0 && progress < seq.length ? progress : 0;
  if (chordMatches(ev, seq[at]!, isMac)) {
    const next = at + 1;
    if (next === seq.length) return { progress: 0, matched: true };
    return { progress: next, matched: false };
  }
  if (at > 0 && chordMatches(ev, seq[0]!, isMac)) return { progress: 1, matched: false };
  return { progress: 0, matched: false };
}

/** The minimal slice of an event target the editable-guard needs. */
export interface EditableTargetLike {
  tagName?: string;
  isContentEditable?: boolean;
  type?: string;
}

// Input types where keystrokes are NOT text entry — shortcuts stay live there
// so keyboard users focused on a checkbox/button don't lose navigation.
const NON_TEXT_INPUT_TYPES = new Set([
  'button', 'checkbox', 'radio', 'range', 'file', 'submit', 'reset', 'color', 'image',
]);

/** True when the event target is a text-entry surface (typing must win). */
export function isEditableTarget(target: EditableTargetLike | null | undefined): boolean {
  if (!target) return false;
  if (target.isContentEditable) return true;
  const tag = (target.tagName ?? '').toUpperCase();
  if (tag === 'TEXTAREA' || tag === 'SELECT') return true;
  if (tag === 'INPUT') return !NON_TEXT_INPUT_TYPES.has((target.type ?? 'text').toLowerCase());
  return false;
}

/** Platform sniff for `mod` resolution + display formatting. Injectable for tests. */
export function isMacPlatform(nav?: { platform?: string; userAgent?: string } | null): boolean {
  const n = nav ?? (typeof navigator !== 'undefined' ? navigator : undefined);
  if (!n) return false;
  return /mac|iphone|ipad|ipod/i.test(n.platform ?? '') || /mac os x|macintosh/i.test(n.userAgent ?? '');
}

const KEY_LABELS: Record<string, string> = {
  escape: 'Esc',
  enter: '↵',
  ' ': 'Space',
  arrowup: '↑',
  arrowdown: '↓',
  arrowleft: '←',
  arrowright: '→',
  backspace: '⌫',
  tab: 'Tab',
};

function chordLabel(chord: KeyChord, isMac: boolean): string {
  const parts: string[] = [];
  if (chord.mod) parts.push(isMac ? '⌘' : 'Ctrl');
  if (chord.ctrl) parts.push(isMac ? '⌃' : 'Ctrl');
  if (chord.alt) parts.push(isMac ? '⌥' : 'Alt');
  if (chord.shift) parts.push(isMac ? '⇧' : 'Shift');
  if (chord.meta && !chord.mod) parts.push(isMac ? '⌘' : 'Win');
  parts.push(
    KEY_LABELS[chord.key] ?? (chord.key.length === 1 ? chord.key.toUpperCase() : chord.key.charAt(0).toUpperCase() + chord.key.slice(1)),
  );
  return isMac ? parts.join('') : parts.join('+');
}

/** Display labels for a spec, one entry per sequence step: "g o" → ['G','O']. */
export function formatKeychord(spec: string, isMac: boolean): string[] {
  return parseSequence(spec).map((c) => chordLabel(c, isMac));
}

export interface ShortcutDef {
  /** Keychord spec: "mod+k", "?", or a sequence like "g o". */
  keys: string;
  onMatch: (event: KeyboardEvent) => void;
  /**
   * Fire even when focus is inside a text-entry target. Without this, only
   * true modifier chords (Ctrl/⌘) can fire there — plain typing keys never do.
   */
  allowInEditable?: boolean;
  /** Human label for the shortcuts help sheet. */
  description?: string;
  /** Per-shortcut kill switch (default true). */
  enabled?: boolean;
}

export interface UseKeyboardShortcutsOptions {
  /** Master switch (feature flag) — false unbinds the listener entirely. */
  enabled?: boolean;
  /** How long a partial sequence ("g" …) stays alive. Default 1000ms. */
  sequenceTimeoutMs?: number;
}

/**
 * Register global keyboard shortcuts on `window`. Cleans up on unmount.
 * - Sequences ("g o") reset after `sequenceTimeoutMs` of inactivity.
 * - Text-entry targets (input/textarea/select/contenteditable) swallow plain
 *   keys; modifier chords still fire (⌘K works from a search box).
 * - `preventDefault` is called ONLY on a full match — partial sequence keys
 *   and unmatched keys are never hijacked.
 */
export function useKeyboardShortcuts(
  shortcuts: ShortcutDef[],
  options: UseKeyboardShortcutsOptions = {},
): void {
  const { enabled = true, sequenceTimeoutMs = 1000 } = options;
  // Refs so consumers can pass inline arrays/closures without re-binding.
  const shortcutsRef = useRef(shortcuts);
  shortcutsRef.current = shortcuts;
  const progressRef = useRef<number[]>([]);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!enabled || typeof window === 'undefined') return;
    const isMac = isMacPlatform();
    const resetProgress = () => { progressRef.current = []; };

    const handler = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.isComposing || event.repeat) return;
      // Bare modifier presses never advance anything.
      if (event.key === 'Shift' || event.key === 'Control' || event.key === 'Alt' || event.key === 'Meta') return;

      const defs = shortcutsRef.current;
      if (progressRef.current.length !== defs.length) progressRef.current = defs.map(() => 0);
      const inEditable = isEditableTarget(event.target as EditableTargetLike | null);

      let anyPartial = false;
      for (let i = 0; i < defs.length; i++) {
        const def = defs[i]!;
        if (def.enabled === false) { progressRef.current[i] = 0; continue; }
        // Typing wins: in a text-entry target, only opt-in shortcuts or real
        // modifier chords (Ctrl/⌘ held) are considered.
        if (inEditable && !def.allowInEditable && !(event.ctrlKey || event.metaKey)) {
          progressRef.current[i] = 0;
          continue;
        }
        const seq = parseSequence(def.keys);
        const { progress, matched } = advanceSequence(progressRef.current[i] ?? 0, event, seq, isMac);
        progressRef.current[i] = progress;
        if (matched) {
          event.preventDefault();
          resetProgress();
          def.onMatch(event);
          return;
        }
        if (progress > 0) anyPartial = true;
      }

      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = anyPartial ? setTimeout(resetProgress, sequenceTimeoutMs) : null;
    };

    window.addEventListener('keydown', handler);
    return () => {
      window.removeEventListener('keydown', handler);
      if (timerRef.current) clearTimeout(timerRef.current);
      resetProgress();
    };
  }, [enabled, sequenceTimeoutMs]);
}
