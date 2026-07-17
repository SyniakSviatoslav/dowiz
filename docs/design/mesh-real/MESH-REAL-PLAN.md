# bebop2 mesh REAL + per-node-local-first REAL — PLAN

> **Дата:** 2026-07-13 · **Тип:** дослідження → аналіз → синтез → план з блюпринтами (НЕ код) ·
> дев'ята задача сьогодні. Збережено в репо + Telegram.
> **Мандат (оператор, non-negotiable MVP):** зробити **bebop2-меш РЕАЛЬНИМ** і **per-node-local-first
> РЕАЛЬНИМ**. Не single-node-хаб-заглушка. pgrust per-node.
> **Стоїть на:** [[ecosystem-strategy-arc-2026-07-13]] (3 кити, patterns) + [[ops-reliability-arc-2026-07-13]]
> (pgrust-рішення) + [[integration-ports-reactive-arc-2026-07-13]] (capability-порти) + kernel/bebop2.
> **5 паралельних дослідницьких смуг** (inventory·transport·delivery-domain·local-first·authz) + **виконаний
> pgrust COMPAT-GATE**.

---

## 0. Головна теза + два відкриття, що змінюють картину

**Теза:** меш стає реальним НЕ переписуванням з нуля, а **з'єднанням шматків, що вже існують**, + добудовою
чотирьох відсутніх шарів (транспорт, delivery-словник, per-node-store, доведення authz). ~90% важкої логіки
(крипта, capability, order-domain, matcher) **уже написано й протестовано**.

**Відкриття 1 (compat-gate ВИКОНАНО, ПРОЙШОВ):** pgrust 18.3 (docker `malisper/pgrust:v0.1`) — `CREATE EXTENSION
citext` ✅ · `pgcrypto` ✅ · bcrypt/gen_random_uuid/digest/citext-match усі ✅; образ везе **повний contrib-набір**.
README «extensions not compatible» застарілий. → **per-node pgrust-store де-ризиковано**, дамп restore-able.

**Відкриття 2 (red-team B2/B3/B4 частково ЗАСТАРІЛІ):** ключова діра «node captured» (self-issued key проходить)
**УЖЕ ЗАКРИТА** на HEAD 898c888 — `verify_chain` викликається на acceptance-path (`hybrid_gate.rs:110`,
`wss_transport recv:187-188` real-clock), self-issued-key REJECTED live (33/33 + 13/13 green), envelope-version
enforced, nonce-set bounded. IMPROVEMENT-PLAN §0 це фіксує. → **фундамент authz реальний і живий**; лишилися
конкретніші прогалини (revocation, KernelFacade, genesis-loader, H2-ordering).

**Що це означає:** «меш не реальний» — надто песимістичне з застарілих доків. Реальна картина: **міцне ядро існує,
відсутні 4 несучі шари.** План = добудувати їх, максимально reuse.

---

## 1. Поточна реальність (REAL / STUB / MISSING / DECIDED) — cargo-test-verified

| Шар | REAL (працює) | STUB / MISSING |
|---|---|---|
| **Крипта** (core/) | Ed25519, **ML-DSA-65 FIPS-204-bit-exact (ACVP 60/60)**, XChaCha20-Poly1305, Argon2id, SHA-512/3, ChaCha20 fail-closed. 180/180 green | proto-crypto=100%-Placeholder-skeleton; **ML-KEM-768 non-FIPS-203-interop** (coefficient-domain), no-external-KAT, timing-leaks, no-zeroize |
| **Capability/authz** (proto-cap) | capability/scope/tlv/roster/hybrid_gate/signed_frame REAL + **WIRED на acceptance-path** (self-issued REJECTED live, 33/33) | **revocation НЕ існує** (лише expiry); genesis-loader-prod-unbuilt; H2 insert-before-verify; KernelFacade-unbuilt |
| **Транспорт** (proto-wire) | envelope/framing REAL (version-enforced), wss_transport REAL (12/12) | **PLAINTEXT ws://** (TLS-off); **iroh 100%-stub** → WSS=єдиний carrier (n=1, НЕ p2p); channel-binding decorative |
| **Delivery-domain** | dowiz-kernel order_machine/money/domain/analytics/intake REAL+tested; matcher.rs deterministic-replicable | **у bebop2 — ZERO** (scope=4×3 enum-labels, нема order/price/PoD/settlement); **dowiz-kernel↔bebop2 wiring=ZERO** |
| **Per-node store** | pgrust compat-PASSED (hub-interim) | per-node-pgrust+event-log+sync UNBUILT (ADR-0008 PROPOSED) |
| **Identity** | — | node_id=H(pq_pub‖classical_pub) ADR-0007 PROPOSED-unbuilt |

> ⚠ CORRECTED (2026-07-17, agentic-mesh consolidation, live-verified): the Capability/authz and
> Identity STUB claims above are stale — since 2026-07-13 the following are BUILT on the live tree
> (`/root/bebop-repo/bebop2/proto-cap/`): **RevocationSet** with `merge`/`drop_anchor`/`gossip_payload`
> (`revocation.rs:49,94,105,114`); **H2 fix** — verify-then-record nonce ordering with RED property
> test (`hybrid_gate.rs:188-206,571`); **genesis loader** fail-closed (`load_genesis`,
> `node_id.rs:116`); **KernelFacade** (`facade.rs:64-96`, `submit_intent`); **node_id =
> H(pq_pub‖classical_pub)** (`node_id.rs:46`, ADR-0007 realized). §8's "revocation-propagation =
> найбільший невирішений" is PARTIALLY resolved: the set + anti-entropy union + sorted gossip payload
> exist; what remains open is propagation *guarantees* (delivery/convergence bounds), not the
> mechanism. The agentic-mesh arc (docs/design/agentic-mesh-protocol-2026-07-17/) builds ON these as
> existing primitives.

**DECIDED (MANIFESTO/DECISIONS 2026-07-12 > 07-11 blueprints):** decentralized/local-first/PQ/mesh non-negotiable
(D6 mesh-NOW); transport=**DTN/BPv7** (D3-locked, reject libp2p/Zenoh); PQ=composed-protocol; central-server DROPPED;
integer-money; reliability>latency store-and-forward. ⚠️ Суперечність: D3 каже DTN/BPv7, а збудовано WSS+iroh-stub.

---

## 2. План = 6 шарів (максимальний reuse)

### Шар A — Per-node авторитетне ядро (reuse dowiz-kernel як decider)
Кожен нод вбудовує **той самий `dowiz-kernel`** (order_machine decide/fold + i64-money + domain) як **локальний
авторитетний state-machine**, лінкований як plain-Rust **rlib** у новий crate `bebop-delivery-domain` (пряме native-
лінкування, БЕЗ WASM/JSON-hop; Cargo-feature-split ховає wasm-bindgen/serde за opt-in). Кожен нод валідує КОЖНУ
подію ЛОКАЛЬНО через `assert_transition` — «central-kernel»→«replicated-identical-kernel», ніколи не довіряє
відправнику. **KernelFacade** (IP-01) робить шлях wire→Law→money **компільованим інваріантом** (адаптер, що імпортує
kernel напряму = build-fails).

### Шар B — Delivery-словник (SignedFrame-події + claim_machine + matcher)
Delivery-lifecycle = event-sourced **choreography** (no orchestrator): кожен крок = `SignedFrame{Capability{Resource,
Action}, payload}`, валідований локально: `OrderPlaced`/`OrderStatusChanged`/`ClaimOffered`/`ClaimAccepted`/
`ClaimReleased`(compensation-first-class)/`Pickup`+`DeliveryConfirmed`(device-signed)/`SettlementRecorded`(i64-money).
Claim-гонки самовирішуються (losing-ClaimAccepted fails assert_transition→стає ClaimReleased, zero-arbiter). Matcher
(matcher.rs) вже доводить потрібну властивість (pure-fn, fingerprint-cross-node-agreement, fail-closed-unmatched); GAP
= розширити routing→**assignment** (N-candidate-couriers, canonical tie-break = rendezvous/HRW-hash(order_id,courier_
pubkey)). **NO-COURIER-SCORING** з doc-convention → **механічний CI-grep** (score/rating/trust/reputation/rank).
Нове (тонке): claim_machine.rs, matcher-tie-break, Resource::{Order,Claim}, Cargo-feature-split, SettlementLedger-
reducer, TLV-payload-spec, CI-grep-gate.

### Шар C — Транспорт реальний (iroh+BPv7, DTN/BPv7 per D3)
Шаром: **BPv7 store-forward overlay** (`bp7-rs` = RFC9171-CBOR-codec ТІЛЬКИ + hand-roll custody/retry/expiry —
НЕ вбудовувати dtn7-rs-daemon: дублює HybridGate + routing-metrics-ризик-NO-COURIER-SCORING) **над iroh-QUIC
convergence-layer** (iroh 1.0 GA Jun2026, quinn-backed, ~90%-hole-punch+relay, dial-by-pubkey, ALPN «bebop2/wire/1»;
**iroh = D3's-missing-NAT-traversal НЕ конфлікт**; Envelope/framing/Transport-trait UNCHANGED) **над WSS-fallback**
(tokio-tungstenite + **tokio-rustls** pure-Rust, browser/edge). **+ML-KEM-768→XChaCha20-Poly1305 payload-encryption**
(defense-in-depth past-semi-trusted-relay). Custody=extension-block keyed-by-Capability.nonce; retransmit-until-ack
на iroh-bidi-stream; offline-reconnect=persist-undelivered-queue drain-oldest-first FRESH-channel-binding-each-replay.
**Фікс B3:** WSS→rustls-TLS; channel_binding з real-TLS/QUIC-exporter (RFC5705); HybridGate.seen node-scoped-persistent
(не per-conn); WebSocketConfig-8MiB+read-timeout+conn-cap+per-IP-rate-limit (iroh quinn::TransportConfig absorbs-
Slowloris-natively).

### Шар D — Authz/identity доведення (B2-ядро вже fixed)
Acceptance-path = **3 ворота фіксованого порядку, все ПЕРЕД станом**: WIRE(HybridGate: replay/expiry→verify_chain→
Ed25519→ML-DSA-65-RequireBoth) → LAW(assert_transition) → MONEY(i64). WIRE вже real+wired; LAW/MONEY у kernel — **потрібен
KernelFacade** щоб зробити ланцюг компільованим. **node_id=H(pq_pub‖classical_pub)** (ADR-0007, unbuilt) = fixes
seeded-owner-JWT (identity-born-from-keygen). **★Revocation НЕ існує** (лише expiry) → побудувати **RevocationSet +
drop-anchor + mesh-propagation** (найбільший open-item, matches Vouchsafe/Lingering-Authority research). Genesis-
loader-prod (зараз лише-тести-enroll). Fix H2 insert-before-verify (verify-THEN-record).

### Шар E — Sync (pull-anti-entropy + Merkle catch-up)
pgrust-per-node = store + event-log-table + read-projections (той самий instance). Local-writes→kernel::decide→commit-
event-log BEFORE-network (offline = «sync-hasn't-run» never-degraded-write). **Event-ID=hash(prev,actor_pubkey,actor_
seq,payload)=idempotency** (log-own-dedup-no-TTL, makes-at-least-once-gossip-safe). **Sync=pull-anti-entropy** (peer-
requests-after-last-actor_seq, folds-locally, dup=no-op); **long-offline catch-up = Merkle/prolly-tree-digest** (Dolt-
content-defined-chunking, diff-root-hashes, pull-divergent-chunks). Frame rides SignedFrame gated NEW **Sync·Pull**-
scope. **★event-sourcing NOT CRDT для money/orders** (CRDT-convergence≠legal-transitions; replicated-state-machine).
CRDT лише-периферія (notes/tags/presence) + **compile-fence** (build-fails-if-order/money-crate-depends-CRDT-crate).
Bootstrap=AnchorRoster + bulk-pull-actor_seq=0-fold-before-live-gossip (same-mechanism, no-separate-protocol).
★NO-2026-sync-engine-adoptable (all single-Postgres-source OR CRDT-merge; Zero=no-offline-writes) → reuse-pattern.

### Шар F — Крипто-хардненг (ML-KEM = єдина крипто-діра)
ML-DSA-65 вже FIPS-exact. **ML-KEM-768 → FIPS-203-interop** (NTT-domain-not-coefficient) + **external-KAT** (ACVP-KEM-
vectors) + **constant-time** (no-secret-branch/var-time-%) + **zeroize**. proto-crypto skeleton→real (wycheproof-vectors
+ constant-time-harness).

---

## 3. pgrust COMPAT-GATE — результат (виконано)

`docker malisper/pgrust:v0.1` = pgrust 18.3. `CREATE EXTENSION citext`✅ `pgcrypto`✅; citext-case-insensitive→1✅;
`gen_random_uuid()`✅; `digest(...,'sha256')`✅; `crypt('pw',gen_salt('bf'))`→`$2a$06$`✅ (couriers.password_hash).
`pg_available_extensions`: citext/pgcrypto/hstore/ltree/pg_trgm/uuid-ossp/postgres_fdw + 40. **Висновок:** дамп
Supabase restore-able у pgrust; per-node pgrust-store життєздатний. Наступний крок (за бажанням): повний test-restore
дампу (потребує pg_restore 17/18 — не в pgrust-образі; через postgres:17-контейнер до відкритого порту).

---

## 4. Reuse-first (що НЕ будуємо)

Reuse UNMODIFIED: dowiz-kernel {order_machine, money, domain} + analytics-pattern; bebop2 {SignedFrame, Capability,
HybridGate, AnchorRoster, verify_chain, Transport-trait, envelope, framing}; matcher.rs {MatcherRequest/Response,
fingerprint, replicability}; core-crypto {Ed25519, ML-DSA-65, XChaCha20, Argon2id, SHA3, entropy}. Це ~90% важкої
логіки. Нове = тонкі шви (wiring, claim_machine, transport-carrier, revocation, sync, ML-KEM-fix).

---

## 5. Хвилі (foundation-first, reuse-first; кожна RED red→green)

| Хвиля | Шар | Що | Reuse | Ризик |
|---|---|---|---|---|
| **G0** | A | `bebop-delivery-domain` crate: link dowiz-kernel-rlib + Cargo-feature-split (wasm/serde opt-in) | dowiz-kernel | середній |
| **G1** | A/D | **KernelFacade** (wire→Law→money компільований, adapter-imports-kernel=build-fails) | IP-01, gates | 🔴 |
| **G2** | B | Delivery-словник: Resource::{Order,Claim}, event-frames, claim_machine.rs, choreography | order_machine, SignedFrame | 🔴 domain |
| **G3** | B | Matcher assignment: N-candidate + rendezvous-hash tie-break + NO-COURIER-SCORING CI-grep | matcher.rs | середній |
| **G4** | E | pgrust-per-node + content-addressed event-log + actor_seq-index | pgrust(compat✓), kernel | 🔴 data |
| **G5** | E | Sync·Pull port: pull-anti-entropy + Merkle/prolly catch-up + CRDT-periphery compile-fence | SignedFrame, EC-12/15 | 🔴 |
| **G6** | C | Transport real: iroh-QUIC carrier + BPv7 overlay (bp7-rs) + custody/retry + offline-reconnect | Transport-trait, envelope | 🔴 |
| **G7** | C | WSS→rustls-TLS + channel-binding-exporter + rate-limit/timeout/conn-cap (fix B3) + ML-KEM→XChaCha payload-enc | core-crypto | 🔴 |
| **G8** | D | Revocation: RevocationSet + drop-anchor + mesh-propagation + genesis-loader-prod + H2-fix | AnchorRoster | 🔴 authz |
| **G9** | D | node_id=H(pq_pub‖classical_pub) ADR-0007 + roster-genesis bootstrap (out-of-band root) | roster.rs | 🔴 · HUMAN |
| **G10** | F | ML-KEM-768 FIPS-203-interop + external-ACVP-KAT + constant-time + zeroize; proto-crypto real | pq_kem | 🔴 crypto |
| **G-RED** | всі | RED red→green + resolve-contradictions | — | обов'язковий |

---

## 6. RED-контракти (кожен reachable red→green)

- **same_event_same_decision_across_nodes** (2 kernel-instances, identical frame → byte-identical Order).
- **illegal_transition_rejected_locally** (forged Pending→Delivered rejected on EVERY receiver, not just sender).
- **self_issued_key_rejected** (EXISTS-GREEN `wss_rejects_self_signed_frame_over_real_carrier`).
- **attenuation_cannot_widen** (EXISTS-GREEN `red_effect_not_subset_of_tail_scope`).
- **revoked_capability_stops_verifying** (DOESN'T-EXIST → write FIRST vs RevocationSet before KernelFacade).
- **money_integer_never_float** + **no_courier_scoring_ci_gate** (build-fails on score-field / float-on-money).
- **offline_node_rejoins_converges_identical** + **duplicate_event_no_op** + **illegal_transition_rejected_on_sync**.
- **money_never_CRDT_merged** (build-fails if order/money-crate depends CRDT-merge-crate).
- **plaintext_ws_rejected_when_tls_required** + **cross_connection_replay_rejected** + **offline_courier_reconnect_
  delivers_exactly_once** + **slowloris_stalled_dropped_by_idle_timeout**.
- **ml_kem_external_ACVP_KAT_bit_exact** + **ml_kem_constant_time** (no secret-dependent branch).
- **CI-lint:** any docs "CLOSED" claim must cite a matching live-path test name (red-team-docs-were-stale lesson).

---

## 7. Суперечності до розв'язання

1. **D3 (DTN/BPv7 locked) vs built (WSS + iroh-stub)** → reconcile: iroh = QUIC-convergence-layer UNDER BPv7-overlay;
   build the locked stack (G6). Не «WSS назавжди».
2. **MIGRATION-PLAN diagram OMITS dowiz-kernel** (the reusable delivery-domain) → update diagram to show reuse (G0).
3. **ADR-0007 + ADR-0008 = PROPOSED** → ratify; ADR-0008 update SQLite→pgrust (red-line-ADR).
4. **red-team-docs STALE vs live-code** → add CI-lint (CLOSED-claim ⇒ live-path-test); derive-status-from-live-test.
5. **bebop2 no-root-workspace + README-phantom-dirs (kernel/cli/reloop)** → fix layout/README to match reality.

---

## 8. Найбільші ризики

- **Revocation-propagation** (Шар D, G8) — найбільший невирішений; drop-anchor тривіальний локально, але mesh-wide-
  поширення потребує gossip/consensus-історії (2026-research-open). Це справжня межа дослідження, не тюнінг.
- **ML-KEM-768 FIPS-interop** (G10) — переробка NTT-domain + external-KAT — крипто-red-line, потрібна максимальна
  строгість (bit-exact ACVP).
- **Transport-lock суперечність** (G6/G7) — будуємо DTN/BPv7-per-D3, а не «залишаємо WSS»; iroh 1.0-GA робить це
  здійсненним, але це найбільший новий шматок.
- **Genesis-trust-anchor** (G9) — відкрите HUMAN-рішення (operator-signed-root vs WoT vs first-contact-QR); блокує
  вихід за межі single-test-node.
- **Stale-docs-drift** — red-team-доки описували transient-стан; правило: статус тільки з live-test, не з прози.

---

## 9. Résumé одним абзацом

Меш стає реальним з'єднанням наявного (крипта 180/180, capability-wired-live, dowiz-kernel-order-domain, deterministic-
matcher = ~90% важкої логіки), а не переписуванням. Чотири відсутні несучі шари: (A/B) reuse dowiz-kernel як per-node-
decider + delivery-словник SignedFrame-подій з choreography + claim_machine + matcher-assignment; (C) реальний транспорт
iroh-QUIC-під-BPv7-overlay (D3) + WSS-rustls-TLS + ML-KEM→XChaCha-payload-enc + B3-фікси; (D) доведення authz — KernelFacade-
компільований-ланцюг + node_id-self-cert + **revocation (найбільша діра)** + genesis-loader + H2-fix; (E) per-node-pgrust
(compat-PASSED) + content-addressed-event-log + pull-anti-entropy-sync + Merkle-catch-up + CRDT-лише-периферія-compile-
fenced (event-sourcing-NOT-CRDT-для-грошей); (F) ML-KEM-FIPS-interop+KAT+CT+zeroize. Reuse-first, RED red→green, статус
лише-з-live-test. Найбільші ризики: revocation-propagation, ML-KEM-interop, DTN/BPv7-транспорт, genesis-anchor-human-gate.
