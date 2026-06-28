import type { Pool } from 'pg';
import { advance } from './service.js';
import { classifyExtraction, type ExtractParseResult } from './menu-extractor.js';
import { locate as defaultLocate, type LocateResult } from './menu-source.js';

// P6 — the SOURCED→ENRICHED extraction pipeline, composed from the already-built + council-reviewed parts
// (MenuSource.locate SSRF-guard, AiOcrParser.parse C1 PII-before-AI, MenuExtractor.classifyExtraction H4
// no-fabrication gate). Drives the acquisition state machine; nothing here calls the AI directly — the
// parser does, behind the redaction boundary. locate + parser are injectable for deterministic tests.

export interface MenuParser {
  parse(input: unknown): Promise<ExtractParseResult>;
}
export interface OrchestrateDeps {
  locate?: (url: string) => Promise<LocateResult>;
}
export interface OrchestrateResult {
  state: 'ENRICHED' | 'MENU_NOT_FOUND' | 'LOW_QUALITY' | 'MANUAL_REVIEW';
  reason?: string;
}

/**
 * Run a source from SOURCED → (PLACE_INGESTED → MENU_EXTRACTED →) ENRICHED, or to a terminal/exit verdict:
 *  - locate finds nothing → MENU_NOT_FOUND
 *  - parse → classifyExtraction (H4): clean → ENRICHED with menu_draft; else LOW_QUALITY / MANUAL_REVIEW.
 * Every state transition rides advance() (legal-edge + REQUIRES_REASON enforced). ENRICHED is the only door
 * to PROVISIONED, so a failed extraction can never reach a shadow write.
 */
export async function orchestrateExtraction(
  pool: Pool,
  sourceId: string,
  websiteUrl: string,
  parser: MenuParser,
  deps: OrchestrateDeps = {},
): Promise<OrchestrateResult> {
  const locate = deps.locate ?? defaultLocate;

  await advance(pool, sourceId, 'PLACE_INGESTED', { website_url: websiteUrl });

  const located = await locate(websiteUrl);
  if (located.kind === 'none' || !located.bytes) {
    const reason = `no menu source: ${located.note ?? 'not found'}`;
    await advance(pool, sourceId, 'MENU_NOT_FOUND', { failure_reason: reason });
    return { state: 'MENU_NOT_FOUND', reason };
  }

  const input =
    located.kind === 'image'
      ? { kind: 'image', bytes: located.bytes, mime: located.mime ?? 'image/png', config: {} }
      : { kind: located.kind, bytes: located.bytes, config: {} };
  const result = await parser.parse(input);

  await advance(pool, sourceId, 'MENU_EXTRACTED', { menu_kind: located.kind });

  const decision = classifyExtraction(result);
  if (decision.verdict === 'ENRICHED') {
    await advance(pool, sourceId, 'ENRICHED', { menu_draft: decision.draft });
    return { state: 'ENRICHED' };
  }
  await advance(pool, sourceId, decision.verdict, { failure_reason: decision.reason });
  return { state: decision.verdict, reason: decision.reason };
}
