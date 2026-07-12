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
  if (!existsSync(VENV_PY) || !existsSync(IMAGE)) {
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
    // … and the full seam produced a canonical draft.
    assert.ok(res.draft.products.length >= 1, 'expected a canonical draft with ≥1 product');
  } finally {
    process.env.LLM_PROVIDER = prevLlm;
    process.env.MENU_OCR_ENGINE = prevEng;
  }
});

test('PaddleOCR seam: engine selector routes to paddle (not tesseract)', async () => {
  const prevEng = process.env.MENU_OCR_ENGINE;
  process.env.MENU_OCR_ENGINE = 'paddle';
  process.env.PADDLE_OCR_PYTHON = '/nonexistent/python-binary';  // force the paddle path to fail
  try {
    const res = await new AiOcrParser().parse(imageInput());
    const err = res.issues.find((i: any) => /OCR Failed \(paddle\)/.test(i.message));
    assert.ok(err, 'selecting paddle must route to the PaddleOCR subprocess (error tagged "paddle")');
  } finally {
    process.env.MENU_OCR_ENGINE = prevEng;
    delete process.env.PADDLE_OCR_PYTHON;
  }
});
