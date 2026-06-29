// G2 — produce parser outputs for each synthetic fixture, for the DeepEval scorer to grade.
// Runs the REAL AiOcrParser on each fixture's `ocr` text. Provider is whatever LLM_PROVIDER selects:
// in CI with an Anthropic key it exercises the live LLM path; locally (no key) it falls back to the
// deterministic heuristic structurer, so the harness is self-testable. Outputs land in outputs/<id>.json.
import { readdirSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { AiOcrParser } from '../../apps/api/src/lib/ai-ocr-parser.js';

const HERE = dirname(fileURLToPath(import.meta.url));
const FIX = join(HERE, 'fixtures');
const OUT = join(HERE, 'outputs');
mkdirSync(OUT, { recursive: true });

const parser = new AiOcrParser();
for (const f of readdirSync(FIX).filter((n) => n.endsWith('.json'))) {
  const fx = JSON.parse(readFileSync(join(FIX, f), 'utf8'));
  const res = await parser.parse({
    kind: 'text',
    bytes: Buffer.from(fx.ocr, 'utf8'),
    config: { expectedCurrency: 'ALL' } as any,
  });
  writeFileSync(join(OUT, `${fx.id}.json`), JSON.stringify({ id: fx.id, ocr: fx.ocr, expected: fx.expected, output: res.draft }, null, 2));
  console.log(`generated outputs/${fx.id}.json (${res.draft.products.length} products)`);
}
