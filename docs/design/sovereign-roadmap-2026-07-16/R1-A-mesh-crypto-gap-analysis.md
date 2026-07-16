# R1-A — Mesh & PQ-Crypto Substrate: Gap Analysis (2026-07-16)

> Cluster: M1, M2, M3, M4, M6, M10, M12 · D2 · F11–F20 · F21–F30 · E10 · E31–E35 · E36–E40.
> Canon: `docs/design/ARCHITECTURE.md` (§0 M-series + SCOPE RULE, §6 F-series, §8 honest gaps)
> + `docs/design/STRATEGIC-VECTORS-LOCKED-2026-07-16.md`.
> Code ground truth: `/root/bebop-repo` (workspace tip `b87b7e2`), plus `/root/dowiz`
> (`feat/kernel-fsm-graph-analysis`) and `/root/dowiz-pq` (`feat/pq-crypto-tier1`, "178 tests").
> Method: direct reads + greps + `cargo test` re-execution. **Read-only — nothing fixed, nothing committed.**
> Two decorrelated Explore sub-agents supplied the transport-layer and crypto-doc cross-checks below.

---

## 0. Headline findings (read this first)

1. **The PQ-capability substrate is far more built than the canon's tone implies — but it is a
   library, not a running protocol.** ML-DSA-65 is a from-scratch FIPS 204 port verified against
   vendored official NIST ACVP vectors (`bebop2/core/src/pq_dsa.rs`, 1286 lines + `pq_dsa/acvp_tests.rs`).
   The M12 capability model — canonical-TLV signing, hybrid Ed25519+ML-DSA gate, anchor-rooted
   delegation chains, verify-then-record nonce ledger, expiry, RevocationSet, fail-closed red-line
   deny — exists and is tested in `bebop2/proto-cap/`. A `Transport` trait with three real impls
   (quinn-QUIC, WSS, in-mem) exists. `cargo test` across all five crates is **GREEN today** (§4).

2. **M1 is the biggest structural gap: dowiz does not ride on the protocol at all.**
   `grep proto-cap|bebop2|mesh-node` over `/root/dowiz/kernel/ tools/` returns exactly one *comment*
   (`kernel/src/domain.rs:524`). Mesh-as-foundation is aspiration; today it is a parallel library
   stack in a separate repo, never consumed by the delivery service it is meant to carry.

3. **The wire is authenticated but NOT confidential (F16 gap, HIGH).** ML-KEM is fully implemented
   but **never invoked for transport encryption** — `grep encaps|decaps` across proto-wire/mesh-node
   returns zero. Confidentiality in transit is TLS-only, and the compiled-in default feature is
   `insecure-tls` (`InsecureAcceptAny` accepts any cert, `iroh_transport.rs:149-155`,
   `Cargo.toml:38`); the WSS carrier runs plaintext `ws://` (`wss_transport.rs:451-458`:
   "authenticated but readable by a passive on-path observer"). `transport_policy.rs` ships a
   `NoopPayloadEnc` passthrough where the ML-KEM→XChaCha20 layer belongs (`:107-133`). F16
   ("encrypts traffic with ML-KEM to a peer it just met") is **NOT BUILT**.

4. **The trust root is classically forgeable (F21 gap, HIGH).** Delegation links are signed
   **Ed25519-only** (`proto-cap/src/roster.rs:114,167,179`) and genesis anchors are 32-byte Ed25519
   keys (`node_id.rs` loader). Frames carry a hybrid leg, but a quantum attacker forges a delegation
   chain from any enrolled anchor, then satisfies the PQ leg with their **own** keypair (subject_key_pq
   is attacker-chosen inside the forged chain). This makes the ML-DSA frame leg **vacuous against
   exactly the F21 adversary it exists to stop.** Canon says "ML-DSA holds" — but the thing ML-DSA
   protects excludes the authorization chain. Unnamed in the canon; named here.

5. **D2 is inverted vs code (CANON-GAP).** Canon: "iroh-QUIC primary, quinn fallback via DECART." Code:
   `iroh` is deliberately NOT a dependency (offline build + `ed25519-dalek 3.0.0-rc.0` pin conflict),
   quinn 0.11.11 is the only QUIC carrier, and `iroh_transport.rs` is quinn-under-the-hood
   (`Cargo.toml:31,41-52`; `iroh_transport.rs:1,7-15`). As written, **F20 ("iroh punch, quinn if iroh
   down") is impossible today — there is no iroh to fall back FROM.** Resolve via DECART: land iroh, or
   amend D2/F20 to "quinn primary; iroh promoted via DECART on network-unlock trigger."

6. **Three parallel PQ stacks, and two of them disagree with FIPS (CORRECTNESS + dual-authority).**
   `bebop2/core/src/pq_kem.rs` is FIPS-203-exact (J(z‖c) implicit rejection, η1=2, passes official ACVP
   — `kem_implicit_rejection_equals_fips203_j` GREEN). But `bebop2/proto-crypto/src/pq_kem.rs` sets
   `ETA1 = 3` ("matched to the external reference impl", `:166`), uses `H(sk‖c)` not `J(z‖c)` (`:583`),
   and `G = SHAKE256` not SHA3-512 (`:136`) — **its own header claims η₁=2 while the constant is 3**, and
   it is KAT-locked to `/root/dowiz-pq/kernel/src/pq/kem.rs`, which shares those non-FIPS choices. So
   two implementations both named "ML-KEM-768" are bit-exact to *different, mutually-inconsistent*
   oracles. Same failure family as the 3-eigensolver dual-authority bug. M2 needs one named canon +
   named oracles, and proto-crypto's KEM struck or corrected.

7. **Canon defect (flagged, NOT resolved): E10 duplicates E36.** STRATEGIC-VECTORS:89 lists
   `E10 ML-DSA hybrid(A)` and :98 lists `Security (E36-40): ML-DSA hybrid; …` — identical three words,
   two anchor IDs, no distinguishing text in either doc. The 147-count double-counts this, or one ID
   was meant to carry different scope never written down. **Operator must merge or differentiate;** this
   analysis treats them as one anchor (hybrid signing = BUILT).

8. **Undefined anchor (D5/D8 pattern): E35 "3-tier locality."** Zero code hits, and no definition of the
   three tiers exists anywhere in ARCHITECTURE.md or STRATEGIC-VECTORS beyond the five-word cluster
   summary. It cannot be built or falsified until canon defines it. Named as a canon gap, not papered over.

---

## 1. Per-anchor: current state → target → gap

Legend: **BUILT** (code + tests) · **PARTIAL** (core exists, named pieces missing) · **NOT BUILT**
(no code either repo) · **CANON-GAP** (target itself ambiguous/undefined).

### M-series

**M1 — Mesh = FOUNDATION.** *Current:* NOT BUILT as integration. Protocol stack green in bebop2; dowiz
has zero dependency on it (only comment `dowiz/kernel/src/domain.rs:524`). *Target:* every dowiz service
produces/consumes protocol frames. *Gap:* an integration seam (a dowiz port speaking SignedFrame) + a
cross-repo topology decision (dowiz depends on bebop2 crates, or crates vendored/published). → Phase C.

**M2 — ML-DSA-65 + ML-KEM-768, FIPS204/203, bit-exact KAT, classical fallback.** *Current:* PARTIAL,
strongest anchor with authority-diffusion. ML-DSA-65 BUILT (ACVP byte-exact, `pq_dsa.rs`). ML-KEM-768
BUILT in `core` (FIPS-exact) and again in `proto-crypto` (non-FIPS divergent stub, headline #6), and a
**third** stack in `/root/dowiz-pq/kernel/src/pq/`. Classical **signature** fallback BUILT (Ed25519,
`core/src/sign.rs`). Classical **KEM** fallback (X25519/ECDH) **ABSENT from bebop2** entirely
(`grep x25519|ecdh bebop2/ = 0`); the ML-KEM⊕X25519 hybrid exists only in the legacy `crates/bebop/src/vault.rs`
(RustCrypto `x25519-dalek`/`ml-kem`). Crypto-ladder cross-check harness (`proto-crypto/{ladder,wycheproof,fips_regen}.rs`)
= `Placeholder` TODO stubs. *Target:* one canonical impl + in-repo KAT + classical fallback for BOTH sig and KEM.
*Gap:* designate canon + demote/delete others; add X25519 KEM leg to bebop2; resolve proto-crypto non-FIPS
divergence; fill or strike ladder placeholders. → Phase A.

**M3 — Quantum-noise optional (QRNG beacon).** *Current:* PARTIAL, more built than canon implies.
`proto-cap/src/entropy.rs` (487 lines): `OsEntropy` mandatory fail-closed floor (getrandom); `AnuQrng`
models the ANU API but is **disabled by default**, real HTTP gated behind `--features anu` (off), returns
`EntropyUnavailable` when off (`:107,154-157`); `SeedPool` mixes `SHA3-512(floor ‖ advisory-QRNG ‖ counter)`
(`:285-292`); tests prove floor-alone sufficient and QRNG-can-never-replace-floor (`:386-415,467-473`).
*Target:* optional off-by-default beacon mixing, graceful ML-DSA-only fallback (F13). *Gap:* operator
toggle wiring into a running node + a live beacon-down integration test; the beacon client itself already
exists feature-gated. **CANON-NOTE:** where a network-touching beacon may live vs M6's zero-dep boundary is
unstated — the caller-supplied-entropy API already resolves it (inject bytes from outside the boundary). → Phase B.

**M4 — Every edge autonomous, self-certifying ML-DSA identity, no central CA.** *Current:* PARTIAL.
`node_id.rs:46-51` `NodeId = sha3_256(pq_pub ‖ classical_pub)`, self-derived, no CA; frames self-signed
hybrid. Authorization deliberately anchor-rooted (self-signed w/o chain → `UnknownIssuer`,
`hybrid_gate.rs:41-47`). *Target:* edges self-certify identity AND sign frames without central CA.
*Gap:* identity layer essentially done; canon should state M4 = identity+signing while *authorization*
roots in per-hub genesis anchors (that IS "no central CA" — each hub owns its roster). PQ gap: chain
Ed25519-only (F21). → Phase A (PQ), otherwise closed.

**M6 — Zero protocol deps; transport swappable behind Trait.** *Current:* BUILT/PARTIAL. proto-cap +
bebop2-core zero-external-dep on signed path (from-scratch Keccak/Ed25519/ML-DSA/ML-KEM; serde only for
out-of-band envelope, never signing — `capability.rs:10-17`). `Transport` trait at `proto-wire/src/lib.rs:58`
(`connect/accept/send/recv` over `SignedFrame`); impls `QuicTransport` (`iroh_transport.rs:235`),
`WssTransport` (`wss_transport.rs:379`), `MemTransport` (`bpv7.rs:332`). Swap is a compile-time generic
(`MeshNode<T: Transport>`, `node.rs:41`). *Target:* + iroh/HTTP/stdio impls. *Gap:* stdio/HTTP transports
absent; iroh absent (D2). Trait + swappability = done. → Phase B (extend), core closed.

**M10 — Inter-hub protocol, intra-hub anarchy.** *Current:* PARTIAL by construction. Compile-firewall real:
`facade.rs:1-16` verifies (wire→Law→money), holds no kernel logic, host kernel unreachable from protocol
crate; intra-hub lives behind host `EventSink`. Missing: any *second service hub* — inter-hub proof exists
only between test nodes over QUIC, never between two real hubs with divergent internals. *Gap:* same seam
as M1. → Phase C.

**M12 — Capability model.** *Current:* BUILT at library level, two named partials. Hybrid signing REAL
both legs (`signed_frame.rs:190-256`, ACVP ML-DSA + real Ed25519). Fail-closed gate ordering
(`hybrid_gate.rs:124-201`): expiry → chain(roster) → red-line → revocation → classical → PQ → **then**
commit nonce (verify-then-record H2 fix; bounded ledger `MAX_SEEN_NONCES=1<<20`, prune, poison→error).
Expiry `is_fresh` (`capability.rs:128`). RevocationSet (`revocation.rs:49-138`, incl. `gossip_payload`).
Red-line `DenyByDefault` default (`redline.rs:42-61`) — **but `is_red_line` maps only Money verbs**;
Auth/Secrets/Migrations are reserved enum variants with no Resource mapping. UCAN-subset in-repo (rejects
the 43★ crate). **Doc-drift hazard:** `proto-cap/src/lib.rs:12` still calls the PQ leg "a marked TODO" —
stale; the code beneath it is wired. *Gap:* red-line mapping for Auth/Secrets/Migrations; decide whether
per-instance in-process replay ledger is the blessed bound or needs persistence; fix stale header. → Phase A.

### D2 — Network: iroh primary / quinn fallback / deny-by-default tokens.
*Current:* INVERTED (headline #5). Deny-by-default tokens BUILT (M12). iroh absent, quinn is the carrier.
*Target/Gap:* CANON-GAP — resolve via DECART (land iroh, or amend text + name a network-unlock trigger). → Phase B.

### F11–F20 (mesh/transport)

- **F11** wire-format disagreement → reject. **BUILT (strict, two layers).** Envelope: `ENVELOPE_VERSION=1`,
  version mismatch rejected fail-closed (`framing.rs:59-61`), 1 MiB cap before alloc. Frame codec: magic
  `b"BEBOPFRM"` + version 0x01, unknown-field reject (no silent forward-compat skip, `wire_codec.rs:287-292`),
  trailing-byte reject, 200-iter hostile-byte fuzz (`:474-505`). Caveat: envelope version is unsigned/outer —
  documented `innovate:` gap (not yet bound into signature, `framing.rs:54-58`).
- **F12** island mode. **BUILT.** `bpv7.rs` BPv7-style custody store-and-forward (exactly-once reconnect test
  `:400`); `sync_pull.rs` MerkleLog anti-entropy pull (convergence test `:1032`). Absent: fork/conflict
  resolution beyond content-addressed union.
- **F13** QRNG down → ML-DSA-only. **BUILT-by-absence** today (floor always present); real test lands with M3. → Phase B.
- **F14** webgl topology viz. **NOT BUILT** (confirmed absent; unrelated wgpu UI work is an honest stub). → Phase C.
- **F15** partition/HRW-merge. **NOT BUILT.** HRW exists only for *courier assignment* (`proto-cap/matcher.rs`),
  not mesh-partition leadership/merge; that specific application found nowhere. → Phase B.
- **F16** ML-KEM encrypt-to-new-peer. **NOT BUILT** (headline #3). → Phase B.
- **F17** gRPC internal + REST edge. **NOT BUILT** — no tonic/prost anywhere; locked target, unimplemented. → Phase C.
- **F18** batching. **NOT BUILT** as a library primitive — only test-only manual batching
  (`mesh_sync_integration.rs:103-200`). → Phase B.
- **F19** unknown capability → fail-closed drop. **BUILT** (`scope.rs from_discriminant→Option`; chain rule 6
  "requested effect ⊆ tail scope"; typed `CapError`, never a pass).
- **F20** iroh/quinn NAT fallback. **NOT BUILT** as designed — no iroh; `discovery.rs` is peer-directory +
  gossip, explicitly "NOT a DHT", no hole-punch/STUN/relay (punted as deployment concern,
  `iroh_transport.rs:23-25` innovate marker). → Phase B (resolve with D2).

### F21–F30 (security/quantum)

- **F21** quantum attacker / ML-DSA holds. **PARTIAL — frames hold, trust root does NOT** (headline #4, HIGH). → Phase A.
- **F22** quantum-noise on. **NOT BUILT** (= M3). → Phase B.
- **F23** key leak → RevocationSet. **BUILT lib-level**; revocation *propagation* over the wire unwired (E38). → Phase B.
- **F24** expired capability rejected. **BUILT** (`is_fresh`, gate pre-check).
- **F25** replay rejected. **BUILT in-process** (per-gate `Mutex<HashSet>` ledger, verify-then-record) —
  explicitly not distributed. Canon must bless the bound or require persistence. → Phase B (decision).
- **F26** red-line money denied. **BUILT** (Money category live); Auth/Secrets/Migrations unmapped. → Phase A.
- **F27** unaudited model + sha3-gate. sha3 exists; a model-artifact sha3-verify-or-deny gate NOT BUILT here
  (model loading is a hub/agent-infra concern — **cross-cluster, owned by cluster B/E9**; tracked, not placed here).
- **F28** operator kill-switch. **WRONG-SHAPE.** `crates/bebop/src/guard.rs:64-124` `KillSwitch` is a ≥2/3
  *consensus* peer-suspension registry ("not a central off-button"), the OPPOSITE of M9's unilateral operator
  hard-kill; mesh substrate itself has no kill-switch (`grep=0` in proto-wire/mesh-node). → Phase C.
- **F29** no remote OTel by default. **NOT BUILT** in bebop2 (`grep opentelemetry|otel = 0`); only dowiz design
  docs + shell env passthrough. → Phase C.
- **F30** XChaCha20-Poly1305 at-rest. **BUILT but mislocated** — real in `crates/bebop/src/vault.rs`
  (Argon2id + ML-KEM⊕X25519 hybrid + XChaCha20-Poly1305), NOT in the mesh substrate / per-hub DB. → Phase B.

### E10 — ML-DSA hybrid. **CANON-GAP (duplicate of E36, headline #7).** Substance: BUILT (`RequireBoth` /
`ClassicalUntilPqAudit`, `hybrid_gate.rs:24-34`).

### E31–E35 (network/mesh one-liners)
- **E31** iroh+HRW. HRW BUILT for courier assignment (`matcher.rs`); iroh NOT BUILT; HRW-for-partition-merge
  NOT BUILT. **CANON-NOTE:** five words conflate two distinct HRW uses. → Phase B.
- **E32** Dijkstra/A*. Real A* exists in `crates/bebop/src/cost_estimate.rs:238-290` for *courier route cost*,
  NOT mesh topology repair. For mesh = NOT BUILT (canon's own F45 marks "gap-fill P1"). → Phase B.
- **E33** Union-Find/MST. Union-Find exists in `dowiz/kernel/src/order_machine.rs:611-656` for *FSM cyclomatic
  number*, not network membership. MST found nowhere in code. For mesh = NOT BUILT (F46 "gap-fill P1"). → Phase B.
- **E34** iroh-quinn NAT. NOT BUILT (= F20). → Phase B.
- **E35** 3-tier locality. **NOT BUILT + CANON-GAP (undefined, headline #8).** → Phase C (blocked on canon def).

### E36–E40 (security one-liners)
- **E36** ML-DSA hybrid. Duplicate of E10 (headline #7).
- **E37** operator-gated genesis. **BUILT lib-level, exemplary.** `node_id.rs:80-180` fail-closed loader
  (missing/malformed/empty → error, "authority never auto-seeded"), `RootDelegationPolicy::Unspecified` default
  + `require_explicit_policy`. **Gaps:** referenced `config/genesis.example.txt` **does not exist**; loader is
  **not wired into `MeshNode` boot** (`grep load_genesis mesh-node/ = 0`); no production key ceremony; anchors
  classical-only (F21). → Phase A (PQ anchors) + Phase C (boot wiring + ceremony + example file).
- **E38** RevocationSet. BUILT lib-level; propagation unwired (= F23). → Phase B.
- **E39** signed event_log. `core/src/event_log.rs` is SHA3-256 **hash-chained** (tamper-evident), NOT
  signature-signed directly — signing happens one layer up at the `SignedFrame`/`HybridGate` boundary before
  a payload is folded in. No unified `SignedEventLog` type. Design is sound; canon wording "signed event_log"
  should be read as "hash-chained log fed only by hybrid-verified frames." → Phase C (clarify/optionally unify).
- **E40** EnvFile+gitleaks. DEV-TIME ops anchor (SCOPE RULE), cross-owned with cluster E; nothing in bebop2
  reads env secrets on the trust path (caller-supplied-entropy model). Closed for this cluster.

---

## 2. Prior-research cross-check (crypto-safe-first-pass follow-ups)

Source doc lives at `/mnt/volume-fsn1-1/dowiz-memory/crypto-safe-first-pass-2026-07-14.md` (memory store, NOT
`docs/design/`). bebop-repo history was rewritten (clean-slate publish), so the memory doc's commit hashes are
unresolvable; verification is by current file content.

- **C4b — mod_l nonce leak (HIGH): STILL OPEN.** `core/src/sign.rs:625-650` `mod_l()` retains a secret-dependent
  branch (`if (byte>>bit)&1 == 1 {…}`) + data-dependent conditional subtract, run on the secret nonce/key
  (`:659,667-675`, called from `sign` `:822-823`). Code's own comment `:701-723` tags it "Tracked as C4b" (HIGH).
  **This is a live classical side-channel on the Ed25519 signing path.** → Phase A.
- **C6b — test fixtures co-derive both hybrid legs: STILL OPEN** (test-only). `wss_transport.rs:707` /
  `iroh_transport.rs:421` feed the same `leaf_seed` to both `keygen_derivable` and `sign_classical`. Scoped
  test-only; low risk. → Phase A (tidy with C4b).
- **proto-crypto `H(sk‖c)` divergence: STILL OPEN** (= headline #6). proto-crypto KEM non-FIPS (H(sk‖c),
  G=SHAKE256, η1=3); `core` KEM is FIPS-exact. → Phase A.
- **C3 — ungated keygen: FIXED + CI-enforced.** `pq_dsa.rs:995` gates `keygen` behind `cfg(any(test,
  dangerous_deterministic, test_keygen))`; `scripts/ci-no-ungated-keygen.sh` wired into `.github/workflows/ci.yml`
  `sovereign-guards`. `CRYPTO-P0-REMEDIATION-2026-07-16.md` is an orthogonal RSA/ECB/AES-CBC hunt (verdict CLEAN),
  does NOT supersede these follow-ups.

**mesh-real docs vs code:** MESH-01..14 mostly now BUILT (facade, claim_machine, matcher, sync_pull, crdt-fence,
iroh_transport, WSS-TLS, revocation, node_id, ML-KEM). **MESH-06 pgrust-per-node still ABSENT** (`grep pgrust
bebop2/ = 0`). MESH-12 genesis loader BUILT but not wired into boot + example file missing (E37 above).

**SOVEREIGN-EVENT-EXCHANGE G1–G7:** self-reported CLOSED claims **verified true** against the tree — all named
guard scripts exist and are wired in CI (`ci-no-courier-scoring`, `ci-crdt-fence`, `ci-kernel-fence`,
`verify-empty-imports` [note: not "empty-import"], `ci-claim-live-test`, plus `ci-no-serde-json-wire`,
`ci-no-duplicate-eventlog`, `ci-no-flat-scope`, `ci-no-redline-gate`, `ci-no-ungated-keygen`). Unusually
well-verified doc.

**Matcher canonicality:** NOT stale duplicates — two different problems. `crates/bebop/src/matcher.rs` (325 ln)
= the *routing/dispatch* matcher (MATCHER-API.md's named reference client: k-d filter → BFS → A*/CH cost engine).
`bebop2/proto-cap/src/matcher.rs` (181 ln) = MESH-05 *assignment tie-break* (HRW/FNV-1a rank of already-reachable
candidates, no graph, no cost). Each referenced only by its own crate's `lib.rs`; neither wired into a downstream
service yet. `bebop2` = canonical protocol line; `crates/bebop` = legacy product node (RustCrypto-based).

---

## 3. Test re-execution (GREEN today, exit 0)

`cargo test -p bebop2-core -p bebop-proto-cap -p bebop-proto-crypto -p bebop-proto-wire -p bebop-mesh-node --offline`:
- `bebop2-core`: **232 passed / 0 failed** (incl. `pq_kem::kem_implicit_rejection_equals_fips203_j`,
  `pq_kem::dual_impl_bit_exact`, ML-DSA ACVP, Ed25519 RFC8032, `scalar_mul_op_count_is_constant`).
- eig parity suites: 10 + 1 passed. Doc-tests green (3 ignored). All five crates compile+test clean.
- Integration proof (`proto-wire/tests/mesh_sync_integration.rs`, real QUIC not mock): 2-node forward,
  2-node bidirectional, idempotent, **3-node convergence**, **3-node gossip roster convergence** — all pass.
- Caveat (agent-found): `mesh-node/src/node.rs` doc claims "concurrent bidirectional send+recv loops" but has
  no `tokio::spawn`/`select!`/run-loop — it provides the primitives; the caller must drive concurrency.

---

## 4. Build phases (ordered; every cluster anchor is IN a phase)

Ordering principle: **harden the cryptographic foundation before building surface on it; make the wire
confidential + self-healing before wiring the product onto it.** You do not integrate a product onto a
substrate whose trust root is classically forgeable (F21) and whose wire is plaintext-readable (F16).

### Phase A — Harden the PQ trust root (crypto correctness first)
**Anchors:** M2, M4 (PQ identity), M12 (red-line categories F26 + stale-header), F21, E10/E36 (confirm),
E37 (PQ anchors), C4b, C6b, proto-crypto FIPS divergence.
**Why first:** every gate authenticates on these primitives. A forgeable delegation chain (F21) makes the
entire hybrid gate vacuous against the quantum adversary; a leaking `mod_l` (C4b, HIGH) is a live classical
break; three mutually-inconsistent "ML-KEM-768"s is a correctness time-bomb. Fix the foundation before pouring.
**Scope:** `proto-cap/{roster,node_id,redline,lib}.rs`, `core/src/{sign,pq_kem,pq_dsa}.rs`,
`proto-crypto/src/pq_kem.rs`, `/root/dowiz-pq` reconciliation; DECART note for the canon-designation.
**Falsifiable done:** (1) delegation links + genesis anchors carry ML-DSA (hybrid); a classically-forged chain
is rejected under a quantum-attacker fixture. (2) dudect/constant-time gate GREEN on `mod_l` (C4b closed).
(3) exactly ONE canonical ML-KEM-768 + ML-DSA-65 with ACVP KAT; every other copy deleted OR marked oracle-only
with a CI test asserting byte-agreement (proto-crypto/dowiz-pq η1/H/G reconciled to FIPS or struck). (4) red-line
gate denies an Auth-, Secrets-, and Migrations-scoped capability (not just Money). (5) `proto-cap/lib.rs` header
no longer claims PQ is TODO.

### Phase B — Confidential, self-healing wire (transport substrate completion)
**Anchors:** M3/F13/F22, M6 (extend), D2/F20/E34, F16, F18, F23/E38, F25 (decision), F30, F15/E31, E32, E33.
**Why second:** depends on Phase A's canonical primitives (F16 encrypts with the *canonical* ML-KEM; revocation
propagation ships the *finalized* RevocationSet). Confidentiality (F16) is the #2 gap after M1; healing
(E31/E32/E33, the M7 no-SPOF mechanism) turns the mesh from "sync layer" into "survives partition."
**Scope:** `proto-wire/src/{transport_policy,discovery,iroh_transport,wss_transport}.rs`, `proto-cap/entropy.rs`
(operator toggle), new mesh-healing module (HRW-merge + Dijkstra/A* + Union-Find/MST over the peer graph),
per-hub at-rest store (relocate/generalize `vault.rs` XChaCha20 into mesh-node), DECART doc for D2/iroh.
**Falsifiable done:** (1) two nodes that just met derive an ML-KEM session key; a passive on-path capture is
ciphertext, not plaintext `ws://` (F16). (2) `--features anu` on → beacon bytes mix into the seed; beacon down →
floor-only seed still succeeds (F13/M3). (3) partition test: two islands with divergent roots deterministically
merge via HRW (F15/E31). (4) a revoked key gossips to a 3rd node and is rejected there (F23/E38). (5) shortest
path + spanning tree computed over a synthetic mesh graph, and a dropped node is routed around (E32/E33/M7).
(6) D2: a written DECART decision — iroh landed, OR canon amended to "quinn primary + named network-unlock trigger."

### Phase C — Product rides the protocol (integration + service surface)
**Anchors:** M1, M10, E37 (boot wiring + ceremony + `config/genesis.example.txt`), F17, F28, F29, F14, E39, E35.
**Why last:** integration is only meaningful once the substrate is hardened (A) and confidential+healing (B).
Wiring dowiz onto a substrate still missing F16/F21 would bake insecure assumptions into the product.
**Scope:** a dowiz→protocol port (SignedFrame producer/consumer), `mesh-node` boot path (load_genesis wiring +
unilateral operator kill-switch of M9 shape), gRPC-internal + REST-edge service skeleton, local-only OTel opt-in,
feature-gated webgl topology renderer, cross-repo dependency decision.
**Falsifiable done:** (1) a dowiz order frame is produced by dowiz, hybrid-signed, carried over the mesh, and
verified+folded by a SECOND hub running a different internal store (M1+M10). (2) `MeshNode` boot loads a genesis
file and fail-closes on empty (E37). (3) an operator kill signal drops a hub/subtree unilaterally — distinct from
the existing consensus KillSwitch (F28/M9). (4) a gRPC-internal call and a REST-edge call both serve the same order
projection (F17). (5) local OTel sink writes locally and nothing leaves the host by default (F29/M8).
**BLOCKED-ON-CANON:** **E35 (3-tier locality)** — first task is an operator/canon ruling that *defines the three
tiers*; it cannot be coded or falsified until then (headline #8). Placed in Phase C so no anchor is orphaned, but
its gate is a canon decision, not code.

---

## 5. Canon defects surfaced (for operator ruling — do not self-resolve)
1. **E10 ≡ E36** — identical "ML-DSA hybrid," two IDs, no distinction. Merge or differentiate (affects the 147 count).
2. **D2 inversion** — "iroh primary" contradicts the offline-buildable quinn-only reality. DECART resolve.
3. **E35 undefined** — "3-tier locality" has no definition anywhere. Define or strike.
4. **F20 impossible-as-written** — "quinn if iroh down" has no iroh to fall back from. Amend with D2.
5. **F21 under-claims** — "ML-DSA holds" ignores that the Ed25519-only delegation chain + genesis anchors are the
   forgeable root; the frame's PQ leg is vacuous without them. Canon should name the trust-root PQ requirement.
6. **M3 vs M6 boundary** — where a network-touching QRNG beacon client may live relative to the zero-dep wire
   boundary is unstated (resolvable: outside the boundary, injected as caller entropy).
7. **E39 wording** — "signed event_log" is actually a hash-chained log fed by pre-verified signed frames; there is
   no single signed-log type. Reword to avoid implying a missing primitive.
8. **M12 replay scope** — the nonce ledger is per-gate-instance in-process, not distributed; canon should bless
   that bound or require persistence (F25).
