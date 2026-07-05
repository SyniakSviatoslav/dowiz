---
CONTEXT:   Building the systemic behavioural-invariants gate (ledger #14) — a live-DOM WCAG-AA
           contrast check over storefront text, meant to catch the dark-on-dark class (#6).
DECISIONS: First version walked ancestors for the first opaque backgroundColor and computed a
           luminance ratio for every curated text node (h1, product name, button).
WHERE:     The gate failed on the venue title h1 with "contrast 1.09:1" — looked like a real
           dark-on-dark bug. Inspection showed the h1 is light cream text over a dark teal
           `linear-gradient(...)` hero (perfectly readable). The heuristic ignored the gradient
           (a background-IMAGE, not backgroundColor), walked past it to a far-down solid bg, and
           produced a bogus ratio → a FALSE POSITIVE that would have failed a legitimate screen.
WHY:       Luminance contrast is only defined for text over a SOLID opaque colour. Over an
           image/gradient it is undefined (the right control is a scrim/overlay, not a ratio).
           A contrast gate that doesn't detect background-image in the ancestor chain will
           mis-judge every hero/banner/over-image label — and per the ratchet rule, a guardrail
           that flags legitimate code is mis-scoped and must be narrowed until it only catches
           the real regression.
CONFIDENCE: high
NEXT-TIME: Any luminance-contrast check must SKIP elements whose ancestor chain has a
           background-image/gradient before an opaque solid bg (contrast undefined there) and
           assert only on solid-surface text. Prove green on the live repo before committing the
           gate; a single false positive means re-scope, not lower the threshold. Over-image
           legibility is a separate invariant (needs a scrim check), not this one.
LINK:      e2e/tests/behavioural-invariants.spec.ts (effBg returns {image:true} → skip) ; ledger #14
---
