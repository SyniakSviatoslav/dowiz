import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { PiiRedactor } from '../src/lib/pii-redactor.js';

// ADR-0011 / ETHICAL-STOP-1 (redact-by-default, BINDING) — proof that incidental THIRD-PARTY
// PII in a menu photo does NOT egress to the external LLM. Two obligations:
//   (1) redaction-recall fixture (Counsel a): seeded staff phone/email is stripped from the
//       text that feeds the prompt;
//   (2) wiring guardrail: the parser interpolates `redactedText` (not `rawText`) at the menu-
//       text injection point — locks the binding decision so a future edit can't silently
//       send raw PII to the model again.

test('OCR redaction (redact-by-default before the LLM prompt)', async (t) => {
  await t.test('strips seeded third-party phone/email from menu OCR text (recall)', () => {
    // A realistic Albanian-context menu footer with INCIDENTAL third-party PII a photo might
    // catch: a staff member's mobile + a personal email scribbled on the page.
    const ocr = [
      'BUKË & VERË — Rruga Myslym Shyri 12, Tiranë',
      'Pizza Margherita .......... 800 Lek',
      'Sallatë Greke ............. 450 Lek',
      'Kontakt kamarieri: Ardit +355 69 234 5678',
      'porosi: ardit.hoxha@gmail.com',
    ].join('\n');

    const { text: redacted, redactions } = new PiiRedactor().redact(ocr);

    // Third-party contact must NOT survive into the prompt input.
    assert.ok(!redacted.includes('+355 69 234 5678'), 'staff phone leaked into prompt input');
    assert.ok(!redacted.includes('ardit.hoxha@gmail.com'), 'staff email leaked into prompt input');
    assert.ok(redactions.some((r) => r.kind === 'phone'), 'phone not detected');
    assert.ok(redactions.some((r) => r.kind === 'email'), 'email not detected');
    // VALUE-ANCHORED (not just `kind` presence): bind each redaction to the SPECIFIC staff-contact
    // token. `redactions.some(kind==='phone')` alone goes green if a price like "800 Lek" is
    // mis-classified as a phone while the real staff phone survives. These anchors fail in exactly
    // that case — the contact-line tokens must be the things replaced.
    assert.ok(
      redacted.includes('Ardit [REDACTED]'),
      'the staff phone after the contact label was not the redacted token',
    );
    assert.ok(
      redacted.includes('porosi: [REDACTED]'),
      'the staff email after the order label was not the redacted token',
    );

    // Menu CONTENT (prices, item names, street address) is preserved — redaction is targeted,
    // not destructive, so extraction quality holds.
    assert.ok(redacted.includes('800 Lek'));
    assert.ok(redacted.includes('Pizza Margherita'));
    assert.ok(redacted.includes('Rruga Myslym Shyri 12'));

    // HONEST RESIDUAL (B8): the pattern redactor has no NAME detector, so a bare personal name
    // ("Ardit") survives. Recorded — the floor for that gap is the human review of the draft
    // before publish, not this redactor. Asserted so the limitation is explicit, not hidden.
    assert.ok(redacted.includes('Ardit'), 'name detection is NOT claimed (documented residual)');
  });

  await t.test('parser feeds redactedText (not rawText) into the LLM prompt (wiring guardrail)', () => {
    const src = readFileSync(
      fileURLToPath(new URL('../src/lib/ai-ocr-parser.ts', import.meta.url)),
      'utf8',
    );
    // The menu text is appended to the prompt as the FINAL `\n\n${...}` interpolation. It must
    // be redactedText. Guard both directions: redactedText present, rawText NOT interpolated.
    assert.ok(
      /\\n\\n\$\{redactedText\}`/.test(src),
      'prompt must end with the REDACTED menu text (\\n\\n${redactedText})',
    );
    assert.ok(
      !/`[^`]*\$\{rawText\}`/.test(src),
      'no prompt template may interpolate ${rawText} — redact-by-default is binding',
    );
    // FINDING-1 (short-circuit defence): the `${redactedText}` check above still passes for
    // `const redactedText = rawText` (redaction disabled, name unchanged). Lock the data BINDING:
    // redactedText must be the destructured OUTPUT of piiRedactor.redact(rawText), never an alias.
    assert.ok(
      /\{\s*text:\s*redactedText\s*\}\s*=\s*this\.piiRedactor\.redact\(rawText\)/.test(src),
      'redactedText must be the OUTPUT of piiRedactor.redact(rawText), not an alias of rawText',
    );
    assert.ok(
      !/\bredactedText\s*=\s*rawText\b/.test(src),
      'redactedText must never be aliased to rawText — that short-circuits redaction',
    );
    // TODO(needs_staging): a true DATA-FLOW proof (vs source-text) needs an integration run that
    // feeds a PDF whose text layer carries a staff phone through parse() into a prompt-capturing
    // mock LLM and asserts the captured prompt omits the raw phone. Source-text guard is the floor.
  });

  await t.test('heuristic onboarding pre-fill is a SEPARATE non-egress surface (residual + escalation)', () => {
    // FINDING-2: the heuristic structurer derives restaurant.phone from rawText (ai-ocr-parser.ts
    // ~L649 `rawText.match`), NOT redactedText — so an incidental third-party staff phone can
    // pre-fill the venue-contact field. ADR-0011-bounded: restaurant.* is LOCAL onboarding pre-fill
    // the owner reviews before publish and is NEVER sent to the external LLM (only redactedText
    // egresses). Switching the source to redactedText would break the intentional venue-OWN-phone
    // pre-fill proven in ai-ocr-parser.test.ts L385-398 → escalate, do not silently "fix". What this
    // guardrail locks: the EGRESS copy carries no staff phone while menu content survives.
    const ocr = 'Trattoria Roma\nTel +355 69 234 5678\nPizza 800 Lek';
    const { text: egress } = new PiiRedactor().redact(ocr);
    assert.ok(!egress.includes('+355 69 234 5678'), 'staff phone must not survive into the egress copy');
    assert.ok(egress.includes('800 Lek'), 'menu price must survive redaction');
    // TODO(escalate/needs_staging): a dedicated integration test must drive parse() through the
    // heuristic path with a THIRD-PARTY (non-venue) phone and assert restaurant.phone is owner-flagged
    // before publish — a product decision (separate consented venue-contact path), not a redaction bug.
  });
});
