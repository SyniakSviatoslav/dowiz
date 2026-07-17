# bebop2 mesh REAL + per-node-local-first REAL — BLUEPRINTS

> **Дата:** 2026-07-13 · Супроводжує [MESH-REAL-PLAN.md](./MESH-REAL-PLAN.md).
> 14 одиниць MESH-01..14. Кожна: **Мета · Межа (чіпаємо/НЕ чіпаємо) · Форма · Reuse · RED · Хвиля.**
> НЕ код — блюпринт. Принцип: **з'єднати наявне (~90% готове), добудувати 4 відсутні шари.**
> 🔴 = red-line (crypto/auth/money/data/незворотнє → human gate). Стан кожної речі — тільки з live-test.

---

## Шар A/B — Per-node ядро + delivery-словник (G0-G3)

### MESH-01 · `bebop-delivery-domain` crate — link dowiz-kernel як rlib
- **Мета:** зробити dowiz-kernel per-node авторитетним decider для bebop2, без WASM/JSON-hop.
- **Межа:** ЧІПАЄМО — новий crate `bebop-delivery-domain` + Cargo-feature-split у `dowiz-kernel`. НЕ ЧІПАЄМО —
  order_machine/domain/money (reuse UNMODIFIED, вони позначені «NOT modified»).
- **Форма:** новий crate path-depends `dowiz-kernel` як plain-Rust **rlib** (native, no-WASM). Feature-split
  `dowiz-kernel/Cargo.toml`: gate `wasm-bindgen`/`serde_json` JS-glue за opt-in `wasm`-feature → bebop2-dep не
  успадковує browser-deps. Кожен нод викликає `assert_transition`/`apply_event` ЛОКАЛЬНО на кожну подію.
- **Reuse:** dowiz-kernel {order_machine, domain, money, analytics}. **RED:** bebop-delivery-domain білдиться без
  wasm-feature (нема serde/wasm у графі); kernel-логіка diff=0. **Хвиля:** G0.

### MESH-02 · KernelFacade — компільований ланцюг wire→Law→money
- **Мета:** зробити «кожна подія проходить wire→Law→money ПЕРЕД станом» build-time-інваріантом, не конвенцією.
- **Межа:** ЧІПАЄМО — новий facade-crate. НЕ ЧІПАЄМО — kernel; gates (HybridGate/assert_transition/i64 вже real).
- **Форма:** facade експонує `submit_intent(SignedFrame)->Result<Vec<Event>,Reject>` (WIRE=HybridGate::check →
  LAW=assert_transition → MONEY=i64) + `read_projection(Scope)->Projection`. Адаптер НІКОЛИ не має `dowiz-kernel`
  у deps. (= dowiz IP-01; закриває клас-бага B2: correct-primitives-bypassable-path.)
- **Reuse:** HybridGate(wired), assert_transition, money. **RED (R0):** scratch-adapter додає dowiz-kernel у deps +
  викликає decide → **BUILD FAILS** (Cargo-graph-guarantee, сильніше за runtime-check). **Хвиля:** G1. 🔴

### MESH-03 · Delivery event-словник + Resource::{Order,Claim} + choreography
- **Мета:** дати мешу реальний delivery-lifecycle як event-sourced choreography (B4-F2: зараз ZERO).
- **Межа:** ЧІПАЄМО — `scope.rs` (адитивні варіанти) + event-payload-TLV-spec. НЕ ЧІПАЄМО — SignedFrame-envelope.
- **Форма:** Resource += `Order`, `Claim` (закриті-enum-pinned-discriminants). Події = `SignedFrame{Capability{
  Resource,Action}, payload}`: OrderPlaced(Order/Append→place_order), OrderStatusChanged(→assert_transition ЛОКАЛЬНО-
  кожен-receiver-ніколи-не-довіряє-sender), ClaimOffered(Claim/Send→matcher), ClaimAccepted(Claim/Append→claim_machine),
  ClaimReleased(compensation-FIRST-CLASS→requeue), Pickup/DeliveryConfirmed(device-signed→InDelivery/Delivered),
  SettlementRecorded(Ledger/Append→money-i64). No-orchestrator; claim-гонки самовирішуються (losing-ClaimAccepted
  fails-assert_transition→стає-ClaimReleased zero-arbiter, every-node-identical-fold).
- **Reuse:** order_machine, SignedFrame, scope-pattern. **RED:** forged OrderStatusChanged{Pending→Delivered}
  rejected on EVERY receiver (not just sender); same-event→same-Order-state across 2 nodes (mirror matcher-replicability).
  **Хвиля:** G2. 🔴

### MESH-04 · `claim_machine.rs` — sibling Law
- **Мета:** courier-claim aggregate (order_machine ніколи не моделював claims).
- **Межа:** ЧІПАЄМО — новий `claim_machine.rs` (same decide/fold-shape). НЕ ЧІПАЄМО — order_machine.
- **Форма:** Offered→Claimed→{Released|PickedUp}, `assert_transition`+`fold_transitions` дзеркально order_machine;
  no-scoring-fields (structural NO-COURIER-SCORING).
- **Reuse:** order_machine-shape. **RED:** illegal claim-transition rejected; PickedUp/Released terminal-legal.
  **Хвиля:** G2.

### MESH-05 · Matcher assignment + NO-COURIER-SCORING CI-grep
- **Мета:** розширити routing→assignment (N-candidate) + зробити no-scoring механічним gate.
- **Межа:** ЧІПАЄМО — matcher extension + новий CI-скрипт. НЕ ЧІПАЄМО — matcher-fingerprint/replicability-core.
- **Форма:** matcher.rs вже pure-fn/fingerprint/fail-closed-unmatched-never-dropped. Розширити `Order{id,src,dst}`
  (routing) → **assignment** N-candidate-couriers-per-order з canonical coordination-free tie-break = **rendezvous/
  HRW-hash(order_id, courier_pubkey)** (кожен нод обчислює те саме, no-coordination). CI-grep: build-fails якщо нове
  struct-поле матчить `score|rating|trust|reputation|rank` (doc-convention→enforced-gate).
- **Reuse:** matcher.rs. **RED:** 2 nodes → identical assignment (fingerprint); refused-order requeues-never-dropped;
  CI-gate red on score-field. **Хвиля:** G3.

---

## Шар E — Per-node local-first + sync (G4-G5)

### MESH-06 · pgrust-per-node store + content-addressed event-log
- **Мета:** кожен нод — локальний pgrust (compat-PASSED) = store + event-log-table + read-projections.
- **Межа:** ЧІПАЄМО — per-node pgrust-provision + event-log-schema. НЕ ЧІПАЄМО — kernel::decide (єдиний write-path).
- **Форма:** pgrust-per-node (тільки-own-operator-data, no-cross-node-query-surface-by-construction). Local-write →
  kernel::decide → commit-event-log BEFORE-network-IO (offline=«sync-hasn't-run» never-degraded-write). Event-id =
  **hash(prev, actor_pubkey, actor_seq, payload)** = idempotency (log-own-dedup-NO-TTL). Read-projections (order-
  status/ChannelLedger/analytics) у тому ж instance.
- **Reuse:** pgrust(compat✓), kernel decide/fold, ADR-0008. **RED:** dup-event(same-content-id) replayed → state-
  unchanged 2nd-time; write-succeeds-offline (no-network); supersedes ADR-0008-SQLite (red-line ADR-update). **Хвиля:** G4. 🔴

### MESH-07 · Sync·Pull port — pull-anti-entropy + Merkle catch-up
- **Мета:** реплікація подій між нодами без central-backend, з дешевим long-offline-catch-up.
- **Межа:** ЧІПАЄМО — новий Sync·Pull-порт + Merkle-digest. НЕ ЧІПАЄМО — decide (sync-frame ніколи не bypass-decide).
- **Форма:** pull-anti-entropy: peer-requests-events-after-last-`actor_seq` per-remote-actor, folds-locally, dup=no-op-
  via-content-id. Long-offline = **Merkle/prolly-tree-digest** of event-log (Dolt content-defined-chunking rolling-hash)
  → diff-root-hashes → pull-ONLY-divergent-chunks. Frame rides SignedFrame(hybrid, channel-bound) gated NEW **Sync·Pull**-
  capability-scope, never-raw-decide. Bootstrap = same-mechanism empty-offset (actor_seq=0, fold-before-live-gossip).
- **Reuse:** SignedFrame, EC-12/15-pattern, Merkle(already-named-RAG-reindex). **RED:** 2-nodes-diverge-offline→reconnect→
  pull→**identical-folded-state**; illegal-gossiped-event rejected-locally + reduce_anomalies-flags (never-silently-
  applied). **Хвиля:** G5. 🔴

### MESH-08 · CRDT-periphery compile-fence
- **Мета:** зробити «CRDT ніколи для money/orders» компільованим, не doc-конвенцією.
- **Межа:** ЧІПАЄМО — dependency-graph lint. НЕ ЧІПАЄМО — periphery CRDT (notes/tags/presence, IP-11-own-device-backup).
- **Форма:** lint/dep-graph-gate: build-**FAILS** якщо будь-який crate, що торкається order/money-state, залежить від
  CRDT-merge-crate (Automerge/cr-sqlite). «we-don't-do-that»→«it-doesn't-compile» (analogous KernelFacade).
- **Reuse:** PARALLEL-EXEC-PLAN:27, IP-01-technique. **RED:** money-crate-adds-CRDT-dep → build-fails. **Хвиля:** G5.

---

## Шар C — Транспорт реальний (G6-G7)

### MESH-09 · Transport: iroh-QUIC carrier + BPv7 store-forward overlay
- **Мета:** зробити реальний p2p mesh-транспорт (iroh 100%-stub сьогодні; D3=DTN/BPv7).
- **Межа:** ЧІПАЄМО — `iroh_transport.rs` body + новий BPv7-overlay. НЕ ЧІПАЄМО — Transport-trait/envelope/framing
  (carrier-neutral, unchanged).
- **Форма:** **iroh 1.0** (GA Jun2026, quinn-backed, hole-punch+relay, dial-by-pubkey, ALPN «bebop2/wire/1») =
  QUIC-convergence-layer (= D3's-missing-NAT-traversal, НЕ конфлікт). **BPv7-overlay** = `bp7-rs` (RFC9171-CBOR-codec
  ТІЛЬКИ) + hand-roll custody(extension-block keyed-Capability.nonce)/retremit-until-ack(iroh-bidi-stream)/expiry
  (CreationTimestamp+Lifetime-vs-real-clock). **НЕ** dtn7-rs-daemon (дублює HybridGate + routing-metrics-ризик-NO-
  COURIER-SCORING; нема-QUIC-CL). Offline-reconnect = persist-undelivered-per-dest-queue(pgrust/rusqlite) drain-oldest-
  first, FRESH-channel-binding-each-replay.
  > ⚠ CORRECTED (operator, 2026-07-16): the undelivered-queue persistence lists `rusqlite` (SQLite) as a co-option.
  > dowiz does NOT use SQLite as an architectural choice — the spectral/sqlless approach (content-addressed
  > `BlockStore` + JSONL `FileEventStore`) is the MAIN storage/retrieval path in dowiz's own kernel/engine, with
  > **pgrust as the uniform SQL-fallback/backup target, not SQLite**. Corrected: the per-dest undelivered queue is an
  > append-only spectral/sqlless store (natural fit for drain-oldest-first) or a **pgrust**-backed queue if a SQL shape
  > is genuinely needed — never rusqlite/SQLite.
- **Reuse:** Transport-trait, envelope, framing, matcher-Transport-abstraction. **RED:** offline_courier_reconnect_
  delivers_exactly_once (drop-mid-session→queue→reconnect-fresh-channel→single-delivery-via-custody-ack-dedupe). **Хвиля:** G6. 🔴

### MESH-10 · WSS-rustls-TLS + replay-safety + DoS-hardening + PQ-payload-enc (fix B3)
- **Мета:** закрити B3 (plaintext/replay/Slowloris) + defense-in-depth PQ-encryption.
- **Межа:** ЧІПАЄМО — wss_transport hardening + payload-enc. НЕ ЧІПАЄМО — native-tls-ban (використати pure-Rust rustls).
- **Форма:** WSS `MaybeTlsStream::Plain`→**tokio-rustls** (pure-Rust, ban-compliant); iroh-QUIC-mandatory-TLS1.3. **+
  ML-KEM-768→XChaCha20-Poly1305 payload-encryption** SignedFrame (both-KAT-tested-core, past-semi-trusted-relay). Replay:
  channel_binding з **real-TLS/QUIC-exporter** (RFC5705, native-rustls/quinn) + HybridGate.seen **node-scoped-persistent**
  (не-per-conn, survive-reconnect). DoS: `WebSocketConfig{max_message/frame 8<<20}` (не-tungstenite-64MiB) + `tokio::time::
  timeout` every-recv + Semaphore-conn-cap + per-IP-token-bucket-pre-accept; iroh `quinn::TransportConfig` absorbs-natively.
- **Reuse:** core ML-KEM/XChaCha, existing recv/real-clock. **RED:** plaintext_ws_rejected_when_tls_required; cross_
  connection_replay_rejected (flip B3-F2-PoC); slowloris_stalled_dropped_by_idle_timeout. **Хвиля:** G7. 🔴

---

## Шар D — Authz/identity доведення (G8-G9)

### MESH-11 · Revocation: RevocationSet + drop-anchor + propagation + H2-fix
- **Мета:** дати мешу revocation (НЕ існує сьогодні — лише expiry) = найбільша діра.
- **Межа:** ЧІПАЄМО — новий RevocationSet + drop-anchor + gossip-propagation + hybrid_gate-ordering. НЕ ЧІПАЄМО —
  attenuation-invariants (вже RED-proven).
- **Форма:** RevocationSet (UCAN-style irreversible-invalidate) + drop-anchor (remove-from-AnchorRoster-HashSet,
  structurally-trivial-locally) + **mesh-wide-propagation** (gossip/consensus — 2026-research-open, Vouchsafe/Lingering-
  Authority). Fix **H2 insert-before-verify**: verify_chain/verify_classical FIRST, record-nonce AFTER (verify-THEN-record
  per own-doc).
- **Reuse:** AnchorRoster, HybridGate. **RED:** revoked_capability_stops_verifying (write-FIRST vs hand-rolled-
  RevocationSet BEFORE-wiring-KernelFacade, so-facade-inherits-real-revoke); nonce-not-consumed-by-unauthenticated-frame
  (H2). **Хвиля:** G8. 🔴 · найбільший open-item.

> ⚠ CORRECTED (2026-07-17, agentic-mesh consolidation, live-verified): MESH-11's premise
> ("revocation НЕ існує сьогодні") and MESH-12's ("зараз лише-тести-enroll",
> "ADR-0007 … unbuilt") are stale. BUILT on the live tree (`bebop2/proto-cap/`): `RevocationSet` +
> `merge` anti-entropy union + `gossip_payload` sorted wire form + `drop_anchor`
> (`revocation.rs:49,94,105,114`); H2 verify-then-record fix with RED test
> (`hybrid_gate.rs:188-206,571`); `load_genesis` fail-closed (`node_id.rs:116`);
> `NodeId::from_keys = SHA3-256(pq_pub‖classical_pub)` (`node_id.rs:46`). GENUINELY still open:
> mesh-wide propagation *guarantees* (convergence/delivery bounds beyond anti-entropy union), and
> MESH-12's HUMAN gate — `RootDelegationPolicy` defaults to `Unspecified` and fails closed until the
> operator chooses (`node_id.rs:157-183`), so the genesis root-delegation decision remains the
> operator's.

### MESH-12 · node_id self-cert + genesis-loader-prod + roster bootstrap
- **Мета:** self-certifying node-identity (ADR-0007) + production roster-genesis (зараз лише-тести-enroll).
- **Межа:** ЧІПАЄМО — node_id-derivation + prod-genesis-loader. НЕ ЧІПАЄМО — verify_chain (reuse).
- **Форма:** `node_id = H(pq_pub‖classical_pub)` (ADR-0007, no-CA, SPKI-lineage) — fixes seeded-owner-JWT (identity-
  born-from-keygen, nothing-to-seed). Prod genesis-loader читає frozen-anchor-set з config/disk (не-inline-tests).
  New-node-join = out-of-band-root-delegation (**HUMAN-decision:** operator-signed-root vs WoT vs first-contact-QR) +
  bulk-pull-actor_seq=0 (MESH-07).
- **Reuse:** roster.rs, sign.rs, hash.rs. **RED:** node_id recomputed-from-both-pubkeys matches; empty-roster fail-
  closed (no-capture); seeded-owner-fixture cannot-mint (nothing-to-seed). **Хвиля:** G9. 🔴 · HUMAN genesis-decision.

---

## Шар F — Крипто-хардненг (G10)

### MESH-13 · ML-KEM-768 FIPS-203-interop + external-KAT + constant-time + zeroize; proto-crypto real
- **Мета:** ML-KEM = єдина крипто-діра (ML-DSA вже FIPS-exact).
- **Межа:** ЧІПАЄМО — pq_kem.rs NTT-domain + KAT-harness + proto-crypto. НЕ ЧІПАЄМО — ML-DSA-65 (уже bit-exact).
- **Форма:** ML-KEM-768 → **FIPS-203-interoperable** (NTT-domain, не coefficient-domain — revert-of-the-revert з
  правильним NTT) + **external ACVP-KEM-vectors** (сьогодні нема KEM-KAT, лише DSA) + **constant-time** (no-secret-
  dependent-branch/continue, no-var-time-%, CT-ciphertext-compare-in-decaps) + **zeroize** (grep=0-hits сьогодні).
  proto-crypto skeleton(Placeholder)→real (wycheproof-vectors + constant-time-assertion-harness).
- **Reuse:** ML-DSA ACVP-methodology (dual-impl-cross-check). **RED:** ml_kem_external_ACVP_KAT_bit_exact (interop з
  reference-impl); ml_kem_constant_time (no-secret-branch, dudect-style). **Хвиля:** G10. 🔴 crypto.

---

## Шар-крос — RED + суперечності (G-RED)

### MESH-14 · Resolve-contradictions + RED-suite + status-from-live-test CI-lint
- **Мета:** прибрати stale/суперечливе + один RED-gate + правило «статус тільки з live-test».
- **Межа:** ЧІПАЄМО — docs/ADR-узгодження + test-крейт + CI-lint. НЕ ЧІПАЄМО — продакшн.
- **Форма:** (1) D3-DTN/BPv7 vs built-WSS → reconcile-note (iroh=QUIC-CL-under-BPv7, G6-будує-locked-stack). (2)
  MIGRATION-PLAN-diagram → додати dowiz-kernel-reuse. (3) ratify ADR-0007 + ADR-0008-update(SQLite→pgrust). (4)
  bebop2-root-workspace-Cargo.toml + fix-README-phantom-dirs(kernel/cli/reloop). (5) **CI-lint: будь-яка docs
  "CLOSED"-заява мусить цитувати matching live-path-test-name** (red-team-docs-were-stale lesson — derive-status-from-
  live-test-not-prose). RED-suite = усі контракти §6 плану.
- **RED:** кожен рядок reachable red→green, regression-ledger-row, нуль expect(true)/skip; CLOSED-claim-без-live-test
  → CI-red. **Хвиля:** G-RED. 🔴

---

## Зведення: блюпринт → шар → хвиля

| BP | Назва | Шар | Хвиля | Red-line |
|---|---|---|---|---|
| MESH-01 | bebop-delivery-domain (kernel-rlib) | A | G0 | — |
| MESH-02 | KernelFacade compiled-gate-chain | A/D | G1 | 🔴 |
| MESH-03 | Delivery event-словник + choreography | B | G2 | 🔴 |
| MESH-04 | claim_machine.rs | B | G2 | — |
| MESH-05 | Matcher assignment + NO-SCORING-CI | B | G3 | — |
| MESH-06 | pgrust-per-node + event-log | E | G4 | 🔴 |
| MESH-07 | Sync·Pull + Merkle catch-up | E | G5 | 🔴 |
| MESH-08 | CRDT-periphery compile-fence | E | G5 | — |
| MESH-09 | iroh-QUIC + BPv7 overlay | C | G6 | 🔴 |
| MESH-10 | WSS-rustls-TLS + B3-fixes + PQ-payload | C | G7 | 🔴 |
| MESH-11 | Revocation + drop-anchor + H2-fix | D | G8 | 🔴 |
| MESH-12 | node_id self-cert + genesis-loader | D | G9 | 🔴·HUMAN |
| MESH-13 | ML-KEM FIPS-interop+KAT+CT+zeroize | F | G10 | 🔴 |
| MESH-14 | Contradictions + RED + live-test-lint | всі | G-RED | 🔴 |

**Інваріант усіх 14:** reuse наявне (~90% готове); кожна подія wire→Law→money-ПЕРЕД-станом (KernelFacade-compiled);
event-sourcing-НЕ-CRDT для money/orders; NO-COURIER-SCORING-механічний; статус тільки з live-test (не з прози);
transport=DTN/BPv7-per-D3 (iroh-QUIC-під-BPv7); revocation=найбільша-діра; ML-KEM=єдина-крипто-діра; pgrust-per-node-
compat-PASSED.
