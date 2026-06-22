# Counsel Opinion — Cinematic Product Media, Phase 1 (`product_media` seam)

**Reviewer:** Counsel (Philosopher · Physician), DeliveryOS Triadic Council
**Subject:** `proposal.md` + `phase1-implementation.md` + `docs/adr/0002-product-media-seam.md`
**Authority:** ADVISORY. Aesthetics/strategy are non-blocking. ETHICAL-STOP is friction on a
grounded red line, not a veto — a conscious human still decides and finishes.
**Verdict in one line:** Phase 1 is ethically clean and strategically *unusually* disciplined.
Zero grounded ETHICAL-STOP. Three non-blocking corrections (one is a real, copy-paste-able
factual error in the spec). One open question for the human.

---

## 1. Reasoning by lens (only what is load-bearing)

### Justice / stakeholders — who wins, who carries the cost
- **Owner** wins (richer storefront, a premium reason to be on Business tier). **Customer**
  pays *nothing* extra: media is lazy-on-modal-open, ≈0 KB base-bundle delta, poster-only under
  save-data/slow-net. The cost surface (R2 storage + CF requests) is **bounded to the owner who
  opts in**, capped per-location, behind the free immutable cache. **No cost is externalised
  onto a party who didn't choose it** — the rare design where the spend-maker is the
  spend-bearer. This is the *just* shape.
- **Courier** is untouched — no surveillance, no agency surface here. Correctly out of scope.

### Dignity / autonomy
- Nothing here watches, coerces, or strips agency from anyone. Out of the dignity domain
  entirely. (Noted only to confirm the lens was applied, not skipped.)

### Honesty / consent — and the dark-pattern test
- **The server stays authoritative for price/availability; client-side hiding is presentation
  only, re-validated at the order endpoint.** This is the honest posture and it is stated
  explicitly (§1 non-goals, §6). The UI cannot lie about what costs what. **Pass.**
- **No dark pattern**: rich media is opt-in premium ornament, not a manipulation of the order
  flow. There is no soft-confirm-as-trap, no urgency theatre, no consent dirty-pattern. The one
  thing to *watch later* (Phase 3+): an autoplay-muted looping video is borderline — it must
  stay genuinely decorative and never imply scarcity/social-proof it can't back. Phase-1 ships
  none of this, so it is a note for the Phase-3 gate, not now.
- **PII:** none introduced. `alt` text and `meta` are owner-authored product data, not personal
  data. Anonymise-not-delete and zero-PII-in-AI are not engaged. **Clean.**

### Care / harm — who gets hurt when it breaks
- The failure-first work here is genuinely good. Every renderer degrades to *today's storefront*
  (`media → image_key → gradient`), never to a broken page or a **blocked order**. "Add-to-Cart
  never blocked by the animation" is stated as a NO-GO at Phase 5 (§11). The human who could be
  hurt — a customer who can't order, an owner who loses a sale to a spinner — is explicitly
  protected. **This is the care lens done right.**
- One residual care-risk lives in **Phase 5, not Phase 1**: R6 (RAF/canvas leak on rapid
  open/close) is *the same failure class* that caused the outage — unbounded lifecycle. The
  design names it, gates it (proven teardown over 50 cycles), and defers it. Holding that gate
  is the whole ethical weight of this program. Phase 1 carries none of that runtime, so it is
  honest to ship Phase 1 now — **provided the team does not let "the schema is in, momentum is
  here" pressure-collapse the R6 gate later.** (See the open question.)

### Long horizon / strategy — second-order effects, reversibility, lock-in
- "Schema now, runtime later" is **strategically sound here, not speculative debt** — and the
  distinction is grounded: the irreversible cost (DDL + RLS + menu_version semantics on a table
  related to live menu tables) genuinely *is* cheaper to pay before the Phase-2 money tables
  land, and the reversible cost (runtime) genuinely *is* deferred behind a default-off flag with
  per-phase rollback. An inert table + one nullable `ON DELETE SET NULL` FoK is close to zero
  carrying cost and is forward-only. This is YAGNI's legitimate exception: *the schema is the
  expensive-to-change part, and the window is now.* I would call this prudence, not premature.
- **The one strategic asterisk** is `locations.plan` (see §3 below): it is the first
  monetisation primitive in the schema, introduced as a side-effect of a *media* feature, with
  the trigger being "first real paid order," not "first paid plan." Be deliberate that the media
  feature is not quietly becoming the place the billing model gets decided. (Open question.)

### Aesthetics / conceptual integrity
- The design *is* aesthetically coherent in the deep sense: "schema rich, runtime minimal,"
  "one heavy decode at a time," "mirror only proven repo patterns," "≈0 KB base bundle" — these
  are restraint expressed as design language, and they line up with the recent storefront polish
  rather than fighting it. Phase 1 is explicitly **byte-identical to today's storefront**, so it
  cannot break the current experience by construction. The elegance here is *real*, not seductive
  — it buys fewer bugs and therefore less harm. Honest UI is beautiful UI; this keeps it honest.
- The MediaRenderer registry-with-stubs is the right shape (a `kind`-keyed switch that
  falls through to `image_key`): it is the minimal seam, not a framework. Good taste.

### Epistemic — what assumption is load-bearing and unexamined
- The load-bearing, **partially-unverified** assumption is the "byte-identical `read_public_menu`
  JSON" claim — see the grounded correction in §3.1. The proposal cites the *wrong* migration as
  the body to copy, which means the verification gate ("byte-identical for NULL
  `primary_media_id`") is doing real work and must not be skipped.
- The missing perspective: **the customer on a cheap device on a bad network in Durrës.** The
  save-data/poster-only path serves them, which is good — but no one in the doc speaks *for* them
  as a stakeholder; they appear only as a degradation case. (Open question reframes this.)

---

## 2. ETHICAL-STOPs

**None.** No grounded red line is crossed by Phase 1.

I checked each grounded line explicitly:
- human-in-loop / zero-autoban — not engaged.
- friction-not-verdict — N/A.
- courier-finishes / GPS-garbage-rejected / cash→alert — not engaged.
- anonymise-not-delete / zero-PII-in-AI / claim-check — no PII introduced.
- soft-confirm-not-a-trap / **server-authoritative** — **upheld** (server stays price/availability
  authority; client hiding is presentation only, re-validated at order).
- **a11y WCAG-AA** — actively respected (reduced-motion → poster/instant, WCAG 2.2.2 pause
  control, aria-live "N of M", CLS=0 gate). This is a strength, not a risk.
- **"schema rich, runtime minimal"** — this design *is* that doctrine, faithfully.
- trigger = first real paid order — see the open question, but no violation.

An ETHICAL-STOP here would be a *preference dressed as a line*. I decline to manufacture one.

---

## 3. Non-blocking corrections + advice (strategic / aesthetic / epistemic)

### 3.1 (Correctness — highest value) The spec copies from the WRONG migration body
`phase1-implementation.md §A.7` says the migration is a *"verbatim copy of the **1790000000033**
body"* of `read_public_menu`. That is wrong and an implementer following it verbatim would copy
the wrong function:
- The single-locale `read_public_menu(p_location_id_or_slug text, p_locale text)` body lives
  **only** at `packages/db/migrations/1780338982022_read_public_menu.ts`. Migration
  `1790000000033` is `localize-modifiers` and does **not** redefine that function.
- The `…032 / …033 / …034 / …035` chain touches a **different** function
  (`read_public_menu_all_locales`, defined at `1780338982028_ssr_public_menu.ts`) / its callers.
- The product JSON object to extend is the `jsonb_build_object` at lines ~82–90 of
  `1780338982022` (8 keys: `id, name, description, price, available, image_key, attributes,
  modifier_groups`). The proposal's `|| CASE WHEN … '{}'` merge approach is correct; the
  **source citation is not.** Fix the reference before Phase 1 code, or the byte-identical gate
  will fail for a stupid reason. *(This is the one I'd most want corrected.)*

### 3.2 (Correctness) The grant-mirror precedent mirrors `orders`, not `products`
`phase1-implementation.md §A.4` says the DML grant-mirror "mirrors access-requests.ts 041
pattern" and the prose implies mirroring **`products`** grants. The real `1790000000041` mirrors
**`orders`** grants (verified: `WHERE table_name = 'orders'`). For `product_media` you almost
certainly *do* want to mirror `products` (the write-authority that owns products should own its
media) — which is fine — but say so explicitly and don't mis-cite 041 as doing that. The 041
header itself documents *why* mirroring exists (migration 015's operational-role lockdown is
"aspirational and may not be the live role"); echo that reasoning so the next reader understands
the grant-mirror is a deliberate robustness move, not magic.

### 3.3 (Strategic / aesthetic) `locations.plan` as a bare `text DEFAULT 'free'` is the seam's
**weakest joint** — it is the one place the "schema rich" discipline lapses. A free-text plan
column with no CHECK enum invites typo-drift (`'business'` vs `'Business'` vs `'biz'`) exactly
the way the design *elsewhere* prevents via the `product_media_kind` enum. If a plan vocabulary
is worth a column, it is worth an enum or a CHECK. Small, cheap, and it keeps the table honest.

### 3.4 (Strategic) Name the orphan-reconcile owner *now*, even though the sweep is Phase 2+
R3 (orphan R2 objects from aborted multi-frame uploads) is accepted-deferred — fine. But "best
effort, swept later" is exactly how small storage leaks become real bills. Phase 1 introduces no
uploads, so there are no orphans yet; just make the reconcile job a **named Phase-2 GO
requirement**, not a someday. Cheap to promise now, expensive to retrofit a habit later.

### 3.5 (Aesthetic / minor) The `down()` no-op is correct discipline, but add a one-line comment
in the actual migration explaining *why* forward-only is safe here (inert + NULL everywhere), the
way `1790000000041` does. Future-you reading a no-op `down()` deserves the reason in-place.

---

## 4. Steel-man of a rejected option

**Steel-man — Option B (Columns-on-`products`, `media jsonb`).** The proposal rejects B "on the
menu_version lever alone": any `products` jsonb write trips the generic bump trigger, so every
secondary-media edit busts the CF HTML cache. That rejection is *correct as stated* — but the
strongest version of B is not "dump media in `attributes`," it is **a dedicated `media jsonb`
column on `products` plus a column-scoped bump trigger** (`AFTER UPDATE OF` listing the
menu-relevant columns but **excluding** `media`), mirroring exactly how the existing
`trg_bump_menu_version_locations` already does column-scoped triggering (`AFTER UPDATE OF
default_locale, supported_locales` — verified in `1780338982021`). That precedent exists *in this
very repo*. With it, B keeps secondary-media edits from bumping the version **without a second
table**, and is genuinely simpler for the "one image, occasionally a video" common case:
- B's honest wins: zero new table, zero new RLS surface, zero grant-mirror, zero denormalised
  `location_id`, and the hot path reads a column it already reads patterns of.
- B's honest, *fatal* loss (why A still wins): per-media `available` toggles, `sort_order`
  reorder, per-row `bytes` budget accounting, and per-media RLS all become **app-enforced jsonb
  surgery** instead of DB-enforced columns. For a feature whose *entire roadmap* is "multiple
  media per product, reorderable, individually toggle-able, budget-capped," modelling that as a
  jsonb blob is the wrong data shape — you'd be hand-rolling a tiny relational engine inside a
  column. A is the right call **because the roadmap is multi-media-first**, not because B's
  trigger problem is unsolvable (it is solvable).

So: A is correct, but the proposal's *reason* for killing B is too narrow. The real reason is
data-shape fit to the multi-media roadmap, not the trigger lever. State the stronger reason —
it's more durable and it's honest about the column-scoped-trigger escape hatch B actually had.

---

## 5. One open question no one asked (for the human)

**The whole program rests on holding the R6 teardown gate at Phase 5 — the exact unbounded-
lifecycle failure class that just cost you a multi-hour outage. Phase 1 is safe precisely because
it carries none of that runtime. So: before you accept Phase 1, will you write down — now, in
ink, while the outage memory is fresh and no momentum is pushing — the explicit condition under
which you would STOP the program at Phase 4 and ship *zero* cinematic reveal? "Schema now,
runtime later" is only honest if "runtime *maybe never*" is a real, pre-committed option. If the
answer is "we'll always finish what we start," then the seam isn't a seam — it's a runway, and
you should price Phase 5's risk into the Phase-1 decision today.**

(Secondary, quieter: `locations.plan` is the first money-primitive entering your schema, arriving
through a *media* feature while your launch trigger is still "first real paid order." Is the media
program the right vehicle to make your first monetisation-model decision — or is `plan` a tier
gate you're inheriting here without having chosen it on its own merits?)

---

*Counsel is advisory. Nothing above blocks a conscious human. The factual corrections in §3.1–3.2
are the only items I'd insist reach the implementer before Phase-1 code, because they are wrong in
the spec, not merely sub-optimal. Everything else is offered, not imposed.*

---

## Round 2 — re-review of `resolution.md` + the shrunk `phase1-implementation.md`

**Verdict in one line:** All three of my non-blocking corrections are addressed; the shrink to
"migration + flag only" is strategically *healthier*, not a deferral of the same risk; and my open
question is correctly carried to the human GO. **Zero grounded ETHICAL-STOP remains.** One thing
still belongs to the human — and the resolution already routed it there.

### 1. My three corrections — disposition

**#1 (wrong-migration citation) — RESOLVED, and I was the one who was wrong.** The architect
defers the `read_public_menu` column-read to Phase 2 (where `primary_media_id` is actually
non-NULL and provable against real data), so in Phase 1 the function is **untouched** and
byte-identity is trivial. Before agreeing the dispute is "moot," I verified the underlying fact,
because if I had been right, deferral could have *hidden* a live error rather than dissolved it.
The fact is the opposite of what I claimed: `1790000000033_localize-modifiers.ts:25` *does*
`CREATE OR REPLACE FUNCTION public.read_public_menu(p_location_id_or_slug text, p_locale text DEFAULT '')`
— the same single-locale signature. `…022` is the *original*; `…033` is the *latest* redefinition.
My §3.1 claim ("the body lives **only** at `…022`; `…033` does not redefine that function") was
**factually wrong**. The architect's correction in C2 is right, and it was made without
point-scoring. **I retract §3.1.**
This makes deferral *honest, not evasive*, and on a second axis I missed: had Phase 1 hand-copied
~125 plpgsql lines, the correct source was `…033`, not the `…022` I cited — so the transcription
would have started from a stale body. Removing the copy entirely removes *both* the drift risk and
*my* mis-citation risk. Not hiding the problem — deleting the surface that carried it.

**#2 (grant-mirror cites the wrong precedent) — RESOLVED by removing the mirror.** C1 drops the
mirror loop wholesale and replaces it with explicit `REVOKE ALL … FROM anon, authenticated,
service_role` + role-guarded `GRANT SELECT … TO deliveryos_api_user`. My objection was "don't
mis-cite 041; say what you actually mirror." The resolution makes the question moot by mirroring
*nothing* and stating the grant intent literally in the DDL (`phase1-implementation.md §A.4`). This
is strictly more legible than what I asked for, and — bonus the Breaker earned — it also closes a
cross-tenant **read** exposure onto the Supabase Data API that the mirror loop would have created.
Better than my correction.

**#3 (`locations.plan` bare text) — RESOLVED exactly as advised.**
`ADD COLUMN … plan text NOT NULL DEFAULT 'free' CHECK (plan IN ('free','business'))`
(`phase1-implementation.md §A.6 / resolution #3`). A CHECK rather than an enum is the *better*
call than my "enum or CHECK" — the tier set can evolve without an enum-alter migration. The
weakest joint of the seam is now honest. Accepted.

### 2. Is the Phase-1 shrink strategically healthy, or just deferral of the same risk?

**Healthy. It is the cleaner expression of the doctrine I praised, not a dodge.** The test for
"honest defer vs. risk-laundering" is: *does the deferred work change shape, gain a real gate, or
just move down the calendar unchanged?* Both deferrals pass:
- **`read_public_menu` column-read → Phase 2:** in Phase 1 `primary_media_id` is *always* NULL,
  so the merge provably adds nothing. Moving it to Phase 2 is not "later, same risk" — it is
  "when real non-NULL data exists, the byte-identity property becomes *testable against that
  data* instead of asserted." The risk doesn't move; it *acquires a gate it cannot have today.*
- **Client `MediaRenderer` registry → Phase 2:** introducing it now mutates `MenuPage.tsx`
  (99.6th-%ile churn, health 4.1) for **zero** user-visible change and with **no** natural
  pixel-identity gate. Deferring it to land *alongside the renderers that exercise it* means it
  arrives with a pixel-identity gate that is meaningful. Touching a fragile hot file for zero
  behaviour is precisely the over-engineering the Ponytail discipline forbids; pulling it out of
  Phase 1 is *removing* speculative churn, not postponing it.

So "schema now, runtime later" stays honest because what remains in Phase 1 is **only** the
expensive-to-change, genuinely-inert part (DDL + RLS + one nullable FK + a flag), and the parts
that were deferred each *gain* a verification gate by waiting. The shrink moved the program *toward*
the restraint I called its real elegance, not away from it. The denormalised `location_id`, the
RLS WITH CHECK, the idempotency hardening (H2), and the positive same-tenant-insert proof (H3) all
remain — the safety surface didn't shrink, only the speculative surface did. **This is the rare
shrink that lowers risk on every axis.**

One small note, non-blocking: H1/H4 are now Phase-2 **X-blockers** (the lazy endpoint must reflect
a `plan` flip without a stale cache, and must filter `available = true`). That routing is correct
— but X-blockers are promises, and promises decay. Keep them in the Phase-2 GO gate as *blocking*
items, not a notes section, for the same reason §3.4 wanted the orphan-reconcile owner named: a
deferred guarantee with no gate is how the cross-tenant draft leak the Breaker caught gets quietly
re-opened at the endpoint layer. The resolution already lists owners (apps/api) — good; just make
them gate, not memo.

### 3. Is the Phase-5 R6 STOP-condition raised enough for the human at STOP-DESIGN-B?

**Yes — it is correctly carried, and it is the *one* item that must reach the human, not the
agents.** `resolution.md` ("Open question → carried to STOP-DESIGN-B human GO") and
`phase1-implementation.md` (Phase-2/5 carry list: "R6 (Phase 5): pre-committed STOP-at-Phase-4
condition → human GO") both route my open question to the human decision, in writing, framed as
"record *now* the condition under which the program ships **zero** cinematic reveal." That is
exactly the ask, preserved without softening. The agents did **not** resolve it among themselves —
which is right: this is a values/strategy call about whether "runtime maybe never" is a genuine
pre-commitment or a fiction, and only the human can sign that. It is raised at the correct altitude.

I am **not** raising it to an ETHICAL-STOP. It does not cross a grounded red line *today* — Phase 1
carries none of the R6 runtime, so nothing is being shipped that needs the gate yet. It is a
pre-commitment the human should make *before momentum accrues*, which is precisely why it belongs at
the GO sign-off and not as a blocker on the migration.

### ETHICAL-STOPs (Round 2)

**None.** No grounded red line is crossed by the shrunk Phase 1. Re-checked: server-authoritative
(upheld — no runtime touched), a11y (no client code in Phase 1), zero-PII (none introduced),
"schema rich / runtime minimal" (the shrink is this doctrine made stricter), cross-tenant isolation
(strengthened by C1's REVOKE + the positive H3 proof). An ETHICAL-STOP here would be manufactured.

### The one thing left for the human — single question

> **Will you write down, now and in ink — before any Phase-2 momentum exists — the explicit,
> pre-committed condition under which this program STOPS at Phase 4 and ships *zero* cinematic
> reveal? If "runtime maybe never" is not a real option you can name today, then this seam is a
> runway, and Phase 5's unbounded-lifecycle risk (the outage's failure class) must be priced into
> the Phase-1 GO decision now, not deferred to a gate you've already decided to pass.**

Everything else is resolved. Corrections accepted (including my own retraction of §3.1). The shrink
is sound. Counsel is satisfied with Phase 1 as scoped, advisory-clear, and ready for the human GO.
