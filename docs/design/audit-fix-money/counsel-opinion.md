# Counsel Opinion — MONEY audit fixes (LC1 tax · LC6 refund black hole · settlement loss)

- **Status:** COUNSEL (advisory) — Triadic Council STEP 2 (examine). Non-blocking except the two ETHICAL-STOPs below, which are *friction requiring a recorded human decision*, not vetoes. The human decides; the live hotfix is never gated on this document.
- **Examined:** `proposal.md`, `ADR-audit-fix-money.md`, `audit-money-orders-2026-07-03.md`, `AUDIT-SYNTHESIS-2026-07-03.md`. Grounded against Context-Handoff v4.5 §7 red-lines + CLAUDE.md Ethics Charter.
- **Posture:** the engineering is strong and the architect surfaced (did not bury) the hard questions — §2.6, ESC-1, and open-Q4 explicitly hand restitution to counsel. This opinion answers forcefully; it does not accuse. Robustness/wedge-mechanism hunting is the Breaker's lane and is not duplicated here.

---

## 1. Reasoning by lenses (only what adds signal)

### Justice / whose cost, whose harm
- **LC1 fix-forward is right and urgent** — stopping a live 16.7% silent overcharge is unambiguous corrective justice for future customers. No hesitation.
- **The absent party is the courier.** LC1 and LC6 are customer-facing money; the **settlement loss is a LIVE harm to couriers** (real workers, own fleet — v4.5 §6), losing reconciliation cash *right now*, yet §7 sequences it **third, behind the dark LC6**. Sequencing by coupling/reviewability is defensible engineering, but the ethical reading is: a live harm to a worker is deprioritized behind dark (zero-harm-today) work on a customer surface. See §3 advice — this is the one sequencing smell.
- **Fault ≠ cost holder (the restitution knot).** The overcharge was *dowiz's* pricing-engine bug; the venue set a tax rate in good faith. But the extra money went to the *venue's* pocket (0% commission model → `total` flows to the venue). So the party at fault (platform) and the party holding the gain (venue) and the party harmed (customer) are three different actors. The proposal flattens this into "human/business decision"; the split is the whole ethical content and must reach the operator intact (ESC-1).

### Dignity / autonomy (courier)
- **The §3.5 `409 ASSIGNMENT_ACTIVE` is a dignity WIN, name it as such.** Today's silent-200-and-strand (H1) makes a real courier **permanently undispatchable** — excluded from all future work by an owner's mis-click. Replacing that with a 409 that redirects to `/deliver` protects the courier's livelihood and honors "кур'єр завжди завершує / cash-as-proof spine." This is a strict improvement even if `/deliver` has a redirect gap (a gap is a routing problem, not a new harm). Strong positive.

### Honesty / consent / no dark-pattern
- **Aesthetics-as-ethics, realized.** The mirror-oracle test (`expected = serverApplyTax(...)`) was **both ugly and unethical**: self-certifying AND the reason real customers were overcharged undetected ("no one can notice"). The independent-constant + definitional-invariant redesign (§2.5) plus the anti-recertification lint ratchet (§2.5, P12) fixes the *epistemic disease*, not just the symptom. This is the leading-indicator principle made concrete; the architect's instinct here is correct and worth preserving verbatim.
- **§3.4 webhook keeps flipping `paid` on terminal orders + records `refund_due` — honest.** Suppressing the flip would hide that funds arrived; recording payment + obligation is the truthful ledger. Endorsed.
- **The settlement backfill is truthful in total but potentially misleading in attribution** — see §4 (the honesty seam).

### Care / harm — proportionality of the fail-closed fold
- **Fail-closed is the ethically correct default.** An unrecorded refund debt = the customer's money silently kept = the exact harm we are repairing. Failing closed protects the wronged party. Agreed.
- **But friction without a human exit becomes a verdict.** If a poisoned/constraint-blocked `payment_events` row makes the `refund_due` INSERT fail *deterministically*, every attempt to cancel/reject that order 500s — a legitimate owner action (customer called to cancel; kitchen needs to reject) is wedged indefinitely with no override. That crosses "тертя-не-вирок / людина-завжди-завершує." Mitigating fact: this only fires on **paid crypto orders, which are dark today** — so it is a *flag-flip* concern, not a live one. Hence ESC-2 is armed-at-flip, not blocking. (The wedge *mechanism* is the Breaker's §8-Q1 hunt; the *value principle* — fail to a human, not to a freeze — is mine.)

### Long horizon / reversibility / strategy
- **Serves the launch trigger** (v4.5 §9 "перший реальний платний заказ"): you cannot honestly take a first paid order on a system that silently overcharges (LC1) or silently keeps refunds (LC6). This is not polish; it is a precondition for taking money at all. Correctly prioritized over feature work.
- **Reversibility:** forward-only `CREATE OR REPLACE` fns, no schema/column change, no `down()` — the change is code-reversible but its *data effect* (backfilled settlement rows) is not trivially un-created. That asymmetry is the reason §4 asks for a logged, legible, operator-aware first run.

### Epistemic — carrying assumption / absent perspective
- **Load-bearing unchecked assumption:** the proposal enumerates affected *orders* (§2.6 query) but never asks **where the overcharged money now sits** (venue revenue vs. over-remitted VAT vs. platform) — which determines *who owes* the restitution and whether a *tax-compliance* obligation exists, not merely a goodwill one. This is the open question (§5).
- **Absent perspective:** the **cash customer**. Inclusive-tax overcharge fired on cash orders too; those customers paid a courier and have no card to refund and often no contact. Any restitution decision must distinguish the *refundable* (crypto/card) from the *practically-unreachable* (cash) and be honest that the latter may only be repairable as future credit or disclosure, not money-back.

---

## 2. ETHICAL-STOPs (grounded red-line crossings only — friction, human decides, non-blocking of the live hotfix)

### ESC-1 — Restitution requires a *recorded* operator decision; "out of scope / if any" is not a resolution
- **Grounded line:** honesty / "UI каже правду" / no silent gain + людина-фінал (recorded decision) + Ethics Charter ("human wellbeing, dignity… never turned against the people it was learned from").
- **The crossing:** a *known, quantified* wrongful charge against *real, identifiable* customers, our own bug. Fix-forward discharges the duty **not to harm future customers**; it does **not** discharge the duty to **decide on repairing the past**. Framing restitution as "if any… out of scope" lets *silence default to keep-the-money* — a decision to retain an ill-gotten gain, made by nobody, recorded nowhere.
- **What the ESC demands (minimal, proportional):** NOT that anyone refund (auto-refunding would itself be reckless — see cash-unreachability, VAT entanglement). It demands that **the operator record an explicit, reasoned decision** — refund / partial / notify-only / documented-no-action-with-cause — **with an owner and a date**, before this remediation is considered *closed*. The forward hotfix ships immediately regardless; the ESC attaches to *closure*, never to *stopping the bleeding*.
- **Plurality behind the demand:** consequentialist (real welfare loss, quantifiable), deontological (no informed consent to a hidden charge → a duty of rectification to the specific parties), corrective justice (make whole those you can; be honest about those you cannot), care (the *venue* — not at fault — should not silently eat a platform bug's restitution bill; the platform authored the defect).
- **Enrichment the operator needs to decide well (attach to ESC-1):** (a) where the money went (§5); (b) refundable vs. cash-unreachable split; (c) that fault was the platform's, gain the venue's, harm the customer's — three actors; (d) a disclosure/comms stance for any venue that ran `tax_rate>0`.

### ESC-2 — The fail-closed refund fold must fail *to a human*, not to a permanent freeze (arms at the crypto flag-flip)
- **Grounded line:** "тертя-не-вирок" + "людина-завжди-завершує" + людина-фінал.
- **The crossing (conditional):** fail-closed with **no described human escape hatch** means a deterministically-failing `refund_due` insert can wedge a legitimate cancel/reject indefinitely — friction with no override is a verdict. As written, the design has no escape path.
- **What the ESC demands:** before the LC6/crypto flag-flip, the design must show a **human-escapable path** — a conscious operator, seeing a stuck order, can force the terminal transition **and** have the *unrecorded refund obligation surfaced as an explicit friction-alert* (the same "готівка → алерт-тертя власнику" shape), rather than the order being frozen. Fail-closed is right; fail-*silent-and-permanent* is not.
- **Scope discipline:** dark today (no paid payments exist), so this gates the *flip*, not the *build*. The wedge-mechanism enumeration is the Breaker's §8-Q1; I assert only the value floor.

*(Not escalated: the self-backfilling settlement deploy is **not** an ESC — it lands in `pending`, a human still pays, so "людина-фінал / нуль-автобану" holds. Its concern is honesty/legibility, handled as advice in §4.)*

---

## 3. Non-blocking advice (aesthetic · strategic)

- **Sequencing: put the two LIVE fixes ahead of the dark one.** Prefer **LC1 → settlement (M-2) → LC6 → Option B**, or at minimum run settlement in parallel rather than gated behind LC6. Rationale: the courier's live reconciliation loss is a present harm to a worker; LC6 is dark (zero harm until flip). Coupling justifies LC6-with-H1, but nothing couples *settlement* behind LC6. Do not let a dark customer-surface fix outrank a live worker-money fix.
- **Preserve the test-redesign framing as a first-class deliverable, not incidental.** §2.5's independent-oracle + demoted-parity + lint-ratchet is the part that stops this class from recurring. It is the elegant core. Ensure the demoted parity test's header truly forecloses re-promotion (a future reader must not be able to mistake drift-detection for correctness).
- **Name the §2.6-vs-§4.1 conceptual seam and resolve it in-text.** The proposal says "never retro-mutate money rows" for tax (§2.6) yet **auto-creates** historical settlement rows on deploy (§4.1). This *looks* contradictory. It is actually defensible — refund = money *out the door*, contested, irreversible → human; backfilled payout = surfacing an *already-existing* obligation into a *pending* (human-paid) state → not money-out. But the coherence is only defensible if stated. Add one line to the ADR distinguishing "auto-mutate money-out (never)" from "auto-surface an existing obligation into a human-gated pending queue (fine)."
- **Aesthetic endorsement:** A-then-B (hotfix, then consolidate) is disciplined restraint — do not let anyone collapse it into B-only. "Schema rich, runtime minimal" holds (no schema change, forward-only fns). This is the design-language the rest of the system should match.

---

## 4. The self-backfilling settlement deploy — honesty of retroactive records (task Q4)

**Substance: acceptable. Presentation: currently not honest enough.**

- **Why it's acceptable auto-on-deploy:** it creates **`pending`** payouts, not `paid` — a human still approves/pays every one (v4.5 nuль-автобану preserved). It surfaces obligations that *already exist* (courier delivered, collected cash, is owed), not new liabilities. It never mutates a `paid` payout (§4.1(2) immutability). So it is not "auto-mutating financial records" in the dangerous sense; it is making a true-but-hidden debt visible into a human-gated queue.
- **Why it is not yet *honest enough*:** the first post-deploy run sweeps **the entire accumulated backlog** (weeks of skipped/crashed-run losses — "a crashed 2 AM run loses an entire day per courier-location pair") into **one** next-period payout. A courier's payout labeled `period = today` may contain deliveries from a month ago; the operator may approve a **surprisingly large one-time number with no explanation of why it spiked**. The money is correct; the *attribution and the surprise* are not honest. The proposal itself concedes this (line 209 "display-level note only"; open-Q3 "unsettled backlog indicator").
- **Advice (non-blocking):**
  1. **Legibility:** mark caught-up items (or surface a per-payout "N caught-up deliveries from before <fix_date>, total X") so both operator and courier can see *why* a period is heavy. Reconciliation UX honesty, not cosmetics.
  2. **First-run heads-up:** before the first post-deploy cron run, give the operator the **expected recovery magnitude** (a read-only pre-count of `delivered + cash_collected + NOT EXISTS settlement_items`), so a large one-time catch-up is *approved knowingly*, not blind. This is the "operator-triggered vs auto" question answered: auto is fine *if* the operator is told the number first.
  3. **Courier legibility:** confirm a courier never *saw* a `pending` figure for a period, mentally banked it, and then watches it shift — the immutability guard protects `paid`, but `pending` numbers moving between periods can still confuse the person being paid. A one-line "includes earlier deliveries" note closes it.

---

## 5. Steel-man of a rejected option — the DB trigger for the refund fold

The proposal rejected a `orders.status` trigger (§3.2 / ADR alt-3) for: hidden control-flow, RLS/DEFINER complexity, no bus events, against the explicit-mutator pattern; and relegated it to "revisit only if a THIRD bypass appears." The strongest case *for* the trigger:

- **The invariant is a data-integrity rule, and the data layer is where it holds for *every* writer** — present, future, sanctioned, forgotten. The audit is a monument to the opposite: "the refund ledger has exactly one writer on a many-writer surface — every new cancel path silently reopens the hole." The app-fold + grep-gate is a **social/process** guarantee (someone must remember to route through `updateOrderStatus`; the next DEFINER-fn author must remember to add the fold; the grep pattern must keep working). The trigger is a **structural** guarantee — the invariant becomes *impossible* to violate.
- **The grep gate exempts exactly the class that caused the bug.** It bans `UPDATE orders SET status` outside the sanctioned mutator **plus the two DEFINER fns** — but the LC6 timeout-sweep bypass *lived in a DEFINER fn*. So the gate must allowlist the very path where the original leak occurred, and the proposal's remedy for that path is "add the fold to each DEFINER fn by hand" — the identical remember-it-everywhere model that failed the first time. A trigger needs no allowlist and no memory; it fires under the DEFINER fn too.
- **"Can't publish bus events" conflates two concerns.** Recording the *obligation* is a data invariant (trigger-appropriate); the owner refunds queue reads `payment_events` **directly** (`owner/refunds.ts:25-30`), not a bus subscription — so a trigger-written row is *sufficient* for owner visibility. Notifying the *customer* of a refund is workflow (app-appropriate) and can stay in the app layer. The objection optimizes reviewability at the cost of the exact durability property that failed.
- **"Hides control flow" is, for a data invariant, a feature** — you *want* it impossible for a caller to forget, not visible for every caller to re-reason about.

**Honest rebuttal (why the proposal's choice is still defensible):** the trigger is harder to unit-test in isolation (P4–P6 are clean against the app fold); a buggy forward-only trigger is riskier to hot-fix than app code; a trigger firing inside the timeout-sweep's fleet-wide transaction has less-obvious lock/perf behavior; and it introduces a paradigm the solo maintainer must now own. These are real.

**Synthesis (my actual recommendation, not either/or):** the proposal frames trigger *vs.* app-fold as mutually exclusive; they are **complementary**. For a money red-line, consider **defense-in-depth** — the app fold as the primary path (testable, bus-capable) with a **minimal SECURITY-DEFINER trigger as the structural floor** that guarantees the invariant even under a future forgotten writer. At the very least, do not defer the trigger to "if a *third* bypass appears" (reactive, and it took *two* to find the class) — record it as the standing structural backstop the moment a third writer is even proposed.

---

## 6. One open question no one asked

**Where is the overcharged tax money *now*, and does that make LC1 a tax-compliance exposure rather than a goodwill-refund question?**

The proposal enumerates affected *orders* but never traces the *destination* of the re-added tax. In an inclusive-VAT market like Albania, if any venue's VAT filing is driven off `orders.tax_total` or the inflated `orders.total`, the double-charge means venues may have **over-remitted VAT to the tax authority** — turning "should we refund customers as goodwill" into "we may have caused our venues to over-report and over-pay VAT," a *legal/compliance* obligation with a third party (the state) in the loop, not a discretionary gesture. Nobody in the chain — audit, synthesis, or architect — asked how `tax_total`/`total` feed any downstream tax reporting. That answer changes the restitution calculus entirely: goodwill is optional; compliance is not. It should be run down *before* the operator's ESC-1 decision, because it may remove the discretion the ESC currently grants.

---

*Counsel · Triadic Council STEP 2 · advisory · human is final · the two ESCs are recorded-decision friction, not vetoes; the LC1 live hotfix ships regardless of either.*
