# Lens B — Data & Sync Architecture for the Local-First Decentralized Hub

> Research report, 2026-07-11. Frame (binding): collapse dowiz's two half-hubs into ONE
> local-first hub running on the participants' devices (Rust/WASM kernel + bebop2 protocol +
> SQLite), no single central server, dropping the 162 Postgres migrations + Supabase/RLS +
> Node/TS + Fly.
>
> Evidence labels: **VERIFIED** = read from a local file this session (cited `file:line`) or a
> primary web source (vendor docs / repo / spec, cited URL). **UNVERIFIED** = secondary source,
> vendor marketing claim, or inference — stated as such.
>
> Local ground truth read this session: the v3 blueprint
> (`/root/bebop-repo/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`),
> `bebop-repo/docs/features/{memory,mesh}.md`, `docs/integrations/{sync,backends}.md`,
> `crates/bebop/src/{ledger,zenoh,matcher}.rs`; dowiz `packages/db/migrations` (162 files),
> the hub review §3 (`/root/dowiz/docs/research/2026-07-11-hub-architecture-review.md`),
> `rebuild/crates/domain/src/{kernel.rs,money.rs,order_status.rs}`; and the living-memory corpus
> (`/root/.claude/projects/-root-dowiz/memory/`): `MEMORY.md`, `pg-privilege-hardening-2026-06-29.md`,
> `b3-auth-hardening-council-2026-07-03.md`, `crypto-payments-build-2026-06-30.md`,
> `rebuild-decision-rust-astro-2026-07-04.md`, `sovereign-core-mvp-handoff-2026-07-06.md`,
> `migration-blueprint-2026-06-25.md`.

---

## 0. What the current schema actually is (the thing being replaced) — VERIFIED

Counted this session directly from `/root/dowiz/packages/db/migrations` (162 files):

| Artifact | Count | Fate under per-device SQLite |
|---|---|---|
| `CREATE TABLE` (unique names) | **84** (REBUILD-MAP census says 86 incl. 2 out-of-band manual — memory `rebuild-decision-rust-astro-2026-07-04.md`) | Re-partitioned into per-device projections + event logs (§2) |
| `CREATE POLICY` (RLS) | **123** | Deleted; replaced by capabilities + E2EE (§1.3) |
| `ENABLE/FORCE ROW LEVEL SECURITY` | **76** occurrences | Deleted |
| `CREATE FUNCTION` (incl. SECURITY DEFINER) | **63** | Logic moves into the Rust kernel or dies |
| `CREATE TRIGGER` | **15** | Kernel `decide`/`fold` replaces them |
| pg-boss queue tables (`pgboss.*`) | schema-owned | Dies; local scheduler replaces it |

Two facts from the memory corpus that change the honest accounting of what RLS is worth today:

1. **The 123 policies are mostly NOT the live enforcement.** The operational role `dowiz_app`
   carries `BYPASSRLS`, so "FORCE RLS is a live no-op until MIG-ITEM2; isolation = app WHERE
   clauses only" (memory `pg-privilege-hardening-2026-06-29.md`, GUC-coverage audit 2026-06-30;
   confirmed by `b3-auth-hardening-council-2026-07-03.md`: the NOBYPASSRLS flip is "NOT safely
   shippable — DEFERRED behind an 8-item pre-flip program"; ~123 raw `.db.query` sites bypass
   the `withTenant` seam). The flip was attempted once on staging and cleanly reverted
   (membership-bootstrap chicken-and-egg). So dropping RLS drops less real protection than the
   number 123 suggests — the *real* invariants live elsewhere (see below). VERIFIED.
2. **The invariants any replacement datastore MUST preserve** (these are the schema's soul, not
   the policies):
   - **Integer-only money**, `Lek(i64)`, no `From<f64>`, checked arithmetic
     (`rebuild/crates/domain/src/money.rs:27-58`; DECISIONS D-money; blueprint C5). VERIFIED.
   - **The byte-frozen 10-status order machine** with exact transition relation and error
     classes (`rebuild/crates/domain/src/order_status.rs:19-70`; kept byte-identical across the
     Node→Rust port per hub review §3.2). VERIFIED.
   - **`decide`/`fold` event-sourcing law**: `decide(&OrderState, Command, &Context) →
     Vec<Event>`, `state = fold(events)`, caller-supplied `Ts` (core never reads a clock),
     `CommandHash` cause chain, `Envelope{seq, at, cause, event}`
     (`rebuild/crates/domain/src/kernel.rs:1-60,214-220`). VERIFIED.
   - **Idempotency as a pure decision** (`kernel/idempotency.rs`, tenant-scoped
     `(location_id, key)` uniqueness — hub review §3.1 item 3). VERIFIED.
   - **Payment monotonicity**: `payment_events` insert-wins ledger, `UNIQUE(provider,
     provider_payment_id, type)`, residual guard `refunded<=captured<=amount`, single writer of
     `payment_status=paid` (memory `crypto-payments-build-2026-06-30.md`). VERIFIED.
   - **Forward-only migrations** discipline (memory `migration-blueprint-2026-06-25.md`,
     `rebuild-decision-rust-astro-2026-07-04.md`: "schema kept, data never migrates"). The
     *discipline* (append-only, never rewrite history) survives as the event log itself.

---

## 1. SQLite as the node datastore — options and 2026 reality

### 1.1 Options table

| Option | What it is | 2026 status | Fit for this hub | Evidence |
|---|---|---|---|---|
| **rusqlite** (native Rust) | Bindings to canonical SQLite, in-process | Mature, de-facto standard for Rust+SQLite; boring in the good way | **Recommended** for vendor/courier native nodes (Tauri/mobile). Kernel stays pure; SQLite is the shell's port | UNVERIFIED this session (no fresh check needed — stable for a decade) |
| **Official SQLite-WASM + OPFS** | sqlite.org's own WASM build, OPFS VFS persistence | Working; OPFS sync-access requires a Web Worker + COOP/COEP headers; Safari <17 broken on the default OPFS VFS (SAHPool VFS works ≥16.4) | Good for the **browser customer node** — but as a *cache*, not an authority (see durability, §1.2) | VERIFIED: [sqlite.org persistence doc](https://sqlite.org/wasm/doc/trunk/persistence.md), [Chrome dev blog](https://developer.chrome.com/blog/sqlite-wasm-in-the-browser-backed-by-the-origin-private-file-system) |
| **wa-sqlite** | Third-party WASM SQLite with pluggable JS VFSes (IndexedDB, OPFS variants) | **Actively maintained**: SQLite 3.50.x; new `OPFSWriteAheadVFS` added April 2026 (concurrent reads, Chrome-only access mode); 2026-03 testing sustains 8–10 concurrent workers with SQLITE_BUSY handling | Viable alternative browser VFS layer; more VFS flexibility than official build | VERIFIED: [wa-sqlite repo](https://github.com/rhashimoto/wa-sqlite), [PowerSync "State of SQLite persistence on the web", May 2026](https://powersync.com/blog/sqlite-persistence-on-the-web) |
| **sql.js** | WASM SQLite, **in-memory only**, manual export/import | Alive but superseded for persistence use-cases | Not fit (no persistence) | VERIFIED (secondary): [browser storage comparison 2026](https://recca0120.github.io/en/2026/03/06/browser-storage-comparison/) |
| **libSQL** (SQLite fork) | Turso's open-contribution SQLite fork, embedded replicas | Production-ready | Fine as a rusqlite substitute; its *sync* is server-anchored (Turso cloud/self-host) — re-centralizes | VERIFIED: [libsql repo](https://github.com/tursodatabase/libsql) |
| **Turso Database** (Rust rewrite, ex-Limbo) | Ground-up SQLite rewrite in Rust: MVCC `BEGIN CONCURRENT`, async I/O, CDC | **Beta, not production** ("libSQL is production ready, Turso Database is not"); Offline Sync in **public beta, no durability guarantees**, TS+Rust SDKs | Watch-list. The CDC + Rust-native + WASM trajectory is exactly this architecture's shape, but not shippable 2026-07 | VERIFIED: [turso repo](https://github.com/tursodatabase/turso), [offline-sync beta](https://turso.tech/blog/turso-offline-sync-public-beta) |
| **cr-sqlite / vlcn** | CRDT extension for SQLite (CRRs, multi-writer merge) | **Effectively dormant**: last release v0.16.3 **Jan 2024**, ~30 months before today; no archive notice but no releases either | **Do not build on it.** The idea (CRRs) is right for the commutative subset; the implementation is unmaintained | VERIFIED: [cr-sqlite releases](https://github.com/vlcn-io/cr-sqlite) (v0.16.3, 2024-01-17) |

### 1.2 The browser-persistence honesty box (OPFS durability/quota, 2026)

- Quota: OPFS/IndexedDB share origin quota; Chrome grants up to ~80% of disk but starts
  "best-effort" — data **can be evicted under storage pressure** unless
  `navigator.storage.persist()` is granted. VERIFIED:
  [MDN storage quotas & eviction](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria),
  [web.dev storage](https://web.dev/articles/storage-for-the-web).
- **Safari deletes all script-writable storage (including OPFS) after 7 days of no interaction
  with the site** (ITP policy; does not apply to home-screen-installed PWAs). VERIFIED:
  [WebKit storage policy](https://webkit.org/blog/14403/updates-to-storage-policy/),
  [Didomi summary of the 7-day cap](https://support.didomi.io/apple-adds-a-7-day-cap-on-all-script-writable-storage).
- Consequence (architectural, load-bearing): **a browser customer node is an ephemeral cache,
  never an authoritative replica.** The customer's canonical record of an order must be a
  signed receipt (vendor-hub-signed event) that can be re-fetched/re-verified, not local rows.
  Vendor/courier nodes must be native apps (rusqlite) or installed PWAs with persistence
  granted.

### 1.3 What replaces Postgres RLS when the DB is local per-device

RLS answers "which *rows* may this *connection* see" on a **shared multi-tenant server**. In a
per-device world the DB has exactly one local user, so RLS's job decomposes into three
different mechanisms:

1. **Tenant isolation → physical partition.** Each venue's hub holds only its own data. The
   84-table shared schema's biggest isolation problem (cross-tenant leakage, the whole
   BYPASSRLS saga) disappears *by construction* — there is no other tenant in the file.
2. **Row/stream authorization → cryptographic capabilities on sync, tied to bebop identity.**
   Who may *subscribe to* or *write into* a replicated stream is decided by verifying
   signatures against the bebop2 self-certified identity (`id = H(pq_pub ‖ classical_pub)`, no
   issuer, no directory — blueprint §4, VERIFIED `UNIFIED-...-v3:93-98`). This is the
   local-first state of the art: **UCAN-style certificate capabilities**, and Ink & Switch's
   **Keyhive** ("local-first access control": convergent capabilities + BeeKEM continuous group
   key agreement; sync servers hold only ciphertext). VERIFIED:
   [Keyhive notebook](https://www.inkandswitch.com/keyhive/notebook/),
   [Beehive lab notebook](https://www.inkandswitch.com/beehive/notebook/). dowiz already has
   the capability-token pattern in embryo: `courier_invites` and the never-consulted 256-bit
   per-channel token (hub review §3.3) are capabilities wearing Postgres clothes.
3. **Confidentiality in transit/at relays → E2EE per stream.** Anything that transits a relay
   or another party's device is encrypted to the capability holders (bebop2 ML-KEM-768 +
   XChaCha20-Poly1305 at rest — blueprint §4). Evolu ships exactly this shape today (E2EE
   binary sync protocol; MIT stateless relay; "clients in a P2P setup" supported). VERIFIED:
   [Evolu relay docs](https://www.evolu.dev/docs/evolu-server).

What RLS did that capabilities do **not** give back for free: server-side *revocation* takes
effect instantly; capability revocation propagates only as fast as sync (Keyhive's open
problem too — UNVERIFIED nuance, flagged). And per the memory corpus, remember the honest
baseline: today's RLS is dormant under BYPASSRLS anyway (§0).

---

## 2. The commutative-vs-consensus partition — the key table

### 2.1 The theory anchor (why this partition is forced, not a style choice)

The CALM theorem: a program has a coordination-free (CRDT-able) implementation **iff it is
monotone**. Non-monotone decisions — ones where a later fact can *invalidate* an earlier
conclusion — require coordination. VERIFIED:
["Keep CALM and CRDT On", VLDB 2023](https://www.vldb.org/pvldb/vol16/p856-856-power.pdf)
([ACM](https://dl.acm.org/doi/abs/10.14778/3574245.3574268));
[Loro's own "When Not to Use CRDTs"](https://loro.dev/docs/concepts/when_not_crdt) (CRDTs merge
rather than reject — wrong for hard invariants like balance ≥ 0);
[Preguiça, CRDT overview](https://arxiv.org/pdf/1806.10254) (escrow/Bounded Counter: the *only*
CRDT trick for resource invariants is pre-partitioning the resource — i.e., smuggled-in
coordination).

Applied to this domain, the two poster cases:

- **"Order accepted by courier A" ∥ "order accepted by courier B" does not merge.** Dispatch
  assignment is mutual exclusion — non-monotone by definition. A CRDT would keep both facts;
  the business needs exactly one winner and a loser who *knows* it lost.
- **Money does not merge.** `Lek` refuses negatives and overflow (`money.rs`); the bebop ledger
  refuses imbalance (`Σ balances == 0`, insufficient funds → fail closed,
  `crates/bebop/src/ledger.rs:79-113` VERIFIED). "Fail closed" is a *rejection* — CRDTs cannot
  reject.

### 2.2 What bebop's own code already assumes about writer authority — VERIFIED

- **`ledger.rs` assumes a single sequential applier.** Transfers are idempotent by
  content-hash id (`transfer_id`, `ledger.rs:38-49`) — replay-safe — but application is
  in-process, ordered, and fail-closed on insufficient funds (`ledger.rs:105-107`). Two
  replicas independently applying different transfer orders can diverge on *which* transfer
  gets rejected. The file says it itself: "Honest: in-process only" (`ledger.rs:13`). The
  ledger is a **single-writer invariant kernel**, not a CRDT.
- **`matcher.rs` decentralizes the *computation*, not the *sequencing*.** `match_orders` is a
  pure deterministic function; `fingerprint` proves two nodes agree **given the same
  `MatcherRequest`** (`matcher.rs:9-15,74-100`). Who assembles the request — which orders and
  couriers are *in* the input set, in what order — is exactly the sequencing problem, and the
  matcher deliberately does not solve it. Same conclusion as the blueprint's DANGER #1
  analysis: the *sequencer* is the control point.
- **`zenoh.rs` is a seam, not a transport** — process-local pub/sub broker mirroring the
  Portkey envelope ("This is the seam, not the wire protocol", `zenoh.rs:1-11`). Real
  networking is unbuilt. Likewise `mesh.md`/`torrent.ts`: content-addressed verified exchange
  with an in-memory swarm; libp2p/hyperswarm are future implementations of the same port.
- **dowiz's kernel is already single-writer-per-order by construction**: `Envelope{seq, …}` is
  a totally-ordered per-order log; `decide` is called against *the* current state
  (`kernel.rs:1-12,214-220`). There is no merge function anywhere in the domain crate — and
  there shouldn't be.

**Conclusion: both codebases converge on the same writer-authority model — a totally ordered,
single-writer event log per stream, with idempotent replay.** The sync architecture must
preserve that, not fight it with CRDT merge.

### 2.3 The partition table — every dowiz entity (all 84 tables, grouped)

Class key: **A = CRDT-safe** (commutative/monotone; LWW-register or grow-only set, merge
freely) · **B = single-writer stream** (one signing authority per stream; others submit signed
*Commands*, authority emits signed *Events*) · **C = two-party/threshold signed** (money facts
requiring counterparty signatures — blueprint L3) · **D = dies / replaced by crypto** ·
**E = external-server-coupled** (cannot be local-first at all).

| dowiz entities (tables) | Class | Writer authority / merge rule | Why |
|---|---|---|---|
| `products, categories, modifiers, modifier_groups, product_modifier_groups, ingredients, recipe_components, product_media, product_translations, category_translations, content_i18n, menu_schedules, menu_versions, delivery_tiers, locations, location_themes, theme_versions, promotions` (definitions), `sales_channels` | **A** | Vendor-authored LWW per field; single natural author (the owner) so merge conflicts are rare/benign; version rows are content-addressed snapshots | Menu/catalog is presentation state — stale-read tolerable; monotone publish |
| `courier_locations, courier_positions, ops_heartbeat/ops_worker_heartbeat, location_alerts` (presence), `customer_signals, analytics_events, analytics_cwv, funnel_events, order_sensor_events, delivery_trace, velocity_events` (telemetry) | **A** | LWW (position/presence) or grow-only append set (telemetry); any-writer, eventual | Telemetry/presence is observational — no invariant to violate |
| `order_messages` | **A** | Per-order causal append-only log (each participant signs own messages) | Chat is commutative under causal order |
| `orders, order_items, order_item_modifiers, order_status_history, order_events, order_routes, order_ratings, recurring_orders, idempotency_keys, anonymous_orders` | **B — vendor hub is THE writer** | One signed event log per order, sequenced by the venue's hub key; customer/courier devices send signed `Command`s, hub's `decide` emits `Envelope{seq,cause,event}`; idempotency = kernel decision (`kernel/idempotency.rs`) | Order state is the 10-status machine — transitions reject (non-monotone); exactly what `decide` already is |
| `courier_assignments, courier_dispatch_queue, courier_shifts, reservations` | **B — vendor hub** | Dispatch = mutual exclusion: couriers submit signed CLAIM commands; hub's deterministic matcher (bebop `match_orders`) picks; losers get a signed refusal. Reservations = capacity, same shape | "Accepted by A" ∥ "accepted by B" cannot merge (§2.1); the open-matcher fingerprint keeps the hub *auditable*, killing DANGER #1 without leaderlessness |
| `payments, payment_events, courier_cash_ledger, courier_payouts, settlement_items, settlement_audit_log, money_breakdown` (order pricing fields) | **C** | Double-entry facts co-signed by the parties they bind (courier+owner ≥k-of-n on PoD — blueprint L3); applied via the conservation-checked ledger (`ledger.rs`), per-venue sequencing; disputes → fail-closed arbitration (L4) | Money is non-monotone AND adversarial; signatures replace the DB's trust, the ledger invariant replaces the CHECK constraints (`crypto-payments-build`'s residual-guard survives as kernel asserts) |
| `users, memberships, organizations, couriers, customers, customer_devices, courier_sessions, courier_invites, claim_invites, access_requests, api_keys, provision_grants, customer_track_grants, customer_contact_reveals` | **B/D** | Identity → bebop2 self-cert keys (no `users` table in the server sense); grants/invites → **capability certificates** (UCAN/Keyhive-style, §1.3) signed by the venue key; membership = a capability, not a row | The RLS-membership machinery (the whole bootstrap chicken-and-egg that broke the NOBYPASSRLS flip) dissolves into key possession |
| `auth_refresh_tokens, customer_otp_sessions, phone_otp, telegram_* (3 tables — excluded per frame anyway), domain_verifications` | **D** | Die. Sessions → device keys; OTP → key exchange/QR pairing | Server-auth artifacts |
| `gdpr_erasure_requests, anonymization_audit_log, courier_audit_log, notification_prefs_audit, upload_audit, backup_audit_log, backup_metadata, order_history` | **B** (per-venue) with a hard caveat | Erasure in a replicated E2EE system = **crypto-shredding** (destroy the stream key so ciphertext replicas rot) + tombstone events; audit logs are append-only per-venue | Honest flag: GDPR erasure across devices you don't control is an open research problem (Keyhive-adjacent); weaker than today's single-DB DELETE — must be designed, not assumed |
| `exchange_rates, free_tier_snapshots, acquisition_sources, webhook_endpoints, analytics_abuse_log, customer_signals` (scoring), `pgboss.*` | **E/D** | Exchange rates need an external oracle (fetch+cache, signed); webhooks/pgboss die (local scheduler + mesh pub/sub); cross-venue anti-abuse (velocity per phone/IP across tenants) **cannot exist without a shared view** — degrade to per-venue velocity + optional shared signed blocklist | Honest loss: today's `5/min per phone-or-IP` global rate limiting (hub review §3.1) is a central-choke-point feature |

**The one-sentence partition:** *menus, presence, telemetry, and chat merge; orders, dispatch,
and reservations are the vendor-key's totally-ordered log; money is co-signed double-entry;
identity is key possession; global anti-abuse and fiat oracles are the casualties.*

---

## 3. Sync approach — comparison and recommendation

### 3.1 Field survey (2026 liveness + serverless honesty)

| Approach / framework | 2026 status | Truly serverless? | Fit |
|---|---|---|---|
| **Event-sourcing + gossip** (dowiz `order_events` + bebop zenoh/mesh seam) | The domain kernel is built and byte-frozen (VERIFIED §2.2); transport is a seam, unbuilt | Yes (modulo §4 floor) | **RECOMMENDED** — it is the only approach that matches the writer-authority model both codebases already assume |
| **CRDTs — Automerge** | Alive: Automerge 3.0 (Aug 2025) cut memory >10× ([automerge.org](https://automerge.org/blog/automerge-3/) VERIFIED); Rust core, WASM; Beelay sync protocol + Keyhive auth incoming | Yes (peer sync; relays optional) | Good for the **class-A subset** (menu docs, presence) if a library is wanted; NOT for orders/money |
| **CRDTs — Yjs** | Production default, ~920K weekly downloads ([PkgPulse 2026](https://www.pkgpulse.com/guides/yjs-vs-automerge-vs-loro-crdt-libraries-2026) UNVERIFIED secondary) | Yes | JS-ecosystem-shaped; wrong language layer for the Rust kernel |
| **CRDTs — Loro** | Alive, fastest benchmarks, youngest ecosystem (same source; also [loro.dev](https://loro.dev/docs/concepts/when_not_crdt)) | Yes | Rust-native candidate for class A; small ecosystem risk |
| **cr-sqlite / vlcn** | **Dormant** — last release Jan 2024 (VERIFIED §1.1) | Yes | Rejected on maintenance |
| **ElectricSQL** | Alive but pivoted: read-path sync engine **for Postgres** (Elixir service reading the WAL) ([electric-sql.com alternatives](https://electric-sql.com/docs/reference/alternatives) VERIFIED) | **No — requires the central Postgres this frame is dropping** | Rejected by frame |
| **PowerSync** | Alive, enterprise-adopted; explicit architecture: source DB → **PowerSync Service** → client SQLite; explicitly rejects CRDTs in favor of "an authoritative server [enforcing] global ordering" ([docs](https://docs.powersync.com/architecture/powersync-service), [blog](https://powersync.com/blog/electricsql-electric-next-vs-powersync) VERIFIED) | **No** | Rejected by frame — but note it independently confirms this report's single-writer thesis |
| **Jazz** | Alive; local-first framework w/ built-in auth/permissions/E2EE (CoJSON); syncs via Jazz Cloud **or self-hosted sync server** | Sync-server-shaped (self-hostable) | JS/TS framework lock-in; wrong layer |
| **Evolu** | Alive; SQLite-in-browser + **E2EE binary sync protocol**; MIT stateless relay, multi-relay, "clients in a P2P setup" supported ([evolu.dev](https://www.evolu.dev/), [relay docs](https://www.evolu.dev/docs/evolu-server) VERIFIED) | Closest of the frameworks (stateless self-hosted relays) | Right *shape* to imitate (E2EE log sync + dumb relays); TS-first, would sit awkwardly on the Rust kernel |
| **Ditto** | Alive, commercial/proprietary; P2P sync over BLE/P2P-WiFi/LAN; **runs Chick-fil-A's cloud-optional POS** ([QSR Magazine](https://www.qsrmagazine.com/operations/fast-food/chick-fil-a-modernizes-its-pos-systems-with-ditto/) VERIFIED) | Yes at the edge (mesh), cloud optional | **The existence proof that P2P food-service ops works in production** — and rejected here only for being closed-source/commercial (violates C6 AGPLv3 destination) |
| **Turso Offline Sync** | Public beta, "no durability guarantees" (VERIFIED §1.1) | No (Turso cloud/self-hosted server anchor) | Watch-list |

### 3.2 Recommendation

**Signed per-stream event logs in per-device SQLite, replicated by anti-entropy gossip over the
bebop mesh seam; CRDT merge only for the class-A tables; no CRDT anywhere near classes B/C.**

Concretely:

1. **Storage**: one SQLite file per node (rusqlite native; official SQLite-WASM+OPFS in
   browser, demoted to cache). Two kinds of tables: `streams` (append-only
   `Envelope{stream_id, seq, at, cause_hash, content_hash, sig, event_bytes}` — the dowiz
   `order_events` schema done right, fixing the placeholder `cause_hash`/`Utc::now()` defects
   the hub review found in §3.2 finding 3) and `projections` (derived, rebuildable by `fold`,
   never synced — replay is the sync).
2. **Authority**: each stream has one signing key = its writer. Order streams → venue hub key.
   Courier/customer devices hold their own bebop2 identities and submit signed Commands; the
   hub's `decide` accepts/refuses; refusals are signed too (auditable). The deterministic
   matcher + fingerprint makes the hub's dispatch decisions *replicable and contestable* — the
   blueprint's anti-DANGER-#1 property — without pretending dispatch can be leaderless.
3. **Merge**: class-A tables ride either hand-rolled LWW columns (`(hlc, actor_id)` pairs) or
   Automerge-3 documents embedded as blobs — decide by spike; both are alive in 2026 (§3.1).
4. **Money**: class-C facts are double-entry `Transfer`s applied through the ported
   conservation ledger (`ledger.rs` invariants + `Lek` arithmetic), sequenced by the venue
   stream, co-signed at PoD by threshold sigs (blueprint L3). Offline spending caps, if ever
   needed, use escrow/Bounded-Counter pre-allocation (VERIFIED CRDT-overview §2.1) — which is
   coordination done in advance, and should be named as such.
5. **Transport**: the `MeshTransport`/zenoh seam, implemented over **iroh** (Rust, QUIC,
   released 1.0 June 2026 — §4.1) or **Zenoh 1.9** (Regions topology, zenoh-pico for
   constrained devices, April 2026 — §4.1). Both are Rust and slot behind the existing trait.

**Tradeoffs, stated honestly:**

- *Vendor hub offline = no new orders for that venue.* Same as today (the venue is closed),
  but now also true if only the *phone* is dead — mitigated by the venue replica (§4.3).
- *This is federation, not leaderlessness.* Each venue is a mini-authority. That is the
  MANIFESTO's own goal (owner controls their data) and the blueprint boundary ("the owner's
  hub is one matcher among many" — v3 §5); the anti-centralization invariant only forbids a
  single *global* sequencer.
- *Two data disciplines to maintain* (LWW merge + event log) instead of one Postgres. The
  partition table (§2.3) is the contract that keeps entities from drifting into the wrong bin.
- *Revocation and abuse control get weaker* (§1.3, §2.3 class E). Named casualties, not
  surprises.

---

## 4. The irreducible-server floor — what genuinely cannot be dropped

| # | Function | Why a server survives | 2026 evidence | Mitigation shape |
|---|---|---|---|---|
| 1 | **Rendezvous / discovery** | A customer's phone must *find* the vendor's hub. Self-cert IDs have no directory by design. First contact needs either a URL (DNS + static hosting) or a discovery relay | iroh 1.0 (June 2026): "dial keys, not IPs" — but via its relay+discovery infra, ~200M endpoint connections/month through public relays ([Pinggy/byteiota on iroh 1.0](https://pinggy.io/blog/iroh_1_0_dial_keys_not_ips/) UNVERIFIED secondary). Zenoh: peers can mesh, but scouting/first-hop config or a router is the practical bootstrap ([zenoh deployment docs](https://zenoh.io/docs/getting-started/deployment/) VERIFIED) | **The QR code on the counter is the serverless rendezvous**: it carries the venue pubkey + relay hints + menu stream id. dowiz already lives on QR (hub review §5). Web fallback = one static page (`/s/:slug`) on any dumb host — a *file server*, not an app server |
| 2 | **NAT traversal** | Phone-to-phone on cellular = CGNAT both sides; hole-punching fails for a real fraction | iroh claims ~90% direct connection success, relay fallback for the rest (vendor-claimed ~90–95%, UNVERIFIED; independent libp2p DCUtR measurement ~70% — [arXiv](https://arxiv.org/pdf/2510.27500)) | Self-hostable stateless encrypted relays (iroh DERP-style; Evolu-style relays). Relays see ciphertext only. **A relay is a server; it is just not *your state's* server** |
| 3 | **Durable storage / device loss** | *"Vendor's phone dies mid-dinner-rush"* — if the venue key's log has no live replica, in-flight orders are unrecoverable. Browser replicas are evictable (Safari 7-day, §1.2) | Physics, not tooling | **≥1 always-on venue replica is non-negotiable**: second device in the kitchen, a €5 SBC, or an E2EE storage-relay subscription. Key loss ≠ data loss (replicas re-share) but key loss = authority loss → social-recovery / threshold key escrow must be in the identity design |
| 4 | **Push notifications** | A new order MUST wake the vendor's phone. iOS: **APNs only, no third-party replacement permitted**. Android: FCM, or UnifiedPush with a self-hosted distributor (ntfy) — real but niche (~35 apps, FOSDEM 2026 talk) | VERIFIED: [UnifiedPush](https://unifiedpush.org/), [FOSDEM 2026](https://fosdem.org/2026/schedule/event/7HJJS7-unifiedpush_-_push_notifications_decentralized_and_open_source/); iOS APNs constraint (secondary but uncontested) | The push payload can be a content-free "wake and sync" ping (zero PII through Apple/Google). **This is the single most business-critical centralized dependency — own it in the threat model.** Native app + persistent socket while charging in the kitchen reduces (not eliminates) reliance |
| 5 | **Payments beyond cash** | Plisio crypto flow is webhook-anchored: "webhook POST /webhook/payments/plisio (money SoT): HMAC fail-closed … ONLY writer of payment_status=paid" (memory `crypto-payments-build-2026-06-30.md` VERIFIED) — a webhook needs a reachable HTTPS endpoint; even self-verified on-chain watching needs an RPC/full node | Structural | **Cash is the truly serverless payment rail** — dowiz's cash-as-proof spine (hub review §4.3) is an architectural asset, not a legacy. Crypto = per-venue watcher on the venue replica (#3 doubles as the webhook/RPC host) |
| 6 | **App distribution + fiat rate oracle** | App Store/Play for native apps; `exchange_rates` needs an external feed | Structural | PWA/APK sideload hedges Android; rates are cache-and-sign, degrade gracefully |

**The honest floor sentence:** *"no central server" is achievable as "no server owns the state
and no single server serves everyone" — but the system still needs (a) relays for NAT+rendezvous,
(b) one always-on replica per venue, (c) Apple/Google push to wake phones, and (d) an HTTPS
endpoint per venue if any non-cash payment rail is kept. All four are self-hostable or
per-venue except push.*

---

## 5. Migration-drop feasibility verdict

**Verdict: FEASIBLE — as a fresh-start schema plus a one-time signed export per venue. NOT
feasible (and not desirable) as a port of the 162 migrations.**

1. **The migrations are a Postgres artifact, not the domain.** The domain that must survive is
   the byte-frozen crate (`rebuild/crates/domain` — money, 10-status machine, decide/fold,
   idempotency), which contains **zero SQL** and is already WASM-gated pure (VERIFIED §0). The
   84 tables are projections + infrastructure; the new node schema is written fresh from the
   §2.3 partition (streams + projections), and the migration *discipline* that matters
   (forward-only, append-only) is inherited by the event log's nature.
2. **Prod data: one-time export, per venue.** Snapshot live Postgres (memory
   `migration-blueprint-2026-06-25.md` warns: reconcile against live `pg_dump --schema-only`
   first — column lists in docs are partly inferred) → per-venue "genesis import" events:
   catalog as class-A state, open orders as synthetic `Envelope` history (or: cut over between
   service hours with zero open orders — the realistic dinner-rush-safe plan), customer/courier
   contacts re-keyed on first contact (they must generate device keys anyway). Historical
   closed orders import as read-only archive events; they need no replay fidelity. An
   owner-data-export path already exists in the corpus (`owner-data-export-ai-2026-06-30.md`).
3. **What breaks — the named casualties:**
   - **123 RLS policies + 63 functions + the whole B3/NOBYPASSRLS program**: deleted, replaced
     by §1.3 capabilities. The multi-year flip effort becomes moot rather than finished —
     honest accounting: much of it was dormant anyway (§0).
   - **pg-boss queues, LISTEN/NOTIFY fan-out, cutover front-door, Fly deploy topology**: all
     die with the central server; local scheduler + mesh pub/sub replace them.
   - **Cross-tenant features**: platform analytics, global velocity/anti-abuse, free-tier
     accounting, platform-admin tables — either die or become opt-in signed aggregate feeds.
   - **GDPR erasure** must be redesigned as crypto-shredding + tombstones (§2.3) — weaker
     guarantee, needs counsel review before shipping.
   - **The Rust API shell (`rebuild/crates/api`) is server-shaped** (axum + sqlx + GUC
     tenancy): its route/handler logic survives conceptually but its persistence layer is
     rewritten against local SQLite — the kernel seam (`decide`) is exactly the cut line, and
     the hub review's finding that checkout currently *bypasses* `decide` (§3.2 finding 1)
     becomes the first thing the new shell fixes by construction.
4. **Sequencing reality-check**: this drop is only safe *after* the kernel is the actual door
   (0b-5 shell-flip is still human-gated per `sovereign-core-mvp-handoff-2026-07-06.md`) and
   ML-DSA is NIST-bit-exact before minting long-lived protocol keys (blueprint G10). Dropping
   Supabase before the event log is real would mean losing the only authoritative store.

---

## Sources

**Local (VERIFIED, file:line cited inline):** bebop v3 blueprint; `crates/bebop/src/{ledger,zenoh,matcher}.rs`; `docs/features/{memory,mesh}.md`; `docs/integrations/{sync,backends}.md`; dowiz `packages/db/migrations` (counted); hub architecture review §3–5; `rebuild/crates/domain/src/{kernel,money,order_status}.rs`; memory corpus files listed in the header.

**Web:** [sqlite.org WASM persistence](https://sqlite.org/wasm/doc/trunk/persistence.md) · [PowerSync: SQLite persistence on the web, May 2026](https://powersync.com/blog/sqlite-persistence-on-the-web) · [Chrome OPFS blog](https://developer.chrome.com/blog/sqlite-wasm-in-the-browser-backed-by-the-origin-private-file-system) · [wa-sqlite](https://github.com/rhashimoto/wa-sqlite) · [WebKit storage policy](https://webkit.org/blog/14403/updates-to-storage-policy/) · [MDN storage quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria) · [cr-sqlite releases](https://github.com/vlcn-io/cr-sqlite/releases) · [Automerge 3.0](https://automerge.org/blog/automerge-3/) · [PkgPulse CRDT guide 2026](https://www.pkgpulse.com/guides/yjs-vs-automerge-vs-loro-crdt-libraries-2026) · [Loro: when not CRDT](https://loro.dev/docs/concepts/when_not_crdt) · [Keep CALM and CRDT On (VLDB'23)](https://www.vldb.org/pvldb/vol16/p856-power.pdf) · [Preguiça CRDT overview](https://arxiv.org/pdf/1806.10254) · [Turso](https://github.com/tursodatabase/turso) · [Turso offline sync beta](https://turso.tech/blog/turso-offline-sync-public-beta) · [libSQL](https://github.com/tursodatabase/libsql) · [Electric alternatives](https://electric-sql.com/docs/reference/alternatives) · [PowerSync service architecture](https://docs.powersync.com/architecture/powersync-service) · [PowerSync vs Electric](https://powersync.com/blog/electricsql-electric-next-vs-powersync) · [Evolu](https://www.evolu.dev/) · [Evolu relay](https://www.evolu.dev/docs/evolu-server) · [Ditto × Chick-fil-A (QSR)](https://www.qsrmagazine.com/operations/fast-food/chick-fil-a-modernizes-its-pos-systems-with-ditto/) · [iroh vs libp2p](https://www.iroh.computer/blog/comparing-iroh-and-libp2p) · [iroh 1.0](https://pinggy.io/blog/iroh_1_0_dial_keys_not_ips/) · [NAT traversal measurement (arXiv)](https://arxiv.org/pdf/2510.27500) · [Zenoh 1.9 Longwang](https://zenoh.io/blog/2026-04-16-zenoh-longwang/) · [Zenoh deployment](https://zenoh.io/docs/getting-started/deployment/) · [Keyhive](https://www.inkandswitch.com/keyhive/notebook/) · [Beehive notebook](https://www.inkandswitch.com/beehive/notebook/) · [UnifiedPush](https://unifiedpush.org/) · [UnifiedPush @ FOSDEM 2026](https://fosdem.org/2026/schedule/event/7HJJS7-unifiedpush_-_push_notifications_decentralized_and_open_source/)
