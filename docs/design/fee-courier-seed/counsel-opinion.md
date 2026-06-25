# Counsel Opinion — Delivery-Fee SoT · Courier-Status Honesty · Encrypted Dev-Seed

Reviewer: Counsel (Triadic Council) · Date: 2026-06-25 · Branch: `fix/design-system-consistency`
Authority: ADVISORY. Aesthetic/strategic notes are non-blocking. ETHICAL-STOPs below are friction
(paused-pending-recorded-human-decision), not vetoes. The human finalizes.

Verdict in one line: **all three items move the system TOWARD honesty, not away from it. No grounded
red line is crossed.** Two items carry a money-truth seam worth a recorded decision before merge; one
is a strategic-proportionality question, not an ethics one.

---

## 1 — Reasoning by lens (only what is load-bearing)

### Honesty / trust (Item 1 — fee)
This change exists *because the current UI lies about money* — the hardcoded 200 over-quotes by a full
fee exactly at the free-over-2000 boundary the venue used to incentivise the customer, and under-quotes
elsewhere. Fixing it is a clear good. The chosen A′ design is the honest shape: it refuses to fabricate
precision it cannot earn (distance-tiered venues degrade to "confirmed at checkout"), and it makes the
**server `total` the final authority**, so the replicated path can never overcharge — a mismatch
resolves *toward* the server, never away. That asymmetry is the ethical core and it is correct.

One honesty seam to name precisely (the question you asked): **is "fee confirmed at checkout" honest, or
a hidden price change?** It is honest *only if two things hold*:
(a) the degrade label is shown **before** the customer commits, in the same visual weight as the price —
not surfaced after Order is pressed; and
(b) the server `total` on the confirmation screen is shown **before any irreversible commitment / cash
handover**, with a visible delta if it differs from the estimate. If the customer can reach a state where
they've effectively agreed to pay and *then* sees a higher number, "confirmed at checkout" has become a
soft-confirm-as-trap — exactly the dark-pattern the red lines forbid. The proposal's design (CTA shows
`{subtotal}+`, muted line, server total on confirmation) satisfies this *if* the confirmation screen is a
real review step the customer passes through, not a post-hoc receipt. **This is the load-bearing UX
detail** — see ETHICAL-STOP-1.

### Money ethics — cash-on-delivery (Item 1)
For 77% cash-on-delivery, the shown number is what the courier physically collects at the door. The red
line "shown sum MUST equal collected sum" is the right frame. The plan **closes** today's gap for the
common flat-fee case (exact match). But it does **not** fully close it for the degrade/tiered case:
there, the customer is told `{subtotal}+`, the server computes the real total, and *whether the courier's
collection figure equals what the customer last saw* depends on whether the confirmation/status screen
clearly re-states the final total the courier will collect. The estimate path is honest about *being* an
estimate; the door-collection truth still rests on the confirmation screen being authoritative and seen.
This is not a flaw to block on — it's the same seam as the honesty point — but it is the place where "saw
≠ collected" could still occur if the confirmation step is weak. Grounded concern → ETHICAL-STOP-1.

A second money-truth detail, load-bearing: the proposal promises `clientTotal === serverTotal` to the
cent. I verified `apps/api/src/lib/money.ts:23` — `applyTax` is BigInt half-up, `taxRate===0 → 0`. The
promise holds **only if `applyTaxClient` mirrors it in integer/BigInt space, not float.** The proposal
says this; the parity guardrail (§1.6) is the thing that makes it true rather than asserted. The honest
move would be to *share the function* (`@deliveryos/domain`) rather than reimplement — two copies of
money math is a standing drift risk the proposal itself flags as R3. Non-blocking, but see advice.

### Honesty (Item 2 — courier status — your "other half-truth?" question)
Is **"N active"** more honest than "N online", or just a different half-truth? It is genuinely more
honest, *and* the proposal is careful about it: "N active" claims only what the endpoint can prove
(account enabled + onboarded), and it **explicitly stops claiming presence it cannot see**. That is the
right epistemic humility — the screen says exactly what it knows and no more. The phone-less/never-logged-
in courier is moved to a neutral "Pending setup" and *excluded* from the count — which removes the
specific dishonesty (inflated live-fleet metric) that misleads the owner's dispatch decision. The only
residual: "Active" could still be *read* by an owner as "available right now". That's a labelling risk,
not a lie — mitigated by removing the green dot. Acceptable. Real presence is correctly deferred to
Option B (live map already carries truth). No red line.

### Care / harm (who gets hurt by a failure)
- Item 1 failure → customer charged more than shown at a cash door → conflict the *courier* absorbs on
  the doorstep (the courier carries the cost of a pricing bug they didn't cause — a dignity concern, the
  courier is the human at the sharp end). The server-authoritative-total + divergence counter mitigate.
- Item 2 failure → owner dispatches a "live" courier who isn't there → a real order stalls. The fix
  *reduces* this harm. Good.
- Item 3 failure → dev-only; blast radius is a screenshot. The only harm vector is PII/prod-leak (below).

### PII / synthetic data (Item 3 — your specific question)
Two sub-questions: (a) does running synthetic PII through the **real** cipher pipeline create a fake
courier that looks real / pollutes `email_hash` space? (b) prod-leak risk?

(a) Low and bounded. The synthetic identity uses a `*.test` TLD, fixed constants, and a `.test` email
whose `sha256` lands in the same `email_hash` keyspace as real couriers — but `email_hash` collision with
a *real* courier is cryptographically negligible, and the row is tenant-scoped to the visual venue. The
one genuine concern is **namespace hygiene**: a persisted synthetic courier in a shared staging DB can
show up in owner-facing lists / counts (it has `status='active'`, `last_login_at=now()`) — i.e. Item 3's
fixture could *re-introduce* Item 2's dishonesty (a fake "active" courier inflating a real owner's view)
if staging is ever used for demo/owner walkthroughs. Not a red line; a hygiene flag — see advice.
(b) Prod-leak is well-defended: the code lives inside the existing `seedVisualHandler` behind the
ADR-0003 dev gate (404 unless `ALLOW_DEV_LOGIN==='true'` AND `x-dev-auth-secret` present, fails closed),
adds zero new prod surface, and memory confirms prod has the gate dark. The proposal's prod-safety E2E
(404-without-secret) is the right proof. No ETHICAL-STOP on PII-leak — the red line "zero-PII-in-AI / no
real PII" is respected (synthetic only). The pipeline reuse is *correct*, not risky: it proves the real
crypto path works, which is a quiet good.

### Aesthetic / conceptual integrity
Strong. The unifying concept across all three is **"say only what you can prove, defer what you can't"**
— the same discipline applies to the fee (replicate where deterministic, estimate where not), the
courier (claim account-state, not presence), and the seed (synthetic, gated). That is real conceptual
coherence, not three unrelated patches sharing a branch. "Schema rich, runtime minimal" is honoured:
no migration, additive fields, FE-derived display. The bundling is justified by a shared verification
surface, but see strategy note on Item 3.

### Long horizon / strategy
- Item 1 + 2 are reversibility-friendly (one-line FE reverts, additive contract fields, kill-switch).
- The standing 2nd-order risk is Item 1's **two copies of money math** — every future server change
  (e.g. `discountTotal` going nonzero) silently re-opens the divergence unless the guardrail catches it.
  The guardrail is the load-bearing safety; shared code would be the structural fix. A year from now we
  could regret the duplicated formula, not the feature.
- Item 3 grows the dev-seed surface for a single 390px screenshot — the proportionality question you
  flagged. Addressed in steel-man below.

---

## 2 — ETHICAL-STOPs (grounded red lines only)

**ETHICAL-STOP-1 — "shown ≠ collected" at the cash door (red line: shown sum MUST equal collected sum;
soft-confirm-must-not-be-a-trap; server-authoritative).**
*Grounding:* 77% cash-on-delivery; the displayed number is what the courier physically collects. The A′
design is honest *only if* the customer sees the authoritative server `total` **before** any commitment,
in a real review step — not as a post-Order receipt. If the degrade/estimate path or the
confirmation-screen ordering lets a customer commit at `{subtotal}+` and discover a higher collected
figure only at/after the door, "fee confirmed at checkout" becomes a soft-confirm trap.
*What this STOP asks (friction, not veto):* record a human decision confirming that the confirmation/
review screen shows the server `total` **before** commitment and that the figure shown there equals the
courier's collection amount. This is satisfiable by the proposal's own design — it just needs to be an
explicit, recorded acceptance criterion + an E2E assertion, not an implicit assumption. Once recorded,
the STOP clears. The conscious human may proceed regardless; this does not block.

*(No ETHICAL-STOP on Item 2 — it strictly reduces dishonesty. No ETHICAL-STOP on Item 3 — synthetic PII +
dark prod gate respect the PII red lines; the namespace-hygiene point below is advisory, not a red line.)*

---

## 3 — Non-blocking advice (aesthetic / strategic / process)

- **A1 (money-math, structural).** Prefer *sharing* the tax/fee function via `@deliveryos/domain` over a
  reimplemented `applyTaxClient`. One source of truth beats a parity test guarding two copies — the test
  catches drift *after* it's written; shared code prevents it. If sharing is infeasible (bundle/SSR
  constraints), keep the parity guardrail as proposed but treat it as a permanent gate, never delete-able.
- **A2 (UX honesty detail).** Make the degrade label carry equal visual weight to the price and name the
  *reason* ("delivery fee depends on your address — confirmed at checkout"), not a bare "confirmed at
  checkout". An honest UI explains *why* it can't be exact; that is both better aesthetics and better
  ethics.
- **A3 (door-truth surfacing).** Consider showing the final server `total` on the courier delivery screen
  itself ("collect: X") so the human at the door reads the same authoritative number the customer agreed
  to — closes the saw≠collected loop at both ends. (Item 3's live courier screen makes this testable now.)
- **A4 (dev-seed hygiene).** Give the synthetic courier an unmistakable display marker (name already
  "Visual Net Courier" — good; keep it) and consider a `is_synthetic`/source tag or a documented note so
  no one mistakes it for a real courier in a staging owner-walkthrough. Low effort, prevents Item 3 from
  re-introducing Item 2's exact dishonesty in demo contexts.
- **A5 (Item 2 wording).** "N active" is honest but still nudge-able toward "available now". A label like
  "N enabled" or a tooltip ("account active — see live map for who's on shift") would be maximally honest.
  Non-blocking; "N active" is acceptable as proposed.

---

## 4 — Steel-man of a rejected option

**Steel-man: "Do NOT build Item 3 — leave the live courier delivery screen as a documented visual gap."**
(The proposal rejected this by adopting Option A; this is the honest case *for* the rejection.)

The strongest version: Item 3 spends real engineering — three UPSERTs, an `argon2` hash, a transaction
with RLS-context juggling, a new `body.courierId` impersonation path on `mock-auth`, and a permanent
synthetic courier in staging — *to make one 390px screenshot render live*. Every new branch in a dev-seed
is surface that can drift, break, or (worst case) be the thing that's wrong the day someone weakens the
prod gate. The `mock-auth` change specifically *broadens impersonation* (sign a token for an
arbitrary `body.courierId`) — a capability that, while dev-gated, is exactly the shape of the dev-login
backdoor this project already got burned by once (see memory: dev-login-backdoor CRITICAL). The
alternative — a static mock / fixture-rendered screenshot, or simply documenting "courier delivery screen
not in the live visual net, covered by component tests" — costs almost nothing, adds zero prod-adjacent
surface, and the visual net's *purpose* (catch layout regressions) is largely served by a deterministic
mock render. The screenshot fidelity gained by a *real* encrypted courier is marginal; the surface added
is permanent. **This steel-man is genuinely strong on proportionality**, and I'd weight it more than the
proposal does.

Why the proposal can still reasonably win: a *real* live render exercises the actual courier auth + crypto
+ RLS path, so the screenshot proves the screen works end-to-end, not just that its CSS lays out — a
qualitatively stronger signal than a mock. And reusing the proven `encryptPII`/`argon2`/`email_hash`
pattern verbatim inherits its correctness. The deciding question is whether that end-to-end signal is
worth the permanent `mock-auth` impersonation broadening — which leads to the open question.

---

## 5 — The question no one asked

**The two items are framed as "make the UI tell the truth." But the truth they surface is only as
authoritative as the *one screen* that carries it — and no one has asked: what does the customer / courier
see at the moment of the cash handover itself?** The whole design routes truth to "the server `total` on
the confirmation screen." Yet the actual moment of money-truth in a 77%-cash business is the doorstep: a
human hands physical cash to another human. We've made checkout honest and the admin count honest — but
have we ever looked at whether the *courier's* screen and the *customer's* last-seen number are the same
number at the door? If they can diverge there, we've moved the lie downstream instead of removing it.
That door-handover parity — not the checkout CTA — is where "shown == collected" is finally true or false,
and it's the one screen this proposal touches only incidentally (via Item 3's now-live courier view). It
deserves to be the *primary* acceptance criterion, not a side effect.

*Secondary, for the health-pass ledger:* this branch is named `fix/design-system-consistency` but carries
two money/state-machine changes and a test-infra extension — none of which are design-system work. Minor
scope/naming drift worth noting so the decision-log reflects what actually shipped.

---

## RE-EXAMINE round 2 — post-human-decision (Date: 2026-06-25)

Re-read against revised `proposal.md`, `resolution.md`, and `ethical-decisions.md` after the human
decided: Item 3 = build-hardened-(b) with all four constraints; cash-parity = proceed (door-handover as
acceptance criterion); branch stays. Three checks below; verdict last.

### Check 1 — Does the RESOLUTION actually clear ETHICAL-STOP-1, with NO new hole?

**Yes — and the design holds the cleared STOP.** The red line was "shown == collected at the cash door."
The resolution (C1) does the structurally-correct thing: it does not try to reconcile *after* commit
(impossible for cash), it **inverts the flow** to `estimate-hint → review server total → confirm cash →
submit`. I traced each previously-suspected hole and each is closed by construction, not by hope:

- **Degrade-mode hole (closed).** When `/info` fails, fields are absent, or `has_distance_tiers` is
  true-or-unknown, the CTA degrades to `Porosit • {subtotal}+`. But the cash `min`, the red-border
  threshold, change-due, and the door figure are **all keyed to the server-authoritative `total`** read
  at the review step (proposal §1.4, §1.6 AC-CASH-PARITY, §1.7), never to `estTotal`. So in degrade the
  customer still confirms cash against the *server* number — the estimate is provably never the collected
  sum. The fail-safe direction (ambiguity → degrade → review server total) means the unsafe
  "claim-exact-then-charge-more" path is eliminated, not merely guarded.
- **422-loop hole (closed).** `CASH_AMOUNT_TOO_LOW` (orders.ts:570) now has a designed re-prompt that
  re-shows the *updated* server total and re-blocks submit until cash ≥ that total (proposal §1.4 / §1.7
  AC-CASH-422). There is no longer a window where the customer sees one figure and submits a lower one
  into a cold generic failure; the re-prompt is itself the new authoritative review. Because the review
  step already keys cash to the server total, this only fires on the narrow stale-window race — it is
  defence-in-depth, not the primary mechanism. Good: no single point carries the safety.
- **Kill-switch hole (closed).** `CHECKOUT_FEE_REPLICATION` now toggles *only* the cosmetic pre-review
  CTA hint; both flag states collect the server-reviewed total (resolution L2). Neither flag state can
  mis-collect. The risky property the flag used to guard no longer exists.

**Is door-handover parity PRIMARY, not secondary?** Confirmed. `ethical-decisions.md` records it
verbatim: *"Door-handover parity is the PRIMARY acceptance criterion, enforced by a red→green parity
guardrail + a door-handover E2E. No ship without it."* The E2E (proposal §1.6) asserts the *courier
delivery screen renders the same `collect: {total}`* as the customer's reviewed figure — i.e. it tests
the doorstep, the actual moment of money-truth I flagged in §5, not just the checkout CTA. This is
exactly the inversion I asked for in §5/A3: the parity criterion now lives at the door, where "shown ==
collected" is finally true or false, rather than at the CTA where it only *looks* true. **My §5 question
is answered and promoted to the primary gate.** ETHICAL-STOP-1 is genuinely cleared with a recorded human
"proceed," and the design holds it. I am not re-raising it.

One residual I name as non-blocking, not a re-raise: C1 pt.2 leaves the *mechanism* of the server-total
read open ("an order preflight that runs `orders.ts` fee math without persisting, OR the existing
soft-confirm/hard-block preflight on `POST /orders`"). Either is fine for the red line (both read the live
server total before commit). I flag only that the chosen mechanism must itself be **read-only before
commit** — a preflight that creates/holds an order is fine; a preflight that has a side-effect the
customer can't reverse if they then decline the cash figure would be a small new soft-confirm seam. The
proposal's framing (estimate-hint → *review* → confirm → submit) reads as review-before-create, which is
correct. Implementation detail for the builder, not an ethics gate. (See new question Q1 below.)

### Check 2 — Do the four constraints make Item 3 residual risk acceptable? Is my A4 namespace-hygiene threat closed?

**The four constraints close the impersonation + collision threats — the dev-login-backdoor-shape is
genuinely defused.** Synthetic-only mint (L1: arbitrary `body.courierId` *removed*, token minted only
for the one server-derived synthetic id) reduces the capability from "impersonate any courier" to
"impersonate one synthetic fixture" — un-abusable even on a leaked staging gate. Namespaced non-email
sentinel hash (M4) makes `ON CONFLICT` provably unable to touch a real `email_hash`, and `.test`-TLD
rejection at registration is real defence-in-depth. Idempotent DELETE-before-insert (M3) and
synthetic-owned conflict targets (L3) close the re-run and order-repoint seams. The human accepted the
residual as owner. This is a clean, well-bounded (b). I do not re-raise the proportionality steel-man —
the human consciously weighed it and chose the end-to-end render signal; that is their call to make and
it is recorded.

**But my A4 threat is only PARTIALLY closed — and this is the one thing I want on the record.** A4 was
never about impersonation or collision (those are the four constraints' domain). A4 is a distinct
*honesty* threat: the synthetic courier persists in the **shared staging DB** with `status='active'` and
`last_login_at=now()`, so it is a real `couriers` row that the **Item-2-honest** owner-facing list will
count inside "**N active**". The resolution's response (R6 + resolution §Counsel-A4: "Visual Net Courier"
display name + a documented note) closes the threat **for a human reading the list** — a person won't
mistake the named fixture for a real courier. It does **not** close it *programmatically*: the synthetic
row is still counted in the very "N active" metric Item 2 just made honest, so on a staging
owner-walkthrough or any staging-as-demo, Item 3 re-introduces a small version of exactly the dishonesty
Item 2 removed (an inflated "active" count). This is the steel-man-able irony: two items in the same PR,
one removing a fake-active inflation, the other re-adding one in the test fixture's blast radius.

This is **not** a red line and **not** an ETHICAL-STOP — staging is not production, no real owner's money
or dispatch decision rides on it, and the named marker prevents human confusion. But the resolution
should not claim A4 is *closed* when it is *mitigated-for-humans-only*. The clean structural fix is cheap
and worth recommending (non-blocking): either (i) an `is_synthetic` boolean on `couriers` that the
owner-facing count/list filters out, or (ii) exclude the namespaced-sentinel `email_hash` from the "N
active" aggregation, or (iii) accept it explicitly with the reasoning "staging count may include 1
synthetic active courier; never surfaced in prod (dev-gate dark)." Any of the three honestly closes the
loop; today's resolution does (iii) implicitly without saying so. **Recommendation: state it as an
explicit accepted-risk with owner, or take the one-line `is_synthetic` filter.** (See new question Q2.)

### Check 3 — New ethical questions introduced by the fixes themselves?

The fixes are sound, but three small new seams appeared that did not exist before the revision. None is a
red line; all are non-blocking, surfaced so they are not discovered later as surprises:

- **Q1 — the preflight's reversibility (from C1's inversion).** The new before-commit review reads the
  server total via "a preflight that runs `orders.ts` fee math, OR the existing `POST /orders` preflight."
  The inversion is the right fix, but it *introduces a new screen the customer passes through* — and any
  new mandatory step is a new place a dark-pattern *could* live (pre-checked tip, a default cash amount
  nudged high, an "agree" framing). The fix is ethically clean *as designed*; the new question is only:
  **ensure the review step is read-only and neutral — it presents the server total and asks for cash, it
  does not pre-commit anything or pre-fill a number the customer must un-choose.** Builder-level, but it
  is genuinely new surface that the original (broken) flow did not have.

- **Q2 — the synthetic-active-courier honesty seam (from Item 3(b), detailed in Check 2).** The fix to
  make Item 3 un-abusable (persist a real-looking synthetic courier) is precisely what creates the small
  Item-2-honesty regression in staging counts. Worth an explicit accepted-risk line, not silent.

- **Q3 — `last_login_at` stamping spreads a fix into a hot auth path (from H4 follow-up).** The H4
  resolution correctly drops the unprovable "Pending setup" *now*, but its deferred Option-B prerequisite
  is to **start stamping `last_login_at` at invite-redeem + refresh-rotation** (auth.ts). That is the
  right fix, but it touches the courier auth/session path (a red-line glob: auth) for a *display* feature.
  When that follow-up lands it deserves its own care-pass — stamping on refresh-rotation means every token
  refresh writes the row; verify it cannot become a write-amplification or a timing signal, and that it
  is gated behind Option B actually shipping (don't stamp a column nothing reads yet — that is latent
  surface). Non-blocking, deferred, flagged so the follow-up doesn't slip a money/auth-adjacent write in
  unreviewed.

### RE-EXAMINE verdict

**The council may exit. No standing ETHICAL-STOP.** ETHICAL-STOP-1 is cleared with a recorded human
"proceed," and the hardened design *holds* it (Check 1: degrade, 422-loop, and kill-switch holes are all
closed by construction; door-handover parity is the recorded PRIMARY criterion with a doorstep E2E — my
§5 question is answered and promoted to the primary gate). Item 3(b) is consciously chosen by the human
with all four constraints recorded; the backdoor shape is defused (Check 2).

Carried forward as **non-blocking** (not a re-raise of any STOP):
1. **A4 is mitigated-for-humans, not structurally closed** — the synthetic active courier is still
   counted in Item-2's honest "N active" on staging. Recommend an explicit accepted-risk line *or* a
   one-line `is_synthetic` filter (Check 2 / Q2). The resolution should say which, not imply "closed."
2. **The new before-commit review step is new dark-pattern-capable surface** — keep it read-only and
   neutral; no pre-filled/pre-committed values (Q1).
3. **The deferred `last_login_at` stamping touches the auth red-line glob for a display feature** — give
   it its own care-pass when it lands, and don't stamp a column nothing reads yet (Q3).
4. Standing from round 1: prefer *shared* `@deliveryos/domain` money math over a mirrored copy (A1);
   branch-name/scope drift is acknowledged in the decision log (§5 secondary) — the human chose to keep
   the branch, which is recorded and fine.

The human is final. These are advisory. The design is honest, coherent, and serves the launch trigger
without polishing past it. Counsel exits.
