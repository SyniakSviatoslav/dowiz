# Reflection — a Phase-A scaffold served real humans (S1 astro sub-target)

**WHAT:** Operator reported (after the /_astro styles fix): "still a huge visual regression, many
functionalities broken or missing completely, no images, flows impossible to use." Diagnosis:
staging human `/s/:slug` was served by the Astro rebuild app — which its own README declares a
**Phase-A scaffold spike: 3 of 27 islands, 1 of 27 routes**, with checkout, images pipeline
(no R2/media URL construction at all), order tracking, venue-open gate, compare/macro-lens,
i18n persistence and full theming explicitly out of scope. Visual A/B (screenshots banked in
session scratchpad; astro capture vs restored capture): tenant theme gone, tabs→bare chips,
no sort/lenses, no compare, no descriptions/badges, broken thumbnail, display-only cart.
Stabilization: `CUTOVER_ASTRO_UPSTream` unset on dowiz-staging (env-only, reversible) → humans
back on the node React SPA with rust APIs — full pre-rebuild UX restored (verified: screenshot +
storefront-styles 3/3 + functional nets). Astro stays deployed dark; re-flip gated on a
feature-parity matrix (task created; h_t frame updated).

**WHY (causal root):** The S1 flip DoD proved **API/JSON parity** (theme/menu/info 0-diff) and
**bot JSON-LD parity** — and then routed HUMANS to a page whose *UI implementation* had no parity
gate at all. The flip decision conflated "the surface's data is parity-proven" with "the surface's
EXPERIENCE is parity-proven". Nothing in the cutover harness could catch it: the front-door
health-checks upstreams, the parity oracles diff payloads, the reliability gate drives APIs —
none of them ever *looked at* the rendered page. Same class as ledger #80 (pixels invisible to
parity nets), one level up: not an asset 404 but an entire product tier missing.

**Ratchet candidates:**
1. **Flip-DoD rule:** a surface flip that replaces UI (not just API) requires a UX-parity gate:
   the feature matrix green + visual-regression net diff vs the node baseline + core flows driven
   E2E on the candidate stack. (Belongs in REBUILD-MAP / cutover runbook; council owns adding it.)
2. e2e/tests/storefront-styles.spec.ts already pins styles; a follow-up spec should pin FEATURE
   PRESENCE (testid census: compare-toggle, macro-lens, cart, category tabs) so a scaffold page
   can never silently pass again — candidate after the parity matrix lands.
3. Doubt note honored: "who flipped astro to humans and when" is in cutover_flags history / fly
   releases, not re-derived here; the mechanism (S1_ASTRO_TEMPLATES + configured upstream) is
   proven by code + the flip's reversal restoring UX.
