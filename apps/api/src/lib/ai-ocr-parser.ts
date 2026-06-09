// @ts-nocheck
import type { MenuParserProvider, ParserInputType } from '../ports.js';
import type { CanonicalMenuDraft, ParseIssue, ParseResult } from '@deliveryos/shared-types';
import { PiiRedactor } from './pii-redactor.js';
import Tesseract from 'tesseract.js';
import crypto from 'crypto';
import { z } from 'zod';

const llmSchema = z.object({
  categories: z.array(z.object({
    externalKey: z.string(),
    name: z.string()
  })),
  products: z.array(z.object({
    externalKey: z.string(),
    categoryKey: z.string(),
    name: z.string(),
    description: z.string().optional(),
    price: z.number(),
    currency: z.string(),
    available: z.boolean(),
    attributesJson: z.record(z.unknown()).optional(),
    imageKey: z.string().optional()
  })),
  modifierGroups: z.array(z.object({
    externalKey: z.string(),
    name: z.string(),
    minSelect: z.number(),
    maxSelect: z.number(),
    required: z.boolean()
  })),
  modifiers: z.array(z.object({
    externalKey: z.string(),
    groupKey: z.string(),
    name: z.string(),
    priceDelta: z.number(),
    available: z.boolean(),
    sortOrder: z.number().optional()
  })),
  links: z.array(z.object({
    productKey: z.string(),
    groupKey: z.string(),
    sortOrder: z.number()
  }))
});

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
    const llmEndpoint = process.env.LLM_ENDPOINT || 'http://localhost:11434/api/generate';
    const llmModel = process.env.LLM_PROVIDER || 'llama3.1:8b-instruct';

    // Using mock in CI
    if (llmModel === 'mock') {
      return this.mockResult(input.config, issues);
    }

    let llmResponse = '';
    try {
      const prompt = `Extract the menu structure from the following text into a JSON object matching the requested schema. Ensure prices are in the standard minor unit integer format (multiply by 10^${input.config.currencyMinorUnit || 0}). Currency should be ${input.config.expectedCurrency || 'ALL'}. Do not write anything other than JSON.\n\n${redactedText}`;

      const res = await fetch(llmEndpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: llmModel,
          prompt,
          stream: false,
          format: 'json'
        })
      });
      if (!res.ok) throw new Error(`LLM Error: ${res.statusText}`);
      const data = await res.json() as any;
      llmResponse = data.response;
    } catch (e: any) {
      issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `LLM Failed: ${e.message}`, severity: 'error' });
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
      issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: 'LLM returned invalid schema', severity: 'error' });
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
