# The Model-Agnostic Playbook — role-keyed metacognition for any model

> **Thesis.** The reasoning that makes an expensive model "good" is not talent — it is a small set of
> **mechanical moves keyed to the ROLE a model is playing, not to which model it is.** Any model
> (Opus, Sonnet, Haiku, Fable, or a future one) dropped into a role reproduces that role's output by
> running the role's checklist. Expensive reasoning *transfers to cheap models* because it is frozen,
> once, into artifacts the cheap model consumes: the planner's per-step **contract**, the **Gate+RED**
> pair, the **circuits/gates**, and the **reflection→ratchet** loop.
>
> This doc is the full role-keyed expansion of the two seed moves in
> [`metacognition-transfer.md`](metacognition-transfer.md) (reconcile-vs-reality · RED-prove-every-guard),
> grounded in the roles thesis of [`player-roles.md`](player-roles.md) and enforced by the mechanisms in
> [`KNOWLEDGE-AS-CIRCUITS.md`](KNOWLEDGE-AS-CIRCUITS.md) + [`task-exit-rule.md`](task-exit-rule.md) +
> `AGENTS.md` (MODEL ROUTING · TOKEN ROUTER · Execution shape). It is evidence-grounded: every move
> cites where it was earned (a reflection, a ledger root, or a sovereign-core step).

The rule for reading it: **find your role, run its checklist. The moves are checkable actions, not
advice — if a move can't be stated as "do X and observe Y," it isn't in this doc.**

---

## §0·GP — The governing principle (operator 2026-07-07): GROUND TRUTH over PROXY REASONING

> **"Harness must remain, but ground truth over proxy reasoning."** This is the meta-cognition
> layer's constitution — the one rule that orders all the others, for any model. It is what makes
> this doc an *ultimate harness for any model used*: a cheap model wins not by reasoning better but
> by being pointed at ground truth instead of a proxy.

> **Retrieval realization → [`meta-cognition-layer.md`](meta-cognition-layer.md).** §0·GP as a running
> engine: a **deterministic** knowledge retriever over the harness's own corpus (recall@5 = 1.0, offline,
> cross-process byte-identical, tamper-checked) that **any** model consults *before acting* —
> `node spikes/living-knowledge/search.mjs "<question>"` → the canonical rule/guardrail/loop file, not a
> remembered approximation. Same question → same files for every model (determinism = model-agnostic).
> The model's parametric recall is the proxy; the layer is the ground truth. Prefer the layer.

- **Ground truth** = a deterministic verdict read off the *real artifact*: a glob/pattern gate, a
  failing test, a re-read of the source at `file:line`, canonical bytes, the value re-read from the
  DB the code actually wrote.
- **Proxy** = anything that stands in for that verdict: a second model's opinion (a review/critic
  subagent, a council), an advisory reasoning-nudge injected before you act, a cached datum trusted
  without re-reading, a mirror-oracle test.

**The rule:** *when a ground-truth check is available, it wins and the proxy is not consulted.* A
proxy is used ONLY when no deterministic check exists for the question — and even then it only
**signals**; it never decides and never gates. If you find yourself about to trust a proxy, first
ask "can I write the deterministic check instead?" — if yes, write it (it becomes a permanent gate:
the ratchet, X4).

This **reprioritizes every move below.** **U3 (ground truth over proxy) is the spine.** The
reviewer-decorrelation moves (**U4 · §3 · X5**) are now **conditional** — reach for a decorrelated
reviewer only where a deterministic check is genuinely impossible. This is why, on 2026-07-07, the
**council + the critic/review proxy agents + the advisory-injection hooks were removed** while the
**deterministic harness gates were kept**: a gate is ground truth; a critic is a proxy. What survives
enforcement is machinery, not opinion (§6).

---

## §0 · Universal core — the moves EVERY role runs, regardless of model or role

These six run in every role. They are the model-agnostic floor.

- **U1 · Reconcile the plan/spec against reality before acting.** A plan or a recalled memory is a
  *hypothesis about a codebase that has since moved.* Before executing a step, verify every named
  file / symbol / flag it references **exists AND has the shape you assume** — not merely that the
  name exists. *Move:* `grep` the symbol the plan names, read its signature, compare to the plan,
  reconcile. *Earned:* 0b-2 (the plan's `codec/request_hash.rs` had moved to the shell) and 0b-3 (the
  plan said "fold `idempotency`/`needs_honest_dispatch` into `decide`", but they *return control-flow,
  not events* — uncomposable into an event-emitting door). *Family:* plan-is-a-hypothesis.

- **U2 · RED-prove every guard — along the dimension its invariant ranges over.** A guard you did not
  watch go **red** is decoration. *Move:* inject the exact defect the guard targets → observe it fail
  → revert → observe green. And inject it at a point that **differs on the axis the invariant
  quantifies over** — a concrete example fixed elsewhere on that axis sits in the guard's blind spot.
  *Earned:* 0b-3 — the LC1 conservation proptest caught the inclusive-venue double-tax that the
  *exclusive-only* concrete test structurally could not (there `charged_tax == tax_total`). *Family:*
  mirror-oracle / false-green.

- **U2·b · Oracle independence — the expected value must not come from the code under test.** The
  *expected* side of any correctness proof must be a **hand-derived literal** (arithmetic shown,
  zero imports) or a **definitional invariant** naming no implementation output. Reject any test whose
  expected side calls the module under test or its mirror. *Move:* ask "where did this expected number
  come from?" — if the answer is the code being tested, it's a mirror-oracle. *Earned:* a fee-parity
  test fed the impl's own output back as expected → it passed *before and after* the double-charge bug
  (#56); 0b-1 used 28 hand-derived decorrelated vectors instead. *Family:* mirror-oracle.

- **U3 · Ground truth over proxy.** Drive the **real bound / deployed / routed / un-bypassed** surface,
  not the edited / assumed / literal model of it. *Move:* assert against the DB the API actually wrote,
  the route the stack actually serves, the value re-read from the source — never against a re-call of
  the code under test. *Earned:* the #1 failure root across 81 ledger rows + 19 reflections ("live
  reality ≠ edited model") — this is the D5 verification spine. *Family:* proxy-vs-ground-truth.

- **U4 · Decorrelate the reviewer — CONDITIONAL (§0·GP): only when no deterministic check exists.**
  First try to make the check *ground truth*: a failing test, a re-read of the real bytes, a
  circuit. Reach for a **different model / context than the one that produced the work** only where
  a deterministic check is genuinely impossible — and when you do, it only **signals**; a gate or a
  human decides, never the reviewer. *Move:* prefer writing the check; if impossible, spawn a fresh
  reviewer instructed to *refute*, handed the source-of-truth to diff against. *Earned:* a rotted /
  same-context reviewer is how #56 shipped "certified green" — but the durable fix was the 28
  hand-derived oracle vectors (a check), not the review. (Historical: the 0b-1/0b-2/0b-3 money steps
  used a fresh-opus `invariant-guardian` read; that review-agent proxy was removed 2026-07-07 in
  favour of the canonical-bytes/oracle-vector checks it was shadowing.) *Family:* mirror-oracle.

- **U5 · Scope discipline + deferral-with-trigger.** Do exactly this step's scope. Defer the adjacent
  temptation with a **named unlock trigger**, and never start work that is **blocked** on an unshipped
  dependency. *Move:* write the deferral as "DEFERRED until <condition>"; write blockers as "BLOCKED
  on X" tokens. *Earned:* 0b-2 grew the alphabet but left `decide` untouched (emitting = 0b-3); A3
  stays BLOCKED on 0b-5. *Family:* scope-creep / entanglement.

- **U6 · Escalate genuine forks to the human before acting.** When the choice is irreversible or
  red-line **and** has ≥2 live interpretations, climb the cheapest-sufficient rung of the Doubt ladder
  — up to the human — *before* committing, and make the fork **legible** (options + recommendation).
  *Move:* present the fork; don't silently invent a red-line shape. *Earned:* 0b-3's `decide`-signature
  fork (surfaced to the operator before coding; the code constraints then confirmed the answer).

---

## §1 · Planner guide — freeze reasoning into a contract the doer runs without cleverness

*(Reverse-engineered from the Fable-5 sovereign-core plan corpus.)* The planner's whole job is to do
the hard thinking **once** and freeze it into artifacts. Checklist:

1. **Anchor current state first ("do not re-plan").** Freeze reality at the top so planning can't drift
   into re-litigating done work.
2. **Decompose so a step = a unit that can name its own gate.** Promote a unit to its own step exactly
   when it can (a) name **one** deterministic gate + **one** RED proof, (b) touch a bounded file set,
   (c) take a single red-line classification. If it can't, it's under- or over-scoped. *(That's why
   "seal the core" is six steps, not one.)*
3. **Emit the fixed per-step contract — the primary transfer artifact.** Every step carries the same
   schema, declared once in a Conventions glossary and inherited:
   **`Scope · Files · DoD · Gate (deterministic, on the real surface) · RED proof · Red-line · Effort`.**
   Keep **DoD** (the state that is true when done) distinct from **Gate** (the mechanism that proves it)
   distinct from **RED proof** (evidence the gate isn't a paper gate). Never collapse them.
4. **Sequence forward-only with explicit dependency tokens.** Write `BLOCKED on X` / `do not start
   early` on the step itself (not in your head). Put the risky live flip **last** (keystone-last).
   Treat anti-context-rot as a dependency edge: **fresh session per phase**.
5. **Place a seam for every deferral, and prove it swaps.** For each capability you defer, name the
   **port / trait / flag / column** that already exists so the later phase *swaps an impl, not
   rewrites* — and prove the seam by making the contract pass against **two** impls (one a throwaway
   that exists only to prove swappability). Audit: the deferral table has **no blank seam cell**.
6. **Attack your own plan, then hand it to a decorrelated reviewer.** End every plan with a
   *"How this could break"* section that names its **own riskiest step** and its **own rot risks**;
   then a different model reviews it under "find 3 ways it breaks" (not "explain it").
7. **Classify red-line by a fixed glob, not judgment.** `Red-line: yes/no` per step by keyword-matching
   the serious classes **money / RLS / PII / schema / auth**; name *which* gate body clears it.
8. **Calibrate risk/effort as filled cells.** Effort in absolute units (S ≤ ½d · M 1–2d · L 3–5d);
   accepted-risk CARRY rows with mitigations; deferral-with-unlock-trigger; a **pre-committed descope
   rule** ("if this overruns, cut X to WARN, ship Y"); an **enforcement-tier honesty column**
   (DENY / DIRECTIVE / AUDIT / WARN→ratchet) — never pretend a hook can see what it can't.

---

## §2 · Doer/Executor guide — run the contract, prove the real surface

1. **Run the contract; don't re-derive intent.** The step's DoD/Gate/RED are the frozen reasoning —
   execute them. If the contract is missing a field, that's a planner gap → surface it (U6), don't
   improvise a red-line.
2. **Reconcile before edit (U1).** Read before you edit; verify the plan's named symbols against
   reality first.
3. **Grep the blast radius before a shape change.** Before changing a type/signature, enumerate every
   construction/call/match site. *Earned:* 0b-2/0b-3 — knowing `Command` is touched only in the domain
   crate's tests (the shell never imports it) is what made "drop `Copy`" safe.
4. **Deterministic code before any LLM call; batch independent work; distilled returns.** (TOKEN
   ROUTER.) Cheapest adequate route; narrow read-only grants; parallel independent tool calls.
5. **RED-prove your guard (U2) and drive ground truth (U3).** Never declare done without pasting the
   red→green proof against the real surface.
6. **Keep the session short.** One task in-progress; kill/recycle a lane at its token budget; persist
   an h_t frame and resume fresh rather than running a marathon (anti-context-rot).

---

## §3 · Reviewer/Verifier guide — adversarial, decorrelated, signal-only

1. **Be adversarial, not confirmatory.** Ask "find N ways this breaks / is wrong," default to
   *refuted* under uncertainty. Reverse-engineering a diff is itself generation, so a confirmatory read
   just re-derives the author's blind spot.
2. **Be decorrelated (U4) — but only if you're here at all (§0·GP).** A reviewer runs only where no
   deterministic ground-truth check exists; if a check is writable, the reviewer's real job is to
   *write it* (and promote it to a gate), not to opine. When you do review, use a different
   model/context than the author and default to *refuted*.
3. **Check the guard's RED exists and fires along its dimension (U2).** A green test is not evidence;
   a test proven to go red on the real defect is.
4. **Diff against the source of truth, not the author's claim (U3).** For a "port X verbatim" change,
   check it arg-by-arg against X, not against the author's description of X. *Earned:* the 0b-3
   guardian diffed `price_cart` against the live `pg.rs:287-343` AND the persistence side.
5. **Signal, don't decide.** Advisory reviews signal; **deterministic gates and humans are the
   authority.** A reviewer PASS never overrides a red gate.

---

## §4 · Cross-patterns — the role handoffs are where transfer actually happens

The roles are not independent; the value is in the seams between them.

- **X1 · The planner's `Gate + RED` pair IS the doer's verification checklist.** This is the load-
  bearing transfer. The planner computes, once, *"what assertion proves this correct, and what exact
  line falsifies it"* → the doer runs *"execute this named check, inject this named defect, paste red,
  revert, paste green."* Expensive "verify well" (talent) becomes cheap "run this ritual" (checklist).
- **X2 · The planner's per-step contract IS the doer's task spec.** A cheap doer needs zero cleverness
  because Scope/Files/DoD are pre-filled. The contract is the metacognition, externalized.
- **X3 · The verification spine (D5) is compiled failure-history.** The reviewer/doer inherits 81
  ledger rows + 19 reflections as **one rule** ("drive the real bound surface") — it never re-learns
  the history; it satisfies the distilled rule as a per-step field.
- **X4 · The reflection→librarian→ratchet loop turns a doer's lived lesson into a planner/reviewer
  gate.** A qualified change → a reflection with a causal WHY → the librarian/critics → a red→green
  Tier-1 guardrail. The doer's pain becomes the next planner's mandatory field. Roles feed each other.
- **X5 · Decorrelation is a cross-role invariant.** Whoever reviews must differ from whoever produced:
  the planner's plan is reviewed by a decorrelated model (LEAD-REVIEW); the doer's diff by a
  decorrelated reviewer (invariant-guardian). Same-context self-review is the false-green generator.

---

## §5 · Failure-mode families → the move that defends each

**The meta-root: nearly every incident in the ledger is a PROXY mistaken for GROUND TRUTH.** The
families below differ only by *where the proxy sat* — and each maps to a distinct mechanical
countermeasure, which is why they stay split. If you remember only the families, you can re-derive
every move above. (Roots 3 and 4 are the exceptions — not proxy problems, but decay and concurrency.)

| Family (causal root) | The proxy that lied | Defended by |
|---|---|---|
| **A · The verifier itself** — mirror-oracle, dead guard, guard proven at one fixture-point, rotted (deep-context) reviewer, paper suite, narrated-readiness | a test/guard/reviewer that measures self-consistency or never fires (#56; LC1 inclusive-tax; the dead `#[deny]` in 0b-2) | **U2** (RED along the axis) + **U2·b** (oracle independence) + **U4** (decorrelated fresh reviewer) |
| **B · A cached datum** — memory prose, filename-grouped estimate, stale remote/worktree, unre-read council packet | a datum trusted without re-reading the artifact (drift up to 27×; a resolved/unresolved verdict inverted) | **U1** (re-read / grep the named symbol + its SHAPE before acting) + `file:line` on every red-line claim |
| **C · The test harness/target** — unit test, fresh DB, payload-parity, serde rule, psql literal, drifted staging | a stand-in exercised instead of the deployed **and routed** surface (7 reflections — the largest family) | **U3** (drive the real bound surface) — the D5 spine; bound-param repro; sub-resource + UX-parity assertion; connect-with-the-real-secret |
| **Structural Root 3 · Discipline decays; only enforced artifacts survive** (the "row-#48 law") | an obligation with no durable artifact + no checker goes invisible within a week; a gate off any runner rots red unseen | every recurring obligation ships, in the same breath, **(a) the artifact + (b) a deterministic checker + (c) an event-line — on an enforced path** (pre-commit/CI); sequence the gate BEFORE the risky work; narrow-never-unregister (§6) |
| **Structural Root 4 · Shared mutable state without ownership** | the git index / overlapping lane file-scope / cross-session working tree — a global with no owner → silent collisions (HEAD unbuildable 40 min) | a lane-ownership manifest a PreToolUse hook consults, or worktree-by-default; `edit→add→commit` as ONE atomic step (never verify with work staged); commit-or-park at every session boundary. **No live guardrail yet — the one open family.** |

---

## §6 · Enforcement — why these are reproducible, not just advice

A move only counts as *codified* when a mechanism enforces it **regardless of which model runs**:

- **Deterministic gates (red→green)** — the move's RED proof becomes a committed test/lint/hook that
  fails on the real defect. (sovereign-gate; the armament suite.)
- **Circuits** — `registry.json` + `run-circuits` turn lessons/design-rules into always-checkable
  entries. ([`KNOWLEDGE-AS-CIRCUITS.md`](KNOWLEDGE-AS-CIRCUITS.md).)
- **The ratchet** — reflection → librarian/critics → a promoted Tier-1 guardrail (X4). A lesson that
  isn't ratcheted into a gate is advisory only.
- **Gate lifecycle: measure → warn → data-triggered ratchet; then narrow, never unregister.** Before
  flipping a gate to hard-DENY, baseline the real violation rate and set the threshold on that data;
  ship WARN with DENY armed behind a flag; promote on an encoded trigger, not a vibe. When a live gate
  false-positives, **narrow** the rule (diff-aware, added-lines-only, exempt `docs/*.md`) — never lower
  the threshold and never unregister it. A gate that trains operators to ignore red is worse than no
  gate; a blind day-one DENY that blocks ~90% of legit traffic gets torn out under pressure (Root 3).
- **Decorrelated review (conditional, §0·GP)** — used ONLY where no deterministic check exists; it
  *signals*, a gate/human decides. Prefer promoting the check to a circuit/test (the ratchet, X4) so
  the review is never needed again. (The standing council + critic/review proxy agents were removed
  2026-07-07; a decorrelated read is now an on-demand, human-gated exception — e.g. a one-shot
  cross-model audit — not standing machinery.)
- **THE EYE + the Doubt ladder** — halt on ≥3 bad / ≥1 critical signal; escalate the cheapest-
  sufficient rung on a genuine fork (U6).
- **MODEL / TOKEN ROUTER** (`AGENTS.md`) — the router assigns the *role* (planner reasoning / doer /
  reviewer) to the cheapest **adequate** model, which is exactly what makes a role-keyed (not
  model-keyed) playbook operational.

---

## §7 · The self-evolving harness — grow by verified proposal, prune by deterministic proof

The living-organism vision (operator 2026-07-07): the harness self-evolves on **actual verified
checks / deterministic math truth**, not on judgment. Two mechanisms, both pure §0·GP — one produces
work, one removes rot. They are the organism's anabolism and catabolism, and neither ever trusts an
opinion.

### §7·A · Speculative execution — speculative decoding lifted from tokens to tasks

Speculative decoding makes a big model fast: a cheap **draft** model proposes tokens, the big model
**verifies them in parallel**, keeps the draft's tokens exactly where they match its own
distribution, and recomputes only where they don't. Lift it from tokens to tasks — this is the
mechanical form of TOKEN/MODEL ROUTING under §0·GP:

- **Draft** = the cheapest adequate model (haiku doer) emits the candidate — a diff, a fix, a plan
  step, a distillation.
- **Verify** = a **deterministic ground-truth check**: the planner's Gate+RED pair, a test, a
  circuit, canonical bytes, a re-read of the real surface. **Not a proxy reviewer** — the verifier
  must be a *check*, or this collapses back into the removed critic-proxy pattern.
- **Accept** on pass: keep the draft. This is the common case and where the speedup lives — you got
  target-model correctness at draft-model cost *because the check caught any miss*.
- **Reject → fallback**: on fail, recompute that one piece on the stronger model (opus) and re-verify.
  A wrong draft is caught, never rubber-stamped.
- **The acceptance rate is MEASURABLE** (gate pass-rates in the `_hev` log). Route more to the draft
  as its verified-acceptance rises, pull back where it falls — the router tunes itself on ground
  truth. A task with **no writable deterministic check is not a speculation candidate**: do it at the
  target model, or first build the check.

The one invariant: *speculation is only as good as its verifier, so the verifier is always a
deterministic check, never a second opinion.* That is the whole difference between §7·A and the
critic-proxy layer that was removed.

### §7·B · Deterministic-proof auto-deletion — the pruning half (operator decision 2026-07-07)

Operator policy: **deterministic math truth is allowed to eliminate rotten/biased processes or code
itself** — the harness may **auto-delete**, not merely propose, when a deterministic proof
establishes rot. This is the pruning dual of §7·A: §0·GP turned on the harness's own body.

- **The trigger is a PROOF, never a heuristic or an opinion.** Admissible rot-proofs are
  deterministic and **re-runnable**: (a) *dead reference* — zero inbound references in the real
  import/dispatch graph; (b) *decoration guard* — RED-prove fails (inject the exact defect; the guard
  never reds ⇒ it protects nothing); (c) *net-negative proxy* — a ground-truth check on its own
  output measures negative value/cost; (d) *dangling reference* — points only at already-removed
  machinery. If you cannot state the rot as a check another run reproduces, you may **not** auto-delete
  — you propose (drop to a human).
- **Safety envelope (mandatory, non-negotiable):** the deletion is **git-reversible**; it **never**
  auto-touches a red-line surface (money / RLS / PII / auth / migrations / secrets) — those still gate
  to a human (U6); the **proof commits alongside the deletion** so the removal is reproducible, not a
  one-off judgment; and both are **logged on an enforced path** (event-line — Structural Root 3).
- **Precedent — this already happened by hand on 2026-07-07:** the proxy layer was removed because a
  deterministic classification (ground-truth gate vs proxy) plus a ground-truth verify (`grep` proved
  no live file references a removed proxy; both arming guardrails green) *proved* the rot. §7·B makes
  that same move a re-runnable mechanism instead of a manual pass — the organism pruning itself on
  proof, the operator's "deterministic math truth eliminates the rotten" made literal.

Together: **§7·A accepts cheap work the ground truth verifies; §7·B deletes machinery the ground
truth proves rotten.** That is the entire self-evolution loop — and it is why the harness can be
cheap *and* not rot.

---

### One-line summary per role (the pocket version)

- **Planner:** freeze the reasoning into a 7-field contract whose Gate+RED the doer can run blind;
  seam every deferral; attack your own plan; classify red-line by glob.
- **Doer:** run the contract, reconcile before edit, grep the blast radius, RED-prove on the real
  surface, keep the session short.
- **Reviewer:** decorrelate, refute, diff against the source of truth, check the RED fires, signal-only.
- **Everyone:** reconcile-vs-reality · RED-prove-along-the-dimension · ground-truth-over-proxy ·
  decorrelate-the-reviewer · scope+trigger · escalate-genuine-forks.

### The pocket constitution (read this if you read nothing else)

**Ground truth over proxy reasoning (§0·GP).** For any question, ask: *is there a deterministic check
I can read off the real artifact?* If yes — write/run it; it decides, and it becomes a permanent gate.
If no — a proxy (a decorrelated read) may *signal*, but a gate or a human decides. The harness is the
set of deterministic checks; it stays. Opinion-machinery (councils, standing critics, advisory
nudges) is proxy; it goes. Any model that runs this ordering is running the harness — that is what
makes it model-agnostic.

*Related:* [`player-roles.md`](player-roles.md) · [`metacognition-transfer.md`](metacognition-transfer.md)
· [`task-exit-rule.md`](task-exit-rule.md) · [`agentic-map-reduce.md`](agentic-map-reduce.md) ·
reflections `docs/reflections/INBOX/2026-07-07-0b3-decide-composition.reflection.md` +
`2026-07-07-ground-truth-over-proxy.reflection.md` (the §0·GP removal).
