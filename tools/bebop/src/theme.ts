// Bebop theme — the CLI's skin, harmonized to the Cowboy Bebop spaceship.
//
// Ground truth: the ship's main hull/signal color is the brand token `--teal #46B0A4`
// (calibrated in docs/design/dowiz-brand/BRAND-BIBLE.md §3 "Bebop teal — success / alive /
// data-signal"), sitting on the warm-noir field `--void #12100E` / `--hull #1A1E1F` with
// `--bone #F2E9DB` text. We reuse those EXACT hexes so the CLI and the product share one
// signal color. No new palette invented.
//
// Rendering: ANSI escapes only. We degrade to plain (no color) when stdout is not a TTY or
// NO_COLOR is set — never crash on a non-terminal pipe.

export const BEBOB = {
  // ship signal
  teal: '#46B0A4',
  tealDeep: '#3EA094',
  // warm-noir field
  void: '#12100E',
  hull: '#1A1E1F',
  bone: '#F2E9DB',
  // brand amber (secondary interactive, used sparingly per 90/10)
  amber: '#E8A544',
  // danger (paired with label/icon, never color-only — WCAG 1.4.1)
  blood: '#E0543E',
} as const;

const ESC = '\x1b[';
const C = {
  teal: `${ESC}38;2;70;176;164m`,
  tealDeep: `${ESC}38;2;62;160;148m`,
  void: `${ESC}38;2;18;16;14m`,
  hull: `${ESC}38;2;26;30;31m`,
  bone: `${ESC}38;2;242;233;219m`,
  amber: `${ESC}38;2;232;165;68m`,
  blood: `${ESC}38;2;224;84;62m`,
  dim: `${ESC}2m`,
  bold: `${ESC}1m`,
  reset: `${ESC}0m`,
} as const;

function colorEnabled(): boolean {
  return process.stdout.isTTY && !process.env.NO_COLOR;
}

export type Paint = (s: string) => string;

export function makePaint(): { [k: string]: Paint } {
  const on = colorEnabled();
  const wrap = (code: string): Paint => (s: string) =>
    on ? `${code}${s}${C.reset}` : s;
  return {
    teal: wrap(C.teal),
    tealDeep: wrap(C.tealDeep),
    void: wrap(C.void),
    hull: wrap(C.hull),
    bone: wrap(C.bone),
    amber: wrap(C.amber),
    blood: wrap(C.blood),
    dim: wrap(C.dim),
    bold: wrap(C.bold),
  };
}

// The ship mark — a small teal sigil used as the prompt glyph. One saturated accent, like the
// brand's "one meaningful color per view" law.
export const SHIP = '◈'; // ◈ — cold teal diamond, the machine's eye

export function banner(p: { [k: string]: Paint }): string {
  const line = `${p.teal(SHIP)} ${p.bold(p.bone('Bebop'))} ${p.dim('— your kitchen, your ship, your cut.')}`;
  return line;
}
