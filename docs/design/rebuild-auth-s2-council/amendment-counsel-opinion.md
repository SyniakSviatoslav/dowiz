# S2-AUTH — Cutover Posture Amendment · COUNSEL OPINION (re-ratification)

> Seat: **Counsel** (ethics · aesthetics · strategy) — re-consulted as one of the two S2 seats
> (`amendment-cutover-reratify.md:5`). I am the S2-counsel who signed the per-request/%-traffic canary
> in the original record (`counsel-opinion.md:159-163`). This is my re-ratification vote on the
> superseding amendment. Advisory, non-blocking; the operator signs the 🔴 and is final.
> Every load-bearing claim below was verified against live source, not trusted from the packet.

---

## VOTE: **RATIFY-WITH-CONDITION** — adopt **Option C1 (family-sticky canary)**; **reject bare atomic-flip.**

No ETHICAL-STOP. This is re-ratification friction that **affirms** the signed decision, not overturns
it — and "affirm the signed decision, tightened" is the *lower*-friction, more-reversible outcome than
supersession. The architect's AQ1 recommendation is correct on auth's terms and I concur, adding a
person-cost weighting the technical framing under-carries. Concretely:

- **AQ1 (posture): RATIFY-WITH-CONDITION.** Adopt **C1 (family-sticky canary — `consistent-hash(family_id) → stack`)**. It *is* the signed decision, not a supersession of it (§B1). C2 (atomic-with-quiesce) is an acceptable fallback **iff** the S2-breaker confirms quiesce converts the window to genuine zero-split — but C1 is preferable because it neither re-opens the signed record nor imposes quiesce's own small user-visible harm (§B4). **Reject bare atomic-without-quiesce.** Gate-iv (byte-identical refresh SQL incl. `interval '5 seconds'`, SQL-`now()` clock) is a hard prerequisite in **every** posture. This is the S2-breaker's call — I concur.
- **AQ2 (verification-parity ordering gate): RATIFY, unconditional.** Not my seat's domain (it is C1 parity, the breaker's), but I affirm it on the honesty lens: no authed surface should flip while *claiming* cross-stack auth works until it is *proven* both directions. Stateless verify path, zero committed effect, lowest-risk item in the packet.
- **AQ3 (elevate C1 + posture to bind S7 courier-auth): RATIFY.** My call (scope/legibility). A genuine gap; the surface-independent-invariant fix is the right shape (§B3).
- **AQ4 (record irreversibility + pre-authored cleanup runbook per auth-family flip): RATIFY.** My call (irreversibility/person-cost). Ratify — with one gap named that no runbook here yet closes (§7).

---

## Verification note (I read the live source behind the amendment's load-bearing claims)

- **The flip is not atomic.** `courier/auth.ts` / harness breaker HIGH-1 (`breaker-findings.md:50-57`): the flip is `UPDATE cutover_flags + NOTIFY`, LISTEN/NOTIFY is *documented* blocked through the transaction pooler (`server.ts:220-221`), so convergence degrades to a **1–5s TTL split-brain per flip**. The amendment's §3a is not rhetoric — it is the harness's own robustness finding. Confirmed.
- **Atomic relocates the split; the trip-wire is reactive.** harness breaker MED (`breaker-findings.md:135-141`), verbatim: atomic-flip "does not remove the concurrent-refresh-split hazard — it relocates it into the flip window, and the trip-wire only detects irreversible damage … detects damage it cannot undo." Confirmed.
- **Courier mints the same hazard, outside S2's gate.** `courier/auth.ts:420`: on reuse-detection, `UPDATE courier_sessions SET revoked_at = now() WHERE family_id = $1`. The family model and the reuse→family-revoke branch are real and live. **The amendment is honest about the asymmetry I want on the record: this is a *soft*-revoke (`revoked_at` UPDATE), marginally *less* irreversible than the owner path's family DELETE.** It did not inflate the courier hazard to match the owner's. Confirmed — and that honesty is a point in the amendment's favor (§B3 turns it the other way, on person-cost).
- **The route-back is the harness's own instruction.** harness resolution REV-C8b (`resolution.md:51-58`): the Q3/Q4 revisions "overturn a signed unanimous 4-seat S2 decision — they must be ratified as a superseding amendment in the S2 record, re-consulting the S2 seats (esp. the S2 breaker), NOT settled inside this 2-seat mechanism council." This packet is that route, honored. Confirmed.

---

## By focus

### B1. Irreversibility on auth — is bare atomic the weaker posture? **Yes, on auth's own terms.**

My original thesis stands unchanged and is the tie-breaker here: *routing back does not un-delete a
family row* (`counsel-opinion.md:146-153`; resolution convergence-3). Rollback is true for *routing*
and false for *state a bad path already mutated*. On the one surface where that asymmetry bites, the
posture question is not "which looks cleaner" — it is "which fails more safely when gate-iv is subtly
open despite our best proof."

The harness's case for atomic rests on one claim: "atomic keeps a family wholly on one stack, so the
concurrent-refresh split is gone." **The harness's own breaker demolished that claim** (HIGH-1): the
flip is *not* atomic — it is 1–5s eventual convergence through a pooler that blocks NOTIFY. So the split
is not removed; it is **relocated** — from a continuous, front-loaded, low-blast-radius place (the
canary %) into a bounded-but-full-population, **post-commit** window where the trip-wire fires only
*after* families are revoked. Against an irreversible effect, a post-commit detector "detects damage it
cannot undo." Bare atomic buys a cleaner-*looking* mechanism by moving the residual hazard from a
**visible, pre-commit, 1%** location to an **invisible, post-commit, 100%** one. That is the weaker
posture on the surface that deletes.

The honest ledger (amendment §3a) is right that **neither canary nor atomic dominates** — the canary
trades wider *time* exposure for smaller *population* + pre-commit detection. But that framing is only
forced if the fork is "canary vs atomic." It is not (§B2). The real human at stake: the vendor — or the
courier — whose refresh landed in the split loses their session. On auth, low-blast-radius +
detect-before-commit is not the timid choice; it is the choice that keeps the irreversible cost off the
most people. **The amendment is correct.**

### B2. Process honesty — the route home, and the precedent. **Right, and it is the sharpest thing here.**

This is the finding I carried in the harness council (`rebuild-cutover-harness/counsel-opinion.md:296-301`,
§C-2): *ratify it where it was decided, by the seats who own it.* The amendment honors it exactly — it
adopts nothing by drafting, returns to the S2-breaker + S2-counsel, and reserves the 🔴 for the operator
against the S2 record. I affirm the route.

**The precedent, named cleanly** (this is a governance question, my seat's domain): a 2-seat *mechanism*
council reasoning about *how to flip* is legitimate. But when its mechanism reasoning has the **effect**
of overturning a substantive auth-safety posture that four seats — including a *decorrelated*
security-sentinel — unanimously converged on and the operator signed as 🔴, the *authority* to change
that runs out at the mechanism council's edge. Not because the objection is wrong (it is a genuinely good
challenge), but because a signed red-line decision is **deterministic authority**; a mechanism council's
challenge is an **advisory signal**. Signals inform; the record + the human decide. If a 2-seat council
could overturn a 4-seat signed auth decision by *importing a differently-reasoned posture from an
adjacent surface* (S3 "catalog = edit-session, atomic is fine" → auth), then **no signed red-line
decision is stable** — any future council could re-open any signed decision by side-channel. The
amendment's scope-guard ("that reasoning does not transfer") is precisely the immune response that
catches the cross-surface reasoning-transport. I affirm it.

**One fairness the record needs, so this stays healthy, not adversarial:** the mechanism council did
*not* try to do this silently. Its R-1 explicitly said "it revises a council decision, so it is not
adopted silently" and routed to operator 🔴. So the honest framing is not "a breach was caught" — it is
"a good mechanism objection was raised *and correctly flagged as touching a signed decision*, and this
amendment is the proper completion of that honesty: it carries the objection home rather than letting an
operator 🔴-signature on a *mechanism* packet stand in for a re-ratification of the *S2* record." That
is a governance process working as designed. I want that affirmation on the record as much as the
precedent — the reviewers who flagged their own overreach deserve the credit, not a reprimand.

### B3. Courier (S7) outside the S2 gate — **elevate to a cross-surface invariant. Yes — and the reason is dignity, not just irreversibility.**

This lands squarely in the perspective I have flagged in *both* prior opinions and that no seat but this
one carries: the **least-powerful actor, de-authenticated mid-shift** (S2 `counsel-opinion.md:170-173`;
harness `counsel-opinion.md:168-182`). The amendment surfaced a real scope gap — `courier/auth.ts` mints
S2-shaped tokens via the identical signer with the identical family-revoke branch, path-owned by S7,
outside S2's gate — and its fix (make C1 + the posture a **surface-independent auth invariant** binding
S2 *and* S7, rather than re-homing 5 routes) is the conceptually-whole move: bind the gate to the *token
shape*, not to the accident of *route ownership*. I affirm the fix.

**Where I turn the amendment's own honesty:** it correctly notes the courier revoke is a *soft*-revoke
(`revoked_at`), technically *less* irreversible than the owner's DELETE. True. But the **person-cost runs
the opposite way from the technical irreversibility.** An owner wrongly evicted re-logs-in at a desk. A
courier wrongly evicted is the most-surveilled, least-powerful actor, **mid-delivery, with cash to
collect and live navigation state**, bearing the cost of a rebuild whose benefit is entirely the
platform's. The invariant should bind S7 not because the courier revoke is *equally irreversible* (it is
slightly less) but because the **person it hurts is the one least able to absorb the harm**. Record the
invariant in both the S2 record (its home) and the S7 record (inheritor), and record *that reason* — so
no future reader flips S7 auth thinking "S7 is just dispatch, S2's gate did not reach here."

### B4. Charter — irreversibility, real people, honesty-over-speed. **Clean; no crossing; the honesty lens is decisive.**

- **Military / warfare / surveillance-for-harm / commons-capture:** untouched. N/A.
- **Irreversibility + real people:** the whole spine, and this is the S4-caliber *verified intersection*
  with a grounded line — but it does **not cross** it, because the amendment *chooses the posture that
  protects the line* rather than walking into it. The friction is exactly that bare atomic would move the
  irreversible risk into a reactive, post-commit, invisible window. That is friction, not a stop.
- **Honesty-over-speed — the sharpest framing, and it is right.** Bare atomic is *aesthetically*
  seductive: "one flag write, the whole family moves cleanly, no messy per-request routing." But that
  cleanliness is **rhetorical** — the amendment (via the breaker) shows the atomicity is a promise the
  pooler cannot keep. This is the seduction this seat exists to catch: **elegance that is seductive
  rather than genuine.** The family-sticky canary *looks* less clean (it has explicit routing) but is
  genuinely whole — it *eliminates* the split rather than *relocating* it. Aesthetics as leading
  indicator of ethics: a design whose own robustness finding contradicts its own atomicity claim is not
  yet conceptually whole, and "schema rich, runtime minimal" cuts *against* bare atomic here — bare
  atomic adds an invisible runtime split-brain window; family-sticky routing commits nothing (it is
  stateless routing, resolved by the shared-DB UPDATE). The honest posture is the whole one.

---

## Does family-sticky canary preserve the signed virtue? **Yes — it *is* the signed decision, tightened.**

The signed decision's **letter**: *"a canary flip gated on the family-revocation-rate matching the Node
baseline, not a hard switch"* (`resolution.md:39-40`). It never said *per-request routing* — that was an
implementation assumption, and it is the *whole* of what the harness legitimately objected to. The
**virtue** the four seats converged on was defense-in-depth: make a parity bug **observable at low blast
radius before it is committed**, on the surface where commit is irreversible — precisely *because* a
"we proved the SQL is byte-identical" claim (gate-iv) is the kind of parity assertion a strangler port
gets subtly wrong (that is the entire reason for the parity oracle; the canary exists so gate-iv is not
trusted alone).

Family-sticky canary preserves **all three** properties — low blast radius (1% of *families*),
pre-commit detection (watch the revoke-rate on the canary arm before widening), not-a-hard-switch
(incremental widen) — and **removes the one thing the harness rightly objected to** (the per-request
concurrent-refresh split). It is not a compromise *between* the signed decision and the harness. It is
the signed decision, answering the harness's valid point. **The signed virtue is kept intact, not
overturned.** The correct amendment was never "supersede the canary" — it was "tighten it to
family-sticky routing." That is why my vote affirms the record rather than replacing it.

---

## Steel-man of the rejected option (obligatory)

**Bare atomic-flip — the option I reject.** Its strongest case, made fairly:

1. **Simplicity as a safety property.** One flag write is the fewest-seam mechanism possible; on a
   red-line surface, *fewer moving parts* is itself a virtue. Family-sticky canary adds a routing layer
   that must parse `family_id` out of the token at the front-door — **new code on the auth hot path**,
   itself a candidate for its own parity bug. "You may be adding a parity-bug surface to avoid a
   parity-bug surface." That is real.
2. **If gate-iv truly holds, the window is safe regardless of posture.** With byte-identical SQL and
   SQL-`now()`, even a cross-stack concurrent refresh in the 1–5s window resolves correctly via the
   shared-DB atomic UPDATE — the DB is the single arbiter, and *no* family is wrongly revoked. On that
   premise, bare atomic is not catastrophic; it is clean.

**Why I still reject it.** Point 2 proves too much: if gate-iv fully closes the hazard, the canary was
never needed and the 4-seat convergence-3 was over-cautious. The S2 council **deliberately** did not
trust gate-iv alone — the posture exists for the case where gate-iv is *subtly open despite proof*. In
that case, pre-commit detection on 1% fails more safely than post-commit detection on 100%. And point 1
is answered by *where* the complexity lands: family-sticky routing keys on `family_id` (already in the
token) and **commits nothing** — a bug in it is at worst a mis-route the shared-DB UPDATE still resolves
correctly, *not* an irreversible revoke. Bare atomic's simplicity is real, but it is simplicity in the
**wrong half of the system**: it simplifies the routing (already reversible) at the cost of the commit
(irreversible). The new seam family-sticky adds is in the *reversible* half — the safe place to add it.

---

## The question nobody asked (§7)

Every posture in this debate — canary, atomic, family-sticky, quiesce — is argued to **minimize wrongful
revokes**. AQ4 rightly requires a *pre-authored cleanup runbook* for the residual. But read what the
runbook actually is: for a wrongly-DELETEd **owner** family, "support-mediated re-auth / vendor
re-login" — thin-but-real, and adequate because the owner is at a desk and can wait. **Nobody has
written the recovery path for the actor for whom "re-login later" is not an option.** A soft-revoked
**courier** hit by the residual is *at a customer's door, with cash to collect, right now* — a
support-ticket-shaped runbook is not a recovery for them; it is an abandonment mid-delivery.

The unasked question is not "how do we minimize the residual" (every posture answers that, and
family-sticky answers it best). It is: **what is the in-the-moment recovery path for the least-powerful
actor, for whom the owner-shaped cleanup runbook is not a recovery at all?** The posture debate optimizes
the *rate*; nobody specified the *experience of the one who is hit*. Family-sticky canary protects that
person best precisely because it drives the residual to 1% of families with pre-commit detection — but
the residual courier, if hit, still needs an answer this amendment does not yet contain. That belongs on
the AQ4 runbook, authored for the courier's reality (fast, in-shift re-auth), not only the owner's desk.
It does not block ratification. It is the person who cannot attend this council.

---

## Ratification summary (for the S2 record)

| AQ | Vote | Owner of the call |
|---|---|---|
| **AQ1** posture | **RATIFY-WITH-CONDITION** — adopt C1 family-sticky canary (preserves the signed decision); C2 quiesce acceptable iff breaker confirms zero-split; **reject bare atomic**; gate-iv hard prerequisite in all postures | S2-breaker + operator 🔴 |
| **AQ2** verification-parity gate | **RATIFY** unconditional (honesty lens; lowest-risk item) | S2-breaker + operator |
| **AQ3** elevate to bind S7 | **RATIFY** — surface-independent invariant; bind for *dignity* (least-powerful actor), record in both S2 + S7 | **S2-counsel** (this seat) + operator |
| **AQ4** irreversibility + cleanup runbook | **RATIFY** — with the §7 gap named: the runbook must be authored for the **courier's** in-shift reality, not only the owner's desk | **S2-counsel** (this seat) + operator 🔴 |

Record on the S2 line: *"per-request canary tightened to family-sticky canary (Option C1), re-ratified
[date], operator-signed; bare atomic-flip rejected (relocates the split into the post-commit convergence
window on an irreversible surface); family-revoke is not rollback-recoverable; C1 parity + this posture
elevated to a cross-surface auth invariant binding S7 courier-auth."*

---
*Advisory only. No ETHICAL-STOP. This vote **affirms** the signed decision (tightened to family-sticky),
which is the more reversible and lower-friction outcome than supersession. The human is final; nothing
here authorizes a code change or is adopted until the operator signs the 🔴 against the S2 record.*
