# Counsel Opinion — B3 Deep Auth Hardening

Advisory. The human decides. Aesthetic/strategic notes are non-blocking. ETHICAL-STOPs are friction
(pause + a recorded human decision), not vetoes, and never override a conscious human.

Scope reviewed: `docs/design/b3-auth-hardening/proposal.md`, against its prior art
(`pg-privilege-hardening/remediation-plan.md`) and the recorded incident
(`secrets-exposure-incident-2026-07-03`). This is a strong, honest design document — most of what
follows either sharpens it or names a horizon it leaves implicit. I am not re-litigating the
robustness work (that is the Breaker's lane).

---

## 1. Reasoning by lens (only the load-bearing parts)

### Care / harm — fail-closed anti-orphan vs. an owner locked out mid-service
The sharp harm is concrete: a restaurant owner in a live dinner rush whose dashboard silently goes
**empty** because a tenant row (an order) fails a location-scoped policy. Empty is worse than an
error, because empty is *plausible* — the owner may act on it (tell a customer the order was lost,
refund, re-fire the kitchen). That is a real person harmed by a security control.

The proposal handles the *structural* version of this well: RC2 carve-out prevents auth-table lockout
(no login death), Option B reverts a bad lane by flag in seconds, the 0-row anomaly metric is meant to
page within a minute, and the owner lane is the first, most-watched canary. On proportionality the
security gain is genuine (a second net under a missed `WHERE`) and the lockout risk is bounded and
reversible — this is friction, not a verdict, with the human kept in the loop. **Not an ETHICAL-STOP
in its designed form.**

Two residual harms the design under-covers:
- The pre-flip orphan audit (§3 C1) is a **point-in-time snapshot**. Rows orphaned *between* the audit
  and the flip — or created NULL-keyed by an un-wrapped write path during the ramp — vanish silently.
  A one-time scan does not cover a live, moving system.
- The denial is designed as an **ops signal, not a user-facing truth**. A 0-row that is actually a
  security denial should not present to the owner as an ordinary empty list. (Picked up as the open
  question — it touches the "UI tells the truth / server authoritative" line.)

### Honesty / epistemics — is partial enforcement a false sense of security, worse than none?
The proposal is admirably candid: §3 says the un-wrapped path is "exactly as (in)secure as today, no
worse." On **confidentiality** that is correct — partial enforcement is a strict subset improvement,
not a regression, and per-route app-layer checks are kept as belt-and-suspenders (R-10, §8). So it is
not worse-than-none on the axis people usually mean.

The honest "worse" is on a *different* axis: during the ramp you pay a **new availability risk** (pool
pinning, orphan denial) for only *partial* confidentiality gain — you carry cost before you carry full
benefit. And there is a real organizational hazard: someone reads "RLS enforcement shipped" and quietly
relaxes vigilance on lanes that are not yet wrapped. That is a *communication* failure, not a design
one — but it is exactly how a defense-in-depth layer becomes a liability. The mitigation is cheap and
belongs in the design: make per-lane enforcement state operator-visible so "which lanes are actually
enforced" is never a matter of belief (see advice).

### Long horizon / strategy — does B3 earn its cost vs. just closing the leak + CI-hardening?
Effort here is large and almost entirely red-line: ~20 policies, ~15–17 worker rewrites, role
machinery, drift reconciliation, a choreographed flip — all operator-gated. Against that, app-layer
authz already exists. For **today's** MVP (tens of tenants, first paid order = the launch trigger) B3
is *not* launch-blocking; the app layer suffices at this scale. The cheapest, most certain
harm-reduction is unambiguously (a) close the live leak and (b) the CI secrets gate — those address a
*certain, present* exposure; B3 addresses a *hypothetical future* one. The proposal's own sequencing
agrees (leak + SAFE-NOW first, RLS ramp last), which is correct.

But the horizon that makes B3 worth it is the **open-source goal** (ADR-020). A multi-tenant authz
model whose source is public is one missed predicate away from a cross-tenant leak that *anyone can
find by reading the diff*. App-layer-only authz is defensible while the code is secret; it is
strategically fragile the moment the code is public. So B3's value **rises with the open-source
milestone** — it is open-source-blocking / scale-blocking, not first-paid-order-blocking. Naming that
horizon correctly is the whole strategic point: do it deliberately as a precondition of responsibly
open-sourcing, and do not let its size crowd out the cheap-certain leak closure that must happen first
regardless.

### Dignity / autonomy — courier
RC4 admits courier writes under `app.current_tenant`, equal to the courier's existing app-layer reach.
No new tracking, no surveillance-creep, no extension of what is watched — it mirrors, it does not
widen. **Dignity-neutral.** Clean.

### Aesthetics / integrity
The "schema rich, runtime minimal" restraint is honoured for the *policies* (dark, inert, proven
before the flip). But note the tension the steel-man below turns on: Option B introduces a *temporary
runtime scaffold* (a second role, `SET LOCAL ROLE` per txn, per-lane flags, a raw-pool wrap) that must
later be built, reasoned about, and torn down at convergence to A. That scaffold is itself surface that
can harbour bugs. Elegant during the ramp; the end-state (Option A) is the simpler object.

---

## 2. ETHICAL-STOPs (grounded red lines only)

**ES-1 — Credential rotation is incident response, not a B3 sub-task. Decouple it now.**
Grounding: a human decision is already on record ("ROTATE both Supabase passwords NOW... independent
of open-sourcing. Do this FIRST" — `secrets-exposure-incident-2026-07-03`) + live standing exposure
(anyone with a repo clone holds the prod credentials). The proposal folds rotation into item #4 as
"rotate + re-role," a HIGH-RISK B3 line entangled with a design decision (whether to retire
`deliveryos_api_user` and promote `dowiz_app`). Two problems: (a) **the postgres SUPERUSER password —
the most dangerous leaked credential — is not named in the proposal at all**; the plan speaks only of
`deliveryos_api_user`, which the remediation plan itself calls legacy/nologin (possibly the *least*
live of the leaked creds). (b) Bundling a must-do-today rotation with a design-dependent re-role can
delay the must-do-today part behind design approval.
This STOP pauses only the *framing*: the pure rotation of **both** leaked credentials (superuser
included) plus the history scrub + CI secrets gate must proceed as standalone incident response,
independent of B3's approval, its rollout, and the re-role decision. Record the human's go on the
rotation separately from any B3 sign-off. The re-role (which login role becomes canonical) is a genuine
B3 decision and may stay here.

**ES-2 (conditional) — Do not flip any lane to enforcement on PROD until silent-denial is detectable
and reversible, and the orphan audit is re-run at the flip moment.**
Grounding: friction-not-verdict + human-in-the-loop + server-authoritative / UI-tells-the-truth. A
fail-closed denial of an owner's *own legitimate data* that is (i) undetected and (ii) presented as an
ordinary empty state is no longer friction — it is a silent verdict the human never sees, and the
server is authoritatively *wrong* while looking right. That crosses the line. The condition to clear
the STOP: (a) the 0-row anomaly metric + per-lane flag-revert are proven **live on prod** (not just
designed), and (b) the NULL-key orphan audit is re-run **at** the flip, not only once earlier, because
the system is live and moving. This is a gate, not a veto — cleared by proof, and the human sets the
flip.

Neither STOP blocks a conscious human who chooses to proceed with the risk recorded. They demand the
decision be *made and written*, not defaulted into.

---

## 3. Non-blocking advice (aesthetic / strategic)

- **Make enforcement state visible, not believed.** Surface per-lane `RLS_ENFORCE_*` state on an
  operator-facing health line (§9 already logs it on boot — promote it to a live readout). This is the
  cheap antidote to the "we shipped RLS" over-confidence hazard.
- **Turn the orphan audit into a continuous gate.** Re-run the NULL-key scan at the flip and keep a
  standing check for NULL-keyed inserts on FORCE-RLS tables during the ramp — a snapshot cannot guard a
  live system.
- **Name the horizon in the doc.** State explicitly that B3 is open-source-/scale-blocking, not
  first-paid-order-blocking, so it is prioritised against the leak-closure and CI work correctly rather
  than by its size.
- **Prefer the simpler end-state sooner if the proof harness is as strong as claimed** (see steel-man).
  The value of Option B is the canary; if per-lane confidence is cheap to reach, minimise the lifetime
  of the temporary dual-role scaffold.

---

## 4. Steel-man of a rejected option — Option A (big-bang role flip)

The proposal rejects A for blast radius and keeps it only as the convergence target. The strongest
case *for* A as the primary, made fairly:

The remediation plan already mandates an **exhaustive pre-flip proof harness** — every policy proven by
impersonating the future state (`SET LOCAL ROLE dowiz_app` + GUC, then `ROLLBACK`) so that "the flip
itself becomes a low-information confirmation, not a discovery event" (remediation plan §Per-phase
verification). If that proof is real, A's blast radius is *theoretical*: there is nothing left to
discover at the flip. Given that, A wins on the virtues Counsel is supposed to weigh:

- **Simplicity / integrity.** A writes *no* temporary scaffold. Option B builds a second role, a
  per-txn `SET LOCAL ROLE`, per-lane flags, and an explicit wrap for raw-pool anon reads — machinery
  that must be built, tested, and later removed, and that is itself new surface for bugs (SET LOCAL ROLE
  reset semantics, grantee-mapping, the un-wrapped raw paths). "Schema rich, runtime minimal" arguably
  favours A: the policies are the schema; A adds *zero* runtime machinery, B adds a runtime layer it
  then has to tear down.
- **Rollback under pressure.** The proposal calls A's rollback "all-or-nothing," but `ALTER ROLE ...
  BYPASSRLS` is a *single lever, seconds to pull* — arguably *easier* to execute in an incident than
  "which of N lanes regressed, flip that flag." One clear reversal beats a decision tree at 20:00 on a
  Friday.
- **Detection is orthogonal to ramp style.** The 0-row anomaly metric protects you the same way whether
  the flip is atomic or laned. If detection is the real safety net (it is), the ramp granularity buys
  less than it appears to.

Where A genuinely loses, and B earns its keep: A's failure, if the proof *misses* a path, is a
fleet-wide partial outage that looks like benign empty data — the worst class to spot. B bounds that to
one lane and one tenant-class at a time. So the honest synthesis is: **B is right precisely to the
extent you distrust the completeness of the pre-flip enumeration.** If the enumeration is trusted, A is
the cleaner, simpler, lower-total-complexity path and the recommendation to converge to A "after a long
soak" may be over-cautious — the soak could be shorter than proposed. If the enumeration is *not*
trusted, that distrust should be stated as the actual reason for B, rather than "blast radius" in the
abstract. (Option C — shadow pool — is correctly rejected: it doubles the connection budget against §2
for a small system, adds a routing surface, and its canary value is already delivered by B's per-lane
flags at no connection cost. I do not restore C.)

---

## 5. The open question no one asked

**When fail-closed denies an owner their own legitimate row, what does the *owner* see — and is that
honest?** The design treats a 0-row denial as an operator metric, but never as a user-facing truth. An
empty dashboard that is actually a security denial is a UI that lies by omission: it says "you have no
orders" when the truth is "we cannot currently show you your orders." An owner who acts on the false
empty (cancels, refunds, re-fires) is harmed by a control meant to protect their tenant. Should a
fail-closed tenant denial be *distinguishable* to the user — a "temporarily unavailable, contact
support" state rather than a silent empty — so the person can tell "nothing here" from "something is
wrong here"? Nobody costed the difference between those two empties, and the whole harm of ES-2 lives
in that gap.
