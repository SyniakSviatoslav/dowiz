# S6-REALTIME/WS Port — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the S6-realtime/WS Triadic Council.
> Advisory, non-blocking. Architect asks "will it work"; Breaker asks "how it breaks";
> this seat asks *should it exist, is it fair/honest/dignified, is it whole, what is the long
> horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no design authority.
> Every load-bearing claim below was verified against live source, not trusted from the packet.

---

## OPINION: **PROCEED-WITH-REVISIONS** · **NO ETHICAL-STOP**

S6 is the surface whose *whole reason to exist* is to keep one tenant's live order feed out of another
tenant's browser, and its cutover is the only one a **live courier crosses mid-delivery**. The packet is
the most self-disciplined in the rebuild so far: it names its own most-likely counsel flag (fire-and-forget
NOTIFY), it collapses the two Node relay guards into one compile-time chokepoint, and it turns the
tri-state authz into a carried invariant rather than re-litigating ADR-0013. I looked adversarially at
every place this surface could cross a grounded line and touch a real person — the courier whose socket
drops mid-delivery, the logged-out courier still watching a customer's live GPS, the owner staring at a
green dashboard that silently missed a new order. **None crosses a grounded red-line in the direction the
line protects**, so the friction here is Opinion, not a stop:

- The **cross-tenant leak** (WS-T1/T2) — the surface's canonical harm — is closed at admission AND
  per-frame, and I do not re-litigate the Breaker's domain (the withhold-vs-relay ordering, the
  in-memory ceiling under DB starvation). The line `сервер-авторитетний` / tenant-isolation is honored by
  the design, not walked into.
- The **logged-out-courier tail** (WS-T8) — a genuine privacy edge (a de-authorized courier receiving a
  real person's live GPS) — is a place I looked hard for a STOP. But the packet's *own* recommended
  disposition (Q1b(b), FIX-IN-PORT) **closes the named vector**. A STOP requires a verified intersection
  where the packet's disposition leaves the line open in the direction it protects; here the packet fixes
  it. Issuing a STOP over a gap the packet already recommends closing would be verdict-not-friction. This
  tracks the **S5/S3** posture (conditions, not stop, when the packet's own disposition closes the line),
  not the **S4** posture (STOP only when the recommended disposition itself left `анонімізувати-не-видаляти`
  open).
- `схема багата, рантайм мінімальний` is kept honestly — `Principal::Channel` exists in the type from day
  one with no head-WS runtime; the two relay guards collapse to one `RelayGuard<Policy>`; the
  `no-raw-courier-ws-send` ESLint rule becomes **module visibility** (the invariant becomes
  *unrepresentable*, not merely tested). That last one is the design-language high point and I affirm it
  out loud (§ aesthetics).

**Why no stop, and where the real friction sits.** Nothing rises to the S4-style verified line-crossing.
The three sharpest things are honesty-of-recovery, not harm-in-design: (1) the packet has **dropped a
piece of already-signed cutover canon** — the low-delivery-window flip the cutover-harness council bound
onto S6; (2) the courier-session fix closes the *reconnect* vector but the packet is silent on the
*mid-stream* residual its own relay guard cannot catch; (3) the fire-and-forget-NOTIFY accepted-risk names
a recovery (refetch-on-reconnect) that **does not cover the loss mode the design makes routine** (a NOTIFY
lost while the listener is reconnecting, sockets still open). All three are cheap to make honest, and none
blocks the port.

---

## Verification note (I read the live source behind every load-bearing claim)

- **The per-frame guard re-authorizes the BINDING, never the SESSION.** `courier-relay-guard.ts:39` — the
  `check` is the tri-state `courierReadVerdict` (live `courier_assignments` binding); `:116-119` relays
  non-courier / non-order members directly and revalidates ONLY couriers in `order:<O>` rooms; the whole
  file re-derives the *binding*, with no `courier_sessions` read anywhere. So a courier **logged out
  mid-stream who still holds the assignment binding keeps receiving frames per-frame** — the Q1b
  upgrade-time session bind closes the *reconnect* vector, not the *mid-stream* one. Confirmed.
- **The owner dashboard has NO non-WS refresh path for a new order.** `DashboardPage.tsx:141-148` — an
  `order.created` WS frame triggers `mergeDelta` + a debounced authed `/owner/orders` refetch;
  `:155-158` — `onReconnect` calls `fetchOrders()` (skipping the first connect). There is **no
  `refetchInterval`, no periodic poll**. Recovery of a *missed* new-order event therefore requires either
  (a) a subsequent WS frame on that room, or (b) an actual socket close→reconnect. Confirmed.
- **A reconnect only fires when the socket actually closed.** `useWebSocket.ts:128-136` — the
  focus/online/visibility `resume` handler reconnects **only if `ws.readyState` is CLOSED/CLOSING**; if the
  socket is OPEN it no-ops. `:86-109` — the socket reconnects forever on a non-1000/1005 close, capped
  backoff 2s→15s + up to 1s jitter (the jittered backoff the cutover leans on). So a lost NOTIFY that does
  **not** drop the socket triggers neither a delta-refetch nor a reconnect-refetch → the dashboard is
  silently stale until the socket eventually drops or the owner reloads. Confirmed.
- **The bus is claim-check / PII-free (Charter-clean).** `DashboardPage.tsx:125-128` — "the realtime bus
  carries NO customer name / phone / item names (claim-check)"; an authed RLS-scoped refetch fills PII off
  the wire. The one live-location-of-a-real-person asset (customer GPS, W4) is relayed to the bound courier
  only, per-frame re-authorized. Confirmed — no PII on the AI-reachable path; `нуль-PII-у-ШІ` is not in play.
- **The honest connection indicator tells the truth about the SOCKET, not the feed.**
  `DashboardPage.tsx:469` — `data-testid="ws-status-dot" data-connected`. It is green exactly when the
  silent-loss mode bites (socket open, NOTIFY lost beneath it). Load-bearing for §7. Confirmed.
- **The cutover-harness council already bound a low-delivery-window flip onto S6.**
  `rebuild-cutover-harness/counsel-opinion.md:302-306` (§C-3): *"(S6) courier in-flight-delivery state
  (assigned order + cash-to-collect) survives the WS reconnect with zero courier-visible loss, and the S6
  flip is scheduled in a low-delivery window."* The S6 packet §9/§13 carries the *state-intact-via-refetch*
  half but **drops the low-delivery-window scheduling constraint** and softens *zero courier-visible loss*
  to *state intact*. Confirmed.

---

## By charge

### 1. The courier crossing the flip (Q5 🔴) — the structure is right; reinstate the dropped canon, sharpen data→person

**Affirm the resolution; do not over-engineer it; restore two operator constraints the packet lost.** The
packet's Q5(b) is genuinely the *gentlest* cutover in the rebuild, and it earns that by structure, not by
special-casing: a WS socket carries **no durable state**, both stacks LISTEN concurrently so there is **no
hard flip moment**, and reconnect-recovery (refetch-on-open) is *already* the designed mechanism for any
disconnect. In-flight delivery survives by construction because assignment / address / cash-to-collect are
DB-authoritative and accept/pickup/delivered are REST (S7). I affirm all of this and explicitly do **not**
ask for connection migration or a new hard reconnect-rate ceiling — the reconnect storm is already bounded
by the FE's jittered backoff (`useWebSocket.ts:101-102`), the per-IP upgrade rate limit, and gradual drain.
Adding a fourth control would be over-provisioning the fear.

**But the tasking's question — "who speaks for the courier, the least-protected actor, during the flip; is
refetch enough?" — has a precise answer the packet half-forgot.** Refetch is enough for *state recovery*;
it is not the whole answer, because the *timing* of the flip is a dignity choice the cutover-harness
council already decided and this packet dropped. Weighed across lenses:

- **Care / `кур'єр-гідність`.** The reconnect gap is "a few seconds" of a **dead live tail** borne by a
  courier on the road for a rebuild benefit they do not share. Bounded and recoverable — but the number of
  couriers who pay it is a *choice*. The cutover-harness bound the humane control: **flip S6 in a
  low-delivery window** so the fewest couriers are mid-delivery when their connection is deliberately
  dropped. The S6 packet's §9 controls (jitter, rate limit, gradual drain) bound the *storm*; they do not
  bound *how many couriers* are on the road at flip-time. That is the low-delivery-window constraint's job,
  and it is missing.
- **Honesty of the DoD.** §13's cutover-concurrency probe proves "in-flight delivery state intact (via REST
  refetch)" — that is the *data* surviving. The canon asked for the *person's* experience: the courier's
  **assigned order + cash-to-collect survive the reconnect with zero courier-visible loss**, not merely
  that a row is refetchable. These are close but not identical: "the row is intact" is proven by a probe on
  a socket; "the courier saw no loss" is proven by walking the courier scenario specifically. The gap is
  the difference between speaking for the data and speaking for the person.

**Disposition (§C-1).** Not a stop — the harm is bounded (a few seconds of stale tail, recovered) and the
structure is sound. Reinstate the two dropped operator constraints as named S6 flip preconditions: **(a)
the S6 flip is scheduled in a low-delivery window** (an operator sign-off constraint, not just a
mitigation-in-prose), and **(b) drain is *gradual* (stop-new + churn-migrate), never a synchronized
mass-close**; and **(c)** sharpen the §13 DoD from "delivery state intact" to the canon's **"the courier's
assigned order + cash-to-collect survive the reconnect with zero courier-visible loss,"** proven by walking
the courier's in-flight scenario, not only a socket-level probe. All three are free — they are canon the
harness already signed; the packet just needs to carry it forward instead of losing it.

### 2. The logged-out courier's live tail (Q1b / WS-T8) — affirm the FIX; name the mid-stream residual; answer the GPS question

**Affirm Q1b(b) as the minimal, correct fix — and do not let it be downgraded to CARRY under scope
pressure.** The finding is real and it is a privacy harm to *two* real people: a logged-out / deactivated
courier keeps a live tail on orders they still hold a binding for — including the **customer's live GPS
stream (W4)** — because WS admission today verifies crypto only, never the REV-1 `courier_sessions`
liveness check that every REST request runs. The minimal control is exactly what the packet proposes:
**reuse the S2 `CourierSession` bind at upgrade** (`extractors.rs:166-208`) — zero new machinery, the bind
is already built, and it makes a WS admission a live-session check at parity with REST. This is the
ponytail answer (minimum viable, reuse what exists), and I affirm it. Re-shipping the crypto-only admission
as "parity" would be the neglect-laundering the S5 seat named — closing a live authz asymmetry that the
port is the natural moment to close.

**But the packet is silent on the residual its *own* relay guard cannot catch, and the tasking's "minimal
control?" deserves the honest boundary.** Q1b closes the **reconnect** vector — the packet's stated finding
(a 14d token outlives the session; the logged-out courier *reconnects* and keeps a tail). It does **not**
close the **mid-stream** case, because — verified above — the per-frame guard re-authorizes the *binding*
(`courier_assignments`), not the *session* (`courier-relay-guard.ts:39,116-119`). So a courier logged out
*while still connected*, who still holds the assignment binding, keeps receiving customer-GPS frames until
the socket drops. Two sub-cases, and the honest disposition splits on one unanswered question:

- **Voluntary logout** self-heals fast: the app closes → the socket drops → the heartbeat kills it in ≤30s
  → no more frames. Bounded, acceptable, no action.
- **Involuntary deactivation** (an owner deactivates a courier, or a session is revoked, while the app
  stays open) is the sharp case, and it turns on **one unanswered question the packet must answer: does
  deactivation drop the assignment binding?** If yes → the relay guard evicts within ≤TTL (~10s), and the
  case is fully closed by the binding path (state it, and affirm). If no → a deactivated courier on an open
  socket **keeps watching a customer's live location indefinitely** until they happen to reconnect (which,
  being deactivated, they will not) — that is the residual, and the cheap fix is the same eviction path the
  guard already has, triggered on session-revocation, not a new subsystem.

**Disposition (§C-2).** Not a stop — the primary vector is closed by the packet's own FIX, the voluntary
case self-heals in ≤30s, and the involuntary case is either already-closed-by-binding-cascade or a bounded,
nameable gap with a cheap in-hand fix. But the packet must **answer the deactivation-drops-the-binding
question on the record** and, if the answer is "no," add the session-revocation eviction (reuse the guard's
existing `evict` path). The line worth naming plainly: a de-authorized courier should not keep watching a
real person's live GPS a second longer than the transport forces.

### 3. Fire-and-forget post-COMMIT NOTIFY (Q4 residual / WS-T6) — the packet's predicted flag, sharper than it states

**This is the one place I raise the packet's own honesty against itself, and it is the sharpest charge in
this document.** The packet is right that CARRY + accepted-risk is the correct *port* call and that a
transactional outbox is a legitimate future hardening, not an S6 deliverable. I do **not** ask for the
outbox here. But the accepted-risk row names a recovery — *"recovery is the client's
refetch-on-reconnect"* — that, verified against the live FE, **does not cover the loss mode this design
makes routine.** The truth is worse than the packet states, and better than a stop.

The named recovery only fires when the crash that loses the NOTIFY is the **same event that drops the
client's socket** — a full process death. In the steady-state monolith that mostly holds (crash → sockets
drop → reconnect → `fetchOrders`). But three loss modes leave the **socket open**, and for all three the
verified FE does **nothing**: no delta arrives (`DashboardPage.tsx:141`), and no reconnect fires because the
socket never closed (`useWebSocket.ts:132`):

1. **A transient NOTIFY loss without a process death** (the producer's session-pool connection blips; the
   fire-and-forget NOTIFY is dropped; the process lives). Rare, but real.
2. **The overlap this packet is built for** (Q5): the producer is on **Node**, the listener + sockets on
   **Rust** — separate processes. A Node-side NOTIFY loss does not drop the Rust-held socket. The very
   window S6 exists to cross is the window where "the crash drops the socket" coverage breaks.
3. **The listener-reconnect gap — the most likely one, and a *designed, routine* event.** §6 makes the
   `PgListener` reconnect on capped backoff, "retried forever." NOTIFY has no backlog; a NOTIFY published
   *while the listener is reconnecting* is silently lost, and the axum process did **not** die (only the
   dedicated session connection blipped), so **every socket on it stays open.** No client reconnect, no
   refetch. This is not a rare crash — it is what happens on every deploy blip, every session-pool hiccup,
   every listener reconnect the design promises to do "forever."

For a **customer** status frame this is benign (self-heals on the next frame; the stale is "still being
prepared," not a wrong charge). For an **owner's `order.created`** — the **launch-trigger event, the first
real paid order the whole rebuild exists to serve** — a silently-lost NOTIFY beneath a green WS dot means a
**missed order**: food never made, customer waiting, owner losing the sale, with no signal anything went
wrong. Weighed across lenses: **honesty** (`UI/design каже правду`) — the accepted-risk claims a recovery
that does not cover its own most-likely loss mode; **care** — the harm falls on the owner and customer, the
two people who can least tell "no orders" from "an order I never saw"; **`готівка→алерт-тертя` in spirit** —
the one event whose silent loss costs a real person deserves a cheap backstop, not silence.

**Disposition (§C-3).** Not a stop — carried property, packet flags it, the dominant crash mode does
self-recover, and the outbox is a legitimate deferral. But two cheap teeth that make the accepted-risk's own
recovery *actually true* rather than aspirational, using seams the packet **already has**: **(a)** when the
`PgListener` transitions degraded→healthy (it already tracks this, §11), **emit a room-wide
`Event::Resync`** to every live member so clients refetch the state they may have missed during the LISTEN
gap — this is the *same* `Resync` seam already built for claim-check + backpressure, fired on one more
trigger (schema-rich, runtime-minimal: the seam exists, use it), and it closes loss mode (3), the routine
one; **(b)** rewrite the accepted-risk row to state the **honest residual** — refetch-on-reconnect covers a
process-crash-that-drops-sockets; it does **not** cover a NOTIFY lost with the socket still open (modes 1–3),
for which the listener-recovery `Resync` is the mitigation and the transactional outbox is the **named
future hardening with a trigger** (not dissolved by the partial fix). The one thing that must not happen is
the rewrite re-shipping "a new paid order can be silently lost beneath a green dashboard" with a footnote no
one reads.

### 4. Charter, scope, and the real people

- **Charter: clean.** WS delivers events; no AI path touches the bus (claim-check keeps PII off the wire —
  `DashboardPage.tsx:125-128`, verified). No military / warfare, no surveillance-for-harm, no
  commons-capture. The one surveillance-adjacent asset — the customer GPS stream (W4) — is scoped to the
  bound courier, per-frame re-authorized, and identity-stripped by claim-check. The single Charter-spirit
  watch-item is §2 (GPS to a de-authorized courier), handled there. It *serves* the launch trigger (the
  first real paid order reaches the owner in real time) — aligned with the Charter's spirit.
- **Scope: disciplined, one creep to watch.** S6 = transport + fan-out authz; the producers (S5/S7/S8) and
  the mint sites (S2 owner/customer, S7 courier) are correctly excluded; no schema change; heads get the
  `Principal::Channel` type seam but no runtime. The one creep: the **connection-lifecycle runtime**
  (Q8 expiry policy + backpressure + the `Resync`-on-recovery I ask for in §C-3) is a small but growing
  runtime. Keep it narrow — one expiry policy, one backpressure rule, one recovery event — so "lifecycle
  policy" does not become a general session-management framework. Same restraint the cutover-harness seat
  asked of the trip-wire.
- **The three real people, by dignity-weight.** The **customer** sees status (least harm; self-heals on
  the next frame). The **owner** sees a new order (the sharpest silent-loss harm — §3). The **courier**
  sees an assignment + a customer's GPS (dignity at the flip — §1; privacy after logout — §2). Every seat
  in this council speaks for the *frame reaching the right socket*; §§1–3 are where I make the council also
  speak for the *person waiting on that frame*.

---

## Non-blocking aesthetic / strategic notes

- **The single-chokepoint-as-module-visibility is the design-language high point — say it out loud.**
  `rooms.rs` exporting no raw send, so the only fan-out write path is the `RelayGuard<Policy>`, converts a
  *tested* invariant (the `no-raw-courier-ws-send` ESLint rule) into an *unrepresentable* violation (you
  cannot write a leaking send; you would have to add a `Policy`, which is exactly the review point we want).
  A whole design makes the harmful thing impossible to express, not merely caught. This is aesthetics doing
  its ethical job — fewer seams for a cross-tenant leak to slip through — and it is the most whole thing in
  the packet. Affirm it loudly.
- **`Resync` is the coherent recovery primitive — make it the *one* recovery concept.** The packet already
  uses `Event::Resync` for claim-check truncation *and* backpressure lag. Firing it on listener-recovery too
  (§C-3) makes it the **single** "the tail gapped; refetch the authoritative state" primitive across all
  three loss surfaces — instead of three ad-hoc heuristics. One whole recovery concept is both more elegant
  and more honest than a green dot over a stale feed. This is "schema-rich, runtime-minimal" at its best:
  the seam exists; extend its trigger, do not build a new subsystem.
- **The independent-flip elegance has an un-cut-vine twin — name the S6 overlap end-trigger.** "Both stacks
  LISTEN concurrently, no hard flip moment" (§9) is a genuine strategic virtue — S6 can flip independently
  of S5/S7. But the same property means the overlap can *quietly persist*: an indefinite dual-listen with no
  end. The session-mode connection budget (§2) is the natural forcing function (you physically cannot run it
  forever), which is good — but make the forcing function *explicit*: **name the S6 overlap end-trigger +
  owner** ("both stacks stable N days ⇒ shed the Node listener's session connection, dated"), so "no hard
  flip moment" does not become "no flip ever." This rhymes with the cutover-harness's un-cut-vine and the
  handoff's open `терпіння↔прив'язаність` item: the overlap you grow comfortable with is the one you never
  end, and from the inside "not yet time" and "never intend to" are indistinguishable without a
  pre-committed trigger.

---

## Steel-man of a rejected option (obligatory)

**Q4 as-written — "CARRY fire-and-forget NOTIFY + accepted-risk, add *no* recovery, defer everything to the
outbox" — the disciplined position my §3 pushes against.**

Its strongest case, made fairly: adding *any* recovery machinery — even the `Resync`-on-listener-recovery I
ask for — to a **port** is scope creep against the whole rebuild's runtime-minimal discipline, and the S6
packet's virtue is precisely that it refuses to grow runtime it does not strictly need. The **dominant**
loss mode (a producer process crash) genuinely *does* self-recover: the crash drops the sockets, clients
reconnect, `fetchOrders` runs, the missed order appears — so the accepted-risk's stated recovery is true for
the case that matters most in the steady-state monolith. A "few seconds of stale" on a *read-tail* is
genuinely acceptable — that is what a tail *is*. And there is a sharper, almost-ethical argument against my
half-measure: a partial recovery (`Resync`-on-recovery) could create a **false sense that loss is
"handled,"** quietly *reducing* the pressure to build the real transactional outbox — so the *most truthful*
posture is arguably the naked accepted-risk (name it plainly, paper over nothing), because a half-fix that
looks like a whole-fix is its own dishonesty. That is a real moral-hazard argument and I do not dismiss it.

**Why I still land on "fire the `Resync` on listener-recovery."** Two things the steel-man underweights.
First, it is **not new machinery** — the `Resync` seam already exists (claim-check + backpressure); firing
it on one more trigger is the *opposite* of runtime growth, it is reuse. Second, and decisively: the loss
mode it closes (mode 3, the listener-reconnect gap) is not the rare crash the steel-man's self-recovery
covers — it is the **routine, designed event** the packet promises to do "forever," and for it the socket
stays open, so the named recovery fires **never**. So the honest posture is not "carry silently" vs "build
the outbox"; it is **"make the recovery you *claim* actually cover the loss you *name*."** The moral-hazard
point is right and I honor it in §C-3 by keeping the outbox a **named future hardening with a trigger**, not
a box the partial fix ticks. The steel-man wins on discipline-in-general and loses on the specific fact that
its own stated recovery has a routine hole its own design digs.

---

## Explicit conditions before code (§C)

Advisory — the human decides. Counsel-weighted, ordered by ethical/epistemic load.

1. **[care/cutover, §1 — reinstate dropped canon] Restore the low-delivery-window flip and the
   courier-visible DoD.** Bind as named S6 flip preconditions: (a) the S6 flip is scheduled in a
   **low-delivery window** (operator sign-off constraint, not prose); (b) drain is **gradual** (stop-new +
   churn-migrate), never a synchronized mass-close; (c) sharpen the §13 DoD from "delivery state intact
   (via REST refetch)" to the cutover-harness canon — **the courier's assigned order + cash-to-collect
   survive the reconnect with zero courier-visible loss**, proven by walking the courier's in-flight
   scenario. All three are canon the harness already signed (`counsel-opinion.md:302-306`); the packet lost
   them and must carry them forward.
2. **[privacy/authz, §2 — affirm the fix, name the residual] Close the courier-session gap and answer the
   GPS question.** AFFIRM Q1b(b) (reuse the S2 `CourierSession` bind at upgrade); do **not** downgrade it to
   CARRY. Then **answer on the record: does courier deactivation drop the assignment binding?** If yes →
   the relay guard evicts within ≤TTL; state it and affirm. If no → add the **session-revocation eviction**
   (reuse the guard's existing `evict` path) so a de-authorized courier stops watching a customer's live GPS
   within ≤TTL, not "until they happen to reconnect." The voluntary-logout case (socket drops ≤30s) needs
   no action; name it as the bounded residual.
3. **[honesty/care, §3 — the sharpest] Make the fire-and-forget recovery *true*, and keep the outbox
   named.** (a) Fire a room-wide `Event::Resync` on the `PgListener` degraded→healthy transition (reuse the
   existing seam) so clients refetch the state lost during a LISTEN gap — closing the routine loss mode the
   named recovery misses. (b) Rewrite the accepted-risk row with the **honest residual**:
   refetch-on-reconnect covers a process-crash-that-drops-sockets; it does **not** cover a NOTIFY lost with
   the socket still open (transient producer loss / the Node-producer↔Rust-listener overlap /
   listener-reconnect gap); the listener-recovery `Resync` is the mitigation, and the **transactional
   outbox is a named future-hardening council with a trigger** — not dissolved by the partial fix. Do not
   re-ship "a new paid order can be silently lost beneath a green dashboard" as a footnote.
4. **[strategic, § aesthetics] Give the S6 overlap a cut-date.** Name the S6-overlap end-trigger + owner now
   ("both stacks stable N days ⇒ shed the Node listener's session connection, dated"), so the
   independent-flip elegance ("no hard flip moment") does not become "no flip ever" — the same un-cut-vine
   the cutover-harness flagged. Keep the connection-lifecycle runtime narrow (one expiry policy, one
   backpressure rule, one recovery event).
5. **[affirm, §§10/5] Leave the cross-tenant core to the Breaker — it is right.** The tri-state admission +
   the one `RelayGuard<Policy>` chokepoint + the module-visibility drift guard are the correct red→green set
   for the surface's whole reason to exist. I add nothing to the withhold-vs-relay ordering, the in-memory
   ceiling under DB starvation, or the OR-9 ≤TTL residual — adding would re-litigate the Breaker's domain.
   Affirm and leave it.

---

## The question nobody asked (§7)

The packet — and every seat in this council — measures S6 from the perspective of **the frame reaching the
right socket and no other**: cross-tenant isolation, per-frame re-authz, byte-identical wire parity. Every
control speaks for the *frame that exists*: does it reach only tenant A, does it survive a reconnect, does
it parse across stacks.

**Nobody speaks for the person waiting on a frame that was never produced into the tail at all — and who is
looking at a green light that tells them everything is fine.** The owner's WS status dot
(`DashboardPage.tsx:469`) reports the truth about the *socket*; it is green precisely when the silent-loss
mode bites (socket open, NOTIFY lost beneath it during a routine listener reconnect). So the honest
indicator **lies by omission at the one moment it matters**: a green "connected" over a stale feed, with no
way for the owner to tell "no new orders" from "an order arrived and the NOTIFY was lost." Every seat
guarantees the frame that *does* fan out goes only to the right person; **nothing yet guarantees the person
reading a healthy-looking dashboard that no frame was silently dropped beneath the green light.**

The unasked question is not technical and it does not block the port: *the surface works hard to guarantee
the frame reaches only the right socket — what guarantees the person reading a green dashboard that no
event was silently dropped beneath it?* The honest answer is the same shape as the delivery answer: couple
the truth-signal to the **listener's** health, not only the socket's — when the listener gapped, the client
is told (the `Resync`, §C-3), so "connected + current" means *both*, not just "socket open." That sits on
the §C-3 register with the `Resync` seam, so the person who cannot attend this council — the owner who lost
a paid order beneath a green light, and never knew — is not the one who discovers the seam.

---
*Advisory only. No ETHICAL-STOP. The human is final. Nothing here authorizes a code change, blocks S6, or
overrides a conscious operator decision.*
