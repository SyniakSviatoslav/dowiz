# Reflection — staging storefront unstyled (/_astro 404 through the front-door)

**WHAT:** Operator reported "styles broken on FE, big visual regression, across all pages."
Diagnosis in 3 curls: staging `/s/demo` HTML (Astro) referenced `/_astro/_slug_.*.css` → node
API JSON 404; admin/SPA assets all 200; prod untouched. Fix: bodyless `/_astro/*` passthrough
to the Astro upstream in front-door.ts, ahead of route matching, flag-independent. Red→green
unit test + live E2E guardrail. Ledger #80. Commit 8171d923.

**WHY (causal root, not the symptom):** The S1 astro sub-target was proven with a 0-diff
HTML/JSON parity oracle — a parity net that never requests the page's SUB-RESOURCES. The astro
build initially served styles in a form that didn't traverse the front-door (or the proof ran
against the astro app directly); when the astro app redeployed 7h before the report, the hashed
CSS externalized/rotated and the missing `/_astro` forward became user-visible everywhere at
once. Root class: **a proxied page's asset prefix is part of the surface contract, and none of
our parity/status nets asserted visual integrity** — HTML 200 + JSON 0-diff can coexist with a
fully unstyled product.

**Ratchet:** (1) e2e/tests/storefront-styles.spec.ts asserts stylesheets 200 text/css + applied
styling — the visual-integrity assertion the parity net lacked; (2) unit test pins the forward
incl. flag-independence (degrade window keeps stale HTML styled); (3) candidate lesson TRIGGER:
"adding a proxied HTML sub-target → enumerate its sub-resource prefixes and route them in the
same change."

**Doubt note:** why it was green before the redeploy is INFERRED (inline-vs-external CSS or
direct-app proof), not proven — the astro app's previous image is gone. The fix is correct
regardless (the forward is required for any external asset), but the exact green-window
mechanism stays unverified.
