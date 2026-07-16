# R1-B — Hub Autonomy, Agent-Infra & Living-Organism: Gap Analysis (2026-07-16)

> Cluster anchors: **M5, M7, M9, M11 · V1 · F1–F10 · E9 · E13–E20**
> Canon: `docs/design/ARCHITECTURE.md` (§0 M-series + SCOPE RULE, §6 F1–F10, §8 honest gaps)
> + `docs/design/STRATEGIC-VECTORS-LOCKED-2026-07-16.md` (§V1, E13-20 cluster line).
> Every CURRENT-STATE claim below is grounded in a direct code read (file:line) or is an
> honest **NOT BUILT**. Reused (not redone): `HK05-REALTIME-MODEL-ROUTING-INTEGRATION-2026-07-16.md`,
> `SYNTHESIZED-BLUEPRINT-PLAN-2026-07-16.md` §2 Cluster C (P0-C1),
> `BRAIN-TOPOLOGY-ORG-PSYCH-EMERGENCE-RESEARCH-2026-07-16.md`,
> memory `crypto-safe-first-pass-2026-07-14.md` (claims re-verified against code, per tasking).
> No web research was needed — every anchor grounded in-repo; the only candidate (MCP spec
> currency) is moot because the built MCP server is a minimal stdio JSON-RPC surface
> (handshake + tools/list + tools/call), not an SDK-version-coupled one.

Repos read: `/root/dowiz` (dowiz kernel + tools/telemetry), `/root/bebop-repo` (bebop2 mesh/PQ
stack + `crates/bebop`), `/root/hermes-agent-kernel-rewrite` (Hermes + `hermes-kernel` Rust crate).

---

## 1. Anchor-by-anchor: CURRENT → TARGET → GAP

### M5 — Every HUB = autonomous HYDRA

**CURRENT:**
- Protocol-only-inter-hub half is real in shape: `bebop2/mesh-node/src/node.rs:1-40` is an
  explicitly carrier-agnostic *port* ("core immutable, ports link, never import kernel"),
  generic over the `Transport` trait, enforcing a per-inbound-event DOD gate — the node never
  reaches into intra-hub internals. Inter-hub speech = `SignedFrame` + `HybridGate`
  (`proto-cap/src/hybrid_gate.rs`), capability-scoped, fail-closed.
- The affirmative "hub changes its OWN rules" substrate exists but is **narrow**:
  `bebop2/core/src/self_mod.rs:1-24` — self-modification effector **ACTIVATED (operator,
  2026-07-16)**, capability-scoped (fail-closed `Unauthorized`), floor-preserving (noether
  Σx² Lyapunov bound + snapshot-root non-regression + test-count non-drop), audited via
  immutable event log; red-lines (`push-to-main`, RLS, migrations, dep-install, `.claude/`
  edits) hard-refused as `EffectorReject::HumanGated` (`self_mod.rs:143`). Live driver
  `core/src/self_mod_loop.rs:1-16` routes every revision through the adversarial
  `deliberate()` mirror dialogue. **But it mutates exactly one parameter** (Kalman
  `q_scaler`) — not routing rules, not ports, not model choice.
- No "hub" abstraction exists as a configurable runtime entity: there is no per-hub policy
  object for ports/bridges/models/API/MCP/agents. `proto-wire/src/transport_policy.rs:17-30`
  is compile-time consts (`MAX_MESSAGE_BYTES`, `IDLE_TIMEOUT_SECS`) + a `TokenBucket`, not
  hot-swappable policy-as-data.
- SCOPE RULE (§0) is doc-law only — correctly so (it re-scopes dev-time gates), but the
  runtime side it promises ("hub may fork, self-gate locally, change its DB/model/API/port")
  currently means "edit source and recompile", not a hub-runtime capability.

**TARGET:** hub may change own rules / open ports & bridges / use any models/API/MCP/agents
at its discretion; protocol defines ONLY inter-hub comms (ARCHITECTURE §0 M5, M10).

**GAP:** (1) a `HubPolicy` runtime module — per-hub rules as data (ports, bridges, model
endpoints, access lists, `RedLinePolicy`, `HybridPolicy`), operator-editable, hot-reload;
(2) generalize `SelfModEffector` beyond one scalar to `HubPolicy` revisions (keeping the
deliberate-gate + floor-gate discipline); (3) dynamic port/bridge open-close machinery (see F2).

---

### M7 — No single point of failure; heal via Dijkstra/A* + Union-Find/MST

**CURRENT: NOT BUILT** (and the canon itself admits it — F45/F46 are marked "gap-fill (P1)").
- Grep across all of `bebop2/`: **zero** Dijkstra, A*, Union-Find, or MST implementations.
- Partial substrate that healing would compose with: peer discovery + revocation eviction
  (`proto-wire/src/discovery.rs:44-174`, `evict_revoked` at `:82`); `RevocationSet::merge`
  anti-entropy (`proto-cap/src/revocation.rs:14-16`); F15's HRW-merge is named but not
  implemented as a partition-merge routine.
- Nearest existing graph math is in the **dowiz kernel**, not the mesh: union-find used for
  connected components inside `dowiz/kernel/src/order_machine.rs:621`, plus the FSM
  graph-analysis suite (has_cycle/topo/reachable, arc closed 2026-07-14) — reusable math,
  wrong surface.
- "No leader election required" is vacuously satisfied: no election code exists anywhere.

**TARGET:** any node can drop; mesh heals via Dijkstra/A* + Union-Find/MST; no leader election.

**GAP:** the entire heal layer: multi-peer topology model in `mesh-node`, shortest-path
re-route on peer loss (Dijkstra/A* over the hub graph), partition detection (Union-Find over
the live-peer set), overlay spanning tree (MST) for gossip, HRW partition-merge (F15).
**Cross-cluster flag:** F45/F46 sit in the F-PRODUCT cluster and the mesh/transport substrate
cluster also plausibly owns Dijkstra/UF/MST (E32/E33 in its range). Merge step must assign
one owner; the *math* should land once (bebop2 core or shared) and be consumed by both.

---

### M9 — Kill-switch + flexible per-hub access ONLY

**CURRENT:** no named kill-switch exists. Grep for `kill-switch|kill_switch|killswitch`
across bebop2 `*.rs`/`*.md`: zero hits. What exists is the *substrate*:
- `proto-cap/src/revocation.rs:35-55` — `RevocationSet`: monotonic, irreversible; revoking a
  subject key "kills every capability ever minted to it, regardless of nonce/scope/expiry"
  (`revocation.rs:40`); surgical per-capability revocation via canonical-TLV hash; `merge`
  for anti-entropy. Wired into both transports and discovery
  (`proto-wire/src/{wss_transport.rs:259,357, iroh_transport.rs:82,116, discovery.rs:82}`).
- `proto-cap/src/node_id.rs:1-19` — MESH-12 operator-gated genesis: fail-closed anchor-file
  loader ("empty/absent list authorizes nothing"); `proto-cap/src/roster.rs:187-204` — trust
  anchors enrolled at genesis only, then frozen. The operator therefore already controls the
  trust root.
- `proto-cap/src/redline.rs:1-45` — capability-scoped red-line deny gate, default DENY,
  per-node `RedLinePolicy` allow-list = the seed of "flexible per-hub access".

**TARGET:** operator may hard-kill a hub/subtree; access controls per-hub configurable, not
global; **no other global control exists** (M9, F28: kill + COLD-backup, restartable).

**GAP:** (1) an operator **kill verb**: a signed protocol frame (anchor-rooted, per genesis
roster) whose handler in `mesh-node` performs COLD-backup-then-halt (F28 semantics); (2)
**subtree** kill semantics — currently undefinable because no hub hierarchy exists (blocked
on F10 sub-hubs; see ambiguity #3 below); (3) per-hub access **config surface**: load
`RedLinePolicy` + roster + revocations + `HybridPolicy` from an operator-editable file with
hot reload (today these are constructed in code). Note the inversion: M11 declares
kill-switch the *only* global control, yet today the only way to stop a runaway hub is
OS-level `systemctl kill` — the one mandated bound is the one not built.

---

### M11 — Living-organism experiment UNBOUNDED

**CURRENT:** the negative constraint (no governing layer above hubs except wire-format) is
trivially satisfied — there is no standing global service of any kind at runtime. Dev-time
gates exist and are heavy in bebop-repo (`.git/hooks/pre-commit`: doc-claims verification,
falsifiable-proof guardrail, logic-gate; `scripts/ci-*.sh` × 16 incl. kernel-fence,
crdt-fence, no-courier-scoring; `scripts/three-model-review.sh` commit gate) and light in
dowiz (`.github/workflows/ci.yml`: telemetry-selftest + eqc-proofs) — all correctly
re-scoped DEV-TIME-only by the SCOPE RULE (§0, HYDRA-CONTRADICTION sweep §8). The positive
machinery for an organism that *evolves* exists in embryo: activated `self_mod.rs` +
`self_mod_loop.rs` + `deliberate.rs` (above).

**TARGET:** no caps except kill-switch + configurable access; hubs freely evolve (F50).

**GAP:** M11 is *dependent*, not independent: it is satisfied exactly when (a) M9's
kill-switch exists (the single permitted bound — currently missing), (b) M5's self-evolution
machinery is general enough that there is something to be unbounded *about*, and (c) the two
guard-anchors inside F1-F10 that M11's own LOCK lines require are built: F10 max-depth-cap
on sub-hub recursion and F3/F27 sha3-verify-or-deny on runtime model ingestion. "Unbounded"
in canon is not "guardless" — each F-line carries its own LOCK-qualifier; those qualifiers
are the build list.

---

### V1 — Independence mechanism (A: ML-DSA split-identity, B: adversarial verifier)

**CURRENT: NOT BUILT as specified; raw material verified real.**

*A-side substrate (verified against code, not just the memory note):*
- ML-DSA-65 from scratch, zero external crates, FIPS 204 byte-exact, **ACVP KAT-verified**:
  `bebop2/core/src/pq_dsa.rs:1-16` + `core/src/pq_dsa/acvp_tests.rs`.
- Hybrid gate with **both legs REAL**: `proto-cap/src/hybrid_gate.rs:1-12` — Ed25519 leg via
  `bebop2-core::sign`, PQ leg via `signed_frame::{sign_pq,verify_pq}`; `RequireBoth` enforces
  real ML-DSA-65 verification, missing/invalid PQ ⇒ `HybridIncomplete`/`PqVerifyFailed`,
  never a fabricated pass. ⚠️ **Doc-drift hazard found:** `proto-cap/src/lib.rs:12-16` still
  says the ML-DSA leg "is a marked TODO" — stale; `hybrid_gate.rs` + `pq_dsa` tests are the
  ground truth. V1 implementation must not trust that stale docstring (fix it in passing).
- C-queue claims from memory **confirmed in code**: C6 `derive_pq_seed` domain separation
  (referenced in `pq_dsa.rs`, per `crypto-safe-first-pass` commit `15f0b24`), C7b canonical
  TLV (`revocation.rs:20-23` hashes `Capability::canonical_bytes_tlv`), C8 SHAKE256-J
  (`core/src/pq_kem.rs`), operator-cert (`wss_transport.rs:864-909` tests), C4 CT
  `scalar_mul` (`core/src/sign.rs`). Open follow-ups V1-A inherits: **C4b (HIGH)** —
  `sign.rs:612 mod_l` still variable-time on the secret nonce (biased-nonce → lattice key
  recovery class); **C3** — `pq_dsa::keygen(seed)` unconditionally `pub`, should be
  feature-gated. If key_K/key_V are hybrid (Ed25519⊕ML-DSA per E10), C4b is on the V1 path.
- Operator-gated genesis pattern exists exactly as V1 names it: `node_id.rs` MESH-12
  fail-closed loader.
- **Zero** split-identity code: grep `key_K|key_V|split_identity` across all three repos =
  no hits outside docs. Nothing signs git diffs/commits with any key.

*B-side fragments (three altitudes, none = the specified merge-blocking independent verifier):*
1. bebop-repo `scripts/three-model-review.sh` pre-commit gate — requires
   `.review/staged.json` with 3 distinct agent ids + non-empty findings (per memory process
   note). Process-level, bebop-only, and the *same session* orchestrates all three — no
   identity separation, no re-execution.
2. `bebop2/core/src/deliberate.rs:1-17` — author↔mirror adversarial dialogue, 2-lap cap,
   least-friction tiebreak; wired into `self_mod_loop.rs`. In-kernel decision gate, not CI.
3. `hermes-kernel/kernel/src/verification.rs:1-29` (HK-03) — `SessionState` FSM,
   `Complete` legal only from `Verified`, evidence-backed; dispatched as CLI op
   (`cli/src/main.rs:82,409`). **Self-context** — same agent claims and checks; its own
   header cites "~30 false-'done' corrections over 8 days". Plus
   `hermes-kernel/kernel/src/governance.rs:1-25` `FalseClaimMeter` (claimed-vs-verified
   audit) — the V5-B claim-latency statistic's sibling, computed but not fed by a per-commit
   latency log.
- dowiz CI (`.github/workflows/ci.yml`) has **no verifier job and no merge-block** of any kind
  tied to a verifier signature.

*Why V1 exists (reused from BRAIN-TOPOLOGY, not re-derived):* 97.8% single authorship —
"no independent second party" (line 41, 120); the 52-second GREEN on a 1,610-insertion diff
— "a claim-shaped artifact replaces a check-shaped artifact" (line 29); the suspension
recursion — "a document can't certify its own permission to remove the checks that would
verify it" (lines 912-914, 1066); and the direct pointer: "the ML-DSA infrastructure already
built for the delivery protocol is sitting unused for exactly this" (line 41).

**TARGET:** every diff signed by key_K; every verify/review signed by key_V; operator-gated
genesis; adversarial verifier re-executes cargo test / node-test / playwright in an
independent context, emits RED|GREEN + rationale; merge blocked without verifier signature;
identity-separation ≠ person-separation logged as enforced approximation.

**GAP (the whole pipeline):** (1) K/V keygen ceremony reusing `load_genesis` +
`derive_pq_seed`; (2) diff-signing tool — canonical bytes (commit sha3 + diff hash, C7b TLV
style) signed with key_K, artifact stored in-repo or in notes; (3) verifier runner — fresh
worktree/clone, re-execute the test suites, emit a key_V-signed RED|GREEN verdict with
rationale; (4) CI merge gate verifying both signatures, K≠V, verdict=GREEN; (5) claim-latency
logging per commit (V5-B) feeding `FalseClaimMeter`; (6) the standing "enforced
approximation" residue line in every verdict.

---

### F1–F10 — autonomy/hub-rules situations (each with its LOCK-qualifier)

| F | Situation | CURRENT (evidence) | GAP to LOCK-qualifier |
|---|---|---|---|
| **F1** | Hub changes own routing policy mid-flight | NOT BUILT — transport policy is compile-time consts (`transport_policy.rs:17-30`); no hot re-config | `HubPolicy` policy-as-data + hot reload (Phase B3) |
| **F2** | Hub opens new inbound port | PARTIAL — `TokenBucket` per-IP accept budget built+tested (`transport_policy.rs:30,189`); deny-by-default capability gate real (M12); **no dynamic listener spawn** | runtime listener open/close under HubPolicy, deny-by-default + TokenBucket applied at the new port |
| **F3** | Pull model from HuggingFace at runtime, sha3-verify-or-deny | NOT BUILT — `sha3_256` primitive + content-addressing pattern exist (`bebop2_core::hash`, KAT-green per `revocation.rs:18-19`); no model-artifact fetch/verify path | model ingestion pipeline: manifest {url, sha3} → download → verify-or-deny → capability-scoped load |
| **F4** | Hub spins own MCP server, proto-cap-sign | PARTIAL — real MCP server built: `crates/bebop/src/mcp.rs:1-40` (stdio JSON-RPC 2.0, handshake+tools/list+tools/call, DoS caps 1 MiB/64 KiB, `field_gate` + red-line vetoes intact `:30-31,143,166`) | the "proto-cap-sign" qualifier: MCP tool calls not bound to minted capabilities (see E18); no signed announcement of the MCP surface over the mesh |
| **F5** | Hub revokes another hub's trust | **BUILT at primitive level** — `RevocationSet` (surgical + blanket, monotonic, `revocation.rs:35-55`), `merge` anti-entropy, `discovery.evict_revoked` (`discovery.rs:82`), wired into WSS+iroh transports | the gossip loop itself ("a real mesh would gossip this set" — `revocation.rs:13-15` names it, nothing runs it) |
| **F6** | Paid 3rd-party API + TokenBucket | PARTIAL — TokenBucket built (transport tier); `Budget` + `Recalibrator` exist in `hermes-kernel/kernel/src/control.rs` (per HK05 table); EnvFile discipline documented (S3), real `.env` on disk | per-agent **spend** bucket wired into the live routing path (gov_route reject on budget exhaustion) |
| **F7** | Hub switches to no-consensus | **Vacuously possible** — no runtime consensus engine exists to opt out of; `proto-cap/tests/mesh_consensus.rs` is a spectral-consensus *parity simulation* (`:10,413`), not a runtime rule | nothing to build for F7 itself; flag: anchor is vacuous until any consensus rule exists (M-series requires none — record and move on) |
| **F8** | Bridge to non-PQ legacy node, flag-as-insecure | PARTIAL — classical fallback exists: `HybridPolicy::ClassicalUntilPqAudit` (`hybrid_gate.rs:31-34`, records PQ-pending, classical leg still real) | the **flag**: surface "insecure bridge active" in local telemetry (M8-compliant, local-only) |
| **F9** | Auto-update own kernel from git, eqc-gate-or-deny | NOT BUILT — deliberately: `self_mod.rs` hard-refuses dep-install/push as `HumanGated`; eqc exists and runs in CI (`dowiz/tools/eqc/`, ci.yml `eqc-proofs` job) | self-update path: fetch → build → run eqc + test floor → apply-or-deny, inside the floor-preserving effector discipline |
| **F10** | Sub-agent opens sub-hub, max-depth-cap | NOT BUILT — no sub-hub concept, no depth cap anywhere (grep negative incl. `mcp.rs`); Hermes spawns sub-agents (Python) with no hub semantics | sub-hub spawn primitive carrying `depth` in its capability; refuse at `depth > cap`; **cap value unspecified in canon** (ambiguity #6) |

---

### E9 — agent = Hermes-tool + verifier

**CURRENT:** substantially built as dev-tooling, split across two repos:
- Hermes tool: `/root/hermes-agent-kernel-rewrite/` (Python agent framework) +
  `hermes-kernel` Rust crate; dowiz side is a thin bash bridge —
  `tools/telemetry/governance.sh:2-32` ("thin I/O + dispatch shim over the native
  hermes-kernel binary", `gov_kern` pipes JSON to `KERNEL_BIN`). Dispatch table
  (`governance.sh:340-364`): record/route/lane/research/hard/judge/gate/precedent/meta/
  falseclaim/learn/anu/ananke/decide.
- Verifier half: HK-03 `verification.rs` FSM + `governance.rs` FalseClaimMeter/`ananke_check`
  /`decide` (Anu proposes, Ananke constrains — `governance.rs:1-17`), all dispatchable
  (`cli/src/main.rs:394-413`).

**TARGET:** the agent role is "Hermes tool + verifier" (E9-A).

**GAP:** (1) the verifier is *self-context* — E9's verifier half becomes real only through
V1-B (independent context + signature); (2) session-close `verification_gate` is not invoked
from the live governance.sh path; (3) the routing wiring gap below (E15) is E9's other half.

---

### E13/E14 — self-host llama.cpp/vLLM GOAL; managed-advisory until GPU-unlock

**CURRENT:** NOT BUILT in dowiz/bebop (zero llama.cpp/vLLM references outside the Hermes
repo). Hermes has provider plumbing that makes this a config-not-code change when unlocked:
`plugins/model-providers/custom/`, ollama support (`tests/test_ollama_num_ctx.py`),
OpenAI-compatible client (`run_agent.py`) — a llama.cpp/vLLM OpenAI-compatible endpoint
plugs into the existing provider profile mechanism. Managed-advisory is today's reality
(headroom proxy + hosted models, per memory). GPU-unlock is pending (W21: wgpu uncached,
trigger = network cargo-add; ARCHITECTURE §8 "GPU-unlock pending network").

**TARGET:** self-host llama.cpp (120k★ MIT) + vLLM (86k★ Apache-2.0) as GOAL; managed
stays advisory until GPU-unlock; Modal H100 $0.001097/s scale-to-zero for burst (E22,
web-verified in canon).

**GAP:** a deployment blueprint, gated: llama.cpp GGUF service behind a provider profile /
Trait-as-Port, sha3-verified weights (composes with F3), budget ceiling + TokenBucket
(F6/F33), Modal adapter for burst. **Cross-cluster dependency:** blocked on the compute/GPU
cluster's unlock (E21-25) — do not schedule before it.

### E15 — harmonic+kelly adaptive tiering

**CURRENT: compute 100% BUILT + TESTED; live wiring 0%.** (HK05 audit re-verified here, with
one correction: the CLI file needs `grep -a` — it contains a non-UTF8 byte that silently
blanks normal grep; the ops ARE there.)
- `hermes-kernel/kernel/src/routing.rs`: `TaskFeatures`:33, `Complexity`:43,
  `classify_complexity`:67, `rank_models_for_bucket`:114 — literally calls
  `harmonic_centrality` (`:26,156`) over a per-bucket success-rate graph.
- `hermes-kernel/kernel/src/control.rs`: `ev`:137, `kelly_fraction`:149, `ruin_prob`:158,
  `lane_size`:182, `pid_parallelism`:191, `ev_route_select`:224, `jury_aggregate`:259.
- CLI dispatch: `cli/src/main.rs:394-413` — `classify_complexity`, `rank_models`,
  `gov_route`, `gov_lane` all live ops; `op_classify_complexity`:199, `op_rank_models`:220,
  `op_gov_route`:545, `op_gov_lane`:572.
- dowiz mirror: `kernel/src/harmonic.rs:26` `harmonic_centrality` (ported from hermes-kernel
  per `:3-4` "HK-05/HK-06"), 10+ tests, wasm-wired.
- The live path: `governance.sh:43-73 gov_route()` folds `track_record.jsonl` by `task` only
  and calls only `gov_route`; `gov_lane_width` (`:178`) takes manual args, never fed by
  `lib.sh::resource_sample()` telemetry. `classify_complexity`/`rank_models` are **never
  called from any live code**.

**TARGET:** harmonic_centrality + kelly_fraction adaptive tiering, wired (ARCHITECTURE §1
E15 "wire П0-C1").

**GAP:** exactly **P0-C1** (SYNTHESIZED-BLUEPRINT §2 Cluster C — files-touched and
acceptance criteria already speced there; reuse verbatim): three new calls in governance.sh,
`bucket` column in track_record.jsonl (missing ⇒ `Simple`, backward-compatible), lane width
fed by live arrival-rate/service-time. RED→GREEN: same task classified `Complex` must route
differently than `Simple`.

### E16 — spectral+BD memory  ⚠️ cross-reference, not absorbed

**CURRENT (documented for the merge step, ownership shared with the storage/knowledge
cluster):** `dowiz/kernel/src/spectral.rs` (eigensolve), `living_knowledge.rs:1-13`
(retrieval ADAPTER, trait + JSON-over-stdio bridge, fail-closed, swappable ONNX spike via
`LK_BRIDGE_CMD`; W18 made it PRIMARY recall, recall@5=1.0 per memory), `spine.rs:1-20`
(W2-7 hash-chain knowledge spine, tamper-evident, Memory/Identity/Intent kinds),
`hermes-kernel/kernel/src/memory_rank.rs` (+ CLI op `memory_rank`:159,411).
**It IS load-bearing for this cluster:** rank_models/track-record folding and per-hub
graph-wiki replication (F48) both ride on this memory substrate. **Flag:** "BD" is never
expanded anywhere in canon (only ever "BD+spectral+history", E8/E16) — ambiguity #2. Do not
double-build: merge step should assign E16 primary ownership to the storage/knowledge lane
and mark this cluster a consumer.

### E17 — MCP

**CURRENT: BUILT (core).** `crates/bebop/src/mcp.rs` — minimal MCP over stdio (JSON-RPC 2.0,
handshake + tools/list + tools/call), native tools call the same Rust engines as the CLI,
DoS-hardened after a prior fable audit (`MAX_TOOL_ARG_BYTES` 1 MiB `:37`,
`MAX_ARG_STR_BYTES` 64 KiB `:41`), `field_gate` vetoes + red-line checks preserved
(`:30-31,325,409`); `BEBOP_MCP_ONCE` test mode. Hermes side: `mcp_serve.py`.
**GAP:** F4's qualifier — capability-token binding (E18) and a signed mesh announcement of
a hub's MCP surface. Otherwise done.

### E18 — per-agent capability-tokens

**CURRENT:** the capability system is BUILT for **mesh frames**, not for **agents**:
`proto-cap/src/capability.rs:48-52,82-93` — `{subject_key, scope, nonce, expiry}` +
optional `subject_key_pq` (ML-DSA-65, 1952 B) hybrid identity; closed `Resource`/`Action`
enums (`scope.rs:1-30`); anchor-rooted delegation chains with narrow-only attenuation
(`hybrid_gate.rs:41-48`); red-line deny (`redline.rs`); fail-closed everywhere. Meanwhile
agent sessions (Hermes tools, MCP calls, subagents) carry **no** minted capabilities —
`self_mod.rs:29-33` even defines a LOCAL stand-in enum "because core cannot depend on
proto-cap", explicitly naming `Resource::Corpus/Action::Append` as the upstream mapping
when driven from the wire layer.
**GAP:** mint/verify per-agent capabilities at the agent boundary: map MCP tools and Hermes
tool classes → `Resource`/`Action` scopes; sub-agent spawn mints an attenuated child
capability (composes with F10 depth).

### E19 — TokenBucket

**CURRENT: BUILT** — `proto-wire/src/transport_policy.rs:30` `TokenBucket` (per-IP
pre-accept budget, pure accounting primitive, tested `:189`). Not applied to agent/API
spend (F6) or GPU budget (F33).
**GAP:** reuse (not rebuild) at two more tiers: per-agent LLM/API spend in the gov_route
path; GPU-job budget when E13/Modal lands.

### E20 — paired-debate

**CURRENT: BUILT twice at primitive level, partially live:**
- `bebop2/core/src/deliberate.rs:1-17` — author↔mirror adversarial dialogue, hard 2-lap cap,
  least-friction auto-adoption; **live** in `self_mod_loop.rs` (every self-mod revision must
  pass the mirror).
- `hermes-kernel/kernel/src/control.rs:259` `jury_aggregate` (3-vote); governance.sh has
  `judge`/`gate`/`research` verbs — but `gov_research` (`:75-80`) only logs "dispatched" to
  precedent JSONL + Telegram; the argue loop itself runs agent-side, unenforced.
**GAP:** enforce paired-debate at the routing tier: Complex-bucket (post-P0-C1) decisions
route through deliberate()/jury before dispatch; verdicts recorded in the precedent store.

---

## 2. Ambiguities / underspecification in canon (flagged, not papered over)

1. **E13–E20 per-item numbering is not in the repo.** STRATEGIC-VECTORS gives only the
   cluster line ("full Descartes per item in session dialog" — that dialog is not on disk).
   The mapping used here (E13/E14 = LLM-infra self-host+managed-advisory per ARCHITECTURE §1
   "LLM infra (E13/E14)"; E15 tiering per "Models (E15)"; E16 memory; E17 MCP; E18 tokens;
   E19 TokenBucket; E20 debate) is inferred and should be ratified at merge.
2. **"BD" in "spectral+BD memory" (E8/E16) is never expanded** anywhere in ARCHITECTURE,
   STRATEGIC-VECTORS, or the arc notes read. Needs one authoritative expansion.
3. **M9 "subtree" kill is undefined** until F10 sub-hub hierarchy exists — the canon uses
   hub/subtree language with no tree structure specified.
4. **Hub vs node vs edge boundary** — M4 (edges autonomous) vs M5 (hubs Hydra) have no code
   counterpart distinction; only `mesh-node` exists. Target text implies a role distinction
   the canon never operationalizes.
5. **V1-B "independent context" is not pinned** — fresh worktree on the same host? separate
   machine? different model family? The escape clause covers identity≠person, but the
   minimum context-isolation bar needs one sentence of canon.
6. **F10 max-depth-cap value unspecified** (any concrete default needs an operator lock).
7. **F7 is vacuous today** — there is no consensus rule to defect from; recorded as
   satisfied-by-absence, revisit only if a consensus engine ever ships.
8. **Stale docstring:** `proto-cap/src/lib.rs:12-16` contradicts `hybrid_gate.rs` (PQ leg
   "TODO" vs "REAL") — fix in the first phase that touches proto-cap.

---

## 3. Build phases (ordered, zero exceptions)

> **Cross-cluster dependency (explicit, for the merge step):** Phase B1 *consumes* the
> mesh/PQ-crypto substrate cluster's M2/M4/M12 identity primitives — verified here as
> already built (pq_dsa ACVP, hybrid gate, canonical TLV, genesis loader), so B1 is NOT
> blocked, but any substrate-cluster refactor of `pq_dsa`/`signed_frame`/`node_id` must land
> first or be coordinated. Phase B3's kill verb needs a new wire-format frame kind — the
> substrate cluster owns the wire format (M6/M10); hub-side handler is ours. Phase B4's M7
> heal math and E13 self-host are gated on, respectively, the substrate cluster
> (Dijkstra/UF/MST ownership, E32/E33) and the compute/GPU cluster (GPU/network unlock,
> E21-25). E16 memory is consumed, owned elsewhere (storage/knowledge lane).

### Phase B1 — Split-identity + adversarial verifier (V1 A+B minimal loop)
- **Anchors:** V1 (whole), E9 (verifier half), feeds V5-B/V5-C.
- **Why first:** per BRAIN-TOPOLOGY, every other phase's "GREEN" is self-certified until an
  independent verifier exists — this is the epistemic bedrock for the remaining 146 anchors;
  and it is the cheapest phase (pure reuse of a finished, ACVP-verified PQ stack).
- **Dependencies:** PQ substrate (done, verified). Nothing else.
- **Scope:** (1) K/V keygen ceremony — operator-gated genesis file listing key_K/key_V
  anchors, `load_genesis` pattern + `derive_pq_seed` domain separation (fix C3 gating and
  schedule C4b as part of key hygiene); (2) diff-signer — canonical TLV over
  (commit-sha3, diff-sha3) signed with key_K; (3) verifier runner — fresh worktree,
  re-execute cargo test / node test / playwright, emit key_V-signed RED|GREEN + rationale +
  the standing "enforced approximation: identity ≠ person" residue line; (4) CI merge gate:
  both signatures present, K≠V, GREEN required on red-line paths (V5-C), claim-latency
  logged per commit into `FalseClaimMeter` input; (5) fix stale `proto-cap/src/lib.rs`
  docstring.
- **Falsifiable done-test:** a diff with a failing test cannot obtain a key_V GREEN
  (verifier re-execution goes RED); a GREEN verdict signed with key_K is rejected by the
  gate; a planted PR without a verifier signature cannot merge. All three REDs demonstrated,
  then GREEN on a clean diff.

### Phase B2 — Wire the routing organism (P0-C1 + budgets + debate + session gate)
- **Anchors:** E9 (tool half), E13/E14 (managed-advisory formalized as current tier), E15,
  E19, E20, F6.
- **Dependencies:** none — all compute already built (routing.rs/control.rs/CLI verified);
  zero file overlap with B1; can run in parallel with B1.
- **Scope:** exactly P0-C1 (reuse SYNTHESIZED-BLUEPRINT §2-C spec verbatim: three calls in
  `governance.sh`, `bucket` column in track_record.jsonl, lane width fed by
  `resource_sample()`); plus: per-agent spend budget (TokenBucket/`Budget`) enforced in the
  gov_route reject path (F6); `verification_gate` (HK-03) invoked at session close from
  governance.sh; Complex-bucket routes pass paired-debate (deliberate()/jury) before
  dispatch, verdict → precedent store (E20).
- **Falsifiable done-test:** HK05's own RED→GREEN — the same task classified `Complex`
  receives a different (wider/costlier) route than classified `Simple`; lane width changes
  when injected arrival-rate telemetry changes; an over-budget call is refused; a session
  that edited code with failing evidence cannot emit "complete".

### Phase B3 — Hub runtime: policy-as-data, kill-switch, per-hub access
- **Anchors:** M5 (core), M9 (whole), M11 (its single bound), F1, F2, F5 (gossip
  completion), F8 (flag).
- **Dependencies:** substrate cluster for the new wire-format frame kind (kill verb) —
  coordinate; B1 for operator-signed frames (kill order must be anchor-rooted, reusing the
  same genesis roster). Hub-side entirely ours.
- **Scope:** `HubPolicy` module — per-hub config (ports, bridges, model endpoints, access
  allow-lists, `RedLinePolicy`, `HybridPolicy`) from an operator-editable file, hot-reload
  (F1); dynamic listener open/close, deny-by-default + TokenBucket on every new port (F2);
  operator kill verb: anchor-signed frame → `mesh-node` handler does COLD-backup-then-halt,
  restartable (F28/M9); revocation gossip loop over the live transport (F5's missing half);
  local-telemetry "insecure bridge" flag whenever `ClassicalUntilPqAudit` is active (F8).
- **Falsifiable done-test:** a kill frame signed by a genesis anchor halts a running node
  only after a COLD snapshot exists (RED: unsigned/non-anchor kill frame is ignored; RED:
  kill without completed backup refuses to halt-lose-data); a HubPolicy edit changes the
  accepted-port set without restart; a revoked peer's frames are dropped by a second node
  after one gossip round.

### Phase B4 — Living organism unbounded: general self-mod, sub-hubs, model autonomy
- **Anchors:** M11 (whole), M5 (completion), M7, F3, F4 (sign qualifier), F7 (record as
  vacuous), F9, F10, E13 (execution, gated), E17 (completion), E18.
- **Dependencies:** **B3 is a hard gate** — M11's unbounded experiment is only permissible
  once its single mandated bound (M9 kill-switch) exists. M7 heal math: coordinate with
  substrate cluster (single implementation, two consumers). E13 execution: gated on
  compute/GPU cluster unlock; the blueprint ships in this phase, the deployment when
  unlocked.
- **Scope:** generalize `SelfModEffector` from one scalar to `HubPolicy` revisions (same
  deliberate-gate + floor-gate + audit discipline, red-lines stay `HumanGated`); sub-hub
  spawn primitive with capability-carried depth + max-depth-cap refusal (F10; cap value =
  operator lock needed, ambiguity #6); sha3-verify-or-deny model ingestion (F3/F27);
  capability-token binding of MCP tools + per-agent minting with attenuation on sub-agent
  spawn (E17/E18, closes F4's qualifier); eqc-gate-or-deny self-update path (F9, reusing
  `tools/eqc` + the effector's floor gate); mesh heal wiring (M7): consume
  Dijkstra/A*/Union-Find/MST to re-route on peer drop and detect/merge partitions; E13
  llama.cpp/vLLM provider profile behind a port with sha3-verified weights + budget ceiling.
- **Falsifiable done-test:** a floor-violating HubPolicy self-revision is refused and
  logged REJECTED (RED) while a mirror-agreed, floor-clean one applies (GREEN); sub-hub
  spawn at depth > cap is refused; a model blob with a wrong sha3 is refused at load; a
  kernel self-update with a failing eqc proof is denied; killing one of three meshed nodes
  leaves the other two exchanging frames over a recomputed route within the heal budget.

---

## 4. Critical-gap summary (most severe first)

1. **V1 = 0% built** while being the epistemic precondition for trusting every other
   phase's GREEN — and its entire substrate is finished and idle (ACVP-verified ML-DSA,
   hybrid gate, genesis loader). Cheapest highest-leverage phase in the cluster.
2. **M9 kill-switch does not exist** — the architecture's single permitted global control,
   and the gate for the M11 unbounded experiment, is currently OS-level only.
3. **M7 heal = 0%** (canon admits it); no Dijkstra/A*/Union-Find/MST anywhere in bebop2.
4. **E15/E9 routing: 100% compute, 0% wiring** (P0-C1) — one bash file away, fully speced.
5. **M5 hub-autonomy machinery is one scalar wide** — activated self-mod exists but only
   mutates a Kalman q_scaler; no HubPolicy, no dynamic ports, no rule generality.
6. **Agent surface carries no capabilities** (E18) — the mature proto-cap system stops at
   the wire; MCP/tools/subagents run outside it.
7. Canon underspecification: E13-E20 per-item text absent from repo; "BD" unexpanded;
   subtree-kill undefined pre-F10; V1-B context-isolation bar unpinned (see §2).
