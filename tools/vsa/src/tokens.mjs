// Token measurement — real BPE via js-tiktoken (cl100k_base) already present in the pnpm
// store; falls back to a labeled chars/3.6 estimate if the store moves. cl100k is not
// Anthropic's exact tokenizer, but it is a consistent, reproducible measuring stick — the
// BEFORE and AFTER are counted with the same ruler, so the RATIO is trustworthy.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..');

let encoder = null;
let method = 'approx(chars/3.6)';

async function loadEncoder() {
  if (encoder) return encoder;
  try {
    const pnpm = path.join(ROOT, 'node_modules', '.pnpm');
    const dir = fs.readdirSync(pnpm).find((d) => d.startsWith('js-tiktoken@'));
    if (dir) {
      const mod = await import(
        path.join(pnpm, dir, 'node_modules', 'js-tiktoken', 'dist', 'index.js')
      );
      encoder = mod.getEncoding('cl100k_base');
      method = 'js-tiktoken cl100k_base';
    }
  } catch {
    /* fall back to approx */
  }
  return encoder;
}

export async function countTokens(text) {
  const enc = await loadEncoder();
  return enc ? enc.encode(text).length : Math.ceil(text.length / 3.6);
}

export async function tokenMethod() {
  await loadEncoder();
  return method;
}
