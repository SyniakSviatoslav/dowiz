# Counsel Opinion — P0 Privacy Hardening

- Role: Counsel (Triad Council), advisory
- Date: 2026-06-21
- Reviews: `docs/design/p0-privacy-hardening/proposal.md` · `docs/adr/ADR-p0-privacy-hardening.md`
- Verdict: **GO** with zero blocking ETHICAL-STOPs. Two grounded frictions raised below are non-blocking *conditions of conscience* — a conscious human may proceed past them with a recorded decision; they do not gate the gate.

Up front: this batch is the rare proposal where the engineering decision and the ethical decision point the same way. Minimizing PII at every seam is not a tax on the product — it *is* the product's dignity. The Architect already failed-closed everywhere (drop the ping, drop the broadcast, drop the address before ever leaking). My job here is not to find a villain; it is to name where "technically correct" and "humanly right" might diverge, and they barely do. I will be brief where I add nothing.

---

## 1. Reasoning by lens (only what is load-bearing)

### Justice / stakeholders — who wins, who bears the cost
- **Courier (P0-1):** the clear winner. Today an on-shift idle courier is tracked continuously and fanned to the owner map — surveillance of a worker who is not working. The guard removes ~half the write volume *and* the watching. The cost falls on no one except the owner who briefly loses a dot on a map for a courier who is, correctly, none of their business at that moment. This is a just redistribution: agency returns to the person with the least power in the system.
- **Customer (P0-2, P0-3, P0-4):** wins on every count — PII stops leaking to Meta via a reverse-engineered channel, stops riding the bus and the logs, stops being immortalized in a Telegram history the customer never consented to and can never reach to delete.
- **Owner (P0-2, P0-4):** the *only* stakeholder who pays. P0-2 can disable their sole alert channel; P0-4 removes the address they currently use to dispatch. Both costs are real and both are addressed (reconfigure prompt; `area`/`full` levels) — but the owner is the one carrying the friction of this privacy gain. That is defensible: the owner is the data *controller* for their customers, so it is right that the controller, not the data subject, absorbs the inconvenience of doing right by the subject. Still — see the humanity lens; "defensible" is not the same as "kind," and one of these can be made kinder for nearly free.
- **Platform / operator (ФОП):** wins legally and reputationally; sheds the single largest legal liability (TOS-violating PII egress) before the first paid order. Strategically the correct thing to do *before* launch, not after.

### Dignity / autonomy (courier first)
- The proposal draws the active-delivery line at `courier_assignments.status IN ('accepted','picked_up')` and I verified the codebase already treats `'assigned'` as a *separate, earlier* state (`shifts.ts:140,:223` enumerate `('assigned','accepted','picked_up')` — `'assigned'` is dispatcher-assigned-but-not-yet-accepted). **The chosen guard correctly excludes `'assigned'`.** This matters more than the proposal flags: it means a courier who has been *offered* a delivery but has not *consented* to it by accepting is **not** tracked. Tracking begins at the courier's own act of acceptance. That is autonomy-respecting by construction, and it is a stronger dignity posture than the proposal advertises. Worth stating explicitly in the ADR so a future refactor does not "tidy" `'assigned'` into the active set and silently re-introduce pre-consent tracking.
- On the operator's question — *should tracking start only at `picked_up` rather than `accepted`?* My read: `accepted` is the right floor, not `picked_up`. At `accepted` the courier has voluntarily taken the job and is en route to pickup; the ETA/route feature the customer is watching depends on that leg. Narrowing to `picked_up` would blind the live track during the pickup leg for a marginal privacy gain over a leg the courier consented to. The line `accepted` = "I have agreed to do this delivery" is the honest, proportionate boundary between work and free time. Keep `accepted`.

### Honesty / consent
- No dark patterns in this batch — it *removes* a covert flow (Baileys) rather than adding one. Good.
- Server stays authoritative throughout (claim-check re-reads the DB row; the UI never becomes the source of truth). Consistent with the red line.
- **One honesty gap, grounded, courier-facing:** I searched the courier UI (`apps/web/src/pages/courier/DeliveryPage.tsx`) and i18n (`packages/ui/src/lib/i18n.ts`). The customer is told plainly that the courier sees their position (`client.sharing_location_note`: "Korrieri sheh pozicionin tuaj në kohë reale"). The courier is told only `courier.gps_active: "GPS Aktiv"` — an indicator that GPS is *on*, with **no statement of the boundary**: that they are tracked *only* during an accepted delivery and *not* in their free time. P0-1 makes that boundary *true in code* but leaves it *unspoken to the person it protects*. This is the friction in F-1 below.

### Care / harm — which failure wounds a real person
- P0-1 fails closed (drop the ping). The one human-harm path: a courier legitimately on delivery whose `accepted` row lags, briefly untracked → ETA degrades to last-known. Seconds, accepted, correct.
- P0-2 fails *open* toward harm in exactly one cell: an owner whose **only** channel was WhatsApp now misses orders until they reconfigure. This is the single place in the batch where the change can cause a real person (an owner, and transitively a customer whose order goes unseen) concrete loss. The proposal mitigates it (R4: pre-deploy ops warning + in-app prompt). I want this hardened from "ops should" to "ops must, with proof" — see F-2. This is friction, not a stop: it is operationally solvable and the privacy/legal gain is large.
- P0-4 fail-closed to `minimal` on ambiguous parse is exactly right — never emit a raw address by accident.

### Long horizon / strategy
- Reversibility: excellent. Forward-only, non-destructive migration; code-only rollback is safe against it. Nothing here is a one-way door.
- Second-order: removing Baileys now avoids the far worse future where the number is banned mid-operation and *all* WhatsApp owners lose alerts simultaneously, unplanned, at peak. Doing it deliberately pre-launch converts an inevitable uncontrolled outage into a controlled migration. This is the strategically wise sequencing.
- Serves the launch trigger (first real paid order) directly: these are launch *blockers*, not polish. Correct prioritization.
- One year out, the thing we would regret is **not** having done P0-2 (a ban during operations) far more than any friction it causes now.

### Aesthetic / conceptual integrity
- The claim-check pattern is already the house style on the pg-boss path; P0-3 extends *the same* pattern to the realtime bus rather than inventing a parallel one. This is integrity — one idea, applied consistently, "schema rich, runtime minimal." The deletion of the masking helpers once data never leaves the DB tier is the elegant tell: the right design makes whole categories of defensive code *disappear*. That is beauty as a leading indicator of correctness.
- The one aesthetic blemish is honest and self-declared: **DEV-1** — the `channel` CHECK is *not* narrowed, leaving disabled `'whatsapp'` rows that make the schema "look incomplete." I think the Architect chose correctly (non-destructive > tidy), but see the epistemic lens — the dependency on app-layer enforcement is the load-bearing assumption.

### Epistemic — assumptions, missing perspectives
- **Load-bearing unchecked assumption (DEV-1):** with the CHECK left broad at `('telegram','push','whatsapp')`, the *only* thing preventing a new `'whatsapp'` row is app-layer discipline (TS union + worker query filter). The database will happily accept a `'whatsapp'` insert forever. If any future code path, migration, or manual ops query writes that channel, nothing stops it, and the worker silently won't process it. The schema no longer encodes the invariant the system depends on. This is acceptable *given* the non-destructive priority, but the safety of DEV-1 rests entirely on "the app layer is the only writer, now and forever" — which is exactly the kind of assumption that quietly stops being true. A `NOT VALID` CHECK (rejects new rows, tolerates existing ones — standard Postgres) would close this without destroying owner config or violating forward-only. Worth a line in the open question.
- **Missing perspective — the courier as data subject of their *own* movement.** The whole batch frames the customer as the data subject to be protected, and rightly. But the courier's GPS breadcrumb (`courier_positions`, 24h purge) is *the courier's* personal location data, and the courier is a worker, not a customer. Under the Albanian framing the platform is *controller* for courier geodata. The 24h purge and the active-delivery guard are good, but the courier's perspective — "what is kept about *me*, for how long, and can I see it" — is structurally absent from the proposal. Not a blocker; a gap in whose-voice-is-in-the-room.

---

## 2. ETHICAL-STOPs

**None.** No grounded red line is crossed. To be explicit about the candidates and why each clears:

- *Surveillance-creep?* — The opposite. P0-1 *removes* unjustified tracking and respects "courier completes / dignity of the worker." Clears.
- *Anonymize-not-delete?* — Honored. P0-2 disables (does not delete) owner targets; the migration is non-destructive. Clears.
- *Soft-confirm-as-trap / dark pattern?* — None added; one covert channel removed. Clears.
- *Server-authoritative?* — Preserved (claim-check re-reads DB). Clears.
- *Zero-PII-to-AI, claim-check, friction-not-verdict, GPS-garbage-rejected?* — Untouched or strengthened. Clears.

The two items below are **frictions** (recorded conditions), explicitly *not* stops. A conscious human may proceed.

---

## 3. Non-blocking frictions, aesthetic & strategic advice

**F-1 (humanity / honesty, cheap to fix) — Tell the courier the boundary you just built.**
P0-1 makes "you are tracked only while on an accepted delivery, never in your free time" true in code. Say it to the courier, once, in plain Albanian — a single i18n string near `courier.gps_active`, e.g. "Pozicioni juaj ndahet vetëm gjatë një dorëzimi aktiv — jo në kohën tuaj të lirë." Cost: one string. Benefit: converts a silent technical gate into a felt act of respect, and pre-empts the worker's reasonable suspicion that the app watches them off-shift. A privacy protection the protected person cannot see is half a protection.

**F-2 (care / operability) — Promote the WhatsApp-owner warning from "should" to a proof-gated MUST.**
R4 / §9 already names the query (`SELECT location_id FROM owner_notification_targets WHERE channel='whatsapp' AND status='active'`). Make running it + warning those owners a *recorded pre-deploy checklist item with the result pasted*, not a "should." This is the one cell where the batch can cause an owner to silently miss a real order at launch. The fix is procedural, not code, and it is the named GO-gate pre-req — just bind it tightly.

**A-1 (aesthetic / epistemic, optional) — Consider a `NOT VALID` CHECK for DEV-1.**
`ALTER TABLE ... ADD CONSTRAINT ... CHECK (channel IN ('telegram','push')) NOT VALID;` rejects *new* whatsapp rows while tolerating the existing disabled ones — restoring the schema-level invariant without destroying owner config or breaking forward-only. Closes the "app layer is the only writer forever" assumption. Non-blocking; the current choice is defensible.

**A-2 (strategy, optional) — When official WhatsApp Business API returns (deferred ADR), the owner-reconfigure prompt from P0-2 is the natural re-onboarding surface.** Design the disabled-target prompt now so it can later say "WhatsApp is back, officially — re-enable?" rather than only "removed, pick something else." Cheap forward-compatibility for a known future.

---

## 4. Steel-man of a rejected option

**Steel-man: "Keep the full address in the Telegram body" (P0-4 Option, the owner's actual workflow).**

The strongest honest case, stated at full strength: For a solo ФОП owner running dispatch *from their phone, in the Telegram chat*, the address in the message body is not a leak — it is the entire tool. They glance at the chat, read the street to the courier over the phone, done. Forcing them to tap a deep-link, wait for the owner SPA to load on mobile data, authenticate, and find the order, *for every single order at peak*, is not a minor friction — it can be the difference between dispatching in five seconds and fumbling for thirty while orders stack. The privacy harm being prevented is largely theoretical (a Telegram chat the owner controls, on the owner's own device); the workflow harm is concrete and per-order. "Minimize PII" can become "minimize the owner's ability to run their business," and the person who pays for the customer's abstract privacy is a real small-business owner at their busiest moment. There is a real chance that an owner under that friction simply flips everyone to `full` on day one — and now the default did nothing except annoy people into opting out of it.

**Why the proposal's choice still holds (but learn from the steel-man):** The `full` opt-in *is* the proposal's answer to exactly this owner — and that is the right structure. But the steel-man exposes a risk the proposal under-weights: if `area` mode is even slightly clumsy in practice (deep-link slow, area too vague to dispatch), owners will mass-opt into `full`, and the *default* will have protected no one. So the proposal's choice survives **conditioned on the `area` body being genuinely dispatch-sufficient** — a real district + street name is usually enough to phone the courier and say "it's on Rruga X, I'll text the number." If post-launch data (R1) shows owners fleeing to `full`, that is the signal that `area` failed as a *usable* default, not that privacy was too aggressive. Measure the `full`-opt-in rate as the canary. (The other rejected options — B per-subscriber re-fetch, C payload encryption, B caching the guard bit — are correctly rejected on N+1 / un-RLS'd-transport / invalidation-correctness grounds; no steel-man rescues them.)

---

## 5. The open question no one asked

**For the human (operator): The courier's own location data has no voice in this proposal — what do we owe the worker about the breadcrumb we keep on *them*?**

Every line of this batch protects the *customer* as data subject. But `courier_positions` is the *courier's* personal movement, the platform is its controller, and the proposal treats it purely as a feature input bounded by a 24h purge. The question no one in the room asked: does the courier know what location data we retain about them, for how long, and can they see or contest it the way we let a customer ask the restaurant to remove their data? We built dignity *into* P0-1 (tracking only on consented delivery) — the open question is whether we extend that same dignity *outward* as transparency: a one-line "what we keep about you and for how long" for the courier, parallel to the `checkout.privacy.*` copy we already give the customer. Not a launch blocker. But it is the difference between a system that is privacy-correct *for paying customers* and one that is privacy-*just* for everyone it touches — including the person doing the hardest, lowest-paid work in it.

---

## RE-EXAMINE (R2)

- Re-read: revised `proposal.md` + `resolution.md`.
- Verdict: **GO.** Every R1 friction is closed or correctly escalated; the architect's revisions created **no new ethical problem.** Zero ETHICAL-STOPs — stated plainly.

### Did the revisions resolve my R1 frictions?

**F-1 (tell the courier the boundary) — RESOLVED, well.** `courier.gps_boundary_note` is added in sq/en/uk (resolution §F-1, proposal §4 P0-1 line 100), rendered near the GPS indicator on `DeliveryPage.tsx`. The copy is honest about the *actual* boundary the code now enforces: "shared only during an active delivery — never in your free time." It does not over-promise (the code gates at `accepted`, the copy says "active delivery" — these align). And it is proof-gated: a Playwright visibility assertion is on the proof list (§9). This is exactly the right shape — the protected person can now *perceive* the protection. No residual friction. One micro-note, non-blocking: "active delivery" in the copy and `accepted` in the code are the same boundary for the courier's lived experience, so the wording is honest; no change needed.

**F-2 (WhatsApp-owner warning → proof-gated MUST) — RESOLVED, correctly hardened.** Promoted from "should" to a GO-gate MUST with three binding conditions (resolution §E-1, proposal §9 line 245, R4): query run against prod, result pasted into the deploy record, and *every affected owner warned AND has another active channel OR is individually contacted*. That third clause is the one I most wanted and it is present — it closes the "owner silently misses a real order" cell, which was the single fail-open-toward-harm path in the batch. This is now as tight as a procedural control can be.

**A-1 (NOT VALID CHECK for DEV-1) — RESOLVED above and beyond.** My A-1 was explicitly optional; the architect adopted it (resolution §E-2, proposal §5 step 2). The schema-level invariant ("no new whatsapp, ever") is now DB-enforced rather than resting on the "app layer is the only writer forever" assumption I flagged as load-bearing. The honesty discipline here is notable: the architect did not just adopt my suggestion, he *recorded the new risk it creates* (NR-3: a future `VALIDATE CONSTRAINT` pass would fail on the disabled rows) and handed it to the breaker with the constraint name encoding intent. That is correct second-order thinking — closing one assumption without opening a silent one.

### HD-1 — idle-courier map dot (privacy-max vs coarse last-known). My recommendation.

This is correctly framed as a HUMAN-DECISION, not a stop — and I confirm: **it is a genuine product choice with an ethical gradient, not a red line.** There is no grounded red line that *forbids* showing an on-shift courier's coarse position to their own employer who is paying them to be on shift; an employer seeing where their on-the-clock worker is sits inside the normal, consented employment relationship. So neither option (a) nor (b) crosses a line. This is the operator's call to make.

**My recommendation, by lens, leans (a) privacy-max — but softly, and I name what (b) would owe:**

- *Dignity / justice (favors a):* The cleanest posture is "we do not watch a worker who is not working." An idle on-shift courier is between jobs; continuous position fan-out to the owner map during that gap is watching for watching's sake, since there is no delivery whose ETA depends on it. Option (a) makes the privacy gain *whole* — the courier is invisible precisely when their movement is no one's operational business. This is the same axis as F-1: dignity you can feel.
- *Operational need (the honest pull toward b):* The owner's stated need is real — "are my couriers nearby / available to take the next order?" A dispatcher who cannot see idle couriers may dispatch blind. But notice the *resolution* of that need: the owner needs to know a courier is *available and roughly where*, not a live breadcrumb. That is satisfiable by **availability + coarse last-known**, not by continuous tracking. So (b) is defensible *if* it is genuinely coarse and last-known, not a re-skinned live feed.
- *The trap in (b):* "coarse last-known idle position" can quietly become "we kept tracking, just relabeled." If the operator chooses (b), the ethical condition is that it must be *actually* degraded — last-known point only (no fresh polling while idle), visibly coarse (e.g. neighborhood-level, not a moving dot), and ideally the F-1 courier copy should then tell the truth about it ("while on shift but between deliveries, your employer sees your approximate last location"). Honesty travels with the choice.

**Recommendation:** default to (a) privacy-max. It is the most dignified, it makes P0-1's gain complete, and the operational loss is small at launch scale (8 concurrent couriers — the owner can ask). If the operator finds dispatch genuinely suffers, (b) is an acceptable fallback **conditioned on** real coarsening + matching courier-facing disclosure. Either is a legitimate recorded human decision. No stop.

### HD-2 — default detail level for the solo-ФОП (full / area / minimal). My recommendation.

Also correctly a HUMAN-DECISION. My ethical+practical read:

**Default `area` (best-effort) is the right default — keep it.** Reasoning across both axes:
- *Ethics:* `area` defaults to the more-private posture and fails closed to `minimal` on ambiguous parse — it never leaks a house number by accident. Defaulting to `full` would default every owner into writing customer addresses into permanent Telegram history they do not control; that is the wrong default even though `full` must remain *available* (R2, explicit opt-in). Defaulting to `minimal` would be the maximally-private default but throws away the dispatch hint on the *minority* of addresses where it can be safely extracted — needless friction with no privacy gain over `area` (since `area` already degrades to `minimal` when it can't split).
- *Practicality:* `area` is the strict superset of `minimal` in safety terms (same fail-closed floor) while occasionally giving the owner a usable hint. There is no scenario where `minimal`-as-default beats `area`-as-default on privacy, and `area` sometimes wins on usability. So `area` dominates `minimal` as a default.

**Is it acceptable that `area` mostly degrades to `minimal`?** Yes — *because the architect made it honest* (resolution §D, proposal §4 P0-4 line 173, R1). The original sin would have been an `area` mode that *claimed* to give an area hint while silently giving nothing. The R1 fix renames it best-effort and the owner copy states plainly it shows a coarse area "only when it can be safely extracted, otherwise order# + total." Degrading-to-minimal is acceptable *only with that honesty in place* — and it is. The owner is not deceived about what they will get; they learn it per-order and can opt to `full` with eyes open. That is the difference between a degraded feature and a dishonest one.

**The one thing to hold:** my steel-man (§4) stands — if owners flee to `full` en masse, that is the signal `area` failed as a *usable* default, not that privacy was too aggressive. The architect already wired the `full`-opt-in rate as the measured canary (R10/NR-4). Correct. So: ship `area`, watch the canary, and read mass-`full`-adoption as a usability bug to fix, not a privacy retreat to accept.

### Any remaining ETHICAL-STOP?

**Zero. Stated plainly: there is no grounded red line crossed by the proposal or by any of the architect's revisions.** All five R1 candidate stops still clear (surveillance-creep → reversed; anonymize-not-delete → honored; dark-pattern → none added; server-authoritative → preserved; GPS-garbage/claim-check/friction-not-verdict → strengthened). The pivots only moved the design *toward* the red lines, not across them: P0-3's producer-side minimization means PII never touches the un-RLS'd transport at all (a *stronger* claim-check posture than the original re-fetch); P0-1's client-backoff fix preserves "courier stays visible while consenting-and-driving" without re-introducing pre-consent tracking. Nothing degraded.

### New ethical problems from the revisions?

Checked each pivot for fresh harm:
- **P0-3 pivot (producer-side minimization):** no new ethical surface. It *removes* PII transport rather than relocating it. The recorded cost (R7/NR-1: dashboard card lacks live name/phone on a brand-new order for seconds until the RLS+JWT fetch) is an owner-side UX latency, not a dignity or honesty harm — and it is honestly recorded, with a masked placeholder rather than a deceptive blank. Clean.
- **P0-1 client backoff (NR-2):** the only watch-item is operational (thundering-herd), already handed to the breaker — not ethical. No dignity regression; the courier is *more* reliably visible-when-consenting, not less.
- **NOT VALID CHECK (NR-3):** introduces a future-migration footgun, recorded and handed off — engineering risk, not ethical.
- **Honest-`area` (NR-4):** the subtle one — telling the truth may push owners to `full` faster, accelerating PII-to-chat. But this is honesty creating a *visible* pressure rather than hiding it; the cure is not to lie about `area`, it is to make `area` good enough that owners don't flee. Correctly handled as a measured canary, not papered over. No new ethical debt.

No revision created a dark pattern, a surveillance expansion, a deletion-not-anonymization, or a dignity regression.

### Open question — courier as data subject of own movement: defer or blocker?

**Confirmed DEFER, not a blocker.** It is correctly recorded as DEFER-FLAG → future ADR (resolution §F, proposal §10). My reasoning for why it does not escalate to a stop: the *acute* harm (continuous off-work tracking) is exactly what P0-1 *fixes* in this batch — the courier's dignity is materially advanced here, not deferred. What remains deferred is the *transparency/access* layer (the courier seeing/contesting their retained breadcrumb), which is a real gap but not a launch blocker: the 24h purge already bounds retention, and no new courier-data harm is introduced by this batch. Deferring a not-yet-existing transparency surface is legitimate; deferring an active harm would not be. This is the former. I note it remains *my* open question to carry forward — when the future courier-ADR opens, this is its seed, and F-1 (the boundary copy) is the first half of that dignity already shipped.

### Counsel R2 verdict

**GO.** The triad did its job: I named the frictions, the breaker stressed the design, the architect resolved both with honesty (adopting even my optional A-1 and recording the risk it created). The two open human decisions (HD-1, HD-2) are genuine product calls with my recommendations on record — default (a) privacy-max for the idle dot, default `area` for detail level — neither blocks. Zero ETHICAL-STOPs. The human decides HD-1/HD-2 on the record; everything else is ready.

---

*Counsel is advisory. The human decides. Recorded frictions (F-1, F-2) and the open question are for a conscious human to accept, act on, or set aside on the record — not gates.*
