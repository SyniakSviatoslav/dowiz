import test from 'node:test';
import assert from 'node:assert';
import { readFileSync, existsSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

// Stage A proof: PaddleOCR (PP-OCRv5/v6) wired behind MenuParserProvider as a
// config-selectable OCR engine (I5), Tesseract stays the default. These tests
// drive a REAL menu screenshot through AiOcrParser.parse() with the engine
// switched to 'paddle', proving the seam routes to the PaddleOCR subprocess.
//
// Run (from apps/api): node --test --import tsx tests/paddle-ocr-seam.test.ts
// (no package.json script — that file is in a governance-protected zone)
import { AiOcrParser } from '../src/lib/ai-ocr-parser.js';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, '..', '..', '..');                       // apps/api/tests -> repo root
const VENV_PY = join(REPO, '.venv-paddle', 'bin', 'python');
const SCRIPT = join(REPO, 'apps', 'api', 'scripts', 'paddle-ocr.py');
const IMAGE = join(REPO, 'e2e', 'artifacts', 'ssr-menu-page.png'); // real rendered menu

function imageInput(): any {
  return { kind: 'image', bytes: readFileSync(IMAGE), mime: 'image/png', config: { expectedCurrency: 'ALL', currencyMinorUnit: 2 } };
}

test('PaddleOCR seam: real menu image → canonical draft via paddle engine (mock LLM)', async (t) => {
  const missingDeps = !existsSync(VENV_PY) || !existsSync(IMAGE);
  // CI gate: when the seam is declared required (PADDLE_REQUIRED=1), a missing venv/image
  // must FAIL — a skipped test is a false-green that hides a broken PaddleOCR seam.
  if (missingDeps && process.env.PADDLE_REQUIRED === '1') {
    assert.fail(`PADDLE_REQUIRED=1 but paddle venv (${VENV_PY}) or test image (${IMAGE}) is absent — the seam cannot be proven`);
  }
  if (missingDeps) {
    t.skip(`missing paddle venv (${VENV_PY}) or test image — install Stage A deps first`);
    return;
  }
  const prevLlm = process.env.LLM_PROVIDER;
  const prevEng = process.env.MENU_OCR_ENGINE;
  process.env.LLM_PROVIDER = 'mock';            // structuring is the existing pipeline
  process.env.MENU_OCR_ENGINE = 'paddle';        // <-- engine swap under test (I5)
  process.env.PADDLE_OCR_PYTHON = VENV_PY;
  process.env.PADDLE_OCR_SCRIPT = SCRIPT;

  try {
    const res = await new AiOcrParser().parse(imageInput());
    // OCR step ran through PaddleOCR without error → no PARSE_ERROR issue …
    const ocrFailed = res.issues.find((i: any) => i.code === 'PARSE_ERROR');
    assert.equal(ocrFailed, undefined, `paddle OCR step should not error: ${ocrFailed?.message}`);
    // … and the full seam produced a canonical draft with a real, usable product
    // (a non-empty name AND a positive price — not garbled/coerced filler).
    assert.ok(res.draft.products.length >= 1, 'expected a canonical draft with ≥1 product');
    const first = res.draft.products[0] as any;
    assert.ok(typeof first.name === 'string' && first.name.trim().length > 0, 'product must have a non-empty name');
    assert.ok(typeof first.price === 'number' && first.price > 0, 'product must have a positive price');
  } finally {
    process.env.LLM_PROVIDER = prevLlm;
    process.env.MENU_OCR_ENGINE = prevEng;
  }
});

test('PaddleOCR seam: engine selector routes to paddle (not tesseract)', async () => {
  const prevEng = process.env.MENU_OCR_ENGINE;
  process.env.MENU_OCR_ENGINE = 'paddle';
  process.env.PADDLE_OCR_PYTHON = '/nonexistent/python-binary';  // force the paddle path to fail
  // Tiny inline PNG-magic buffer: routing fails at the (nonexistent) python binary, so the
  // bytes are never decoded — no real image file (or unguarded readFileSync) is needed here.
  const tinyImage: any = { kind: 'image', bytes: Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), mime: 'image/png', config: { expectedCurrency: 'ALL', currencyMinorUnit: 2 } };
  try {
    const res = await new AiOcrParser().parse(tinyImage);
    const err = res.issues.find((i: any) => /OCR Failed \(paddle\)/.test(i.message));
    assert.ok(err, 'selecting paddle must route to the PaddleOCR subprocess (error tagged "paddle")');
    // Negative control: a silent reroute to the default Tesseract engine would NOT carry the
    // "paddle" tag and would leave no tesseract trace either — assert no fallback happened.
    assert.ok(!res.issues.some((i: any) => /tesseract/i.test(i.message)), 'paddle selection must not silently fall back to tesseract');
  } finally {
    process.env.MENU_OCR_ENGINE = prevEng;
    delete process.env.PADDLE_OCR_PYTHON;
  }
});
