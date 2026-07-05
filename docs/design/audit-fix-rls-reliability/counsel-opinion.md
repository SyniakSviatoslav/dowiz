# Counsel Opinion — GUC/tx discipline · latent-RLS completion · GDPR-erasure liveness · pg-boss reliability

Advisory. The human decides. Aesthetic/strategic notes are non-blocking. ETHICAL-STOPs are friction
(a pause + a recorded human decision), not vetoes, and never override a conscious human.

Scope reviewed: `proposal.md` with its three companion fact sheets (`site-inventory.md`, `rls-state.md`,
`pgboss-state.md`), the source it cites (`anonymizer-gdpr.ts` re-read line-by-line — the LC4 claim
holds exactly as stated), the B3 opinion this proposal is a prerequisite of
(`docs/design/b3-auth-hardening/counsel-opinion.md`), and the v4.5 red lines. This is an unusually
honest and well-verified design — the audit does not flatter itself (it says its own flagship
undercounts the class ~5×). I am not re-litigating robustness (Breaker's lane); most of what follows
sharpens priority, names a horizon left implicit, or answers the four focus questions directly.

---

## 1. Reasoning by lens (only the load-bearing parts)

### Care / harm · justice — LC4 is the sharpest harm in the document, and it lands on the one stakeholder with no voice
Frame it four ways and they converge. **Consequentialist:** one transient error strands a legally
mandated erasure forever, over a green queue — unbounded harm, silent, per-person. **Deontological:**
GDPR Art. 17 is a duty owed to the data subject; the system silently defaults on it. **Care:** the
harmed party is not a system *user* — it is a person who exercised the right to *leave* and to be
forgotten; they have no dashboard, no receipt, no channel, no way to know their data still sits in the
table past its deadline. **Justice:** every other stakeholder in this repo (owner, courier, customer,
operator) has an interface and an alert; the data subject has none. So the deepest silent failure in
the whole audit lands on the only party who cannot see it and cannot complain to the operator. That is
why LC4's priority is not a scheduling preference — it is where the moral weight sits.

This also maps directly onto a *grounded red line*, not just legality: v4.5 §7 worker-liveness — «нуль
false-green». A green queue over work that legally must complete and never does is the exact false-green
the red line forbids, applied to the anonymizer red line («анонімізує-не-видаляє» presupposes it
actually *runs*). The proposal already sees this (§3, §6: "same-day-able," "council fast-track"). My
contribution is to make that priority binding, not aspirational — see ES-1.

### Honesty / epistemics — the deferred B3 flip was believed to isolate credentials; on the tables that matter it isolates nothing
This is the proposal's most important disclosure and it deserves to be louder than a §0 prerequisite
note. `rls-state.md §2/§7` is unambiguous: `couriers` (password_hash + encrypted PII) and
`courier_sessions` (token_hash, family_id) **have no RLS enabled anywhere**. A table with RLS disabled
is not protected by removing BYPASSRLS from the role — there are no policies to enforce, so any role
with a SELECT grant reads every row regardless of the flip. The deferred `ALTER ROLE … NOBYPASSRLS`,
flipped as-staged, would be **isolation theater on precisely the credential/PII tables it was believed
to protect.** The B3 opinion already named the softer version of this hazard ("someone reads 'RLS
enforcement shipped' and relaxes vigilance… how a defense-in-depth layer becomes a liability"). This
audit reveals the hazard is worse than B3 knew: the belief isn't just premature on some lanes, it is
*false on the crown-jewel tables*. That is a load-bearing false belief attached to an irreversible act
(open-sourcing, ADR-020) — grounded enough for ES-2.

### Availability vs correctness — the MIG-2 ordering risk is where a control designed to protect can take the whole product down
The proposal is correct that a mis-ordered anon-scoping migration fails *closed* (empty storefront /
broken checkout), not open (a leak) — the safe direction. But weigh who bears it: a fail-closed anon
checkout is **the launch-trigger surface down for every tenant at once** («перший реальний платний
заказ» is *the* success metric, v4.5 §9), on a solo-operator N=1 system with no support team to catch
it fast (v4.5 §2). The design's ordering is right; my worry is *executability*. The proposal governs the
hard edge "(5) before MIG-2/3" with documented discipline. Documented discipline is a human-memory
guardrail — and human memory failing across a multi-week ramp is the exact failure class that produced
the 49-site GUC mess this proposal exists to clean up. Cheap to gate, catastrophic to forget: the
textbook case for a deterministic gate. Not an ETHICAL-STOP (availability of checkout is not itself a
catalog red line, and the design crosses none — it identifies and sequences the risk correctly). But
firm advice: mechanize the hard edge. See advice #2.

### Long horizon / strategy — the load-bearing unexamined assumption is "one tired human executes a 7-step ordered ramp without a gate at each edge"
The design's *correctness* is real. Its unstated premise is that a solo dev, no review team, no support
(v4.5 §2), reliably executes helper → shape-B → shape-A → MIG-1/4 → public-conversion → MIG-2/3 → flip
in order, across weeks, with every hard edge held in his head. That premise is the single biggest risk
and it is the *same* "explicit-but-forgettable" pattern that created the mess. The strategic move is to
convert every hard edge (§6) from prose into a red test that blocks the next step. This also serves the
launch trigger over polish: LC4 + the detector-integrity slice of pg-boss earn their cost now (legal +
false-green); the 49-site + 4-migration flip work is open-source-/scale-blocking, not
first-paid-order-blocking (B3 opinion established this and it holds) — so it should not gate the fast
fixes. Which is the scope question (Focus 4): **split.** See advice #1.

### Aesthetics / integrity
One genuinely elegant move: the `anonymous: true` ctx variant (§1.2) makes "no context" an *auditable
decision* rather than an accident — it turns the fail-open anon hole into an explicit marker at the call
site. That is the "honest UI is beautiful" principle applied one layer down, to the data-access
grammar; endorse it. The tension worth naming: 49-site conversion + 4 red-line migrations + a pg-boss
sweep is not one coherent object — it is three, welded. "Schema rich, runtime minimal" and the ponytail
restraint both argue for three legible objects with three revert boundaries over one mega-lane whose
blast radius is unreviewable and whose rollback is all-or-nothing.

---

## 2. ETHICAL-STOPs (grounded red lines only)

**ES-1 — The LC4 legal-erasure fix must ship decoupled and now; it must not inherit the review latency
of the flip-coupled RLS/migration work.**
Grounding: a *live* legal-compliance gap (GDPR Art. 17 erasure that silently never completes) + the
worker-liveness red line «нуль false-green» + the anonymizer red line. The proposal itself says LC4 has
no flip dependency and is same-day-able (§6). The red-line *crossing* is not in the fix — it is in the
**bundling**: folding a must-do-now legal fix into one council lane that also carries 4 red-line
migrations and a 49-site conversion means the legal fix waits behind the riskiest, slowest component.
This is exactly the pattern B3-ES-1 stopped for credential rotation ("bundling a must-do-today thing
with design-dependent work can delay the must-do-today part"). This STOP pauses only the *framing*:
record a human decision that LC4 (§3, plus the pg-boss boot-isolation + real `deadLetter` wiring its
own safety net depends on — see advice #5) ships as standalone fast-track, its go recorded separately
from any approval of the RLS/flip lane. It blocks nothing; it refuses to let the legal fix default into
the big lane's latency.

**ES-2 — Record explicitly, before any "we have RLS isolation" claim informs the open-source decision,
that the deferred B3 flip did NOT isolate the credential tables — and gate open-sourcing on MIG-1/MIG-4
landing.**
Grounding: honesty / no-false-sense-of-security + the hardening red line (cross-tenant = 0 *everywhere*;
these tables hold `password_hash` / `token_hash` / encrypted PII, `rls-state.md §7`) + irreversibility
(open-sourcing per ADR-020 is on record as the end goal, and B3's value was established as
open-source-blocking). The failure this STOP guards is a *belief*, not a line of code: if the operator
carries forward "B3 staged ⇒ credentials isolated" into the irreversible act of publishing the source,
anyone reading the diff can see `couriers`/`courier_sessions` have no RLS at all. The proposal surfaces
the gap correctly but frames it as changing B3's "preconditions" — a framing soft enough that the
false-isolation belief can survive it. The friction: a recorded acknowledgment that the credential
tables are *not* RLS-isolated today, and that no open-source flip proceeds on the isolation premise
until MIG-1 (couriers/courier_sessions FORCE) and MIG-4 (token-bearing ENABLE-only tables) land and P1
proves them. This blocks the migrations from *nothing* — they are the fix — it blocks the *silent
carry-forward* of a false belief into an unrecoverable act.

Neither STOP overrides a conscious human. Both demand the decision be *written*, not defaulted into —
consistent with how B3-ES-1 decoupled rotation.

---

## 3. Non-blocking advice (aesthetic / strategic)

- **#1 · Split the lane into three (Focus 4 verdict: split).** (Lane 0) `withTenantTx` helper + the
  `no-bare-set-config` lint gate — a pure, reversible, low-risk addition that is the *shared enabler*
  both LC4 (§3.5) and the RLS conversion depend on; land it first and both build on it independently.
  (Lane A) LC4 + pg-boss boot-isolation + real `deadLetter` wiring — legal + detector-integrity, no
  flip dependency, fast-track (ES-1). (Lane B) 49-site GUC conversion + the 4 RLS migrations + flip
  preconditions — large, flip-coupled, staged/soaked per §6. Three objects, three revert boundaries;
  the mega-lane's all-or-nothing revert is itself a risk on a no-review shop.

- **#2 · Mechanize the MIG-2/3 hard edge; don't govern it by prose (Focus 3).** The failure is
  loud-on-staging, not subtle — so reuse the net you already have: make the existing Ship-Discipline
  staging anon-checkout E2E a *named blocking precondition* on prod application of MIG-2/3 ("MIG-2/3 may
  not apply to prod unless the anon-checkout E2E is green on staging with MIG-2/3 already applied
  there"). That converts "remember (5) before MIG-2/3" from human memory into a red test that blocks the
  deploy — proportionate, no new machinery (ponytail restraint over a bespoke gate).

- **#3 · Attach a human resolution owner + SLA to the LC4 dead-letter terminal, and reconsider the
  `failed` label (Focus 1, the DLQ question — answered directly).** Yes: an un-erased record parked in a
  DLQ *is* a compliance state that needs handling — it is a **breached, open legal obligation, not a
  closed ticket.** The design does the hard part right by making it loud (O-GDPR → DRIFT + Sentry, "an
  ops page, not a log line"). But loud ≠ resolved: a `failed` row that no owner is bound to resolve
  re-creates the exact false-green pathology one ring out — a muted alert over an un-erased person is
  the `in_progress`-forever strand with a nicer name. Three fixes, none blocking the core liveness fix:
  (a) bind a human owner + a resolution SLA to the `failed`/dead state (this is what turns "no auto-retry
  from dead — human lever" from park-and-forget into an actual lever); (b) reconsider the *label* —
  `failed` reads in system-speak as "done badly, move on," whereas the truth is "unresolved legal
  obligation"; a state that cannot be triage-closed as ordinary failure tells the DB the compliance
  truth; (c) design-note the Art. 12 data-subject obligation (if you cannot act on an erasure you may
  owe the subject a notice with reasons). **This is a near-ES:** it *becomes* an ETHICAL-STOP if shipped
  with dead-letter-as-terminal and no resolution owner, because then the fix quietly reintroduces the
  pathology it cures.

- **#4 · On the settlements money path specifically, prefer the narrower helper form.**
  `courier/settlements.ts:25,59,75` is the worst-shape site (pool.query — GUC and payout SELECT on
  *different connections* even today) and it is a *money* path. The proposal's own fallback applies here
  with force: on this site, a wrong-key mistake in the exclusive union is a cross-tenant *payout* read —
  a money red line. Adopt Option-2's named form (`withCourierTx`) for the settlement/money lane even if
  Option 1's generic union wins elsewhere; make the cost of a coin-flip a compile error, not a runtime
  hope, where the coin is money.

- **#5 · Ship pg-boss boot-isolation *with* LC4, not in the later queue sweep.** LC4's entire safety net
  (the O-GDPR reconciliation check) runs on the same worker infrastructure whose boot-fragility is H3 —
  `pgboss-state.md §3`: one throw in any of ~23 sequential registrations amputates every later worker
  *including reconciliation*. So LC4-fixed-without-boot-isolation has a safety net that can itself be
  silently amputated at boot. The detector that proves LC4 works must be un-amputatable *first*.

---

## 4. Steel-man of the rejected option — Option 3 (AsyncLocalStorage request-context auto-injection)

The proposal rejects ALS as "magic that hides the transaction boundary… breaks for workers/webhooks…
violates boring/proven/reversible." The strongest case *for* ALS, made fairly:

The root cause of all 49 sites is that context management is **manual and per-site**, so (proposal's own
words) "each new site is a coin-flip." Option 1/2 shrink the coin-flip to "did you call the helper" — but
they do not eliminate it. The lint gate fires on the *string* `set_config`; it cannot catch the more
insidious failure of a bare `db.query` that *should* be tenant-scoped and simply **isn't wrapped at
all** — there is no `set_config` to flag, so the query silently runs context-free. ALS + a query-layer
interceptor is the **only** option that fixes the *default* rather than adding a *discipline*: if context
lives in ALS, every query inherits the right tenant txn automatically, and "forgot to wrap" ceases to be
expressible. For a review-less solo shop (v4.5 §2), *safe-by-default* is a categorically stronger virtue
than *explicit-but-disciplined* — because the current disaster is a monument to how explicit-but-
forgettable fails. The framework precedent is strong (ALS/middleware-scoped tenancy is how mature
multi-tenant systems actually enforce isolation). The objections are also softer than stated: ALS does
not "break" for workers/webhooks — they *establish* context at their entry root (the same one place they
would call the helper) and everything downstream inherits it; and ALS is just as reversible as a helper
(internal wiring, no schema or wire-contract commitment). "Magic hides the boundary" cuts *both* ways —
the bug class exists precisely because the boundary is visible-and-therefore-forgettable; sometimes the
safest boundary is the one you cannot forget to draw.

Where ALS genuinely loses, honestly: (a) the B3 hook wants **one txn choke point** to prepend
`SET LOCAL ROLE`, and ALS's transparent pool.query makes the txn boundary implicit — solvable but
murkier than an explicit helper; (b) **traceability under failure** — a wrong/absent-context query gives
the explicit helper a stack frame at the call site, whereas ALS gives "somewhere upstream context was
lost," which is harder for a lone operator to trace during a cross-tenant incident; (c) Node ALS has
historical async-boundary context-loss / perf sharp edges. So the rejection is *defensible* — but the
stated reason should be "we want explicit traceability and a clean single B3 choke point," **not "magic
is bad."** And the strongest synthesis is a *hybrid* the proposal is already one step from: Option 1 as
the explicit choke point, **plus** an ALS-backed dev/test tripwire that asserts "a tenant-keyed query ran
with no context in scope" — capturing ALS's safe-by-default *canary* value without ALS's production
magic. The §1.5.4 checkout NULL-GUC assert is a narrow instance of exactly this; generalize it and you
harvest the best of the rejected option.

---

## 5. The open question no one asked

**The entire program is built to make failure visible to the operator and to the system's detectors —
so who makes an erasure visible to the person it was for?** Every state in the GDPR design
(`pending` / `in_progress` / `completed` / `failed` / dead) is operator-facing; every alert (O-GDPR,
DRIFT, Sentry) points *inward*. The one stakeholder the feature exists to serve — the data subject who
asked to disappear and legally must — is structurally absent from every state, every proof, and every
alert. When their erasure strands or fails, no signal reaches them, and they have no way to verify they
were ever forgotten. The whole audit's ethic is "make silent failures loud"; the loudest instance of
that ethic stops at the operator's console and never reaches the person. Should a right-to-erasure carry
a *subject-facing receipt* — a verifiable confirmation that erasure completed, or an honest notice that
it could not and why — rather than being a purely internal operator state machine? Nobody costed the
difference between "the operator can see it completed" and "the person can trust they were forgotten,"
and the deepest silent failure in this document lives in exactly that gap. (Distinct from B3's open
question, which asked what the *owner* sees on a fail-closed denial; this asks what the *data subject*
sees on their own erasure — the perspective absent from the entire document.)
