import test from 'node:test';
import assert from 'node:assert/strict';
import {
  parseKeychord,
  parseSequence,
  chordMatches,
  advanceSequence,
  isEditableTarget,
  formatKeychord,
  type ChordEventLike,
} from '../use-keyboard-shortcuts.js';

// Minimal keyboard-event stand-in — the matcher is pure and never touches the DOM.
const ev = (key: string, mods: Partial<Omit<ChordEventLike, 'key'>> = {}): ChordEventLike => ({
  key,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

test('parse + match: single plain key ("g")', () => {
  const chord = parseKeychord('g');
  assert.equal(chord.key, 'g');
  assert.equal(chord.mod, false);
  assert.equal(chordMatches(ev('g'), chord, false), true);
  // Uppercase event key normalizes, but Shift+G is NOT a plain "g".
  assert.equal(chordMatches(ev('G', { shiftKey: true }), chord, false), false);
  // A modifier turns it into a different chord — must not fire.
  assert.equal(chordMatches(ev('g', { ctrlKey: true }), chord, false), false);
  assert.equal(chordMatches(ev('x'), chord, false), false);
});

test('parse + match: platform modifier chord ("mod+k")', () => {
  const chord = parseKeychord('mod+k');
  assert.equal(chord.key, 'k');
  assert.equal(chord.mod, true);
  // mac → ⌘K, and Ctrl+K must NOT fire
  assert.equal(chordMatches(ev('k', { metaKey: true }), chord, true), true);
  assert.equal(chordMatches(ev('k', { ctrlKey: true }), chord, true), false);
  // non-mac → Ctrl+K, and ⌘K must NOT fire
  assert.equal(chordMatches(ev('k', { ctrlKey: true }), chord, false), true);
  assert.equal(chordMatches(ev('k', { metaKey: true }), chord, false), false);
  // bare "k" without the modifier never fires
  assert.equal(chordMatches(ev('k'), chord, false), false);
  // event key case-insensitivity
  assert.equal(chordMatches(ev('K', { ctrlKey: true }), chord, false), true);
});

test('match: "?" is shift-agnostic (shift is what produces the character)', () => {
  const chord = parseKeychord('?');
  assert.equal(chordMatches(ev('?', { shiftKey: true }), chord, false), true);
  assert.equal(chordMatches(ev('?'), chord, false), true);
  // but ctrl+? is a different chord
  assert.equal(chordMatches(ev('?', { shiftKey: true, ctrlKey: true }), chord, false), false);
});

test('parse + match: explicit modifier tokens ("shift+/", "ctrl+alt+d")', () => {
  const slash = parseKeychord('shift+/');
  assert.equal(slash.key, '/');
  assert.equal(slash.shift, true);
  const cad = parseKeychord('ctrl+alt+d');
  assert.equal(cad.ctrl, true);
  assert.equal(cad.alt, true);
  assert.equal(cad.key, 'd');
  assert.equal(chordMatches(ev('d', { ctrlKey: true, altKey: true }), cad, false), true);
  assert.equal(chordMatches(ev('d', { ctrlKey: true }), cad, false), false);
});

test('sequence: "g o" advances, completes, and resets on a stray key', () => {
  const seq = parseSequence('g o');
  assert.equal(seq.length, 2);

  // g → partial, o → match
  let r = advanceSequence(0, ev('g'), seq, false);
  assert.deepEqual(r, { progress: 1, matched: false });
  r = advanceSequence(r.progress, ev('o'), seq, false);
  assert.equal(r.matched, true);
  assert.equal(r.progress, 0, 'a completed sequence resets its progress');

  // g → x resets to 0 (no match)
  r = advanceSequence(0, ev('g'), seq, false);
  r = advanceSequence(r.progress, ev('x'), seq, false);
  assert.deepEqual(r, { progress: 0, matched: false });

  // g → g re-anchors on the first chord ("gg o" still ends with a clean "g o")
  r = advanceSequence(1, ev('g'), seq, false);
  assert.deepEqual(r, { progress: 1, matched: false });

  // "o" alone must never complete the sequence
  r = advanceSequence(0, ev('o'), seq, false);
  assert.deepEqual(r, { progress: 0, matched: false });
});

test('sequence: single-chord spec is a length-1 sequence ("mod+k")', () => {
  const seq = parseSequence('mod+k');
  assert.equal(seq.length, 1);
  const r = advanceSequence(0, ev('k', { ctrlKey: true }), seq, false);
  assert.equal(r.matched, true);
});

test('ignore-in-input: text-entry targets are editable, buttons/checkboxes are not', () => {
  assert.equal(isEditableTarget({ tagName: 'INPUT', type: 'text' }), true);
  assert.equal(isEditableTarget({ tagName: 'INPUT' }), true, 'input with no type defaults to text');
  assert.equal(isEditableTarget({ tagName: 'input', type: 'search' }), true);
  assert.equal(isEditableTarget({ tagName: 'TEXTAREA' }), true);
  assert.equal(isEditableTarget({ tagName: 'SELECT' }), true);
  assert.equal(isEditableTarget({ tagName: 'DIV', isContentEditable: true }), true);
  // non-text-entry targets keep shortcuts live
  assert.equal(isEditableTarget({ tagName: 'INPUT', type: 'checkbox' }), false);
  assert.equal(isEditableTarget({ tagName: 'INPUT', type: 'radio' }), false);
  assert.equal(isEditableTarget({ tagName: 'BUTTON' }), false);
  assert.equal(isEditableTarget({ tagName: 'DIV' }), false);
  assert.equal(isEditableTarget(null), false);
  assert.equal(isEditableTarget(undefined), false);
});

test('formatKeychord: platform-aware display labels', () => {
  assert.deepEqual(formatKeychord('mod+k', true), ['⌘K']);
  assert.deepEqual(formatKeychord('mod+k', false), ['Ctrl+K']);
  assert.deepEqual(formatKeychord('g o', false), ['G', 'O']);
  assert.deepEqual(formatKeychord('?', false), ['?']);
  assert.deepEqual(formatKeychord('escape', false), ['Esc']);
});
