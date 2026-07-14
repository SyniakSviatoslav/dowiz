# bebop — Autonomous Self-Evolution Architecture

> Status: v1 (2026-07-14). bebop self-evolves **fully autonomously — no human in the loop** —
> governed entirely by its own deterministic machinery. The single floor is **reversibility**:
> not a gate imposed from outside, but bebop's own foundational invariant (grow-only CRDT,
> attic-move-not-delete, `rollback_to_best`). "What exists" is refined by the live scaffolding survey.

## 0. Thesis (one sentence)

**bebop evolves itself at full speed with no human review — its resonator watchdog, fuse, and
`rollback_to_best`, its grow-only CRDT memory, and CI-green are the grounding — and the one thing
held constant is reversibility, because self-correction is only possible over state you can recover.**

## 1. The invariant: reversibility (bebop's own design, not an external gate)

Every self-correction primitive bebop already has *presupposes recoverable state*:
- `rollback_to_best` — needs a "best" to return to.
- grow-only CRDT sets + **attic-move-not-delete** (`memory.rs` was fixed to exactly this) — never destroys.
- the resonator's Lyapunov watchdog — freezes a step when error climbs, then rolls back.

So an action that **cannot be undone** (force-push over history, repo/branch delete, secret exposure)
is the one class that escapes bebop's *own* recovery loop. Keeping those out is not a limit on
autonomy — it is holding the GitHub boundary consistent with the architecture bebop runs on. Remove
reversibility and the self-evolution loop loses the floor it stands on.

## 2. The self-evolution loop (fully autonomous, deterministic)

```
  ┌─ propose ──────────┐   free-LLM mesh worker generates a change (branch)
  │                    ▼
  │              run + reflect        resonator: generate → reflect → supervise
  │                    │
  │              supervise            Lyapunov watchdog: error climbs ⇒ FREEZE + rollback_to_best
  │                    │
  │              CI-green?  ── no ──► rollback_to_best, try next  (deterministic pass gate)
  │                    │ yes
  │              land on a branch / open PR (reversible)
  └──────── fuse: max_iterations OR Converged (‖error‖<ε) ──────► stop
```

- **No human approval anywhere in this loop.** The pass condition is deterministic: CI-green +
  governance HARD-BANs not tripped + the watchdog not frozen.
- **Convergence governor** = the resonator's termination (`Converged` / `Fused` / `Stalled`) — the
  same LaSalle/energy certificate that proves the field-UI and hydraulic-loop convergence.
- **Grounding (DPI anchor)** = the test suite / CI, *not* a person. An internal-only loop leaks
  meaning (supermartingale); CI is the external truth that refills it. Deterministic, autonomous.

## 3. Identity & the GitHub boundary — capability, not gate

The agent acts through a **GitHub App** (least-privilege, per-repo) whose permission set simply
*does not contain* the irreversible operations — so they aren't "gated," they're **absent**:
- ✅ `contents: write` (branches/commits), `pull_requests: write`, `issues: write`, `checks: read`.
- ❌ no `administration`, no branch-delete on protected refs, no force-push over protected history,
  no `secrets`, no `actions` permission changes, no org-admin.
- **Branch protection + required status checks** on `main` make destructive merges *impossible* at
  the platform level — GitHub enforces it, not the agent's honor. The agent lands work on branches
  and PRs; anything mergeable is CI-green by construction.
- The `SHA256:…` you hold is a **fingerprint** (public identifier), not the credential. The App's
  private key / installation token is the real handle and stays operator-side.

## 4. Free-LLM mesh as the worker pool

The existing `agents-mesh.sh` (Hermes → OpenCode → Goose → Aider → OpenHands, ordered fallthrough) is
the compute. For autonomous operation: `MESH_ALLOW_AUTO_APPROVE` **on** for reversible in-branch work
(that's the whole point — no babysitting), **per-run budget caps** (token/time/PR-count) so a runaway
loop bounds itself, and the mesh's own no-auto-approve default remains the backstop for anything that
would touch the absent-capability set (it simply can't, so this is belt-and-suspenders).

## 5. Deterministic governance ported (self-check, not human-check)

- **agent-governance HARD-BANs** (Voodoo ban etc.) run on every proposal — deterministic filter.
- **`verify-self-mod` + floor-invariant** pattern (built for the kernel this session) ported to
  bebop: a self-mod is admitted only if it doesn't regress the safety floor / benchmarks / entropy.
- **Append-only audit log** of every autonomous action — the operator *reviews after*, never *gates
  before*. Full-speed with a black box recorder.

## 6. What exists today (ground-truth survey)

bebop is a **library of deterministic safety, control-loop, identity, and mesh primitives** —
unit-tested (crates/bebop ~454 tests, bebop2 ~242) — with exactly ONE gating pipeline wired to a
runnable surface (the MCP server). Correcting v1: **W/A/H = the protocol lanes** — proto-**W**ire
(transport) · proto-cap = **A**uthz/capability · proto-crypto = crypto (**H**) — *not* Worker/Agent/Human.

BUILT + PROVEN, ready to build ON:
- **`resonator.rs`** (544 LOC, math-proven) — generate→reflect→supervise, `{Converged|Fused|Stalled}`,
  Lyapunov freeze-on-divergence, `rollback_to_best`. **But actors are pure `fn` pointers — no LLM, no I/O.**
- **`wiring::wire`** (MCP server) — the fail-closed gating pipeline (field veto + stabilizer +
  forbidden-zone + scope + living-memory + tamper-evident hash-chained audit), externally driven.
- **`loop_runtime.rs`** — the 6-layer state machine (INTAKE→…→DELIVER, governor/Kalman/Goodhart); its
  generate/reflect/supervise are **explicit dead-code stubs** ("real LLM out of scope").
- Real hybrid Ed25519⊕ML-DSA-65 identity, UCAN delegation, grow-only revocation CRDT, pull anti-entropy.
- Non-destructive memory: `LivingMemory` attic-move+restore, `agentic_git` snapshot chain — **the
  reversibility floor is real and already there.**

## 7. The honest gap map → autonomous self-evolution

The self-evolution loop **does not exist yet** — it is dead library code + explicit stubs:
1. **No wired driver** — nothing constructs `LoopRuntime` or calls the resonator outside tests/benches;
   `agent_loop.rs` isn't even compiled.
2. **No generate step, no real act step** — loops are pure-Rust by design (no LLM in-loop); the act
   primitive `native_exec` is a STUB that returns a plan and runs nothing.
3. **NO self-modification capability at all** — exhaustive grep: no source/config write path, no
   `git commit`, no cargo/compiler invocation, no patch loop. "Self-evolution" today = in-memory
   salience state + a persisted error-pattern store, NOT code/behaviour rewriting.
4. **Governance/identity never applied at runtime** — persona injectors + error-pattern learning built
   but uncalled; settings unpersisted.
5. **Mesh can't go live** — iroh QUIC carrier is a stub, WSS is plaintext, all state in-memory (no
   persistence), root-of-trust unbootstrapped (fail-closed → a fresh node gets zero authority).
6. **Safety fences DORMANT** — NO-COURIER-SCORING / crdt-fence / kernel-fence exist as scripts wired
   into no hook/CI; some dowiz CI gates stranded by the deleted `package.json`.

**Buildable by me (no creds, no self-mod effector):** wire the resonator into a runnable driver, drop
the free-LLM mesh into the generate seam, port verify-self-mod + activate the dormant fences, the App
manifest + branch-protection as files. **The one gated piece:** the self-modification *effector* —
source-write + git-commit + compile — is exactly the self-mod + credential territory the classifier
has reserved for the human all session. bebop gets the safe harness; wielding the effector is yours.

## 8. What I build vs what routes through you

- **I build + prove (no creds):** the driver loop, the mesh auto-drive, the ported governance /
  verify-self-mod / audit, the App manifest + branch-protection config as files.
- **You run (creds):** installing the GitHub App, its private key, enabling branch protection, adding
  the mesh API keys. The credential wall reserves these for you regardless — and that's the same
  boundary that keeps the whole thing recoverable.

## 9. The one line, restated

Full autonomy, no human review, deterministic self-governance — **on the reversibility bebop is
already made of.** That's not less than "max freedom"; it's the only version where bebop's own
self-correction keeps working, because there is always a state to correct back to.
