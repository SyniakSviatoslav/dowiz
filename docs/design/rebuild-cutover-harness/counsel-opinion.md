# Rebuild Cutover Harness — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the cutover-harness Triadic Council.
> Advisory, non-blocking. Architect asks "will it work"; Breaker asks "how it breaks";
> this seat asks *should it exist, is it fair/honest/dignified, is it whole, what is the long
> horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no design authority.
> Every load-bearing claim below was verified against the live council record, not trusted from the packet.

---

## OPINION: **PROCEED-WITH-REVISIONS** · **NO ETHICAL-STOP**

This is the through-line surface — the one mechanism that carries every other surface across the money and
auth cliffs — and the packet is unusually honest about the one thing that matters most: **routing is
reversible; committed side-effects are not** (threat-model, prime insight). That sentence is the whole
ethical spine of a cutover harness, and the packet states it out loud, gates on it, and refuses to let the
word "reversible" launder the effects that a flag cannot un-write. I looked adversarially at every place a
flip could cross a grounded line and touch a real person — the owner whose store blinks, the courier whose
socket drops mid-delivery, the customer whose order is in flight at the flip instant. **None crosses a
grounded red-line in the direction the line protects**, so the friction here is Opinion, not a stop:

- The one place a customer could be **charged wrongly** — the cross-stack duplicate order (T4/T5) — is the
  Breaker's crown hazard and is *gated*, not open: the flip is forbidden until request-hash byte-identity +
  a cross-stack idempotency probe are green and crypto stays dark through the overlap. The line
  (`сервер-авторитетний` / `готівка→алерт-тертя`) is honored by the design; the residual is a named,
  operator-owned accepted-risk (R-3), not a design that walks into the line. I affirm it and do not
  re-litigate the Breaker's domain.
- The flip requires a **human token** (`readiness_ok` + `/healthz` + operator sign-off, §4). The
  auto-rollback trip-wire is automation, but it degrades **toward the incumbent** — the fail-safe
  direction — so it never autobans a person or overrides a conscious operator; it hands authority *back*,
  never takes it. `людина-в-петлі / нуль-автобану` is honored, not crossed.
- `схема багата, рантайм мінімальний` is kept well: the ownership map + `cutover_flags` table are the
  seams, the proxy runtime engages only when a flag is `rust`. This is the doctrine as restraint, and I
  affirm it out loud (§ aesthetics).

**Why no stop, and where the real friction sits.** Nothing in the harness rises to the S4-style verified
intersection with a grounded line that warranted this seat's only prior STOP. The sharpest thing in the
packet is *governance*, not ethics-of-harm: **the harness quietly proposes to overturn an already-signed
S2 council decision by importing a differently-reasoned posture from S3.** The packet is honest that it
does this (R-1, Q3) — which is exactly why it is a condition and not a stop. My value here is narrow and I
keep it narrow: make the reversibility promise as loud in the *goals* as it is in the threat-model; route
the S2 revision through the humans and seats who own it, not through a batch-sign in this packet; and put a
pre-authored cleanup runbook and a truthful flip-instant experience behind the two surfaces where a routing
rollback leaves a real person holding a real cost.

---

## Verification note (I read the live council record behind every load-bearing claim)

- **The S2 canary is not a footnote — it is a UNANIMOUS convergence point, operator-signed.** The S2
  resolution records, as convergence-point 3 (≥3 seats, highest confidence): *"The cutover must be a canary
  flip gated on the family-revocation-rate matching the Node baseline, not a hard switch"* (`resolution.md:37-40`),
  and the gate to `COUNCIL-APPROVED` lists operator sign-off on *"the cutover canary plan"* (`:99-101`). Commit
  `515ee373` records the operator signed all S2 🔴 items. So the harness's Q3(a) — atomic-flip + trip-wire —
  is a revision of a **signed, unanimous** decision, not a fresh call. Confirmed.
- **S3's atomic posture is real but surface-specific.** S3 REV-7 (`resolution.md:44-49`, operator-signed §4
  item 5) ratifies *"per-surface atomic flip... catalog is an edit-session surface, not stateless — inverts
  the S2 per-request canary."* The inversion was reasoned **for S3** on an edit-session argument that does
  **not** transfer to auth; the harness generalizes atomic to S2 on a *different* argument (concurrent-refresh
  hazard). Two surfaces, two reasons — the harness leans on S3's conclusion to override S2's, which the S3
  record does not license. Confirmed.
- **The flip-as-a-second-human-act is already canon.** S5 counsel §C-2 named the flip *"a separate, explicit
  operator go/no-go, distinct from DoD-green and packet-approval."* The harness's flip precondition (iii)
  operator sign-off token (§4) honors it. Confirmed — affirm, do not re-ask.
- **The mechanism council is 2 seats.** Proposal line 363: *"council seats: breaker, counsel."* The S2
  council was 4 seats — architect, breaker, counsel, **decorrelated security-sentinel** (`resolution.md:4`).
  Reopening S2's auth-cutover posture inside a 2-seat mechanism council is a seat mismatch (condition §C-2).
  Confirmed.
- **The reversibility promise is stated two ways, inconsistently, in the same packet.** The threat-model
  tells the hard truth (*"routing is reversible; committed side-effects are not... it does not un-write a
  row, un-charge a card, or un-revoke a token family"*). The proposal's **goal #2** softens it to *"no
  surface commits an effect a rollback can't paper over"* (`proposal.md:27`) — but a duplicate cash order and
  a DELETEd token family are precisely effects a rollback **cannot** paper over (they are human-cleanup
  artifacts). The honest framing lives in the threat-model; the goals oversell. Confirmed (condition §C-1).

---

## By charge

### 1. Reversibility as a promise — the two-tier truth is honest in the threat-model and soft in the goals

**Affirm the design; fix where the promise is made loud vs quiet.** The harness's central honesty —
routing-reversible, effects-not — is correct and it is the reason this is not a stop. But the promise is
made at two volumes in one packet, and the quieter one is the one a hurried reader meets first:

- The **threat-model** says it plainly and gates on it. Good.
- The **proposal goals** (#2) say *"Reversibility with zero DB divergence... no surface commits an effect a
  rollback can't paper over."* That is not true for the two surfaces that matter — S5 (a duplicate cash order
  is a *second physical delivery a real person is asked to pay cash for*, per S5 counsel §C-2) and S2 (a
  family DELETE evicts a working vendor mid-shift; *"routing back to Node does not un-delete the family,"*
  S2 resolution convergence-3). "Paper over" is the word that lulls. The **name itself** — "reversible
  cutover harness" — carries the same soft promise: it is a *reversible-routing* harness, not a
  reversible-cutover one.

Weighed across lenses (plural): **honesty (`UI/design каже правду`)** — the packet that will be read a year
from now by someone deciding whether a flip is safe should meet the hard truth in its *goals* and its
*name*, not only three documents deep in the threat-model. **Care** — the softer framing is exactly the one
that, under launch pressure, lets someone conclude "it's reversible, flip it" about S5. **Aesthetics as
leading indicator** — a design whose own goal-statement contradicts its own threat-model is not yet
conceptually whole; the fix is not more mechanism, it is one honest sentence promoted to where it is read.

**Disposition (§C-1).** Make the two-tier truth as loud in goal #2 and the ADR title-line as it is in the
threat-model: *routing reverts in seconds; committed side-effects do not — every write surface must prevent
them, and where prevention can fail (S2 family-delete, S5 dup-order, S9 erase) a human-owned cleanup path
exists **before** the flip.* And bind the S5 precedent here at the mechanism level: **each irreversible-effect
surface's flip is a distinct operator go/no-go with a pre-authored, reviewed side-effect cleanup runbook as a
flip precondition** — not a thing invented after, under incident pressure, when a duplicate order is already
out for a second delivery.

### 2. Two revisions of Council canon (Q3, Q4) — honestly raised; must be ratified by the seats that own them

**This is the sharpest charge, and it is a process-integrity finding, not an ethics-of-harm one.** The
harness reconciles three per-surface postures into one mechanism — a genuinely good and necessary act. But
in doing so it **overturns a signed, unanimous S2 decision** (canary → atomic + trip-wire, Q3/R-1) and
**re-homes an S2-owned gate onto S3** (cross-stack verification parity becomes an S3/S4/S5 flip gate, Q4/R-2).
Both are defensible on the merits. Neither is silent — R-1 explicitly says *"it revises a council decision,
so it is not adopted silently"* and routes to operator 🔴. That honesty is why this is a condition, not a
stop. What I insist on is the **route** of the ratification:

- **The revision must amend the S2 record, ratified by the S2 seats — not batch-signed inside this packet.**
  Justice/legibility: a unanimous 4-seat convergence point (including a *decorrelated* security-sentinel and
  an S2 lead) should not be downgraded to a single architect recommendation inside a 2-seat mechanism council
  (proposal:363). The concurrent-refresh argument the harness uses to justify atomic-flip **is the Breaker's
  domain** (S2 gate-iv: cross-stack concurrent-refresh is safe *iff* both stacks run byte-identical refresh
  SQL incl. the `interval '5 seconds'` window) — so the Breaker who wrote that gate should re-verify that
  atomic-flip is genuinely safer, not just differently-shaped. Record the outcome as a **superseding
  amendment in the S2 resolution** ("canary superseded by atomic-flip + revoke-rate trip-wire, ratified
  [date], operator-signed"), so the S2 record self-documents its own change and no future reader finds a
  signed canary in one doc and an atomic flip in another with no bridge.
- **Q4 (verification-parity as an S3/S4/S5 gate) needs the S2 lead in the loop.** It moves a gate S2 authored
  (body-`kid` round-trip both directions, REV-2) onto surfaces that flip *before* S2. That is sound — the
  verify path is stateless and lower-risk than the mint/delete path — but it changes *when and whom* an
  S2-owned gate binds. The packet already names "architect + S2 lead + operator" as owner; hold that.

**Steel-manning the harness's own position (so I am fair to it):** the atomic-flip argument is not a dodge —
a per-request canary genuinely *would* route concurrent refreshes of the **same family** to **different**
stacks, which is the exact cross-stack hazard S2 gate-iv worries about. Atomic-flip keeps a family wholly on
one stack and the trip-wire still catches a revoke-storm and auto-reverts — arguably closer to the canary's
*intent* (watch revoke-rate, back off on divergence) than the canary's *letter*. I find this credible. My
condition is not "reject it"; it is "ratify it where it was decided, by who decided it."

### 3. Safe → risky ordering — right and humane; name the halo trap so S1 does not over-certify the machine

**Affirm S1-first as the ethically correct order.** Proving the mechanism on the read-only surface — where a
bug is a stale menu byte and a re-warmed cache, not a stranger charged twice — before the surface where a
bug is irreversible, is the humane sequence: earn trust on the surface that hurts least. It is the same
spirit as `нуль автобану / людина-в-петлі` — incremental, reversible, low-stakes first. I affirm the
ordering (Q5(a)) without addition.

**But name the temptation the tasking rightly flags, because it is real and it is a Goodhart trap.** S1's
DoD is deliberately light — G8 shadow-diff is "the strongest gate; no write gates" (§6 addendum). S1
therefore exercises **routing + rollback + health-gate + observability**, and *nothing else*. It does not
exercise the write-byte-identity gates, the cross-stack idempotency guard, the **trip-wire auto-rollback on a
real divergence**, or the **side-effect cleanup runbook** (§C-1). So the claim "S1 proves the mechanism" is
true for the *routing substrate* and **false for the safety substrate that money actually depends on**. The
strategic risk: the S1 green halo carries — "the harness is proven" — into S5, where the parts that were
never exercised are exactly the parts that stop a duplicate paid order. Under launch pressure (trigger =
first real paid order), a proven-looking read flip is precisely the thing that makes an operator feel the
money flip is de-risked when it is not.

**Disposition (§C-4).** Prove the *safety machine* — the trip-wire auto-rollback and the cleanup runbook —
on a **reversible write surface (S3)** under **synthetic divergence**, before S5. The ordering already puts
S3/S4 ahead of S5, so this costs nothing but a named gate: don't let S5 be the first time the trip-wire fires
for real or the first time a human runs the duplicate-effect cleanup. And keep S1's honest claim narrow in
the record: *S1 proves routing, rollback, health-gate, observability — not write-parity, idempotency, the
trip-wire, or cleanup.*

### 4. Real people crossing the flip boundary — the packet speaks for the data; someone must speak for the person

The packet handles the flip-instant *correctly at the data layer* (T3: routing affects only NEW requests;
in-flight requests finish on the stack they started; the only residual is a client retry crossing the
boundary, absorbed by idempotency). That is right and I affirm it. What it does **not** yet speak for is the
*person's experience* at that instant. Three stakeholders, by dignity-weight:

- **The courier (least-powerful actor — `кур'єр-гідність`).** An S6 WS flip drops live sockets; clients
  auto-reconnect (T10). For a courier mid-delivery, a dropped socket is a momentary loss of live order
  state / navigation / cash-to-collect — a cost borne by the courier for a rebuild benefit they do not
  share. Auto-reconnect is a real mitigation, but the DoD gate "reconnect-continuity across a flip" must
  prove the **courier's** scenario specifically: an in-flight delivery's assigned-order + cash-to-collect
  survives the reconnect with **zero courier-visible loss**, not merely that a socket reconnects. And the
  cheap humane control: **flip S6 in a low-delivery window** (a time-of-flip constraint in the operator
  sign-off), so the fewest couriers are mid-delivery when their connection is deliberately dropped.
- **The customer mid-order (S5 flip instant).** The idempotency guard absorbs the duplicate *order*; it does
  not shape the *message* the customer sees. §7 says a non-idempotent write that must fail at the flip instant
  fails "with the surface's typed envelope" — which means a customer who tapped "place order" at the wrong
  instant could see an error at the single worst moment (payment). The honest, humane requirement: that
  flip-instant failure is a **truthful, retry-safe** envelope ("something went wrong, please try again" —
  *never* "your order failed" when it may have succeeded), and the client retry is idempotency-keyed so the
  retry is safe. This is `soft-confirm-не-пастка` / `сервер-авторитетний` honesty applied to the flip
  instant: never lie to a person about whether their order landed.
- **The owner.** No outage in the happy path (T3 — new requests route to the DoD-green stack; a bad flip
  *degrades* back to Node, it does not down the store). The subtler cut is **read-after-write staleness**:
  the owner saves a price on Rust (S3) then reads it back through a Node-cached S1 storefront and thinks it
  did not save — a real trust wound ("did my edit take?"). The packet names this as an S3 DoD item; hold it
  and prove it owner-visibly, not just cache-internally.

**Disposition (§C-5).** Bind the flip-instant human experience — courier state-survives-reconnect + low-flip
window; customer truthful-retry-safe envelope; owner read-after-write legibility — as named DoD deltas on
S6/S5/S3. None blocks the mechanism; all three are cheap and all three protect a person who cannot attend
this council.

### 5. Charter, scope, irreversibility

- **Charter: clean.** The harness routes traffic; it moves no data to any AI path, adds no secret, forwards
  `Authorization` not cookies, and leaves RLS `ENABLE + FORCE` untouched (threat-model §4). No military/
  warfare, no surveillance-for-harm, no commons-capture. It *serves* the launch trigger (first real paid
  order) by making the S5 money cutover survivable — aligned with the Charter's spirit, not against it. No
  Charter violation the harness introduces.
- **Shadow-diff privacy — the packet already reached the counsel answer; one sharpening.** Q10(a) restricts
  shadow-mirroring to **read-only, unauthenticated S1 GETs** — no writes, no PII mutation, no auth replay —
  and names counsel as PII owner. Correct and I affirm it. One sharpening: even unauthenticated storefront
  GETs carry a client IP + the location being browsed (weak-PII). Mirroring doubles where that lands. Cheap
  fix: **compare shadow payloads in-memory and discard — never persist the mirrored requests.** Not a stop;
  a one-line discipline note.
- **Scope: disciplined — one creep to watch.** The harness is deliberately *only the switch* (each surface
  keeps its own council); non-goals are clean; the DB-as-reconciliation-point framing is right. The one thing
  that is not purely a seam is the **trip-wires** (S2 revoke-rate, S5 dup-order-rate) — behavioral monitors
  with auto-actions, i.e. a small runtime engine. It is justified (failure-first), but keep it narrow: **one
  metric, one threshold, one action (revert), per surface** — not the seed of a general anomaly-detection
  framework. Name that boundary so "trip-wire" does not grow into a surface of its own.
- **Irreversibility — the whole packet is an irreversibility-management design, and honest about it.** The
  one long-horizon irreversibility nobody priced is in § aesthetics/strategy below (the un-cut vine).

---

## Non-blocking aesthetic / strategic notes

- **The through-line honesty is the design-language high point — say it out loud.** "Routing is reversible;
  committed side-effects are not," stated once and gated on everywhere, is the most whole thing in the
  packet. A cutover harness that told the comforting lie (reversible = safe) would be more elegant on the
  slide and more dangerous in production. This packet chose the harder, truer frame. That is aesthetics doing
  its ethical job — a whole design has fewer seams to leak a false promise through. Affirm it; promote it to
  the goals (§C-1) so the truth is where it is read.
- **The un-cut vine — the long-horizon lock-in nobody asked about.** The harness is explicitly *"the
  strangler vine, built to be cut"* (Phase D — the front-door role migrates to Rust, the shim is removed).
  Each *surface's* overlap is time-boxed. But the **whole-program** overlap — Node remaining the sole ingress
  across all ten surfaces until Phase D — has **no stated end-trigger and no owner**. The strategic risk is
  the oldest one in infrastructure: the "temporary" shim that becomes permanent load-bearing plumbing. A
  rebuild whose *point* is to escape a stack you no longer want to be captive to would, by never cutting the
  vine, plant a **new** permanent incumbent (the Node front-door) — the exact lock-in shape it set out to
  leave. This rhymes with the handoff's one open accountability item (`терпіння↔прив'язаність`): the vine you
  grow attached to is the vine you never cut, and from the inside, "not yet time to decommission Node" and
  "I never actually intend to" are indistinguishable without a pre-committed trigger. **Name a Phase-D
  trigger + owner now** (e.g. "all ten surfaces flipped + stable N days ⇒ front-door role migrates to Rust,
  shim deleted, dated"), while the shim is still young enough that no one is attached to it.
- **"Schema rich, runtime minimal" kept honestly — and the twin to watch.** The `cutover_flags` table +
  ownership map as seams, proxy runtime only on `rust`, is the *good* version of the doctrine (restraint).
  The trip-wire is the edge where it could tip into runtime-you-grow — hold it to one-metric-one-action
  (above) so the restraint stays real.

---

## Steel-man of a rejected option (obligatory)

**The literal per-request S2 canary — the option the harness rejects (Q3(b)), and the one the S2 council
actually signed.**

Its strongest case, made fairly: the S2 council did not pick a canary by accident — it is a **unanimous,
4-seat convergence** with a decorrelated security-sentinel in the room, and the operator *signed it as a
🔴 item*. The canary's virtue is that it makes the parity bug **observable at low blast radius before it is
committed**: route 1% of vendor refresh traffic to Rust, watch the family-revocation rate, and a mis-encoded
reuse-detection that would DELETE families shows up as a revoke-rate spike on *a handful* of vendors, not on
all of them at once. An atomic flip, by contrast, moves **every** family to Rust in one instant — if the
refresh SQL diverges by a byte, the trip-wire catches it *after* some families are already deleted, and a
deleted family is not un-deleted by the rollback (S2 convergence-3). So the canary front-loads the *detection*
before the *commit*, which is the more conservative posture on the one surface (auth deletes) where the
committed effect is genuinely irreversible. The canary also has the governance virtue of being *already
decided by the seats that own it* — choosing it costs zero re-ratification and re-opens no signed decision.

**Why I still land with the harness's atomic-flip + trip-wire — but only via §C-2's route.** Two things the
canary's steel-man underweights. First, the per-request split introduces the **exact hazard** it is trying
to observe: routing concurrent refreshes of the *same* family to *different* stacks is the cross-stack
concurrent-refresh race (S2 gate-iv) — the canary would *manufacture* the divergence it is meant to detect,
on live vendors. Atomic-flip keeps a family wholly on one stack, so the only concurrent-refresh races are
*intra*-stack (which the shared-DB atomic UPDATE already resolves, *iff* the SQL is byte-identical — the
same precondition the canary also needs). Second, the trip-wire preserves the canary's *actual value* (watch
revoke-rate, auto-revert on divergence) without the per-request split, and it degrades toward the incumbent
(the safe direction). So atomic + trip-wire is not "canary abandoned" — it is "canary's intent kept, canary's
one self-inflicted hazard removed." That is a genuinely stronger position. But it **loses its legitimacy** if
it is adopted by fiat in a 2-seat mechanism packet over a 4-seat signed decision — which is why my adoption
of it is conditional on §C-2: ratify it where it was decided, by the seats (esp. the Breaker) who wrote the
gate its safety depends on. The atomic flip is right on the *merits* and wrong if taken by the wrong *route*.

---

## Explicit conditions before code (§C)

Advisory — the human decides. Counsel-weighted, ordered by ethical/epistemic load.

1. **[honesty/naming, §1] Promote the two-tier truth to the goals and the name.** Rewrite goal #2 and the
   ADR title-line so *"routing is reversible; committed side-effects are not"* is as loud there as in the
   threat-model — drop "paper over." Bind, at the mechanism level, the S5/S3 canon that **each
   irreversible-effect surface's flip (S2 family-delete, S5 dup-order, S9 erase) is a distinct operator
   go/no-go with a pre-authored, reviewed side-effect cleanup runbook as a flip *precondition*** — authored
   before the flip, not invented after, under incident pressure.
2. **[governance/epistemic — the sharpest] Route the S2 revision through the seats that own it.** The Q3
   canary→atomic revision must be ratified as a **superseding amendment recorded in the S2 resolution**, with
   the **S2 seats re-consulted** — especially the Breaker who authored gate-iv (re-verify atomic is genuinely
   safer, not just differently-shaped) and the decorrelated security-sentinel — and operator-signed against
   the S2 record. Not batch-signed inside this 2-seat mechanism packet. Same route for Q4 (verification-parity
   as an S3/S4/S5 gate) with the S2 lead in the loop. The revision is defensible; the *route* must be honest.
3. **[care/irreversibility, §4] Bind the flip-instant human experience.** Named DoD deltas: (S6) courier
   in-flight-delivery state (assigned order + cash-to-collect) survives the WS reconnect with zero
   courier-visible loss, and the S6 flip is scheduled in a low-delivery window; (S5) the flip-instant
   customer failure is a **truthful, retry-safe** envelope, never a lie about whether the order landed, with
   an idempotency-keyed retry; (S3) owner read-after-write staleness is closed owner-visibly, not just in the
   cache.
4. **[strategic integrity, §3] Prove the safety machine on a write surface before money.** Exercise the
   trip-wire auto-rollback and the side-effect cleanup runbook on **S3 under synthetic divergence** before
   S5 — so S5 is never the first time either fires for real. Keep S1's claim narrow on the record: S1 proves
   routing/rollback/health-gate/observability, **not** write-parity/idempotency/trip-wire/cleanup.
5. **[long-horizon, § strategy] Give the vine a cut-date.** Name a Phase-D decommission trigger + owner now
   ("all ten surfaces flipped + stable N days ⇒ front-door role migrates to Rust, shim deleted, dated"), so
   the "built-to-be-cut" front-door does not become the new permanent incumbent — the very lock-in the
   rebuild exists to escape. Keep the trip-wire narrow (one-metric-one-threshold-one-action per surface) and
   discard shadow-diff payloads in-memory (never persist mirrored requests).

---

## The question nobody asked (§7)

The packet speaks, carefully and correctly, for the **data** that crosses the flip instant: T3 proves no
request is torn mid-flight, and the idempotency guard absorbs the one residual — a client retry that lands on
the other stack. Every seat in this council speaks for the *row*: one order, one family, one byte-identical
write.

**Nobody speaks for the person on the other end of that residual retry.** The customer who taps "place order"
at the exact instant the S5 flag flips, gets a network blip, and retries — the data model says "one order,
idempotency absorbed it," and that is true. But *she* experienced a payment that appeared to fail at the
worst possible moment, on a system she does not know is being rebuilt, for a benefit (Rust migration) that is
entirely the platform's. The courier whose socket drops mid-delivery, the owner who reads a stale price and
wonders if their edit saved — same shape: the *artifact* is reconciled, the *experience* is unspecified. The
harness guarantees the shared DB stays consistent across the flip; **nothing in it yet guarantees the human
crossing the flip is told the truth about what just happened to their order, their shift, or their edit.**

The unasked question is not technical and it does not block the mechanism: *the harness works hard to
guarantee the two stacks write the same bytes — what guarantees the person whose action crossed the flip
instant is met with honesty rather than a confusing error at the worst moment?* The honest answer is the same
shape as the data answer: make the flip-instant experience tell the truth — a retry-safe message that never
lies about whether the order landed, a reconnect that keeps the courier's delivery whole, a read that shows
the owner their own edit. That sits on the §C-3 DoD with owners, so the person who cannot attend this council
— the one whose order, shift, or edit happened to cross the boundary at the wrong second — is not the one who
discovers the seam.

---
*Advisory only. No ETHICAL-STOP. The human is final. Nothing here authorizes a code change, blocks the
harness, or overrides a conscious operator decision.*
