// @ts-nocheck
import type { MenuParserProvider, ParserInputType } from '../ports.js';
import type { CanonicalMenuDraft, ParseIssue, ParseResult } from '@deliveryos/shared-types';
import { PiiRedactor } from './pii-redactor.js';
import Tesseract from 'tesseract.js';
import crypto from 'crypto';
import { z } from 'zod';

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
type LlmProvider = 'ollama' | 'groq' | 'openai' | 'mock';

function detectProvider(model: string): LlmProvider {
  if (model === 'mock') return 'mock';
  const env = (process.env.LLM_ADAPTER || '').toLowerCase();
  if (env === 'groq') return 'groq';
  if (env === 'openai') return 'openai';
  if (env === 'ollama') return 'ollama';
  if (process.env.GROQ_API_KEY) return 'groq';
  if (process.env.OPENAI_API_KEY) return 'openai';
  return 'ollama';
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

export class AiOcrParser implements MenuParserProvider {
  readonly id = 'ai-ocr';
  private piiRedactor = new PiiRedactor();

  async parse(input: ParserInputType): Promise<ParseResult> {
    const issues: ParseIssue[] = [];
    const t0 = Date.now();

    let rawText = '';
    let ocrConfidence = 1;
    let ocrMs = 0;

    if (input.kind === 'pdf') {
      // Extract text directly from PDF, skip OCR
      try {
        // Pre-load worker module onto globalThis to prevent pdfjs-dist from
        // doing its own dynamic import("./pdf.worker.mjs") which fails in
        // esbuild-bundled environments (worker file is not on disk at runtime).
        const workerMod = await import('pdfjs-dist/build/pdf.worker.mjs');
        (globalThis as any).pdfjsWorker = workerMod;
        const pdfjs = await import('pdfjs-dist');
        const doc = await pdfjs.getDocument({ data: new Uint8Array(input.bytes) }).promise;
        const pages: string[] = [];
        for (let i = 1; i <= doc.numPages; i++) {
          const page = await doc.getPage(i);
          const content = await page.getTextContent();
          pages.push(content.items.map((item: any) => item.str).join(' '));
        }
        await doc.destroy();
        rawText = pages.join('\n\n');
      } catch (e: any) {
        issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `PDF parse failed: ${e.message}`, severity: 'error' });
        return this.fallbackError(issues);
      }
      ocrMs = Date.now() - t0;

      if (!rawText.trim()) {
        issues.push({ rowNumber: 1, code: 'OCR_LOW_QUALITY' as any, message: 'PDF contained no extractable text.', severity: 'error' });
        return this.fallbackError(issues);
      }
    } else if (input.kind === 'image') {
      // 1. OCR
      try {
        const result = await Tesseract.recognize(input.bytes, 'sqi+eng', {
          logger: m => {}
        });
        rawText = result.data.text;
        ocrConfidence = result.data.confidence / 100.0;
      } catch (e: any) {
        issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `OCR Failed: ${e.message}`, severity: 'error' });
        return this.fallbackError(issues);
      }
      ocrMs = Date.now() - t0;

      if (ocrConfidence < 0.4) {
        issues.push({ rowNumber: 1, code: 'OCR_LOW_QUALITY' as any, message: 'Image quality is poor, results may be inaccurate.', severity: 'warning' });
      }
    } else {
      throw new Error(`AiOcrParser does not support kind: ${input.kind}`);
    }

    // 2. PII Redaction
    const { text: redactedText, redactions } = this.piiRedactor.redact(rawText);
    for (const r of redactions) {
      issues.push({ rowNumber: 1, code: 'POTENTIALLY_UNSAFE_VALUE', message: `PII redacted (${r.kind})`, severity: 'warning' });
    }

    const t1 = Date.now();

    // 3. LLM Extraction
    const llmModel = process.env.LLM_PROVIDER || 'llama3.1:8b-instruct';
    const provider = detectProvider(llmModel);

    if (provider === 'mock') {
      return this.mockResult(input.config, issues);
    }

    const modelForProvider = provider === 'groq'
      ? (process.env.GROQ_MODEL || 'llama-3.1-8b-instant')
      : provider === 'openai'
        ? (process.env.OPENAI_MODEL || 'gpt-4o-mini')
        : llmModel;

    let llmResponse = '';
    try {
      const currency = input.config.expectedCurrency || 'ALL';
      const prompt = `Extract the menu structure from the menu text below into a JSON object. Use EXACTLY this schema — field names, types, and nesting must match:

{
  "categories": [{ "externalKey": "cat1", "name": "Category Name" }],
  "products": [
    {
      "externalKey": "prod1",
      "categoryKey": "cat1",
      "name": "Product Name",
      "description": "optional description or null",
      "price": 50000,
      "currency": "${currency}",
      "available": true
    }
  ],
  "modifierGroups": [],
  "modifiers": [],
  "links": []
}

Rules:
- "price" must be a JSON NUMBER (int), NOT a string. Price in ${currency} minor unit (e.g. 500 ALL = 50000).
- "available" must be a JSON BOOLEAN (true/false), NOT a string.
- "externalKey", "categoryKey" must be JSON STRINGS.
- Omit "description" and "imageKey" if not present (do not include with null value).
- Output ONLY valid JSON object, no markdown, no commentary.\n\n${redactedText}`;

      llmResponse = await callLlm(provider, prompt, modelForProvider, 120000);
    } catch (e: any) {
      issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `LLM Failed (${provider}): ${e.message}`, severity: 'error' });
      return this.fallbackError(issues);
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

    const provenance = {
      source: 'ai-ocr',
      raw_text_hash: crypto.createHash('sha256').update(redactedText).digest('hex'),
      model_id: llmModel,
      ocr_engine: 'tesseract.js',
      ocr_ms: ocrMs,
      llm_ms: llmMs
    };

    // Note: since provenance isn't in CanonicalMenuDraft interface directly,
    // it will be attached via the server route handling to the import_sessions.draft_json._provenance

    return {
      draft: canonicalDraft,
      issues,
      summary: {
        valid: canonicalDraft.products.length,
        errors: 0,
        warnings: issues.length,
        mode: 'merge',
        low_confidence_count: lowConfidenceCount
      }
    };
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
