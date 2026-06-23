---
TRIGGER: e2e/tests/behavioural-invariants.spec.ts
CAUSE: >
  Luminance (WCAG) contrast is only defined for text over a SOLID opaque colour.
  Over a background-image or gradient it is undefined — the correct control is a
  scrim/overlay, not a ratio. A contrast check that walks ancestors for the first
  opaque backgroundColor will skip PAST gradients/images and compute a bogus ratio
  for every hero/banner/over-image label → false positives that fail legitimate
  screens (observed: the /s/demo venue h1, light text on a dark teal gradient,
  scored 1.09:1).
ACTION: >
  When editing the behavioural-invariants contrast check (or adding any luminance
  contrast assertion) → cause: contrast is undefined over images/gradients → do:
  SKIP elements whose ancestor chain has a background-image before an opaque solid
  bg (return a skip sentinel; do not assert), and assert only on solid-surface
  text. Prove the gate GREEN on the live repo before committing — one false
  positive means re-scope, never lower the threshold. Over-image legibility is a
  separate invariant (scrim check), not this one.
LINK: e2e/tests/behavioural-invariants.spec.ts ; docs/reflections/INBOX/2026-06-23-contrast-gate-over-gradient.reflection.md
SCOPE: Luminance/WCAG contrast assertions only. Not other behavioural invariants.
STATUS: active
---

# Contrast gates must skip text over images/gradients

Source: reflection `2026-06-23-contrast-gate-over-gradient.reflection.md` (ledger #14).

Building the behavioural-invariants gate, the WCAG-AA contrast check flagged the
storefront venue `h1` at 1.09:1 — a false positive. The `h1` is light cream text
over a dark teal `linear-gradient(...)` hero (readable). The heuristic ignored the
gradient (a `background-image`, not `backgroundColor`), walked to a distant solid
bg, and produced a meaningless ratio.

Luminance contrast is undefined over images/gradients. The fix: `effBg()` returns
`{image:true}` the moment it meets a `background-image !== 'none'` ancestor, and the
test treats that as **skip** (not checked, not failed). Assert contrast only on text
over a solid opaque surface — that is the dark-on-dark class the gate exists to catch.
A guardrail that flags legitimate code is mis-scoped: narrow it, don't relax the bar.
