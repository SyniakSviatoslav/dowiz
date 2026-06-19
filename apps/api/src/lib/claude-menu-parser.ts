import Anthropic from '@anthropic-ai/sdk';
import { z } from 'zod';
import type { MenuParserProvider, ParserInputType } from '../ports.js';
// @ts-ignore — runtime types from shared-types
import type { CanonicalMenuDraft, ParseIssue, ParseResult, RestaurantMeta } from '@deliveryos/shared-types';

// Claude vision menu parser (Stage 12 successor). Sends the menu PDF/photo to a
// vision-capable Claude model NATIVELY (no OCR pre-step), and extracts the full
// menu structure PLUS restaurant-level metadata (name/address/phone/hours) in one
// structured-output call. This replaces the text-only ollama path (which produced
// empty output for image-only PDFs and could not see item photos or the header).
//
// PII note (invariant Z1): the model only ever sees the menu document. The
// restaurant name/address/phone it returns is BUSINESS contact info — not customer
// PII — and is required by the menu-first onboarding flow to pre-fill the location.
// No customer data is requested or returned.

const MODEL = process.env.ANTHROPIC_MENU_MODEL || 'claude-opus-4-8';

// Structured-output JSON schema (additionalProperties:false + all-required, per
// the structured-outputs contract). Absent text → "", no allergens → [].
const MENU_JSON_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['restaurant', 'categories', 'products'],
  properties: {
    restaurant: {
      type: 'object', additionalProperties: false,
      required: ['name', 'address', 'phone', 'hoursText'],
      properties: { name: { type: 'string' }, address: { type: 'string' }, phone: { type: 'string' }, hoursText: { type: 'string' } },
    },
    categories: {
      type: 'array',
      items: { type: 'object', additionalProperties: false, required: ['externalKey', 'name'], properties: { externalKey: { type: 'string' }, name: { type: 'string' } } },
    },
    products: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false,
        required: ['externalKey', 'categoryKey', 'name', 'description', 'price', 'available', 'hasImage', 'allergens'],
        properties: {
          externalKey: { type: 'string' }, categoryKey: { type: 'string' }, name: { type: 'string' },
          description: { type: 'string' }, price: { type: 'integer' }, available: { type: 'boolean' },
          hasImage: { type: 'boolean' }, allergens: { type: 'array', items: { type: 'string' } },
        },
      },
    },
  },
} as const;

// Defensive runtime validation (the model is constrained, but never trust blindly).
const ExtractionSchema = z.object({
  restaurant: z.object({ name: z.string(), address: z.string(), phone: z.string(), hoursText: z.string() }),
  categories: z.array(z.object({ externalKey: z.string(), name: z.string() })),
  products: z.array(z.object({
    externalKey: z.string(), categoryKey: z.string(), name: z.string(), description: z.string(),
    price: z.number(), available: z.boolean(), hasImage: z.boolean(), allergens: z.array(z.string()),
  })),
});

function buildPrompt(currency: string, minorUnit: number): string {
  const priceExample = minorUnit === 0 ? '"800 Lek" → 800' : '"8.50" → 850';
  return [
    'Extract this restaurant menu from the attached document for a food-delivery storefront.',
    'Return data matching the required JSON schema EXACTLY. Rules:',
    `- price: integer in the MINOR units of ${currency} (${minorUnit} minor digits). Example: ${priceExample}. Missing price → 0.`,
    '- Extract EVERY item: category, name, description (verbatim if present else ""), price.',
    '- categoryKey on each product MUST equal a category externalKey. Invent short stable keys (e.g. "pizzas").',
    '- externalKey: a short stable slug per item (e.g. "margherita-pizza").',
    '- allergens: identify any (dairy, eggs, nuts, soy, gluten, shellfish, fish) else [].',
    '- hasImage: true ONLY if the item is shown with a photo in the document.',
    '- available: true unless marked sold out / unavailable.',
    '- restaurant: extract venue name, full street address, contact phone, and opening-hours text from the header/footer. Use "" for anything you cannot find. Do NOT invent these.',
    'Return ONLY the structured menu.',
  ].join('\n');
}

export class ClaudeMenuParser implements MenuParserProvider {
  readonly id = 'claude';
  private client: Anthropic | null = null;

  private getClient(): Anthropic {
    if (!this.client) this.client = new Anthropic(); // reads ANTHROPIC_API_KEY
    return this.client;
  }

  async parse(input: ParserInputType): Promise<ParseResult> {
    const issues: ParseIssue[] = [];
    const cfg = (input as any).config || {};
    const currency: string = cfg.expectedCurrency || 'ALL';
    const minorUnit: number = typeof cfg.currencyMinorUnit === 'number' ? cfg.currencyMinorUnit : 0;

    if (!process.env.ANTHROPIC_API_KEY) {
      issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: 'Claude parser not configured (ANTHROPIC_API_KEY missing)', severity: 'error' });
      return fallbackError(issues);
    }

    // Native document/image content block — no OCR pre-step.
    const b64 = input.bytes.toString('base64');
    const media: any =
      input.kind === 'pdf'
        ? { type: 'document', source: { type: 'base64', media_type: 'application/pdf', data: b64 } }
        : { type: 'image', source: { type: 'base64', media_type: (input as any).mime, data: b64 } };

    let raw: string | undefined;
    try {
      const resp = await this.getClient().messages.create({
        model: MODEL,
        max_tokens: 16000,
        messages: [{ role: 'user', content: [media, { type: 'text', text: buildPrompt(currency, minorUnit) }] }],
        // Constrain the response to our schema (structured outputs).
        output_config: { format: { type: 'json_schema', schema: MENU_JSON_SCHEMA as any } },
      } as any);
      if ((resp as any).stop_reason === 'refusal') {
        issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: 'Claude declined to process this document', severity: 'error' });
        return fallbackError(issues);
      }
      raw = (resp.content.find((b: any) => b.type === 'text') as any)?.text;
    } catch (e: any) {
      issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `Claude parse failed: ${e?.message || e}`, severity: 'error' });
      return fallbackError(issues);
    }

    return mapToResult(raw, currency, issues);
  }
}

// Pure mapping from the model's JSON text → ParseResult. Exported for unit tests
// (no network / no key needed to verify the mapping logic).
export function mapToResult(raw: string | undefined, currency: string, issues: ParseIssue[]): ParseResult {
  if (!raw) {
    issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: 'Claude returned no content', severity: 'error' });
    return fallbackError(issues);
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: 'Claude did not return valid JSON', severity: 'error' });
    return fallbackError(issues);
  }
  const v = ExtractionSchema.safeParse(parsed);
  if (!v.success) {
    const e = v.error.errors[0];
    issues.push({ rowNumber: 1, code: 'PARSE_ERROR', message: `Claude returned invalid schema: ${e?.path?.join('.') || 'root'} → ${e?.message}`, severity: 'error' });
    return fallbackError(issues);
  }
  const data = v.data;

  const draft: CanonicalMenuDraft = {
    categories: data.categories.map((c) => ({ externalKey: c.externalKey, name: c.name })),
    products: data.products.map((p) => ({
      externalKey: p.externalKey,
      categoryKey: p.categoryKey,
      name: p.name,
      description: p.description || undefined,
      price: Math.round(p.price),
      currency,
      available: p.available,
      // allergens + hasImage ride along in attributesJson for the commit step / UI.
      attributesJson: { allergens: p.allergens, hasImage: p.hasImage },
    })),
    modifierGroups: [],
    modifiers: [],
    links: [],
    translations: [],
  };

  const restaurant: RestaurantMeta = {
    name: data.restaurant.name || undefined,
    address: data.restaurant.address || undefined,
    phone: data.restaurant.phone || undefined,
    hoursText: data.restaurant.hoursText || undefined,
  };

  return {
    draft,
    issues,
    summary: { valid: draft.products.length, errors: 0, warnings: issues.length, mode: 'merge', low_confidence_count: 0 },
    restaurant,
  };
}

function fallbackError(issues: ParseIssue[]): ParseResult {
  return {
    draft: { categories: [], products: [], modifierGroups: [], modifiers: [], links: [], translations: [] },
    issues,
    summary: { valid: 0, errors: issues.filter((i) => i.severity === 'error').length, warnings: issues.filter((i) => i.severity === 'warning').length, mode: 'merge' },
  };
}
