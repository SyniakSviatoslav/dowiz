// P6-3 (council C1) — the PRIMARY PII-before-AI control for the net-new HTML-scrape path.
//
// The PiiRedactor matches structured PII (email/phone/iban/card) but CANNOT reliably match free-text
// NAMES (staff bios, testimonials, "Owner: Jeton Berisha"). Council fix: do NOT rely on a name regex —
// positively ISOLATE the menu region so the regions where names live (About/Team/Reviews/footer) never
// reach the AI at all, then a name-guard FAILS CLOSED if the residual text is still PII-dense.
//
// This is a deliberately conservative text extractor (no DOM lib dep): it strips non-content elements
// and known non-menu sections, then returns visible text. Over-stripping is SAFE here (a missing menu
// item routes to LOW_QUALITY/MANUAL_REVIEW — never a silent send); under-stripping is caught by the guard.

// Elements that never carry menu content — removed wholesale (with their content).
const DROP_ELEMENTS = /<(script|style|noscript|nav|header|footer|aside|form|svg|template|iframe)\b[^>]*>[\s\S]*?<\/\1>/gi;
// Self-closing / void noise.
const DROP_VOID = /<(meta|link|input|img|br|hr)\b[^>]*\/?>/gi;

// Headings/sections whose TEXT signals a non-menu region (where staff names / testimonials live).
// If a heading matches, the text from it up to the next heading is dropped.
const NON_MENU_HEADING =
  /(about|our story|story|team|staff|chef|meet|review|testimonial|contact|find us|location|hours|opening|careers|jobs|press|gallery|blog|news|history|owner|founder)/i;

const HEADING_SPLIT = /<\/?h[1-6]\b[^>]*>/i;

/** Strip HTML tags to visible text, collapsing whitespace. */
function htmlToText(html: string): string {
  return html
    .replace(/<[^>]+>/g, ' ')
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&#39;|&apos;/gi, "'")
    .replace(/&quot;/gi, '"')
    .replace(/[ \t]+/g, ' ')
    .replace(/\s*\n\s*/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

/**
 * Isolate the likely-menu region of an HTML page as plain text. Drops non-content elements, then
 * drops any heading-delimited section whose heading reads as non-menu (About/Team/Reviews/...).
 */
export function extractMenuRegion(html: string): string {
  let cleaned = html.replace(DROP_ELEMENTS, ' ').replace(DROP_VOID, ' ');

  // Split on headings; drop a section if its heading text is non-menu. The first chunk (pre-first-
  // heading) is kept (often the menu list itself).
  const parts = cleaned.split(HEADING_SPLIT);
  const kept: string[] = [];
  for (let i = 0; i < parts.length; i++) {
    const text = htmlToText(parts[i] ?? '');
    if (!text) continue;
    // A heading chunk is short and titles the FOLLOWING content. Detect via the next chunk pattern:
    // here we treat any chunk that is itself a non-menu heading phrase as a drop signal for itself.
    if (text.length <= 60 && NON_MENU_HEADING.test(text)) {
      // drop this heading AND the immediately-following content chunk
      i += 1;
      continue;
    }
    kept.push(text);
  }
  return kept.join('\n').replace(/\n{3,}/g, '\n\n').trim();
}

// ── Name-guard (fail-closed backstop) ──
// Anchored signals that survived the allowlist and indicate residual person-PII. We do NOT try to
// detect arbitrary names (unsolved); we detect the high-signal CONTEXTS where third-party names appear.
const PII_CONTEXT_SIGNALS: RegExp[] = [
  // "Chef Maria", "Owner: Jeton Berisha", "by Arben K.", "Meet our founder ..." (case-insensitive
  // trigger via char-classes; TitleCase name kept case-sensitive so menu items aren't matched).
  /\b(?:[Cc]hef|[Oo]wner|[Mm]anager|[Ff]ounder|[Dd]irector|[Pp]roprietor|[Hh]ost|[Hh]ostess|[Bb]y|[Mm]eet|[Ss]erved by|[Pp]repared by)\b[:.,]?\s+(?:[Oo]ur\s+)?[A-Z][a-z]+(?:\s+[A-Z][a-z.]+){0,2}/g,
  // testimonial attribution: «— Arben K.» / «- Maria H.»
  /[—\-–]\s*[A-Z][a-z]+(?:\s+[A-Z]\.?)+/g,
  // quoted testimonial sentences (5+ words inside quotes)
  /["“][^"”]{20,}["”]/g,
];

export interface PiiScan {
  score: number; // count of residual person-PII signals
  dense: boolean; // true → fail closed (route to MANUAL_REVIEW)
  hits: string[];
}

/**
 * Scan menu-region text for residual person-PII signals AFTER the allowlist. A non-zero score means
 * the allowlist did not fully isolate the menu → fail closed rather than send names to an external LLM.
 * Threshold is intentionally strict (default 1): the binding is "no third-party PII to the model".
 */
export function scanResidualPii(text: string, threshold = 1): PiiScan {
  const hits: string[] = [];
  for (const re of PII_CONTEXT_SIGNALS) {
    const rx = new RegExp(re.source, re.flags.includes('g') ? re.flags : re.flags + 'g');
    let m;
    while ((m = rx.exec(text)) !== null) hits.push(m[0].slice(0, 60));
  }
  return { score: hits.length, dense: hits.length >= threshold, hits };
}
