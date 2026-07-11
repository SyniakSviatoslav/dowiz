# G03 — Checkout Communication 6-kind selector vs 3-kind validator (live conversion-killing 400/422)

> **Date:** 2026-07-11 · **Gap owner doc:** audit `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md`
> §1 risk 7, §6.3 item 5, §9 rec 5 · **Known since:** 2026-07-04 (IG lane discovery, memory
> `rebuild-decision-rust-astro-2026-07-04.md:127-129`) · **Status: RESEARCH + BLUEPRINT ONLY —
> nothing in this doc is implemented; no file outside this one was created or modified.**
> Every claim below is labeled **VERIFIED** (re-checked against code/git/live prod in this session)
> or **CLAIMED** (from memory/docs, not independently re-checked).

---

## 1. Gap & evidence

**The gap.** The checkout "Communication" selector is REQUIRED and offers **6** messenger kinds;
the order-create validator accepts only **3**. A customer choosing Phone, Signal, or SimpleX cannot
place an order — the single flow that produces revenue fails for 3 of 6 advertised contact methods.

**The failure chain (all VERIFIED in the current tree AND in `origin/main` = prod lineage):**

1. **FE offers 6, required.** `apps/web/src/lib/messenger.ts:6-8` —
   `MESSENGER_KINDS = ['phone','whatsapp','viber','telegram','signal','simplex']` (ADR-0016).
   `apps/web/src/pages/client/checkout/ContactInfoSection.tsx:85-87` renders all 6 as `<option>`s;
   `CheckoutPage.tsx:234-237` hard-requires a selection before submit; `CheckoutPage.tsx:329` sends
   `customer.messenger_kind` on every order.
2. **Validator accepts 3.** `packages/shared-types/src/legacy.ts:48` —
   `messenger_kind: z.enum(['telegram','whatsapp','viber']).optional()` inside a `.strict()` object.
   Identical at `origin/main:packages/shared-types/src/legacy.ts:48` (VERIFIED via `git show`).
3. **Route parses with it.** `apps/api/src/routes/orders.ts:93` — `CreateOrderInput.parse(request.body)`;
   on ZodError → `reply.sendError(400, 'VALIDATION_FAILED', …)` at `orders.ts:96`.
4. **What the customer sees.** `CheckoutPage.tsx:461-462`: a 400 is not in the designed 422/409
   business-error branch, so the customer gets the generic *"Failed to place order. Please try
   again."* plus the call-the-restaurant fallback CTA (`useFallbackPhone.ts`). Retrying can never
   succeed. No telemetry distinguishes this failure class.

**Precision correction:** memory and the audit call this "the 422". The Node path actually returns
**HTTP 400 `VALIDATION_FAILED`** (`orders.ts:93-97`); 422 is what the Rust S5 port emits for the
same class. The falsifiable proof below pins the real contract per lineage — asserting 422 against
the Node stack would be a false-RED.

**Second latent break in the same parse (VERIFIED):** `CreateOrderInput` is `.strict()` and has **no
`receiver` key**, but the FE sends a top-level `receiver: { name, messenger_kind, handle }` whenever
"deliver to someone else" is unchecked (`CheckoutPage.tsx:332-334`). Strict-mode unknown-key
rejection → the same 400 **for all 6 kinds**. This is S5 breaker finding H2 ("strict-parse drift
BROADER than messenger_kind", memory `rebuild-decision-rust-astro-2026-07-04.md:284`). Downstream
code is already waiting for the field: `orders.ts:588-593` consumes `(input as any).receiver` (dead
code today), `order-persistence.ts:88-109` persists `receiver_*`, and migration 074 created the
columns.

**Live-prod confirmation (VERIFIED 2026-07-11, HTTP probes, read-only):**
- `https://dowiz.fly.dev/assets/messenger-BlyeLgm8.js` (1,130 B, lazy chunk of `/s/demo`) contains
  `simplex` ×3, `signal.me`, `phone` ×5 → the 6-kind module is what prod serves customers.
- `ClientRoutes-BYpPyqVv.js` contains `checkout.comm_required` → the REQUIRED selector is live.
- `origin/main` tree: 6-kind `messenger.ts`, `ContactInfoSection.tsx` present, 3-kind `legacy.ts:48`.
- Prod DB: migration `1790000000074_checkout-communication.ts` **is in `origin/main`'s
  `packages/db/migrations/`** and prod is migrated through 084 (audit §4.1) → the DB CHECK already
  admits all 6 kinds on prod. **The DB is not the blocker; the Zod enum is the single gate.**

**Blast radius estimate:** every prod order where the customer picks Phone/Signal/SimpleX
(3 of 6 visible options — Phone is listed FIRST and is the most natural choice for the Albanian
market) and every "deliver to someone else" order. Live since the 2026-07-03 prod deploy (v405/v410).

---

## 2. Research findings

### 2.1 Full change-radius map (every definition/validation/consumption site)

**Definition sites (the drift is here — 4 independent kind lists in TS, 2 in Rust, 2 in SQL):**

| Site | Kinds | Role | Status |
|---|---|---|---|
| `apps/web/src/lib/messenger.ts:6-8` | 6 | FE canonical-in-practice (type + `MESSENGER_KINDS` + label/icon/link/isPhone helpers) | correct |
| `packages/shared-types/src/legacy.ts:48` | **3** | order-create gate (`CreateOrderInput.customer.messenger_kind`) | **THE BUG** |
| `packages/shared-types/src/legacy.ts` (whole `CreateOrderInput`) | — | missing top-level `receiver{}` key under `.strict()` | **BUG #2** |
| `apps/api/src/routes/courier/me.ts:79` | 3 | courier own-profile messenger update | consistent w/ its FE (see below) — not customer-facing |
| `apps/web/src/pages/courier/ShiftPage.tsx:283-286` | 3 | courier profile selector (telegram/whatsapp/viber only) | consistent pair with me.ts:79 |
| `rebuild/crates/api/src/routes/orders/dto.rs:40` (`enum MessengerKind`, 6) + `:74-82` (`ReceiverInput`) | 6 + receiver | Rust S5 order-create — **already carries the full fix** (REV-S5-3), incl. RED tests `dto.rs:242-261` (`signal_phone_simplex_messenger_kinds_now_parse`, unknown-kind rejected) | fixed on Rust lineage |
| `rebuild/crates/api/src/routes/courier/me.rs:261-265` | 3 | Rust courier profile (carried verbatim) | mirrors Node |
| `packages/db/migrations/1790000000038_messenger-deeplink.ts:8` | 3 | original CHECK (customers/couriers/orders) | superseded by 074 |
| `packages/db/migrations/1790000000074_checkout-communication.ts:13` | **6** | current CHECK on `customers.messenger_kind`, `couriers.messenger_kind`, `orders.customer_messenger_kind`, `orders.receiver_messenger_kind` + `receiver_*` columns | **applied staging (2026-06-30) AND in origin/main → applied prod via release_command (07-03 deploy)** |

**Validation path (write):** `CheckoutPage.tsx:329/333` → `POST /api/orders` →
`orders.ts:93 CreateOrderInput.parse` (**the only gate that is wrong**) → customer upsert
`orders.ts:546-553` (passes kind through) → snapshot + receiver `orders.ts:588-593` (comment at
`:591`: "Pass-through (DB CHECK is the kind gate)") → `order-persistence.ts:88-109` → DB CHECK (6-kind, correct).

**Consumption sites (read — all already 6-kind-tolerant, VERIFIED):**
- `apps/api/src/routes/courier/assignments.ts:49,65` — exposes `customer_messenger_kind/handle` to
  the assigned courier (status-gated).
- `apps/web/src/pages/courier/DeliveryPage.tsx:473-474` — `messengerLink(kind, handle)` renders a
  deep link; `messenger.ts:48-79` handles all 6 (SimpleX deliberately returns `null` = copyable text).
- `apps/web/src/pages/client/OrderStatusPage.tsx` — customer→courier link (same helper; CLAIMED from
  the IG-lane report, helper-driven so kind-agnostic).
- `apps/api/src/lib/anonymizer/index.ts:290` (current tree) and **`origin/main` anonymizer `:216-218`
  (VERIFIED)** — GDPR erasure NULLs `receiver_name/handle/messenger_kind` on BOTH lineages. The base
  erasure path for this feature is already prod-complete (independent of the G01 GDPR-trio gaps,
  which concern `delivery_photo_key`/fan-out/webhook — different fields).
- **Notification adapters: ZERO consumption** (VERIFIED: grep over `apps/api/src/notifications/**`
  incl. `adapters/` = 0 hits for messenger). Kinds never reach Telegram/push rendering. The IG-lane
  report independently confirmed this.
- `apps/api/src/lib/channel.ts` + `apps/web/src/lib/channel.ts` — **different concept**: `x-channel`
  acquisition attribution (`qr|nfc|link|…`), not MessengerKind. The 07-04 "unification" draft queue
  coupled them only because both duplicate an allowlist across api/web. Keep them decoupled in this fix.
- i18n: `packages/ui/src/lib/i18n-catalog.ts:4099+` already carries `checkout.comm_*` keys in
  **en/sq/uk** (catalog convention; repo rule "al,en" — sq is the Albanian locale). Kind labels are
  untranslated proper nouns by existing convention → **zero new i18n keys required** for the fix.
- E2E: `e2e/tests/client/{checkout,client-checkout-happy-path}.spec.ts` exist, but **no current spec
  covers signal/simplex/phone kinds or receiver** (VERIFIED grep — the IG-lane spec died with its worktree).

**NEW finding from this research — the length trap (VERIFIED):** `legacy.ts:49` caps
`messenger_handle` at `max(120)`. A real SimpleX invite link (`https://simplex.chat/invitation#/?v=2&smp=…`)
is typically 200–400 chars. **The enum fix alone would still 400 real SimpleX orders on handle
length.** The DB column is unbounded `text`; the Rust `dto.rs` `CustomerInput.messenger_handle` is
an unconstrained `Option<String>`. Any fix that tests simplex with a short dummy handle would be a
false-green (exactly the VbM anti-pattern) — the proof below uses a real-length invite string.

### 2.2 Draft-worktree fate: `agent-aee00a7da688fe62c` — worktree GONE, content recoverable

- **Not in `git worktree list`** (only `/root/dowiz`, the two stale 07-02 worktrees, one prunable
  scratchpad). The worktree directory `/root/dowiz/.claude/worktrees/agent-aee00a7da688fe62c` does
  not exist; `.claude/worktrees/` is empty (VERIFIED `find /` + `ls`).
- The branch `worktree-agent-aee00a7da688fe62c` still exists but carries **zero unique commits**
  (`git log <branch> --not <all mainlines>` = empty; its reflog file is empty). The lane's work was
  left **untracked** in the worktree (per memory `rebuild-decision-rust-astro-2026-07-04.md:79`) —
  untracked files in a deleted worktree are gone from disk. **The draft is LOST as files.**
- **BUT the full content is recoverable** from the subagent transcript
  `/root/.claude/projects/-root-dowiz/a6c4d150-bb37-4576-8fc3-7c977bf3ea0e/subagents/agent-aee00a7da688fe62c.jsonl`
  (1.54 MB, 2026-07-04 08:18; the parent session is the one that authored
  `rebuild-decision-rust-astro-2026-07-04.md`). It contains every `Write` payload: the shared-types SSOT draft
  (`shared-types-messenger.ts` — a 7-kind Zod enum incl. `instagram`, with operator apply-steps),
  the DB migration draft `migration-1790000000085_messenger-kind-instagram.ts`, the full Triadic
  Council record (proposal / 3 breaker rounds / counsel / resolution / ADR), 18 FE unit tests, and
  an e2e spec `checkout-communication-instagram.spec.ts`.
- **Quality assessment of the draft (from its recovered final report + SSOT file):** disciplined —
  council-gated, breaker-attacked ×3 (0 unresolved CRIT/HIGH), red→green-proven FE tests, correct
  identification that `packages/shared-types/` is protect-paths hard-blocked with no agent override.
  **However, it does NOT fix G03.** Its scope was *adding `instagram` as a 7th kind*; it explicitly
  deferred `legacy.ts:48` and `courier/me.ts:79` as "pre-existing gaps, out of scope" and its FE
  change would have made the mismatch *worse* (7 offered / 3 accepted) — which is why it was
  correctly HELD. The actual G03 fix (6-kind enum + `receiver{}` in `legacy.ts`) exists nowhere as a
  Node draft; its only implemented form is the Rust S5 port (`dto.rs`, with tests).

### 2.3 Migrations dir check (VERIFIED)

`packages/db/migrations/` (162 files): messenger CHECKs live in **038** (3-kind, original) and
**074** (6-kind replacement + `receiver_*` columns + RLS re-assert, forward-only `down()` no-op).
074 is present in `origin/main` and inside prod's applied range (066..084). **No DB migration is
needed for the 6-kind fix.** The only outstanding messenger migration is the *instagram* 7-kind
CHECK — a lost-worktree draft (recoverable per §2.2), red-line `packages/db/migrations/` placement,
operator-gated, and NOT part of G03's critical path.

### 2.4 Fallback UX for phone-only customers (grounding for the recommendation)

The design already answers "is phone always collected?": **phone-yielding kinds
(phone/whatsapp/viber/signal) fold the phone field into the kind's input** and drive
`customer.phone` (throttle/OTP/dedup intact — `CheckoutPage.tsx:231-246,327`); telegram/simplex are
phone-less **by design** (no customer row, IP-floor throttling — `orders.ts` tolerates absent phone).
So for a Phone-kind customer, the handle IS the phone; nothing else needs collecting. The correct
end-state is simply: the server accepts what the FE was designed (and council-approved, ADR-0016)
to send. An FE-only interim degrade exists (Option C below) but hides the defect and still cannot
serve SimpleX.

---

## 3. Options & tradeoffs

**A. Minimal contract fix (RECOMMENDED, Phase 1):** widen `legacy.ts:48` to the 6-kind enum, raise
`messenger_handle` max to 500, add optional `receiver{ name, messenger_kind(6), handle }` `.strict()`
object (mirror `dto.rs:74-82` — field is `handle`, not `messenger_handle`). ~15 changed LOC in ONE
protect-paths-gated file; zero API-route/DB/FE changes needed (all downstream code already consumes
the fields). Fastest kill of the conversion leak. Tradeoff: leaves 3 duplicate kind lists standing
(FE, shared-types, Rust) — drift remains possible until Option B.

**B. Full MessengerKind unification (SSOT), as Phase 4:** create `packages/shared-types/src/messenger.ts`
(adapt the recovered draft, *minus* instagram), barrel-export, make `legacy.ts` reference it, FE
re-imports type+array from `@deliveryos/shared-types`, optionally align courier me.ts/ShiftPage.
Right end-state; bigger radius (shared-types build, FE import chain, `verbatimModuleSyntax`
type-export nuance the draft already solved). Do NOT block Phase 1 on it.

**C. FE-only interim degrade (emergency stopgap only):** for `phone` (and optionally `signal`) omit
`messenger_kind` from the payload — phone still flows to `customer.phone`, order succeeds. No gates
(FE is agent-editable). Tradeoffs: loses the courier's Signal deep-link, records no kind, cannot
serve SimpleX or receiver{}, and masks the defect the fix is supposed to prove. Only worth shipping
if the operator gate on A stalls beyond days.

**D. Do the instagram extension now:** rejected — it widens the FE before the validator/DB admit a
7th kind (recreating this exact bug) and drags a red-line migration into a conversion hotfix.
Sequence strictly AFTER A (+B), as its own operator-scheduled change.

---

## 4. Recommended execution blueprint (phased)

Gate markers: 🟢 = agent-executable · 🔴 = OPERATOR (protect-paths / red-line / prod).
Repo rules honored: Verified-by-Math (every proof has a RED case), Ship Discipline (staging first,
prod on operator approval), i18n al(sq),en (+uk per catalog convention — expected **0 new keys**),
`packages/db/migrations/` red-line (touched only in Phase 4b, forward-only).

### Phase 0 — Recover & stage the record (effort: S, ~½ session)
| Step | Action | Gate | VbM proof |
|---|---|---|---|
| 0.1 | Extract the IG-lane artifacts from the transcript (§2.2 path) into `docs/design/checkout-messenger-kind-422/recovered/` (agent-writable docs path): council record, SSOT draft, migration-085 draft, e2e spec | 🟢 | recovered file list ↔ transcript `file_path` census (14-entry list in §2.2) matches 1:1 |
| 0.2 | Write the ready-to-apply `legacy.ts` diff as an operator draft in the same dir (6-kind enum + `max(500)` handle + `receiver{}` mirroring `dto.rs:74-82`) + a shared-types unit-test file | 🟢 | draft compiles standalone via a scratch copy (the IG lane's proven technique) |

### Phase 1 — Land the Node contract fix (effort: S, ~15 LOC + tests)
| Step | Action | Gate | VbM proof |
|---|---|---|---|
| 1.1 | Apply the draft to `packages/shared-types/src/legacy.ts` (enum 3→6; `messenger_handle` max 120→500; add optional `.strict()` `receiver{name: 1..120, messenger_kind: 6-enum, handle: 1..500}`) | 🔴 **protect-paths hard block** (`protect-paths.sh:54` — `packages/shared-types/` has NO agent/council override; operator edits, or `cp`s the draft, or explicitly authorizes in-session as with prior one-liners) | — |
| 1.2 | Add shared-types unit tests (new test file is inside the same protected package → same gate; alternatively place in `apps/api` test tree 🟢) | 🟢/🔴 | **GREEN:** `CreateOrderInput.parse` accepts all 6 kinds × {with, without} `receiver`; simplex with a **real 300-char invite link**; phone-less telegram/simplex. **RED (falsifiable):** `'instagram'`, `'icq'`, `''` rejected; `receiver` with an unknown key rejected; receiver with 3-field-incomplete rejected. Run the suite against the UNFIXED file first → must fail (red→green demonstrated) |
| 1.3 | `pnpm --filter @deliveryos/shared-types build && pnpm typecheck` + api unit suite; NO changes needed in `orders.ts` / `order-persistence.ts` / FE (verified consumers §2.1) | 🟢 | typecheck 0 errors; api tests green; grep-proof that no other file declares a 3-kind enum on the order path |
| 1.4 | REGRESSION-LEDGER row (`docs/regressions/REGRESSION-LEDGER.md`) + guardrail test committed with the fix (self-improvement rule) | 🟢 | ledger row cites the red→green test names |

### Phase 2 — Staging validation (effort: S–M)
| Step | Action | Gate | VbM proof |
|---|---|---|---|
| 2.1 | Commit on a feature branch; `bash scripts/deploy-staging.sh`. **No migration step** (074 already applied — assert, don't assume) | 🟢 (deploy script is the sanctioned path) | read-only constraint probe via staging proxy: `pg_constraint` shows the 6-kind `customers_messenger_kind_check` (074's verified names) — RED if 038's 3-kind text is still live |
| 2.2 | **Playwright API-level proof vs staging** (deliberately request-level: the KNOWN staging checkout-flow break at `checkout-phone` testid — audit §6.4, open since 07-04 — must NOT gate this fix; per Mandatory Proof Rule API-only needs ≥1 `request.*` assertion) | 🟢 | **GREEN:** `request.post('/api/orders')` with `kind=phone` → **201**; `kind=signal` → **201**; `kind=simplex` + real-length invite + no phone → **201**; `kind=telegram` + `receiver{…simplex…}` → **201** (each asserts the created order echoes the kind). **RED:** `kind='icq'` → **400 + code `VALIDATION_FAILED`** (Node contract — NOT 422; pin the observed pair). Run the same spec against PROD's current build first → the three new-kind cases MUST FAIL (proves the spec can go red = falsifiable) |
| 2.3 | UI-level Playwright (`VITE_BASE_URL=https://dowiz-staging.fly.dev`, select Signal in the real selector → order confirmation view) — best-effort; if it hits the pre-existing checkout-flow break, file that under its own open item, do not couple | 🟢 | DOM assertion on the confirmation route; skip-with-reason allowed only for the pre-existing break signature |
| 2.4 | GDPR spot-check: erase a staging order created with `receiver{}` + simplex → anonymizer NULLs `receiver_*` (path already exists both lineages, §2.1) | 🟢 | SQL read-back: `receiver_name/handle/messenger_kind IS NULL` post-erasure; RED = any survives |

### Phase 3 — Prod ship (rides the G01/G02 lineage decision) (effort: S once vehicle exists)
| Step | Action | Gate | VbM proof |
|---|---|---|---|
| 3.1 | **Dependency:** prod deploys from `origin/main` (tip `c8b2d5a0`, 07-03); the working branches are hash-bifurcated (audit §7.4). This fix must land on **whichever lineage G01 (GDPR-trio prod merge) / G02 (bifurcated-history merge design) selects as the prod train**. The fix is a ~15-LOC single-file diff → trivially cherry-pickable/re-appliable onto a curated-squash hotfix branch off `origin/main` (recommended: ride the SAME vehicle as the GDPR trio — one staging rehearsal, one deploy) or carried by the eventual paleo merge. The Rust S5 surface already has it (`dto.rs`) — if/when S5 cuts over, parity holds by construction (its tests assert the same matrix) | 🔴 prod merge/deploy = operator | diff-of-diffs: the applied hotfix hunk ≡ the staging-validated hunk (byte-identical `legacy.ts` section) |
| 3.2 | Post-deploy prod probe: re-run the Phase-2.2 spec against prod using a demo tenant (the 12 shadow demos) with immediate order cancel/cleanup | 🔴 creating prod orders = explicit operator approval (Ship Discipline) | same 4-GREEN/1-RED matrix vs prod; plus log telemetry: `VALIDATION_FAILED` rate on `POST /api/orders` mentioning `messenger_kind` drops to ~0 (falsifiable: it was the failure signature before) |

### Phase 4 — Unification + instagram (operator-scheduled, NOT on the G03 critical path)
| Step | Action | Gate | VbM proof |
|---|---|---|---|
| 4a | SSOT: `packages/shared-types/src/messenger.ts` (recovered draft minus instagram), barrel export, `legacy.ts` + `courier/me.ts` + FE import it; delete FE literal | 🔴 protect-paths | grep-count proof: exactly ONE `['phone','whatsapp',…]` literal in the tree; FE build green; existing kind tests still green |
| 4b | Instagram 7th kind: place recovered `1790000000085_messenger-kind-instagram.ts` (or renumber past 086) into `packages/db/migrations/` — **RED-LINE, operator-only, forward-only**; apply staging FIRST; only then FE/SSOT add `instagram` (M2 rule: a live kind must never outrun its CHECK) | 🔴 red-line migrations + operator | staging `pg_constraint` shows 7-kind CHECK before any FE deploy; recovered IG e2e spec (§2.2) goes green; RED: pre-migration, an `instagram` order must 400/422 |
| 4c | Courier surface 3→6 decision (me.ts:79 + ShiftPage options + `me.rs:261`) — currently a CONSISTENT pair, not a bug | 🔴 product decision | if extended: same red→green enum matrix on `PUT /api/courier/me` |

**Total critical-path effort (Phases 0–3): roughly one focused session + one operator apply/deploy window.**

---

## 5. Risks & rollback

- **Regression radius of widening the enum: additive-only.** Old kinds stay valid; existing rows
  (incl. pre-074 NULL kinds) untouched; DB CHECK already admits 6 on both envs; every read surface
  is helper-driven and 6-kind-tolerant (§2.1). The RED tests keep the enum CLOSED (unknown kinds
  still rejected — DB-CHECK parity, same property `dto.rs:261` asserts).
- **Existing orders with old kinds:** unaffected — no data migration, no re-validation of stored rows.
- **Phone-less kinds load-shape:** telegram/simplex orders create no customer row and fall to the
  IP-floor throttle — that path has been live since 06-30 for telegram; simplex merely joins it.
- **Receiver/GDPR:** `receiver_*` erasure is already implemented on BOTH lineages (VERIFIED
  `origin/main` anonymizer `:216-218`) — enabling receiver orders does not widen the (separate,
  G01) erasure-completeness gaps. Do not conflate; do ride the same deploy vehicle if convenient.
- **False-green traps this blueprint explicitly de-fuses:** (a) testing simplex with a short dummy
  handle (real invite links exceed the old 120 cap — §2.1 length trap); (b) asserting 422 where the
  Node stack returns 400 (§1 precision correction); (c) validating via the broken staging UI flow
  instead of request-level assertions (§6.4 known break).
- **Rollback:** revert the single commit → exact status-quo-ante (FE keeps offering 6, new kinds
  400 again). No migration to unwind (074 predates the fix and stays). Orders created during the
  fixed window remain valid under the unchanged DB CHECK and are readable by all existing surfaces
  — rollback loses no data and poisons nothing. For the Phase-4b migration: forward-only discipline
  applies (`down()` no-op), which is why instagram stays off the critical path.
- **Process risk:** the fix file sits in a protect-paths zone — an agent CANNOT self-serve it, and
  the one prior attempt to route around that (the IG worktree) ended as lost untracked work. The
  blueprint therefore stages everything as operator-appliable drafts in `docs/design/` (the repo's
  proven pattern for 074) and keeps the operator step to a single `cp`/edit + review.

## 6. Operator decision points

1. **Authorize the `legacy.ts` edit** (Phase 1.1 — protect-paths zone): apply the staged draft
   yourself, or grant explicit in-session authorization (precedent: the 07-04 "build hooks"
   one-liners). This is the ONLY gate between today and a staging-validated fix.
2. **Ship vehicle** (Phase 3.1): confirm this rides the same prod train as the G01 GDPR trio /
   G02 lineage decision — or, if those stall, approve a minimal curated hotfix branch off
   `origin/main` carrying just this diff (it is independent and 15 LOC).
3. **Prod canary order approval** (Phase 3.2): permission to create+cancel probe orders on a demo
   tenant against prod.
4. **`messenger_handle` cap 120→500** (folded into 1.1): confirm 500 (matches
   `delivery_instructions`; DB is unbounded text) or set another value — SimpleX needs ≥ ~400.
5. **Interim FE degrade (Option C)** — only if decision 1 stalls: yes/no.
6. **Phase 4 scheduling:** SSOT unification (4a), instagram + red-line 085 migration (4b), courier
   3→6 (4c) — schedule or explicitly PARK each in the ledger so they stop being silently carried.

---

*Research performed read-only 2026-07-11. Evidence: current tree `feat/paleo-dinosaur-digs`
(`e5eb3d03`, dirty, untouched), `origin/main` (`c8b2d5a0`) via `git show`/`git ls-tree`, live prod
HTTP probes (`/s/demo` + asset chunks), migrations dir census, hook source (`protect-paths.sh:54`),
memory corpus (`checkout-communication-2026-06-30.md`, `rebuild-decision-rust-astro-2026-07-04.md`),
and the recovered subagent transcript `agent-aee00a7da688fe62c.jsonl`. The only file created is this one.*
