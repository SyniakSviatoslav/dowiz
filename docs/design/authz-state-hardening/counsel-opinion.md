# Counsel Opinion — Authz / State Hardening Batch (B7 · N1 · N2 · N4 · N5)

**Role:** advisory. Aesthetics/strategy non-blocking. One ETHICAL-STOP below = friction (a paused-
council + recorded-human-decision requirement), **not** a veto, **not** a forever-block, and it does
**not** override a conscious human. The human is final.

**Verified before opining** (so this is grounded, not taste):
- `no_show_count` is consumed downstream (`compute.ts:92` → `evaluatePreflight`), so it is **not** a dead
  counter — but its *only* preflight effect is `code:'no_show_history', severity:'soft'`
  (`evaluatePreflight.ts:127-134`). `hard_block` is reserved exclusively for objective item-
  unavailability (`:104-106`). **There is no auto-deny path.** The strike can, at most, raise an
  *acknowledgeable* `soft_confirm` on a future order — and that prompt itself says "There were N
  no-show(s) in the past" (`:132`), so the subject is partially disclosed-to at point-of-friction.
- The system's own written contract: `customer_signals` is "Advisory… NEVER used for auto-block… Owner
  acknowledge/dismiss only" (`1780421100057:104`); `no_show_count` is "Advisory… Never used for
  auto-ban" (`1780421100053:14`). Both survive the code. The manual no-show mark, however, **bypasses
  `customer_signals` entirely** and writes the raw counter + an event (`signals.ts:224-250`).
- The architect's ground-truth self-correction (strike is location-scoped, NOT cross-tenant) is
  confirmed by schema (`1780310074262_orders.ts:8-16,74-77`). Credibility earned.

---

## 1. The four mechanical items — confirmed, no ethical fork

No grounded red line is crossed; each *reduces* harm. Brief lens notes only where load-bearing:

- **B7 (settlement scope).** Care + justice: the bug let one owner re-write *other tenants'* courier
  payouts and self-inflict a ~100k-lock DoS on the order hot-path. The fix makes runtime honor the
  route boundary `requireLocationAccess` already proved. Attribution (`actor_kind='owner'`) is a
  dignity *plus* — manual money-touching actions become accountable. Confirm. The steel-man for B7-C
  (async job) is correctly deferred and kept schema-ready — that *is* "схема багата, рантайм
  мінімальний" restraint, well applied.
- **N1 (customer BOLA).** Care + honesty: a 7-day bearer token reading/cancelling/messaging *any*
  co-`customer_id` order is a real harm to real people (impersonation, griefing, fake reputation). 404-
  not-403 is the honest posture (don't leak existence). Confirm. The forward-constraint in §3.8 (a
  future cross-order view must be a *separate* account-scoped token, never a gate relaxation) is the
  right line — hold it in the ADR.
- **N2 (RLS-GUC).** Correctness; broke *closed* (no leak today). The "never `set_config` an RLS GUC to
  `undefined`" guard is the right ratchet. Confirm.
- **N4 (money-blindness).** Honesty + care: showing an owner "nothing owed" when the truth is "couldn't
  read it" risks under-paying a courier — the UI lying about money. Fail-loud is correct; the N4-C
  refinement (one bad PII blob masks the name but never hides the amount) is elegant — it separates "a
  bad blob" from "can't read the money." Confirm. One inline-fix flag stands: §5.5 — the owner
  settlements page **must** render a real error state on 500, or the 500 just becomes a different
  silence. Verify the FE state exists.

---

## 2. N5-6a — confirmed charter-safe, ship now

6a (no-show requires a real dispatch attempt; double-submit → 409) is unambiguously correct and
**reduces** dignity harm: it removes the pure false-positive — striking a customer who never had a
delivery attempted, so had nothing to "not show" for. This is the cleanest record-don't-judge
improvement in the batch. Ship it independently. No gate.

---

## 3. N5-6b — ETHICAL-STOP (one, narrow, grounded). Friction, not veto.

**ETHICAL-STOP-N5b.** *Grounded line: "record, don't judge" + the system's own written invariant.*

The grounding is not my taste and not "subject-invisibility" in the abstract. It is concrete: an owner
presses a button that increments a *consumed* reputation counter against a named person, while that
write is **(a) unattributed** (no record of who marked it or why) and **(b) not reversible through the
normal acknowledge/dismiss path** — because it bypasses `customer_signals`, the very table whose
contract says "Owner acknowledge/dismiss only." So today the system **contradicts its own written
dignity contract**: it judges (writes a consequence-bearing strike) without recording the judgment as
attributable and reversible. That inversion — not the existence of the feature — is the grounded line.

This STOP does **not** block 6a, does **not** block the batch, and does **not** forbid the feature. It
asserts: **N5 may not be considered _closed_ on 6a alone.** The unattributed-strike gap requires the
recorded human decision the architect already flagged. The STOP simply affirms that flag is genuinely
grounded and pins the floor below.

### Is subject-invisibility itself the line? — No, not at today's consequence level.
Because the only downstream effect is an *acknowledgeable* `soft_confirm` that *already* discloses the
count to the subject at friction-time (`evaluatePreflight.ts:132`). The subject is not wholly in the
dark; what they lack is *contestability*. A contest channel becomes a floor the moment the consequence
exceeds acknowledgeable-friction. So the disclosure obligation should **scale with consequence
severity** — and right now severity is low. Invisibility-as-such does not cross today; *unattributability*
does (it always does — it is independent of consequence).

### Minimum floor to discharge
- **6b-1 (attribution + reversibility) is the floor.** Write the manual no-show as an attributable,
  dismissible `customer_signals` row (actor owner-id + reason + timestamp), keeping the counter. The
  architect verifies this is likely **zero-migration** (reuse the existing FORCE-RLS, location-scoped
  table). This is cheap, and it is precisely what turns "owner acknowledge/dismiss only" from
  aspiration into truth. It is the smallest change that stops the system contradicting itself.
- **6b-2 (subject contest channel) is deferrable to a _named_ trigger, not "later."** Clean trigger:
  *the first time `no_show` is consumed by anything stronger than an acknowledgeable `soft_confirm`*
  (i.e. if the preflight ever escalates it toward `hard_block`, or any auto-gating, or it feeds a
  feature the subject cannot pass through) — at that instant a contest channel becomes mandatory,
  shipped simultaneously. Disclosure obligation rises with consequence. Write that trigger into the ADR
  so the deferral cannot silently become permanent.

### The single cleanest question the human must answer (to discharge N5-6b)

> **Must every owner-marked reputation strike be an attributable, dismissible record before it may touch
> a person's counter — or is a raw, unattributed counter increment acceptable for MVP, given the
> strike's only effect today is an acknowledgeable soft-confirm that already shows the customer the
> count?**

Yes → ship 6b-1 (cheap, zero-migration) to discharge the STOP; defer 6b-2 to the named trigger above.
No → record that human decision explicitly (the STOP requires it on the record), accept the system runs
against its own written "acknowledge/dismiss only" contract for MVP, and set a review date.

---

## 4. Steel-man — deferring 6b *entirely* (ship 6a only, leave even 6b-1)

The strongest honest case for the option I did **not** land on:

6a already removes the only *acute, novel* harm — the false-positive strike on a never-dispatched
customer. What remains is a customer who was *genuinely* dispatched-to and genuinely did not receive the
attempted delivery: in those cases the strike is *true*, and its worst downstream effect is an
acknowledgeable confirm that self-discloses the count. No auto-deny exists (verified). At the real MVP
scale — pre-first-paid-order, 50–200 locations — the population of struck customers is approximately
**zero**. Building a reputation-governance record/disclosure subsystem now polishes a corner no real
person has yet touched. The ponytail / "схема багата, рантайм мінімальний" discipline argues exactly
this: the best record is the one not written until a real struck customer exists. Every hour spent here
is an hour *not* spent on the actual launch trigger (first real paid order) — and operator attention is
the scarcest resource in this project. Defer all of 6b behind the existing flag; revisit at the first
real no-show *dispute*. This is a coherent, charter-respecting position, and the human is entitled to
choose it.

My one counter (offered, not imposed): 6b-1 is near-free (zero-migration, reuse `customer_signals` + an
actor id), and the gap it closes is not "a missing feature" but "the system contradicting its own
written contract." Cheap + self-consistency tips me to 6b-1-now. But the steel-man is real; weigh it.

---

## 5. Open question nobody asked — *who actually witnessed the no-show?*

Every option above debates *how* the strike is recorded and *whether the subject* sees it. Nobody asked
the prior question: **on whose knowledge does the strike rest?** The owner presses the button, but the
person with first-hand evidence of whether a customer "didn't show" is the **courier** who attempted the
delivery (and the delivery-attempt / at-door evidence trail). 6a gates on the *order* reaching a
dispatched state — but "assignment reached `picked_up`" is not the same fact as "the courier attested
the customer did not answer the door." The owner's mark is, structurally, often hearsay about an event
only the courier observed.

The absent perspective is the **courier as witness**; the unexamined assumption is **owner-assertion =
ground truth**. Worth one line for the human: *should a no-show strike attach (or require) the courier's
delivery-attempt attestation as its evidentiary ground, rather than rest on owner assertion alone?* This
is the same family as "GPS-сміття-відкинуто" / claim-check — ground a consequence-bearing claim in the
evidence of the one who was actually there. Not a blocker; possibly the cleanest long-horizon shape, and
it would make 6b-1's `evidence` jsonb carry something real rather than a bare owner click.

*(Adjacent, even quieter: the strike survives GDPR anonymization — `no_show_count` is PRESERVED, per
`docs/audit/phase5-exit.md:64`. Decay is owner-convenience forgetting; the subject's right-to-be-
forgotten is a different forgetting. Are they the same? Out of scope for this batch — noted so it is not
lost.)*

---

### Disposition
- B7 · N1 · N2 · N4 · N5-6a: **no Counsel objection.** Ship per the proposal's red→green DoD. (One
  inline-fix to verify: N4 owner-side 500 error state on the FE.)
- N5-6b: **ETHICAL-STOP-N5b** (friction). N5 is not "closed" until the human answers §3's single
  question on the record. Recommended discharge: 6b-1 now, 6b-2 at the named trigger. The deferral
  (§4) is a legitimate human choice if recorded.
