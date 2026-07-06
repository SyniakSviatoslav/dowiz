# Reflection — rich OG demo previews, and the stack that actually serves the surface

**Date:** 2026-07-06 · **Slug:** og-preview-cutover-stack-mismatch
**Qualified because:** ≥3 code files + a red-line surface (reversed a signed P6-2/P6-3 privacy invariant).

## CONTEXT
Operator asked to upgrade the 12 demo storefronts so a pasted `/s/:slug` link unfurls as a rich product
card (name + Google rating + dish photo) on WhatsApp/Telegram/email, and — via AskUserQuestion — chose
"full per-venue unfurl, no consent gate," reversing the council-bound H3/P6-2/P6-3 rule that forced
generic OG for unconsented shadow tenants. Built: a server-side `GET /og/:slug.png` sharp/SVG card route,
rewired OG tags across Node (`preview-render`, `ssr`, `ssr-renderer`, `spa-shell`) + Rust (`storefront.rs`).

## DECISIONS
1. **Server-side sharp SVG→PNG card** over Playwright-pre-render+R2. WHY: no R2 creds / no flyctl to
   upload; `.gitignore:92` keeps outreach artifacts out of git (AGPL repo); sharp already a dep. Self-hosted,
   always-fresh, nothing committed.
2. **Bundle DejaVu + FONTCONFIG_FILE** at module load. WHY: `node:22-slim` ships no fonts → librsvg text is
   blank in prod though fine locally. Validated the fontless case by isolating fontconfig to a private dir.
3. **Kept `noindex` + honest banner + decline** even under "no consent gate." WHY: those protect the
   OPERATOR (passing-off / SEO-penalty), not the venue, and cost the unfurl nothing (unfurl ≠ index).
4. **Deploy Rust from `git archive HEAD` + overlay one file.** WHY: the branch had a concurrent, uncommitted
   `request_hash` (S5/money) refactor staged; deploying the working tree would ship half-done money-surface work.

## WHERE
`apps/api/src/lib/og-card.ts` (new), `routes/public/og-card.ts` (new), `lib/preview-render.ts`,
`lib/spa-shell.ts`, `lib/ssr-renderer.ts`, `routes/public/ssr.ts`, `modules/acquisition/provision-verifier.ts`,
`tests/preview-render.test.ts`, `tests/provision-verifier.test.ts`, `rebuild/crates/api/src/routes/storefront.rs`.

## WHY (causal root)
**A change's effect surface is the DEPLOYED-and-ROUTED stack, not the source you edited.** I built the whole
Node OG path, deployed it, saw `/og/*.png` render correctly — and nearly declared victory. But `/s/:slug`
still showed the OLD generic OG. Root cause: **staging serves the storefront surface from the Rust upstream
(S1 cut over)** — the `x-dowiz-cutover: rust:S1` response header was the oracle, and I only checked it AFTER
the tags came back stale. The Node `renderShadowPreview` I edited isn't even in the request path for
`/s/:slug` on staging; `/og/*` worked only because Rust has no such route so it falls through to Node. Two
separate stacks serve two parts of the same unfurl. Editing the correct source ≠ affecting the surface.

Secondary causal root: **a signed invariant is encoded in more places than the one you're reversing.** H3
lived in Node `preview-render`, Node `spa-shell`, a Node **provisioning GATE** (`provision-verifier` requires
`genericOg=true` — would have silently blocked all new demo provisioning), a Node test, AND the Rust
`storefront.rs` + its test. Reversing the policy meant flipping all six, not one.

## CONFIDENCE
HIGH on both roots — the cutover mismatch was directly observed (`x-dowiz-cutover: rust:S1` + byte-identical
Rust `shadow_preview_html` output), and the gate/test breakages were caught by grepping the invariant's
encodings before they shipped. Font-in-prod solution proven (deployed card byte-identical to local render).

## NEXT-TIME
- For ANY storefront/SSR-surface change on staging, curl the surface and read `x-dowiz-cutover` FIRST —
  decide which stack (Node vs Rust) actually serves it BEFORE choosing where to edit. A green `/og` proof
  did not imply the tags path was mine.
- When reversing a named invariant (Hn/Pn-n), grep the repo for its encodings across BOTH stacks and its
  gates/tests, not just the rendering function. A provisioning gate that enforces the old rule is a silent
  landmine.
- `fly [build].dockerfile` resolves RELATIVE to the config file's dir — never pass an absolute path (it
  doubles). And `flyctl deploy` can exit 0 while the build actually errored — grep the output for `Error:`.

## LINK
- [[og-preview-demo-upgrades-2026-07-06]] (memory) · docs/design/demo-preview-upgrades/PLAN.md
- [[rebuild-decision-rust-astro-2026-07-04]] (cutover harness / x-dowiz-cutover oracle)
- REGRESSION-LEDGER #77 class ("contract parity invisible until the live surface is driven") — same family:
  the live surface was driven by a DIFFERENT stack than the one edited.
