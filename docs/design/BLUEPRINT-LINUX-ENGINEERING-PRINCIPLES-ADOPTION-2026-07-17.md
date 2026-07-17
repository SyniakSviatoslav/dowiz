# BLUEPRINT — Linux Kernel Engineering Principles: What dowiz Adopts, Extends, and Rejects (2026-07-17)

> Planning document; writes no product code. Built under the Detailed Planning Protocol
> (`AGENTS.md` §"Detailed Planning Protocol"): ground-truth-first, inline DECART for every new
> integration, 2-question doubt audit, Anu/Ananke check. Style contract: plain evidence-grounded
> prose, no metaphor; every load-bearing claim carries a `file:line` cite, a primary-source URL, or
> an explicit confidence flag.
>
> **Branch context:** all repo cites verified live on `feat/harness-llm-backend` at `9b180d8b2`
> (worktrees: `/root/bebop-repo`, `/root/dowiz-agentic-mesh`, `/root/hermes-agent-kernel-rewrite`
> read where cited).
>
> **Relationship to existing doctrine — read this first.** dowiz already has standing engineering
> doctrine: the seven Hermetic principles (`docs/design/hermetic-architecture-2026-07-16/
> HERMETIC-ARCHITECTURE-PRINCIPLES.md`), the Detailed Planning Protocol + DECART rule + 2-question
> audit (`AGENTS.md`), the rust-native-bare-metal decision (memory:
> `rust-native-bare-metal-decision-2026-07-14`), and the M-series mesh canon
> (`docs/design/ARCHITECTURE.md` §0). **This document is not a third framework.** For each Linux
> practice it issues one of five verdicts against that doctrine:
> `ALREADY-EQUIVALENT` · `REINFORCES` (Linux independently confirms an existing dowiz rule) ·
> `EXTENDS` (existing rule, Linux supplies a missing piece) · `GAP` (nothing equivalent exists —
> adopt) · `DOES-NOT-TRANSFER` (reject, with the reason stated).
>
> **Non-overlap boundary:** `BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md`
> (P24) owns the Linux *monitoring* techniques — procfs, perf_events, eBPF, cgroups accounting,
> ring buffers. This document deliberately covers none of those. Its scope is the layer above:
> development process, security architecture, testing discipline, concurrency patterns, and
> configuration modularity.

---

## 0. Method, sources, and the honest scale caveat

**Research provenance.** Three primary-source research passes (2026-07-17) against
kernel.org's `Documentation/process`, `Documentation/dev-tools`, `Documentation/security`,
`Documentation/kbuild`, `Documentation/RCU` trees (fetched from docs.kernel.org and verbatim-checked
against the raw `.rst` in `torvalds/linux` master), man7.org man pages, the 2002 USENIX LSM paper,
LWN.net (McKenney's RCU series), github.com/google/syzkaller docs, and lkml archives. Confidence
flags carried forward honestly:

- The 2012 Torvalds "WE DO NOT BREAK USERSPACE" mail (message-id
  `CA+55aFy98A+LJK4+GWMcbzaa1zsPBRo76q+ioEjbx-uaMKH6Uw@mail.gmail.com`) — quotes are
  **secondary-corroborated** (two independent mirrors), not primary-fetched: lore.kernel.org and
  lkml.org bot-walled every request this session. The *current official policy docs* that encode
  the same rule were primary-verified.
- The 2017 Torvalds "security problems are just bugs" mail — **primary-verified verbatim** via the
  Indiana University LKML hypermail archive (`lkml.iu.edu/hypermail/linux/kernel/1711.2/01701.html`).
- "Mechanism, not policy" — **no primary kernel-doc citation located** for the phrase itself
  (checked cpufreq, kobject, driver-model docs); provenance to Per Brinch Hansen (RC 4000) and
  Raymond's *Art of Unix Programming* rests on secondary sources this session. The concept is used
  below on its merits, not on authority.

**Repo ground truth.** One dedicated 9-axis audit of `/root/dowiz`, `/root/bebop-repo`, and the
worktrees, with the key correction it forced: several mechanisms described in docs as current are
**dormant** (the entire Claude-Code hook gate layer was neutralized to no-ops on 2026-07-15 by
operator directive — commit `5cd0c1b64`) or **feature-gated off by default** (Wasmtime fuel
metering). Every comparison below is against the live floor, not the aspirational one. One
correction to the hermetic synthesis while grounding: ADR-020, a phantom on 2026-07-16
(HERMETIC-ARCHITECTURE-PRINCIPLES.md §2 RC-1), now exists — `docs/adr/0020-oss-license-tm-dco.md`.

**Scale caveat, stated once and applied throughout.** Linux coordinates thousands of contributors
across three decades, in C, against a hardware surface, with a one-maintainer merge bottleneck.
dowiz is one operator plus an agent swarm, in Rust, ~600 kernel/engine tests, with an operator
merge gate. A Linux practice earns adoption here **only if it solves a problem dowiz demonstrably
has** (cited), not because Linux does it — per the DECART rule's ban on appeal-to-authority. Several
famous practices are rejected below on exactly this ground (§H).

---

## A. Development process & stability discipline

### A1. "Never break userspace" → the Frozen-Surface Ledger — verdict: EXTENDS

**The Linux practice (primary).** `Documentation/process/handling-regressions.rst` calls it "the
first rule of Linux kernel development." Torvalds (2012, secondary-corroborated): *"If a change
results in user programs breaking, it's a bug in the kernel. We never EVER blame the user
programs."* The rule's precise shape matters more than the slogan:

1. It protects **observable behavior**, not documented API: *"No amount of 'you shouldn't have used
   this' or 'that behavior was undefined' is at all relevant"* (handling-regressions.rst). The
   docs frame it with the tree-falls-in-the-forest question — only breakage a user actually
   notices counts.
2. It has **enumerated exceptions**: security fixes as "the lesser evil"; kernel-*internal*
   interfaces explicitly excluded; long-unnoticed breakage; staging code; optional off-by-default
   features (reporting-regressions.rst).
3. Its twin (Kroah-Hartman, `stable-api-nonsense.rst`, primary): *"You think you want a stable
   kernel interface, but you really do not."* The syscall ABI is frozen precisely so that
   **everything internal can churn freely** — bug fixes, security rework, deleting deprecated
   patterns. The frozen edge buys internal freedom; they are one policy, not two.

**dowiz today.** The rule half-exists as canon: M6 (zero-dep wire/trust boundary), M10 ("inter-hub
protocol = defined; intra-hub anarchy = allowed") — `docs/design/ARCHITECTURE.md:15,19`. The
event-sourced substrate makes historical events a compatibility surface: every replay guarantee
quantifies over the events already in the log (HERMETIC-ARCHITECTURE-PRINCIPLES.md §RC-3). But
**no artifact enumerates which byte formats are frozen and which are free.** Nothing today
distinguishes "changing `Capability`'s TLV encoding breaks every peer" from "renaming a kernel
module is free," except a reviewer remembering it. By the repo's own Ananke standard
(`AGENTS.md:246-250`), that is a rule that has already failed on any forgetful turn.

**Adoption (proposal): the Frozen-Surface Ledger.** One short in-repo doc (extend
`docs/design/ARCHITECTURE.md` or a sibling `COMPAT.md`) listing, exhaustively, the surfaces with a
never-break guarantee, each with its pin test:

| Surface (candidate list — verify at implementation) | Where |
|---|---|
| Capability TLV encoding + `verify_chain` semantics | `/root/bebop-repo/bebop2/proto-cap/src/tlv.rs`, `capability.rs:45-60` |
| Signed frame format | `proto-cap/src/signed_frame.rs` |
| Event-log record encoding + `event_id` derivation | `/root/dowiz/kernel/src/event_log.rs` |
| sha3 content-address derivations (event chain, spine chain) | `event_log.rs:30`, `spine.rs:92` |
| Public HTTP API + `/s/:slug` behavior | `apps/api/` |
| proto-wire sync formats | `bebop2/proto-wire/` |

Everything **not** on the ledger is declared explicitly unstable — the stable-api-nonsense half —
which licenses aggressive internal refactoring without per-change compatibility ceremony. Two rules
ride along: (a) behavior-observed-by-a-peer counts even if undocumented (the 2012 rule's real
content); (b) the only sanctioned exceptions are the Linux ones — security-fix-as-lesser-evil
(operator-gated, since money/auth are red-lines anyway) and provably-nobody-depends-on-it.

Mechanically, each ledger row needs a golden-bytes pin test (serialize a fixture, assert exact
bytes) — the same pattern as `PACING-CONTRACT.md`'s mirror-pin and P6's reference vectors, so this
extends an existing discipline rather than inventing one. Note the interlock: hermetic P6 Finding B
already flagged that "byte-identical" claims are only tested by same-process double-calls; ledger
pin tests close that gap for the frozen surfaces specifically.

**DECART (adopt vs. not).** Cost to adopt: one doc + ~6 golden-byte tests, no new deps. Cost of
not adopting: a silent wire-format change is the exact mesh-partition failure M6/M10 exist to
prevent, discoverable only when two hubs on different builds disagree — the most expensive possible
detection point. Probe (strongest argument against): with a single operator and effectively one
deployed build today, no peer exists yet to break — the ledger could be premature. Answer: the
agentic-mesh arc (B1 AgentBridge landed `f30189262`) is actively creating multi-party consumers of
these exact formats now; freezing before the second consumer exists is when freezing is cheap.
DECISION: adopt.

### A2. Bisectability — every commit builds and passes — verdict: GAP

**The Linux practice (primary).** `submitting-patches.rst`: *"take special care to ensure that the
kernel builds and runs properly after each patch in the series. Developers using `git bisect` to
track down a problem can end up splitting your patch series at any point; they will not thank you
if you introduce bugs in the middle."* Plus: one logical change per patch, each "justifiable on its
own merits." `bug-bisect.rst` gives the mechanical reason: one wrong good/bad judgment "will send
the rest of the bisection totally off course."

**dowiz today.** CI triggers on `push`/`pull_request` only and tests **the branch tip, never
intermediate commits** (`/root/dowiz/.github/workflows/ci.yml:6-11`; no `merge_group`, no
per-commit job — audit axis 7). The pre-commit hook with real logic exists but is unwired
(`.husky/pre-commit` present; `core.hooksPath` unset). The repo already *depends* on bisectability:
the `regression-hunt` skill is literally built on `git bisect`, and
`BRAIN-TOPOLOGY-ORG-PSYCH-EMERGENCE-RESEARCH-2026-07-16.md:938` already argued that suspended ship
discipline stacks unverified commits on high-churn files and degrades bisect. The claim-latency
ledger (`tools/ci-truth/src/main.rs:446-659`) watches per-commit CI plausibility but is advisory
(always exits 0). So dowiz has the *consumer* of bisectability and the *telemetry* around it, but
no *producer* discipline.

**Adoption (proposal).** Two-part, cost-scaled to a ~511-test suite (kernel 458 + engine 53,
audit axis 7) that already runs on every push:

1. **Rule (standing, agent-facing):** every commit pushed to a feature branch must build and pass
   `cargo test` for kernel+engine on its own. One logical change per commit — which
   `contextual-commit` (already installed) and the existing merge-per-feature history
   (`git log`: "Merge P08:", "Merge P04:", each a green-tested unit) already approximate.
2. **Enforcement (Ananke, so it isn't remembered):** a CI job that, for PRs/pushes with more than
   one new commit, walks `git rev-list origin/main..HEAD` (cap: 10 commits; more → job asks for a
   squash) and runs build+test per commit in a detached worktree. The worktree re-execution
   machinery already exists — `v5c-reexec` does exactly this for HEAD
   (`tools/ci-truth/src/main.rs:360-443`); extending it to iterate a rev-list is an extension of a
   native Rust tool, satisfying the minimize-bash directive. Ship it **advisory first** (§B5's
   bake-in rule), promote to blocking after a quiet period.

**DECART.** Cost: one subcommand extension + CI minutes (≤10× an already-fast suite, only on
multi-commit pushes). Cost of not adopting: `regression-hunt` silently loses its precondition;
culprit attribution for any future fuzz finding (§C2) or perf regression (`bench_track.py`) decays
from "one commit" to "one branch." Probe: agent swarms often produce WIP chains where intermediate
states honestly don't pass; forcing green-per-commit could push agents toward giant squashed
commits, which hurt bisect granularity from the other direction. Answer: the cap-plus-squash-request
design makes the tradeoff explicit per-series instead of pretending it away; the rule's unit is the
*logical change*, not the keystroke history. DECISION: adopt, advisory-first.

### A3. Maintainer trees, merge windows, linux-next — verdict: mostly DOES-NOT-TRANSFER; one piece EXTENDS

**The Linux practice (primary, `2.Process.rst`).** A ~2-week merge window at ~1,000 changes/day,
then 6–10 weeks of -rc stabilization where "only patches which fix problems" land; "exactly one
person who can merge patches into the mainline kernel repository: Linus Torvalds," scaled by "a
lieutenant system built around a chain of trust" of subsystem maintainer trees; and linux-next as
"a snapshot of what the mainline is expected to look like after the next merge window closes" —
the integration gate code must pass through *before* the window.

**What does not transfer.** The maintainer hierarchy and the time-based merge-window/-rc cadence
solve coordination and throughput problems among thousands of mutually-unknown contributors with a
single trusted integrator. dowiz's equivalents already exist in simpler form and are not the
bottleneck: operator gate on `main` (`ROADMAP-GROUND-TRUTH-2026-07-14.md`: main = frozen anchor,
main-merge = operator gate) plays the Linus role; feature branches/worktrees play subsystem trees;
there is no release cadence to stabilize toward. Importing windows or a lieutenant layer would add
ceremony that addresses no observed failure — the DECART probe kills it. **This is the clearest
cargo-cult risk in the whole comparison.**

**The piece that transfers: linux-next's function.** linux-next exists to answer "do these
independently-developed trees still compose?" *before* they hit mainline. dowiz has precisely this
failure class, documented: the shared-working-tree TOCTOU incident (`AGENTS.md:263-299`) and the
standing cross-branch todo map exist because parallel arcs (`feat/harness-llm-backend`,
`feat/agentic-mesh-protocol-2026-07-17`, `feat/spectral-energy-flow-evolution`) develop against
each other blind. **Adoption (proposal, lightweight):** a throwaway integration check — an
ephemeral octopus/sequential merge of the active arc branches in a scratch worktree +
`cargo test`, run on demand (before an operator merge session) or on a timer. It is a *probe*, not
a branch anyone develops on; its only output is "these arcs still compose: yes/no + first
conflict." Cost: one small native tool or ci-truth subcommand. Not urgent; file under P-later with
the trigger "≥2 arcs touching the same crate concurrently" — which is already true of
kernel/src, so schedule it with the next governance wave.

### A4. coding-style.rst — verdict: ALREADY-EQUIVALENT (mechanics) / REINFORCES (philosophy)

**The actual content (primary, not folklore):** 8-char tabs; 80-col soft limit with an explicit
never-break-grep-able-strings carve-out; K&R braces; "if you need more than 3 levels of
indentation, you're screwed anyway"; anti-Hungarian ("encoding the type of a function into the name
... is asinine"); "functions should be short and sweet, and do just one thing," ~5-10 locals;
centralized exit via `goto` with descriptive labels; "NEVER try to explain HOW your code works in a
comment" — comment what/why; a ban on editor modelines in source files; a skeptical typedef stance
(typedefs hide whether a thing is a struct or pointer).

**dowiz mapping.** Everything mechanical is owned by rustfmt/clippy and transfers as tooling, not
doctrine. The C-specific items invert or dissolve in Rust: goto-cleanup → `?` + `Drop`; the
anti-typedef stance is about *hiding* representation, and Rust newtypes do the opposite (make
representation distinctions type-checked), so the rule's *motivation* (the reader must not be lied
to about what a value is) survives while its letter reverses. What genuinely transfers is three
review-time sentences, all of which existing doctrine already implies: short single-purpose
functions (ponytail/innovate mode), why-not-how comments, and deep-nesting-as-design-smell. No new
rule needed; §G folds the comment discipline into an existing bullet. No adoption beyond that —
writing a dowiz style bible would duplicate rustfmt's job, which is the exact over-engineering the
repo's own ponytail discipline exists to refuse.

### A5. Stable-kernel rules — verdict: EXTENDS (small, fold into incident discipline)

**The practice (primary, `stable-kernel-rules.rst`, verbatim criteria):** a -stable patch "must be
obviously correct and tested," "cannot be bigger than 100 lines, with context," must fix "a real
bug that bothers people... an oops, a hang, data corruption, a real security issue" — explicitly
excluding theoretical-race fixes and cosmetic cleanups.

**dowiz mapping.** No stable branch exists and none is needed (one deployed build). But the
*criterion* is a precise, portable definition of what may land during stabilization or an incident:
small, obviously correct, fixes user-observed breakage, adds no features. The `incident` skill
already directs "stabilize with reversible actions first"; this adds the patch-shape rule to it.
Adoption: one sentence in §G. No mechanism, no cost.

### A6. Regressions vs. internal-API freedom — verdict: REINFORCES

Covered in A1 as the twin rule; called out separately because it independently confirms two
standing dowiz decisions from the outside: "core-immutable, integrations=ports"
(rust-native-bare-metal memory: adapters-not-purges) and the SCOPE RULE's dev-time-fence-vs-runtime
distinction (`ARCHITECTURE.md:23-27`). Kroah-Hartman's argument — a frozen internal API would
freeze bugs and deprecated patterns in place — is the same argument the hermetic P2 audit makes
against keeping dead canonical primitives. No new rule; the Frozen-Surface Ledger (A1) is the
actionable form.

---

## B. Security architecture

### B1. LSM hooks vs. dowiz's gate layer — verdicts: REINFORCES (restrictive-only) + EXTENDS (hook placement, composition)

**The Linux practice (primary: 2002 USENIX paper + `security/lsm.rst` + `admin-guide/LSM`).**
Three load-bearing design decisions:

1. **Hooks at the object, not the surface.** LSM rejected syscall-table interposition:
   interposition "is not race-free, may require code duplication, and may not adequately express
   the full context needed to make security policy decisions." Hooks fire inside the kernel, on the
   real object (`inode`, `task_struct`), *after* DAC checks — which "avoids time of check to time
   of use (TOCTTOU) races."
2. **Restrictive-only.** "LSM hooks are primarily restrictive: where the kernel was about to grant
   access, the module may deny access, but when the kernel would deny access, the module is not
   consulted" — the sole permissive exception being the pre-existing `capable()` logic.
3. **Ordered composition.** Modern stacking: capabilities always first, then any number of "minor"
   modules (Yama, Landlock, ...), then at most one "major" MAC module (SELinux/AppArmor/...), in
   `CONFIG_LSM` order. Every stacked module must independently pass.

**dowiz today.** The runtime object-level gate is genuinely LSM-shaped already:
`HybridGate::check` with its 14-variant enumerated deny axis (hermetic P4 §2d) and
`redline.rs` — "every requested red-line pair must be a subset of SOME allow entry"
(`/root/bebop-repo/bebop2/proto-cap/src/redline.rs:48,115`) — deny-by-default, restrictive,
typed. The **dev-time** layer is thinner than its documentation: the live matcher is a
six-substring check (`money.rs|order_machine.rs|event_log.rs|auth|otp|jwt` —
`tools/ci-truth/src/main.rs:237-245`) that does not cover RLS or migrations; the richer glob
matchers (protect-paths, serious-gate — the one that actually named
migrations/schema/rls/money together) survive only in git history (`5cd0c1b64~1`), neutralized by
operator directive. And `v1-verify` — a fully built and tested merge-gate contract
(DiffAttestation/Verdict TLV, K≠V key separation, `v1.rs:327-383`) — **is referenced by no
workflow**: in LSM terms, a hook compiled but never registered.

**What Linux adds:**

- **Name the restrictive-only invariant as a rule** (it is currently a property, not a law): *a
  gate anywhere in dowiz — CI, admission, capability check — may only narrow what an earlier layer
  allowed, never widen it.* The one legitimate permissive pattern (an explicit allow-list
  override, LSM's `capable()` analogue) must be a named, typed mechanism, never an ad-hoc bypass.
  This is hermetic P4 (one mechanism, two poles) applied specifically to authorization.
- **Check at the object, not the tool surface.** The TOCTOU rationale is the same lesson dowiz
  already paid for: the git-index race (`AGENTS.md:263-299`) was a check at the surface
  (`git status` glance) invalidated before use; the fix (worktree isolation, and `v5c-reexec`'s
  fresh-worktree re-execution) is a check at the object. The string-matcher gating on *paths in a
  diff* is surface-level; `v1-verify`'s attestation over actual diff content is object-level and
  already built. Adoption: wire `v1-verify` into CI (advisory first) when the operator restores
  gate friction — until then, record it in the roadmap as the successor to `is_redline`, and
  extend `is_redline`'s list with `migrations|rls|schema` now (one line, no governance change:
  it only widens a deny, per the rule above).
- **Composition order.** With multiple gates returning verdicts (cargo-test, v5c-reexec,
  claim-latency, future v1-verify), adopt the LSM stance explicitly: gates are independent,
  ordered, and conjunctive — all must pass; no gate reads another gate's verdict as input
  (independence is what RC-2 in the hermetic synthesis says dowiz's verification organs lack).

### B2. Capabilities — verdict: ALREADY-EQUIVALENT, plus one adopted guard-rule from Linux's own failure

**The Linux practice (primary, `capabilities(7)`).** Root split into ~40 independent capability
bits; five per-thread sets (permitted / effective / inheritable / bounding / ambient); file
capabilities. And a rare documented self-critique, verbatim from the man page's notes to kernel
developers: *"Don't choose CAP_SYS_ADMIN if you can possibly avoid it! ... It can plausibly be
called 'the new root' ... Don't make the problem worse."* The fine-grained-privilege design
partially failed because one bit became an overloaded catch-all.

**dowiz today.** `proto-cap` is verified to already be this pattern, and in one respect stronger:
`Capability { subject_key, subject_key_pq, scope, nonce, expiry }`
(`bebop2/proto-cap/src/capability.rs:45-60`); `Scope` is a *set* of (Resource, Action) pairs with
real set-subset attenuation — "UCAN 'narrow-only' attenuation actually narrows"
(`scope.rs:122-126,163-167`, the G4 fix); delegation chains verify narrow-only end-to-end
(`Delegation::sign` + `verify_chain`, exercised by the R4 test at `scope.rs:402-455`). The
delegation-chain subset rule is a *per-chain* bounding set — structurally stronger than Linux's
per-thread bounding set, because nothing anywhere in a chain can exceed its issuer. Inheritable/
ambient sets have no analogue because there is no `execve` boundary; nothing is missing there.

**The adopted rule — "no CAP_SYS_ADMIN":** the failure mode Linux documents is not missing
mechanism but scope-vocabulary decay — one variant accreting unrelated powers because adding a new
narrow variant is more work than reusing a broad one. Standing rule (§G): *no `Resource`/`Action`
variant may span more than one nameable resource class; adding a variant whose honest description
contains "and" or "admin" or "misc" requires DECART + operator gate.* Cheap now (the enums are
small, closed, `Copy` — `scope.rs:17,65`); nearly impossible to retrofit later, which is exactly
Linux's testimony.

### B3. Namespaces — verdict: EXTENDS (isolation-tier honesty), with the gap stated plainly

**The Linux practice (primary, `namespaces(7)`).** "A namespace wraps a global system resource in
an abstraction that makes it appear to the processes within the namespace that they have their own
isolated instance" — eight types, each virtualizing exactly one global resource; user namespaces
as the unprivileged-container enabler, with a documented security history (the `setgroups` class
of bugs) showing that isolation primitives themselves become attack surface.

**dowiz today (audit axis 3, the honest floor).** The real inter-agent boundary is a capability
check plus a token bucket **inside one address space**: `AdmissionRecord { tier: SandboxTier, caps,
bucket }` (`/root/dowiz-agentic-mesh/kernel/src/ports/agent/admission.rs:299-320`). The named
tiers overstate the floor: `WasmComponent` maps to fuel metering that is not compiled by default
(`agent-adapters/src/fuel.rs:174-180` behind non-default `wasmtime-fuel`), with **no
`ResourceLimiter`/memory cap anywhere in any repo**; `NativeProcessRequiresKvm` maps to a KVM
probe + fail-closed refusal (`kernel/src/isolation/microvm.rs:66-115`) with no VMM behind it and
seccomp existing only as a TODO comment (`microvm.rs:15`). One genuine OS primitive exists:
`unshare -n` network-namespace wrapping for untrusted shell commands, best-effort with silent
fallback (`/root/bebop-repo/crates/bebop/src/sandbox.rs:85-107`).

**Adoption (proposal): the isolation-tier honesty rule.** This is hermetic P7 (no self-certified
capability) applied to sandboxing: *an agent may be admitted at tier T only if T's enforcement
mechanism is compiled into the running build and probed at admission time; otherwise admission
fail-closes to the strongest tier actually present.* The KVM path already fail-closes — correct;
the rule generalizes that to every tier. Concrete near-term items it forces, in order of cost:
(1) `unshare -n` fallback becomes fail-closed-or-labeled, not silent; (2) when `wasm-host` runs
untrusted agents, `wasmtime-fuel` + a `ResourceLimiter` memory cap become default-on for that
build — fuel/memory limits are the WASM boundary's equivalent of what seccomp does for a process
(§B4); (3) the namespace principle proper — one boundary per resource (CPU via fuel, memory via
limiter, network via the port allow-list) — becomes the checklist for any future tier. Linux's
`setgroups` history supplies the accompanying caution for the DECART probe: each new isolation
primitive is itself new attack surface, so tiers land one at a time, each with its own probe test.

### B4. seccomp — verdict: REINFORCES (default-deny), plus the layering lesson

**The Linux practice (primary, `seccomp_filter.rst` + `seccomp(2)`).** Filter mode attaches a BPF
program over syscall number+args; the docs' self-description is deliberately modest: *"System call
filtering isn't a sandbox. It provides a clearly defined mechanism for minimizing the exposed
kernel surface."* Kill-process is the recommended default for unmatched syscalls;
`no_new_privs` is a hard prerequisite so a filter can never be used to confuse a more-privileged
program it execs.

**dowiz mapping.** Deny-by-default is already canon (M12 `deny-by-default capability-tokens`,
`ARCHITECTURE.md:33`; `check_port_scope` on every port — `proto-cap/src/port.rs`). What transfers
is the *composition stance*: seccomp, namespaces, capabilities, and LSM are four independent
layers, each reducing a different surface, none claiming to be the sandbox alone. dowiz's current
single in-process capability check is one layer where its threat model (untrusted third-party
agents, M5 hub anarchy) eventually requires the stack; §B3's tier rule is where each next layer
registers. The `no_new_privs` analogue is worth one standing sentence: *an admitted agent must
never be able to cause an admission decision wider than its own caps* — today guaranteed by the
delegation subset rule (B2); keep it true when admission gets new inputs.

### B5. "Security problems are just bugs" + warn-first-then-enforce — verdict: REINFORCES, and names an existing dowiz pattern

**The Linux practice (primary-verified, Torvalds 2017).** *"Security problems are just bugs ...
the patches you then introduce for things like hardening are primarily for DEBUGGING ... let's
warn about what looks dangerous, and maybe in a _year_ when we've warned for a long time ...
_then_ we can start taking more drastic measures."* KSPP's complementary bar (kspp.github.io +
`self-protection.rst`): eliminate *classes* of bugs, features "will not be developer-'opt-in'",
assume "an unprivileged local attacker has arbitrary read and write access to the kernel's
memory," make the kernel "fail safely, instead of just running safely."

**dowiz mapping.** Three existing pieces of doctrine get outside confirmation: security findings
are handled as root-caused bugs with RED→GREEN proofs, not advisories (crypto-safe-first-pass:
the SSR-2020 batch-verify forgery was fixed by removing the unsound speedup — correctness over
throughput, exactly the just-bugs posture); fail-closed is canon (S5); and the claim-latency
detector already ships advisory-always-exit-0 (`main.rs:446-659`) — which is Linus's bake-in
doctrine practiced without being named. **Adoption: name it as the standing gate lifecycle** (§G):
*every new enforcement gate ships in warn-only mode, is promoted to blocking only after a stated
quiet period with zero false positives, and the promotion is a recorded decision.* This also
gives the currently-neutralized gate layer a principled re-entry path when the operator restores
friction: gates come back advisory, earn blocking status individually. KSPP's "not
developer-opt-in" supplies the counter-weight: once promoted, a gate applies to all code, no
per-agent exemptions — that half is Ananke.

---

## C. Regression, fuzzing, and sanitizer discipline

### C1. kselftest / KUnit — verdict: ALREADY-EQUIVALENT (in-tree tests) / defer the split

**The Linux practice (primary).** kselftest: in-tree under `tools/testing/selftests/`, TAP output,
runs against the running kernel, 45-second default per-test timeout, "designed to be quick";
regression tests travel in-tree with the fix. KUnit is the distinct in-kernel *unit* framework
(~100 tests <10 s), hardware-independent, CI-friendly. (The "~20-minute suite target" sometimes
attributed to kselftest was **not found** in the primary doc — flagged, not repeated.)

**dowiz today.** Tests are in-tree and travel with code: 84 `#[cfg(test)]` modules in dowiz, ~511
kernel+engine tests unconditionally in CI on every push (`ci.yml:118-140`) — the KUnit role.
Playwright e2e against the deployed URL plays the kselftest role (test the running system).
Regression-test-with-the-fix is already RED→GREEN doctrine. **No smoke/full split exists** (audit
axis 8) — and none is adopted: at current suite runtime the split solves a problem dowiz does not
have. Trigger recorded instead: when the unconditional suite exceeds ~5 minutes wall-clock, split
per kselftest's model (quick default target + full target), and adopt the per-test timeout
discipline at the same moment.

### C2. syzkaller / syzbot — verdict: GAP (the largest verified gap in this comparison)

**The Linux practice (primary).** syzkaller: "an unsupervised coverage-guided kernel fuzzer" over
the syscall surface, KCOV-guided. syzbot industrializes it: continuous fuzzing of mainline,
auto-reporting with generated reproducers (Syz-DSL first, C when possible), automated
cause-bisection to the introducing commit, fix-bisection, and fixed-everywhere tracking. Scale, as
a live snapshot (2026-07-17, syzkaller.appspot.com/upstream — changes continuously): 7,230 fixed
bugs, 1,281 open. The design lesson is not the tool but the posture: **the externally-reachable
input surface is fuzzed continuously by an independent, automated party, and every finding is
attributed to a commit.**

**dowiz today (audit axis 4).** Zero fuzz targets, zero `proptest`/`quickcheck`, zero
sanitizer/Miri/loom CI jobs **across all five repos** — checked against every lockfile and every
workflow, not just greps. One deliberate non-adoption exists on record: proto-wire hand-rolled a
xorshift PRNG "to avoid proptest's dev-tree" (`bebop2/proto-wire/src/sync_pull.rs:1038-1040`).
Const-time testing: real `ct_eq` + a heuristic timing test in `pq_kem.rs:412,580,673-717`;
`constant_time.rs:1-14` is an explicit placeholder stub. Meanwhile the verifiable-cognition arc
already recorded "bebop = 0 proptest/fuzz/dudect" as a known debt.

dowiz's syscall-surface equivalent is precisely enumerable — it is the untrusted-byte decode set,
which is also mostly A1's frozen-surface list: capability TLV (`proto-cap/src/tlv.rs`),
`signed_frame` parsing, `verify_chain`, proto-wire `sync_pull` message decode, event-log record
decode, and the wasm-host import surface. These parse attacker-controllable bytes in a PQ-crypto
trust boundary. The mixed-order Ed25519 forgery found in B4 review (memory:
crypto-safe-first-pass) is direct evidence this class of bug exists in this codebase and is
findable; it was found by an expensive human-style review — fuzzing is the cheap continuous
complement.

**Adoption (proposal).**

1. `fuzz/` subcrates (cargo-fuzz) in `bebop2/proto-cap` and `proto-wire`, one target per decoder:
   `fuzz_tlv_decode`, `fuzz_signed_frame`, `fuzz_verify_chain` (structured: random cap chains
   checked against the subset invariant), `fuzz_sync_pull`; in dowiz kernel: `fuzz_event_decode`.
   Each target's oracle is "no panic, no OOM, and decode-encode-decode fixpoint where applicable" —
   plus invariant assertions (a verified chain's effective scope ⊆ root scope: the B2 invariant as
   a fuzz property).
2. CI: a short deterministic run per PR (`-runs=` bounded, corpus committed), plus a longer cron
   run — the syzbot cadence at hobby scale. Findings minimized into the corpus as regression
   inputs (travels-with-the-tree, per C1).
3. The syzbot attribution lesson interlocks with A2: fuzz findings bisect to a commit only if
   commits are bisectable — adopt them together.

**DECART (new dev-dependency — mandatory).**

| Candidate | Bare-metal fit | Falsifiable correctness | Perf | Supply-chain | Maintainability | Reversibility |
|---|---|---|---|---|---|---|
| **cargo-fuzz (libfuzzer-sys)** | separate `fuzz/` crate; production crates stay zero-dep (M6 intact) | coverage-guided, corpus = reproducible regression set | in-process, fast | Rust-project-adjacent tooling; dev-only, never shipped | one target file per decoder | delete `fuzz/` dir |
| proptest | dev-dep in the production crate's dev-tree — the exact objection recorded at `sync_pull.rs:1038` | property-based, no coverage guidance | fast | heavier dev-tree | good | easy |
| hand-rolled xorshift harness (status quo pattern) | zero deps | no coverage guidance, no corpus, no minimization | fast | none | every harness bespoke | n/a |

DECISION: cargo-fuzz, because coverage guidance is the load-bearing property (it is what made
syzkaller outperform blind fuzzing) and the `fuzz/`-subcrate layout sidesteps the recorded
dev-tree objection that killed proptest. Older-as-adapter note: the existing xorshift harnesses
stay for the structured tests they already serve — not purged. Probe (strongest argument
against): libfuzzer needs a nightly toolchain for `-Z` sanitizer flags, adding a toolchain
dimension to CI; and fuzzing PQ verify paths burns CPU on mostly-rejected inputs. Answer: run
fuzz CI on nightly in its own job (isolated from the stable build), and structure-aware targets
(mutate valid frames) spend most cycles past the signature check. If nightly proves too brittle in
CI, fallback documented: `cargo fuzz` locally + cron on the Hetzner box only.

### C3. KASAN / UBSAN / KCSAN / lockdep — verdicts: mostly covered by Rust; two small adoptions; lockdep DOES-NOT-TRANSFER-YET

**The Linux practice (primary).** KASAN: shadow-memory OOB/UAF detection, three modes scaling from
CI-heavy to in-field. UBSAN: compiler-inserted UB checks (signed overflow, misalignment). KCSAN:
watchpoint-sampling *data-race* detector. lockdep: builds a graph over lock **classes** with
observed acquisition-order edges; a single observed A→B plus a single observed B→A anywhere, ever,
proves deadlock *potential* — "with a 100% certainty that no combination and timing of these
locking sequences can cause any class of lock related deadlock" when clean. Its key property:
it converts one benign run into a proof over a class of schedules.

**dowiz mapping, honestly.**

- **KASAN → the Rust type system**, except inside `unsafe`. Kernel/engine are effectively
  safe-Rust, std-only. Adoption (cheap, targeted): a CI-adjacent `cargo miri test` run **scoped to
  crates that contain any `unsafe`** — a grep gate: if `unsafe` count is zero, the job is a no-op.
  Verify the actual unsafe census at implementation time (not audited this pass — flagged).
- **UBSAN → overflow semantics.** Rust release builds wrap silently unless `overflow-checks = true`.
  The kernel computes money in integers (`kernel/src/money.rs`); a silent wrap in a money path is
  exactly the P4 pole-collapse class (compare `unwrap_or(0)` finding #11 in the hermetic table).
  Adoption: `[profile.release] overflow-checks = true` for kernel (+ checked/saturating ops
  already-or-hereafter explicit in money paths). Cost: one Cargo.toml line + a benchmark check
  via the existing `bench_track.py` gate to confirm the cost is noise. This is the single
  cheapest adoption in this document.
- **KCSAN/lockdep → DOES-NOT-TRANSFER-YET.** kernel/src is single-threaded with no shared-state
  locking (audit axis 5: no `RwLock`/`Arc` at all; one `Mutex` each in `TokenBucket` and
  `InMemoryStore`, used non-concurrently today; `OnceLock` write-once singleton at
  `retrieval/recall.rs:306`). Real threading lives one layer up
  (`llm-adapters/src/dispatch.rs:89`: thread-per-job + `Arc<TokenBucket>`, `cache.rs:33`:
  `Arc<Mutex<S>>`) with at most two lock classes and no observed nesting. lockdep-style machinery
  for ≤2 lock classes is ceremony. **Named trigger instead (the lockdep lesson kept, the tool
  deferred):** the moment any code path can hold two dowiz locks simultaneously, adopt (a) a
  documented total lock order and (b) a `loom` model test for that pairing — DECART for loom to be
  run at that trigger, not now. Until then Rust's `Send`/`Sync` plus single-threaded kernel is the
  operative guarantee.

---

## D. Performance & concurrency patterns

### D1. RCU — verdict: DOES-NOT-TRANSFER-YET; pattern recorded with a trigger

**The Linux practice (primary: `whatisRCU.rst`, McKenney's LWN series).** Split updates into
removal and reclamation, separated by a grace period; readers run lock-free and never block
("Only readers that are active during the removal phase need be considered"); publish/subscribe
via `rcu_assign_pointer`/`rcu_dereference`. Fits read-mostly data where momentary staleness is
tolerable; the trade is near-zero read cost against deferred reclamation.

**dowiz mapping.** RCU solves reader-side cache-line contention under concurrent read-heavy load.
The audit found **no concurrently-shared read-heavy state anywhere in kernel/src**: the read-heavy
structures (`PriceCatalog` `catalog.rs:30`, `RoadGraph` `router.rs:47`, `KnowledgeSpine`
`spine.rs:122`) are plain owned structs, single-threaded by construction; the one
write-once/read-many case already uses the right primitive (`OnceLock`). Adopting an RCU-shaped
crate today would add machinery with zero contended readers to serve. **Trigger recorded:** if the
dispatch layer (or a future multi-threaded hub runtime) ever shares a mutable
config/routing/catalog across threads with read-mostly access, reach for `arc-swap`
(atomically-swappable `Arc`, RCU's publish/replace shape) or `left_right` before `RwLock` —
verified-live Rust analogues, with `crossbeam-epoch` for lock-free structures needing deferred
reclamation. That choice gets its DECART at the trigger. The audit's one near-term candidate:
`llm-adapters/src/cache.rs:33`'s `Arc<Mutex<S>>` would become `arc-swap`-shaped *if* it ever
becomes read-contended — measure first via the existing bench gate, per the perf skill's
no-premature-optimization rule.

### D2. Per-CPU data — verdict: DOES-NOT-TRANSFER-YET; same trigger discipline

**The Linux practice (primary, `this_cpu_ops.rst`).** Per-CPU copies eliminate cache-line
bouncing — "no concurrent cache line updates take place" — at the price of "the need to add up the
per cpu counters when the value of a counter is needed"; the stated main use is counters.

**dowiz mapping.** Same structural answer as D1: contended shared counters do not exist yet (the
only shared counter is `wasm.rs:51`'s `AtomicU64` id source; P24's telemetry design owns the
hot-path counter story and already imports the per-context-buffer idea from Linux's ring-buffer
design — deliberately not duplicated here). Trigger: when a counter shows up contended in a
`bench_track.py` regression, shard per worker thread and aggregate on read. One sentence of
pattern, zero machinery today.

### D3. "Mechanism, not policy" — verdict: REINFORCES (dowiz already practices a stronger form)

**Provenance flag:** no primary kernel-doc statement of the phrase was located this session
(secondary provenance: Brinch Hansen's RC 4000; Raymond's *TAOUP*). Used on merit.

**dowiz mapping.** The SCOPE RULE is this principle stated more radically than Linux states it:
"Every CI gate / policy ... applies to the canonical repo + operator's own build ONLY ... At
runtime every hub is a sovereign Hydra ... No dev-time gate may block a hub from self-evolving"
(`ARCHITECTURE.md:23-27`); M10 "inter-hub protocol = defined; intra-hub anarchy = allowed"; the
Trait-as-Port + adapter-crate layout is mechanism (kernel primitives) with policy pushed to
config/adapters (M5: "capability/backend choice is config, never a hard-coded fork" —
`AGENTS.md:208-210`). Adoption: one review question added to the planning protocol's checklist
(§G): *"is this new gate/primitive a mechanism a hub can override, or has policy leaked into the
kernel?"* — the one-sentence form of what the HYDRA-contradiction sweep (`ARCHITECTURE.md:127`)
did as a batch, made recurring.

---

## E. Configuration & modularity — Kconfig

**The Linux practice (primary, `kconfig-language.rst` + `kbuild/kconfig.rst`).** An explicit typed
dependency graph over ~10k CONFIG symbols; `depends on` as the sound forward constraint vs.
`select` as the unsound reverse one, with the doc's own warning — "select will force a symbol to a
value without visiting the dependencies. By abusing select you are able to select a symbol FOO
even if FOO depends on BAR that is not set." And config-space *testing*: `allnoconfig`,
`defconfig`, `randconfig` (seeded, tunable) — random sampling of the combinatorial config space
because nobody can enumerate it, run continuously by 0day/kernelCI bots.

**dowiz today (audit axis 6).** Feature gates exist and are principled: kernel
`default=["std"]`, `wasm` (keeps native consumers serde-free), `pgrust` (keeps the kernel
network-free unless opted in); engine `gpu`/`webgl`/`webgpu`/`splat` deliberately empty so CI
needs no GPU toolchain (`kernel/Cargo.toml`, `engine/Cargo.toml`). **But CI compiles exactly one
point of the config space:** zero `--features`/`--no-default-features`/`cargo hack` occurrences in
any workflow — `wasm`, `pgrust`, and all four engine flags are *never built in CI*. The failure
mode is silent feature bit-rot: exactly the class randconfig exists to catch, and dowiz has
already had the adjacent incident (hermetic finding #12: the `wasm` feature's funnel JSON leaking
HashMap order — a defect living precisely in a non-default-compiled path).

**Adoption (proposal): feature-matrix CI.** With 3 kernel features and 4 engine flags the
powerset is trivially small (≤2³ and ≤2⁴ build points) — dowiz does not need random sampling; it
can afford what Linux cannot: **exhaustive** config-space *build* coverage. CI job:
`cargo hack check --feature-powerset` (build/check only; test under default features stays as-is)
for kernel and engine. Cargo's additive-features model makes Kconfig's `select`-unsoundness
mostly moot; the surviving analogue is worth one review sentence: a feature must not silently
force behavior of another feature it doesn't declare (`wasm` requiring `std` must be declared in
the feature graph, not assumed).

**DECART (new dev-tool — mandatory).**

| Candidate | Fit | Correctness | Perf | Supply-chain | Maintainability | Reversibility |
|---|---|---|---|---|---|---|
| **cargo-hack** | CI-only binary install; zero code/dep footprint in any crate | exhaustive powerset, deterministic | ≤24 `cargo check` invocations, cached | widely-used cargo subcommand, dev-only | one workflow line | delete the job |
| hand-rolled matrix in YAML | zero new tools | must be hand-maintained as features change — a hand-mirror of the feature graph, the exact drift class PACING-CONTRACT exists to kill | same | none | every new feature edits CI | easy |
| native Rust tool in `tools/` | matches minimize-bash directive | re-implements cargo-hack | same | none | new crate to maintain | easy |

DECISION: cargo-hack — the hand-rolled matrix fails on maintainability-by-drift (it re-creates the
unpinned-mirror bug class), and a native reimplementation fails ponytail (rebuilding a solved
dev-tool). Older-as-adapter: n/a (nothing replaced). Probe (strongest argument against): the
minimize-bash/native-tools directive could be read as demanding the `tools/` reimplementation.
Answer: that directive targets bash *as the logic layer*; `cargo hack` is an external binary
invoked like `git`/`zstd`, which the directive explicitly permits — and the feature list is read
from Cargo.toml, so nothing is mirrored by hand.

---

## F. Ranked adoption table

| # | Practice (cluster) | Verdict | Cost | What it closes | When |
|---|---|---|---|---|---|
| 1 | Fuzz the wire-decode surface: cargo-fuzz targets on TLV/signed-frame/verify_chain/sync_pull/event-decode + PR-short/cron-long CI (C2) | GAP | fuzz/ subcrates + nightly CI job | 0-fuzz-anywhere floor on a PQ trust boundary; proven bug class (SSR-2020 forgery) | first governance wave |
| 2 | Frozen-Surface Ledger + explicit internal-instability twin (A1/A6) | EXTENDS | 1 doc + ~6 golden-byte pin tests | unwritten wire-compat contract, before second consumers multiply | with mesh Wave-0 follow-up |
| 3 | Feature-powerset CI via cargo-hack (E) | GAP | 1 CI job | wasm/pgrust/engine flags never compiled in CI; known bit-rot class (finding #12) | immediately (cheapest structural fix) |
| 4 | `overflow-checks = true` release profile for kernel (C3/UBSAN) | GAP | 1 line + bench confirmation | silent wrap in integer money math (P4 pole-collapse class) | immediately (cheapest single line) |
| 5 | Bisectable commits: rule + per-commit worktree build/test job, advisory-first (A2) | GAP | v5c-reexec extension | regression-hunt's decayed precondition; fuzz/bench culprit attribution | with #1 (they interlock) |
| 6 | Gate lifecycle: warn-only → quiet period → blocking, promotion recorded; restrictive-only invariant named (B1/B5) | REINFORCES→rule | doctrine only | principled re-entry path for the neutralized gate layer | doctrine now; wiring at operator's gate-restore |
| 7 | Isolation-tier honesty: tier admitted only if enforcement compiled+probed; fuel+ResourceLimiter default-on for untrusted wasm (B3) | EXTENDS | rule now; limiter work at mesh B2/B3 phases | SandboxTier claims exceeding the floor (P7 self-certification in type form) | rule now; mechanism with mesh arc |
| 8 | "No CAP_SYS_ADMIN" scope-vocabulary rule (B2) | rule from Linux's failure | doctrine only | scope-decay before it starts | now |
| 9 | `is_redline` += `migrations|rls|schema`; record v1-verify as its successor (B1) | EXTENDS | 1 line | narrowest matcher gap (deny-widening only — no governance change) | now |
| 10 | Arc-integration compose-probe (linux-next function) (A3) | EXTENDS | small ci-truth subcommand | cross-worktree blind composition | trigger already met; next governance wave |
| 11 | miri-if-unsafe CI no-op job (C3) | GAP (small) | 1 job | unsafe-block coverage, currently unmeasured | with #3 |
| 12 | Stable-patch criterion folded into incident discipline (A5) | EXTENDS | 1 sentence | incident-time patch shape | now |
| — | RCU/arc-swap, per-CPU sharding, loom/lock-order, smoke/full split | DOES-NOT-TRANSFER-YET | triggers recorded in §C3/§D1/§D2/§C1 | — | at trigger |
| — | Maintainer hierarchy, merge windows/-rc cadence, style bible, stable branches | DOES-NOT-TRANSFER | — | see §H | never absent new facts |

---

## G. Proposed AGENTS.md additions

> **Not applied.** `AGENTS.md` and `.claude/CLAUDE.md` are governance-sensitive; per the standing
> gate-topology rule the unlock is the operator's own action. The block below is paste-ready and
> self-contained; adopting rows 6/8/12 of §F is purely this text, no code.

```markdown
## Linux-derived engineering rules (adopted 2026-07-17 — see
## docs/design/BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md for sources & DECARTs)

1. **Never break the wire (Frozen-Surface Ledger).** The surfaces listed in the Frozen-Surface
   Ledger (wire TLV, signed frames, event-log encoding, content-address derivations, public API)
   carry a never-break guarantee over *observed peer behavior*, not just documented API; each
   ledger row has a golden-bytes pin test. Everything NOT on the ledger is explicitly unstable —
   internal refactors need no compatibility ceremony. Sole exceptions: security-fix-as-lesser-evil
   (operator-gated) and provably-unconsumed surfaces.
2. **Bisectable commits.** Every commit pushed to a feature branch builds and passes kernel+engine
   tests standalone; one logical change per commit. Series >10 commits: squash to logical units.
   Enforcement: per-commit worktree re-exec CI job (advisory until promoted per rule 4).
3. **Restrictive-only gates.** Any gate (CI, admission, capability check) may only narrow what an
   earlier layer allowed, never widen it. Permissive overrides exist only as named, typed
   mechanisms. Gates compose independently and conjunctively; no gate consumes another gate's
   verdict as input.
4. **Gate lifecycle (warn → prove → enforce).** Every new enforcement gate ships warn-only,
   promotes to blocking only after a stated quiet period with zero false positives, and the
   promotion is a recorded decision. Once blocking, a gate applies to all code — no per-agent
   exemptions.
5. **No catch-all scope ("no CAP_SYS_ADMIN").** No capability Resource/Action variant may span
   more than one nameable resource class; a variant whose honest description needs "and"/"admin"/
   "misc" requires DECART + operator gate.
6. **Isolation-tier honesty.** An agent is admitted at sandbox tier T only if T's enforcement
   mechanism is compiled into the running build and probed at admission; otherwise fail-closed to
   the strongest tier actually present. Best-effort fallbacks must be labeled or refused, never
   silent.
7. **Fuzz the decoders.** Every decoder of untrusted bytes on a frozen surface ships with a
   cargo-fuzz target (no-panic + invariant oracle); minimized findings join the committed corpus
   as regression inputs.
8. **Exhaustive feature-space builds.** CI builds the full cargo feature powerset (cargo-hack) for
   kernel and engine; a feature that needs another feature declares it in the feature graph.
9. **Mechanism-not-policy review question.** Every new gate/primitive answers in its plan: "is
   this a mechanism a hub can override (SCOPE RULE), or has policy leaked into the kernel?"
10. **Incident patch shape.** During stabilization/incident response, only patches that are small,
    obviously correct, and fix user-observed breakage may land; no features ride along.
```

---

## H. What does NOT transfer, and why — stated as required

- **The maintainer hierarchy and merge-window/-rc cadence (A3).** These solve trust-scaling and
  release-coordination among thousands of mutually-unknown contributors funneling through one
  integrator. dowiz's operator gate + feature-branch/worktree model already provides the
  chain-of-trust function at its actual scale; importing windows or lieutenants would add pure
  ceremony against no observed failure. This is the single clearest non-transfer.
- **lockdep/KCSAN machinery today (C3)** — ≤2 lock classes, no nesting, single-threaded kernel;
  the *lesson* (prove deadlock-freedom from structure, not from luck) is kept as a trigger rule.
- **RCU and per-CPU sharding today (D1/D2)** — no contended readers, no contended counters;
  triggers + verified Rust analogues recorded so the reach-for is right when the day comes.
- **A prose style bible (A4)** — rustfmt/clippy own the mechanics; duplicating them in doctrine
  violates the repo's own ponytail discipline.
- **Stable branch trees (A5)** — one deployed build; only the patch-shape criterion transfers.
- **KASAN as a tool (C3)** — the Rust type system is the adoption; only the unsafe-block residue
  needs Miri, and only where `unsafe` exists.

---

## I. 2-question doubt audit (AGENTS.md ritual, applied to this artifact)

**Q1 — least confident about (not rounded down):**
1. The proto-cap delegation-chain description rests on `scope.rs`/`capability.rs` reads plus a
   test citation (`scope.rs:402-455`); I did not read `verify_chain`'s body itself —
   "per-chain bounding set" is inferred from the R4 test's assertion, not from the verifier line
   by line.
2. The kernel `unsafe` census (C3) was never run; the miri-if-unsafe job is designed to be
   correct either way, but the claim "effectively safe-Rust" is an inference from the std-only
   audit, not a grep I performed.
3. The 2012 Torvalds mail wording is secondary-corroborated only (bot-walled archives); the
   policy content is independently primary-verified via handling-regressions.rst, so no
   recommendation rests on the quote alone.
4. Frozen-surface candidate list (A1 table) is marked "verify at implementation" — I have not
   confirmed that each listed file is the *only* encoder of its surface (P2's four-Laplacians
   finding shows duplicate implementations are a real pattern here).
5. cargo-fuzz's nightly-toolchain requirement in this CI environment is asserted from general
   knowledge, not probed against the actual runners; the fallback (local + Hetzner cron) is
   stated in the DECART for exactly this reason.
6. Per-commit CI cost (A2) assumes the current suite stays fast; no wall-clock measurement of the
   full suite was taken this session.
7. The claim "no ResourceLimiter anywhere" is the audit subagent's grep, relayed; I spot-verified
   fuel.rs but not the absence claim myself.

**Q2 — the biggest thing I might be missing:** the operator suspended the governance gates
deliberately (2026-07-15), and half of this document's adoptions are gates. If the suspension
reflects a standing judgment that gate friction costs more than it catches, then rows 5/6/9 of §F
are pushing against a decided direction rather than filling a gap. The honest reading of the
evidence (advisory-mode claim-latency was built *after* the suspension) is that the operator
wants *observation without friction* — which is why every proposed gate here enters warn-only
under rule G-4, making the blueprint compatible with either reading. But the call is the
operator's, and this document should not be read as re-litigating the suspension.

## J. Anu / Ananke check

**Anu (does each decision follow from evidence in front of me?):** every ADOPT row cites both the
Linux primary source and the dowiz file:line gap it closes; every DOES-NOT-TRANSFER names the
missing problem. Weakest link: A3's compose-probe rides on one incident + the todo-map's existence
rather than a recurrence count — flagged as lower-urgency accordingly. No decision in this
document rests on "Linux does it" alone; the two places authority could have leaked
(mechanism-not-policy phrasing, 2012 mail) carry explicit provenance flags.

**Ananke (is the good outcome structural, not remembered?):** rows 1, 3, 4, 5, 11 are CI jobs or
profile flags — structural by construction. Rows 2's pin tests fire on any drift. Rows 6-10 are
doctrine and therefore *remembered* unless the operator restores hook enforcement — this is
stated, not hidden: they are written as paste-ready AGENTS.md text (the same standing-instruction
mechanism the DECART rule uses today) with G-4 defining the promotion path to structural
enforcement when friction returns. The document's own done-check: every §F row above the
DOES-NOT-TRANSFER line names a concrete artifact (job, test, subcommand, or doctrine block) that
either exists after implementation or doesn't — falsifiable per phase.

---

*Research basis: three primary-source passes (development-process; security-architecture;
testing/concurrency/Kconfig) and one 9-axis repo audit, 2026-07-17. Key primary sources:
docs.kernel.org process/dev-tools/security/kbuild/RCU trees (verbatim-checked against
torvalds/linux master), capabilities(7)/namespaces(7)/seccomp(2) (man7.org), Wright et al. 2002
USENIX LSM paper, Torvalds 2017 LKML (lkml.iu.edu hypermail, primary-verified), Torvalds 2012
LKML (secondary-corroborated — flagged), github.com/google/syzkaller docs + live syzbot dashboard,
McKenney's LWN RCU series (Articles 262464/263130/264090), kspp.github.io. No product code, no
AGENTS.md/CLAUDE.md edits made.*
