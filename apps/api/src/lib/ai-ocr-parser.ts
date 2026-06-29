import type { MenuParserProvider, ParserInputType } from '../ports.js';
// @ts-ignore — legacy types removed from shared-types, used by OCR parser at runtime
import type { CanonicalMenuDraft, ParseIssue, ParseResult } from '@deliveryos/shared-types';
import { PiiRedactor } from './pii-redactor.js';
import { extractMenuRegion, scanResidualPii } from './menu-region.js';
import { collectOcrPriceMinors, computeGrounding, extractTrailingPriceMinor } from './menu-grounding.js';
import Tesseract from 'tesseract.js';
import crypto from 'crypto';
import { z } from 'zod';
import { execFileSync } from 'child_process';
import { writeFileSync, unlinkSync, existsSync } from 'fs';
import { tmpdir } from 'os';
import { join } from 'path';

function coerceNum() {
  return z.preprocess((v) => {
    if (typeof v === 'string') { const n = Number(v); return isNaN(n) ? v : n; }
    return v;
  }, z.number());
}
function coerceStr() {
  return z.preprocess((v) => typeof v === 'number' ? String(v) : v, z.string());
}
function coerceBool() {
  return z.preprocess((v) => {
    if (typeof v === 'string') return v === 'true' || v === '1';
    if (typeof v === 'number') return v !== 0;
    return v;
  }, z.boolean());
}

const llmSchema = z.object({
  // Restaurant-level contact extracted from the menu header/footer (business
  // contact info, NOT customer PII). Optional — older prompts won't return it.
  restaurant: z.object({
    name: coerceStr().optional().default(''),
    address: coerceStr().optional().default(''),
    phone: coerceStr().optional().default(''),
    hoursText: coerceStr().optional().default(''),
  }).optional(),
  categories: z.array(z.object({
    externalKey: coerceStr(),
    name: z.string()
  })),
  products: z.array(z.object({
    externalKey: coerceStr(),
    categoryKey: coerceStr(),
    name: z.string(),
    description: z.string().nullable().optional(),
    price: coerceNum(),
    currency: z.string(),
    available: coerceBool(),
    attributesJson: z.record(z.unknown()).optional(),
    imageKey: z.string().nullable().optional()
  })),
  modifierGroups: z.array(z.object({
    externalKey: coerceStr(),
    name: z.string(),
    minSelect: coerceNum(),
    maxSelect: coerceNum(),
    required: coerceBool()
  })).optional().default([]),
  modifiers: z.array(z.object({
    externalKey: coerceStr(),
    groupKey: coerceStr(),
    name: z.string(),
    priceDelta: coerceNum(),
    available: coerceBool(),
    sortOrder: coerceNum().optional()
  })).optional().default([]),
  links: z.array(z.object({
    productKey: coerceStr(),
    groupKey: coerceStr(),
    sortOrder: coerceNum()
  })).optional().default([])
});

// ── LLM Adapter ─────────────────────────────────────────────────────
// 'heuristic' is the zero-dependency, no-API-key structurer (pure code) used
// when no LLM provider is configured — so menu import works on a server with
// nothing wired up (open-source, free). A real LLM, when configured, still wins.
type LlmProvider = 'ollama' | 'groq' | 'openai' | 'openrouter' | 'zen' | 'mock' | 'heuristic';

function detectProvider(model: string): LlmProvider {
  if (model === 'mock') return 'mock';
  const env = (process.env.LLM_ADAPTER || process.env.LLM_PROVIDER || '').toLowerCase();
  if (env === 'heuristic' || env === 'none' || env === 'offline') return 'heuristic';
  if (env === 'zen' || env === 'opencode') return 'zen';
  if (env === 'groq') return 'groq';
  if (env === 'openai') return 'openai';
  if (env === 'openrouter') return 'openrouter';
  if (env === 'ollama') return 'ollama';
  // OpenCode Zen is preferred when configured: OpenAI-wire compatible, free models, and the
  // fallback for the OpenRouter account running out of credits. Detected before the others.
  if (process.env.OPENCODE_ZEN_API_KEY) return 'zen';
  if (process.env.GROQ_API_KEY) return 'groq';
  if (process.env.OPENAI_API_KEY) return 'openai';
  // OpenRouter is the configured provider in prod (OPENROUTER_API_KEY). It is OpenAI-wire
  // compatible, so it reuses the chat/completions shape. Without this branch the key was
  // ignored and import silently degraded to the heuristic structurer.
  if (process.env.OPENROUTER_API_KEY) return 'openrouter';
  // An explicit ollama endpoint/model means "ollama configured" → honour it
  // (and surface its failure). With nothing configured at all, fall back to the
  // heuristic structurer instead of an ollama endpoint that will never answer.
  if (process.env.LLM_ENDPOINT || process.env.LLM_PROVIDER) return 'ollama';
  return 'heuristic';
}

async function callLlm(provider: LlmProvider, prompt: string, model: string, timeoutMs: number): Promise<string> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);

  try {
    if (provider === 'groq') {
      const apiKey = process.env.GROQ_API_KEY || '';
      const endpoint = process.env.GROQ_ENDPOINT || 'https://api.groq.com/openai/v1/chat/completions';
      const res = await fetch(endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiKey}`,
        },
        body: JSON.stringify({
          model,
          messages: [
            { role: 'system', content: 'You are a menu data extraction assistant. Return ONLY valid JSON matching the requested schema. No markdown, no commentary.' },
            { role: 'user', content: prompt },
          ],
          temperature: 0.1,
          response_format: { type: 'json_object' },
        }),
        signal: controller.signal,
      });
      if (!res.ok) {
        const errBody = await res.text().catch(() => '');
        throw new Error(`Groq ${res.status}: ${errBody.slice(0, 200)}`);
      }
      const data = await res.json() as any;
      const content = data?.choices?.[0]?.message?.content;
      if (!content) throw new Error('Groq returned empty content');
      return content;

    } else if (provider === 'openai') {
      const apiKey = process.env.OPENAI_API_KEY || '';
      const endpoint = process.env.OPENAI_ENDPOINT || 'https://api.openai.com/v1/chat/completions';
      const res = await fetch(endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiKey}`,
        },
        body: JSON.stringify({
          model,
          messages: [
            { role: 'system', content: 'You are a menu data extraction assistant. Return ONLY valid JSON matching the requested schema. No markdown, no commentary.' },
            { role: 'user', content: prompt },
          ],
          temperature: 0.1,
          response_format: { type: 'json_object' },
        }),
        signal: controller.signal,
      });
      if (!res.ok) {
        const errBody = await res.text().catch(() => '');
        throw new Error(`OpenAI ${res.status}: ${errBody.slice(0, 200)}`);
      }
      const data = await res.json() as any;
      const content = data?.choices?.[0]?.message?.content;
      if (!content) throw new Error('OpenAI returned empty content');
      return content;

    } else if (provider === 'openrouter') {
      // OpenRouter — OpenAI-wire compatible aggregator. Same chat/completions shape.
      const apiKey = process.env.OPENROUTER_API_KEY || '';
      const endpoint = process.env.OPENROUTER_ENDPOINT || 'https://openrouter.ai/api/v1/chat/completions';
      const res = await fetch(endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiKey}`,
          // OpenRouter attribution headers (optional but recommended).
          'HTTP-Referer': process.env.APP_BASE_URL || 'https://dowiz.fly.dev',
          'X-Title': 'dowiz menu import',
        },
        body: JSON.stringify({
          model,
          messages: [
            { role: 'system', content: 'You are a menu data extraction assistant. Return ONLY valid JSON matching the requested schema. No markdown, no commentary.' },
            { role: 'user', content: prompt },
          ],
          temperature: 0.1,
          response_format: { type: 'json_object' },
        }),
        signal: controller.signal,
      });
      if (!res.ok) {
        const errBody = await res.text().catch(() => '');
        throw new Error(`OpenRouter ${res.status}: ${errBody.slice(0, 200)}`);
      }
      const data = await res.json() as any;
      const content = data?.choices?.[0]?.message?.content;
      if (!content) throw new Error('OpenRouter returned empty content');
      return content;

    } else if (provider === 'zen') {
      // OpenCode Zen — OpenAI-wire compatible gateway with free models. A free model can have its
      // free promotion end (returns 4xx), so try the configured model then a chain of other live
      // free models before failing — import keeps working without a redeploy. All share the
      // standard chat/completions shape; the local OCR step already produced the text.
      const apiKey = process.env.OPENCODE_ZEN_API_KEY || '';
      const endpoint = process.env.OPENCODE_ZEN_ENDPOINT || 'https://opencode.ai/zen/v1/chat/completions';
      const candidates = [...new Set([model, 'deepseek-v4-flash-free', 'nemotron-3-ultra-free', 'mimo-v2.5-free'])];
      let lastErr = '';
      for (const m of candidates) {
        const res = await fetch(endpoint, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${apiKey}`,
          },
          body: JSON.stringify({
            model: m,
            messages: [
              { role: 'system', content: 'You are a menu data extraction assistant. Return ONLY valid JSON matching the requested schema. No markdown, no commentary.' },
              { role: 'user', content: prompt },
            ],
            temperature: 0.1,
            response_format: { type: 'json_object' },
          }),
          signal: controller.signal,
        });
        if (!res.ok) { lastErr = `${m} → ${res.status}: ${(await res.text().catch(() => '')).slice(0, 160)}`; continue; }
        const data = await res.json() as any;
        const content = data?.choices?.[0]?.message?.content;
        if (content) return content;
        lastErr = `${m} → empty content`;
      }
      throw new Error(`OpenCode Zen failed (tried ${candidates.join(', ')}): ${lastErr}`);

    } else {
      // Ollama
      const endpoint = process.env.LLM_ENDPOINT || 'http://localhost:11434/api/generate';
      const res = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model,
          prompt,
          stream: false,
          format: 'json',
        }),
        signal: controller.signal,
      });
      if (!res.ok) throw new Error(`Ollama ${res.status}: ${res.statusText}`);
      const data = await res.json() as any;
      return data.response;
    }
  } finally {
    clearTimeout(timer);
  }
}

// ── LLM response cache (token economy, ADR-0012) ──────────────────────────────
// The menu parse is near-deterministic (temperature 0.1) and identical inputs recur: a re-import of the
// same PDF, a retry after a transient failure, the same scraped page across runs. Cache the VALIDATED
// response keyed by (provider | model | full prompt) so an identical parse costs ZERO LLM tokens. The
// key includes the prompt verbatim — schema text, memory examples, AND the redacted source — so any
// change misses (never a stale parse). Per-instance, bounded (FIFO-evicted at the cap) and TTL'd.
const LLM_CACHE_MAX = 200;
const LLM_CACHE_TTL_MS = 24 * 60 * 60 * 1000; // 24h
const llmCache = new Map<string, { value: string; expires: number }>();
export const LLM_CACHE_CAP = LLM_CACHE_MAX; // exported for the guardrail test
export function llmCacheKeyOf(provider: string, model: string, prompt: string): string {
  return crypto.createHash('sha256').update(`${provider} ${model} ${prompt}`).digest('hex');
}
export function llmCacheGet(key: string): string | undefined {
  const e = llmCache.get(key);
  if (!e) return undefined;
  if (e.expires < Date.now()) { llmCache.delete(key); return undefined; }
  return e.value;
}
export function llmCacheSet(key: string, value: string): void {
  if (llmCache.size >= LLM_CACHE_MAX && !llmCache.has(key)) {
    const oldest = llmCache.keys().next().value;
    if (oldest !== undefined) llmCache.delete(oldest);
  }
  llmCache.set(key, { value, expires: Date.now() + LLM_CACHE_TTL_MS });
}

export class AiOcrParser implements MenuParserProvider {
  readonly id = 'ai-ocr';
  private piiRedactor = new PiiRedactor();
  private memoryService: any;

  constructor(memoryService: any = null) {
    this.memoryService = memoryService;
  }

  /**
   * On-demand PaddleOCR (PP-OCRv5/v6) via a Python subprocess. Engine swap is
   * config only (I5), selected by config.ocr_engine or MENU_OCR_ENGINE. NOT a
   * daemon (I4): one process per image, then it exits. Menu-image content only,
   * no PII / product-runtime state (I1). Returns text + a 0..1 confidence.
   */
  private paddleOcr(bytes: Buffer, mime: string): { text: string; confidence: number; version: string } {
    const py = process.env.PADDLE_OCR_PYTHON || 'python3';
    const script = process.env.PADDLE_OCR_SCRIPT || join(process.cwd(), 'scripts', 'paddle-ocr.py');
    if (!existsSync(script)) throw new Error(`paddle-ocr.py not found at ${script} (set PADDLE_OCR_SCRIPT)`);
    const ext = mime === 'image/png' ? 'png' : mime === 'image/webp' ? 'webp' : 'jpg';
    const tmp = join(tmpdir(), `dowiz-ocr-${crypto.randomBytes(6).toString('hex')}.${ext}`);
    writeFileSync(tmp, bytes);
    try {
      const out = execFileSync(py, [script, tmp], {
        timeout: 120_000,
        maxBuffer: 16 * 1024 * 1024,
        stdio: ['ignore', 'pipe', 'ignore'],
      }).toString();
      const j = JSON.parse(out);
      if (j.error) throw new Error(j.error);
      return {
        text: j.text || '',
        confidence: typeof j.confidence === 'number' ? j.confidence : 0,
        version: j.version || 'paddleocr',
      };
    } finally {
      try { unlinkSync(tmp); } catch { /* best-effort temp cleanup */ }
    }
  }

  async parse(input: ParserInputType): Promise<ParseResult> {
    const issues: ParseIssue[] = [];
    const t0 = Date.now();

    let rawText = '';
    let ocrConfidence = 1;
    let ocrMs = 0;
    let ocrEngineUsed = 'tesseract.js';

    if (input.kind === 'pdf') {
      // Extract text directly from PDF, skip OCR
      try {
        // Pre-load worker module onto globalThis to prevent pdfjs-dist from
        // doing its own dynamic import("./pdf.worker.mjs") which fails in
        // esbuild-bundled environments (worker file is not on disk at runtime).
        // @ts-expect-error — pdfjs worker lacks types
        const workerMod = await import('pdfjs-dist/build/pdf.worker.mjs');
        (globalThis as any).pdfjsWorker = workerMod;
        const pdfjs = await import('pdfjs-dist');
        const doc = await pdfjs.getDocument({ data: new Uint8Array(input.bytes) }).promise;
        const pages: string[] = [];
        for (let i = 1; i <= doc.numPages; i++) {
          const page = await doc.getPage(i);
          const content = await page.getTextContent();
          // Reconstruct visual lines: PDF text comes as positioned fragments, so
          // join by row (y-coordinate) instead of flattening everything to one
          // line — otherwise every menu item collapses into a single string.
          const pageLines: string[] = [];
          let line = '';
          let lastY: number | null = null;
          for (const item of content.items as any[]) {
            if (typeof item.str !== 'string') continue;
            const y = item.transform?.[5];
            if (lastY !== null && typeof y === 'number' && Math.abs(y - lastY) > 2 && line) {
              pageLines.push(line);
              line = '';
            }
            line += (line && !line.endsWith(' ') && item.str ? ' ' : '') + item.str;
            if (item.hasEOL && line) { pageLines.push(line); line = ''; }
            if (typeof y === 'number') lastY = y;
          }
          if (line) pageLines.push(line);
          pages.push(pageLines.join('\n'));
        }
        await doc.destroy();
        rawText = pages.join('\n\n');
      } catch (e: any) {
        issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `PDF parse failed: ${e.message}`, severity: 'error' });
        return this.fallbackError(issues);
      }
      ocrMs = Date.now() - t0;

      if (!rawText.trim()) {
        // Image-only / scanned PDF (no text layer). We deliberately don't rasterise+OCR PDFs
        // server-side (would need a heavy native canvas dep + unbounded per-page render). The
        // image upload path DOES OCR, so route the user there instead of a dead end.
        issues.push({
          rowNumber: 1,
          code: 'OCR_LOW_QUALITY' as any,
          message: 'This looks like a scanned (image-only) PDF with no text layer. Upload the menu as a photo or image (JPG/PNG) instead — images are read with OCR.',
          severity: 'error',
        });
        return this.fallbackError(issues);
      }
    } else if (input.kind === 'image') {
      // 1. OCR — engine selectable by config (I5): config.ocr_engine wins, then
      // MENU_OCR_ENGINE, default 'tesseract'. 'paddle' shells out to PaddleOCR.
      const engine = ((input.config as any)?.ocr_engine || process.env.MENU_OCR_ENGINE || 'tesseract').toLowerCase();
      try {
        if (engine === 'paddle') {
          const p = this.paddleOcr(input.bytes, input.mime);
          rawText = p.text;
          ocrConfidence = p.confidence;
          ocrEngineUsed = `paddleocr ${p.version}`;
        } else {
          const result = await Tesseract.recognize(input.bytes, 'sqi+eng', {
            logger: m => {}
          });
          rawText = result.data.text;
          ocrConfidence = result.data.confidence / 100.0;
        }
      } catch (e: any) {
        issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `OCR Failed (${engine}): ${e.message}`, severity: 'error' });
        return this.fallbackError(issues);
      }
      ocrMs = Date.now() - t0;

      if (ocrConfidence < 0.4) {
        issues.push({ rowNumber: 1, code: 'OCR_LOW_QUALITY' as any, message: 'Image quality is poor, results may be inaccurate.', severity: 'warning' });
      }
    } else if (input.kind === 'html' || input.kind === 'text') {
      // P6-3 (council C1) — the net-new scrape path. Isolate the menu region BEFORE the AI boundary
      // (staff names / testimonials live in About/Team/footer), then FALL THROUGH to the single
      // redaction below (:404). One redactor, all kinds converge — the HTML path cannot bypass it.
      const decoded = Buffer.from(input.bytes).toString('utf8');
      const regionText = input.kind === 'html' ? extractMenuRegion(decoded) : decoded;

      // Name-guard: if person-PII signals survive the allowlist, FAIL CLOSED — never send third-party
      // names to an external model (binding zero-PII-in-AI, ADR-0011 / operator decision #4 not waived).
      const scan = scanResidualPii(regionText);
      if (scan.dense) {
        issues.push({
          rowNumber: 1,
          code: 'PII_DENSE' as any,
          message: `menu region still carries person-PII signals (${scan.score}) — routed to manual review, not sent to the model`,
          severity: 'error',
        });
        return this.fallbackError(issues);
      }
      rawText = regionText;
      ocrMs = Date.now() - t0;
    } else {
      throw new Error(`AiOcrParser does not support kind: ${(input as { kind: string }).kind}`);
    }

    // PRIVACY (ADR-0011 / ETHICAL-STOP-1, redact-by-default — BINDING): the OCR text is
    // redacted with piiRedactor BEFORE it enters the LLM prompt. A menu PHOTO can carry
    // incidental THIRD-PARTY PII (staff name/phone, handwritten note) the owner's consent
    // does not cover — that must never egress to an external model (OpenRouter / Zen). The
    // redacted copy feeds both the prompt (line ~515) and the provenance hash. Accepted
    // tradeoff: the redactor may also strip the venue's OWN phone, weakening onboarding
    // pre-fill; the remedy is a SEPARATE consented venue-contact path, never raw PII to the
    // model (docs/.../ethical-decisions.md, gate CLOSED 2026-06-26).
    const { text: redactedText } = this.piiRedactor.redact(rawText);

    const t1 = Date.now();

    // 3. LLM Extraction
    const llmModel = process.env.LLM_PROVIDER || 'llama3.1:8b-instruct';
    const provider = detectProvider(llmModel);

    if (provider === 'mock') {
      return this.mockResult(input.config, issues);
    }

    if (provider === 'heuristic') {
      return this.heuristicResult(rawText, input.config, issues, ocrEngineUsed, ocrMs, redactedText);
    }

    const modelForProvider = provider === 'zen'
      ? (process.env.OPENCODE_ZEN_MODEL || 'deepseek-v4-flash-free')
      : provider === 'groq'
      ? (process.env.GROQ_MODEL || 'llama-3.1-8b-instant')
      : provider === 'openai'
        ? (process.env.OPENAI_MODEL || 'gpt-4o-mini')
        : provider === 'openrouter'
          ? (process.env.OPENROUTER_MODEL || 'openai/gpt-4o-mini')
          : llmModel;

    // Retrieve relevant memories to enhance the OCR prompt
    let memoryExamples = '';
    if (this.memoryService) {
      try {
        await this.memoryService.initialize();
        const memories = await this.memoryService.search('menu ingredients description bom', {
          topK: 2,
          // We could add filters here if we tag memories appropriately
          // For now, we'll search broadly and filter results if needed
        });
        
        if (memories?.results && memories.results.length > 0) {
          // Format memories as concise examples
          const examples = memories.results
            .slice(0, 2) // Limit to 2 memories to save tokens
            .map((mem: any) => {
              try {
                // Try to parse the memory as JSON to extract structured data
                const data = typeof mem.memory === 'string' ? JSON.parse(mem.memory) : mem.memory;
                 if (data.products && Array.isArray(data.products)) {
                  // Extract a concise example of a product with bom and description
                  const product = data.products[0];
                  if (product) {
                   const bomExample = product.attributesJson?.bom?.[0] 
                        ? ('{ name: "' + product.attributesJson.bom[0].name + '", quantity: "' + (product.attributesJson.bom[0].quantity || '') + '" }')
                        : '{ name: "Ingredient", quantity: "Amount" }';
                   const descExample = product.description 
                        ? ('"' + product.description.substring(0, 30) + '..."')
                        : '"Product description from ingredients"';
                    return `- Product: ${product.name || 'Unnamed'}, Description: ${descExample}, BOM: [${bomExample}]`;
                  }
                }
              } catch (e) {
                // If memory is not JSON or doesn't have expected structure, use as text snippet
                const snippet = typeof mem.memory === 'string' 
                  ? mem.memory.substring(0, 100) 
                  : String(mem.memory).substring(0, 100);
                return `- Text snippet: ${snippet}...`;
              }
              return '- Memory item';
            })
            .filter(Boolean);
            
          if (examples.length > 0) {
            memoryExamples = `\n\nEXAMPLES FROM PREVIOUS CORRECTIONS:\n${examples.join('\n')}\n`;
          }
        }
      } catch (err) {
        // If memory service fails, continue without memories
        console.warn('[AI-OCR] Failed to retrieve memories for enhancement:', (err as any).message);
      }
    }

    let llmResponse = '';
    let llmCacheKey = '';
    try {
      const currency = input.config.expectedCurrency || 'ALL';
      const prompt = `Extract the complete menu structure from the text below. Return ONLY valid JSON matching this schema:

{
  "restaurant": { "name": "Pizza Roma", "address": "Rruga Sami Frasheri 12, Tirana", "phone": "+355 69 123 4567", "hoursText": "Mon-Sun 10:00-23:00" },
  "categories": [{ "externalKey": "cat1", "name": "Appetizers" }],
  "products": [
    {
      "externalKey": "prod1",
      "categoryKey": "cat1",
      "name": "Margherita Pizza",
      "description": "Tomato sauce, mozzarella, fresh basil on thin crust",
      "price": 800,
      "currency": "${currency}",
      "available": true,
      "attributesJson": {
        "bom": [
          { "name": "Tomato sauce", "quantity": "100g", "allergens": [] },
          { "name": "Mozzarella", "quantity": "120g", "allergens": ["dairy"] },
          { "name": "Basil", "quantity": "5 leaves", "allergens": [] }
        ]
      }
    }
  ],
  "modifierGroups": [],
  "modifiers": [],
  "links": []
}${memoryExamples}

CRITICAL RULES:
1. PRICE: Every product MUST have a price. Extract the visible price in minor unit (e.g. "800 Lek" → 800, "1200 ALL" → 1200). If a price appears in the text but is unclear, provide your best estimate. NEVER omit price — it is required.
2. INGREDIENTS: If ingredients are listed in the menu text, add them to "attributesJson.bom" as an array of objects with "name" (string), "quantity" (string, optional), and "allergens" (string array of common allergens: dairy, eggs, nuts, soy, gluten, shellfish, fish).
3. DESCRIPTION: Include the product description if present. Combine ingredient list into a description if no separate description exists.
4. TYPES: "price" must be a NUMBER, "available" must be BOOLEAN, "externalKey"/"categoryKey" must be STRINGS.
5. FORMAT: Output ONLY the JSON object. No markdown fences, no commentary.
6. RESTAURANT: From the header/footer, extract the venue "name", full street "address", contact "phone", and opening-"hoursText" if present. Use "" for any field not found. Do NOT invent these. This is the venue's OWN business contact — never any customer's details.\n\n${redactedText}`;

      // Token economy: an identical (provider+model+prompt) parse is served from cache — zero tokens.
      llmCacheKey = llmCacheKeyOf(provider, modelForProvider, prompt);
      const cached = llmCacheGet(llmCacheKey);
      if (cached !== undefined) {
        llmResponse = cached;
      } else {
        llmResponse = await callLlm(provider, prompt, modelForProvider, 120000);
      }
    } catch (e: any) {
      // The configured LLM failed (provider down / out of credits / every free-model promo ended).
      // Degrade — don't cascade to 0 products: fall back to the zero-dependency heuristic structurer
      // so the owner still gets a reviewable draft. Surfaced as a warning, not a hard error.
      issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `LLM unavailable (${provider}): ${e.message} — fell back to heuristic extraction`, severity: 'warning' });
      return this.heuristicResult(rawText, input.config, issues, ocrEngineUsed, ocrMs, redactedText);
    }
    const llmMs = Date.now() - t1;

    // 4. Zod Validation
    let parsedJson: any;
    try {
      parsedJson = JSON.parse(llmResponse);
    } catch (e) {
      issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: 'LLM did not return valid JSON', severity: 'error' });
      return this.fallbackError(issues);
    }

    const validation = llmSchema.safeParse(parsedJson);
    if (!validation.success) {
      const firstErr = validation.error.errors[0];
      issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `LLM returned invalid schema: ${firstErr?.path?.join('.') || 'root'} → ${firstErr?.message}`, severity: 'error' });
      return this.fallbackError(issues);
    }

    const draft = validation.data;

    // Cache only a VALIDATED response (never garbage/partial) so a repeat parse skips the LLM entirely.
    if (llmCacheKey) llmCacheSet(llmCacheKey, llmResponse);

    // Format draft into CanonicalMenuDraft
    const canonicalDraft: CanonicalMenuDraft = {
      categories: draft.categories,
      products: draft.products,
      modifierGroups: draft.modifierGroups,
      modifiers: draft.modifiers,
      links: draft.links,
      translations: []
    };

    // Calculate low confidence and errors
    let lowConfidenceCount = 0;
    if (ocrConfidence < 0.6) {
      lowConfidenceCount = canonicalDraft.products.length;
      issues.push({ rowNumber: 1, code: 'POTENTIALLY_UNSAFE_VALUE' as any, message: 'Low confidence in extraction', severity: 'warning' });
    }

    // B2 hallucination grounding (ADR-0011) — FLAG-GATED DARK (MENU_GROUNDING_ENABLED, default off)
    // behind the B5/B12 RLS gate (import_sessions FORCE + strengthened verify:rls). A parsed price
    // with no normalizer-matching token in the OCR text is a likely hallucination → a warning for the
    // owner draft-review. NEVER auto-publishes — the existing draft → owner-publish gate stands.
    // Grounds against redactedText (prices survive redaction; the redactor strips only phone/email/
    // card/iban/url) so no raw PII is reintroduced.
    let grounding: { grounded: number; ungrounded: number } | undefined;
    if (process.env.MENU_GROUNDING_ENABLED === 'true') {
      const minorUnit = typeof (input.config as any).currencyMinorUnit === 'number' ? (input.config as any).currencyMinorUnit : 0;
      const ocrMinors = collectOcrPriceMinors(redactedText, minorUnit);
      const g = computeGrounding(canonicalDraft.products as any, ocrMinors);
      grounding = { grounded: g.groundedCount, ungrounded: g.ungrounded.length };
      for (const u of g.ungrounded) {
        issues.push({
          rowNumber: 1,
          code: 'PRICE_NOT_GROUNDED' as any,
          message: `Price ${u.price} for "${u.name ?? u.externalKey ?? 'item'}" was not found in the source text — verify before publishing`,
          severity: 'warning',
        });
      }
    }

    const provenance = {
      source: 'ai-ocr',
      raw_text_hash: crypto.createHash('sha256').update(redactedText).digest('hex'),
      model_id: llmModel,
      ocr_engine: ocrEngineUsed,
      ocr_ms: ocrMs,
      llm_ms: llmMs,
      ...(grounding ? { grounding } : {}),
    };

    // Note: since provenance isn't in CanonicalMenuDraft interface directly,
    // it will be attached via the server route handling to the import_sessions.draft_json._provenance

    // Restaurant metadata (business contact) for onboarding pre-fill — only when
    // the model actually returned non-empty values.
    const rm = (draft as any).restaurant;
    const restaurant = rm && (rm.name || rm.address || rm.phone || rm.hoursText)
      ? { name: rm.name || undefined, address: rm.address || undefined, phone: rm.phone || undefined, hoursText: rm.hoursText || undefined }
      : undefined;

    return {
      draft: canonicalDraft,
      issues,
      summary: {
        valid: canonicalDraft.products.length,
        errors: 0,
        warnings: issues.length,
        mode: 'merge',
        low_confidence_count: lowConfidenceCount
      },
      ...(restaurant ? { restaurant } : {}),
    };
  }

  /**
   * Zero-dependency, no-API-key menu structurer. Turns the OCR/PDF text into a
   * canonical draft using deterministic line heuristics (no LLM). Open-source &
   * free — the fallback so import works on a server with no LLM provider. The
   * owner reviews every draft before commit, so rough extraction is acceptable.
   */
  private heuristicResult(
    rawText: string,
    config: any,
    preIssues: ParseIssue[],
    ocrEngineUsed: string,
    ocrMs: number,
    redactedText: string,
  ): ParseResult {
    const t1 = Date.now();
    const currency = config.expectedCurrency || 'ALL';
    const minorUnit = typeof config.currencyMinorUnit === 'number' ? config.currencyMinorUnit : 0;
    const issues = [...preIssues];

    const lines = rawText
      .split(/\r?\n/)
      .flatMap((l) => l.split(/\s{3,}/)) // PDF text often joins rows with wide gaps
      .map((l) => l.trim())
      .filter(Boolean);

    // ── Restaurant header (business contact, not customer PII) ──────────
    const phoneMatch = rawText.match(/(\+?\d[\d\s().-]{6,}\d)/);
    const phoneRaw = phoneMatch?.[1];
    const phone = phoneRaw && phoneRaw.replace(/[^\d+]/g, '').length >= 7
      ? phoneRaw.trim()
      : undefined;
    const addressLine = lines.find((l) =>
      /\b(rrug[ae]n?|rr\.|bulevardi|bul\.|blvd|sheshi|lagj[ei]a|street|st\.|avenue|ave\.?|road|rd\.)\b/i.test(l)
      && !this.priceOf(l, minorUnit),
    );
    const hoursLine = lines.find((l) =>
      /\d{1,2}[:.]\d{2}/.test(l)
      && /(mon|tue|wed|thu|fri|sat|sun|hën|mar|mër|enj|pre|sht|die|open|hap|-|–|every|çdo)/i.test(l)
      && !this.priceOf(l, minorUnit),
    );
    // Only claim a venue name when there's a real header (address/phone present),
    // and only from the top line — avoids mislabelling a category as the venue.
    const hasHeader = !!(addressLine || phone);
    const top = lines[0] || '';
    const nameLine = hasHeader && top.length >= 2 && top.length <= 40
      && !this.priceOf(top, minorUnit) && !/@|\d{4,}/.test(top)
      ? top
      : undefined;
    const restaurant = (nameLine || addressLine || phone || hoursLine)
      ? {
          name: nameLine || undefined,
          address: addressLine || undefined,
          phone,
          hoursText: hoursLine || undefined,
        }
      : undefined;

    // ── Categories + products ───────────────────────────────────────────
    const categories: any[] = [];
    const products: any[] = [];
    const catKeyByName = new Map<string, string>();
    let currentCatKey: string | null = null;

    const ensureCategory = (name: string): string => {
      const key = catKeyByName.get(name.toLowerCase());
      if (key) return key;
      const newKey = `cat${categories.length + 1}`;
      categories.push({ externalKey: newKey, name });
      catKeyByName.set(name.toLowerCase(), newKey);
      return newKey;
    };

    for (const line of lines) {
      // Skip lines we already consumed as header metadata.
      if (line === nameLine || line === addressLine || line === hoursLine || (phone && line.includes(phone))) continue;
      if (/@/.test(line)) continue; // email / handle line

      const price = this.priceOf(line, minorUnit);
      if (price !== null) {
        // Product line: name is everything before the price token, dot leaders stripped.
        let name = line.slice(0, price.index).replace(/[.\s·•—–-]+$/u, '').trim();
        if (!name) name = line.replace(price.raw, '').replace(/[.\s·•—–-]+$/u, '').trim();
        if (!name) continue;
        if (!currentCatKey) currentCatKey = ensureCategory('Menu');
        products.push({
          externalKey: `prod${products.length + 1}`,
          categoryKey: currentCatKey,
          name,
          description: null,
          price: price.minor,
          currency,
          available: true,
        });
      } else if (line.length <= 40 && /[a-zA-ZÀ-ſ]/.test(line)) {
        // No price + short alpha line → treat as a category header.
        currentCatKey = ensureCategory(line.replace(/[:\s]+$/, ''));
      }
    }

    if (products.length === 0) {
      issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: 'No menu items could be detected in the document.', severity: 'error' } as any);
      return this.fallbackError(issues);
    }

    const canonicalDraft: CanonicalMenuDraft = {
      categories,
      products,
      modifierGroups: [],
      modifiers: [],
      links: [],
      translations: [],
    };

    return {
      draft: canonicalDraft,
      issues,
      summary: {
        valid: products.length,
        errors: 0,
        warnings: issues.length,
        mode: 'merge',
        low_confidence_count: 0,
      },
      ...(restaurant ? { restaurant } : {}),
      // provenance fields surfaced for parity with the LLM path
      _provenance: {
        source: 'ai-ocr',
        raw_text_hash: crypto.createHash('sha256').update(redactedText).digest('hex'),
        model_id: 'heuristic',
        ocr_engine: ocrEngineUsed,
        ocr_ms: ocrMs,
        llm_ms: Date.now() - t1,
      },
    } as any;
  }

  /**
   * Extract a trailing price token from a menu line. Returns null when the line
   * has no plausible price. `minor` is the integer price in the currency's minor
   * units (e.g. "8.50" with minorUnit 2 → 850, "800 Lek" with minorUnit 0 → 800).
   */
  private priceOf(line: string, minorUnit: number): { minor: number; index: number; raw: string } | null {
    // B7: single source of price normalization — shared with menu-grounding so the grounding check
    // compares against EXACTLY what the parser extracts (no substring drift).
    return extractTrailingPriceMinor(line, minorUnit);
  }

  private fallbackError(issues: ParseIssue[]): ParseResult {
    return {
      draft: { categories: [], products: [], modifierGroups: [], modifiers: [], links: [], translations: [] },
      issues,
      summary: { valid: 0, errors: issues.filter(i => i.severity === 'error').length, warnings: issues.filter(i => i.severity === 'warning').length, mode: 'merge' }
    };
  }

  private mockResult(config: any, preIssues: ParseIssue[] = []): ParseResult {
    return {
      draft: {
        categories: [{ externalKey: 'cat1', name: 'Pizzas' }],
        products: [{
          externalKey: 'prod1', categoryKey: 'cat1', name: 'Mock Pizza', price: 1000, currency: config.expectedCurrency || 'ALL', available: true
        }],
        modifierGroups: [], modifiers: [], links: [], translations: []
      },
      issues: preIssues,
      summary: { valid: 1, errors: 0, warnings: preIssues.length, mode: 'merge', low_confidence_count: 0 }
    };
  }
}
