# SELF-CRITIQUE — Two-Question Doubt Audit (decorrelated verifier pass, 2026-07-16)

> **Role:** independent adversarial verifier, NOT part of the 25-agent pipeline (5 Fable gap-analyses →
> 1 Fable merge → 19 Opus blueprints) that produced this roadmap. Same decorrelated-verifier posture the
> project already uses (`research-verifier`, `security-sentinel`): a different pass catches what the
> original pass's own blind spots would miss. Read-only. Nothing in the roadmap or canon was edited.
> **Corpus read:** MASTER-ROADMAP, R2-MERGED-PHASE-ROADMAP, ARCHITECTURE.md, STRATEGIC-VECTORS-LOCKED,
> blueprints P05/P15 (full) + P04/P09/P13 (skim), reports R1-A/R1-B (full) + R1-C/R1-D/R1-E (skim).
> **Investigations run against live code** at dowiz HEAD `fab17275a`, bebop-repo HEAD `b87b7e2`.
> Method models the project's own documented failure mode (BRAIN-TOPOLOGY: "self-certification = claim
> replaces check"; "append-only law bias"). Two outcomes are both valid: an investigation that CONFIRMS a
> load-bearing problem, and one that CLEARS a doubt. Both are recorded below.

---

## §1 — Question 1: "What are you least confident about right now?"

Seven concrete things the pipeline did not properly re-verify, took on faith from an earlier link, or
mislabeled. Each: what it is · where in the 25-agent chain it originated · investigation performed ·
verdict (CLEARED or CONFIRMED-LOAD-BEARING).

### 1.1 — llama.cpp/vLLM "self-host LLM execution is GPU-unlock-gated" (CONFIRMED — see §3 for full)

**What it is.** E13/E14 (self-host llama.cpp/vLLM) is deferred behind a single "GPU-unlock" trigger,
concretely defined as `cargo add wgpu`.

**Chain origin.** Canon `ARCHITECTURE.md:34` ("managed-advisory until GPU-unlock") → R1-B §E13/E14
*reinforced* it ("blocked on the compute/GPU cluster's unlock (E21-25) — do not schedule before it") →
R2 **O18** fused the two into one trigger ("GPU-unlock (network `cargo add wgpu`) … | P17 video/wgpu;
**P15 E13-execution**") → P05 §8 and P15 §9 propagated verbatim. **No link in the chain challenged the
premise that a self-hosted LLM needs a GPU.** R1-B is where the reinforcement hardened; the merge is
where the conflation was written into one operator decision.

**Verdict: CONFIRMED LOAD-BEARING ERROR.** Full investigation and root-cause in **§3**. Headline: a
llama.cpp server binary is *already installed on this host* (ollama 0.30.9, `/usr/local/lib/ollama/
llama-server`), HuggingFace (GGUF weights) returns HTTP 200, there is **no GPU** (`no /dev/nvidia*`),
and `cargo add wgpu` (crates.io) returns 403 — proving the two things bundled under "GPU-unlock" have
*different current states*. This is the operator's flagged item; it is real.

### 1.2 — "Graph-math lands ONCE in Phase 4, consumed by Phase 9" spans a repo boundary with no path (CONFIRMED)

**What it is.** R2's merge table (§1, "Major merges performed") resolves a naming collision by decreeing:
"Math primitives land ONCE in Phase 4 (**kernel**, zero-dep), consumed by Phase 9 (**mesh** heal)."
The master doc and P04 restate this as settled.

**Chain origin.** R1-A independently assigned Dijkstra/A*/UF/MST to its **Phase B (bebop2 mesh repo)**;
R1-C independently assigned F45/F46 route math to the **dowiz kernel**. Two reports named the *same math*
in *two different repos*. R2's merge "resolved" the collision by asserting a single home (dowiz kernel)
+ cross-consumption — but did not verify the consumption path exists.

**Investigation.** `/root/dowiz/kernel` and `/root/bebop-repo/bebop2` are **separate git repositories**.
`grep bebop2|proto-cap|mesh-node /root/dowiz/kernel/Cargo.toml` → **no dependency**. M6 (canon) forbids the
protocol from taking any external crate dependency at the wire/trust boundary. So "math lands once in the
dowiz kernel and Phase 9's bebop2 mesh consumes it" has, today, **no dependency edge and a canon law
(M6) standing against creating one.** The only buildable readings are (a) *duplicate* the math into
bebop2 (contradicts "once"), or (b) publish/vendor a shared crate (a cross-repo decision the roadmap
never scopes, and which brushes M1's current zero-integration reality).

**Verdict: CONFIRMED — the merge's marquee de-duplication is not proven buildable as written.** It is not
fatal (a shared vendored crate is a legitimate answer) but "resolved: land once, consume twice" is
asserted, not demonstrated, and it silently crosses a repo boundary the flat phase table hides.

### 1.3 — "146 distinct anchors, zero dropped, proven not asserted" (CLEARED — with one caveat)

**What it is.** The master §4 and R2 §5 claim the accounting is "reproducible / checkable, not asserted."

**Chain origin.** R1 reports surfaced E10≡E36 and D5/D8-undefined → R2 §5 built the tally → master §4
trusts R2.

**Investigation (re-derived the tally by hand).** Per-phase counts sum: 12+8+8+2+6+2+2+12+20+8+8+14+11+5+9+6+4+4+5
= **146**. Nominal decomposition M1-12(12)+V1-6(6)+D1-8(8)+S1-9(9)+E1-62(62)+F1-50(50) = **147**. Distinct
= 147 − 1 (E36 alias) = 146; of which 144 defined + 2 placeholders (D5,D8). Each phase's listed anchor set
re-counted individually — all match the tally row. **The arithmetic is internally sound.**

**Verdict: CLEARED — but "proven" is slightly overstated.** The 146 is *proven modulo two unratified
operator assumptions*: E10≡E36 being a true duplicate (O2, unratified) and D5/D8 being genuinely
undefined (if the operator ratifies definitions the distinct count shifts). So it is "arithmetically
consistent given two pending rulings," not "proven" in the absolute sense the wording implies. Minor.

### 1.4 — File:line citation freshness across the chain (MOSTLY CLEARED — no fabrication, but drift is live)

**What it is.** R1 read code at one HEAD; blueprints cite the same lines minutes-to-hours later; the
master trusts the blueprints. The operator asked whether code moved underneath the citations.

**Investigation (spot-checked the load-bearing crypto/router citations at current HEAD):**
- `roster.rs:114/167/179` — line 114 "Ed25519 signature (64 bytes)", 167 `sign::sign`, 179 classical
  `verify`. **Confirms the F21 "trust root is Ed25519-only / classically forgeable" claim.** ✓
- `sign.rs:625` `fn mod_l`, `:719` "Tracked as C4b … HIGH priority". **Confirms C4b is real and open.** ✓
- Stranded router `crates/bebop/src/cost_estimate.rs`: `pub fn route(` at **209**, A* at **238**, file 482
  lines. R1-A cited "**238-290**"; R2/P04 cited "**205-290**". Both are loose, overlapping, and neither is
  wrong-but they **disagree**, showing the range was paraphrased/re-derived downstream, not pinned. The
  underlying fact (a real zero-dep A*/CH router is stranded here) holds. ✓
- `dowiz/kernel/src/domain.rs:524` — the "one comment" proving zero protocol integration: line 524 is a
  comment ("…mirrors bebop2 BP-21…"). The claim (dowiz's only protocol reference is a comment) holds; the
  comment's *content* is slightly different from what "stray proto-cap comment" implies. ✓
- `apps/*` — `git ls-files 'apps/*'` = **0**. Confirms headline #2 (fully deleted). ✓
- **dowiz HEAD moved during this very session:** the system-prompt git snapshot shows tip `574f05604`;
  actual HEAD is now `fab17275a` (a newer "G9 breach alert" commit landed *after* the roadmap docs were
  written at 15:30-15:40). bebop-repo is unchanged (`b87b7e2`, exactly as R1-A cited).

**Verdict: MOSTLY CLEARED.** No fabricated citation found; every load-bearing crypto line re-verifies.
But two real signals: (a) two agents cite *different line ranges* for the same stranded router, and
(b) dowiz HEAD **already advanced** since the docs were written — the drift risk the operator named is
live, not hypothetical. Any implementer must re-pin citations at their working HEAD, not trust the
blueprint's line numbers.

### 1.5 — Several "falsifiable done-tests" presuppose infrastructure that doesn't exist yet (CONFIRMED — partial)

**What it is.** Master §6.4: "each phase's falsifiable done-test is the closure criterion." The operator
asked whether they are actually falsifiable *as written*.

**Chain origin.** R1 wrote per-phase done-tests → R2 tabled them → blueprints inherited them.

**Investigation (read the done-tests as an implementer would).** Several are *bootstrap-circular*:
- **P1's** done-test needs "a probe branch with a planted secret, a failing kernel test, an nginx `FROM`,
  a commit without Signed-off-by … EACH turn CI RED" — but the planted-failure probe harness and the
  restored CI are *what P1 builds*. P1 must construct its own falsifier before it can be falsified.
- **P6's** done-test is "a diff with a failing test cannot obtain a key_V GREEN (re-execution goes RED)"
  — but the independent-context re-execution mechanism (fresh worktree/clone runner) is the deliverable
  of P6 itself, and its isolation bar (O9) is *unresolved*. The test tests the thing that defines the
  test.
- **P3's** done-test assumes "a quantum-attacker fixture" and "a dudect/CT gate" — neither exists today
  (R1-A: ladder/wycheproof harness = `Placeholder` stubs). The fixture is itself unbudgeted work.

**Verdict: CONFIRMED (partial, not fatal).** This is normal bootstrapping — early phases must build their
own falsifiers — but the master's framing ("the done-test IS the closure criterion, not a subjective
looks-done") is softer than advertised for P1/P3/P6, where the criterion and the harness are co-built.
Relatedly: the roadmap calls every downstream GREEN "unverifiable since `f9ab28ff1`" (no CI) while
simultaneously resting on R1-A's *locally re-run* `cargo test` (232 core tests GREEN) as ground truth —
"unverifiable" means "un-CI-gated," which is weaker than the word suggests.

### 1.6 — Flat phase-numbering hides ~10x effort asymmetry; no time/skill budget anywhere (CONFIRMED)

**What it is.** 19 phases presented as peers in one table + one critical path. Anchor count is the only
size signal.

**Chain origin.** R2 assigned anchors→phases; master rendered them as a flat index; neither attaches
effort, calendar time, or required skill to any phase.

**Investigation (compared phases by actual work, not anchor count).**
- **P9 (20 anchors)** = ML-KEM session encryption + `NoopPayloadEnc`→real AEAD + full mesh-heal layer
  (HRW-merge, Dijkstra/A*, Union-Find partition detect, MST overlay) + revocation gossip loop + at-rest
  crypto relocation + D2/iroh DECART. This is **months of specialist cryptography + distributed-systems
  work.**
- **P4 (2 anchors)** = port a router + 6 geo functions + DSU/MST + wasm exports — also *weeks*, not a
  "small" phase despite 2 anchors.
- **P6/P7 (2 anchors each)** — P6 is an entire split-identity signing + CI-merge-gate pipeline; P7 is a
  subtle event-log dedup/prev-chaining money bug + compensation FSM. Neither is "small."
- **P13 (11 anchors)** = cross-hub distributed delivery: two real hubs with divergent internals,
  partition-then-merge double-finalization, saga ledger, first `.proto`/tonic. Deep distributed-systems.

**Verdict: CONFIRMED.** Anchor count is a *bad* effort proxy (P9's 20 ≈ P4's 2 in wall-clock, both large;
P17's 4 anchors are mostly blocked-on-trigger and near-zero work). The roadmap contains **no time
estimates and no skill callouts at all**, and the "Waves = maximum parallelism" framing tacitly assumes a
*fleet of parallel implementers* — which contradicts the single-operator/single-session reality this
project runs under. P3 (wire crypto), P9 (mesh distsys), P13 (cross-hub) each plausibly need deep
specialist time the roadmap neither budgets nor flags. This is a genuine planning gap, not a cosmetic one.

### 1.7 — The V1 verifier — the roadmap's "epistemic bedrock" — is itself single-party self-review re-skinned (CONFIRMED)

**What it is.** Master §1.5 and P6 position V1 (key_K signs diffs, key_V signs verdicts, "K≠V", merge
blocked without a verifier signature) as the fix for self-certification and the precondition for trusting
every downstream GREEN.

**Chain origin.** BRAIN-TOPOLOGY (self-certification = dominant failure) → STRATEGIC-VECTORS V1 → R1-B
§V1 → P6 → master §1 headline #5. The framing "verification today is self-context; V1 fixes it" runs the
whole length of the chain.

**Investigation (read V1's own escape clause).** STRATEGIC-VECTORS V1 admits in writing:
"identity-separation ≠ person-separation … **A+B is the enforced approximation, logged as such**." Both
keys are held by the **same operator/agent**; the "independent context" is (per O9, *unresolved*) at most
a fresh worktree on the **same machine running the same model family**. So the mechanism proposed to break
"one party certifies its own work" is **one party certifying its own work with two keys and a fresh
checkout** — cryptographically real, epistemically the same party. The roadmap acknowledges this honestly
in one line, then leans on P6 as "epistemic bedrock" for the other 146 anchors.

**Verdict: CONFIRMED LOAD-BEARING.** The foundation the roadmap says makes everything else trustworthy is,
by its own admission, a re-skin of single-party review, not a second party. This is the item to
*least* over-trust: "P6 landed, therefore downstream GREENs are now independently verified" would be
exactly the self-certification substitution BRAIN-TOPOLOGY warns about, one level up. It does add real
value (re-execution in a clean checkout catches non-reproducible GREENs) — but it is not, and cannot be
without a second human/model, the independence its "bedrock" framing implies.

---

## §2 — Question 2: "What's the biggest thing I'm missing?"

**The roadmap optimizes the PATH to a 147-anchor architecture while never being structurally able to ask
whether that architecture should be built at all — for a product that, by the roadmap's own headline
findings, has never completed a single real order, has zero protocol integration today, and just had its
entire product UI deleted.**

The operator suggested the meta-critique that "147 anchors, zero exceptions" is itself the
append-only/completionism bias BRAIN-TOPOLOGY flagged. That is **true and I confirm it** — but the sharper
form, the thing a fresh reader sees in 30 seconds, is the **inverted value-critical-path**:

- The roadmap's critical path is **P3 → P9 → P10 → P13 → {P14, P16} → P17**: crypto correctness →
  confidential wire → hub runtime → delivery spine → dispute/UI → demo. That is a path optimized so the
  *quantum-resistant self-healing mesh substrate* is load-bearing-first.
- **G11 — "first real order," the only evidence the product is wanted at all — is a done-test buried
  inside Phase 13 and re-mentioned in Phase 19.** Proof-of-demand is a *late side-effect* of building the
  substrate, not the gate that would justify building it.
- Read plainly: this is a plan to build ML-KEM confidential transport, kill-switches, recursive sub-hub
  spawning with capability-carried depth caps, escrow arbitration state machines, per-hub replicated
  graph-wikis, and unbounded living-organism self-modification — **before anyone has ordered a sandwich.**
  The 25-agent chain is meticulously honest about every *gap* in that edifice and structurally silent on
  whether the *edifice* is a solution to a problem anyone has.

Why the chain cannot see it: its task was "map the shortest honest path to all 147 anchors, zero
deferred." Given that framing, questioning the target is out of scope by construction — so the canon's
own possible overreach (147 decisions marked `LOCK`, escapable only by DECART, fixed *before* the first
order) is inherited wholesale. The roadmap markets "zero anchors dropped, zero exceptions" as its rigor;
that same total-coverage reflex **is** the append-only law bias, applied one level up — to the canon
instead of to a memory file. The completionism that looks like discipline is the failure mode wearing a
discipline costume.

The honest move the roadmap cannot make from inside its own frame: **most of these 147 anchors should be
DECART-*deferred* until a first real order justifies them.** A pre-first-order product does not need
recursive sub-hub depth caps or Kleros-adjacent escrow arbitration; it needs one order to flow end to end.
The roadmap even *contains the evidence* for this (headlines #1-2: zero integration, UI deleted, no order
ever) — it just can't draw the conclusion, because drawing it would mean dropping anchors, and dropping
anchors is the one thing its charter forbade.

---

## §3 — llama.cpp / GPU-gating investigation (dedicated, definitive verdict)

**The claim under test.** E13/E14 (self-host llama.cpp/vLLM) is deferred until "GPU-unlock," a single
trigger the roadmap concretely defines as `cargo add wgpu` (R2 O18: "GPU-unlock (network `cargo add
wgpu`) … | P17 video/wgpu; **P15 E13-execution**"; P05 §8; P15 §9). The operator's hypothesis: the
roadmap conflated "self-host an LLM" with "needs a GPU," and thereby deferred real, available-today
capability behind an unnecessary and factually-wrong trigger.

**What llama.cpp actually is.** llama.cpp exists *specifically* to run LLM inference on **CPU** using
quantized GGUF models. The canon itself half-knows this — it quotes llama.cpp as "LLM inference in C/C++,
GGUF self-host standard" (STRATEGIC-VECTORS:79) — yet still gates it on GPU. A GPU accelerates llama.cpp;
it is not required. vLLM (E14) genuinely benefits from GPU and is far less practical CPU-only — so E14's
GPU-preference is defensible; **E13's GPU-gate is not.**

**Live investigation on this host (2026-07-16):**

| Probe | Result | Meaning |
|---|---|---|
| `/usr/local/lib/ollama/llama-server` | **present**, ollama 0.30.9 installed | A llama.cpp inference server is **already on disk.** No build, no GPU crate. |
| `ls /dev/nvidia*` / `nvidia-smi` | **absent** | There is **no GPU** on this box. A literal "GPU-unlock" hardware trigger will never fire here. |
| `curl huggingface.co` | **200** | GGUF weights source is **reachable right now.** |
| `curl crates.io/api/v1/crates/wgpu` | **403** | `cargo add wgpu` genuinely **is** blocked. |

**The smoking gun.** The two capabilities the roadmap fused into one "GPU-unlock" trigger have
**demonstrably different current states**: `cargo add wgpu` → **403 (blocked)**, while llama.cpp is
**already installed** and its weights source (HuggingFace) → **200 (open)**. They are not one trigger.
`cargo add wgpu` succeeding or failing tells you *nothing* about whether you can run a local LLM, and vice
versa. Worse, R1-B already recorded that the consumption side is trivial ("Hermes has provider plumbing
… ollama support … OpenAI-compatible client … a config-not-code change when unlocked") — so the pieces to
stand up self-hosted CPU inference (server binary + reachable weights + existing OpenAI-compatible client)
are **all present today**, gated behind a trigger that (a) is about a *graphics* crate and (b) is about
*GPU hardware this machine doesn't have and doesn't need for llama.cpp.*

**Root cause.** The error entered at the canon (`ARCHITECTURE.md:34`, "managed-advisory until
GPU-unlock") and was **reinforced rather than challenged** at R1-B ("blocked on the compute/GPU cluster's
unlock (E21-25) — do not schedule before it"), then **welded into a single operator decision** at R2 O18
(which literally lists "P17 video/wgpu; P15 E13-execution" under one trigger), then propagated verbatim
through P05 §8 and P15 §9. Three independent conflations stacked: (1) "GPU" the loose word bundled a
graphics-rendering crate (wgpu) with LLM inference; (2) both were *offline-blocked*, so they got one
"network/GPU-unlock" label — but they're blocked by *different* network resources (crates.io vs
HuggingFace) with *different* current states; (3) the most **sovereignty-aligned** option (local CPU
llama.cpp — zero external dependency, the literal M6 ideal) was deferred *furthest*, behind the *least*
relevant trigger, while the *least* sovereign option (managed/hosted models via the headroom proxy) was
blessed as "correct-for-now." That inversion — deferring the sovereign option, keeping the dependent one —
directly contradicts the project's own stated values.

**VERDICT: this is a genuine mistake, and it is a "big deal" by the operator's threshold** (an action/claim
made without understanding something first). Not because the roadmap *builds* on a false premise — it
correctly ships E13 as blueprint-only and returns an honest `Err` offline — but because it **defers an
available, cheap, sovereign capability behind an unnecessary, mislabeled, and (for llama.cpp) factually
false trigger**, and no link in the 25-agent chain caught it.

**What should change:**
1. **Split O18 into two triggers.** (a) `graphics-unlock` = `cargo add wgpu` reachable → gates P17
   wgpu/video, F14 topology viz, E23 webgpu, splat rendering. (b) `model-weights-unlock` = a GGUF model
   fetched/vendored + a runnable llama.cpp server → gates E13 execution. These are independent; today (a)
   is **red** and (b) is **green-ish** (server present, HF reachable).
2. **Un-gate E13 (llama.cpp) from GPU entirely.** Re-scope it as CPU-first GGUF inference behind the
   existing Trait-as-Port / ollama-compatible client. Keep only a *small-model* budget note (CPU
   inference is slow; pick a small quantized model). Leave **E14 (vLLM)** GPU-preferred.
3. **Fix the canon line.** `ARCHITECTURE.md:34` "managed-advisory until GPU-unlock" is wrong for
   llama.cpp; it should read "managed-advisory until model-weights-unlock; llama.cpp CPU path needs no
   GPU." Fold this into BLUEPRINT-P02's canon-diff.
4. **Reconsider sequencing.** Because local CPU llama.cpp is the most M6-aligned inference option and is
   nearly available today, a minimal self-host spike could plausibly move *earlier* than Phase 15/17 — it
   does not belong on the GPU critical path at all.

---

## §4 — If this roadmap ships as-is, the single most important thing to fix first

**Before touching any of the 19 phases, split the roadmap's implicit "build all 147 locked anchors" charter
by inserting a real demand gate: make G11 (first real order, end-to-end on the mesh) an explicit
Wave-0 milestone whose result decides how many of the remaining 146 anchors are built now versus
DECART-deferred — and, as the concrete first instance of that discipline, un-gate llama.cpp (E13) from the
GPU trigger it never needed.** The arithmetic is sound and the blueprints are careful; the risk is not
sloppiness, it is building a quantum-mesh cathedral, honestly and completely, for a sandwich nobody has
ordered yet.
</content>
