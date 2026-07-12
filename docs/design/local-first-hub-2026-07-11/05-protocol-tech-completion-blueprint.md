# 05 — Protocol Technical-Completion Blueprint: from library-kit to working protocol

> **Execution blueprint, 2026-07-11 (late evening).** The concrete, phased plan to finish the
> TECHNICAL part of the bebop delivery protocol: a wire, storage, a node runtime,
> settlement/dispute, on-wire identity, and an interop/second-implementation path. Zero code was
> written for this doc; both repos read-only, left as found. This session's live checks:
> bebop-repo HEAD has moved again to `84d5adc` ("bound call_tool input size + pilot fan-out (DoS
> hardening)") — one commit past the `57b1c9a` that `01-bebop2-protocol-state.md` froze; all of
> 01's structural findings re-verified at the new HEAD (zenoh absent from `Cargo.lock`: 0 matches;
> `dispute|arbitrat` in all Rust src: 0 matches; `sqlite|rusqlite` in any Cargo.toml: 0 matches;
> `bebop2/{kernel,cli,reloop}` dirs: absent — all VERIFIED this session).
>
> Labels: **VERIFIED** (checked against code/git/web this session), **VERIFIED-in-repo-doc**
> (carried from a cited sibling lens that verified it), **UNVERIFIED**, **DESIGN-JUDGMENT**
> (my call, reasoned from verified facts).
>
> Standing decisions honored throughout (binding, not re-litigated): local-first ratified as
> destination with rungs above P1 gated on G11 GREEN; **COD mandatory**; **NO courier scoring**
> (reputation.rs courier path removed/dark in the dowiz lineage); anonymity = hub-guarantee +
> channel choice; multichannel, no dedicated app; **crypto HYBRID-ONLY until external audit**
> (living memory `local-first-and-no-courier-scoring-2026-07-11.md`, VERIFIED read).

---

## 1. The current gap, tight (ground truth = `01-bebop2-protocol-state.md`, re-verified at `84d5adc`)

What bebop **is** today: a green 388-test **library kit** — pure replicable matcher
(`crates/bebop/src/matcher.rs:74` `match_orders`, `:100` `fingerprint`), PoD attribution
(`pod.rs:73/:88`), conservation ledger (`ledger.rs:79-81` Σ=0, `:38` content-addressed idempotency),
reputation, consensus kill-switch, audited-crate hybrid PQ vault, and a wasm32 no_std crypto core
with a byte-verified empty import section (01 §3, §7 — VERIFIED there, structure re-confirmed here).

What it is **not** — the five zeros (each re-verified this session):

| Zero | Evidence |
|---|---|
| **Zero network code.** `zenoh.rs:1-10` self-describes as a "local broker stand-in"; the `zenoh` crate is not a dependency (Cargo.lock grep = 0); matcher's `Transport` trait (`matcher.rs:149`) has only `InMemoryTransport`. No HTTP/TCP/QUIC/libp2p/iroh anywhere. | VERIFIED |
| **Zero storage.** No SQLite/rusqlite in any Cargo.toml; ledger and reputation are in-memory `HashMap`s; the vault is one JSON blob. | VERIFIED |
| **No runnable node.** The `bebop` binary is the coding-agent CLI; nothing executes matcher/pod/ledger as a delivery node. `bebop2/{kernel,cli,reloop}` don't exist as dirs. | VERIFIED |
| **Settlement + dispute = 0 lines.** `grep -riE "dispute|arbitrat"` over all Rust src = 0. F2 is a spec, G4/G8 open. | VERIFIED |
| **PQ crypto non-interoperable + unaudited** (G09/G10): ML-KEM coefficient-domain, ML-DSA CBD-sampled A + 32-byte challenge, Ed25519 secret-dependent `scalar_mul` branch, no Wycheproof/ACVP/external audit. | VERIFIED-in-repo-doc (01 §6) |

The seams the plan builds on are real and test-proven: `Transport`/`MatcherClient`
(`matcher.rs:127-186`, remote==local test `:306`), the `Mesh` pub/sub port (`zenoh.rs`, over
`Portkey::Envelope{topic,from,to,body}` — note: **string-typed, unsigned, no bytes** — VERIFIED
read), dowiz's `Envelope{seq,at,cause,event}` + `decide` at `rebuild/crates/domain/src/kernel.rs:306`
(VERIFIED this session), and DECISIONS D2's per-event content-hash + signature slot. The decided
architecture this plan implements is `02-local-first-architecture.md` §3 (per-vendor sovereign node
= single-writer sequencer; SQLite event log; iroh + WSS + €6 relay per
`2026-07-11-relay-hetzner-tailscale-mesh.md` §3).

One naming landmine to fix on day one (VERIFIED this session): dowiz's rebuild workspace already
contains a crate **named `bebop`** (`rebuild/crates/bebop` — the OS-agnostic *agent* core, depending
only on `domain`). The protocol crates coming from bebop-repo must land under distinct names
(`bebop-wire`, `bebop-settle`, …) or the workspace will not build.

---

## 2. The seven-part completion plan

Conventions: **effort in focused sessions** (one session ≈ a half-day of concentrated work,
matching the unit used in `D-transition-blueprint.md`); every phase names its **VbM proof with the
RED case** (per the standing Verified-by-Math rule: a proof that cannot go RED proves nothing);
"node" = the vendor sovereign node unless stated. Crate placement (DESIGN-JUDGMENT, rationale in
§2.3): protocol crates live in bebop-repo as versioned libraries; the runnable product node lives
in dowiz `rebuild/` where `kernel::decide` already is.

### Phase W — The wire (replace the zenoh stand-in with a real transport)

**Entry precondition:** none on the dowiz product; needs only the bebop-repo workspace green
(it is, 388/388 — VERIFIED-in-repo-doc). Can start immediately as pure library work.

**Scope (modules described, not coded):**

1. **`bebop-wire` crate — frame + codec.** A versioned, signed, length-bounded frame
   (DESIGN-JUDGMENT, aligning with 02 §3.6's canonical-bytes ranking and DECISIONS D2):
   - Header: `magic [u8;4]` + `version u16` (major/minor split: unknown **major** ⇒ reject
     fail-closed; unknown **minor** ⇒ accept, ignore unknown trailing fields), `frame_type u8`
     (HELLO, COMMAND, EVENT, ACK/NACK, SYNC_REQ, SYNC_BATCH, CAP_PRESENT, CAP_REVOKE, PING),
     `flags u8`, `body_len u32` with a **hard max-frame bound** (the exact DoS class HEAD commit
     `84d5adc` just fixed in the agent CLI — carry the lesson to the wire on day one).
   - Body: **borsh-encoded canonical bytes** of a fixed schema struct. Borsh is chosen now
     because it is a published, deterministic, "made for signing" codec
     (VERIFIED-in-repo-doc, 02 §3.6 ranked borsh > postcard > CBOR; the 6over3 `bebop` format is
     explicitly banned for signatures there). bebop2's own hand-written fixed-layout codec
     (ARCHITECTURE.md "no serde in core" rule — VERIFIED read) **graduates into the same trait
     later**; the codec sits behind a `Canonical` encode/decode trait so the swap is a
     one-crate change. Neither borsh nor iroh is currently a dependency (Cargo.lock grep = 0,
     VERIFIED) — both are new, deliberate additions at the **shell**, never inside
     `bebop2-core`/`dowiz-core` (C2 purity: the wasm32 empty-import gate must stay green).
   - Signature block: `sig_count u8` + `[ (scheme_id u8, signer_id [u8;32], sig) ]`. The
     signature covers **header ‖ body** (so a frame can't be transplanted across types or
     versions). Scheme ids: `0x01` Ed25519 (host crate — the audited half, MANDATORY on every
     value-bearing frame), `0x02` ML-DSA-65 (optional PQ half, additive only — the HYBRID-ONLY
     rule of §5 wired into the byte format itself).
   - `content_hash = SHA-256(body)` is the frame's dedup/idempotency key (same rule as
     `ledger.rs:38` and mesh.md content-addressing — dedup is free).
2. **`WireTransport` port** (one Rust trait, async): `connect(peer) / send(Frame) / recv() /
   subscribe(topic)`. Three implementations:
   - **`transport-iroh`** — vendor↔courier leg. iroh 1.0 (v1.0.2, wire+API stability commitment —
     VERIFIED-in-repo-doc, relay research §2a), Ed25519 NodeId dialing = bebop's self-cert
     classical key, **self-hosted token-gated `iroh-relay` on the same Hetzner CX23** (~€6/mo
     all-in, relay research §3.1). Direct QUIC on LAN for same-premises for free.
   - **`transport-wss`** — customer leg. Plain WSS terminating TLS **on the node** behind the
     nginx `stream` SNI-passthrough relay (relay research §1.3; Cloudflare Tunnel stays rejected —
     it terminates TLS at edge). The browser never runs iroh (relay research §2a: browser iroh is
     relay-only-over-WebSocket anyway — WSS is strictly lighter for one order).
   - **`transport-mem`** — the current in-process `Mesh`/`InMemoryTransport`, re-expressed over
     the same trait. It stays forever as the deterministic test double; `matcher.rs:306`'s
     remote==local test becomes a trait-generic conformance test that every transport must pass.
3. **Sync sub-protocol** (single-writer replication, deliberately dumb): `SYNC_REQ{stream_id,
   have_seq}` → `SYNC_BATCH{envelopes seq > have_seq}`. Because every stream is single-writer and
   totally ordered (B-lens §2.2 — both codebases already assume this), no merkle/gossip machinery
   is needed for orders; CRDT sync for menus is a later, separate lane (B-lens §2.3 class A).
4. **The wire spec is written as it is built** — `docs/spec/WIRE-v0.md` in bebop-repo, byte
   layouts + the golden vectors of Phase I. The spec is the deliverable that makes this a
   protocol; see Phase I.

**VbM proof:** two OS processes on **two machines** (vendor-node stub on a box, courier stub on a
phone or second host, through the self-hosted relay) complete a full order round-trip:
COMMAND(PlaceOrder) → EVENT(Priced, Pending) → COMMAND(Claim) → EVENT(Assigned) → PoD →
EVENT(Delivered), all frames signature-verified. **RED case:** a proxy that flips one byte in a
frame body (or transplants a valid signature onto a different frame_type) ⇒ receiver rejects,
order state unchanged; **second RED:** a frame with `version.major+1` ⇒ rejected fail-closed, and
a frame with only an ML-DSA signature (no classical) on a value path ⇒ rejected (hybrid rule).

**Effort:** 6–8 sessions (frame+codec 2, iroh leg 2–3, WSS leg 1–2, sync+conformance tests 1).
**Depends on:** nothing. **Feeds:** every later phase.

### Phase S — Storage (SQLite on-device; the event log as the sequencer's substrate)

**Entry precondition:** Phase W's frame/envelope schema frozen at v0 (the log stores exactly the
signed envelope bytes the wire carries — one canonical byte string, hashed once, signed once,
stored once, per DECISIONS D2).

**Scope:**

1. **`node-store` module (rusqlite, vendor node).** Schema (per 02 §3.5 + B-lens §3.2, refined):
   - `streams(stream_id TEXT PK, kind TEXT, head_seq INTEGER, head_hash BLOB)` — one stream per
     order (plus venue-level streams for roster/menu snapshots).
   - `events(stream_id TEXT, seq INTEGER, at INTEGER, cause_hash BLOB, content_hash BLOB,
     prev_hash BLOB, signer_id BLOB, sig BLOB, event_bytes BLOB, UNIQUE(stream_id, seq))` —
     append-only; `prev_hash` makes each stream a hash chain; **this table IS the system of
     record**. `cause_hash` is real from day one (killing the `"placeholder"` defect the hub
     review found).
   - `projections_*` — rebuildable folds (current order state, open-obligation balances, menu).
     Never synced; **replay is the sync**.
   - Durability: WAL mode, `synchronous=FULL` on the event-append transaction (money-bearing;
     `NORMAL` may lose the last commit on power-cut — VERIFIED-in-repo-doc 02 §3.5), `NORMAL`
     acceptable for projections. Litestream → dumb blob store as the warm-spare/backup lane.
   - **Invariant preservation is the schema's contract:** money exists only as `Lek(i64)` fields
     inside `event_bytes` (never a DB float — SQLite REAL banned from any money column by a
     schema-lint test); the 10-status machine is enforced in `decide`, never by triggers (there
     are none); payment monotonicity = the kernel's residual guard (`refunded ≤ captured ≤
     amount`) asserted in `decide` + `UNIQUE(stream_id, seq)` making the insert-wins ledger
     append-only.
2. **Browser/WASM story (per C-lens §1.2 + B-lens §1.2, decided — not new research):** customer
   page = keyless, in-memory fold of its own order slice only (a browser tab is an ephemeral
   cache, never an authority — Safari 7-day eviction); courier **installed PWA** may cache with
   OPFS (`opfs-sahpool` VFS, Safari ≥16.4) but its record of truth is always re-fetchable signed
   envelopes from the node. No wa-sqlite/OPFS on the critical path for MVP.
3. **Replay/verify on open:** node start = walk each stream, verify hash chain + signatures,
   fold to projections, compare `head_hash`.

**VbM proof:** crash-restart drill — `kill -9` the node mid-append under a load script, restart,
replay; the folded state hash equals a byte-identical oracle fold, and either the interrupted
append is absent entirely or present exactly once (atomicity). **RED case:** flip one byte of one
`event_bytes` row on disk (or delete a middle event) ⇒ the hash-chain/signature verification at
open detects it, the node **refuses to serve that stream** and reports the exact seq — a divergent
replay can never silently serve. Second RED: reorder two events ⇒ `prev_hash` mismatch detected.

**Effort:** 5–7 sessions. **Depends on:** W (envelope bytes). **Feeds:** R, X.

### Phase R — The node runtime (the missing kernel/cli/reloop; a runnable vendor node)

**Entry precondition:** W + S landable; **dowiz Phase 1 (the `kernel::decide` bypass fix) merged**
— the node's whole premise is "one real door", and today the Rust checkout never constructs
`Command::PlaceOrder` (hub review finding #1, independently re-verified by lens A —
VERIFIED-in-repo-doc). Building the node *is* the definitive fix: in the node there is no other
door to bypass.

**Scope — a new binary crate `rebuild/crates/node` (bin name `dowiz-node`)** (DESIGN-JUDGMENT:
it lives in dowiz because `decide` lives in `rebuild/crates/domain`; it imports the bebop-repo
protocol crates as libs; it must NOT be named `bebop` — that name is taken in this workspace,
VERIFIED):

| Module | Responsibility |
|---|---|
| `main.rs` | tokio runtime; config (TOML: venue id, relay addr, key vault path, fiscal mode); vault unlock; starts the three loops below. |
| `sequencer.rs` | **The single-writer loop — the heart.** One mpsc queue of verified inbound `Command` frames → `kernel::decide(&state, cmd, &ctx)` → sign each resulting `Event` into an `Envelope{seq, at, cause_hash, sig}` with the node key → append to SQLite (S) → publish EVENT frames (W). Strictly serial per stream; **this loop is the only writer** — the bypass class of bug becomes structurally impossible, and a grep-gate CI test proves no other module calls the store's append. |
| `adapters/` | Channel adapters, all funneling into the one queue: `wss_customer.rs` (storefront order intent → `Command::PlaceOrder`), `courier_iroh.rs` (claim/status/PoD commands), `telegram.rs` (owner-notify + `getUpdates` long-poll — outbound-only, CGNAT-friendly), later `onion.rs` (the .onion mirror of the same web adapter — the anonymity channel-adapter of doc 04 §5.1, additive). |
| `dispatch.rs` | Single-owner dispatch: offer → capability with `exp` (§5) → push nudge (FCM/ntfy wake-only) → re-offer on timeout. Uses `match_orders` as a pure library call; **never consults courier reputation** (red line). |
| `sync.rs` | Serves SYNC_REQ from courier/customer devices; feeds the warm-spare replica. |
| `fiscal.rs` | Queue-and-drain NIVF requests, 48h-offline-tolerant, node-only (02 §3.9). Stub in MVP if the venue keeps its certified POS. |

**Per-device runtime matrix** (decided by C-lens §1.4, restated as build targets):
vendor node = native Rust binary on an always-on box (N100 mini-PC class; the €6 relay never runs
it); courier phone = push-woken installed PWA (WASM kernel for local verify/sign) with a thin
native wrapper for reliable FCM later; customer = one-shot browser page, keyless, WSS only.
iOS/Android background walls are physics — phones are **intermittent signers, never daemons**.
No bebop2 `kernel/cli/reloop` dirs are created for this; the *node* runtime supersedes that
placeholder trio for the delivery role (the reloop wasm-KAT harness remains a bebop2-internal
crypto tool, tracked under Phase H).

**VbM proof:** end-to-end offline-then-sync drill — with the **relay killed**, a customer order
placed on the venue LAN (or replayed from a queued intent) is accepted through
adapter → `decide` → signed append; the node then reconnects and the courier device converges to
the identical stream (same head_hash) via SYNC. **RED cases:** (a) an event appended with a wrong
`cause_hash` or unsigned ⇒ fold refuses; (b) the grep-gate finds any write path outside
`sequencer.rs` ⇒ CI RED; (c) a second sequencer process pointed at the same DB is rejected by an
exclusive-writer lock (single-writer is enforced, not assumed).

**Effort:** 8–12 sessions. **Depends on:** W, S, dowiz P1. **Feeds:** X, I.

### Phase X — Settlement + dispute (the 0-line economic heart, under COD)

**Entry precondition:** R runs an order end-to-end; the COD event vocabulary (already live in
deliver-v2: `payment.method:'cash'`, `payment_outcome`, `courier_cash_ledger` hold rows — 02 §3.8
VERIFIED-in-repo-doc) frozen as the wire schema.

**Scope — new crate `bebop-settle` (bebop-repo) + `node/settlement.rs`:**

1. **Obligation ledger over signed events.** Port `ledger.rs`'s invariant kernel (Σ=0,
   content-addressed transfer ids, fail-closed) onto per-order obligation accounts:
   `customer_debt`, `courier_custody`, `vendor_due`. The COD superpower (02 §3.8): no digital
   money moves — the ledger books **obligations settled by counter-signed custody hand-offs**:
   - `Priced` (node-signed) ⇒ `customer_debt += total`
   - `CashCollected` (courier-signed, customer OTP/countersign) ⇒ debt → `courier_custody`
   - `Delivered` (courier PoD via `pod.rs:73` — the existing seam; the claim binds
     order/courier/ts/loc, replay-at-wrong-loc already RED-tested `pod.rs:153`)
   - `SettlementReceived` (node-signed + courier countersign) ⇒ custody → `vendor_due` → close;
     Σ over the order's accounts returns to 0.
   A **`CustodyHandoff{order_id, from_id, to_id, amount:Lek(i64), ts}` requires BOTH parties'
   signatures before the ledger applies it** — counter-signature replaces escrow; the transfer id
   `H(from‖to‖amount‖order‖nonce)` makes replay a no-op (exactly `ledger.rs:38`'s discipline).
   Money stays `Lek(i64)` end-to-end; the matcher's `f64` costs never cross this boundary (the
   01 §5 flag becomes a compile-visible type wall: `bebop-settle` has no `f64` in its API).
2. **Dispute state machine — `bebop-settle/dispute.rs`** (F2 spec, finally as code, cut to COD
   reality): `OPEN → EVIDENCE → AUTO → ESCALATE → SETTLE`, fail-closed: any timeout/ambiguity ⇒
   **HOLD** (obligation stays open, courier custody frozen in the books, no state advances).
   Evidence = the signed event chain + the NIVF fiscal receipt reference (a free,
   government-verifiable proof-of-sale to chain PoD to — 02 §3.8). **No JURY tier in MVP**
   (DESIGN-JUDGMENT, per 02 §3.8): couriers are vendor-employed, so the vendor↔courier
   employment relationship is the arbiter; ESCALATE = a human owner decision recorded as a signed
   `DisputeResolved` event. PoD stays **contestable-by-design** (G7): a valid PoD signature is
   evidence, never ground truth. UMA/Kleros-class external arbitration remains a Phase-2+
   research note, not build scope.
3. **NO courier scoring — enforced structurally.** `reputation.rs`'s courier path does **not**
   port into the dowiz lineage: `record_delivery/score/risk_premium` keyed by courier ids are
   left dark; the only reputation surface, if ever needed, keys on **venue/node ids**. Concretely:
   `dispatch.rs` has no reputation input (single-owner dispatch needs none), and a CI test
   asserts the settle/dispatch crates contain no reference to courier trust scores. This is also
   legally load-bearing (couriers-as-venue-staff keeps dowiz outside platform-work law —
   VERIFIED-in-repo-doc, SYNTHESIS §9 launch-legality).

**VbM proof:** a completed COD delivery drill nets the order's obligation accounts to **Σ = 0**
with both signatures present on every custody edge, asserted by a deterministic fold. **RED
cases:** (a) an unsigned or single-signed `CustodyHandoff` ⇒ ledger refuses, Σ stays ≠ 0, order
cannot close; (b) the same custody spent twice (two `SettlementReceived` against one collection,
or a replayed handoff with a new nonce) ⇒ second application rejected, double-spend surfaced;
(c) a dispute window timeout with no resolution ⇒ state is HOLD, not SETTLE (fail-closed proven).

**Effort:** 8–12 sessions (ledger-over-events 3–4, dispute machine 3–4, drills/REDs 2–4).
**Depends on:** R (events exist), Phase-5 capabilities for signer identity. **Feeds:** I.

### Phase A — Identity/authz on the wire (capabilities replace JWT/RLS)

**Entry precondition:** W (frames carry signature blocks). Can be built in parallel with S/R.

**Scope — new crate `bebop-cap`:**

1. **Capability token** = a borsh-canonical struct
   `{issuer_id, subject_id, scope, nonce, iat, exp, sig[]}` — scope grammar starts minimal:
   `order:<id>:deliver`, `order:<id>:read`, `venue:<id>:roster`. Issued by the vendor node key at
   enrollment/dispatch; verified anywhere by signature + `exp` + nonce — **no DB, no network**
   (the ADR-0013 per-frame guard mapped 1:1 per C-lens §3.3: admission = capability present;
   per-frame re-authz = verify sig+exp on every frame at fan-out; revocation = signed
   `CAP_REVOKE` frame gossiped, unseen-but-unexpired ⇒ withhold — fail-closed, exactly the
   tri-state `RelayGuard` semantics, now stronger offline).
2. **HYBRID-ONLY enforcement in the verifier, not in policy prose:** a value-bearing capability
   or event signature verifies **iff the classical (audited host-crate Ed25519) signature
   verifies**; an ML-DSA half, when present, is additionally checked and its failure is also
   fatal — but PQ alone is never sufficient. This is the G09 posture (audited half alone
   suffices; PQ is additive) compiled into `bebop-cap`'s verify function. bebop2's hand-rolled
   primitives stay off the mandatory path until Phase H clears them.
3. **Enrollment:** courier onboarding = an in-person trust event; the owner mints the courier's
   self-cert id into the venue roster stream (02 §3.7 — no KYC oracle, DANGER #4 sidestepped).
   Customer keeps OTP + opaque track-token (keyless — the one-shot browser holds no vault).
   Key loss = re-enrollment, honestly (C-lens §4.2); a lost courier key gets a signed roster
   revocation, not a reset.

**VbM proof:** an **expired or forged capability is rejected offline** — the verifying process
runs with networking disabled and no DB, and still (a) accepts a fresh valid capability,
(b) rejects `exp < now`, (c) rejects a tampered scope, (d) rejects a wrong-issuer signature.
**RED case (the hybrid rule's own falsifier):** a capability with a *valid* ML-DSA half but an
*invalid/absent* Ed25519 half MUST be rejected — if it verifies, the hybrid gate is broken.

**Effort:** 4–6 sessions. **Depends on:** W. **Feeds:** R (dispatch offers), X (signer identity).

### Phase I — Interop / second implementation (what makes it a PROTOCOL)

**Entry precondition:** W/S/R/X schemas stable enough to freeze a `v0.1` (post the first
end-to-end drill; do NOT write the spec before the wire has carried a real order — spec-first
here would freeze guesses).

**Scope:**

1. **The versioned spec** — `docs/spec/` in bebop-repo: `WIRE-v0.md` (frame bytes, codec,
   version rules), `EVENTS-v0.md` (envelope schema, the 10-status transition table verbatim,
   COD settlement vocabulary), `CAP-v0.md` (capability format + verify algorithm, hybrid rule),
   `SYNC-v0.md`. Written from the running code, each section carrying its vector references.
2. **Conformance vector set** — `spec/vectors/` (JSON + raw binary), generated by a small Rust
   `vectorgen` bin from the reference impl and committed:
   (a) canonical-bytes encode/decode pairs; (b) signature verify vectors incl. tampered/RED
   entries; (c) state-machine vectors — every legal transition AND every illegal one with the
   expected refusal; (d) matcher fingerprint vectors (same request ⇒ same fingerprint — the
   replicability claim made checkable by outsiders, `matcher.rs:100`); (e) ledger conservation
   vectors incl. the double-spend RED; (f) one **full order-lifecycle wire transcript** (every
   frame, hex) — the "speak it end-to-end" vector. This is the same KAT discipline bebop2 already
   applies to crypto (`kat/` dir — VERIFIED present), lifted to the protocol layer.
3. **The honest minimum for "someone else can speak it"** (DESIGN-JUDGMENT): spec + vectors + a
   conformance runner (a bin that takes any impl's output dir and diffs against vectors) + **one
   deliberately-thin second client, not a second node**: a TypeScript (or Python) customer-leg
   client that builds a signed order intent, speaks WSS frames, and verifies status envelopes —
   built *only from the spec*, by a session that is forbidden to read the Rust source (the real
   test of the spec). A second full node implementation is post-validation scope; claiming
   "second implementation" for anything less than the customer leg would be dishonest, and
   anything more is over-engineering before G11. This also discharges DANGER #2 (open protocol,
   closed access) with a reference alt-client.

**VbM proof:** the second client passes the full vector set and completes a live cross-impl
order round-trip against the Rust node. **RED case:** mutate any single vector byte ⇒ the
conformance runner fails that vector; and a spec-only reimplementation session that gets a
different canonical byte string for the same envelope ⇒ the spec is wrong — fix the spec, that
divergence firing at least once during I is *expected and welcome*.

**Effort:** spec+vectors 4–6 sessions; thin second client 3–5 sessions.
**Depends on:** W, S, R, X frozen at v0.1.

### Phase H — Crypto-hardening gates (the G09 ladder; cross-cutting, parallel)

**Not a build phase — a gate ladder that runs beside everything and blocks exactly one thing:**
any bebop2 hand-rolled primitive becoming load-bearing for value. Today's posture is safe by
sequencing accident (vault/pod run on audited RustCrypto crates for BOTH halves — 01 §6 VERIFIED)
— this phase makes it safe by policy + test, then earns the upgrade:

| Rung | What | Gate it opens | Status/effort |
|---|---|---|---|
| H1 | **Wycheproof vectors** wired into bebop2-core CI for the interoperable set (Ed25519/SHA/AEAD/Argon2id). C2SP/Wycheproof is actively maintained with ML-DSA (FIPS 204) and ML-KEM vector sets ([repo](https://github.com/C2SP/wycheproof), [ML-DSA PR #112](https://github.com/C2SP/wycheproof/pull/112), activity through Jan 2026 — VERIFIED web this session). | Tier-1 → Tier-2 candidacy for the classical set | 2–3 sessions |
| H2 | **FIPS interop re-derivation of the PQ pair**: ML-KEM keys to NTT domain; ML-DSA uniform RejNTTPoly sampling + λ/4=48-byte challenge + FIPS serialization (pk 1952B). Until this, the PQ set is bespoke and can never match any vector (01 §6.1-2 VERIFIED). | Makes H3 possible at all | ~15–25 days (G09-D1's own estimate, VERIFIED-in-repo-doc) |
| H3 | **Differential-vs-oracle**: bit-exact agreement with the audited `ml-kem`/`ml-dsa` RustCrypto crates + NIST's static ACVP gen-val JSON vectors (usable offline from [usnistgov/ACVP-Server](https://github.com/usnistgov/ACVP-Server/releases) — ACVTS live for FIPS 203/204/205 since 2024-08-13, [pages.nist.gov/ACVP](https://pages.nist.gov/ACVP/) — VERIFIED web this session). This is the unified plan's own G10 gate ("ACVP oracle before protocol keys minted"). | PQ half may join hybrids for long-lived identity | 3–5 sessions after H2 |
| H4 | **Constant-time remediation + measurement**: fix the Ed25519 secret-dependent `scalar_mul` branch (`sign.rs:666-680` — VERIFIED-in-repo-doc 01 §6.3) with a ladder; remove secret-dependent division in ML-KEM compress/decompress (KyberSlash class); dudect/timing harness in CI. | Tier-2 (hybrid half guards value) | 4–6 sessions |
| H5 | **External audit** (out-of-house, paid). | Tier-3: a bebop2 primitive may guard value **alone**. Nothing in this blueprint waits for H5. | external; calendar-months |

**VbM proof per rung is intrinsic** (vectors ARE the falsifiable proof; the RED entries ship in
the vector files). **The standing rule until H4+H5:** every signature that guards money,
identity, or capability MUST verify on the audited classical half (enforced in code by Phase A's
verifier — which is itself RED-tested).

---

## 3. Critical path, total effort, and sequencing against product validation

### 3.1 The honest dependency spine

```
            (dowiz P1: decide-bypass fix — 4–6 sessions, already committed policy)
                                   │
   W wire (6–8) ──────────────┬────┤
   A capabilities (4–6) ──────┤    │
                              ▼    ▼
                     S storage (5–7)
                              │
                              ▼
                     R node runtime (8–12)   ← G11-GATED for production traffic
                              │
                              ▼
                     X settlement/dispute (8–12)
                              │
                              ▼
                     I spec + vectors + 2nd client (7–11)

   H crypto ladder (H1 2–3 · H2 15–25d · H3 3–5 · H4 4–6 · H5 external) — parallel, gates only
   the bebop2-primitives-guard-value upgrade; blocks NOTHING above.
```

**Total protocol-side effort: ~42–62 focused sessions** (W6–8 + S5–7 + R8–12 + X8–12 + A4–6 +
I7–11 + H1/H3/H4 in-repo rungs ~9–14; H2's 15–25 days and H5 are separate lanes). For calibration:
`D-transition-blueprint.md` priced the full local-first arc at ~30–45 sessions — that figure
covered the product cutover (P0–P5) *without* dispute, interop, or the crypto ladder; this
blueprint prices the **protocol completion** including them. The two overlap heavily in the
middle (S≈P2, R≈P4): the combined program is realistically **~50–70 sessions**, not the sum.

### 3.2 What may run NOW (independent of dowiz product validation) vs what waits for G11 GREEN

The ratified gate (living memory, SYNTHESIS §5-6): rungs above P1 wait for **G11 GREEN** — the
first real non-operator order. The honest split is **library-vs-production-traffic**, not
build-vs-not-build:

**Independent — start any time, zero pivot risk (all are bebop-repo library/spec work or
dowiz-P1-equivalent):**
- **dowiz P1** (the decide-bypass fix) — wanted under every future; already the agreed Phase 1.
- **Phase W** as a library crate + two-machine drill: it touches no dowiz production surface,
  and its VbM runs on throwaway hosts. Deleting the zenoh stand-in illusion has standalone value.
- **Phase A** (capabilities): pure library + offline verifier.
- **Phase H1** and **H2/H3/H4** (crypto ladder): entirely bebop2-internal; H2 is long-pole —
  starting it early is the only way it's ready when needed.
- **Phase I's vectorgen discipline** (writing vectors as W/S land, before the spec doc).

**Gated on G11 GREEN (first real order) — because they put the protocol in front of real money
and real users:**
- **Phase R in production** (the node as the venue's actual door — this is ladder rung P4's
  "money single-writer on device"; building the binary and drilling it on staging is fine,
  cutting a venue over is not).
- **Phase X in production** (real COD obligations in the signed ledger).
- **Phase I's spec freeze + second-client** (freezing v0.1 before one real order has flowed
  through the vocabulary would fossilize guesses; the spec is cheap to draft, expensive to
  version).

**The sequencing sentence:** run W + A + H now as protocol-library lanes while the product side
chases the first real order; S and a staging-only R can follow immediately behind; nothing
touches a paying venue until G11 GREEN, and nothing bebop2-hand-rolled touches value until H4.

---

## 4. Top risks + the hybrid-crypto gate (standing)

| # | Risk | Sev | Mitigation baked into the plan |
|---|---|---|---|
| 1 | **Building the protocol instead of getting order #1** (serial-pivot #5 — the corpus's own deepest risk, SYNTHESIS §1/§6). | **Highest** | The §3.2 split is the mitigation: only library lanes run pre-G11; the moment protocol work starts displacing validation work, stop protocol work. This blueprint deliberately prices phases so the operator can see what is being traded. |
| 2 | **Minting long-lived identities before ML-DSA interop (G10/H2-H3).** Keys minted with the bespoke PQ scheme can never verify against FIPS peers; re-keying a live network is misery. | High | Hybrid rule makes the classical half the durable anchor; capability `exp` keeps authz short-lived by construction; design rule: **no long-lived PQ-half commitment until H3 passes** — the PQ slot in the frame stays optional-additive. |
| 3 | **Single-writer node availability** — "vendor's phone dies at dinner rush" (B-lens §4.3). | High | R ships with the warm-spare lane (Litestream follow-mode / second device) from day one; the always-on box (not a phone) is the recommended anchor (02 §7 risk 3). The kill-the-relay drill is in R's VbM; add a kill-the-node drill before any venue cutover. |
| 4 | **iroh dependency risk** (1.0 is one month old; n0 is a startup; public relays are dev-only). | Med | Self-hosted relay from day one (relay research §2a); iroh confined behind `WireTransport` with `transport-mem` and WSS as living alternates — the port is test-proven swappable (`matcher.rs:306` pattern, kept). |
| 5 | **Dispute/settlement scope creep** (jury tiers, DLT, threshold oracles — all named in the L3/L4 vision). | Med | X is cut to COD + employment-arbiter + fail-closed HOLD. The threshold-verifier and any DLT remain Phase-2+ *vision*, not this blueprint's scope (C8: over-engineering is the #1 enemy). |
| 6 | **Bus-factor-1 planning layer**: the v3 unified blueprint and the F1–F4 set are still untracked files on one machine (01 §5, re-verified — still `??` in git status at `84d5adc`), and bebop HEAD moves mid-session (twice observed today). | Med | Flagged for the operator (a commit is a one-liner someone with write intent must make — this program is read-only). Blueprint phases pin schemas by **content (vectors)**, not by HEAD, so drift is detectable. |
| 7 | **Namespace/two-cores confusion**: `rebuild/crates/bebop` (dowiz agent core) vs bebop-repo's `crates/bebop` (protocol libs) vs `bebop2/core` (crypto). | Low-Med | Distinct crate names (`bebop-wire`, `bebop-cap`, `bebop-settle`, `dowiz-node`) declared in W/R scope; a workspace that won't compile is at least a loud failure. |
| 8 | **`f64` leaking into money** at the matcher/settlement boundary (01 §5). | Low-Med | X's API-level type wall (no `f64` in `bebop-settle`); schema-lint banning REAL in money columns (S). |
| 9 | **GDPR erasure across replicas** is crypto-shred + tombstones — weaker than a central DELETE (B-lens §2.3). | Low (pilot) | Per-order PII envelope + crypto-shred is already the ratified anonymity design (doc 04); counsel review before scale, not before pilot. |

**The hybrid-crypto gate, restated once, as the closing rule:** until the H-ladder clears
(Wycheproof + interop re-derivation + differential-vs-ACVP-oracle + constant-time + external
audit), **no bebop2 hand-rolled primitive guards value alone**. Every value-bearing signature —
event envelopes, capabilities, custody hand-offs, PoDs — must verify on an externally-audited
classical implementation (today: the RustCrypto host crates that `vault.rs` already uses —
VERIFIED); the PQ half is additive hardening, never the anchor. Phase A compiles this rule into
the verifier and Phase W bakes it into the byte format, each with its own RED case — so the gate
is enforced by tests that can fail, not by prose that can't.

---

*Produced 2026-07-11 (late evening) from: `01-bebop2-protocol-state.md` (+ live re-verification at
HEAD `84d5adc` this session), `SYNTHESIS.md`, `02-local-first-architecture.md`, `B-data-sync.md`,
`C-runtime-transport-identity.md`, `D-transition-blueprint.md`, `04-anonymity-mesh-messenger-revision.md`,
`docs/research/2026-07-11-relay-hetzner-tailscale-mesh.md`, bebop-repo
`UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3` + `bebop2/ARCHITECTURE.md` + source reads of
`{matcher,pod,ledger,reputation,zenoh}.rs`, the living-memory rulings, and fresh web checks
(Wycheproof/C2SP, NIST ACVP — cited inline). Both repos left exactly as found; this file is the
only artifact created.*
