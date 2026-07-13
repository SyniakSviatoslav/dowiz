# bebop2 mesh REAL + per-node-local-first REAL — RESEARCH CONSPECT (5 lanes + compat-gate)

> Дата: 2026-07-13 · 5 паралельних дослідницьких смуг (M1 inventory · M2 transport · M3 delivery-domain ·
> M4 local-first · M5 authz), усі глибоко ґрунтовані в реальному коді + live `cargo test`. + виконаний
> pgrust COMPAT-GATE. Для [MESH-REAL-PLAN.md](./MESH-REAL-PLAN.md) + [BLUEPRINTS](./BLUEPRINTS-MESH-REAL.md).
> MVP non-negotiable (operator): меш РЕАЛЬНИЙ + per-node-local-first РЕАЛЬНИЙ. pgrust per-node.

## ★ pgrust COMPAT-GATE — ВИКОНАНО, ПРОЙШОВ (live, docker malisper/pgrust:v0.1 = pgrust 18.3)
CREATE EXTENSION citext ✅ · pgcrypto ✅ · citext-case-insensitive-match=1 ✅ · gen_random_uuid ✅ ·
digest-sha256 ✅ · bcrypt crypt/gen_salt('bf')→$2a$06$ ✅ (couriers.password_hash). Образ везе ПОВНИЙ
contrib (citext/pgcrypto/hstore/ltree/pg_trgm/uuid-ossp/postgres_fdw +40). README «extensions not
compatible» ЗАСТАРІЛИЙ. → pgrust per-node store de-risked; дамп restore-able.

## ★★ КЛЮЧОВЕ (M1+M5): red-team B2/B3/B4 (00:56 2026-07-13) ЧАСТКОВО ЗАСТАРІЛІ — багато вже FIXED на HEAD 898c888:
- verify_chain WIRED на acceptance-path (hybrid_gate.rs:110, wss_transport recv:187-188 real-clock),
  self-issued-key REJECTED live (wss_rejects_self_signed_frame_over_real_carrier), 33/33 proto-cap green,
  13/13 proto-wire green. RequireBoth flips real verify_pq. envelope-version enforced (framing:56-58).
  bounded-nonce MAX_SEEN_NONCES=1<<20+prune. → "node captured" CLOSED, re-verified live.
- IMPROVEMENT-PLAN §0 explicitly: reports audited a TRANSIENT pre-fix state; status must derive from live test.

## LANE M1 — Current REAL vs STUB vs MISSING vs DECIDED (file-anchored, cargo-test-verified)
- REAL: core crypto (Ed25519/ML-DSA-65 FIPS-204-bit-exact ACVP-60/60/XChaCha20-Poly1305/Argon2id/SHA-512/3/
  ChaCha20-fail-closed-entropy, 180/180 green). proto-cap capability/scope/tlv/roster/hybrid_gate/signed_frame
  REAL+WIRED(acceptance-path). proto-wire envelope/framing REAL, wss_transport REAL(12/12) but PLAINTEXT-ws://.
  dowiz-kernel order_machine(decide/fold 10-states)/money(i64-overflow-checked-no-float)/domain(place_order/
  apply_event)/analytics(ChannelLedger)/intake REAL+tested+NO-COURIER-SCORING. crates/bebop matcher.rs
  deterministic-replicable(fingerprint LocalClient==RemoteClient, fail-closed unmatched-never-dropped) REAL.
- STUB: proto-crypto 100% Placeholder-skeleton. iroh_transport 100%-stub(all-methods Err(NotConnected)) → WSS-
  plaintext = ONLY carrier (n=1, NOT-p2p). WSS-confidentiality plaintext(TLS-off, native-tls-banned). ML-KEM-768
  real-algo but NON-FIPS-203-interop(coefficient-domain-not-NTT)+no-external-KAT+timing-side-channels+no-zeroize.
  channel-binding decorative(never-compared-to-live-transcript). handshake-struct never-exchanged.
- MISSING: ★delivery-domain vocabulary in bebop2 (scope=4×3-enum-labels Route/Ledger/DeliveryIntent/Presence×
  Send/Read/Append, ZERO order/price/PoD/matcher/settlement structs). ★dowiz-kernel↔bebop2 wiring (ZERO either
  direction). node_id=H(pq_pub‖classical_pub) ADR-0007-PROPOSED-unbuilt. per-node-store ADR-0008-PROPOSED-unbuilt.
  DTN/BPv7+TCPCLv4+BIBE transport(D3-LOCKED-unbuilt, actual=WSS CONTRADICTS-locked-decision). roster-revocation/
  rotation(genesis-frozen, no-propagation, no-prod-genesis-loader-only-tests-enroll). H2 insert-before-verify-
  ordering(still-open). KernelFacade(dowiz-IP-01-blueprint-unbuilt). bebop2/{kernel,cli,reloop} README-claims-
  don't-exist. bebop2 no-root-workspace-Cargo.toml.
- DECIDED (MANIFESTO/DECISIONS 2026-07-12 OUTRANK 07-11 blueprints per D8): decentralized/local-first/PQ/mesh
  NON-NEGOTIABLE (D6 mesh-NOW-not-deferred); transport=DTN/BPv7 (D3-locked, reject-libp2p-gossipsub/Zenoh);
  PQ=composed-protocol-not-primitives (C12); central-server-DROPPED (C13/D1); integer-money (C5); reliability>
  latency-store-and-forward (C11); node identity self-certifying (ADR-0007); per-node-store PQ-at-rest (ADR-0008).
  ⚠️ MIGRATION-PLAN diagram OMITS dowiz-kernel (gap: the reusable asset isn't in the written plan).

## LANE M2 — Transport (make it real; D3=DTN/BPv7)
- iroh 1.0 GA (Jun-15-2026, wire-stable, quinn-backed, ~90%-direct-hole-punch+auto-relay, dial-by-pubkey,
  ProtocolHandler/ALPN) = D3's MISSING NAT-traversal layer, NOT-a-conflict (raw-quinn has no "2-couriers-behind-
  NAT-find-each-other"). LAYERING: BPv7-store-forward-overlay (bp7-rs RFC9171-CBOR-codec-ONLY + hand-roll custody/
  retry/expiry — DON'T embed dtn7-rs-daemon: duplicates HybridGate + routing-metrics-risk-NO-COURIER-SCORING; no-
  QUIC-CL-exists-for-TCPCLv4) OVER iroh-QUIC-convergence(custom-ALPN "bebop2/wire/1", same Envelope/framing bytes,
  Transport-trait/envelope/framing UNCHANGED) OVER tokio-tungstenite+tokio-rustls WSS-fallback(browser/edge).
- Carrying PQ-SignedFrames: custody=own extension-block keyed-by-Capability.nonce (BPv7-dropped-built-in-custody,
  RFC9713-drafting); retransmit-until-ack=TCPCLv4-session-contract as app-logic on iroh-bidi-stream (open→frame→
  await-custody-ack-or-timeout→exp-backoff→drop-past-lifetime); bundle-expiry=CreationTimestamp+Lifetime vs real-
  clock (reuse); offline-reconnect=persist-undelivered-per-dest-queue(rusqlite/pgrust) drain-oldest-first, each
  replay FRESH-channel-binding (never-resend-byte-identical).
- FIX B3: (F1-TLS) WSS→tokio-rustls(pure-Rust, native-tls-ban-compliant) + iroh-QUIC-mandatory-TLS1.3 + ON-TOP
  ML-KEM-768→XChaCha20-Poly1305 SignedFrame-payload-enc (both KAT-tested in core, defense-in-depth-past-semi-
  trusted-relay). (F2/F3-replay) channel_binding from REAL TLS/QUIC-exporter (RFC5705 export_keying_material,
  native rustls/quinn) + move HybridGate.seen OUT-of-per-conn INTO node-scoped-persistent-store (survive-reconnect;
  repo REMEDIATION-PLAN already-names-this). (F5/F7-DoS) WebSocketConfig{max_message/frame 8<<20 matching
  framing::MAX_ENVELOPE_BYTES not-tungstenite-64MiB-default} + tokio::time::timeout on every recv + Semaphore-conn-
  cap + per-IP-token-bucket pre-accept; iroh quinn::TransportConfig(max_idle_timeout/max_concurrent_bidi/receive_
  window) absorbs-Slowloris-natively.
- CRATES: bp7-rs(RFC9171 codec, security-inert, wasm), quinn(via iroh), tokio-rustls. NOT dtn7-rs/ud3tn(daemons).
- RED: plaintext_ws_rejected_when_tls_required; cross_connection_replay_rejected; offline_courier_reconnect_
  delivers_exactly_once; slowloris_stalled_dropped_by_idle_timeout.

## LANE M3 — Delivery-domain (make it real; REUSE dowiz-kernel)
- REUSE ~90%: dowiz-kernel order_machine{OrderStatus,assert_transition,fold_transitions}/domain{Order,place_order,
  apply_event,compute_order_total}/money(all) UNMODIFIED as per-node AUTHORITATIVE decide/fold; analytics ingest/
  fold PATTERN for settlement-reducer; bebop2 SignedFrame/Capability/HybridGate/AnchorRoster/Transport as envelope;
  matcher.rs MatcherRequest/Response+fingerprint+replicability-pattern.
- Link dowiz-kernel as plain-Rust-rlib into NEW crate bebop-delivery-domain (direct-native no-WASM/JSON-hop) +
  Cargo-feature-split gating wasm-bindgen/serde_json behind opt-in wasm-feature (so bebop2-dep doesn't inherit
  browser-deps). Every node embeds SAME dowiz-kernel, calls assert_transition/apply_event LOCALLY on every event
  (never-trust-sender) = "central-kernel"→"replicated-identical-kernel".
- Event vocabulary = SignedFrame{Capability{Resource,Action},payload}: OrderPlaced(Order/Append→place_order),
  OrderStatusChanged(Order/Append→assert_transition-LOCALLY-every-receiver), ClaimOffered(Claim/Send→matcher),
  ClaimAccepted(Claim/Append→new-claim_machine), ClaimReleased(compensation-first-class→requeue=matcher-unmatched),
  Pickup/DeliveryConfirmed(Order/Append device-signed→InDelivery/Delivered), SettlementRecorded(Ledger/Append→
  money::apply_tax i64-only). CHOREOGRAPHY no-orchestrator, compensations-first-class, claim-races-self-resolve
  (losing ClaimAccepted fails assert_transition-on-replay→becomes-ClaimReleased zero-arbiter, every-node-identical-
  fold).
- MATCHER: matcher.rs already-proves needed-property (pure-fn, fingerprint-cross-node-agreement, fail-closed-
  unmatched-never-dropped). GAP: Order{id,src,dst} = ROUTING (can-courier-reach) not ASSIGNMENT (N-candidate-
  couriers-per-order). NEW = N-to-1 selection with canonical coordination-free tie-break = rendezvous/HRW-hash of
  (order_id,courier_pubkey).
- NEW code (thin): claim_machine.rs (sibling-Law Offered→Claimed→{Released|PickedUp}), matcher N-candidate-tie-break,
  Resource::{Order,Claim} scope-variants, Cargo-feature-split, SettlementLedger-reducer(analytics-shaped, money-
  sourced), TLV-payload-spec, MECHANICAL NO-COURIER-SCORING-CI-grep(score/rating/trust/reputation/rank on new
  struct-fields; today doc-comment+closed-enum-convention only).
- RED: same_event_same_decision_across_nodes; illegal_transition_rejected_locally; money_integer_never_float;
  no_courier_scoring_ci_gate(build-fails-on-score-field); refused_order_not_dropped.

## LANE M4 — Per-node local-first (pgrust + event-log + sync)
- pgrust per-node (compat-gate-PASSED) = queryable-store + event-log-backing-table + read-projections(order-status/
  ChannelLedger/analytics) SAME instance, "no-shared-central-DB"(ADR-0008) + real-Postgres-maturity. Current=hub-
  interim; TARGET=one-pgrust-per-node(only-own-operator-data, no-cross-node-query-surface-by-construction). Local
  writes→kernel::decide→commit-local-event-log BEFORE-any-network-IO (offline="sync-hasn't-run-yet" never-degraded-
  write).
- ★NO 2026-sync-engine directly-adoptable: Automerge/cr-sqlite/Ditto=CRDT-merge(money-antipattern); ElectricSQL/
  PowerSync/Zero/LiveStore=single-Postgres/central-backend-source(opposite-of-N-sovereign-nodes); Zero=HARD-BLOCKER
  (no-offline-writes-by-design). LiveStore=closest-conceptual(event-sourced-not-CRDT-deterministic-fold) but-central-
  backend. → REUSE PATTERN not library.
- WHY event-sourcing-NOT-CRDT (authoritative): CRDT-guarantees-commutative-CONVERGENCE but convergence≠obeys-legal-
  transitions (LWW-can-silently-double-apply-refund/resurrect-cancelled-order, money-doesn't-commute); correctness=
  REPLICATED-STATE-MACHINE (same-event-seq+same-deterministic-fold⇒same-state, fold-rejects-what-order_machine-
  disallows, not "eventually-agrees").
- SYNC=pull-anti-entropy peer-to-peer no-central-backend; peer-requests-events-after-last-actor_seq per-remote-actor
  (EC-12/15 offset), folds-locally, dup=no-op via content-addressed-id=hash(prev,actor_pubkey,actor_seq,payload)=
  idempotency(log-own-dedup-no-TTL, makes-at-least-once-gossip-safe). ★LONG-OFFLINE-CATCH-UP = Merkle/prolly-tree-
  digest of event-log (Dolt content-defined-chunking rolling-hash) diff-root-hashes pull-only-divergent-chunks (same
  primitive already-named RAG-reindex BLUEPRINTS-ECOSYSTEM:78). Every sync-frame rides SignedFrame(hybrid channel-
  bound) gated NEW Sync·Pull-capability-scope, never-raw-decide-bypass.
- CRDT ONLY periphery(notes/tags/presence, IP-11 Automerge-own-device-backup-only) + ★COMPILE-FENCE (PARALLEL-EXEC-
  PLAN:27 "sync-crdt-forbidden-from-domain/settle/dispatch" analogous IP-01-KernelFacade → build-FAILS-if-order/money-
  crate-depends-CRDT-merge-crate; "we-don't"→"it-doesn't-compile").
- BOOTSTRAP: AnchorRoster(genesis-frozen roster.rs) + operator-out-of-band-root-delegation(ADR-0007-open-HUMAN:QR/
  operator-signed-root) + one-bulk-pull-actor_seq=0-all-actors, fold-entire-log-BEFORE-live-gossip = SAME pull-anti-
  entropy empty-offset (NO-separate-bootstrap-protocol, one-mechanism-not-two).
- BLUEPRINT units: LF-01 pgrust-per-node(supersedes-ADR-0008-SQLite=red-line-update), LF-02 content-addressed-event-
  log+actor_seq-index, LF-03 Sync·Pull-port+Merkle-catch-up, LF-04 CRDT-periphery-compile-fence, LF-05 genesis-
  bootstrap. RED: offline-rejoins→converges-identical; money-never-CRDT-merged(build-fails); dup-event=no-op; illegal-
  transition-rejected-on-sync(reduce_anomalies-flags).

## LANE M5 — Authz/identity (finish; B2 core-gap already-fixed)
- Acceptance-path = 3-gates FIXED-order all-BEFORE-state: WIRE(HybridGate::check = replay/expiry→verify_chain-anchor-
  rooted-narrow-only→Ed25519→ML-DSA-65-RequireBoth) → LAW(assert_transition illegal=409) → MONEY(i64-kernel-invents-
  no-number). Today WIRE-gate REAL+WIRED; LAW/MONEY exist-in-kernel BUT KernelFacade(makes-wire→Law→money-COMPILED-
  invariant-not-convention) still-blueprint(IP-01) → SHIP-before-any-new-adapter (exactly-the-class-B2-found: correct-
  primitives-bypassable-path).
- node_id=H(pq_pub‖classical_pub) ADR-0007 possession-based no-CA (SPKI/RFC2693 lineage "public-key-IS-identity") =
  fixes D1-F1(seeded-test@dowiz.com-minting-owner-JWT: "nothing-to-seed, identity-born-from-keygen"). UNBUILT (no
  node_id code in proto-cap). Every-actor(owner/courier/customer/node)=SAME{Ed25519,ML-DSA-65}-keypair, role=what-
  Delegation-grants-that-key never-identity-field (structural NO-COURIER-SCORING).
- Roster genesis = ONE-deliberate-centralization: AnchorRoster enrolled-once-frozen. PROD genesis-loader UNBUILT
  (only-tests-.enroll-inline; connect/accept build empty-roster + rely-on-.with_roster). ADR-0007-open-HUMAN:
  operator-signed-root vs WoT vs first-contact-QR.
- Capability minting=self-service(grants-nothing-alone); authority=Delegation-chain-rooted-frozen-roster,
  attenuation-ONLY-per-link(effect⊆scope, narrows-monotonically, tail-binds-subject) RED-proven(red_effect_not_
  subset/red_broken_chain/red_self_issued_rejected). = UCAN/Biscuit restrict-only-no-re-ask-issuer, from-scratch-
  TLV/Ed25519⊕ML-DSA-65 not-JWT/DID.
- ★REVOCATION DOESN'T EXIST (grep Revocation/revoke/drop_anchor=0). Only-passive-expiry. drop-anchor(remove-from-
  AnchorRoster-HashSet)=structurally-trivial BUT no-mesh-wide-PROPAGATION-design (each-node-local-roster-copy →
  needs-gossip/consensus-story). = LARGEST open-item, matches-2026-research (Vouchsafe arxiv:2601.02254, Lingering-
  Authority arxiv:2606.22504). Build RevocationSet+drop-anchor+propagation.
- H2 insert-before-verify STILL-OPEN (hybrid_gate:88-105 nonce-inserted-BEFORE verify_chain/verify_classical,
  contradicts-own-"verify-then-record"-doc; OOM/panic-subissues-fixed bounded-set/no-expect but ordering-defect-not).
- offline-verifiable no-central-issuer = fits-mesh (phone-home-not-guaranteed) vs JWT/OAuth-breaks-when-intermittent.
- RED: self-issued-key-REJECTED(EXISTS-GREEN wss_rejects_self_signed); unwired-path-fails-live-test(add-CI-lint-any-
  "CLOSED"-doc-claim-needs-matching-live-path-test); attenuation-cannot-widen(EXISTS-GREEN); revoked-capability-stops-
  verifying(DOESN'T-EXIST→write-first vs hand-rolled-RevocationSet before-wiring-KernelFacade).

## CROSS-CUTTING crypto-hardening (M1 B1): ML-KEM-768 FIPS-203-interop(NTT-domain-not-coefficient)+external-KAT(ACVP-
KEM-vectors)+constant-time(no-secret-dependent-branch/var-time-%)+zeroize; proto-crypto skeleton→real(wycheproof-
vectors+constant-time-harness). NOTE: ML-DSA-65 already-FIPS-exact; ML-KEM is the crypto-gap.

## CONTRADICTIONS to resolve in plan: (1) D3-DTN/BPv7-LOCKED vs built-WSS+iroh-stub (reconcile: iroh=QUIC-CL-under-
BPv7-overlay, build-the-locked-stack). (2) MIGRATION-PLAN-diagram OMITS dowiz-kernel (the reusable delivery-domain).
(3) ADR-0007/0008 PROPOSED→ratify+update(SQLite→pgrust). (4) red-team-docs-STALE vs live-code (add CI-lint: CLOSED-
claim needs live-path-test-name; derive-status-from-live-test-not-prose). (5) bebop2 no-root-workspace + README-
phantom-dirs(kernel/cli/reloop).

Sources: iroh-1.0/quinn/bp7-rs/RFC9171/9174/9713/5705, ucan/biscuit/SPKI-RFC2693/Vouchsafe/Lingering-Authority,
Automerge/cr-sqlite/ElectricSQL/PowerSync/Zero/Ditto/LiveStore, Dolt-prolly-tree, rendezvous-hashing, choreography-
saga. Files: bebop2/{core,proto-cap,proto-wire,proto-crypto}/src/*, crates/bebop/matcher.rs, dowiz/kernel/src/*,
ADR-0007/0008, MANIFESTO/DECISIONS, MIGRATION-PLAN, red-team B1-B4/REMEDIATION/IMPROVEMENT-PLAN. Commits 18a0ad1/
07911d9d/2d2731bf. compat-gate: malisper/pgrust:v0.1 live.
