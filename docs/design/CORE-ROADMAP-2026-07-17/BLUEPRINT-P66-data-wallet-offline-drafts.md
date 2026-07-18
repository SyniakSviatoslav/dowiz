# BLUEPRINT P66 — Data wallet & offline drafts: on-device self-custody store, single-writer LWW drafts, query-before-replay reconnect, Signal-style QR transfer with the mandatory anti-phishing confirmation (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Component:
> **CUSTOMER-IDENTITY / OFFLINE-RESILIENCE (client-side)**. Wave **W2** of the CORE roadmap
> (`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5). Scope = the §5 W2 table row **P66**:
> the on-device data wallet + checkout autofill (§16.23), the single-writer **last-write-wins**
> offline draft (NO CRDT — R4 §3.1), the `Draft`→`PaymentInflight` state machine with
> **query-status-by-key before replay** (X6 — **consumes P60's idempotency contract, never
> redefines it**), the two per-platform storage mechanisms (Tauri `tauri-plugin-store` explicit
> `save()` / web IndexedDB + `online`-event outbox), and the **Signal-style QR wallet transfer**
> (X25519 ECDH → SHAKE256 KDF → AES-256-GCM, animated-QR transport) with the **mandatory
> anti-phishing confirmation** Signal's own 2025 incident forced (R4 §6.4). Grounds every design
> claim in R4 (`docs/research/OPUS-R4-ORDERFLOW-COURIER-NOTIFICATIONS-2026-07-18.md` §3 + §6) + live
> kernel code. Structural template + rigor precedent: `BLUEPRINT-P60-payment-adapter-core.md`,
> `BLUEPRINT-P57-canvas-text-input.md`, `BLUEPRINT-P51-open-map-routing.md`.
>
> **Operator rulings applied as inputs, NOT re-litigated** (all CLOSED per the task + synthesis):
> the customer identity is a **client-side data wallet** — no dowiz-central account, no per-venue
> account (§16.23); the wallet is **device-resident, self-custody, Signal-style QR device-linking
> for transfer, loss is the user's own responsibility** (§16.47); offline mid-checkout is held as a
> **local draft, restored on reconnect, payment fires only when back online** (§16.52 / §16.14);
> the idempotency contract is **owned by P60, consumed here** (X6); **NO break-glass** — the
> §4-B self-custody-severity fork is **CLOSED in favour of "self-custody is absolute"**: a lost
> wallet key/device means the saved data is genuinely gone, by construction, not a gap.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding:
**the entire wallet-transfer crypto composes from primitives the kernel already vendors** — X25519
ECDH, SHAKE256 (the sovereign KDF, replacing Signal's HKDF-SHA256 with an in-tree hash), and
AES-256-GCM are all present today behind the `pq` feature; the idempotency contract this blueprint
depends on is already fully specified by P60; the autofill target (`TextField::set_value`) is
already exposed by P57. P66 is a **client-side** greenfield build that **re-derives no crypto and no
idempotency law** — it composes existing contracts.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| **X25519 raw ECDH primitive EXISTS**: `x25519(k: &[u8;32], u: &[u8;32]) -> [u8;32]` (scalar-mult, the Diffie-Hellman core) | `kernel/src/pq/x25519.rs:16` | **VERIFIED — the wallet-transfer ECDH REUSES this; NO new `x25519-dalek` dep (§4.5)** |
| **SHAKE256 XOF/PRF EXISTS**: `shake256(input, out)`, `shake256_xof(seed,i,j,len)`, `prf(s,b,len)` (FIPS-203 seed-expansion), `xof_g`/`xof_h`/`xof_j` | `kernel/src/pq/keccak.rs:139`, `:145`, `:180`, `:156-170` | **VERIFIED — the transfer KDF REUSES SHAKE256 as the sovereign substitute for Signal's HKDF-SHA256; NO new `hkdf`/`sha2` dep (§4.5 DECART)** |
| **AES-256-GCM AEAD dep ALREADY in tree**: `aes-gcm = "0.10.3"` under the `pq` feature (with `curve25519-dalek = "4"`) | `kernel/Cargo.toml:85-86`, feature `:50` | **VERIFIED — the transfer AEAD REUSES this dep; matches R4 §6.2's "prefer AES-GCM single AEAD over Signal's AES-CBC + separate HMAC"** |
| **Domain-separated hash `sha3_256` EXISTS** (append-only event-log primitive) | `kernel/src/event_log.rs:30` | **VERIFIED — the idempotency-key derivation (P60 §3) REUSES this exact function** |
| Event-sourced **decide/fold discipline** EXISTS: `commit_after_decide` (decide-before-commit), `MeshEvent`, content-addressed local store | `kernel/src/event_log.rs:366`, `:134`, module doc `:4` ("running the kernel `decide`/`fold` locally") | **VERIFIED — the Draft + Transfer machines model transitions as events + a pure `decide`, tests assert on event sequences (item 3)** |
| **Money type + `Currency::Eur` EXIST**, i64 minor units, no f64 | `kernel/src/money.rs:29` (`Currency`), `:33` (`Eur`), `:59` (`Money`) | **VERIFIED — the wallet stores a payment-method *reference*, NEVER money and NEVER card data (§4.1); a cart line reuses `Money`** |
| **P60 owns the idempotency contract** consumed here: `IdempotencyKey(pub [u8;32])` "minted at draft creation by P66, domain-separated SHA3 (`event_log::sha3_256` of `b"dowiz.pay.idem\0" ‖ order_id ‖ wallet_id ‖ nonce`)"; trait methods `create_with_key(key, plan) -> ClientHandoff` + `query_status_by_key(key) -> PaymentStatus`; the `IdemLedger` reconnect authority; `PaymentStatus { NoneYet, IntentCreated, Authorized, Captured, Voided, Refunded, Failed }` | `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P60-payment-adapter-core.md` §3 (`IdempotencyKey` L180, trait L255-272, `PaymentStatus` L197-200), §4.2 (`IdemLedger` + the per-provider normalization gap) | **VERIFIED read in full — P66 CONSUMES this verbatim; §2 forbids redefining it (X6)** |
| **P57 exposes the autofill seam** P66 consumes: `TextField::value() -> &str` ("for P66 snapshot / consumer submit"), `TextField::set_value(&str)` ("for P66 restore / prefill, scope-gated"), `WidgetId`; P57 §2.2: "**NOT draft persistence / autofill — P66 owns** the on-device wallet + offline draft (query-before-replay). P57 exposes `value()`/`set_value()` … P57 stores nothing across sessions" | `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P57-canvas-text-input.md` §3 (`TextField` L253-266), §2.2 (L162-164) | **VERIFIED — the autofill projection (§4.1) calls `set_value`; the consumer-submit read calls `value()`. P57 is the upstream text-entry dependency, P66 the consumer** |
| **`kernel/src/qr_code.rs` does NOT exist yet** — the pure in-kernel QR encoder is P53's build item ("`kernel/src/qr_code.rs`, no new deps", feeding a share panel) | repo `ls kernel/src/qr_code.rs` → absent this pass; `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:1639-1665` (P53 owns it) | **VERIFIED — P66 CONSUMES P53's QR *encoder* for animated frames; QR *decode* (camera) is a platform port P66 defines (§4.5). Named dependency, not a silent gap** |
| **No wallet/draft/store code anywhere** — grep `DataWallet\|OfflineDraft\|tauri_plugin_store\|indexed_db\|data.wallet` over `--include=*.rs` (excl. docs) → **0 hits** | repo-wide grep this pass | **VERIFIED — P66 is greenfield: a new `wallet` crate + a new out-of-core `wallet-adapters` crate (§2)** |
| **`TokenBucket` EXISTS**, zero-dep, degrade-closed (referenced for the client-side draft-submit cap) | `kernel/src/token_bucket.rs:34` (`new`), `:74` (`try_acquire`) | VERIFIED — a client-side single-outstanding-intent predicate reuses the same discipline (server-side abuse is P60 M7, not re-owned here) |
| The **no-card-data compile firewall pattern** (identifier-absence scan, `concat!`-assembled forbidden tokens, hard test failure) | `kernel/src/ports/payment.rs:508-560`; extended by P60 §4.1 (`no_card_data_type_in_core`) | **VERIFIED — P66 EXTENDS this to the wallet crate (`no_card_data_in_wallet`, §4.1) AND adds a `no_break_glass_in_wallet` scan (§4.7)** |
| R4 research verdicts (CRDT ruled out for single-writer draft; two-runtime storage; the `Draft`/`PaymentInflight` reconnect machine; Signal `ProvisioningCipher` crypto; the 2025 QR-phishing incident + confirmation fix) | `docs/research/OPUS-R4-ORDERFLOW-COURIER-NOTIFICATIONS-2026-07-18.md` §3.1-3.3, §6.1-6.4 | VERIFIED read in full — P66 consumes its findings, does not re-research |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Research verdicts consumed (R4 §3 + §6, condensed — cited, not re-derived) + the closed rulings

R4 is the research substrate; its findings are inputs here. The load-bearing ones, each already
reconciled against the operator rulings:

1. **CRDTs are ruled out — this is R4's explicit load-bearing verdict (R4 §3.1).** A checkout draft
   is *one device, one user, one order*: "last write" and "only write" are the **same event**.
   `automerge`/`yrs` are excellent libraries whose *entire* value is reconciling **concurrent edits
   from multiple writers** — machinery that here "never fires — pure liability (binary doc format,
   bigger dependency, harder debugging). **A plain versioned JSON blob with last-write-wins is
   strictly correct here.**" P66 builds exactly that; CRDT is barred in anti-scope (§2), not
   "considered and deferred."

2. **The reconnect machine is `Draft` → `PaymentInflight` with query-before-replay (R4 §3.3).** The
   dangerous case is not "draft not submitted"; it is "payment request maybe went out before the
   socket dropped." R4 prescribes exactly: (a) generate an idempotency key **client-side at
   draft-creation time, store it *with* the draft, never regenerate**; (b) a mini state machine
   `Draft → PaymentInflight` set **optimistically the instant the request is sent**; (c) on
   reconnect **branch on that state** — `Draft` → local resume; `PaymentInflight` → **query
   order/payment status by idempotency key first, never blind-replay** — "the request may have
   succeeded server-side while the client never saw the ack. This is what prevents a restored draft
   from double-charging." This dovetails with §16.49 (payment fires client-side, hub sees only a
   token) and §16.14 (no dowiz-central state).

3. **Storage is two runtimes, same shape (R4 §3.2).** Tauri 2.x: `tauri-plugin-store` 2.4.3, with
   an **explicit `store.save()` on each meaningful field edit** — *do not* trust the debounce /
   graceful-exit autosave, "since the exact scenario being defended against is a crash/kill
   mid-checkout." Web/wasm: **IndexedDB** via `idb`/`indexed_db_futures` (`gloo-storage` has no
   IndexedDB), `navigator.storage.persist()` to resist eviction, and — because Background Sync is
   Chromium-only — an **`online`-event + retry-with-backoff outbox** is the mandatory path,
   Background Sync only an optional fast-path.

4. **The wallet transfer is Signal's device-TRANSFER, stripped of the server (R4 §6.1-6.2).** What
   maps directly: ephemeral **X25519 ECDH** + a KDF-derived symmetric key, a **QR-carried ephemeral
   pubkey as the authenticated out-of-band bootstrap**, a **version-tagged encrypted envelope**.
   What R4 says to **drop**: the **server-relayed provisioning mailbox** ("there is no central dowiz
   server, §16.14 — use a direct local transport or re-encode as a second animated QR"), **Sesame**
   ongoing multi-device bookkeeping, and the server-issued `provisioningCode` ("**physical QR
   proximity *is* the authorization**"). And the **simplification**: prefer **AES-GCM** (single
   AEAD) over Signal's AES-CBC + separate HMAC. Building blocks are all RustCrypto (R4 §6.3): "vendor
   the primitive crates … don't depend on libsignal wholesale" — which P66 satisfies for free by
   reusing the kernel's already-vendored `pq/x25519` + `pq/keccak::shake256` + `aes-gcm` (§0, §4.5).

5. **The anti-phishing confirmation is a real lesson from a real 2025 incident (R4 §6.4).** "In 2025,
   Russia-aligned actors phished victims into scanning **attacker-controlled** linking QR codes
   disguised as group invites, silently hijacking accounts; Signal's fix was an **explicit user
   confirmation step** before a new device is granted access." Implication R4 states verbatim: "the
   crypto is necessary but not sufficient — the UX must make the **scan direction and a user-visible
   confirmation** explicit ('You are about to copy your saved details to a NEW device — confirm?') …
   Bake the confirmation step into the Tier-3 UX spec, not just the crypto." P66 makes this a
   **structural, mandatory state** (§4.6), not a UX suggestion.

**Closed rulings, applied as fixed inputs (not re-opened):**
- **§16.23 / §16.47 / §16.52** — client-side wallet, device-resident self-custody, offline draft
  restore. Verbatim scope from the master roadmap §16 (read this pass).
- **§4-B CLOSED → self-custody is absolute (NO break-glass).** The synthesis §4-B flagged
  "self-custody severity" as an open operator fork ("self-custody is absolute" vs "a named narrow
  recovery path exists"). The task states the operator has **closed it in favour of absolute
  self-custody**: no `dowiz_break_glass` recipient, no escrow, no recovery service — a lost key means
  the wallet is gone. P66 enforces this **by construction** (§4.7), and states it plainly as a
  deliberate tradeoff (§16.47's "loss is explicitly the user's own responsibility"), never a gap.
- **X6 CLOSED → the idempotency contract is P60's, consumed here.** P66 mints the key and calls
  `create_with_key`/`query_status_by_key`; it does **not** define the contract (§2, §4.3).
- **§4-E is the engineering decision this blueprint makes (not an operator gate):** the transfer
  return transport (animated-QR vs BLE vs same-LAN) + the confirmation UX. P66 **chooses
  animated-QR** and justifies it with a size budget (§4.5); the confirmation is §4.6.

---

## 2. Scope — what P66 owns vs deliberately does NOT

**P66 owns (build items §4):** two new client-side crates — `wallet` (pure logic + ports; path-dep
on `dowiz-kernel` for the reused crypto/hash/money primitives) and `wallet-adapters` (out-of-core,
holds the platform deps `tauri-plugin-store`/`idb`/QR-decode behind the firewall, exactly the P60
`payment-adapters` split).

| Item | Content |
|---|---|
| M1 | **On-device wallet store + checkout autofill** (`wallet/src/record.rs`): `WalletRecord` (versioned JSON, LWW), the `WalletStore` port, the autofill projection into P57 `TextField::set_value`, and the **no-card-data firewall** extended to the wallet crate (payment-method *reference* only) |
| M2 | **Per-platform storage adapters** (`wallet-adapters`): `TauriStoreAdapter` (`tauri-plugin-store`, **explicit `save()` per edit**) + `IdbStoreAdapter` (`idb` + `navigator.storage.persist()` + `online`-event listener), both behind `WalletStore` |
| M3 | **`Draft` → `PaymentInflight` state machine** (`wallet/src/draft.rs`): idempotency key **minted at draft creation** (P60's derivation), the pure `decide_draft`/`fold` saga, the draft record shape |
| M4 | **Reconnect outbox — query-before-replay** (`wallet/src/outbox.rs`): on reconnect, branch on state; `PaymentInflight` ⇒ **`query_status_by_key` FIRST**, never blind resubmit. **The falsifiable double-charge-prevention test lives here** (§6) |
| M5 | **Signal-style QR wallet transfer** (`wallet/src/transfer.rs`): ephemeral X25519 ECDH (reuse `pq/x25519`), SHAKE256 KDF (reuse `pq/keccak`), AES-256-GCM AEAD (reuse `aes-gcm`), version-tagged envelope; **animated-QR transport chosen + size-budgeted** (§4-E); encode via P53's `qr_code.rs`, decode via a `QrScanPort` platform adapter |
| M6 | **The mandatory anti-phishing confirmation** (`wallet/src/transfer.rs`): a `TransferState` machine where sealing/accepting is **unrepresentable without an explicit `ConfirmTransfer` on the source device** + a short-fingerprint (SAS) match; direction-explicit UX copy (R4 §6.4) |
| M7 | **No break-glass — self-custody absolute** (§4-B closed): structural absence of any recovery/escrow/dowiz-recipient type, enforced by a `no_break_glass_in_wallet` identifier scan; stated as a deliberate tradeoff |

**P66 explicitly does NOT own:**

- **NOT the idempotency contract — P60 owns it (X6).** P66 **mints** the `IdempotencyKey` at draft
  creation (using P60 §3's derivation `event_log::sha3_256(b"dowiz.pay.idem\0" ‖ order_id ‖ wallet_id
  ‖ nonce)`) and **calls** `create_with_key` / `query_status_by_key` (P60 §3 trait). It does **not**
  define, rename, or fork these. A diff that introduces a second idempotency key type or a parallel
  "query status" method in the wallet crate is a **scope violation** (X6: "neither blueprint may
  define it independently"). **Consumer.**
- **NO CRDT, EVER (R4 §3.1 — the explicit research verdict).** No `automerge`, no `yrs`, no op-log,
  no tombstones, no merge machinery. Single-writer LWW is strictly correct; adding CRDT would be
  over-engineering the research explicitly ruled out. A diff that pulls a CRDT crate into the wallet
  is a **scope violation regardless of test state**.
- **NO break-glass / recovery / escrow, EVER (§4-B closed).** No `dowiz_break_glass_pubkey`, no
  recovery key, no cloud escrow, no "reset my wallet from the server." A lost key = data gone
  (§16.47). A diff introducing any recovery path is a **scope violation** (self-custody red-line).
- **NO raw card data, EVER — the PCI red-line is P60's, honored here structurally.** The wallet
  stores a `PaymentMethodRef` (an opaque provider-scoped payment-method id, e.g. Stripe `pm_…`),
  **never a PAN/CVV/expiry**. The no-card-data firewall (§4.1) makes a card field in the wallet crate
  a hard build failure. (R4 §3.3 / §16.49: the client tokenizes with the provider; only a reference
  is ever stored.)
- **NOT the payment adapter, webhook, or N-leg atomicity** — P60 owns all of it. P66 hands P60 a
  `ClientHandoff` request via `create_with_key`; the webhook (P60 §4.4) — never the client — is the
  sole writer of capture truth. **Consumer.**
- **NOT the text editor / caret / keyboard** — P57 owns `TextField`, `EditCmd`, cursor/selection.
  P66 only calls `value()`/`set_value()` at the autofill and submit boundaries (§4.1). **Consumer.**
- **NOT the checkout wizard UI / the card moment** — P69 owns the storefront narrative arc and
  invokes the Path-C redirect / Path-B sheet. P66 supplies the wallet autofill values and the draft
  machine P69 drives. **Consumer-facing dependency of P69.**
- **NOT the QR *encoder*** — P53 owns `kernel/src/qr_code.rs` (pure, no new deps). P66 consumes it
  for animated frames and defines the `QrScanPort` decode side (a platform adapter — camera). If P53
  has not landed when P66 builds, P66's transport lane is gated on it (named, §4.5), not blocked
  silently.
- **NOT the capability-cert chain (P59) crypto.** X8 is explicit: the wallet transfer **shares the
  primitive *family*** (X25519/KDF/AEAD from the kernel `pq` module) but is **deliberately a
  separate, simpler mechanism — do not merge wallet-transfer crypto into the cert chain.** P66
  reuses the *primitives* (`pq/x25519`, `pq/keccak::shake256`, `aes-gcm`) and the *self-custody
  framing* + the *§6.4 anti-phishing lesson*; it does **not** touch `HybridSigner`, biscuit blocks,
  or the cert chain. **Sibling, not a merge.**
- **NOT server-side abuse limiting** — P60 M7 owns the checkout-intent `TokenBucket` and the
  single-outstanding-intent cap at the hub. P66's client-side "one open `PaymentInflight` at a time"
  predicate (§4.3) is a UX guard, not the security boundary (the security boundary is P60's,
  server-side, degrade-closed).

**Reconciliation with §16.54 (honest, not silently widened):** §16.54 extends the offline-draft
principle to "the whole installed Tauri client is cache-first with full offline functionality." P66
builds the **checkout draft + wallet** slice of that (the launch-critical one). The general
menu/status offline cache is P69's storefront concern reusing the same `WalletStore`/outbox
shapes — named here so nobody double-owns it, not built here.

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── wallet/src/record.rs — NEW crate `wallet`, client-side, pure ────────────────
//  Depends on `dowiz-kernel` (features = ["pq"]) ONLY for reused primitives:
//  event_log::sha3_256, pq::x25519, pq::keccak::shake256, aes-gcm, money::Money.
//  NO tauri, NO idb, NO reqwest here (those live out-of-core in `wallet-adapters`).
//  Red-proof: `no_card_data_in_wallet` (§4.1) + `no_break_glass_in_wallet` (§4.7).

/// Opaque provider-scoped payment-method reference (e.g. Stripe "pm_…"). NEVER a PAN.
/// No `pan`/`cvv`/`expiry`/`card_number` field exists or may be added (§4.1 firewall).
pub struct PaymentMethodRef(pub String);

/// A saved delivery address. Free-form lines (the catalog/address parse is not P66's).
pub struct Address { pub label: String, pub lines: Vec<String>, pub note: Option<String> }

/// The minimum contact set the customer consents to hand a hub at checkout (§16.23).
pub struct Contact { pub email: Option<String>, pub phone_e164: Option<String> }

/// THE on-device wallet. Versioned JSON, LAST-WRITE-WINS (R4 §3.1 — NOT a CRDT).
/// `rev` is a strictly-monotone local counter; on the (rare, non-required) two-tab race
/// the higher `rev` wins — single-writer makes this strictly correct, no merge needed.
pub struct WalletRecord {
    pub schema_version: u16,   // = WALLET_SCHEMA_VERSION; forward-compat gate on load
    pub rev: u64,              // monotone; ++ on every committed edit (LWW ordering key)
    pub updated_at_ms: u64,    // wall clock, advisory only (rev is the authority)
    pub wallet_id: [u8; 32],   // stable per-device client id (feeds the idem-key derivation)
    pub name: Option<String>,
    pub addresses: Vec<Address>,
    pub contact: Option<Contact>,
    pub method_ref: Option<PaymentMethodRef>,   // reference only — never card data
}
pub const WALLET_SCHEMA_VERSION: u16 = 1;

/// The on-device store port. The pure crate knows nothing about tauri/idb (bulkhead §5.3).
pub trait WalletStore {
    fn load(&self) -> Result<Option<WalletRecord>, WalletStoreError>;
    /// Persist the record. Tauri impl calls store.save() HERE (explicit, per edit — R4 §3.2).
    fn save(&mut self, rec: &WalletRecord) -> Result<(), WalletStoreError>;
    fn clear(&mut self) -> Result<(), WalletStoreError>;   // user self-delete (§16.58)
}
pub enum WalletStoreError { Io(String), Corrupt, VersionTooNew(u16), QuotaExceeded }

// ── wallet/src/draft.rs — the Draft → PaymentInflight machine (R4 §3.3) ──────────

pub struct DraftId(pub [u8; 32]);   // content id of the draft at creation

/// The idempotency key type is P60's — RE-EXPORTED, never redefined (X6).
pub use dowiz_kernel::ports::payment_provider::IdempotencyKey;

/// The offline checkout draft. Held locally; restored on reconnect (§16.52).
pub struct CheckoutDraft {
    pub draft_id: DraftId,
    pub order_id: String,           // the target hub's order id (once assigned)
    pub cart: CartSnapshot,         // integer Money lines (kernel money::Money), no f64
    pub wallet_fill: WalletFill,    // the autofilled name/address/contact/method_ref
    pub idem_key: IdempotencyKey,   // minted ONCE at creation, NEVER regenerated (R4 §3.3)
    pub state: DraftState,
}
pub struct CartSnapshot { pub currency: dowiz_kernel::money::Currency,
                          pub lines: Vec<CartLine> }
pub struct CartLine { pub leaf_id: String, pub qty: u32, pub unit: dowiz_kernel::money::Money }
pub struct WalletFill { pub name: Option<String>, pub address: Option<Address>,
                        pub contact: Option<Contact>, pub method_ref: Option<PaymentMethodRef> }

/// The exact two-state machine R4 §3.3 prescribes. A third state is unrepresentable.
pub enum DraftState {
    /// Editing locally; nothing submitted. Reconnect ⇒ resume locally.
    Draft,
    /// Payment request WAS sent (set optimistically the instant of send, before any ack).
    /// Reconnect ⇒ query_status_by_key FIRST, never blind replay (§4.4).
    PaymentInflight,
}

/// Event-sourced saga (item 3 — tests assert on the sequence, not just end-state).
pub enum DraftEvent {
    DraftCreated { draft_id: DraftId, order_id: String },
    IdemKeyMinted { key: IdempotencyKey },     // exactly once, at creation
    FieldFilled { field: FilledField },
    PaymentSubmitted,                          // Draft -> PaymentInflight (optimistic)
    StatusResolved { status: PaymentStatus },  // from query_status_by_key
    DraftCleared { draft_id: DraftId },        // terminal: committed or user-abandoned
}
pub enum FilledField { Name, Address, Contact, Method }

/// Re-exported from P60 (X6) — the client NEVER self-certifies; it reads this from the hub.
pub use dowiz_kernel::ports::payment_provider::PaymentStatus;

/// On reconnect, the branch decision (R4 §3.3). Pure; no I/O.
pub enum ReconnectAction {
    ResumeLocalEditing,                 // state == Draft
    QueryThenDecide,                    // state == PaymentInflight -> query_status_by_key FIRST
}
pub fn decide_reconnect(state: &DraftState) -> ReconnectAction;

/// After the query resolves, what the client does — NEVER a blind resubmit on a live intent.
pub enum PostQueryAction {
    ShowSuccessClearDraft,   // Captured/Authorized/IntentCreated: the intent LIVES — do not resubmit
    ResubmitSameKey,         // NoneYet/Failed: safe to (re)submit with the SAME idem_key
}
pub fn decide_post_query(status: &PaymentStatus) -> PostQueryAction;

/// Client-side UX guard (NOT the security boundary — that is P60 M7, server-side).
pub const MAX_OPEN_INFLIGHT_DRAFTS: usize = 1;

// ── wallet/src/transfer.rs — Signal-style QR transfer (R4 §6) ────────────────────
//  ALL crypto REUSED from the kernel `pq` module — zero new crypto deps (§4.5 DECART).

pub const TRANSFER_QR_TTL_S:  u32   = 120;    // Signal's ~1–2 min link-code window (R4 §6.1)
pub const MAX_TRANSFER_BYTES: usize = 4096;   // sealed-envelope ceiling (size budget §4.5)
pub const QR_FRAME_PAYLOAD_MAX: usize = 2953; // QR v40 binary capacity → ≤2 animated frames
pub const AEAD_KEY_LEN:   usize = 32;         // AES-256-GCM
pub const AEAD_NONCE_LEN: usize = 12;
pub const FINGERPRINT_LEN: usize = 8;         // short-auth-string shown at confirmation (§4.6)
pub const TRANSFER_KDF_CTX: &[u8] = b"dowiz.wallet.transfer.v1"; // KDF domain sep (vs cert chain)
pub const TRANSFER_ENVELOPE_VERSION: u8 = 1;

/// Ephemeral X25519 keypair for ONE transfer. Generated fresh, never persisted (self-custody).
pub struct EphemeralKeypair { pub secret: [u8; 32], pub public: [u8; 32] }

/// QR-1: emitted by the NEW (receiving) device. Carries its ephemeral pubkey + a nonce + a ttl.
/// This is the out-of-band authenticated bootstrap (R4 §6.1) — read by the source device's camera.
pub struct TransferInit { pub new_device_pub: [u8; 32], pub nonce: [u8; 12], pub issued_ms: u64 }

/// QR-2 (animated): emitted by the SOURCE device AFTER confirmation. The sealed wallet.
pub struct SealedWallet {
    pub version: u8,                 // = TRANSFER_ENVELOPE_VERSION
    pub src_ephemeral_pub: [u8; 32], // source's ephemeral pubkey (the peer half of the ECDH)
    pub nonce: [u8; AEAD_NONCE_LEN],
    pub ct: Vec<u8>,                 // AES-256-GCM(ct ‖ tag) over the serialized WalletRecord
}

/// The transfer machine. The AwaitingConfirmation state is MANDATORY and UNSKIPPABLE (§4.6).
/// Producing a `SealedWallet` is ONLY reachable through `Confirmed` — sealing without an explicit
/// user confirmation is unrepresentable (the anti-phishing invariant, R4 §6.4).
pub enum TransferState {
    Idle,
    // NEW device side:
    AwaitingScanOfInit { kp: EphemeralKeypair, init: TransferInit }, // showing QR-1
    AwaitingSealed,                                                  // scanning for QR-2
    Received { rec: WalletRecord },
    // SOURCE device side:
    ScannedInit { peer: TransferInit, kp: EphemeralKeypair, fingerprint: [u8; FINGERPRINT_LEN] },
    AwaitingConfirmation { peer: TransferInit, kp: EphemeralKeypair,
                           fingerprint: [u8; FINGERPRINT_LEN] },     // MANDATORY GATE (§4.6)
    Confirmed { sealed: SealedWallet },                             // showing QR-2 (animated)
    Failed(TransferError),
}
pub enum TransferError { QrDecodeFailed, Expired, TooLarge, AeadInvalid, UserRejected, VersionUnsupported }

/// The two source-side commands. Sealing is gated on Confirm (§4.6).
pub enum TransferCmd { ScanInit(TransferInit), ConfirmTransfer, RejectTransfer }

/// The transport is CHOSEN: animated-QR (§4-E). Frames carry SealedWallet chunked to ≤ QR_FRAME_PAYLOAD_MAX.
pub trait QrEncodePort { fn encode(&self, bytes: &[u8]) -> Vec<QrMatrix>; }   // reuses P53 kernel/src/qr_code.rs
pub trait QrScanPort   { fn next_frame(&mut self) -> Option<Vec<u8>>; }       // camera; platform adapter
pub struct QrMatrix { pub size: u16, pub modules: Vec<u8> }  // 1 bit/module, P53's output shape

/// Connectivity port (web: `online` event; Tauri: tokio connectivity loop) — R4 §3.2.
pub trait Net { fn is_online(&self) -> bool; }
pub trait Clock { fn now_ms(&self) -> u64; }
```

Rejected alternatives (DECART one-liners): **a CRDT (`automerge`/`yrs`)** — rejected, single-writer
LWW is strictly correct and CRDT machinery never fires (R4 §3.1; pure liability). **HKDF-SHA256 +
`hkdf`/`sha2` crates** (Signal's literal KDF) — rejected: the kernel already vendors SHAKE256
(`pq/keccak::shake256`), a strong XOF that serves as the KDF with **zero new deps**; the KDF goal (a
PRF over the ECDH shared secret) is identical (§4.5). **AES-CBC + separate HMAC** (Signal's literal
AEAD) — rejected for AES-256-GCM (single AEAD, fewer footguns, already a dep — R4 §6.2). **A
server-relayed provisioning mailbox** (Signal's literal transport) — rejected: no central dowiz
server (§16.14); animated-QR is zero-infra (§4.5). **BLE / same-LAN transport** — rejected: platform
coverage gaps (Web Bluetooth is Chromium-only, absent on iOS Safari — the same gap as web push;
raw sockets are unavailable to a web client), whereas animated-QR works on *every* camera+screen
device (§4.5). **A `dowiz_break_glass_pubkey` recipient / recovery escrow** — rejected: §4-B closed
to absolute self-custody (§4.7). **A stringly-typed / regenerated idempotency key** — rejected: P60's
typed `[u8;32]` minted once and reused is the contract (X6); regeneration is the double-charge bug
(§4.4). **Raw card storage in the wallet** — rejected: PCI red-line, `PaymentMethodRef` only (§4.1).

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 4.1 M1 — on-device wallet store + autofill + the no-card-data firewall

New crate `wallet` (repo root), module `record.rs` per §3. The `WalletRecord` is a versioned
JSON blob (serde + serde_json, already pulled by the kernel `pq` feature) persisted through the
`WalletStore` port. **Autofill** projects the record into P57 `TextField`s at any hub's checkout:
for each field the wizard exposes a `WidgetId`, `text_field.set_value(&wallet.name/address/…)`
(P57 §3), resolving §16.23's "friction vs central-account" tension — the customer never types their
details twice and never holds a dowiz or per-venue account. **Consumer submit** reads back via
`text_field.value()` at the P69 submit boundary.

The **no-card-data firewall** — `no_card_data_in_wallet`, extending P60 §4.1's proven identifier-
absence scan (`payment.rs:508-560` pattern): the wallet crate `include_str!`s its own sources and
asserts none of `card_number`/`cardnumber`/`pan`/`cvv`/`cvc`/`expiry`/`exp_month`/`exp_year`/
`card_holder` appear (forbidden tokens `concat!`-assembled so the scan body never self-matches). A
`kernel/tests`-style integration scan covers the whole `wallet/src/` tree. **RED→GREEN teeth:**
adding a `pan: String` field to any wallet struct fails the build.

RED→GREEN: `wallet_round_trips_through_store` (save→load fidelity via a mock `WalletStore`);
`autofill_sets_textfield_values` (a mock `TextField` receives `set_value` for name/address/contact);
`method_ref_is_opaque_not_card` (the wallet holds a `PaymentMethodRef("pm_…")`, no card field
exists to populate). **Adversarial:** a test that *adds* a `cvv:` field to a wallet fixture and
asserts `no_card_data_in_wallet` fires (teeth are real); loading a `WalletRecord` with
`schema_version > WALLET_SCHEMA_VERSION` ⇒ typed `VersionTooNew`, never a silent misparse; a
corrupt/truncated blob ⇒ `Corrupt`, never a panic (bulkhead §5.3).

### 4.2 M2 — per-platform storage adapters (out-of-core, behind the `WalletStore` firewall)

New crate `wallet-adapters` (repo root, path-dep on `wallet`; the platform deps live HERE, outside
the pure crate — the P60 `payment-adapters` split). Two adapters implement `WalletStore`:

- **`TauriStoreAdapter`** over `tauri-plugin-store` 2.4.3 (R4 §3.2). `save()` calls
  `store.save()` **explicitly on every meaningful edit** — the blueprint's binding note: **do NOT
  rely on the 100 ms debounce / graceful-exit autosave**, because the exact defended scenario is a
  crash/kill mid-checkout. The native outbox (M4) is a `tokio::time::interval` connectivity loop.
- **`IdbStoreAdapter`** over `idb` (R4 §3.2; `gloo-storage` has no IndexedDB). Calls
  `navigator.storage.persist()` on init to resist eviction; registers an **`online`-event listener**
  that triggers the outbox flush (Background Sync only as an optional Chromium fast-path).

RED→GREEN (adapter crate, headless): `tauri_adapter_saves_explicitly` (a mock plugin records that
`save()` was invoked per edit, not deferred); `idb_adapter_persist_requested` (asserts
`storage.persist()` called); `online_event_triggers_flush` (a synthetic `online` event drains the
outbox). **Adversarial:** IndexedDB `QuotaExceeded` mid-save ⇒ typed `WalletStoreError::QuotaExceeded`
surfaced to the UI, draft NOT lost (kept in memory + retried); a Tauri `save()` that returns an I/O
error ⇒ typed `Io`, the edit stays queued.

### 4.3 M3 — the `Draft` → `PaymentInflight` state machine + idempotency key minting (X6)

`wallet/src/draft.rs` per §3. **The idempotency key is minted exactly once, at draft creation**,
using **P60's derivation verbatim** (X6): `IdempotencyKey(event_log::sha3_256(concat(b"dowiz.pay.idem\0",
order_id, wallet_id, nonce)))`. It is stored **in** the `CheckoutDraft` and **never regenerated**
(R4 §3.3) — this is the load-bearing rule (§4.4 shows why regeneration is the double-charge bug).
The machine has exactly two states (`DraftState`), modeled as an event-sourced saga
(`DraftEvent`), mirroring the kernel `decide`/`fold` shape (`event_log.rs:366`):

- `PaymentSubmitted` transitions `Draft → PaymentInflight` **optimistically, the instant the request
  is sent, before any response** (R4 §3.3). This is the whole point: if the socket drops between send
  and ack, the persisted state already says "we may have charged."
- The **client-side single-open guard** (`MAX_OPEN_INFLIGHT_DRAFTS = 1`) refuses a second concurrent
  submit — a UX guard, not the security boundary (that is P60 M7, server-side, degrade-closed).

RED→GREEN: `idem_key_minted_once_at_creation` (the key is present on `DraftCreated`+`IdemKeyMinted`
and byte-identical after N field edits — assert the event sequence); `submit_sets_inflight_before_ack`
(the fold reaches `PaymentInflight` on `PaymentSubmitted` with no response folded yet);
`draft_survives_restart` (re-fold the persisted event log after a simulated app kill → same state +
same `idem_key`). **Adversarial:** an edit *after* `PaymentInflight` is refused/queued (you cannot
mutate a cart whose payment is in flight); minting a *second* key for the same draft ⇒ rejected
(the key is bound to the draft at creation — the anti-regeneration teeth).

### 4.4 M4 — reconnect outbox: query-before-replay (the double-charge-prevention core, §16.52)

`wallet/src/outbox.rs`. On the `online` transition (`Net::is_online` true), for every persisted
draft: `decide_reconnect(state)` (§3, pure) branches —

- **`Draft` ⇒ `ResumeLocalEditing`.** Restore the cart + wallet-fill into the P57 fields; nothing was
  sent; the customer continues. (§16.52 "no lost progress.")
- **`PaymentInflight` ⇒ `QueryThenDecide`.** Call **`query_status_by_key(idem_key)` FIRST** (P60 §3
  trait), **never a blind resubmit**. Then `decide_post_query(status)`:
  - `Captured` / `Authorized` / `IntentCreated` ⇒ `ShowSuccessClearDraft` — **the intent already
    lives on the hub; do NOT resubmit.** The payment succeeded server-side while the client never saw
    the ack; the query reveals it; zero second charge. This is R4 §3.3's exact prevention.
  - `NoneYet` / `Failed` ⇒ `ResubmitSameKey` — safe to (re)submit **with the same `idem_key`**, which
    P60's `create_with_key` treats idempotently (a replayed key returns the same handoff, P60 §4.2).
    Belt *and* suspenders: even the resubmit path cannot double-charge, because the key is identical.

**The falsifiable double-charge-prevention test (task-mandated):** simulate — draft created, key
minted, `PaymentSubmitted` → `PaymentInflight`, socket dropped **after** the hub captured but
**before** the client saw the ack. On reconnect, assert (a) the client calls `query_status_by_key`
**before** any `create_with_key`; (b) the query returns `Captured`; (c) `decide_post_query` yields
`ShowSuccessClearDraft`; (d) **`create_with_key` is called ZERO times** — no second charge. **RED
(the teeth):** a mutation that **regenerates** the idem key on resubmit (R4 §3.3's "never regenerate"
violated) produces two distinct keys → the mock hub records **two** intents → the test fails. **GREEN:**
key minted once at creation, reused → one intent. **Adversarial:** (i) query itself fails (hub still
unreachable) ⇒ stay in `PaymentInflight`, retry with backoff, **never** resubmit on an unknown status
(fail-closed — an unknown is treated as "maybe charged"); (ii) `Voided`/`Refunded` on query ⇒ show
that outcome, clear draft, do not resubmit; (iii) two tabs both reconnect ⇒ both query the same key,
both read the same status, the single-open guard + shared key make a double-submit unrepresentable.

### 4.5 M5 — Signal-style QR wallet transfer (X25519 → SHAKE256 → AES-256-GCM), animated-QR transport

`wallet/src/transfer.rs`. The crypto flow is Signal's device-transfer (R4 §6.1-6.2) with the server
dropped, **composed entirely from kernel primitives (§0) — zero new crypto deps**:

1. **New (receiving) device** generates `EphemeralKeypair` (X25519), emits **QR-1** = `TransferInit
   { new_device_pub, nonce, issued_ms }` — the out-of-band authenticated bootstrap (R4 §6.1).
2. **Source device** scans QR-1, generates *its own* `EphemeralKeypair`, computes
   `shared = pq::x25519(source_secret, new_device_pub)` (`kernel/src/pq/x25519.rs:16`).
3. **KDF:** `key = shake256(shared ‖ TRANSFER_KDF_CTX ‖ new_device_pub ‖ source_pub)[..32]`
   (`kernel/src/pq/keccak.rs:139`) — the **sovereign substitute for Signal's HKDF-SHA256**, same KDF
   goal (a PRF over the ECDH secret), no new `hkdf`/`sha2` dep (DECART §3). The context string
   domain-separates it from any cert-chain KDF (X8 — sibling, not merged).
4. **Fingerprint (SAS):** `fingerprint = shake256(new_device_pub ‖ source_pub)[..FINGERPRINT_LEN]`,
   shown on **both** devices — the anti-phishing binding (§4.6).
5. **After confirmation (§4.6 — mandatory)** the source seals: `ct = AES-256-GCM(key, nonce,
   serialize(WalletRecord))` (`aes-gcm` 0.10.3, `kernel/Cargo.toml:85`), wrapped as `SealedWallet {
   version, src_ephemeral_pub, nonce, ct }`.
6. **Transport = animated-QR (§4-E, CHOSEN).** `SealedWallet` is serialized, chunked to
   `≤ QR_FRAME_PAYLOAD_MAX`, encoded via **P53's `kernel/src/qr_code.rs`** (`QrEncodePort`), and
   displayed as an animated sequence (QR-2). The new device scans frames (`QrScanPort`, a platform
   camera adapter), reassembles, does the mirror ECDH (`x25519(new_secret, src_ephemeral_pub)`),
   re-derives `key`, and AEAD-opens to recover the identical `WalletRecord`.

**Transport decision — animated-QR, justified (the §4-E named engineering decision, R4 §8 item 2):**

| Transport | Zero-infra? | Every platform? | Verdict |
|---|---|---|---|
| **Animated-QR** | ✅ (optical, no server, no pairing) | ✅ any camera+screen incl. web + iOS | **CHOSEN** |
| BLE | ❌ pairing stack + OS permissions | ❌ Web Bluetooth is Chromium-only, absent on iOS Safari (same gap as web push) | rejected |
| same-LAN socket | ❌ needs shared network + mDNS + firewall traversal | ❌ web clients cannot open raw sockets | rejected |

**Size budget (R4 §8 flagged this must be checked, not assumed):** `WalletRecord` JSON ≈ name (~40 B)
+ addresses (~150 B each) + contact (~80 B) + `PaymentMethodRef` (~64–128 B) + metadata (~48 B) ≈
**≤ 1 KB** for a typical wallet. Sealed = version(1) + src_pub(32) + nonce(12) + ct(~1 KB) + tag(16)
≈ **~1.06 KB** — well under `MAX_TRANSFER_BYTES = 4096` and **inside a single QR v40 binary frame**
(`QR_FRAME_PAYLOAD_MAX = 2953`). Animated (≤ 2 frames) is pure ceiling headroom for a heavy wallet
(multiple addresses/methods); `TooLarge` refuses anything over the cap rather than degrading. **The
payload is small — animated-QR is confirmed adequate**, resolving R4 §8's flagged uncertainty.

RED→GREEN: `transfer_round_trips_identical_wallet` (two in-memory devices; B recovers `WalletRecord`
byte-identical to A's — the **wallet-transfer end-to-end test**, task-mandated); `kdf_matches_on_both
_sides` (both devices derive the same 32-byte key from mirror ECDH); `sealed_fits_one_frame` (a
typical wallet serializes to ≤ `QR_FRAME_PAYLOAD_MAX`). **Adversarial:** (i) a single flipped byte in
`ct` ⇒ `AeadInvalid` (GCM tag fails), **no partial write** — B's wallet is untouched; (ii) an expired
QR-1 (`now - issued_ms > TRANSFER_QR_TTL_S`) ⇒ `Expired`, no ECDH performed; (iii) `version` ≠
`TRANSFER_ENVELOPE_VERSION` ⇒ `VersionUnsupported`; (iv) an oversized wallet ⇒ `TooLarge` before any
seal; (v) a dropped animated frame ⇒ reassembly waits/retries, never opens a partial envelope.

### 4.6 M6 — the mandatory anti-phishing confirmation (R4 §6.4 — a real Signal 2025 lesson)

The 2025 QR-phishing incident (R4 §6.4) proved the crypto is necessary but **not sufficient**: an
attacker who tricks the source into sealing toward the *attacker's* ephemeral key steals the wallet.
Signal's fix was an **explicit user confirmation step**. P66 makes it **structural, not advisory**:

- The source device's `TransferState` passes through `AwaitingConfirmation` **before** it can seal.
  **`Confirmed`/`SealedWallet` is reachable ONLY via a `TransferCmd::ConfirmTransfer`** — there is no
  code path that produces a `SealedWallet` from `ScannedInit` directly. "Seal without confirmation"
  is an **unrepresentable state** (§5.1), the strongest form of the lesson.
- The confirmation surface shows **the direction explicitly** ("You are about to copy your saved
  details to a **NEW** device — confirm?", R4 §6.4 verbatim intent) **and the `fingerprint`** (SAS)
  the new device also displays. The user compares the two short strings; a mismatch means the QR was
  substituted (the attacker's pubkey yields a different fingerprint), and the user picks
  `RejectTransfer` ⇒ `Failed(UserRejected)`. This defeats the exact QR-substitution vector.

RED→GREEN: `seal_requires_confirm` (driving the machine from `ScannedInit` toward a seal **without**
`ConfirmTransfer` never yields a `SealedWallet` — asserted at the type/transition level);
`fingerprint_shown_both_sides` (source and new device compute the same `fingerprint` from the honest
key pair). **Adversarial (the anti-phishing teeth):** an attacker substitutes `new_device_pub` in
QR-1 with an attacker key ⇒ the source computes a `fingerprint` that **differs** from the one the
genuine new device displays ⇒ the confirmation-compare fails ⇒ `RejectTransfer` ⇒ `UserRejected`, no
seal, no leak. A test drives exactly this substitution and asserts the transfer fails closed.

### 4.7 M7 — no break-glass: self-custody is absolute (§4-B closed)

Per the closed §4-B ruling, a lost wallet key/device means the saved data is **genuinely gone** —
the customer re-enters it next order (§16.47: "loss is explicitly the user's own responsibility").
This is enforced **by construction**, not by policy:

- There is **no recovery type**: no `RecoveryKey`, no `EscrowEnvelope`, no `dowiz_break_glass_pubkey`,
  no "reset from server" method anywhere in the `wallet` crate. The transfer envelope has exactly one
  recipient — the new device's ephemeral key — and dowiz is never a recipient.
- A `no_break_glass_in_wallet` scan (mirroring §4.1's firewall) asserts the wallet sources contain
  none of `break_glass`/`breakglass`/`escrow`/`recovery_key`/`dowiz_recipient`/`backup_to_dowiz`
  (`concat!`-assembled). **RED→GREEN teeth:** adding any such symbol fails the build.

This is stated plainly as a **deliberate tradeoff**, consistent with the capability-cert /
HybridSigner self-custody framing (§16.47) — **not a gap.** The only "recovery" that exists is the
user's own choice to have transferred the wallet to a second device *before* losing the first (M5),
which is the self-custody-correct answer.

RED→GREEN: `no_recovery_symbol_present` (the scan is green on the real crate);
`adding_escrow_field_fails` (a fixture with an `escrow:` field trips the scan). **Adversarial:** a
test asserting that a `WalletRecord` sealed for device B **cannot** be opened by any key other than
B's ephemeral secret (no second recipient, no master key) — the AEAD binds the ciphertext to exactly
one derived key.

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose. **No card data can reach the wallet:** there is no card-data type
(§4.1) — the `no_card_data_in_wallet` scan makes "the wallet holds a PAN" a *tested-unreachable*
state; only a `PaymentMethodRef` (opaque provider id) exists. **No double-charge on reconnect:** the
`idem_key` is minted once and stored with the draft (§4.3); the only reconnect path from
`PaymentInflight` calls `query_status_by_key` first (§4.4), and the resubmit arm reuses the *same*
key, which P60's `create_with_key` treats idempotently — "two distinct charges from one draft" is
unrepresentable because there is exactly one key per draft and regeneration is refused (§4.3). **No
seal without confirmation:** `SealedWallet` is reachable only through `TransferCmd::ConfirmTransfer`
(§4.6) — the phishing state "sealed toward an unconfirmed peer" has no producer. **No break-glass:**
there is no recovery type (§4.7) — "dowiz recovers a lost wallet" is unrepresentable (no dowiz
recipient exists in any envelope). **No lost mid-checkout progress:** the draft + its event log are
persisted per edit (§4.2) and re-folded on restart (§4.3) — Snapshot-Re-entry (§5.4). **Money
integrity:** cart lines are `money::Money` (i64 minor units), never f64; the wallet holds no money at
all, only a method reference.

### 5.2 Schemas & scaling axes (item 8)

`WalletRecord`: axis = saved entities/wallet (addresses × methods). A person has O(few) — kilobytes.
Break point — a wallet with O(hundreds) of addresses would exceed one QR frame; `MAX_TRANSFER_BYTES`
caps it and the animated transport chunks the rest (§4.5). `CheckoutDraft` store: axis = concurrent
open drafts/device, bounded by `MAX_OPEN_INFLIGHT_DRAFTS = 1` plus a few editing drafts — no break
point in sight. Draft event log: axis = edits/checkout, tiny (a checkout is O(10) edits); the log
demotes to a cold record on `DraftCleared`. Transfer frames: axis = sealed bytes / `QR_FRAME_PAYLOAD
_MAX` → frame count; the size budget (§4.5) pins this at ≤ 2 frames for a real wallet; break point —
a very large wallet slows the animated scan, which `MAX_TRANSFER_BYTES` bounds. No axis touches a
server: every structure is per-device local.

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

**Isolation/bulkhead:** the pure `wallet` crate has **no** tauri/idb/reqwest dep — those live in the
out-of-core `wallet-adapters` crate behind the `WalletStore`/`QrScanPort` ports (the P60
`payment-adapters` firewall). A storage failure (disk full, IndexedDB eviction) reaches the logic
only as a typed `WalletStoreError`; a transfer failure only as a typed `TransferError` — never a
panic, never a propagating fault. **Mesh awareness:** the wallet is **device-local, NEVER gossiped,
NEVER on the mesh transport** (`iroh_transport`/`discovery` carry zero wallet payload). The only two
egress points are (a) the checkout submit to the hub — the *minimum* fields the customer consented to
hand it (§16.23) — and (b) the device-to-device QR transfer, which is **out-of-band optical, not a
network transport at all**. This is the strongest possible expression of §16.14 "no central state."
**Living memory:** the draft + its event log are append-only, content-addressed by `draft_id`/
`idem_key` (reuse `event_log` discipline); reconnect recall = query-before-replay over the persisted
draft (a living-memory read). The wallet record is LWW (rev-monotone) and **user-deletable** (§16.58)
— but there is deliberately **NO attic / resurrect-from-cold path for a lost-key wallet** (§4.7): the
self-custody boundary is where living-memory's "demote-never-delete" stops, on purpose.

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

**Self-Termination claimed** (hard invariant boundary, unrepresentable state — not a supervisor's
choice): the no-card-data firewall (§4.1), the seal-requires-confirmation invariant (§4.6), and the
no-break-glass absence (§4.7) each make an unsafe state *unrepresentable*, not caught. **Self-Healing
claimed narrowly** (error-correcting convergence): the reconnect query-before-replay (§4.4)
reconciles a possibly-committed payment to a single correct outcome without a second charge —
genuine idempotent convergence, not a retry gamble; the `online`-event outbox with backoff is the
transport-level self-heal. **Snapshot-Re-entry claimed** (cheap regenerative recovery from the last
valid epoch): a mid-checkout draft survives an app kill because the event log is persisted per edit
and re-folded on restart (§4.3, §16.52) — recovery is a cheap re-fold, not a bespoke path.
Mechanical rollback: the whole phase is additive (two new client-side crates, zero edits to the
kernel or to P60/P57), so deletion restores today's tree.

### 5.5 Linux discipline (item 9) + tensor/spectral/eqc (item 16)

Verdicts per the adoption framework: **ALREADY-EQUIVALENT** — one idempotency contract (P60's,
re-exported not forked), one crypto primitive family (`pq/x25519` + `pq/keccak::shake256` +
`aes-gcm`, all reused), one money authority (`money::Money`), one autofill seam (P57's
`value`/`set_value`); **REINFORCES** — the identifier-absence firewall doctrine extended from
`payment.rs`/P60 to the wallet crate (`no_card_data_in_wallet` + `no_break_glass_in_wallet`);
**EXTENDS** — a new client-side event-sourced surface (the single-writer LWW draft + the transfer
machine) modeled on the kernel `decide`/`fold` law; **GAP** honestly named — QR **decode** (camera →
bytes) needs a platform adapter (`QrScanPort`); there is no pure-kernel decoder, and P53's
`qr_code.rs` (the *encoder*) is not yet on disk, so the transport lane is gated on P53 (§4.5),
named not hidden; the animated-frame reassembly is a small new client-side primitive. Item 16:
tensor/spectral/eqc machinery is deliberately **NOT** invoked — the wallet is LWW JSON + ECDH/KDF/
AEAD + two small state machines, where a spectral form would be ritual math (Anu/Ananke discipline
forbids exactly this). The honest reuses: `event_log::sha3_256` for the idem-key derivation and
`pq/keccak::shake256` for the transfer KDF — existing hashes, not new machinery.

### 5.6 Error-propagation gates + smart index (item 14)

The bug classes this blueprint could introduce are each turned into a **compile-time or CI-time**
failure, not a runtime surprise: (a) a card field in the wallet ⇒ `no_card_data_in_wallet` CI scan
(§4.1); (b) a recovery/escrow path ⇒ `no_break_glass_in_wallet` CI scan (§4.7); (c) a regenerated
idempotency key ⇒ the `idem_key_minted_once_at_creation` regression test (§4.3) + the
double-charge-prevention test (§4.4); (d) a seal without confirmation ⇒ unrepresentable via the
`TransferState` type (§4.6); (e) a CRDT dep ⇒ caught by a `deny.toml`/dependency-fence entry barring
`automerge`/`yrs` from the wallet crate (mirrors the existing kernel dependency fences). Type system
first, CI scan second — never a runtime check.

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no wallet crate; `no_card_data_in_wallet` absent | store round-trips; autofill sets `TextField` values; adding a `pan:` field fails the build | **no-card-data-in-wallet scan** (ledger row) |
| M2 | no storage adapters | Tauri `save()` called per edit; IDB `persist()` requested; `online` event flushes outbox; quota error typed | storage-adapter explicit-save test |
| M3 | no draft machine | key minted once + byte-identical after edits; submit sets `PaymentInflight` pre-ack; survives restart re-fold | idem-key-minted-once test (ledger row) |
| M4 | naive replay double-charges | **on reconnect, `query_status_by_key` called BEFORE `create_with_key`; `Captured` ⇒ zero resubmit; regenerated-key mutation ⇒ two charges ⇒ RED** | **double-charge-prevention** test (ledger row) |
| M5 | no transfer | **two devices round-trip a byte-identical `WalletRecord`**; tampered `ct` ⇒ `AeadInvalid`, no partial write; expired QR ⇒ `Expired`; sealed fits one frame | **wallet-transfer end-to-end** test (ledger row) |
| M6 | seal reachable without confirm | seal ONLY via `ConfirmTransfer`; fingerprint shown both sides; **substituted-QR ⇒ fingerprint mismatch ⇒ `UserRejected`, no leak** | **anti-phishing-confirmation** test (ledger row) |
| M7 | recovery/escrow symbol present | `no_break_glass_in_wallet` green; adding an `escrow:` field fails the build; sealed wallet opens for exactly one key | no-break-glass scan (ledger row) |

**Not-done clauses:** any card-data type/field in the `wallet` crate = **NOT done** (§4.1 PCI
red-line); a regenerated idempotency key, or a reconnect path that resubmits on a live
`PaymentInflight` without querying first = **NOT done** (§4.4 double-charge red-line); a `SealedWallet`
producible without an explicit `ConfirmTransfer` = **NOT done** (§4.6 anti-phishing); any
recovery/escrow/dowiz-recipient path = **NOT done** (§4.7 self-custody red-line); a CRDT dependency in
the wallet crate = **NOT done** (R4 §3.1 anti-scope); redefining rather than consuming P60's
idempotency contract = **NOT done** (X6); a bare `i64`/`f64` cart amount without a `Currency` tag =
**NOT done**.

---

## 7. Benchmark plan (item 10) — pure legs micro-benched; QR I/O out-of-core

Criterion harness (the kernel bench discipline, reused), on the pure `wallet` crate: add
`wallet/transfer_seal_open_1kb` (X25519 ECDH + SHAKE256 KDF + AES-256-GCM round-trip over a 1 KB
wallet — target < 500 µs, all reused primitives), `wallet/reconnect_decide` (the pure
`decide_reconnect` + `decide_post_query` branch — target < 1 µs), `wallet/lww_apply` (an edit +
rev-bump + serialize — target < 20 µs). All added RED-commit-first so baselines auto-seed; results to
`BENCH_HISTORY.md`, never prose estimates. **Out-of-core** QR encode/scan (animated frames, camera)
and the platform storage I/O are **not** kernel-micro-benched — covered by the `wallet-adapters`
integration test with a **frame-count budget** (a typical wallet ⇒ 1 frame, §4.5) and a
scan-reassembly latency budget. Telemetry (client-side native trackers, P-H lane): a
`draft_restored` counter, a `double_charge_averted` counter (incremented whenever a reconnect query
on `PaymentInflight` returns `Captured`/`Authorized` and the resubmit is skipped — the safety
property made observable), a `transfer_success`/`transfer_failed{reason}` counter, and a
`transfer_rejected_at_confirm` counter (the anti-phishing gate firing) — so a regression in any
safety property surfaces automatically, not only at review.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the 20-point contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` **X6 (idempotency — P60 owns, P66 consumes)**, **X8
(wallet transfer shares the primitive family with the cert chain but is a SEPARATE, simpler
mechanism; reuse the self-custody framing + §6.4 anti-phishing lesson)**, §4-E (transport +
confirmation = P66's engineering decision), §4-B (self-custody severity — **CLOSED to absolute, no
break-glass**), §5 W2 P66 row · `docs/research/OPUS-R4-ORDERFLOW-COURIER-NOTIFICATIONS-2026-07-18.md`
(read in full — **§3.1 CRDT ruled out, §3.2 two-runtime storage, §3.3 the `Draft`/`PaymentInflight`
query-before-replay machine, §6.1-6.2 Signal ProvisioningCipher crypto + what to drop, §6.3 RustCrypto
building blocks, §6.4 the 2025 QR-phishing incident + confirmation fix**) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` **§16.23** (client-side data wallet, no
account), **§16.47** (device-resident, Signal-style QR, self-custody, loss is the user's
responsibility), **§16.52** (offline checkout draft, restore on reconnect), §16.14 (no central
state, honest status), §16.49 (payment client-side, hub sees only a token), §16.54 (Tauri cache-first
offline), §16.55 (no DOM — autofill fills canvas `TextField`s), §16.58 (customer self-delete), §16.43
(*ad fontes* — reuse vendored crypto primitives, don't re-implement). **Upstream dependencies
(consumed, never redefined):** **P60** (`BLUEPRINT-P60-payment-adapter-core.md` §3 `IdempotencyKey` +
trait, §4.2 `IdemLedger` + normalization gap — the idempotency contract), **P57**
(`BLUEPRINT-P57-canvas-text-input.md` §3 `TextField::value`/`set_value` — the autofill/submit seam),
**P53** (`kernel/src/qr_code.rs` — the QR encoder, X8/§4.5). **Downstream consumer:** **P69**
(customer storefront & checkout — drives the draft machine and the autofill at the card moment).
Kernel ground-truth cites all in §0. Memory: `crypto-safe-first-pass-2026-07-14` (crypto reused, not
re-implemented — the SSR-2020/CT lessons; §4.5's SHAKE-over-HKDF and AES-GCM-over-CBC choices honor
this) · `rust-native-bare-metal-decision-2026-07-14` (DECART tables §3/§4.5) · `test-integrity-rules
-2026-06-27` (money-RLS-PII red-lines; no-f64; no-card-in-store) · `anu-ananke-strict-discipline
-feedback-2026-07-17` (style; §5.5's refusal of decorative spectral) · `never-bypass-human-gates
-2026-06-29` (§4-B was human-gated — now closed, applied not re-opened) · `verified-by-math-2026-07-07`.
Supersedes: nothing (additive, greenfield).

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the `WalletRecord`/`CheckoutDraft`/`SealedWallet` types and the
  two state machines (§3) precede any adapter; the pure logic is the source, the platform storage is
  the derived shadow.
- **P2 CORRESPONDENCE** (one concept, one primitive): one idempotency contract (P60's, re-exported
  not forked, X6), one crypto family (kernel `pq` primitives, reused not re-vendored), one autofill
  seam (P57's), one money authority (`money::Money`) — the same concept never gets a second
  implementation.
- **P4 POLARITY** (paired inverses as law): seal↔open (AES-GCM), submit↔query-before-replay (the
  reconnect inverse that reconciles a possibly-committed payment, §4.4), source-device↔new-device
  pairing bound by the shared fingerprint (§4.6).
- **P6 CAUSE-AND-EFFECT** (determinism as law): the mirror ECDH + SHAKE KDF deterministically yield
  the same key on both devices; the once-minted idempotency key makes replay a no-op; the LWW `rev`
  is strictly monotone — each safety property carries a falsifier (the property/regression tests, §6).
- **P7 GENDER** (paired verification, no self-certification): the transfer requires an *independent*
  confirmation on the source device + a fingerprint match — no device self-certifies the pairing
  (§4.6); the client **never** self-certifies payment success — it delegates truth to the hub via
  `query_status_by_key` (§4.4, P60's webhook is the sole writer). Neither the payment moment nor the
  device-pairing moment trusts a single unrefereed party.

(P3/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the reused `pq/x25519`+`keccak`+`aes-gcm`, P60's contract, P57's seam, P53's QR gap) |
| 2 DoD | §6 |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; `DraftEvent`/`TransferState` event-sequence assertions |
| 4 predefined types/consts | §3 |
| 5 adversarial/breaking tests | §4.1–4.7 (pan-field-added teeth, regenerated-key double-charge, tampered-ct AEAD, expired-QR, substituted-QR phishing, escrow-field-added, quota-exceeded) |
| 6 hazard-safety as math | §5.1 (no-card, no-double-charge, no-seal-without-confirm, no-break-glass, Snapshot-Re-entry — all reachability) |
| 7 links docs/memory | §8 (P60/P57/P53 upstream, P69 downstream named) |
| 8 scaling axes | §5.2 (each with a named break point) |
| 9 Linux discipline | §5.5 (all verdict classes incl. the honest QR-decode/P53 GAP) |
| 10 benchmarks+telemetry | §7 (pure legs benched; QR I/O out-of-core; safety counters) |
| 11 isolation/bulkhead | §5.3 (pure crate + out-of-core adapters firewall) |
| 12 mesh awareness | §5.3 (device-local, never gossiped; transfer is out-of-band optical, not a transport) |
| 13 rollback/self-heal vocabulary | §5.4 (Self-Termination + Self-Healing + Snapshot-Re-entry claimed precisely) |
| 14 error-propagation gates | §5.6 (type-first, then CI scans; CRDT dep fence) |
| 15 living memory | §5.3 (append-only draft log, content-addressed; the deliberate no-attic self-custody boundary) |
| 16 tensor/spectral + eqc reuse | §5.5 (spectral honestly NOT invoked; sha3_256 + shake256 reused) |
| 17 regression ledger | §6 (five+ rows) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0/§2/§4 (P60 contract, P57 seam, P53 QR, `pq/x25519`+`keccak::shake256`+`aes-gcm`, `event_log::sha3_256`, `money::Money` all reused; DECART §3; CRDT/HKDF/CBC/BLE/break-glass rejected with reasons) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Order below is the dependency order; T1–T4 are buildable today with zero network (pure logic + the
already-in-tree `pq` crypto). T5–T6 need the platform adapters; T5's QR encode consumes P53's
`kernel/src/qr_code.rs` (if absent, stub `QrEncodePort` and leave a named `P53-qr-encoder` TODO — a
named gate, not a silent gap). Nothing waits on an operator gate — §4-A/§4-B/§4-E are closed.

1. **T1 (M1 — the wallet store + firewall).** Create crate `wallet` (repo root, `Cargo.toml`
   path-dep `dowiz-kernel = { path = "../kernel", features = ["pq"] }`). Write `record.rs` per §3
   (types verbatim) + the `WalletStore` port. Write the RED scan `no_card_data_in_wallet` FIRST (copy
   the `FORBIDDEN`+`concat!` pattern from `kernel/src/ports/payment.rs:508-560` and P60 §4.1). Then
   the store round-trip + autofill projection (calls P57 `TextField::set_value`). Acceptance: `cargo
   test -p wallet` green; a deliberately-added `pan:` field fails the scan (prove the teeth).
2. **T2 (M3 — the draft machine + idem key, X6).** Write `draft.rs` per §3. Re-export
   `IdempotencyKey`/`PaymentStatus` from `dowiz_kernel::ports::payment_provider` (do **not** redefine
   them). Mint the key with P60's derivation: `IdempotencyKey(event_log::sha3_256(&[b"dowiz.pay.idem\0",
   order_id, &wallet_id, &nonce].concat()))`. RED: `idem_key_minted_once_at_creation`,
   `submit_sets_inflight_before_ack`, `draft_survives_restart`. Acceptance: `cargo test -p wallet`
   green. **Freeze the key as minted-once — never regenerate.**
3. **T3 (M4 — reconnect query-before-replay, the double-charge core).** Write `outbox.rs` +
   `decide_reconnect`/`decide_post_query` (pure). Write the **double-charge-prevention test FIRST**
   against a **mock** `PaymentProvider` (P60 trait): draft → `PaymentInflight` → drop-after-capture →
   reconnect asserts `query_status_by_key` called **before** `create_with_key`, `Captured` ⇒ zero
   resubmit. The RED variant regenerates the key and asserts two charges (teeth). Acceptance: green;
   fail-closed on unknown status.
4. **T4 (M5+M6 — transfer crypto + mandatory confirmation).** Write `transfer.rs` per §3. ECDH via
   `dowiz_kernel::pq::x25519::x25519`; KDF via `dowiz_kernel::pq::keccak::shake256`; AEAD via
   `aes-gcm` (already a kernel dep). Model `TransferState` so `SealedWallet` is reachable **only**
   through `TransferCmd::ConfirmTransfer` (§4.6). RED: `transfer_round_trips_identical_wallet`
   (the e2e test), `seal_requires_confirm`, and the **substituted-QR ⇒ `UserRejected`** adversarial
   test; tampered-ct ⇒ `AeadInvalid`; expired ⇒ `Expired`. Acceptance: `cargo test -p wallet` green;
   `sealed_fits_one_frame` confirms the size budget.
5. **T5 (M2 + the QR/transport adapters — out-of-core).** New crate `wallet-adapters` (repo root,
   path-dep on `wallet`; `tauri-plugin-store`/`idb`/QR-decode deps live HERE, outside the firewall).
   Implement `TauriStoreAdapter` (explicit `save()` per edit), `IdbStoreAdapter` (`persist()` +
   `online`-event outbox), and the animated-QR `QrEncodePort` (consume P53's `kernel/src/qr_code.rs`)
   + `QrScanPort` (camera). RED (headless/mock): explicit-save asserted, `online` flushes, frame
   round-trip. Acceptance: `cargo test -p wallet-adapters` green; `cargo tree -p wallet` shows NO
   `tauri`/`idb` dependency (the firewall holds).
6. **T6 (M7 — no break-glass + wiring seam + ledger).** Write the `no_break_glass_in_wallet` scan
   (§4.7); add the CRDT dependency fence to `deny.toml` (bar `automerge`/`yrs` from `wallet`). Hand
   the wallet-autofill + draft-machine seam to **P69** by blueprint number (P69 drives them; it does
   not redefine them). Add the five §6 ledger rows to `docs/regressions/REGRESSION-LEDGER.md`.
   Acceptance: scans green; ledger rows present; the double-charge-prevention and wallet-transfer
   e2e tests are named in the ledger as permanent regressions.
