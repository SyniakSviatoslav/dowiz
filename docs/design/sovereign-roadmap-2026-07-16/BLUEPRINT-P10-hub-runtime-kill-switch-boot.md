# BLUEPRINT — Phase 10: HUB RUNTIME — POLICY-AS-DATA, KILL-SWITCH, BOOT

> Master roadmap: `R2-MERGED-PHASE-ROADMAP.md`. This is 1 of 19 phases.
> **Anchors:** M5, M9, E37, F1, F2, F5, F8, F28.
> **Depends on:** Phase 3 (PQ trust-root — genesis anchors carry ML-DSA, so a kill frame is
> quantum-unforgeable), Phase 6 (V1 signing ceremony — the kill-switch needs a real operator
> key ceremony to authenticate the operator), Phase 9 (wire format — the kill frame is a new
> frame KIND on that wire; the revocation gossip transport lives there).
> **Parallel-safe with:** Phase 11, Phase 12.
> **Critical path:** P3 → P9 → **P10** → P13. This phase is the gate for Phase 15 (M11
> unbounded organism is only permissible once its single mandated bound — the M9 kill-switch —
> exists).
> Sources: `R1-B §B3` (hub-autonomy), `R1-A §Phase-C` (kill-switch cross-reference),
> `ARCHITECTURE.md` §0 (M5/M9/M11 + SCOPE RULE), §6 (F1/F2/F5/F8/F28). Current-state file:line
> claims below were re-verified against `/root/bebop-repo` at authoring time.

---

## 1. Current-state evidence

### 1.1 HEADLINE FINDING — the one mandated global control is the one thing not built, and a same-named legacy construct implements its exact opposite

M9 is unambiguous: *"Kill-switch + flexible access ONLY: operator may hard-kill a hub/subtree
… No other global control exists. LOCK."* M11 restates it: the kill-switch is the **only**
governing layer above hubs. It is the single global control the entire mesh-foundation
architecture permits — and it does not exist.

- **No named kill-switch anywhere in the mesh substrate.** `grep -i "kill.switch|kill_switch|
  killswitch"` across `bebop2/proto-wire` and `bebop2/mesh-node` returns zero hits (R1-A §F28,
  R1-B §M9). The only mechanism that actually stops a runaway hub today is OS-level
  `systemctl kill` — a blunt, non-protocol-aware SIGKILL that (a) is not authenticated by the
  mesh trust root, (b) cannot be issued *over the mesh* to a remote hub, and (c) halts-then-
  loses-data (no COLD snapshot). The one bound the architecture mandates is delegated to the
  operating system, outside the protocol entirely.

- **A legacy `KillSwitch` exists — and it is the OPPOSITE of M9.** `crates/bebop/src/guard.rs:64-124`
  defines `struct KillSwitch`. Verified: it is a **≥2/3 consensus vote registry** —
  `recompute()` computes `threshold = known_nodes.len() * (2.0/3.0)` and suspends a target
  only when that many *distinct nodes* vote to suspend it. Its own doc-comment states the
  design intent: *"no single node can kill another (no central off-button)."* This is a
  governance/quorum mechanism that happens to share the word "kill." M9 wants **unilateral
  operator authority**; this construct **structurally forbids** unilateral authority. A
  consensus-gated kill-switch is not a kill-switch — it is a voting system. It must be
  **deprecated / removed from the kill path**, not extended.

### 1.2 No `HubPolicy` runtime entity — transport policy is compile-time constants

M5 mandates every hub be an autonomous Hydra that *"may change OWN rules, open ports/bridges,
use any models/API/MCP/agents at its discretion."* F1 makes this concrete: *"Hub changes its
OWN routing policy mid-flight."* Neither is buildable as currently shaped:

- `bebop2/proto-wire/src/transport_policy.rs` holds the transport rules as `pub const`
  (`MAX_MESSAGE_BYTES`, `IDLE_TIMEOUT_SECS`) plus a `TokenBucket` accounting primitive
  (verified :17-45). There is **no policy-as-data object**: which ports are open, which bridges
  exist, which models are reachable, the `RedLinePolicy`, the `HybridPolicy` — all are either
  compile-time constants or constructed inline in code. "Change a rule mid-flight" today means
  *edit source and recompile* (R1-B §M5). The SCOPE RULE promises runtime self-gating; the code
  offers none.
- `bebop2/proto-cap/src/redline.rs:1-45` (`RedLinePolicy`, default-DENY allow-list) and
  `bebop2/proto-cap/src/hybrid_gate.rs:24-34` (`HybridPolicy::{RequireBoth, ClassicalUntilPqAudit}`)
  are the *seeds* of per-hub access data — but they are constructed programmatically, not loaded
  from an operator-editable, hot-reloadable source.

### 1.3 Genesis loader built but not wired into boot; no ceremony; example file missing (E37)

`bebop2/proto-cap/src/node_id.rs:80-180` is an **exemplary, fail-closed** genesis loader
("empty/absent list authorizes nothing"; `RootDelegationPolicy::Unspecified` refused via
`require_explicit_policy`), and `roster.rs:187-204` freezes anchors after genesis. But
(R1-A §E37, verified): `grep load_genesis mesh-node/ = 0` — **the loader is never called at
boot**; `config/genesis.example.txt` **does not exist**; there is **no documented key ceremony**.
`MeshNode` starts without ever establishing its trust root.

### 1.4 Revocation decision path absent; no dynamic listeners; no insecure-bridge flag

- **F5 (revoke another hub's trust):** the primitive is BUILT — `proto-cap/src/revocation.rs:35-55`
  (`RevocationSet`: monotonic, irreversible, surgical-or-blanket, canonical-TLV) wired into
  `discovery.rs:82` (`evict_revoked`). What is missing is (a) the **decision/policy half** — the
  operator verb or policy rule that *decides* to revoke and mutates the set — and (b) the
  **gossip loop** that propagates it (`revocation.rs:13-15` names it, nothing runs it). Per the
  master roadmap seam: the gossip **transport** is Phase 9's job (F23/E38); **this phase owns the
  trust-revocation decision/UI**, which produces the delta Phase 9 gossips.
- **F2 (open a new inbound port):** PARTIAL — the per-IP accept `TokenBucket` is built and tested,
  but there is **no dynamic listener spawn** — a hub cannot actually open a new port at runtime
  (R1-B §F2).
- **F8 (bridge to a non-PQ legacy node):** `HybridPolicy::ClassicalUntilPqAudit` records that PQ
  is pending, but there is **no flag surfaced** — an operator cannot see that a hub is running a
  classical-only bridge. It is currently a silent security downgrade (R1-B §F8).

---

## 2. HubPolicy-as-data design (M5, F1)

The concrete mechanism that makes F1 real. `HubPolicy` is the **single operator-editable runtime
entity** holding every rule that is today a compile-time constant. It is *data*, loaded at boot
and hot-reloadable, not code.

### 2.1 Fields

```
HubPolicy {
  revision:        u64,                 // monotonic; bumped on every apply
  policy_sha3:     [u8;32],             // canonical-TLV hash of this revision, for audit
  listeners:       Vec<ListenerSpec>,   // F2 — bind addr, transport kind, enabled, accept_bucket
  bridges:         Vec<BridgeSpec>,     // F8 — peer endpoint, per-bridge HybridPolicy
  model_endpoints: Vec<ModelEndpoint>,  // M5 — {url, sha3} manifest ref (INGESTION is Phase 15)
  access_lists:    AccessLists,         // M9 — allow/deny per capability scope (per-hub, not global)
  red_line_policy: RedLinePolicy,       // M12 — reuse proto-cap/redline.rs default-DENY allow-list
  rate_limits:     RateLimitConfig,     // F2 — new-listener bucket + per-IP accept bucket sizes
}
```

`HybridPolicy` is **per-bridge** (a field of `BridgeSpec`), not global — a hub may run one
`RequireBoth` bridge and one `ClassicalUntilPqAudit` bridge simultaneously; §5.2 makes the
latter observable. The compile-time constants in `transport_policy.rs` become the **defaults** a
`HubPolicy` may override, not the ceiling.

### 2.2 Source of truth + hot-reload mechanism

- **File-backed, deny-by-default.** Loaded at boot from `config/hub-policy.txt` (operator-editable,
  gitignored). Absent ⇒ `HubPolicy::deny_all_default()` (no listeners, no bridges, red-line
  default-DENY). Never fails open.
- **Hot-reload path.** Two triggers, one apply function:
  1. *File edit* — a file-watch (inotify) on `hub-policy.txt` fires on change.
  2. *Signed `PolicyUpdate` frame* — an operator-anchor-signed frame carrying a new revision
     (same signature path as the kill frame, §3), for changing a remote hub's policy over the mesh.
- Both funnel into `apply_revision(candidate: HubPolicy)`:
  1. **Parse + validate.** Malformed ⇒ reject, keep last-good revision (fail-closed; a bad edit
     never takes a hub down).
  2. **Floor-gate.** A revision may **never widen a red-line scope** (Auth / Money / Secrets /
     Migrations, per M12). Any candidate that does is REFUSED and logged `REJECTED`; red-lines stay
     `HumanGated`. This is the invariant Phase 15 inherits when it lets a hub *self-author*
     revisions — the pipeline, floor-gate, and audit are built here; Phase 15 only adds the
     self-mod author.
  3. **Atomic swap.** Bump `revision`, recompute `policy_sha3`, atomically swap the in-memory
     `Arc<HubPolicy>` (RCU-style — readers hold a snapshot, no lock on the hot path; a frame in
     flight completes against the snapshot it began with). **No process restart.**
  4. **Reconcile side-effects.** Diff old vs new: open newly-present/enabled listeners, drain +
     close removed ones (§5.1); re-scan bridges to set/clear the insecure-bridge flag (§5.2); emit
     a typed `PolicyRevision{revision, sha3}` telemetry event into Phase 8's local sink.

---

## 3. Kill-switch design (M9, F28)

The operator kill verb, implemented as an **anchor-signed frame**, replacing the consensus
`guard.rs::KillSwitch`.

### 3.1 Frame format — coordinate with Phase 9's wire frame-kind registry

Phase 9 owns the wire frame-kind registry (`enum FrameKind` on the wire, M6/M10). **This phase
requests exactly one new discriminant: `FrameKind::OperatorKill`.** The kill order rides inside
the existing `proto-cap::SignedFrame` envelope (already carries the hybrid Ed25519⊕ML-DSA
signature legs), so no new crypto is invented here — only a new payload kind and a hub-side
handler.

```
KillOrder {                       // canonical-TLV payload of an OperatorKill frame
  target_hub_id:        NodeId,   // which hub (or subtree root) to halt
  nonce:                [u8;32],  // replay protection (reuse hybrid_gate nonce ledger, F25)
  issued_at:            u64,      // unix seconds
  expiry:               u64,      // reject if now > expiry
  require_cold_backup:  bool,     // MUST be true in the M9 profile; false is a config error
  reason:               Option<String>,
}
```

**Verification path (fail-closed, deny-by-default):**
1. Frame arrives on `OperatorKill` kind → hybrid-verify (Ed25519 **and** ML-DSA-65) against the
   **genesis roster** loaded at boot (Phase 3's *hybrid* anchors — a classically-forged kill is
   rejected under a quantum adversary).
2. The signer MUST be an enrolled **genesis anchor carrying the `kill` capability scope** (§4.3).
   A valid signature from a non-anchor key, or an anchor lacking `kill` scope, is **dropped** —
   the node keeps running. (RED cases below.)
3. Nonce not previously seen (replay ledger) and `now ≤ expiry`, else drop.
4. `target_hub_id` matches this hub (or a descendant, once F10 subtrees exist — subtree-kill
   semantics are deferred to Phase 15 per O13; this phase implements the single-hub case and
   leaves the target-match hook).

The kill key is a **genesis/operator anchor**, established by Phase 6's ceremony machinery
(`load_genesis` + `derive_pq_seed`) — **not `key_K`/`key_V`** (Phase 6's diff-signer / verifier
keys). Same ceremony, distinct role and scope. §4.4 makes this explicit so the two are never
conflated.

### 3.2 COLD-backup-THEN-halt sequencing — never halt-then-lose-data

The handler is a small state machine. **The ordering invariant is absolute: a confirmed COLD
snapshot must exist before any halt, on every accepted path.**

```
Running ──(verified kill)──▶ Killing ──(snapshot confirmed)──▶ Halted (restartable)
                               │
                               └──(snapshot failed/absent)──▶ Killing-Blocked (still running, alert)
```

1. **Running → Killing.** On a *verified* kill: stop accepting new work (deny listeners, drain
   in-flight frames) but **do not exit**.
2. **COLD snapshot.** Invoke Phase 12's COLD zstd archiver + restore-verify (E27/F38). Block on a
   **snapshot-confirmed receipt** — `integrity_check=ok` and a byte-identical restore-verify. The
   snapshot captures the event-log tip so the hub is fully restorable.
3. **Confirmed → Halted.** Flush the event-log tip, close listeners, graceful shutdown / process
   exit. State on disk ⇒ the hub is **restartable** afterward (kill is not destroy).
4. **Failed/absent → Killing-Blocked.** If the snapshot fails or times out, the handler
   **REFUSES to halt** — it reverts toward a safe running/degraded state and raises a telemetry
   alert. Losing data is never preferable to staying up. (This is the F28 `LOCK + COLD-backup`
   qualifier made mechanical.)

The handler lives in `mesh-node`. The frame **encoding + registry** are Phase 9's; the
**signature verification** reuses `proto-cap` `hybrid_gate` + `roster`; the **archiver** is
Phase 12's. This phase owns only the handler + sequencing + the `OperatorKill` kind request.

---

## 4. Boot / genesis-wiring design (E37)

### 4.1 `MeshNode` boot sequence (fail-closed, ordered)

```
boot():
  1. roster  = load_genesis("config/genesis.txt")?      // Err on missing/empty/malformed → REFUSE TO START
  2. policy  = load_hub_policy("config/hub-policy.txt")  // absent → deny_all_default()
  3. wire OperatorKill handler (§3) against `roster`
  4. wire hot-reload watcher (§2.2) on hub-policy.txt
  5. wire revocation-decision path (§4.5) onto the RevocationSet
  6. reconcile listeners from `policy` (§5.1); set insecure-bridge flag (§5.2)
  7. begin accepting frames
```

Step 1 reuses `node_id.rs`'s existing fail-closed loader — the fix is a single call site
(`grep load_genesis mesh-node/` must go from 0 to ≥1). **No trust root ⇒ no boot.** Authority is
never auto-seeded.

### 4.2 `config/genesis.example.txt` outline (the missing file)

```
# config/genesis.example.txt — EXAMPLE ONLY. Copy to config/genesis.txt (gitignored) via
# the key ceremony (KEY-CEREMONY.md). Fail-closed: an empty or absent file authorizes NOTHING.
# One anchor per line. RootDelegationPolicy MUST be explicit (Unspecified is refused at load).

root_delegation_policy = RequireExplicit

# role | node_id | key_classical(Ed25519,hex) | key_pq(ML-DSA-65,base64) | enrolled_at | scope
operator      | op-0  | 3a9f… | MIIC… | 2026-07-16 | kill,policy-update
trust-anchor  | ta-0  | 7c21… | MIIB… | 2026-07-16 | delegate
trust-anchor  | ta-1  | b0e4… | MIIB… | 2026-07-16 | delegate
```

The `operator` anchor's `scope` carries `kill` (authorizes `OperatorKill` frames, §3.1) and
`policy-update` (authorizes signed `PolicyUpdate` frames, §2.2). Anchors are frozen after genesis
(`roster.rs`), so a compromised operator key is handled by revocation + a fresh genesis
(re-ceremony), not in-place edit.

### 4.3 Kill-authority scoping

`kill` is a distinct capability scope so that not every trust anchor can halt the hub — only an
anchor explicitly enrolled with `kill`. This keeps M9's "unilateral operator" authority
**bounded to the operator anchor**, without reintroducing consensus.

### 4.4 Key-ceremony doc outline (`KEY-CEREMONY.md`)

This phase writes the runbook; the ceremony reuses Phase 6's K/V keygen machinery.

1. **Generate** a hybrid keypair (Ed25519 + ML-DSA-65) offline on a clean/air-gapped host, using
   the **gated** keygen (Phase 3's C3 fix — keygen behind `cfg`, never in a default build).
2. **Domain-separate** the PQ seed via `derive_pq_seed` (Phase 3/6).
3. **Record** the public halves in `genesis.txt` with `role=operator`, `scope=kill,policy-update`.
4. **Store** the private halves in the systemd EnvFile / vault (S3 — never in-repo, gitleaks-gated).
5. **Enroll** at genesis; freeze the roster.
6. **Verify** — sign a test kill frame and confirm it verifies against the roster *before*
   trusting the deployment (and confirm an unsigned/non-anchor frame is rejected).
7. **Rotation** — a compromised operator key is revoked (§4.5) and replaced via a fresh genesis
   (anchors are frozen post-genesis by design).
8. **Distinctness note** — the operator/kill anchor is **not** `key_K`/`key_V` (Phase 6 diff/
   verifier keys). Same ceremony machinery; different roles, scopes, and files.

### 4.5 Revocation-decision path (F5 — the human/policy half)

This phase owns the **decision**, not the transport. An operator verb (`revoke <peer_key>` —
surgical per-capability via canonical-TLV hash, or blanket subject-key) or a policy rule mutates
this node's `RevocationSet` (reuse `revocation.rs` — monotonic, irreversible). Two consequences:
(a) **local** enforcement is immediate — this node's `evict_revoked` drops the peer at once;
(b) the resulting **delta** is handed to Phase 9's gossip loop (F23/E38), which propagates it so a
**second** node drops the peer within one gossip round. The clean seam: **P10 decides + mutates +
enforces locally; P9 gossips.**

---

## 5. Dynamic listener + insecure-bridge-flag designs

### 5.1 Dynamic listener open/close (F2)

- **Reconciler.** `reconcile_listeners(policy)` diffs desired (`policy.listeners`) against actual
  (running listeners): open each listener newly present + `enabled`; gracefully drain + close each
  one removed or disabled. Invoked at boot and on every `apply_revision` (§2.2).
- **Deny-by-default.** A bind not in `policy.listeners` is never opened; an inbound connection to a
  closed bind is refused. No implicit ports.
- **Rate limit on *new listener requests*.** A `new_listener_bucket` `TokenBucket` (reuse the
  `transport_policy.rs` primitive) bounds how fast the hub may open new ports — preventing a
  runaway hub (or a Phase-15 self-mod agent) from mass-opening the port range. This is distinct
  from, and in addition to, the **per-IP accept bucket** already applied to each open port.

### 5.2 Insecure-bridge telemetry flag (F8)

- **Never silent.** On every `apply_revision` (and at boot), scan `policy.bridges`: if **any**
  bridge has `HybridPolicy::ClassicalUntilPqAudit`, emit/maintain a typed
  `InsecureBridge{bridge_id, pq_pending_since}` metric into **Phase 8's local telemetry sink**
  (M8-compliant, local-only, never exfiltrated). The flag is a first-class telemetry line, not a
  log level — a telemetry query shows it whenever active.
- **Clears** only when the bridge flips to `RequireBoth` (PQ audit complete) or is removed. F8's
  "flag-as-insecure" LOCK becomes an observable invariant: a classical-until-PQ bridge can never be
  a silent downgrade.

---

## 6. Acceptance criteria (numbered; RED cases explicit)

**Kill-switch (M9/F28):**
1. **GREEN** — a kill frame signed by a genuine genesis/operator anchor key halts a running node,
   **but only after** a COLD snapshot is confirmed to exist; the node is restartable and replays to
   the exact pre-kill state.
2. **RED (unsigned)** — a kill frame with no valid signature is ignored; the node keeps running; a
   telemetry drop event is emitted.
3. **RED (non-anchor)** — a kill frame signed by a valid-but-non-genesis-anchor key (or an anchor
   lacking `kill` scope) is rejected fail-closed; the node keeps running.
4. **RED (replay)** — a previously-seen kill nonce is dropped.
5. **RED (no backup)** — a *verified* kill frame issued before a completed COLD snapshot **refuses
   to halt** (enters `Killing-Blocked`, raises an alert) rather than losing data.
6. **Ordering invariant** — assert that in every accepted path a snapshot-confirmed receipt
   (`integrity_check=ok`) precedes process exit; there is no code path where halt precedes backup.
7. **Legacy deprecation** — the consensus `guard.rs::KillSwitch` (≥2/3 vote) is removed from / not
   referenced by the operator kill path; `grep` proves the kill path depends on no quorum.

**HubPolicy-as-data (M5/F1):**
8. Editing a `HubPolicy` field (e.g. removing a port from the accepted set) takes effect **without a
   process restart**; the removed listener stops accepting while a still-listed port keeps working.
9. **RED (floor-gate)** — a policy revision that widens a red-line scope (Auth/Money/Secrets/
   Migrations) is REFUSED and logged `REJECTED`; a floor-clean revision applies and bumps `revision`.
10. **RED (malformed)** — a malformed policy edit is rejected; the last-good revision stays live
    (a bad edit never takes the hub down).

**Boot / genesis (E37):**
11. **GREEN** — `MeshNode` boot loads a genesis file and begins accepting frames.
12. **RED (fail-closed)** — `MeshNode` boot on an **empty / missing / malformed** genesis
    **refuses to start** (returns Err, non-zero exit) — authority is never auto-seeded.
13. `config/genesis.example.txt` exists, documents the fail-closed semantics + the hybrid-anchor
    (Ed25519 + ML-DSA-65) format; `KEY-CEREMONY.md` runbook exists and names the operator/kill
    anchor as distinct from `key_K`/`key_V`.

**Dynamic listener (F2):**
14. A new listener opens at runtime **only if** present + enabled in `HubPolicy`; a bind not in
    policy is never opened (deny-by-default).
15. New-listener-open requests exceeding the `new_listener_bucket` are throttled/refused; each open
    port still carries the per-IP accept bucket.

**Insecure-bridge flag (F8):**
16. Whenever a bridge is `ClassicalUntilPqAudit`, Phase 8's local telemetry shows
    `insecure_bridge_active`; the flag clears on flip to `RequireBoth` or bridge removal; it is
    never silent.

**Revocation decision (F5 — this phase's half):**
17. An operator `revoke` verb produces a monotonic `RevocationSet` delta and this node immediately
    drops the revoked peer's frames locally.
18. **Integrated with Phase 9** — the delta gossips to a second node, which drops the revoked
    peer's frames within **one gossip round** of the revocation decision.

---

## 7. Cross-phase boundaries (what this phase does NOT own)

- **Phase 9** owns the wire frame-kind registry (this phase only *requests* `FrameKind::OperatorKill`)
  and the revocation **gossip transport** (F23/E38). This phase owns the hub-side handler and the
  revocation **decision**.
- **Phase 3/6** own the key material: Phase 3 makes genesis anchors hybrid (ML-DSA); Phase 6 builds
  the K/V + operator-key ceremony machinery this phase reuses.
- **Phase 12** owns the COLD zstd archiver + restore-verify the kill handler blocks on.
- **Phase 8** owns the local telemetry sink the insecure-bridge flag and policy-revision events
  write to.
- **Phase 15** owns *self-authored* HubPolicy revisions (generalizing `SelfModEffector`) and model
  **ingestion** (sha3-verify-or-deny). This phase builds the HubPolicy data + hot-reload + apply
  pipeline + floor-gate that Phase 15 authors *into*; it does not build the self-mod author, the
  model fetch/verify path, or F10 subtree-kill semantics (deferred, O13).

---

## 8. Planning-protocol completion appendix (2026-07-17, decorrelated pass)

Per the Detailed Planning Protocol (`AGENTS.md`) and the Anu/Ananke doctrine. This blueprint had no
DECART, 2Q audit, or Anu/Ananke check — all supplied here, alongside fresh citation verification.

### 8.1 — Citation verification against live repo (bebop-repo HEAD `397b8cd8`)

All 8 of this blueprint's file:line/grep citations were checked directly. **All hold up, several to the
exact line:**
- `crates/bebop/src/guard.rs:64-124` (`KillSwitch` ≥2/3 consensus vote) — confirmed: struct + doc-comment
  ("no single node can kill another") at lines 64-77, `vote_suspend`/`recompute` logic follows within
  range. Accurate.
- `bebop2/proto-wire/src/transport_policy.rs:17-45` (`MAX_MESSAGE_BYTES`, `IDLE_TIMEOUT_SECS`,
  `TokenBucket`) — confirmed exact: `MAX_MESSAGE_BYTES` line 19, `IDLE_TIMEOUT_SECS` line 23,
  `TokenBucket` struct line 28. **Notable, not cited by the blueprint:** this file's own header now reads
  *"MESH-10 transport hardening policy"* — the same hardening pass that fixed the wss_transport.rs
  staleness found in the P09 pass (that file's §12.1) also touched this exact file. No claim here is
  invalidated, but it corroborates the file is under active, recent churn.
- `bebop2/proto-cap/src/redline.rs:1-45` (`RedLinePolicy` default-DENY) and `hybrid_gate.rs:24-34`
  (`HybridPolicy::{RequireBoth, ClassicalUntilPqAudit}`) — both confirmed exact, doc-comments match
  verbatim.
- `bebop2/proto-cap/src/node_id.rs:80-180` (genesis loader) — confirmed: `GenesisError` enum +
  fail-closed doc-comments ("FAIL-CLOSED: ... enrolls NOTHING") present in this range.
- `roster.rs:187-204` (frozen `AnchorRoster`) — confirmed, doc-comment "the roster is frozen: exactly
  these keys may bootstrap" at line ~187.
- **`grep load_genesis mesh-node/` = 0 — re-run fresh this pass, still zero.** Loader-never-called
  finding confirmed current, not stale.
- **`grep -niE "kill.switch|kill_switch|killswitch"` across `proto-wire`/`mesh-node` = 0 — re-run fresh,
  still zero.** Confirmed current.
- `revocation.rs:35-55`/`:13-15` and `discovery.rs:82` (`evict_revoked`) — confirmed, `evict_revoked` is
  an **exact** line match.

**Verdict: this blueprint's evidence base is the strongest of the four assigned to this pass** — dense,
all-verified, no staleness found (contrast with P09's one material stale claim). Worth noting precisely
because it shows the citation quality this protocol asks for is achievable, not merely aspirational.

### 8.2 — DECART

**Owed and missing — two real new-tool choices, neither DECART'd. Written here:**

**(1) Hot-reload file-watch mechanism (§2.2).** *"a file-watch (inotify) on hub-policy.txt fires on
change"* — naming a Linux facility with no existing crate dependency anywhere in this repo
(`grep -rn "notify\|inotify" Cargo.lock`/`Cargo.toml` across both repos → zero). A genuine new-dependency
decision, undiscussed until now.

| Candidate | Bare-metal fit | Correctness/security | Perf | Supply-chain | Maintainability | Reversibility | Evidence |
|---|---|---|---|---|---|---|---|
| `notify` crate (cross-platform inotify/kqueue/ReadDirectoryChangesW) | Good, but pulls 3 platform-specific backends a hub only ever runs one of | Mature; needs debounce for save patterns, crate provides it | Instant, event-driven | New dep, moderate transitive surface | External maintenance, occasional API churn | High — swappable behind a `PolicyWatcher` trait | crates.io |
| raw `inotify` crate | Linux-only (breaks portability) | Correct, needs hand-rolled event coalescing | Instant | Smaller than `notify`, still new | More manual code, single-platform | Medium | crates.io |
| **stat-`mtime` polling loop (`std::fs::metadata().modified()` on a timer)** | Universal, zero OS-specific code | Trivially correct (compare a timestamp) | 1-2s worst-case detection latency (tunable) — irrelevant for a hand-edited config file | **Zero new dependency** | Simplest to audit (15-line loop) | Trivial to swap later | — |
| shell out to `inotifywait` (external binary) | **REJECTED** — violates ALL-RUST-NATIVE (runtime binary dependency, not a thin Rust port) | — | — | — | — | — | — |

**DECISION: stat-`mtime` polling loop, zero new dependency.** Matches the codebase's own demonstrated
bias — nearly every cited module in this pass carries a "no new dependency" comment (`revocation.rs`,
`hydra.rs`'s `FileEventStore`, `field_frame.rs`'s SIMD work) — and the trigger here (an operator
hand-editing a rarely-changed policy file) does not need sub-second reactivity. **Mandatory probe
(honest case against):** if an operator's incident-response workflow ever depends on a policy edit
taking effect *instantly* (e.g., closing a listener under active attack), a 1-2s poll delay is a real
cost — and the signed-`PolicyUpdate`-over-the-mesh trigger (§2.2's second path) already covers the urgent
case, which somewhat defangs the objection but does not eliminate it for the *local* file-edit path. If
raised at build time, `notify` is the correct upgrade — this is a cheapest-sufficient default, not a
permanent rejection.

**(2) `HubPolicy` concurrent-swap primitive (§2.2 step 3).** *"Atomically swap the in-memory
`Arc<HubPolicy>` (RCU-style — readers hold a snapshot, no lock on the hot path)"* — ambiguous between
`std::sync::RwLock<Arc<HubPolicy>>` (std-only, but a read-lock IS a lock, however cheap) and the
`arc-swap` crate (genuinely lock-free `ArcSwap::load()`, purpose-built for this pattern, no dependency
currently in either repo). **This is the one case in this pass where the specialized crate may be
genuinely justified by the blueprint's own stated constraint** — "no lock on the hot path" is a literal
requirement plain `RwLock` does not satisfy under contention, however unlikely. Recorded rather than
decided: if per-frame policy reads are frequent enough that `RwLock` contention is measurable,
`arc-swap` is the correct outcome; if not, plain `RwLock<Arc<HubPolicy>>` is the zero-dep default.
Flagged for a build-time bake-off — the blueprint text should say this explicitly rather than leave
"RCU-style" doing unexamined work.

### 8.3 — 2-question doubt audit (per-blueprint)

**Q1:**
1. §2.2's inotify mention reads as an aside, not a decided design element — I treated it as a real
   commitment worth a DECART; a charitable reading is it was one illustrative option. I judged it worth
   doing anyway per the task's instruction to flag implied new-tool choices.
2. I did not check whether `guard.rs::KillSwitch` has any existing caller today that would break if
   *deleted* rather than merely bypassed — the acceptance criterion (§6.7) only requires the new kill
   path not depend on it, weaker than "safe to delete." I did not grep for `KillSwitch::` call sites.
3. `KEY-CEREMONY.md` (§4.4) is specified in detail but I did not check whether a document by that name
   (or an equivalent, alongside the H8/P1-PAUSE runbooks found in the P12 pass) already exists.
4. I confirmed `load_genesis` in `node_id.rs` but did not specifically verify Phase 6's
   `derive_pq_seed`/K-V keygen machinery §4.4 says this ceremony reuses.
5. The floor-gate invariant (§2.2 step 2) depends on `redline.rs` — confirmed to exist and match the
   cited range, but that file gained new Auth/Secret/Migration categories in commits `c4edbf1`/`4f3553f`
   *after* this blueprint's implicit baseline. I did not check whether P10's floor-gate design already
   accounts for these newer categories.
6. I did not check whether "a frame in flight completes against the snapshot it began with" (§2.2 step
   3) is achievable given how frames are currently dispatched in `mesh-node` — asserted, not verified
   against a concrete call site.

**Q2 — biggest thing this pass might be missing:** this blueprint is the most evidence-rigorous of the
four (§8.1), which risks over-trusting its *design* sections (§2-§7) by association — strong
current-state evidence does not imply the proposed HubPolicy/kill-switch/boot design is itself correct,
only that the problem statement is accurate. The design has not been red-teamed here. The two DECART
gaps found (§8.2) suggest the design sections were written with "good citations, thinner
tooling-decision rigor" — reused-primitive-heavy evidence sections were rigorous *because* they were
reuse, while the design sections introduce genuinely new mechanisms (a hot-reload watcher, a concurrent
swap) that got named but not chosen.

### 8.4 — Anu (logic) & Ananke (organization) check

**Anu.** The headline claim — M9's kill-switch doesn't exist, and the same-named legacy construct
implements its structural opposite — is independently re-derivable from the two live greps in §8.1
(kill-switch grep = 0, `guard.rs::KillSwitch` = quorum-gated) and holds. The kill-frame verification path
(§3.1) is logically chained to real, checked primitives (hybrid roster, replay ledger from F25, genesis
anchors) rather than asserted floating requirements. Where the design cannot be derived from what exists
today — e.g., whether `RwLock` contention is actually a problem — this appendix names that as unresolved
(§8.2) rather than the blueprint silently picking "RCU-style" language that reads more decided than it
is. That imprecision is the one place this blueprint's Anu compliance was weaker than its evidence
sections.

**Ananke.** The floor-gate invariant (§2.2) and COLD-snapshot-before-halt ordering (§3.2) are both
genuinely structural: the acceptance criteria (§6.5, §6.9) test the *invariant*, not a description of
intended behavior, and the §3.2 state machine makes "halt before backup" a code path that must not
exist, not a convention to remember. **What relies on future diligence:** the two DECART gaps in §8.2 —
nothing in this blueprint's structure would have caught "a new dependency was implied without a DECART"
except this decorrelated pass; there is no equivalent of Phase 1's "DECART-dep lint" (referenced in
sibling blueprint P11 §7) wired to catch an *implied* dependency named only in blueprint prose rather
than an actual `Cargo.toml` line. That lint protects code, not planning documents — worth naming as a
structural gap the planning protocol itself has.

---

## Addendum (2026-07-17, Layer-I consolidation, L1) — update-blob code-signing is a Phase-10 unit

Folded in from `MASTER-ROADMAP-MVP-2026-07-12.md` (D4.4, `:41,68,89` — would otherwise be lost,
per `CORE-ROADMAP-2026-07-17/P-I-audit-cross-repo-consolidation.md` §3 item L1): **node self-update
artifacts must ML-DSA-verify against a pinned root before apply; an unsigned or tampered update
blob refuses (RED gate).** This lands in Phase 10 because this phase already owns boot integrity
and the M9 kill-switch signing path — update-blob verification is the same trust-root discipline
applied to the boot artifact's successor. The primitive is **already built and live on this
branch**: `kernel/src/pq/codesign.rs` (`codesign_keypair` + `PinnedRoot`, with tests — verified
2026-07-17). The unit here is wiring it into the hub-runtime update path, not building it; it
inherits this blueprint's Phase-3/C4b gating exactly like the kill-switch signing leg.
