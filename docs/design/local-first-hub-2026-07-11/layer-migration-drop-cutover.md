# Layer Blueprint — Migration-Drop / Data-Export / Strangler-Cutover Mechanics

> **Layer blueprint, 2026-07-11 (night).** How dowiz actually LEAVES Supabase + Node/TS + Fly +
> the 162 migrations (84 tables / 123 RLS policies / 63 functions / 15 triggers — counted in
> `B-data-sync.md` §0, structure re-skimmed this session in `/root/dowiz/packages/db/migrations/`),
> per the ratified local-first ladder, **without losing prod data and without ever breaking the
> live first-client v1 pilot**. Zero code written; repos read-only (code is changed by a parallel
> session); this file is the only artifact created.
>
> **Labels:** **VERIFIED** = read from a local file this session (path cited) or a primary web
> source. **VERIFIED-in-repo-doc** = carried from a sibling doc that itself verified it.
> **UNVERIFIED** = secondary web source or estimate. **DESIGN-JUDGMENT** = my call, reasoned from
> verified facts.
>
> **Standing decisions honored (binding, not re-litigated):** local-first RATIFIED as destination,
> rungs above P1 gated on G11 GREEN; COD mandatory; NO courier scoring; anonymity =
> hub-guarantee + channel choice; multichannel / no dedicated app; crypto HYBRID-ONLY until
> external audit; storefront sovereignty.
>
> **Read first, this session:** `B-data-sync.md` (feasibility verdict §5), `D-transition-blueprint.md`
> (the P0–P5 ladder §1), `SYNTHESIS.md` (§5 ladder + §9 addenda), `05-protocol-tech-completion-blueprint.md`
> (Phases W/S/R/X/A/I/H), `gap-blueprints-2026-07-11/G04-rebuild-cutover-rebaseline.md` (strangler
> mechanism + 085–089 renumber hazard §2.4), `packages/db/migrations/` (structure skim:
> `1780310074262_orders.ts` — `subtotal integer CHECK (subtotal >= 0)`, `total integer` — integer
> money is already the schema's law; `1780421100051_force-rls.ts` — the FORCE-RLS block),
> `docs/design/rebuild-plan/REBUILD-MAP.md` ("data never migrates; strangler-by-surface; **the E2E
> net is the parity oracle**" — line 5, VERIFIED), `03-anonymity-architecture.md` §2.5 (crypto-shred),
> `docs/research/2026-07-11-launch-without-lawyer-albania.md` §3.4/§5.2 (counsel triggers).

---

## 0. Frame: what "dropping the migrations" actually means (and the one doctrine it supersedes)

The 162 migrations are **not ported**. The ratified verdict (`B-data-sync.md` §5, VERIFIED):
**feasible as a fresh-start schema (streams + projections, written from the B §2.3 partition
table) plus a one-time signed export per venue** — gated on **0b-5 (the kernel is the actual
door; the shell-flip is still human-gated per `sovereign-core-mvp-handoff-2026-07-06.md`)** and
**ML-DSA NIST-bit-exactness before any long-lived protocol key is minted** (H2/H3 in doc 05;
blueprint G10). Dropping Supabase before the event log is real would mean losing the only
authoritative store — so the ladder's whole data-side job is to make the event log real, prove it
against Postgres, and only then move authority.

**One conscious doctrine supersession** (in the spirit of SYNTHESIS §7): REBUILD-MAP's cutover
doctrine is "schema kept, **data never migrates**" (REBUILD-MAP line 5, VERIFIED) — correct for
the Node→Rust cutover because both stacks shared one Postgres. The migration-drop breaks that rule
in **exactly one place**: the one-time per-venue signed genesis export (§2). Everything else about
the doctrine survives: strangler-by-surface, per-surface flags, and the E2E net as the
language-independent parity oracle.

**Casualties, named up front** (B §5.3, VERIFIED-in-repo-doc — restated so nobody is surprised
later): cross-tenant analytics **dies**; instant server-side revocation degrades to
capability-expiry + signed CAP_REVOKE propagation; DELETE-style GDPR erasure becomes
crypto-shredding (§4); global velocity/anti-abuse (5/min per phone-or-IP across tenants) degrades
to per-venue velocity + optional shared signed blocklist; the 123 RLS policies and the entire
NOBYPASSRLS program become moot rather than finished (honest accounting: largely dormant today
anyway — `dowiz_app` carries BYPASSRLS, B §0, VERIFIED-in-repo-doc).

**The pilot constraint, stated once as law:** the first client's venue rides **prod**, which is
Node-everywhere with the cutover harness inert (G04 §2.2 live probe: prod controls negative —
VERIFIED-in-repo-doc). Every rung below runs **beside** the pilot's working path, never under it,
until that rung's parity oracle is green on a non-pilot venue (§5).

---

## 1. The data-side strangler sequence — per rung

Context rungs (not data rungs, but hard preconditions): **P0** = Wave-0/1 + venue #1; **P1** = fix
the `kernel::decide` bypass so ONE honest signed event log exists (`cause_hash` is currently the
literal `"placeholder"` at `pg.rs:863` — VERIFIED-in-repo-doc, SYNTHESIS §2). No data below P2
moves until P1 is green, because replicating today's log would replicate a decoration
(D §1.2-1, VERIFIED).

### P2 — SQLite read-replica dual-run (Postgres stays 100% authoritative)

**Data mechanics.** This is the industry-standard shadow-table/shadow-read step, done with the
event log instead of CDC middleware:

1. **Source of replication = the P1-honest event log**, not table rows. A projector process (the
   first real consumer of the parked 1.3 sync port) tails `order_events`/envelopes from Postgres
   (`SELECT ... WHERE seq > have_seq` polling per stream — deliberately the same shape as doc 05's
   `SYNC_REQ{stream_id, have_seq} → SYNC_BATCH` sub-protocol, so P2's plumbing IS P4's plumbing
   pointed backwards) and appends them into an embedded SQLite file with doc 05's Phase-S schema:
   `streams` + append-only `events(stream_id, seq, at, cause_hash, content_hash, prev_hash,
   signer_id, sig, event_bytes, UNIQUE(stream_id, seq))` + rebuildable `projections_*`.
   WAL mode, `synchronous=FULL` on event appends (VERIFIED-in-repo-doc, 05 Phase S).
2. **Class-A data (menu/catalog/presence — B §2.3)** replicates as versioned snapshots (the
   existing `menu_versions` content-addressed rows are already snapshot-shaped — VERIFIED,
   migration `1780338982018_menu_versions_table.ts` exists) — no CRDT machinery yet; Postgres is
   still the only writer.
3. **Reads flip per surface, shadow-first:** owner console + courier surfaces read from the local
   replica behind a flag, with a **shadow-read comparison window** first (serve from Postgres,
   *also* read from SQLite, diff, log divergence — the "shadow read is a dry run of switching the
   data source" pattern; industry references: [InfoQ shadow-table strategy](https://www.infoq.com/articles/shadow-table-strategy-data-migration/),
   [Google Cloud on dual-write migrations](https://medium.com/google-cloud/online-database-migration-by-dual-write-this-is-not-for-everyone-cb4307118f4b) — both
   UNVERIFIED-secondary, pattern corroboration only). Note the Google piece's own warning:
   app-level dual-**write** is error-prone; that is exactly why P2 replicates the *log* (single
   writer, idempotent replay) instead of dual-writing rows.
4. **The parity check (the rung's whole point):** per order, `fold(events)` in the SQLite replica
   must equal the Postgres-projected state — a **byte-compare oracle** over a canonical
   serialization of `OrderState`, run (a) continuously on every terminal transition, (b) nightly
   over all open streams, (c) on demand before any P3 flip. Divergence increments a counter that
   **blocks the P3 gate** and fires the alerter.

- **Entry precondition:** Wave-1 merged AND P1 landed AND **G11 GREEN** (first real non-operator
  order — the pilot supplies exactly this) (D §3, VERIFIED).
- **VbM RED case:** inject a dropped event (delete one envelope from the replica feed) → the
  fold-parity byte-compare detects divergence, the alarm fires, and the flag that would let any
  surface read from SQLite **refuses to flip** (the gate reads the divergence counter). Second
  RED: reorder two envelopes → `prev_hash` mismatch at append time. A parity suite that cannot go
  RED validates nothing (VbM standing rule).
- **Reversibility:** total — delete the replica; reads flag back to Postgres. Zero authority ever
  left Postgres.
- **Effort:** 4–6 sessions (D ladder, VERIFIED). **Dependencies:** P1; doc-05 Phase S schema;
  per-event content-hash may use commodity crates or bebop2 SHA-2/3 at Tier 1 (G09 §4-P4).
- **Independently valuable even if the arc stops here:** offline reads for couriers (the single
  largest product gap — hub review §4.6 via D §1.2, VERIFIED-in-repo-doc) + a portable, verifiable
  event-log backup of every venue.

### P3 — First device-authoritative surface: menu / availability / stop-list / shift-presence (non-money)

**Data mechanics.** For the class-A subset ONLY, the arrow reverses:

1. **Device (vendor node) becomes the writer.** Menu edits land on the device SQLite first as
   **signed** events/LWW rows (`(hlc, actor_id)` columns or Automerge-3 blobs — decide by spike,
   B §3.2, VERIFIED-in-repo-doc). Signatures are **host-crate Ed25519**, not bebop2
   (`sign.rs` scalar_mul is non-CT — G09 via D ladder, VERIFIED-in-repo-doc).
2. **Postgres is demoted to replica+relay for these surfaces** — a reverse-projector writes the
   device-authored menu state back into the same Postgres tables (`products`, `categories`,
   `menu_versions`, …) so that **the live storefront, the Node prod stack, and the pilot venue
   keep reading exactly what they read today**. This reverse projection is the pilot-safety
   mechanism AND the reversibility mechanism: the old path never sees a schema change, only a new
   writer upstream of it.
3. **Server-side menu-write endpoints go flag-off, not deleted** (D ladder row 3, VERIFIED). The
   flag is per-venue: the pilot venue can stay on server-side menu writes indefinitely while a
   test venue runs device-authoritative.
4. **Convergence discipline:** offline edits queue on the device; on reconnect they replay in HLC
   order; concurrent conflicting edits resolve by the declared LWW rule — and any resolution that
   produces divergent replicas (device vs Postgres projection differing after sync settles) is a
   detected fault, not a shrug.

- **Entry precondition:** P2 parity green ≥2 weeks; ≥1 venue ≥20 orders/wk ×4 wks OR ≥3 claimed
  venues (D §3, VERIFIED).
- **VbM RED cases:** (a) partition test — edit menu offline, server down, reconnect → converge to
  identical state on device + Postgres projection; forced-divergence injection (two devices, same
  field, crafted HLC tie) must trip the divergence alarm, not silently pick. (b) Signature RED: an
  unsigned or foreign-key menu event is rejected by the projector. (c) Pilot-canary RED: the
  storefront Playwright menu slice (the E2E parity oracle, §3) runs against the pilot venue's
  storefront after every reverse-projection deploy — any DOM diff vs the Postgres-authored
  baseline is RED and auto-reverts the surface flag.
- **Reversibility:** per-surface, per-venue flag back to server-authoritative (strangler). Because
  Postgres kept a full synced copy the whole time, flip-back is one flag — no data migration back.
- **Effort:** 6–8 sessions (D ladder, VERIFIED). **Dependencies:** P2 plumbing; host-crate
  signing; doc-05 Phase W (wire) for device→server transport, or an interim HTTPS post of signed
  batches (DESIGN-JUDGMENT: the interim is acceptable because authority is signature-based, not
  transport-based).

### P4 — Money / order-state single-writer on the vendor node (the hard rung)

**Data mechanics.** Order streams (class B) and money facts (class C) move to the venue device as
THE sequencer; Postgres becomes the shadow:

1. **The write path inverts for orders of cut-over venues:** customer storefront → signed
   order-intent → store-and-forward relay → venue node → `kernel::decide` in the node's
   `sequencer.rs` (the single-writer loop, doc 05 Phase R, VERIFIED-in-repo-doc) → signed
   `Envelope{seq, at, cause_hash, sig}` appended to device SQLite → EVENT frames out.
2. **Postgres now dual-runs in the opposite direction:** a downstream projector applies the
   device-signed envelopes into the existing Postgres tables (orders, order_items,
   order_status_history, payments, courier_cash_ledger) as a **read-only mirror**. Three reasons,
   all load-bearing: (a) every un-cut surface (analytics, owner dashboards still on the old
   stack) keeps working; (b) the parity oracle can still byte-compare both stores; (c)
   **reversibility stays real** — flipping a venue back to server-authoritative means the mirror
   is already current, so the flip-back is a flag plus a quiesce window, not a data rescue.
   D is honest that "money migration back is real work — the first rung with material re-entry
   cost" (VERIFIED); the standing mirror is what keeps that cost bounded.
3. **Settlement/COD facts** ride the obligation ledger of doc 05 Phase X (Σ=0, counter-signed
   `CustodyHandoff`, fail-closed HOLD) — `Lek(i64)` end-to-end, no `f64` in the settle API,
   SQLite REAL banned from money columns by schema-lint (05 Phase S/X, VERIFIED-in-repo-doc).
4. **Idempotency** stays a pure kernel decision (tenant-scoped `(location_id, key)`), now enforced
   at the sequencer; `UNIQUE(stream_id, seq)` + content-hash dedup make replays no-ops.
5. **085–089 hazard intersects HERE** (G04 §2.4, VERIFIED-in-repo-doc): the five money-migration
   drafts (settlements-catchup, refund-due trio, gdpr-erase-definer, cutover-flags) were never
   formally placed; sovereign migrations took numbers 085/086 on 07-07, so the drafts **must be
   renumbered to ≥087 (proposed 087–091)** and the settlements-catchup watermark literal
   (`2026-07-10`, now PASSED) **must be re-dated with the pre-apply assert** before ANY settlement
   work touches Postgres — applying 085 verbatim today is the exact double-pay hazard. Disposition
   (G04 D2) is **required before P4's settlement lane starts**, whichever path: either the drafts
   are renumbered + applied (if Postgres-side settlement still matters pre-cutover) or formally
   retired in the disposition doc (if settlement is born on-device in Phase X and Postgres never
   runs the new fn). Leaving them undispositioned is the one way this rung can silently corrupt
   money **on the old stack** while everyone watches the new one.

- **Entry precondition (the full gate bundle — red-line):** ≥3 paying venues (or 1 venue ≥4 wks
  ≥20 orders/wk); P3 stable; **bebop2 crypto at Tier 2 hybrid minimum** (Wycheproof +
  differential-vs-oracle + dudect + fuzz — G09 §4-P4; until then every value-bearing signature
  verifies on the audited classical half, PQ additive only); **wasm32 empty-import gate green**
  (already green at bebop HEAD `57b1c9a` — SYNTHESIS reconciliation note, VERIFIED-in-repo-doc);
  **the reliability layer proven**: warm-spare venue replica live (Litestream/second device),
  kill-the-node drill + kill-the-relay drill both green, store-and-forward relay holding signed
  intents through venue-node death; **money council GO**; **0b-5 shell-flip done** (the kernel is
  the door — B §5.4); ML-DSA bit-exactness (H3) before any long-lived protocol identity is minted.
- **VbM RED cases:** (a) **forge drill** — a relay-injected event without a valid signature is
  REJECTED, order state unchanged; (b) **kill-the-relay drill** — an order completes from the
  device log after reconnect; (c) **kill-the-node drill** — the warm spare serves the stream, no
  envelope lost (atomicity per 05 Phase S crash-restart drill); (d) **no-money-invented** —
  device-computed totals ≡ the pricing oracle, and the Postgres mirror's `payments` rows
  byte-match the device ledger fold; (e) **dual-store divergence** — mutate one mirrored row in
  Postgres → the nightly byte-compare flags it (proves the oracle watches both directions).
- **Reversibility:** per-surface, per-venue flag back to server-authoritative; the standing
  Postgres mirror makes flip-back a flag + quiesce, but this is honestly the first rung where
  reverse costs real work if the mirror has been allowed to lag — therefore **mirror lag is
  itself an alarmed metric** with a max-lag SLO that blocks onboarding further venues.
- **Effort:** 12–18 sessions + councils (D ladder, VERIFIED); overlaps doc 05's R (8–12) + X
  (8–12). **Dependencies:** P3; doc-05 W/S/R/X/A; G04 D2 disposition; Tier-2 gate; reliability
  layer.

### P5 — Decommission Supabase / Node / Fly-as-app-host (burn-the-boats, done last)

**Data mechanics — the actual drop:**

1. **Final freeze + archive:** quiesce all remaining Postgres writers (by P5 there should be
   none authoritative); take a final `pg_dump` (full) + `pg_dump --schema-only` (the
   reconciliation baseline the migration-blueprint memory demands — column lists in docs are
   partly inferred, B §5.2, VERIFIED-in-repo-doc); content-hash and sign both dumps; store as the
   cold archive (owner-held keys). This archive is the legal/forensic record, not a live system.
2. **Per-venue closure check:** every active venue has a signed genesis export (§2) already
   consumed + a live device log whose head passed parity; venues never exported (dead/demo) live
   only in the cold archive.
3. **Cross-tenant remainder:** platform-analytics, free-tier snapshots, acquisition, abuse logs —
   exported once into the archive, then **die** (the named casualty). Any future aggregate =
   opt-in signed feeds from venue nodes.
4. **Infra teardown order:** Supabase project → paused, then deleted after the archive verifies
   (restore-drill: load the dump into a scratch Postgres, run the schema-drift check against the
   recorded schema hash — RED if diff); Node app + workers decommissioned (this subsumes/retires
   G04 Phase D with its own REV-C10-grade checklist: full E2E run on the new path, map-coverage
   zero-orphans); Fly reduced to the irreducible floor — which per SYNTHESIS §4/§9 isn't Fly at
   all: **€6/mo Hetzner relay (SNI-passthrough, TLS on node) + push gateway + static storefront
   host + encrypted backup target**. The unplaced migration drafts are formally retired in the
   disposition record; `pgmigrations` history goes into the archive with everything else.
- **Entry precondition:** all surfaces device-authoritative or relay-thin; **≥2 months dual-run
  parity** on the Postgres mirror; GDPR/fiscalization design signed (§4); infra-cost or
  platform-EOL forcing event (D §3, VERIFIED).
- **VbM RED case:** the **staging drill** — Postgres switched off for a full L0–L11 order
  lifecycle on the new path; RED = anything still reads it (connection attempts logged at the
  DB proxy = the falsifier). Plus the archive restore-drill RED above.
- **Reversibility:** poor by design — this is the boats rung. The mitigations are the signed cold
  archive (data is never lost, only the *live* system is gone) and doing P5 only after the floor
  has been boring for two months.
- **Effort:** 5–10 sessions + ops (D ladder, VERIFIED). **Dependencies:** everything above;
  K7 (GDPR/fiscalization lawful) not fired.

---

## 2. The one-time per-venue signed export (genesis)

The single sanctioned violation of "data never migrates." **DESIGN-JUDGMENT throughout this
section, built on B §5.2 (VERIFIED) and the owner-data-export path already in the corpus
(`owner-data-export-ai-2026-06-30.md`, cited there).**

### 2.1 Mechanism

1. **Snapshot, not live-read:** the exporter opens one `REPEATABLE READ` transaction (or
   `pg_dump --snapshot` against an exported snapshot ID) so every table reads at one LSN. The
   snapshot LSN + `pg_dump --schema-only` hash + the `pgmigrations` census are recorded in the
   genesis header — this pins **which schema lineage** was exported (prod = migrations ≤ 084;
   staging carries 085/086 + shim-run drafts — G04 §2.4, VERIFIED-in-repo-doc; the pilot's real
   data lives on **prod**, so the exporter is written and tested against prod's schema, staging's
   drift notwithstanding).
2. **Re-partition, not table-copy.** The exporter reads Postgres rows and emits the **new** shapes
   (B §2.3 partition):
   - **Catalog / class A** (products, categories, modifiers, modifier_groups,
     product_modifier_groups, translations, content_i18n, media refs, menu_schedules,
     delivery_tiers, location + theme, promotions definitions) → genesis **state snapshot**
     documents with initial HLC/version stamps.
   - **Courier roster** (couriers, courier_shifts skeleton) → roster stream genesis; standing
     `courier_invites`/session rows are NOT copied — couriers re-enroll by minting device keys and
     receiving **capability certificates** (the invites were "capabilities wearing Postgres
     clothes" — B §1.3, VERIFIED-in-repo-doc).
   - **Open orders** → synthetic `Envelope` history (one envelope per recorded
     `order_status_history` transition, `cause_hash` chained, timestamps preserved from the rows) —
     **or, strongly preferred: cut each venue over in a zero-open-orders window between service
     hours** (B §5.2's "realistic dinner-rush-safe plan", VERIFIED) so this lane is empty.
   - **Closed orders + payment/settlement facts** → read-only **archive events** (no replay
     fidelity required; they exist for owner history + dispute look-back). Settlement rows are
     exported only WITH the G04-D2 watermark disposition recorded (§1-P4-5).
   - **Customer PII** → **not bulk-exported.** Phones/addresses re-key on first contact
     (customers must generate/receive per-order envelopes anyway); what exports is the order
     skeleton with a hash commitment where PII used to be (§4). This is the enforcement point:
     **cleartext PII never enters a device-held SQLite file.**
3. **Signing + verification chain:** the export tool canonically encodes (borsh, per doc 05
   Phase W) a `Genesis{venue_id, source_lsn, schema_hash, taken_at, sections…}`, content-hashes
   it, and signs with the platform/operator export key; the venue node **countersigns on import**
   (acceptance event = envelope seq 0 of every stream; `prev_hash` chains start at the genesis
   hash). Verification is threefold: (a) **determinism** — running the exporter twice at the same
   snapshot must produce byte-identical output (hash equality; RED if not — a nondeterministic
   exporter can't be signed honestly); (b) **count/sum invariants** — row counts per section +
   Σ of money fields (`Lek(i64)`) must equal SQL aggregates computed inside the same snapshot
   transaction; (c) **behavioral parity** — the imported node renders the venue's storefront/menu
   and open-order list through the Playwright oracle byte/DOM-equal to the Postgres-rendered one
   (§3).
4. **What is deliberately LEFT BEHIND** (dies with Postgres, per B §2.3/§5.3, VERIFIED-in-repo-doc):
   all 123 RLS policies + 63 functions + 15 triggers (logic lives in `decide` or dies);
   auth_refresh_tokens / OTP sessions / telegram connect tables (server-auth artifacts);
   pg-boss queues; webhook_endpoints; **cross-tenant analytics (analytics_events, funnel_events,
   analytics_cwv aggregated across venues), free_tier_snapshots, acquisition_sources, global
   velocity/abuse logs — the cross-tenant view ceases to exist**; exchange_rates (re-fetched,
   cache-and-sign); platform-admin tables (archived only).

### 2.2 Tooling note (web-checked)

Generic Postgres→SQLite converters exist —
[pg2sqlite](https://github.com/caiiiycuk/postgresql-to-sqlite) (converts a `pg_dump` file;
schema-unaware of our stream model), [db-to-sqlite](https://pypi.org/project/db-to-sqlite/)
(SQLAlchemy-based table copier), and pg_dump `--attribute-inserts` hand-conversion
([guide](https://www.codegenes.net/blog/how-to-convert-a-postgres-database-to-sqlite/)) — all
UNVERIFIED beyond their pages. **None is the mechanism** here, because the genesis is a
re-partition into signed streams + projections, not a table copy; they are acceptable only for
the read-only cold-archive lane (a browsable SQLite of historical closed orders) where fidelity
to the *old* shape is the point. The export tool is bespoke, small, and testable
(DESIGN-JUDGMENT; effort ~2–4 sessions inside the P4 window, drilled repeatedly against the demo
venue long before the pilot venue).

---

## 3. Invariants preserved across the swap + the parity oracle

### 3.1 The invariant table (the schema's soul — B §0, all VERIFIED there; mapping per doc 05)

| Invariant (today, Postgres) | After the swap (SQLite/node) | Proof that it survived |
|---|---|---|
| **Integer money** — `subtotal integer CHECK (>=0)`, `total integer` (VERIFIED `1780310074262_orders.ts`); `Lek(i64)`, no `From<f64>` | `Lek(i64)` inside `event_bytes` only; **SQLite REAL banned from money columns by schema-lint test**; no `f64` in the settle API (type wall) | schema-lint RED on any REAL money column; no-money-invented drill (§1-P4) |
| **Byte-frozen 10-status machine** (`order_status.rs:19-70`) | Enforced solely in `kernel::decide` — the node has **zero triggers by design** | state-machine conformance vectors: every legal transition AND every illegal one with expected refusal (05 Phase I) |
| **Forward-only migrations discipline** | Inherited by construction: the event log is append-only; `prev_hash` hash-chain; `UNIQUE(stream_id, seq)`; replay is the sync | flip one stored byte / delete a middle event → chain verification refuses the stream at open (05 Phase S RED) |
| **FORCE-RLS / tenant isolation** (76 FORCE occurrences; dormant under BYPASSRLS) | **Physical partition** (one venue per file — no other tenant exists in the file) + **capability tokens** (bebop-cap: sig + `exp` + nonce, verified offline; hybrid rule compiled into the verifier) | offline forge/expiry RED set (05 Phase A): expired, tampered-scope, wrong-issuer, and PQ-only-signature capabilities ALL rejected |
| **Idempotency** — tenant-scoped `(location_id, key)` | pure kernel decision at the sequencer + content-hash dedup | replayed command → no second envelope (delta count = 0) |
| **Payment monotonicity** — `payment_events` insert-wins, `refunded ≤ captured ≤ amount`, single writer of `paid` | residual guard asserted in `decide`; single writer = the sequencer loop itself; Σ=0 conservation on obligation accounts | double-spend RED (05 Phase X): replayed/duplicated custody handoff rejected, Σ≠0 surfaces |
| **Single money surface / single writer** | `sequencer.rs` is the only appender; exclusive-writer DB lock; grep-gate CI proves no other write path | second sequencer process → rejected by lock (05 Phase R RED); grep-gate RED on any out-of-band append |

### 3.2 The parity oracle — language-independent, per surface

REBUILD-MAP's doctrine carries over verbatim: **"the E2E net is the parity oracle"**
(REBUILD-MAP line 5, VERIFIED; estate: 174 E2E specs, 151 PARITY / 23 REBASE — REBUILD-MAP §
"Test estate", VERIFIED). The Playwright suite asserts HTTP responses and rendered DOM — it does
not know or care whether Node+Postgres or Rust-node+SQLite served the bytes. That makes it the
one check that survives every substrate swap in this program, exactly as it already served the
Node→Rust cutover (G04 A3 "parity re-run", 0-leaf-diffs discipline — VERIFIED-in-repo-doc).

Three oracle tiers, per rung:

1. **Fold-parity (state level):** canonical byte-compare of `fold(events)` across stores — SQLite
   replica vs Postgres (P2), device vs Postgres mirror (P4). Cheap, continuous, per-order.
2. **E2E parity (behavior level):** the relevant Playwright slice runs against the new path with
   the flag on and must produce the identical green run + identical response bodies for
   contract-bearing endpoints. Per-surface DoD stays REBUILD-MAP's: E2E slice green + contract
   diff empty + invariant-cluster tests red→green (VERIFIED, REBUILD-MAP §"Cutover DoD").
3. **Drill parity (failure level):** the rung's RED drills (§1) — dropped-event, forge,
   kill-node, kill-relay, double-spend — each must actually fire RED when injected. A drill that
   can't fail proves nothing (VbM standing rule).

**Oracle sunset:** tier-1 dies with Postgres at P5 (nothing left to compare against — replaced by
the hash-chain + conformance vectors of doc 05 Phase I); tier-2 and tier-3 are permanent.

---

## 4. The GDPR / erasure change — DELETE → crypto-shredding

**Today:** S9 GDPR erasure runs end-to-end to `completed` centrally (Rust engine on staging, Node
on prod; `gdpr_erasure_requests` table; DELETE/anonymize semantics against one DB — G04 §2.1,
VERIFIED-in-repo-doc). This works because there is one copy.

**After:** an append-only, hash-chained, multi-device-replicated log cannot un-say things. The
ratified design (03-anonymity §2.5, VERIFIED) splits every order record:

- **Signed skeleton (immutable, replicated, PII-free):** order id, items + `Lek(i64)` amounts,
  the 10-status transitions, per-order pseudonym pubkey, PoD proof, NIVF fiscal reference.
- **PII envelope (encrypted, erasable):** address, contact, optional name — encrypted under a
  **per-order data key**; only a hash commitment enters the signed log; envelope lives on the
  vendor node (+ courier device for the run's duration), never gossiped.
- **Erasure = destroy the per-order key** (+ a tombstone event). Ciphertext rots into noise on
  every replica; the skeleton survives for fiscal/audit. EDPB Guidelines 02/2025 treats
  key-destruction as rendering data practically unidentifiable where literal deletion is
  infeasible (VERIFIED via 03-anonymity §2.5, edpb.europa.eu cited there).

**Transition rules (DESIGN-JUDGMENT, the part this layer adds):**

1. **Dual-erasure during dual-run (P2–P4):** while Postgres holds any copy, an erasure request
   fans out to BOTH stores — the existing DELETE path on Postgres AND key-shred on any device
   envelope. A request is `completed` only when both legs report; a one-legged completion is the
   RED case (assert: after completion, the phone/address appears in neither `pg_dump` grep nor
   any device envelope decryption attempt — the key must be gone).
2. **The export is the enforcement boundary (§2.1-2):** PII crosses into device-held SQLite only
   as an encrypted envelope with a live key — never cleartext. Genesis export of historical
   closed orders carries hash commitments only; historical PII stays in Postgres (erasable by
   DELETE) until it ages out or P5 archives it under owner-held keys.
3. **Weaker guarantee, said plainly:** crypto-shredding depends on the key actually being
   destroyed everywhere it was held, and capability/key revocation propagates only as fast as
   sync (B §1.3, VERIFIED-in-repo-doc). This is honestly weaker than a single-DB DELETE.
4. **The counsel trigger, honestly flagged:** the launch-without-lawyer doc's own thresholds say
   pilot-scale needs no counsel, and counsel becomes advisable at **large-scale processing,
   profiling/DPIA territory, high-risk-processing authorization questions, or any Commissioner
   inquiry** (§3.4, VERIFIED) — and hard trigger #6 is "any regulator contact → lawyer,
   immediately" (§5.2, VERIFIED). Changing the erasure *mechanism* for data subjects (DELETE →
   key-destruction) is a controller-behavior change that must be reflected in the privacy notice
   and **reviewed by counsel before scale — before P5 at the latest, and before P4 if the pilot
   venue's real customers' PII starts living in device envelopes** (matches doc 05 risk #9:
   "counsel review before scale, not before pilot", VERIFIED-in-repo-doc; and D's K7 kill-switch:
   if the device-held-PII design cannot be made lawful/practical, P5 hard-stops while P0–P4
   remain valid — VERIFIED). Fiscal records (NIVF references, the venue's own sale records) are
   retention-bound and are precisely why the *skeleton* is designed PII-free.

---

## 5. v1-pilot safety + the VbM ledger per rung

### 5.1 The pilot-safety rules (standing, all rungs)

The first client's venue is on **prod = Node everywhere, cutover harness inert** (G04 §2.2 probe,
VERIFIED-in-repo-doc). The transition never breaks it because:

1. **The pilot is always on the working path.** No rung's flag flips for the pilot venue until
   that rung has been green on a **non-pilot venue first** (demo / sushi-durres seeded venues
   exist in the migration lineage — VERIFIED, `1790000000045/46/56/57_*` seed migrations). Flags
   are per-surface AND per-venue from P3 up.
2. **Additive-only until authority moves.** P2 is invisible to the pilot by construction
   (read-only shadow). P3's reverse projection guarantees the pilot's storefront keeps reading
   the same Postgres rows. P4 cut-over is per-venue; the pilot venue is cut **last**, after ≥1
   other venue has run device-authoritative for its soak window.
3. **Rollback stays one flag.** The strangler mechanism is proven at 2.4 s per-surface rollback
   (G04 §2.2/§3, VERIFIED-in-repo-doc) — the ladder inherits it, plus the P4 Postgres mirror
   keeps flip-back a flag rather than a data rescue.
4. **The degrade-storm lesson is a precondition, not a memory.** Before any new flag machinery
   touches a live path: boot-grace + alert-on-degrade + restart-regression test (G04 A1,
   VERIFIED-in-repo-doc — the 07-05 storm silently degraded 6 surfaces during a Node restart and
   discovery was accidental). Silent auto-degrade of a pilot surface is the exact class this
   rung-ladder must never reproduce; money surfaces refuse auto-degrade by design.
5. **Money never moves without an operator Y.** P4 entry is a red-line council bundle (money
   council + Tier-2 evidence + reliability drills + G04-D2 disposition); per-venue cutover is an
   explicit ⛔ act, mirroring the S5/S9 per-surface-Y discipline (G04 A7, VERIFIED-in-repo-doc).
6. **Kill-criteria stay armed** (D §4.2, VERIFIED): K2 (parity RED ×2 → stop, fix or retire the
   replica), K4 (venue-device-as-sequencer misses orders worse than today's server path → fall
   back to relay-authoritative intake), K5 (any rung idle >14 days → dated auto-park), K7
   (GDPR/fiscalization unlawful → P5 hard stop).

### 5.2 The VbM ledger (one row per rung — each proof must be able to go RED)

| Rung | GREEN proof | The RED case (pre-committed falsifier) | Blocks |
|---|---|---|---|
| **P2** | fold-parity byte-compare ≡ per order, continuous + nightly; replica serves offline reads | **injected dropped event → divergence DETECTED, alarm fires, and the read-flag flip is mechanically refused** (the gate reads the divergence counter); reordered envelopes → prev_hash RED | P3 entry |
| **Export** | double-run determinism (hash-equal); count/Σ invariants ≡ in-snapshot SQL; Playwright renders genesis ≡ Postgres | flip 1 byte of genesis → import signature/hash verify REJECTS; nondeterministic re-run → hash mismatch RED | any venue's node genesis |
| **P3** | partition→reconnect converges device ≡ Postgres projection; pilot storefront E2E slice unchanged | forced concurrent conflict → divergence alarm (not silent pick); unsigned/foreign-key event → projector rejects; pilot-canary DOM diff → auto-revert flag | P4 entry |
| **P4** | full COD lifecycle on-device: forge-reject + kill-relay + kill-node + Σ=0 + no-money-invented, all green; mirror lag < SLO | relay-injected unsigned event ACCEPTED = broken (must REJECT); mutated mirror row undetected by nightly compare = broken; double-spent custody applied twice = broken; mirror-lag breach → onboarding freeze | P5 entry; pilot-venue cutover |
| **P5** | staging drill: Postgres OFF through L0–L11; archive restore-drill green; cost line ≈ relay+DNS+push | any read of Postgres during the drill (DB-proxy connection log = the falsifier); archive restore schema-hash diff → RED | the actual decommission |

### 5.3 Effort + dependency recap (data-side only)

P2 4–6 · P3 6–8 · P4 12–18 (+ export tool 2–4 inside it) · P5 5–10 sessions (D ladder, VERIFIED;
export estimate DESIGN-JUDGMENT). Hard external gates: G11 GREEN (P2), venue-usage thresholds
(P3/P4), **bebop2 Tier-2 + wasm32 green + reliability drills + money council + 0b-5 + ML-DSA
bit-exactness (P4)**, counsel-reviewed erasure + fiscalization design + 2-month soak (P5).
G04-D2 (085→087–091 renumber + watermark re-date/retire) is required before any settlement work
under every path and is this layer's only inherited Postgres-side red-line migration act.

---

*Produced 2026-07-11 (night), read-only except this file. Sources: sibling lenses B/D/SYNTHESIS/05
+ G04 + REBUILD-MAP + 03-anonymity + launch-without-lawyer (all cited inline with labels);
migrations structure skimmed directly; web: [InfoQ shadow-table strategy](https://www.infoq.com/articles/shadow-table-strategy-data-migration/),
[Bussler, dual-write migrations](https://medium.com/google-cloud/online-database-migration-by-dual-write-this-is-not-for-everyone-cb4307118f4b),
[pg2sqlite](https://github.com/caiiiycuk/postgresql-to-sqlite), [db-to-sqlite](https://pypi.org/project/db-to-sqlite/),
[pg→SQLite hand-conversion guide](https://www.codegenes.net/blog/how-to-convert-a-postgres-database-to-sqlite/)
(all UNVERIFIED-secondary, pattern corroboration only). Parent session consolidates; no Telegram
sent from here.*
