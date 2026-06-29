# Counsel Opinion — B4: platform-admin gate on `/api/admin/*`

- Re: `docs/design/admin-platform-authz/proposal.md` (Option D — `requirePlatformAdmin` per-request
  re-check + DR-drill hardening)
- Role: advisory (Counsel). Aesthetic/strategy = non-blocking. ETHICAL-STOP = friction requiring a
  recorded human decision, NOT a veto and NOT a forever-block.
- Bottom line: **the gate is a net REDUCTION in capture/abuse risk and should ship.** One grounded
  ETHICAL-STOP is raised as friction — it does **not** block the v1 gate; it asks for one recorded
  human decision (tenant legibility floor) with a date.

---

## 1. Reasoning per lens (only what's load-bearing)

### Ethics / Charter
The charter line in play is the fourth: *"AI is a collective human tool… never captured for the
exclusive benefit, control, or enrichment of any narrow group, and never turned against the people it
was learned from."* Plus the standing **least-power** discipline.

- **The gate IMPROVES the charter posture, it does not degrade it.** Today every one of (potentially
  thousands of) tenant owners can read fleet-wide backup inventory, all-tenant public phones +
  fallback config, and cross-tenant notification audit. *That* is the capture/leak. Replacing
  "all owners see everything" with "≤5 named, audited ops principals" is a strict narrowing of who
  holds cross-tenant power. Correct direction.
- **Least-power is honored in the mechanism, and elegantly:** one boolean authority (not a role
  engine), no 4th token role, no mint site, **no API write path to the allowlist at all**
  (self-serve escalation is structurally impossible, §8). This is "schema rich, runtime minimal"
  applied to authority itself. Genuinely good.
- **Where the charter still bites — the asymmetry of the watcher.** The platform-admin is the
  operator; the audit log (`platform_admin_audit_log`) is readable only inside a platform-admin
  context (`pa_audit_read USING (true)`); the **restaurants whose fleet data is spanned have zero
  visibility and no channel to learn of or contest access.** "Watched by a watcher you cannot see and
  cannot contest" is the structural *shape* of capture the charter names — even when each individual
  access is legitimate ops work. This is the one place the proposal trades a real value (tenant
  legibility) for simplicity without recording the trade as a human decision. See ETHICAL-STOP-1.
  I am honest that the data spanned here is mostly **ops-metadata** (backup inventory, public phones,
  fallback config, notification counts) — not the deep PII the surveillance red-line guards (GPS,
  addresses). That is precisely why this is **friction, not veto**.

### Aesthetics / coherence
- **The core move is beautiful and that is a quality signal.** "The admin plane *is* the platform
  plane → uniformly platform-admin-only; we do NOT branch tenant-vs-platform scoping inside admin
  handlers" removes the exact in-handler-authZ-branching that breeds BOLA. Single-purpose gate,
  one predicate before any query. Conceptual integrity is high; fewer moving parts → fewer leaks.
- **One inelegance worth naming (non-blocking):** `pa_audit_read USING (true)` means any
  platform-admin reads *every* audit row, including other admins' and their own. The mechanism that
  is supposed to watch the watchers is readable only by the watchers and writable-away by no one but
  also reviewable by no one else. Fine at N=1–5; conceptually it is the one asymmetry in an otherwise
  symmetric design.

### Strategy / operability
- **ADR-0004 consistency is a real strategic asset** — same `status='active'` immediate-revoke
  model, same enforcement philosophy. Lower cognitive load, one mental model for "server-authoritative
  liveness." The BOE that justifies the per-request read on a cold plane (while ADR-0004 rejected it
  on the hot plane) is correct and well-argued.
- **Building the audit table NOW (before it's strictly needed) is strategically right.** The trust
  model "operator IS platform-admin" is acceptable at founder-scale, but it silently changes the day
  the **first non-founder ops hire** gets a grant. The audit trail must pre-exist that moment, not
  chase it. The proposal does this. Good long-horizon instinct.
- **Operability footgun (non-blocking, real):** the `ADMIN_PLANE_ENABLED=false` kill-switch darkens
  the *whole* admin plane — including `GET /backups`, `dr-report`, `backups/verify`. Those are your
  **recovery tools**. Darking your DR/recovery plane during the incident where you most need to list
  and verify backups is the wrong-way-round failure. Consider scoping the kill-switch to the
  weaponizable write/drill endpoints, or splitting "dark the plane" from "dark the destructive ops."

---

## 2. ETHICAL-STOP (1) — friction, not veto

**ETHICAL-STOP-1 — Tenant legibility of platform access (record a human decision; do NOT block the gate).**

- **Grounded line:** charter — *"never captured for the exclusive benefit/control of any narrow
  group, and never turned against the people it was learned from."* The restaurants are "the people
  it was learned from"; the platform-admin tier spans their fleet data; the audit that legitimizes it
  is **self-watched** (operator = admin = auditor = the only reader of the audit, §5
  `pa_audit_read USING (true)`) and **invisible to the affected tenant**, with no notification and no
  appeal channel.
- **Concrete abuse scenario (not invented):** a platform-admin pulls `GET /fallback/health` =
  every tenant's name/slug/**public phone**/fallback_config, plus cross-tenant `notification-audit`,
  for a non-ops reason (curiosity about a competitor restaurant they also own; due-diligence for an
  acquirer; a grudge). Every action is "valid platform-admin," every action is logged — to a log only
  the same person can read. Nothing in v1 lets the spanned restaurant ever know or object.
- **Why friction, not veto:** the gate is a net improvement and the data is ops-metadata, not deep
  PII. Blocking a launch-blocker security fix on transparency would be disproportionate friction.
- **What the STOP requires (one recorded human decision + a date — then ship the gate regardless):**
  pick the **minimum legibility floor** and write down which, and when:
  1. **Audit egress out-of-band** — append-only mirror of `platform_admin_audit_log` to a sink the
     platform-admins cannot silently rewrite (so the watcher is watched by *something*). Cheapest,
     highest leverage.
  2. **Scope-minimization within platform-admin** — is uniform all-6 access actually least-power, or
     should the two heavy DR endpoints require a second confirming admin / break-glass? (Deferrable,
     but name it.)
  3. **Tenant right-to-know** — any future channel by which a restaurant learns platform-ops accessed
     its fleet record. Explicitly may be "deferred to date X / tenant-count Y" — but record that it
     was *decided*, not *defaulted*.

The recorded decision unblocks the council. The gate itself does not wait on it.

---

## 3. Non-blocking aesthetic / strategic advice

- **Praise, keep:** "admin plane = platform plane, no in-handler branching" is the right spine —
  protect it from future "just one tenant-scoped exception in this handler" requests; that's the
  BOLA door reopening.
- **Audit reader asymmetry:** when headcount > founders, give `pa_audit_read` a notion of "you can't
  be the sole reader of your own trail." Cheap now (out-of-band mirror), expensive to retrofit after
  a dispute.
- **Kill-switch granularity:** don't let `ADMIN_PLANE_ENABLED=false` blind your recovery tools during
  an incident (above).
- **Provision ≥2 at bootstrap (R3):** correct, but pair it with a runbook line on *de-provisioning a
  departed founder* — the insider-removal story is the whole point of choosing B over A; make sure the
  ops script is exercised, not just written.

---

## 4. Steel-man of a rejected option — Option C (network-isolated internal ops service)

The proposal rejects C as "over-engineered for 1–5 admins." On throughput economics, fair. But on the
**one axis this charter explicitly cares about — concentration of power / capture by a narrow group —
C is the strongest option, and the proposal under-credits it.**

Argue it at full strength: with **Option B**, the entire barrier between the public internet and
fleet-wide tenant data is *one DB boolean* plus *the continued good behavior and credential-hygiene of
the same operator who also controls the audit*. A single leaked operator session reachable from the
public edge = fleet-wide read. The trust model collapses to "trust one human + one row." With **C**,
the public edge **cannot reach the admin plane at all**; an attacker (or a rogue operator working from
outside their sanctioned position) needs an **independent second factor — network position (mTLS/VPN)
— that no single boolean and no single leaked token can satisfy.** That is defense-in-depth against
exactly the failure the charter fears: power concentrated such that one compromise = capture of
everyone's data.

And note the proposal *already concedes C's value* — Option D bolts "thin C-style hardening" onto the
two DR endpoints. The steel-man is simply: that concession is too thin. The charter's capture concern
is a reason to pay C's cost **because** the population is tiny and powerful, not despite it — a
5-person tier holding fleet god-mode is *precisely* the configuration where one more independent
barrier is worth a separate deploy. "Over-engineered for 1–5 admins" answers the *volume* question;
it does not answer the *concentration* question, which is the one the charter actually asks.

(Honest counter, for balance: B+audit gets ~80% of C's protection at ~20% of the cost, and C without
a per-actor principal loses the audit *actor* — so C is a complement to B, not a replacement. The
right reading is "B now, C-on-the-public-edge as the next hardening when tenant-count or headcount
crosses a threshold," not "C instead of B." The steel-man's real bite: schedule C, don't dismiss it.)

---

## 5. One open question nobody asked

The proposal asks "who watches the platform-admin?" and answers "the audit log." The unasked,
harder question is: **what is the wronged restaurant's remedy?** When the operator is simultaneously
the platform-admin, the auditor, the sole reader of the audit, and the one who runs the grant/revoke
script — a tenant who is harmed by platform access has *no notification, no appeal, no independent
channel*. For a SaaS sold to independent restaurants (the very "people it was learned from"), is
**"trust us, we audit ourselves"** a promise you can honestly make to a customer — and if it is
acceptable at 5 tenants, **what concrete trigger** (tenant count? first non-founder ops hire? first
data dispute? first acquirer due-diligence request?) flips it from acceptable to a breach of the
charter — and is anyone watching for that trigger, or will it pass silently?

---

## Re-examine round 2 (post-RESOLVE — light pass on §11 + resolution.md)

Read §11 (legibility-floor pre-staging), the RESOLVE disposition table (F1–F10, E1–E4, R7–R11), and
the DEFINER fn delta (§5/§7). Three asked questions; one bottom line.

**Bottom line: no new ETHICAL-STOP. STOP-1 remains the only blocker, and it is correctly framed as
friction. The gate ships once the human ratifies the floor.** RESOLVE strengthened, not weakened, the
posture: E2 (kill-switch split — recovery reads never darkened) directly closes my non-blocking #1;
the floor is now pre-staged with a named mechanism + named trigger + explicit recorded deferrals. Two
refinements below sharpen the *human decision*, not the engineering.

### Q1 — Is the §11 floor set right? (the single cleanest question)

The floor is set **right** — neither too low nor too high:
- **Not too low:** it names a *real* out-of-band mechanism (append-only mirror to a sink admins
  cannot silently rewrite), not a vague "we'll be careful." It refuses the null floor of permanent
  self-watching.
- **Not too high:** it correctly *defers* tenant-right-to-know and per-drill second-admin as
  recorded-DECIDED deferrals rather than gold-plating them before a single paying tenant exists.
  Demanding them pre-staging would be disproportionate friction on a launch-blocker security fix.

The one weakness is **not** the floor's height — it is that the floor pre-stages the mirror's
*construction* and *owner* (R9: Ops + Architect) but **not the ownership of detecting the enact-
trigger.** Most trigger-arms are self-surfacing (you know when you make a non-founder ops hire; a
dispute and an acquirer DD announce themselves), so the deferral is honest. But the floor as written
lets the trigger be *recorded* yet *unwatched* — which is exactly the silent-pass failure my §5 open
question named. A floor with a watched trigger is a commitment; a floor with an unwatched trigger is a
hope.

**The single cleanest question the human must answer:**

> *"Self-watched ops-metadata access is acceptable at founder-scale — but at which **named, owned**
> trigger does the out-of-band audit mirror become **mandatory-before-next-access**, and **who owns
> watching for that trigger** so it cannot pass silently?"*

Answering that one question (a named trigger + an assigned trigger-watcher) converts §11 from a
well-intentioned plan into a durable commitment, and discharges STOP-1. Everything else in §11 is
already adequately framed.

### Q2 — Does the SECURITY DEFINER `is_platform_admin()` raise a NEW power-concentration concern?

**No — ethically neutral. Mechanism, not policy.** The DEFINER fn does not move where authority lives;
it only makes the *read* of that authority reliable regardless of RLS posture (the F3 requirement).
Concretely:
- It is **read-only** (`SELECT EXISTS`, STABLE), pinned `search_path` (reuses the ledger-#33 DEFINER
  guardrail), `REVOKE ALL FROM PUBLIC`, `GRANT EXECUTE` to the operational role only. It cannot
  escalate — it returns a boolean derived from rows that already exist in `platform_admins`.
- Authority still resides in the table rows, and §8's "no API write path to the allowlist" is
  untouched — the DEFINER fn adds **no** new write/grant path. The least-power shape ("one boolean,
  read reliably") is *preserved*, arguably sharpened.

One honest caveat, **not** a new STOP and **not** new to B4: a DEFINER object is privileged, so "who
can `CREATE OR REPLACE` this function" is a real DDL-privilege question — but it is the *same* question
already governing every DEFINER fn in this system (GDPR-anonymize, ownership-transfer). It folds into
the existing DEFINER-hygiene posture (search_path pin + migration-only DDL), not a fresh concentration
surface. Confirm neutral.

### Q3 — Is deferring R8/R9/R10 honest scheduling, or quiet permanent acceptance? Trigger concrete enough?

**Honest scheduling — not abandonment — for R9 and R10; R8 is honestly accepted-with-named-real-fix.**
The test for an honest deferral is three-part: the trigger is (a) concrete/observable, (b) owned to
build, (c) watched to fire. RESOLVE satisfies (a) and (b) and *records the deferrals as DECIDED, not
defaulted* — that recording is itself the honesty signal that distinguishes scheduling from silent
acceptance. R8's acceptance is honest because it explicitly names the mirror (R9) as the real fix
rather than pretending `USING(true)` is fine forever.

The concreteness gap is narrow and shared with Q1, in two spots:
1. **The tenant-count threshold arm is an unset number** ("a threshold the human sets") — until the
   human writes the number, that arm is a placeholder, not a trigger. (The other arms — first
   non-founder hire / first dispute / first acquirer DD — are concrete binary events.)
2. **Trigger-*detection* ownership is implicit.** R9/R10 name owners for *building* when fired, not
   for *noticing* the fire. Self-surfacing arms make this low-risk, but the threshold arm needs an
   actual monitor.

So: deferring is honest **as written**, and becomes durable the moment the human (in answering Q1)
sets the threshold number and names the trigger-watcher. No re-scoping needed — just those two values
recorded alongside the floor decision.

### New ETHICAL-STOP?

**None.** STOP-1 stands as the sole blocker and remains friction, not veto. The DEFINER fn is neutral;
the floor is correctly set; the deferrals are honest. The gate ships once the human records the floor
decision — ideally answering the single Q1 question (named + owned + watched trigger), which also
discharges the one concreteness gap in R9/R10. The aesthetic/strategic notes (Option C scheduled, not
dismissed, via R10; kill-switch split via E2) are confirmed addressed.
