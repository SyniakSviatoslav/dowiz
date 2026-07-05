# Counsel Opinion — Audit-fix: cross-tenant authz + the data-access seam

Role: Counsel (Philosopher/Physician) · Triadic Council STEP 2 — EXAMINE.
Authority: **ADVISORY**. Aesthetics/strategy are non-blocking. One ETHICAL-STOP below is **friction**
(pauses the council, requires a *recorded human decision*), **not a veto** — it does not override a
conscious human and does not block forever. Human decides.
Inputs verified first-hand: `proposal.md`, `AUDIT-SYNTHESIS-2026-07-03.md`,
`lib/anonymizer/index.ts:114-222`, `routes/owner/gdpr.ts:40-88`, red-line register
(`DeliveryOS-Context-Handoff-v4_5.md §7`).

This proposal *fixes* red-line violations; it is aligned with the register, not in tension with it.
The examination is therefore about what the plan does around and ahead of the code — not whether to ship it.

---

## 1. Reasoning by lens (only what is load-bearing)

### Justice / stakeholders
The fix redistributes protection toward the party with the least power. LC5's victim is not tenant B
the *owner* — it is tenant B's **customer** (the diner whose `name`/`phone` a stranger can null), and in
F3 the **courier** (whose decrypted roster a *customer* can read, whom a *co-worker* can deactivate).
Neither ever consented to a shared blast radius; neither can detect the harm. The plan is just in
direction. The one distributional gap: the plan reasons entirely about **owner-vs-owner** isolation and
liability. The data subject — customer and courier — has no seat. That absence recurs in §5 (open question).

### Dignity / autonomy (courier)
F3's most human edge is buried in the proposal as "secondary (defense-in-depth)": a courier working for
tenants A+B, deactivated by A, loses B too. That is a stranger revoking a worker's livelihood across an
employer boundary the worker never crossed. Scoping the mutation to `courier_locations` restores courier
agency — this is not a defense-in-depth also-ran, it is the dignity fix. Recommend it earn its own proof,
not ride along.

### Honesty / consent — the 200→404/403 question
Silently tightening is the **honest** move, not a dark pattern: the prior `200` was itself the lie — the
server falsely authorized a cross-tenant act. Returning `404`/`403` makes the server authoritative and the
UI truthful (both red lines). No *legitimate* same-tenant caller is harmed (they still get `200`). Two
honesty caveats, both non-blocking: (a) the `404` (existence-hiding) over `403` is the right privacy
choice and matches the GET sibling — keep it; (b) silence toward *machines* is not automatically fine —
the operator's own cross-tenant tooling (demo-seeding scripts, the `settlements /regenerate`
all-locations path already flagged F13, the promotions `500→401` already flagged for Breaker) could break
without a heads-up. A one-line ADR/changelog note of the status-code deltas prevents a future debugging
session from chasing a "regression" that is actually the fix.

### Care / harm — where the real weight sits
LC5 is **irreversible** and the recovery net is **down** (LC7: backup/DR inoperable end-to-end). If it was
exploited, a real tenant's customer PII is *gone* — not restorable. This ranks LC5 **above** LC2 by harm,
even though the lane rated it HIGH (the proposal already notes "CRITICAL-impact"; the synthesis lists it
LC5, PII+authz). Two consequences the plan does not draw:
1. **Sequence LC5 first**, not LC2. Irreversibility + no backup = highest-harm-first.
2. The forensic question "was it exploited?" is **not optional hygiene — it is the only way to learn whether
   unrecoverable harm already happened**, because there is no restore path to make anyone whole.
   And here is the sharp, grounded detail: the anonymizer stamps
   `locationId = options.subject?.locationId || row.location_id` (`index.ts:131`, mirrored `:208`) into the
   audit log. Pre-fix, a cross-tenant erasure logs the **attacker's** tenant (caller-supplied `subject`),
   while the victim row's true `location_id` sits in `row.location_id` unused for the stamp. So a
   cross-tenant erasure may be **recorded in the log as same-tenant** — the very trail you would use to
   answer "was this exploited?" may be blind to the exploit. Retrospective detection may be *genuinely
   unanswerable* from current logs. "We shipped the fix" is therefore **not** the same claim as "no customer
   was harmed."

### Long horizon / strategy — the sequencing crux
The proposal's own evidence *is* the argument against symptom-only fixing: `dashboard.ts:626` (scoped) vs
`orders.ts:862` (unscoped) — the same operation written twice, hardened once. Fix the 11 without an
enforced seam and the **next clone re-introduces LC2** — the exact mechanism that produced it. The plan
gets this right by binding the point-fixes and the lint into the **same Tier-1 batch** (§8). Endorse
strongly. The strategic *failure mode to guard against* is human, not technical: under time pressure,
shipping the Tier-1 point-fixes and quietly deferring "the lint" to a next batch. The lint is not
separable — it is the class-killer; deferring it recreates the bug. This must be named so the human
decides it deliberately, not by omission. On the launch trigger (first real paid order): this batch is a
genuine multi-tenant launch-blocker, not polish — you cannot ethically onboard real tenant #2 onto a
system where tenant A can erase tenant B's customers. Well-aligned with the trigger; low regret at a
year's distance *if* the seam ships with the symptoms.

### Aesthetics / conceptual integrity
"The JOIN **is** the tenant boundary" is honest and coherent — authorization folded into the read, not
bolted beside it, and already proven at `dashboard.ts:626` and the GET sibling. Bringing the second copy
up to the first is coherence-restoring, not invention; fewer ways to do one thing → fewer bugs → less
harm (the aesthetics-as-leading-indicator claim holds here literally: the *duplication* was the bug).
Choosing lint-first (B) over big-bang repos (A) honors "schema rich, runtime ruthlessly minimal" — the
minimal enforcing mechanism that closes the class, reusing the existing `-- no-location-id` escape-hatch
convention (design-language continuity). This is restraint, not over-engineering. The over-engineering
risk lives in Option A's big-bang, which the proposal correctly defers.

### Epistemic — the carrying, unexamined assumption
The load-bearing claim is §5's: "the entire live tenant-IDOR surface is the ~11 owner-plane sites + the
anonymizer sink — a small, enumerable set," resting on the belief the ~250-site tail is scoped. The
proposal's own answer to that belief is the lint ("converts 'we believe' into 'the build proves it'").
But note the circularity: **the assumption and its mitigation are the same mechanism.** The lint only
converts belief→proof if the lint's *heuristic* is sound — and the proposal admits it is heuristic
(SQL-in-template parsing, "false-negatives on dynamically-built SQL," §4B cons). A dynamically-concatenated
tenant query evades the parser and is *neither* caught by the lint *nor* covered by the manual sweep's
"scoped" assumption. If the heuristic is weak, belief and proof fail *together*. This is not a reason to
reject B — it is a reason to **adversarially test the lint** (can a concatenated/interpolated tenant query
slip past it?) before trusting it as the gate, and to *bound and hand the residual false-negative surface*
to Option A's priority queue rather than treat it as covered.

---

## 2. ETHICAL-STOP (1) — friction, not veto

**STOP-1 — LC5 is a live, exploitable, IRREVERSIBLE cross-tenant PII-erasure with no recovery net; the
plan may not ship it as "just a code fix" without a recorded human decision on forensics + disclosure.**

- **Grounded red line(s):** PII governance / "анонімізує-не-видаляє" (§7 anonymizer) + "крос-tenant=0"
  (§7 hardening). The intersection is not the *fix* (which honors both) — it is the plan's **silence** on
  the live exploitation window on an irreversible-harm surface.
- **Why this is a STOP and not advice:** §7 (contract impact) frames the whole batch as "corrections of a
  leak, not breaking changes" — a purely engineering framing. An active, exploitable, *unrecoverable*
  cross-tenant PII-erasure in a live multi-tenant system carries a duty **beyond code**: (a) determine
  whether it was exploited, and (b) if a real third-party tenant's customer data was affected, decide the
  disclosure/notification duty. The plan answers neither; it answers "fix it." On a PII red line, that
  silence is exactly where friction belongs.
- **What the STOP requires (a recorded decision, then proceed):** the operator records one of —
  1. "Checked the audit trail for cross-tenant erasure/mutation in the exposure window; found none" — **and
     notes whether the trail can even answer this** (see §1 Care: the `locationId` stamp may record the
     attacker's tenant, so 'found none' may mean 'blind,' not 'clean'); **or**
  2. "Found evidence / cannot rule it out; here is the tenant/customer disclosure plan."
- **Proportionality anchor (do not over-fire):** the disclosure *duty* scales with the **actual** tenant
  population. If today's tenants are only operator-seeded claimable demos (per memory: Dubin & Sushi,
  Eljo's, artepasta) and the first real independent paid tenant is not yet live, then external disclosure
  is near-vacuous *now* — but the **log-check is cheap and honest regardless**, and the duty becomes
  **binding at real tenant #2**. So the recorded decision must also state: is the log-check/notification
  capability in place *before* multi-tenant onboarding, or does it become a hole the moment a second real
  tenant exists? This is the strategic edge of the same STOP.
- **Not a veto:** a conscious human may record "proceed, demos only, log-check clean/blind-but-accepted"
  and the council continues. STOP-1 forces the question onto the record; it does not answer it.

LC2 (order-mutation IDOR) rides *within* the same log-check but does **not** merit its own STOP: it is
serious but broadly reversible (a wrongly-`CANCELLED` order is money-adjacent harm, not permanent data
destruction), so it carries a lighter forensic/disclosure weight than irreversible PII loss. Keeping
STOPs few and grounded is the discipline.

---

## 3. Non-blocking advice (aesthetic / strategic)

1. **Sequence LC5 first**, not LC2 — irreversibility + broken backup = highest-harm-first.
2. **Log the *attempt*, not just block it.** The fix makes a cross-tenant erasure/PATCH return `404` — but
   a `404`'d cross-tenant erasure attempt is a *signal*. Record it (actor-tenant ≠ subject-tenant) so
   *future* attempts are detectable even after the hole is closed. This also fixes the forensic blind spot
   in §1 (Care) going forward. Small, high-value.
3. **Changelog/ADR one-liner for the status-code deltas** (`200→404/403`, promotions `500→401`) so machine
   callers and future debuggers are not blindsided by an honest tightening.
4. **Adversarially test the lint heuristic** before trusting it as the gate — feed it a
   dynamically-concatenated tenant query; if it evades, bound the residual surface and hand it to Option
   A's priority queue rather than call the tail "proven."
5. **Track the escape-hatch count as a ratchet metric.** `-- tenant-scoped-ok: <reason>` is grep-auditable
   (good), but its *growth over time* is the real erosion signal. Ratchet on violations **and** on
   escape-hatch count; a rising hatch count means the seam is quietly leaking.
6. **Elevate the F3 shared-courier scope fix** out of "defense-in-depth" — give it its own red→green proof
   (deactivate a courier in A, assert still-active in B). It is a courier-livelihood protection, not a
   footnote.
7. **Name the data subject** (customer, courier) as a stakeholder in the ADR — the batch currently reasons
   only owner-vs-owner.

---

## 4. Steel-man of the rejected option — Option A (repository layer), first not deferred

The proposal defers A in favor of B-first. The strongest case *for* A-first, stated at full strength:

B closes the class **by detection**; A closes it **by construction.** B's guarantee is only as strong as a
heuristic SQL-in-template parser — and the proposal itself concedes false-negatives on dynamically-built
SQL. A makes the tenant predicate **physically un-authorable**: handlers do not write tenant SQL at all,
so there is no template for a parser to miss and no "we believe the tail is clean" to take on faith — the
belief §5 rests on simply evaporates, because the ~250 tail sites would not author SQL. For a **red-line**
invariant (authz/PII), "structurally impossible to omit" is categorically better than "usually detected,"
and the cost of a single missed detection here is *irreversible customer-PII destruction*. A honest
sequencing under this view: extract the **highest-harm aggregates first** (Customer, then Order — exactly
where LC5/LC2 live), so the two irreversible/money surfaces become un-omittable *now*, and let B's lint be
the **backstop for the not-yet-migrated tail** rather than the primary gate. Under this framing, B-first
inverts the priority — it hardens the cheap detection layer before the expensive certainty layer, on the
surfaces that can least afford a miss.

**Why the proposal's counter still holds (but leaves a residue):** A-alone leaves *partial adoption* during
its multi-week migration — a new raw query can still appear until A is complete — which is the exact
present failure mode; so A **needs B anyway**, hence B-first as the immediate gate. That counter is sound.
The residue the steel-man leaves on the table, and which §3.4 above operationalizes: **B's heuristic
soundness is load-bearing and currently unproven**, so B-first is right *only if* the lint is adversarially
validated and its false-negative surface is explicitly routed into A's queue — otherwise the council has
swapped "provable isolation" for "cheap isolation" on a red line without saying so out loud.

---

## 5. One open question nobody asked

**When tenant B's customer PII is irreversibly erased by tenant A — or an order wrongly cancelled — who is
accountable *to the customer*, and does the customer ever find out?**

The entire batch adjudicates tenant-vs-tenant liability. Nobody has named the **data subject's recourse or
right-to-know.** The anonymizer already writes an audit log with `actorId`/`actorKind` — but it stamps the
*caller-supplied* `locationId`, so a cross-tenant erasure can be logged as if it were legitimate and
same-tenant. The record exists but may not be *tamper-evident against the actor* and may not distinguish
"tenant A erased tenant A's customer" from "tenant A erased tenant B's customer." Should the erasure path
carry a per-record, actor-vs-subject-tenant provenance ("erased by actor X, whose tenant is A, against a
subject in tenant B, at time T under request R") — so that a wrongful, irreversible erasure is at minimum
**attributable and disclosable** to the affected party, even when it can never be undone? On a surface
where harm is permanent and the backup is broken, *attributability* may be the only justice left to
offer — and it is the one thing the current plan does not yet guarantee.
