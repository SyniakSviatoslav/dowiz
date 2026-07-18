# BLUEPRINT P68 — Hub supervisor: update + backup: A/B-slot atomic-flip auto-update with a real-code-path health gate, owner-triggered rollback, mandatory age-snapshot-before-promote, and a sovereign encrypted backup envelope that dowiz can never decrypt (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Component:
> **SELF-HOST OPS / DURABILITY (hub-side supervisor)**. Wave **W3** of the CORE roadmap
> (`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5, the W3 table row **P68**). Scope = exactly that
> row: (a) **encrypted auto-backup** — client-side encryption BEFORE upload to `hetzner:dowiz` or
> the vendor's own S3-compatible target, dowiz never sees plaintext; (b) **auto-update by default
> with owner-triggered rollback** — A/B slots + atomic `current` symlink flip + a **real**
> health-check-before-promote gate + auto-rollback, `self_update` used ONLY for signed/checksummed
> binary fetch; (c) **age-snapshot-before-promote** (X9, R5 risk #1) — the promote step takes a
> state snapshot FIRST because forward-only schema migrations can outrun a code rollback; (d)
> **golden-image co-ownership with P67** — P68 co-signs the A/B slot layout and the backup-scheduler
> config baked into the image P67 owns. Grounds every design claim in R5 §1–§2
> (`docs/research/OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md`) + live kernel code. Structural
> template + rigor precedent: `BLUEPRINT-P66-data-wallet-offline-drafts.md`,
> `BLUEPRINT-P60-payment-adapter-core.md`, `BLUEPRINT-P51-open-map-routing.md`.
>
> **Operator rulings applied as inputs, NOT re-litigated** (CLOSED per the task + synthesis §4-B):
> self-hosted hubs get built-in encrypted auto-backup to `hetzner:dowiz` or the vendor's own
> S3-compatible target — dowiz never sees plaintext (§16.27); auto-update by default with an
> explicit owner-triggered rollback (§16.27); and the **§4-B self-custody-severity fork is CLOSED
> in favour of "self-custody is absolute — NO break-glass"**: no `dowiz_break_glass_pubkey` is ever
> in a backup's recipient set, by construction. A vendor who loses their backup key loses their
> backups — stated plainly, exactly as P66 frames a lost wallet key (§16.47 "loss is explicitly the
> user's own responsibility"), never a gap.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding, mirroring
P66's: **the entire backup-envelope crypto composes from primitives the kernel already vendors** —
X25519 ECDH, SHAKE256 (the sovereign KDF/XOF), and AES-256-GCM are all present today behind the `pq`
feature, so the age-style envelope reuses them with **zero new crypto dependencies** (the `age`/`rage`
crate is NOT pulled in — DECART §4.1). The state-snapshot anchor is the event-log's existing
content-addressed hash-chain tip; the kernel is deliberately RNG-free, so entropy enters via a port
(reinforcing the pure-crate/out-of-core split). P68 is a **hub-side** greenfield build (two new
crates) that re-derives **no crypto** and shares the golden image with P67.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| **X25519 raw ECDH primitive EXISTS**: `x25519(k:&[u8;32], u:&[u8;32]) -> [u8;32]` | `kernel/src/pq/x25519.rs:16` | **VERIFIED — the backup-envelope recipient ECDH REUSES this; NO new `x25519-dalek`/`age` dep (§4.1)** |
| **SHAKE256 XOF/PRF EXISTS**: `shake256(input,out)`, `shake256_xof`, `prf`, `xof_g/h/j` | `kernel/src/pq/keccak.rs:139,145,156,170,180` | **VERIFIED — the envelope file-key + wrap-key KDF REUSE SHAKE256 (sovereign substitute for age's HKDF); NO new `hkdf`/`sha2` dep (§4.1)** |
| **AES-256-GCM AEAD dep ALREADY in tree**: `aes-gcm = "0.10.3"` (+`curve25519-dalek="4"`) under the `pq` feature | `kernel/Cargo.toml:85-86`, feature `:50` | **VERIFIED — the envelope AEAD REUSES this dep; matches P66 §0's identical reuse. age's payload cipher is ChaCha20-Poly1305; the sovereign envelope uses AES-256-GCM (AES-NI accelerated, already vendored) — a deliberate substitution, §4.1** |
| **Event log is a content-addressed SHA3-256 hash chain**; tip/`prev` = `sha3_256(prev‖actor_pubkey‖actor_seq‖payload)`; `verify_chain` detects corruption at rest; chain-tip content-id retrievable | `kernel/src/event_log.rs:30` (`sha3_256`), `:129-150` (chain), `:201-212` (tip advance + `verify_chain`) | **VERIFIED — the pre-promote snapshot's `EpochHash` IS the chain-tip content-id; restore verifies it lands exactly at the pre-promote epoch (§4.4, Snapshot-Re-entry §5.4)** |
| **Kernel is RNG-free** — "all randomness enters via caller seed"; optional `qrng` provider behind a feature; `SHAKE256(quantum‖os)` mixing | `kernel/src/pq/entropy.rs:3-10,37-51` | **VERIFIED — the pure `supervisor` crate takes an `Rng` PORT (ephemeral-key entropy); adapters supply OsRng/qrng, tests supply a seeded RNG for KATs (§3, §4.1). NO RNG compiled into the pure crate** |
| **`hetzner:dowiz` rclone remote is LIVE with real data**: type `s3`, endpoint `fsn1.your-objectstorage.com`, bucket `dowiz` (created 2026-07-13), prefixes `backups/ cold/ db/ images/`, 141 objects / 13.07 GiB | `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md:17,60,112,115` | **VERIFIED — the backup TRANSPORT reuses this proven remote (`hetzner:dowiz/<hub-id>/`); §16.27 extends it to a per-hub encrypted-backup target. NOT Cloudflare R2 (a memory note was imprecise; this live check is authoritative)** |
| **`money::Money` i64 minor-units EXIST**, no f64 | `kernel/src/money.rs:29,33,59` (from P66 §0, re-confirmed) | VERIFIED — the supervisor touches NO money; noted only to state the boundary |
| **The no-*-data compile firewall pattern** (identifier-absence scan, `concat!`-assembled forbidden tokens, hard build failure) EXISTS and is extended per-crate | `kernel/src/ports/payment.rs:508-560`; extended by P60 §4.1 (`no_card_data_type_in_core`), P66 §4.1/§4.7 (`no_card_data_in_wallet`, `no_break_glass_in_wallet`) | **VERIFIED — P68 EXTENDS this to `no_dowiz_recipient` over the supervisor crate (§4.2), the §4-B structural guarantee** |
| **`self_update` crate is forward-only, NO rollback, NO version pinning**; fetches GitHub/GitLab/Gitea/S3; `self_replace` in-place swap; `MoveAll` transactional multi-file; `signatures` (zipsign) + SHA-256/512 verify; `rename`-based swap cannot cross filesystems; blocks on interactive TTY confirm by default | `docs/research/OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md` §2.2 (read in full) | **VERIFIED — P68 uses `self_update` for FETCH+VERIFY ONLY behind a `ReleaseSource` port, `.no_confirm(true)`; the A/B-slot + symlink-flip + health-gate machinery is the real answer (R5 §2.3-2.4). Rollback is NEVER delegated to `self_update`** |
| **`age`/`rage` crate is pre-1.0 beta** ("all crate versions prior to 1.0 are beta releases for testing purposes only"); the *format* is stable, the crate API may shift; X25519 recipients + streaming | R5 §1.2 (read in full) | **VERIFIED — the sovereign envelope AVOIDS the pre-1.0 crate-API churn AND the new dep, by reusing kernel primitives to build an age-STYLE (not age-format) envelope; DECART §4.1** |
| **No supervisor/backup/update code anywhere** — grep `supervisor\|self_update\|rclone\|break_glass\|health.*probe\|ab.*slot` over `--include=*.rs` (excl. the `agent::loop` "no supervisor" comments) → **0 product hits** | repo-wide grep this pass | **VERIFIED — P68 is greenfield: a new `hub-supervisor` crate (pure) + a new out-of-core `hub-supervisor-adapters` crate (§2)** |
| **P67 (Hub provisioning & claim) owns the golden-image spec** (X9): warm pool + Packer golden snapshot; the image bakes the hub binary in an A/B slot layout, `cloudflared`+tunnel token, pre-minted P59 roots, demo fixtures, the backup scheduler, shadow local ingress; **"the image spec is written once, in P67, with P68 as co-owner of the slot/backup layout"** | `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` X9 (L211-220), §5 W3 P67/P68 rows (L450-456), swarm dispatch (L480-481) | **VERIFIED — P68 CO-SIGNS the A/B slot layout + backup-scheduler config within P67's image; P68 does NOT redefine the image spec, the tunnel, the certs, or the fixtures (§2)** |
| R5 research verdicts (age-envelope-over-rclone not rclone-crypt; A/B-slot + symlink-flip + health-gate; `self_update` fetch-only; the migration/state-rollback hazard; the no-break-glass default) | R5 §0, §1.1-1.3, §2.1-2.4, §7 risks #1-#2 | VERIFIED read in full — P68 consumes its findings, does not re-research |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Research verdicts consumed (R5 §1 + §2, condensed — cited, not re-derived) + the closed rulings

R5 is the research substrate; its findings are inputs here. The load-bearing ones, each already
reconciled against the operator rulings:

1. **Encrypted backup = age-STYLE envelope + rclone transport, NOT rclone-crypt (R5 §0/§1.3).**
   rclone-crypt is *symmetric* (the decrypt password must live on the hub to encrypt) and carries a
   "cannot ever change the password" trap; the recommendation is a **public-key recipient** envelope
   where the hub holds only the *public* key and the private identity lives offline in the vendor's
   data-wallet — the same self-custody framing §16.47 chose for the customer wallet (P66) and §16.48
   for owner certs. rclone stays as the **transport** (already proven live, §0). **P68's refinement
   over R5 (task point 4):** where R5 named the `age`/`rage` *crate*, P68 checks — exactly as P66 did
   for wallet-transfer — whether the envelope can compose from the kernel's already-vendored
   primitives (`pq/x25519` + `pq/keccak::shake256` + `aes-gcm`) instead of pulling a fresh, pre-1.0
   crate. It can (§4.1). P68 therefore builds an **age-*style*** X25519 envelope with **zero new
   crypto deps**, and documents the one honest tradeoff (interop with the off-the-shelf `age` CLI is
   given up; the restore tool is the vendor's own open-source hub binary instead — §4.1 DECART).

2. **Auto-update rollback = A/B slots + atomic symlink flip + health gate; `self_update` fetch-only
   (R5 §2.2-2.4).** `self_update` solves *fetch + verify + atomic single-binary swap* but is
   **forward-only with no rollback and no pinning** — "the slot machinery is the real answer." The
   mature pattern (blue-green, scaled to one box): two release slots, `current` a symlink, a flip is a
   single `rename()` syscall (atomic — no request sees a half-deploy), rollback is re-pointing the
   symlink; stage into the idle slot and verify BEFORE touching `current`; **health-check before
   promote** where the probe "must exercise a real code path… 503 during warm-up, 200 only when truly
   ready" (a process that merely *responds* is NOT proof the app *works*); auto-rollback on failed
   health, keep the previous slot so owner rollback is "flip back + restart" in seconds; version
   pinning via the `ReleaseSource` trait since it isn't built in; `.no_confirm(true)` so the daemon
   never blocks on a TTY.

3. **Migration/state rollback is the sharpest edge — R5's #1-ranked risk (R5 §2.3 + §7 risk #1).**
   "Forward-only DB migrations mean rollback of *code* can outrun rollback of *schema*… The
   supervisor must snapshot the event-log/pgrust state **before** promote, so an owner rollback
   restores code *and* a compatible state, not code against a migrated-forward schema. This is the
   single most dangerous gap — it can brick a self-hosted hub's data." P68 makes the pre-promote
   snapshot a **structurally mandatory, unskippable** step (§4.4) — promote-without-snapshot is
   unrepresentable (§5.1), the strongest form of the lesson, exactly as P66 made seal-without-confirm
   unrepresentable.

4. **The break-glass question is R5's #2-ranked risk, now CLOSED (R5 §1.3, §7 risk #2 → §4-B).** "If
   backups use a pure vendor-held X25519 identity, a vendor who loses the key loses every backup…
   an explicit ruling on whether a break-glass recipient exists" is needed; "that key *is* the 'dowiz
   can see plaintext' backdoor the invariant forbids." **The operator has closed §4-B to absolute
   self-custody (task binding): NO `dowiz_break_glass_pubkey`, ever.** P68 enforces this **by
   construction** (§4.2) — there is no code path that adds a dowiz-controlled key to a recipient set,
   and a falsifiable scan proves it (the task-mandated no-dowiz-recipient-key test). "dowiz never sees
   plaintext" is a **structural** guarantee here, not policy prose.

**Closed rulings, applied as fixed inputs (not re-opened):**
- **§16.27** — built-in encrypted auto-backup (dowiz never plaintext) + auto-update-by-default with
  owner-triggered rollback. Verbatim scope from the master roadmap §16.27 (read this pass, L2084-2094).
- **§4-B CLOSED → self-custody is absolute (NO break-glass).** Applied structurally (§4.2), stated as
  a deliberate tradeoff (§16.47), never a gap. Same ruling P66 §4.7 already applied to the wallet —
  P68 applies the *same* ruling to backups, so the wallet, cert, and backup designs stay consistent
  (synthesis §4-B: "asking for one ruling applied to both").
- **X9 CLOSED → the golden image is P67's; P68 co-signs the slot + backup layout only.** P68 does not
  redefine the hub binary, tunnel, certs, fixtures, or ingress (§2).
- **The transport is CHOSEN (engineering, not an operator gate):** rclone to `hetzner:dowiz/<hub-id>/`
  (default) or the vendor's configured S3 remote (§4.3) — the already-live remote (§0), reused not
  rebuilt.

---

## 2. Scope — what P68 owns vs deliberately does NOT

**P68 owns (build items §4):** two new hub-side crates — `hub-supervisor` (pure logic + ports;
path-dep on `dowiz-kernel` for the reused crypto/hash primitives) and `hub-supervisor-adapters`
(out-of-core, holds the platform deps `self_update`/rclone-shell-out/systemd-or-process-control/
filesystem/OsRng behind the ports — exactly the P66 `wallet`/`wallet-adapters` and P60
`payment`/`payment-adapters` split).

| Item | Content |
|---|---|
| M1 | **Sovereign encrypted backup envelope** (`hub-supervisor/src/backup.rs`): age-STYLE X25519→SHAKE256→AES-256-GCM **STREAM** envelope (truncation/reorder-resistant chunking), multi-*vendor*-recipient file-key wrap, seal + open; **zero new crypto deps** — the DECART reuse-vs-`age`-crate decision |
| M2 | **§4-B no-dowiz-recipient structural guarantee** (`hub-supervisor/src/backup.rs` + a CI scan): the recipient set is sourced ONLY from vendor config; the `no_dowiz_recipient` identifier scan + the recipient-count invariant; **the falsifiable no-dowiz-recipient-key test** (§6) |
| M3 | **Backup scheduler + transport** (`hub-supervisor-adapters`): systemd-timer / in-kernel-tick cadence; `rclone copy` of sealed blobs to `hetzner:dowiz/<hub-id>/` (default) or vendor-S3; 3-2-1 fan-out; retention; the restore path (`rclone copy` back + envelope-open with the vendor identity) |
| M4 | **State snapshot primitive** (`hub-supervisor/src/snapshot.rs`): the event-log chain-tip `EpochHash` anchor + the PgStore/W13 projection; a **local plaintext** snapshot for instant rollback (never uploaded — the vendor's own data on the vendor's own box) reusing the same tar mechanism the M1 envelope wraps for off-site |
| M5 | **A/B slot update supervisor** (`hub-supervisor/src/update.rs`): two slots + `current` symlink + the atomic-flip machine; `self_update` fetch+verify-ONLY behind `ReleaseSource`; owner version pinning (`self_update` has none) |
| M6 | **Real health-check-before-promote gate** (`hub-supervisor/src/update.rs` + a probe adapter): the probe exercises a **real code path** (event-log head verify + a synthetic order read + a kernel order-machine transition), 503 while warming / 200 only when truly ready; promote is UNREPRESENTABLE without a pass |
| M7 | **Age-snapshot-before-promote + auto/owner rollback** (`hub-supervisor/src/update.rs`): the **mandatory** pre-promote snapshot (M4); crash-loop auto-rollback restoring **code AND state**; the one-command owner rollback; **the falsifiable rollback-after-schema-migration test** (§6) |

**P68 explicitly does NOT own:**

- **NOT the golden-image spec — P67 owns it (X9).** P67 writes the image spec once: the hub binary
  baked in, `cloudflared` + the pre-created remotely-managed tunnel token, the pre-minted P59
  self-signed root cert, the demo fixtures (§16.54), and the shadow local ingress config (R3 risk #5).
  P68 **co-signs exactly two slices of that image**: (i) the **A/B slot layout** — `releases/{A,B}/`,
  the `current` symlink, the baked-in initial slot = the image's hub version, the `dowiz-hub-supervisor`
  unit, the unpinned default (auto-update on); and (ii) the **backup-scheduler config** — the timer
  cadence, the default `hetzner:dowiz/<hub-id>/` remote + the vendor-S3-override stub, and the
  recipient-config *location* left **EMPTY** in the image (the vendor's backup pubkey is provisioned
  at *claim* time by P67's claim service, NEVER baked — which makes §4-B trivially true of the image:
  no key is baked, so no dowiz key is baked). A diff that redefines P67's tunnel/cert/fixture/ingress
  spec from inside P68 is a **scope violation** (X9: "written once, in P67"). **Co-owner of two slices,
  not a second author of the whole image.**
- **NOT the claim/provisioning flow, warm-pool economics, or `TunnelProvider`/`VpsProvider`** — all
  P67. P68 activates on an *already-provisioned* hub. Pre-claim (dowiz-operated warm pool), backups
  are **disabled or scoped to demo fixtures only** — there is no vendor data and no vendor recipient
  key yet, so the §4-B invariant is vacuously honored; backups *activate at claim* when the vendor
  supplies their recipient pubkey. **Consumer of P67's provisioned image.**
- **NOT the payment adapter, order machine, RLS, or any money leg** — P60/P62/kernel own those. The
  health probe *reads* an order through the existing kernel order_machine and *reads* a menu leaf; it
  never writes money, never mutates a real order (it drives a synthetic probe order in an isolated
  probe context). The supervisor touches **no money**. **Consumer (read-only probe).**
- **NOT the capability-cert chain (P59) crypto.** X8/§0 discipline (inherited from P66): the backup
  envelope **shares the primitive *family*** (X25519/KDF/AEAD from the kernel `pq` module) but is a
  **separate, simpler mechanism — do not merge backup crypto into the cert chain.** P68 reuses the
  *primitives* and the *self-custody framing*; it does **not** touch `HybridSigner`, biscuit blocks,
  or the cert chain. The KDF context strings domain-separate the backup envelope from both the wallet
  transfer (P66) and any cert KDF. **Sibling, not a merge.**
- **NOT a new backup CIPHER-as-dependency.** The `age`/`rage` crate is **rejected** (DECART §4.1):
  the envelope reuses kernel primitives, zero new crypto deps — consistent with P66's identical
  finding and *ad fontes*'s "primitives over dependencies" spirit. rclone-crypt is likewise not the
  primary cipher (R5 §1.3); rclone is transport only.
- **NOT `self_update`'s rollback (it has none) or its pinning (it has none).** `self_update` is used
  **only** for signed/checksummed fetch behind the `ReleaseSource` port. A diff that routes rollback
  or pinning *through* `self_update` is a **scope violation** (R5 §2.2: forward-only).
- **NOT config-only hot-reload (arc-swap).** R5 §2.3 step 6 names a `arc-swap` validate-before-swap
  config hot-reload as a *secondary* mechanism. It is **deliberately out of P68's Wave-0 core** — it
  is config-only, not part of the update/rollback story, and folding it in would blur the rollback
  core. Named here so nobody double-owns it; a small follow-up, not built here.
- **NOT incremental/dedup backup, cross-region replication, or backup encryption-at-rest key
  *rotation* beyond "next backup uses the new recipient set."** Wave-0 is **full snapshots** to one
  or more rclone remotes. Incremental backup is a named future scaling axis (§5.2), not a Wave-0 gap.

**Reconciliation with §16.29 (honest, not silently widened):** §16.29 puts media storage and
dispute/refund handling on the vendor + payment provider. P68's backup covers the hub's **state**
(event-log + PgStore projection), not vendor media blobs (those live in R2/vendor storage per §16.29
and are backed up by that provider). Named here so the backup scope is not read as "all bytes on the
box."

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── hub-supervisor/src/backup.rs — sovereign encrypted backup envelope ──────────
//  ALL crypto REUSED from kernel `pq` — zero new crypto deps (DECART §4.1).
//  X25519 ECDH → SHAKE256 KDF → AES-256-GCM STREAM (age-STYLE, NOT the age crate).
//  Depends on `dowiz-kernel` (features=["pq"]) ONLY: pq::x25519, pq::keccak::shake256, aes-gcm.
//  NO self_update, NO rclone, NO OsRng here (those live out-of-core in -adapters).
//  Red-proof: `no_dowiz_recipient` (§4.2).

pub const BACKUP_ENVELOPE_VERSION: u8   = 1;
pub const BACKUP_CHUNK_BYTES:      usize = 64 * 1024;   // age's STREAM chunk size (constant memory)
pub const AEAD_KEY_LEN:            usize = 32;          // AES-256-GCM
pub const AEAD_TAG_LEN:            usize = 16;
pub const STREAM_NONCE_LEN:        usize = 12;          // 11-byte BE chunk counter ‖ 1-byte last-flag
pub const STREAM_MORE:  u8 = 0x00;                      // interior chunk
pub const STREAM_FINAL: u8 = 0x01;                      // LAST chunk (truncation/extension resistance)
pub const BACKUP_KDF_CTX:  &[u8] = b"dowiz.backup.envelope.v1";        // payload file-key domain sep
pub const BACKUP_WRAP_CTX: &[u8] = b"dowiz.backup.recipient-wrap.v1";  // per-recipient wrap domain sep

/// A VENDOR-controlled X25519 backup RECIPIENT public key. Constructed ONLY from a public key
/// read out of the vendor's own backup config (§4.2). There is deliberately NO constructor from a
/// compile-baked constant and NO `from_dowiz_*` producer — that is the §4-B firewall in the type.
pub struct RecipientPubKey([u8; 32]);
impl RecipientPubKey { pub fn from_vendor_config(pk: [u8; 32]) -> Self { Self(pk) } }  // the ONLY ctor

/// The recipient set for a backup. Built ONLY via `from_vendor_config`; the sealing API takes NO
/// other key source, so "append a dowiz key" has no producer (§4.2). 1..N, all vendor-controlled.
pub struct RecipientSet(Vec<RecipientPubKey>);

/// Per-recipient wrap stanza (age-style header): the random file-key AEAD-wrapped to ONE recipient.
pub struct RecipientStanza {
    pub ephemeral_pub: [u8; 32],                         // per-recipient ephemeral X25519 public
    pub nonce: [u8; STREAM_NONCE_LEN],                   // wrap nonce (chunk counter 0, STREAM_FINAL)
    pub wrapped_file_key: [u8; AEAD_KEY_LEN + AEAD_TAG_LEN],  // AES-256-GCM(wrap_key, file_key)
}

/// The sealed backup header. `recipients.len()` MUST equal the vendor-config recipient count (§4.2).
pub struct BackupHeader { pub version: u8, pub recipients: Vec<RecipientStanza> }

pub enum BackupError {
    NoRecipients, DowizRecipientRejected, AeadInvalid, Truncated, ChunkReorder,
    VersionUnsupported, StateReadFailed, Io(String),
}

/// Entropy + transport PORTS (out-of-core; the pure crate is RNG-free like the kernel, §0).
pub trait Rng      { fn fill(&mut self, buf: &mut [u8]); }                    // OsRng/qrng in adapters; seeded in tests
pub trait BlobSink   { fn put(&mut self, key: &str, bytes: &[u8]) -> Result<(), BackupError>; }  // rclone copy up
pub trait BlobSource { fn get(&mut self,  key: &str) -> Result<Vec<u8>, BackupError>; }           // rclone copy back

// ── hub-supervisor/src/snapshot.rs — the STATE snapshot (rollback story for state) ─

/// The event-log chain-tip content-id at snapshot time (kernel `event_log::sha3_256` chain, §0).
/// The content-addressed anchor: a restore is verified to land EXACTLY at this epoch (§4.4).
pub struct EpochHash(pub [u8; 32]);

/// A pre-promote LOCAL state snapshot. PLAINTEXT, on-disk, NEVER uploaded — the vendor's own data
/// on the vendor's own box is NOT a "backup to dowiz", so it is NOT under the §4-B encrypt rule.
/// The OFF-SITE backup (M1) is the encrypted-to-vendor-key artifact; this local one is the instant
/// auto-rollback source (a supervisor with no vendor private key can still restore it, §4.4).
/// Retained exactly ONE generation (the currently-running version's predecessor epoch).
pub struct StateSnapshot {
    pub epoch: EpochHash,          // restore verifies the chain tip returns to this id
    pub from_version: Version,     // the code version this state is compatible with
    pub taken_at_ms: u64,
    pub event_log_path: String,    // event-log segment up to `epoch`
    pub projection_path: String,   // PgStore/W13 projection files (re-foldable from the log if lost)
}

// ── hub-supervisor/src/update.rs — A/B slot update supervisor ───────────────────

pub const SLOT_COUNT:              usize = 2;    // A/B, fixed
pub const HEALTH_PROBE_TIMEOUT_S:  u32   = 30;   // 503→200 window; timeout ⇒ never flip
pub const CRASH_LOOP_WINDOW_S:     u32   = 120;  // post-flip watch window
pub const CRASH_LOOP_MAX_RESTARTS: u32   = 3;    // > this many restarts in the window ⇒ auto-rollback

pub struct Version(pub String);                  // semver; compared via ReleaseSource
pub enum   Slot { A, B }

/// The update state machine. `Promoted` is reachable ONLY through `SnapshotTaken` THEN
/// `HealthPassed`: promote-without-snapshot and promote-without-health are UNREPRESENTABLE (§5.1) —
/// the type has no producer for either. This is R5 risk #1's kill, expressed structurally.
pub enum UpdateState {
    Idle          { current: Slot, pinned: Option<Version> },
    Fetched       { into: Slot, version: Version },                         // self_update verified into idle slot
    Migrated      { into: Slot, version: Version },                         // forward migrations ran under new code
    SnapshotTaken { into: Slot, version: Version, snapshot: EpochHash },    // MANDATORY pre-promote (§4.4)
    HealthPassed  { into: Slot, version: Version, snapshot: EpochHash },    // real-code-path probe returned Ready
    Promoted      { current: Slot, previous: Slot, rollback_to: EpochHash },// symlink flipped (atomic rename)
    RolledBack    { current: Slot, trigger: RollbackTrigger, restored: EpochHash },
    Failed(UpdateError),
}
pub enum RollbackTrigger { CrashLoop, HealthTimeout, OwnerRequested }
pub enum UpdateError {
    FetchFailed, VerifyFailed, MigrationFailed, HealthFailed, SnapshotFailed,
    PinnedVersion, NoPreviousSlot, Io(String),
}

/// A REAL health result. 503 WarmingUp until a real code path served correctly; 200 Ready ONLY then.
pub enum HealthResult { WarmingUp, Ready, Failed(String) }

/// PORTS (out-of-core). `ReleaseSource` wraps `self_update` for FETCH+VERIFY ONLY + the pin check.
pub trait ReleaseSource {
    fn latest(&self) -> Result<Version, UpdateError>;
    fn fetch_verified(&self, v: &Version, into: Slot) -> Result<(), UpdateError>;  // zipsign + SHA-256/512
}
pub trait HealthProbe { fn probe(&self, slot: Slot) -> HealthResult; }             // exercises a real path (§4.3-M6)
pub trait SlotFs {                                                                 // real filesystem
    fn flip_current(&mut self, to: Slot) -> Result<(), UpdateError>;               // ONE rename() syscall (atomic)
    fn current(&self) -> Slot;
    fn previous(&self) -> Option<Slot>;
}
pub trait ServiceCtl {
    fn restart(&mut self) -> Result<(), UpdateError>;
    fn restart_count_since(&self, since_ms: u64) -> u32;                           // crash-loop detector input
}
pub trait StateStore {                                                            // event-log + projection
    fn snapshot(&self, from: &Version) -> Result<StateSnapshot, UpdateError>;      // pre-promote (M4)
    fn restore(&mut self, snap: &StateSnapshot) -> Result<EpochHash, UpdateError>; // returns the landed tip
    fn chain_tip(&self) -> EpochHash;                                             // event_log tip (§0)
}

// ── pure decide functions — no I/O, fully testable (item 3) ──────────────────────
pub enum PromoteStep { TakeSnapshot, RunHealthProbe, FlipSymlink, Restart, Refuse(UpdateError) }
/// Enforces the order snapshot → health → flip; refuses any out-of-order or pinned promote.
pub fn decide_promote(st: &UpdateState, pinned: &Option<Version>, target: &Version) -> PromoteStep;
pub enum RollbackStep { FlipToPrevious, RestoreSnapshot, Restart, Refuse(UpdateError) }
/// Code AND state both roll back: flip the slot AND restore the pre-promote snapshot (R5 risk #1).
pub fn decide_rollback(trigger: RollbackTrigger, previous: Option<Slot>, snap: &StateSnapshot) -> RollbackStep;
```

Rejected alternatives (DECART one-liners): **the `age`/`rage` crate** — rejected: the kernel already
vendors X25519 + SHAKE256 + AES-GCM, so an age-*style* envelope composes with **zero new deps**, and
avoids `age`'s pre-1.0 crate-API churn (R5 §1.2); the only cost is off-the-shelf-`age`-CLI interop,
recovered via the vendor's own open-source restore binary (§4.1). **rclone-crypt as the primary
cipher** — rejected: symmetric (decrypt password must live on the hub), "cannot change the password"
trap, no PQ path (R5 §1.3); rclone stays transport-only. **`self_update` for rollback/pinning** —
rejected: it is forward-only with neither (R5 §2.2); A/B slots + symlink flip are the rollback,
`ReleaseSource` is the pin. **ChaCha20-Poly1305** (age's payload cipher) — rejected for AES-256-GCM:
already a kernel dep, AES-NI accelerated, matches P66 §0's identical substitution. **HKDF-SHA256**
(age's KDF) — rejected: `pq/keccak::shake256` is the sovereign XOF, same PRF goal, zero new deps.
**A non-STREAM single-AEAD-over-the-whole-archive** — rejected: unbounded memory + no
truncation resistance; the STREAM chunking (counter nonce + final-flag) is age's own construction and
is required for truncation/reorder resistance (§4.1). **A `dowiz_break_glass_pubkey` recipient /
escrow** — rejected: §4-B closed to absolute self-custody (§4.2). **A shared local *encrypted*
pre-promote snapshot** — rejected: the supervisor must auto-rollback WITHOUT the vendor's offline
private key, so the local rollback snapshot is plaintext-on-the-vendor's-own-box (§4.4); only the
off-site copy is enveloped. **config hot-reload in P68 core** — rejected as out-of-scope secondary
(§2). **Incremental/dedup backup** — rejected for Wave-0: full snapshots, incremental is a named
future axis (§5.2).

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 4.1 M1 — sovereign encrypted backup envelope (X25519 → SHAKE256 → AES-256-GCM STREAM)

New crate `hub-supervisor` (repo root), module `backup.rs` per §3. The envelope is age's *design*
(public-key recipients, a random file key wrapped per recipient, a STREAM-chunked AEAD payload)
composed **entirely from kernel primitives — zero new crypto deps** (the task-point-4 finding, the
same reuse P66 §0 proved for wallet transfer):

**Seal (`seal(state_tar, recipients, rng) -> (BackupHeader, impl Iterator<Chunk>)`):**
1. Draw a random `file_key: [u8;32]` from the `Rng` port (adapters supply OsRng/qrng; the pure crate
   is RNG-free, §0).
2. **Per recipient `R_i`** (each a vendor-controlled `RecipientPubKey`): draw an ephemeral X25519
   keypair `(e_i_sec, e_i_pub)`; `shared_i = pq::x25519(e_i_sec, R_i)` (`kernel/src/pq/x25519.rs:16`);
   `wrap_key_i = shake256(shared_i ‖ BACKUP_WRAP_CTX ‖ e_i_pub ‖ R_i)[..32]`
   (`kernel/src/pq/keccak.rs:139`); `wrapped_i = AES-256-GCM(wrap_key_i, nonce_i, file_key)`. Emit a
   `RecipientStanza { e_i_pub, nonce_i, wrapped_i }`. **This is how multiple *vendor* keys are
   supported without re-encrypting the archive** — and it is exactly why §4-B is enforced at the
   *recipient-set construction* boundary (§4.2), not per-crypto-call.
3. **Payload STREAM:** the state tar is read in `BACKUP_CHUNK_BYTES` (64 KiB) chunks; chunk `j` is
   sealed `AES-256-GCM(file_key, stream_nonce(j, is_last), chunk_j)` where
   `stream_nonce(j, is_last) = j_as_11B_BE ‖ (STREAM_FINAL if is_last else STREAM_MORE)` — the
   Bellare-Namprempre-Rogaway / age STREAM construction: a per-chunk counter nonce (no reuse) plus a
   final-chunk domain-separator byte. **This is the load-bearing correctness detail** — it is what
   makes truncation (drop the tail) and reordering (swap chunks) *fail to open* rather than silently
   producing a shorter/scrambled archive.

**Open (`open(header, chunks, vendor_identity, out)`):** find the stanza whose ECDH the vendor's
identity satisfies — `shared = pq::x25519(vendor_sec, stanza.e_pub)`, re-derive `wrap_key`, AEAD-open
`wrapped_file_key` → `file_key`; then stream-decrypt each chunk, requiring the `STREAM_FINAL` flag to
appear **exactly once, at the true end** (a stream that ends without a `STREAM_FINAL` chunk is
`Truncated`; a `STREAM_FINAL` before the last received chunk is `ChunkReorder`).

**DECART — reuse vs the `age` crate (task point 4, stated honestly):**

| Option | New deps | Interop | Verdict |
|---|---|---|---|
| **Kernel-primitive age-*style* envelope** | **0** (reuse `pq/x25519`+`keccak::shake256`+`aes-gcm`) | restore = the vendor's own open-source hub binary (`dowiz-hub restore`) | **CHOSEN** — *ad fontes* "primitives over dependencies", matches P66; sidesteps age's pre-1.0 crate-API churn (R5 §1.2) |
| `age`/`rage` crate | +`age` (+ its tree) | off-the-shelf `age -d` CLI can decrypt | rejected — a fresh pre-1.0 dep for an interop nice-to-have the open-source restore tool already covers |

The **one honest tradeoff**, stated plainly (not hidden): choosing the sovereign envelope gives up
`age`-CLI interop — a vendor cannot decrypt with a stock `age` binary. But the restore tool is the
**vendor's own open-source (AGPLv3) hub binary** they already run (`dowiz-hub restore <blob>` +
their identity), so disaster recovery does not depend on a third-party tool — consistent with P66's
"a lost key means the data is genuinely gone" honesty and §16.27's "a venue whose hardware fails can
still recover" (recover *with the same open-source software*). **This is the primary point-4 finding:
genuine reuse is possible, so it is preferred.**

RED→GREEN: `seal_open_round_trips_identical` (a 1 MB fixture tar seals then opens byte-identical for a
single vendor recipient); `multi_recipient_each_opens` (2 vendor keys → each independently opens the
same archive); `stream_nonce_is_unique_per_chunk` (KAT over the counter construction). **Adversarial
(the STREAM teeth — the class `crypto-safe-first-pass` caught a real bug in):** (i) flip one byte in a
payload chunk ⇒ `AeadInvalid`, **no partial write** to `out`; (ii) **truncation** — drop the final
chunk ⇒ `Truncated` (the last received chunk lacks `STREAM_FINAL`), never a silently-short restore;
(iii) **reorder** — swap two interior chunks ⇒ `ChunkReorder`/`AeadInvalid` (counter-nonce mismatch);
(iv) **extension** — append a forged chunk after the `STREAM_FINAL` ⇒ rejected (nothing decrypts past
the final flag); (v) open with a non-recipient key ⇒ AEAD-open of the wrap fails, no `file_key`
recovered. **Mandatory gate:** because this is a hand-rolled chunked-AEAD, an **independent adversarial
crypto review** of the STREAM construction (nonce uniqueness, truncation/reorder/extension resistance,
domain separation from the wallet/cert KDFs) is a **DoD blocker** (§6), mirroring P59's mandatory
independent-review gate and the `crypto-safe-first-pass-2026-07-14` lesson (a real forgery was found
in a hand-rolled verify — do not ship hand-rolled crypto unrefereed).

### 4.2 M2 — §4-B no-dowiz-recipient structural guarantee (the falsifiable no-dowiz-key test)

The task requires the "dowiz never sees plaintext" invariant be **structural and testable, not policy
prose** — "a scan proving no dowiz-controlled key exists in any backup's recipient list." P68 enforces
it at **two layers**, both falsifiable:

1. **Data-flow firewall (the strong form).** `RecipientPubKey` has exactly one constructor —
   `from_vendor_config([u8;32])` — and `RecipientSet` is built **only** from those. The `seal` API
   takes a `RecipientSet` and **no other key parameter**; there is *no code path* that appends a
   compile-baked constant, an environment key, or a `dowiz_*` key. So "a dowiz key in the recipient
   set" has **no producer** (§5.1 reachability). The recipient set's source is a single vendor config
   file, provisioned at claim (P67), never the image (§2).
2. **Identifier-absence scan (the belt-and-suspenders form).** `no_dowiz_recipient` — extending P66
   §4.7's proven `no_break_glass_in_wallet` pattern (`payment.rs:508-560` lineage): the crate
   `include_str!`s its own sources and asserts none of `break_glass`/`breakglass`/`escrow`/
   `recovery_key`/`dowiz_recipient`/`dowiz_pubkey`/`platform_key`/`master_key`/`backup_to_dowiz`
   appear (forbidden tokens `concat!`-assembled so the scan body never self-matches).

**The falsifiable no-dowiz-recipient-key test (task-mandated):** seal a backup with a vendor
recipient set; assert (a) `header.recipients.len() == vendor_config.recipients.len()` (no extra
stanza was injected); (b) for a test-fixture set of **known dowiz-controlled pubkeys**, none appears
among the recipient stanzas' derivable identities; (c) there is no API surface that accepts a second
key source. **RED (the teeth):** a mutation that adds a dowiz pubkey to the recipient set — via a new
`from_dowiz(...)` ctor or a hardcoded stanza — trips (a) the recipient-count mismatch and (b) the
`no_dowiz_recipient` scan, failing the build. **GREEN:** recipient set == vendor config, scan clean.
**Adversarial:** a backup sealed for the vendor is fed to a `try_open(dowiz_identity)` — it returns
`AeadInvalid` (dowiz holds no matching private key), proving dowiz-side decryption is impossible **by
construction, not by policy** (the §16.27 invariant satisfied structurally).

**Honest loss framing (P66 parity):** a vendor who loses their backup private key loses their
backups — there is no recovery, no escrow, no dowiz copy of the key. Stated plainly as the deliberate
§16.47 tradeoff, never a gap. The *only* mitigation is the vendor holding **multiple of their own**
recipient keys (the multi-recipient support in M1 exists precisely for this — a primary key + an
offline paper-backup key, both vendor-controlled), which is the self-custody-correct answer.

### 4.3 M3 — backup scheduler + rclone transport + the restore path

New crate `hub-supervisor-adapters` (repo root, path-dep on `hub-supervisor`; the platform deps —
`self_update`, the rclone shell-out, systemd/process control, filesystem, OsRng — live **HERE**,
outside the pure crate, behind the §3 ports; the P66/P60 firewall split). The backup lane:

- **Cadence** — a systemd timer (installed hub) or an in-kernel tick (`tokio::time::interval`) drives
  a scheduled seal + upload. The cadence is a value baked into P67's image backup-scheduler config
  that P68 co-signs (§2) — default proposal: a snapshot on a fixed interval plus one at every
  pre-promote (M4/M7), 3-2-1-1-0 discipline (memory `ops-reliability-arc`).
- **Seal + upload** — the scheduler tars the hub state (event-log + PgStore/W13 projection, reusing
  the M4 snapshot mechanism), streams it through the M1 `seal` to the vendor recipient set, and
  `rclone copy`s the sealed blob to `hetzner:dowiz/<hub-id>/backups/snapshot-<epoch>-<ts>.dwz`
  (default) **or** the vendor's configured S3 remote — the already-live `hetzner:dowiz` remote (§0),
  reused not rebuilt. rclone handles S3 auth, retry, and multi-remote 3-2-1 fan-out (R5 §1.3).
- **Restore** — `rclone copy` the blob back + M1 `open` with the vendor's identity (supplied from
  their data-wallet, offline). The dowiz-side operator **cannot** decrypt (holds no private key), the
  §16.27 invariant satisfied by construction (§4.2).

RED→GREEN (adapter crate, headless with a mock `BlobSink`/`BlobSource`): `scheduler_seals_and_puts`
(a tick produces a sealed blob and calls `BlobSink::put` with the `<hub-id>/backups/` key shape);
`restore_round_trips_via_mock_remote` (put → get → open → byte-identical state);
`vendor_s3_override_selected` (a configured vendor remote is used instead of `hetzner:dowiz`).
**Adversarial:** an rclone upload failure ⇒ typed `Io`, the blob is retained locally + retried (never
silently dropped); a partial upload (network cut mid-copy) ⇒ the blob is not marked done, re-uploaded
next tick (rclone's own atomicity + a done-marker); a corrupt blob at rest ⇒ `open` returns
`AeadInvalid`/`Truncated`, surfaced, never a silent bad-restore.

### 4.4 M4 — state snapshot primitive (the rollback story for STATE, R5 risk #1)

`hub-supervisor/src/snapshot.rs` per §3. A `StateSnapshot` captures the hub state at a
content-addressed epoch: the **event-log chain-tip `EpochHash`** (`event_log::sha3_256` chain tip,
`kernel/src/event_log.rs:201-212`) plus the PgStore/W13 projection files. Two consumers, **one
mechanism** (CORRESPONDENCE — one snapshot concept, one primitive):

- **Off-site backup (M3):** the snapshot tar → M1 `seal` (encrypted to the vendor) → rclone upload.
- **Pre-promote local rollback (M7):** the snapshot tar written **local, plaintext, on the vendor's
  own box** — deliberately **NOT** encrypted and **NOT** uploaded. Reasoning (the important design
  distinction the task asks for): the auto-rollback path must restore **without** the vendor's offline
  private key (a crash-loop at 3am cannot wait for the vendor to fetch their identity), so the local
  rollback snapshot must be plaintext-and-supervisor-readable. This does **not** violate §4-B: "dowiz
  never sees plaintext" is about the **off-site backup** (bytes that leave the box for storage dowiz
  might control); the vendor's own hub reading its own local disk is not a backup-to-dowiz. On a
  self-hosted box it is the vendor's hardware; on a claimed hub it is the vendor's instance. The
  distinction is stated so no reviewer mistakes the plaintext local snapshot for a §4-B breach.

**Why the event-log tip is the anchor:** the log is event-sourced (`decide`/`fold`), so state is a
pure fold of the event log; the tip content-id *is* the epoch. Restore verifies the chain tip returns
**exactly** to the pre-promote `EpochHash` — a content-addressed, verifiable landing (not "probably
the right state"). If a forward migration only touched the *projection* (re-foldable), restoring the
event-log segment + re-folding under old code suffices; if it irreversibly transformed *stored
events*, the projection files in the snapshot restore that too. Retention: exactly one generation (the
predecessor epoch of the currently-running version) — O(1) disk.

RED→GREEN: `snapshot_captures_chain_tip` (the `EpochHash` equals `event_log` tip at capture);
`restore_lands_at_epoch` (after mutating state then restoring, the chain tip == the snapshot's
`EpochHash`, byte-verified); `local_snapshot_is_not_uploaded` (the plaintext local path is never
handed to `BlobSink`). **Adversarial:** a snapshot taken then the log advanced then restored ⇒ the
advance is gone, tip back at the epoch (no partial restore); a corrupt local snapshot ⇒
`verify_chain` (kernel, §0) flags it, restore refuses rather than loading corruption.

### 4.5 M5 — A/B slot update supervisor (atomic symlink flip, `self_update` fetch-only, pinning)

`hub-supervisor/src/update.rs` per §3. The layout (co-signed into P67's image, §2):
`/opt/dowiz/releases/{A,B}/` two slots, `/opt/dowiz/current` a symlink to the live one, the baked
initial slot = the image's hub version, unpinned by default (auto-update on). The update flow (the
`UpdateState` machine):

1. **Fetch + verify** — `ReleaseSource::fetch_verified` wraps `self_update` for **fetch + verify
   ONLY** (`.no_confirm(true)`, signed zipsign + SHA-256/512 — R5 §2.2), staging into the **idle**
   slot. Verify failure ⇒ `VerifyFailed`, `current` untouched. `Idle → Fetched`.
2. **Pin check** — if `pinned` is set and `latest > pinned`, `decide_promote` returns
   `Refuse(PinnedVersion)` (built via `ReleaseSource` since `self_update` has no pinning — R5 §2.2).
3. **Forward migrations** run under the new binary in the idle slot. `Fetched → Migrated`.
4. → **M6 health gate**, then **M7 snapshot + flip**.

The **flip is a single `rename()` syscall** over a temp symlink (`SlotFs::flip_current`) — genuinely
atomic, no request ever sees a half-deployed `current` (R5 §2.3). `self_update`'s cross-filesystem
`rename` limitation is avoided: slots share one filesystem by the image layout (§2).

RED→GREEN: `fetch_stages_into_idle_slot` (never the live slot); `pinned_refuses_newer`
(`decide_promote` returns `PinnedVersion` when `latest > pinned`); `flip_is_single_rename` (the
`SlotFs` mock records exactly one `rename`); `no_confirm_set` (the `ReleaseSource` adapter never
blocks on a TTY — a daemon must not hang, R5 §2.3). **Adversarial:** a verify failure mid-fetch ⇒
`current` unchanged, idle slot discarded; a fetch that lands a cross-filesystem staging dir ⇒ caught
by the shared-filesystem invariant test (the layout guarantees it); an update attempt while pinned ⇒
refused, no fetch even started.

### 4.6 M6 — real health-check-before-promote gate (exercises a real code path)

`hub-supervisor/src/update.rs` + a `HealthProbe` adapter. R5 §2.3's sharp nuance: **a health check
confirming the process *responds* is NOT proof the app *works*.** The probe **must exercise a real
code path** and return **503 while warming, 200 only when truly ready**. P68's probe, run against the
staged new binary on a scratch port / in probe mode **before** the flip:

- **Event-log head verify** — open the event-log DB and `verify_chain` the tail (kernel, §0): proves
  the new binary can read the real store, not just boot.
- **Synthetic order read** — serve a `/s/:slug` menu-leaf read through the real catalog/order path
  (P62/kernel), asserting a correct, well-formed response (not a 200 with garbage).
- **Kernel order-machine transition** — drive one synthetic order through a real
  `order_machine` transition in an isolated probe context (no real money, no real customer), asserting
  the state advances correctly.

Only when all three pass does `probe` return `Ready` (200); until then `WarmingUp` (503); any failure
is `Failed(reason)`. `decide_promote` reaches `HealthPassed` **only** on `Ready` — a promote of an
unhealthy build has **no producer** (§5.1).

RED→GREEN: `probe_503_until_ready` (WarmingUp before the real path is served, Ready after);
`promote_requires_health_pass` (driving toward a flip without a `Ready` never reaches `Promoted` — a
type/transition assertion). **Adversarial (the load-bearing one — proves the probe is real, not
"process up"):** a staged binary that serves **200 on a shallow `/health` but returns a CORRUPT order
read** ⇒ the probe's order-read assertion fails ⇒ `Failed` ⇒ **NOT promoted**. A binary that boots
but cannot open the event log ⇒ `Failed` at head-verify ⇒ not promoted. A probe that hangs ⇒
`HEALTH_PROBE_TIMEOUT_S` fires ⇒ `HealthTimeout` ⇒ not promoted.

### 4.7 M7 — age-snapshot-before-promote + auto/owner rollback (the rollback-after-migration test)

`hub-supervisor/src/update.rs`. This is R5's #1-ranked risk killed structurally. The promote sequence
(`decide_promote` enforces the ORDER — snapshot, then health, then flip):

1. **Mandatory pre-promote snapshot** — `StateStore::snapshot(from_version)` (M4) is taken **FIRST**,
   before the flip and before the new code serves any real traffic. `Migrated → SnapshotTaken`. There
   is **no transition to `Promoted` that does not pass through `SnapshotTaken`** — promote-without-
   snapshot is unrepresentable (§5.1). This is the whole point of R5 risk #1: forward-only migrations
   (step M5.3) may have run, so the snapshot is the *only* way to get state back to a version the old
   code understands.
2. **Health gate** (M6) → `HealthPassed`.
3. **Flip** (M5, atomic rename) + restart → `Promoted { current, previous, rollback_to: <snapshot> }`.
4. **Post-flip watch** — a `CRASH_LOOP_WINDOW_S` window; if `ServiceCtl::restart_count_since` exceeds
   `CRASH_LOOP_MAX_RESTARTS`, **auto-rollback** (`RollbackTrigger::CrashLoop`).

**Rollback (`decide_rollback` — code AND state both revert):**
`RollbackStep::FlipToPrevious` (symlink back to the previous slot) **AND** `RestoreSnapshot` (restore
the pre-promote `StateSnapshot` — M4, the local plaintext copy so no vendor key is needed) **AND**
`Restart`. Restoring the snapshot is what prevents the corruption R5 risk #1 names: flipping code back
**without** restoring state would leave the old binary running against a **forward-migrated schema**.
Both revert together, to a compatible (code, state) epoch. **Owner-triggered rollback** is the same
step invoked explicitly (`RollbackTrigger::OwnerRequested`) — "flip back + restore snapshot + restart"
in seconds, no re-download (the previous slot is still on disk, R5 §2.3), plus the owner version-pin
(M5) so auto-update does not immediately re-apply the bad version.

**The falsifiable rollback-after-schema-migration test (task-mandated):** simulate — v1 running,
state at epoch `E1`; update to v2 fetched + a **forward migration runs** (mutating the event-log/
projection to a v2 shape); pre-promote snapshot taken at `E1`; flip to v2; v2 **crash-loops**. On
auto-rollback assert: (a) the symlink is flipped back to the v1 slot; (b) `StateStore::restore` is
called with the `E1` snapshot; (c) the post-rollback chain tip **== `E1`** (byte-verified — state is
back at the pre-migration epoch); (d) the running binary is v1 against `E1` state — a **compatible
pair**, never v1-code-against-v2-migrated-schema. **RED (the teeth):** a mutation that flips the slot
back but **skips `RestoreSnapshot`** (code-only rollback) leaves the chain tip at the v2-migrated
epoch ≠ `E1` ⇒ the test asserts the corruption and fails. **GREEN:** both revert, tip == `E1`.
**Adversarial:** (i) the previous slot was deleted ⇒ `NoPreviousSlot` typed error, refuse (never a
silent half-rollback); (ii) the snapshot step was stubbed/failed ⇒ the promote never reached
`SnapshotTaken`, so there was no flip to roll back from (fail-closed — you cannot be stuck
mid-migration with no snapshot); (iii) an owner rollback while auto-update is unpinned ⇒ the rollback
also sets the pin, else the next tick re-applies the bad version (a real footgun, tested).

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose. **dowiz cannot decrypt an off-site backup:** the recipient set has
exactly one source (vendor config) and the seal API takes no other key parameter (§4.2) — "a
dowiz-controlled key in the recipient set" has **no producer**; a `try_open(dowiz_identity)` fails the
AEAD (no matching private key) — dowiz-side plaintext is a *tested-unreachable* state, not a policy.
**Code rollback cannot outrun schema migration:** `Promoted` is reachable only through `SnapshotTaken`
(§4.7) and every rollback restores that snapshot (§4.4) — "old code against a forward-migrated schema"
has no producer (R5 risk #1 killed structurally). **No half-deployed slot is ever served:** the flip
is a single `rename()` (§4.5) — atomic, no interleaving. **No unhealthy build is promoted:** promote
is reachable only through a real-code-path `Ready` (§4.6) — "promoted a broken build" has no producer.
**No brick from a bad update:** auto-rollback on crash-loop + owner one-command rollback + the retained
previous slot make "stuck on a broken version with no way back" unreachable. **Truncation/reorder of a
backup cannot silently restore partial data:** the STREAM counter-nonce + final-flag (§4.1) make a
short/scrambled stream fail to open. **Money:** the supervisor holds no money type at all; the health
probe reads through the order machine in an isolated context and writes nothing.

### 5.2 Schemas & scaling axes (item 8)

`RecipientSet`: axis = vendor-controlled keys — O(few) (a primary + a paper backup); each adds one
stanza (~80 B header). Break point — hundreds of recipients bloat the header; not a real case (§4.2).
Backup blob: axis = hub state size (grows with orders/events); the **STREAM chunking (64 KiB,
constant memory)** handles arbitrary size — no in-memory break point; the break point is
*bandwidth/retention* — a very large full snapshot per tick → **incremental/dedup backup** (named
future axis, not Wave-0, §2). Backup cadence: axis = timer interval; break point — very high order
volume making full snapshots expensive → incremental. Release slots: **fixed at 2 (A/B)** — O(1) disk;
break point — wanting multi-step rollback (N>1 back) → N slots (named, out of scope). Snapshot
retention: exactly one generation — O(1); break point — wanting a rollback ladder → a snapshot ring
(named). No axis touches the mesh transport — every structure is node-local (§5.3).

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

**Isolation/bulkhead:** the pure `hub-supervisor` crate has **no** `self_update`/rclone/systemd/OsRng
dep — those live in the out-of-core `hub-supervisor-adapters` crate behind the `ReleaseSource`/
`SlotFs`/`ServiceCtl`/`HealthProbe`/`BlobSink`/`BlobSource`/`Rng` ports (the P66/P60 firewall). A
failed upload surfaces as a typed `BackupError`, a failed fetch/flip as a typed `UpdateError` — never
a panic, never a propagating fault. The supervisor is a **separate process** from the hub it manages
(the whole point — it must survive the hub crash-looping to roll it back); a supervisor crash never
corrupts hub state (it holds no write path to the event log except the well-fenced `StateStore::restore`
under rollback). **Mesh awareness:** backup + update are **node-local** — the sealed blob goes to S3
via rclone, **never** over `iroh_transport`/`discovery` (zero mesh payload). The one mesh-relevant
angle (stated honestly): auto-update-by-default keeps the mesh from **fragmenting into stale protocol
versions** (R5 §2.1) — but the update channel is a **pull** (`ReleaseSource::latest` fetches a release
feed), not gossip; version skew is a coordination *concern*, the mechanism is node-local. **Living
memory:** the event-log is the append-only content-addressed living memory (§0); the pre-promote
snapshot is a content-addressed epoch marker (`EpochHash`) — a Snapshot-Re-entry point (§5.4); backup
retention is demote-to-cold (the `cold/` prefix already exists on `hetzner:dowiz`, §0), never
silent-delete, matching the living-memory "demote-never-delete" discipline — **except** the deliberate
self-custody boundary (§4.2): a lost vendor key means the off-site backup is unrecoverable, on purpose.

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

**Self-Termination claimed** (hard invariant boundary, unrepresentable state — not a supervisor's
choice): promote-without-snapshot (§4.7), promote-without-health-pass (§4.6), and a dowiz key in the
recipient set (§4.2) are each *unrepresentable* — the type has no producer, not "caught at runtime."
**Self-Healing claimed narrowly** (error-correcting convergence): the crash-loop detector →
auto-rollback (§4.7) converges a bad promote back to the last-good (code, state) epoch; the rclone
retry-with-backoff and the local-blob-retained-on-upload-failure (§4.3) are transport-level
self-heal. **Snapshot-Re-entry claimed** (cheap regenerative recovery from the last valid epoch): the
pre-promote `StateSnapshot` (§4.4) IS a re-entry point — the pre-promote event-log tip `EpochHash` is
the "last valid epoch" marker; rollback = re-enter that epoch cheaply (restore the local plaintext
snapshot + flip the symlink), no re-download, no bespoke recovery path. Mechanical rollback of the
*phase itself*: it is additive (two new hub-side crates, zero edits to the kernel/P60/P66/P67 code —
only the co-signed image-config slices), so deletion restores today's tree.

### 5.5 Linux discipline (item 9) + tensor/spectral/eqc (item 16)

Verdicts per the adoption framework: **ALREADY-EQUIVALENT** — one crypto primitive family
(`pq/x25519` + `pq/keccak::shake256` + `aes-gcm`, all reused, zero new deps), one event-log authority
(the chain-tip `EpochHash`), one transport (the already-live `hetzner:dowiz` rclone remote), one
firewall doctrine (the identifier-absence scan). **REINFORCES** — the A/B-slot + atomic-symlink-flip
pattern is textbook Linux/Unix blue-green (a `rename()` is the canonical atomic-swap primitive), and
the supervisor-as-separate-process is standard init/service-manager discipline; the identifier-absence
firewall extended from `payment.rs`/P60/P66 to the supervisor crate (`no_dowiz_recipient`). **EXTENDS**
— a new hub-side ops surface (the update state machine + the sovereign backup envelope) modeled on the
kernel's `decide`/`fold` law (pure `decide_promote`/`decide_rollback`, tests assert on the
`UpdateState` sequence). **GAP** honestly named — `self_update`, rclone, and systemd/process control
are **not** kernel-native and live out-of-core behind ports; the STREAM chunked-AEAD is a small new
client-side crypto primitive that (unlike the reused ECDH/KDF/AEAD calls) carries a **mandatory
independent-review** burden (§4.1, §6). Item 16: tensor/spectral/eqc machinery is deliberately **NOT**
invoked — the supervisor is ECDH/KDF/AEAD + two small state machines + a symlink flip, where a
spectral form would be ritual math (Anu/Ananke discipline forbids exactly this). The honest reuses:
`event_log::sha3_256` for the epoch anchor and `pq/keccak::shake256` for the envelope KDF — existing
hashes, not new machinery.

### 5.6 Error-propagation gates + smart index (item 14)

Each bug class this blueprint could introduce is turned into a **compile-time or CI-time** failure,
not a runtime surprise: (a) a dowiz-controlled recipient key ⇒ the `no_dowiz_recipient` CI scan + the
recipient-count invariant + the data-flow single-source firewall (§4.2); (b) a promote without a
snapshot or without a health pass ⇒ **unrepresentable** via the `UpdateState` type (§4.6, §4.7) —
caught at compile time, no producer exists; (c) a code-only rollback (skips state restore) ⇒ the
rollback-after-schema-migration regression test (§4.7) goes RED; (d) a truncated/reordered backup ⇒
the STREAM adversarial tests (§4.1) + the mandatory crypto review; (e) `self_update` used for rollback
or pinning ⇒ a scope-fence (rollback/pin live only in `hub-supervisor`, `self_update` only behind
`ReleaseSource::fetch_verified`); (f) an `age`/`rage` or CRDT/HKDF dependency creeping in ⇒ a
`deny.toml`/dependency-fence entry barring them from the supervisor crate (mirrors the kernel
dependency fences and P66's CRDT fence). Type system first, CI scan second — never a runtime check.

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no envelope; hand-rolled STREAM unreviewed | seal/open round-trips byte-identical (single + multi vendor recipient); **tampered/truncated/reordered/extended ⇒ typed fail, no partial write**; **independent crypto review of the STREAM construction signed off** | **backup-envelope-STREAM-integrity** test + review gate (ledger row) |
| M2 | no recipient firewall | **no-dowiz-recipient-key test green**: recipient set == vendor config, no dowiz key present, `try_open(dowiz)` ⇒ `AeadInvalid`; adding a `from_dowiz`/hardcoded stanza fails the build | **no-dowiz-recipient** scan + test (ledger row) |
| M3 | no scheduler/transport | tick seals + `rclone`-puts to `<hub-id>/backups/`; restore round-trips via mock remote; vendor-S3 override selected; upload failure ⇒ retained + retried | backup-scheduler explicit-put test |
| M4 | no snapshot primitive | `EpochHash` == event-log tip at capture; **restore lands the tip exactly at the epoch (byte-verified)**; local plaintext snapshot never uploaded | state-snapshot-lands-at-epoch test (ledger row) |
| M5 | no A/B machine | fetch stages into the IDLE slot; **flip is a single `rename()`**; pinned refuses newer; `.no_confirm(true)` (no TTY block) | atomic-flip + fetch-into-idle test |
| M6 | naive "process up" health | **503 until a REAL code path (event-log verify + order read + order-machine transition) serves correctly, 200 only then**; a build that 200s `/health` but serves a corrupt order read ⇒ NOT promoted | **real-code-path-health-gate** test (ledger row) |
| M7 | code-only rollback corrupts state | **snapshot taken BEFORE promote (unrepresentable to skip)**; **rollback-after-schema-migration test**: forward migration runs, v2 crash-loops, auto-rollback flips slot AND restores snapshot, tip == pre-promote epoch; code-only-rollback mutation ⇒ tip ≠ epoch ⇒ RED | **rollback-after-schema-migration** test (ledger row) |

**Not-done clauses:** any code path that puts a dowiz-controlled key in a recipient set, or any
off-site backup dowiz can decrypt = **NOT done** (§4.2, §16.27 red-line); a promote reachable without
a pre-promote snapshot, or a rollback that reverts code without restoring state = **NOT done** (§4.7,
R5 risk #1 red-line); a health gate that promotes on "process responds" without exercising a real code
path = **NOT done** (§4.6); a hand-rolled STREAM shipped without an independent crypto review = **NOT
done** (§4.1, `crypto-safe-first-pass` lesson); `self_update` used for rollback or pinning = **NOT
done** (R5 §2.2); the `age`/`rage` crate pulled in for the cipher = **NOT done** (DECART §4.1);
redefining P67's golden-image spec (tunnel/certs/fixtures/ingress) from inside P68 = **NOT done** (X9);
a non-atomic (non-`rename`) slot flip = **NOT done**.

---

## 7. Benchmark plan (item 10) — pure crypto legs micro-benched; I/O out-of-core

Criterion harness (the kernel bench discipline, reused), on the pure `hub-supervisor` crate:
`backup/seal_open_100mb` (X25519 wrap + SHAKE256 KDF + AES-256-GCM STREAM over a 100 MB archive —
target sustained throughput bounded by AES-NI, e.g. > 500 MB/s single-core; the reuse means AES-NI
applies), `backup/wrap_per_recipient` (one recipient stanza — target < 100 µs),
`update/decide_promote` + `update/decide_rollback` (the pure order-enforcing branches — target
< 1 µs), `snapshot/chain_tip_anchor` (reading the event-log tip — target < 10 µs). All added
RED-commit-first so baselines auto-seed; results to `BENCH_HISTORY.md`, never prose estimates.
**Out-of-core** — the rclone upload, the `self_update` fetch, the health-probe latency, and the
symlink flip (a single `rename`, ~µs) are **not** kernel-micro-benched; covered by the
`hub-supervisor-adapters` integration test with a **health-probe-latency budget** (must be well under
`HEALTH_PROBE_TIMEOUT_S`) and an **upload-throughput budget**. Telemetry (client-side native trackers,
P-H lane): `backup_succeeded`/`backup_failed{reason}`, `update_promoted`/`update_rolled_back{trigger}`,
`health_probe_failed`, `dowiz_recipient_rejected` (the §4-B guard firing), and the elegant
safety-property-made-observable counter **`pre_promote_snapshot_taken` which MUST equal
`promote_attempts`** — a mismatch means a promote occurred without a snapshot (the R5-risk-#1 red-line
regression), surfaced automatically, not only at review (mirrors P66's `double_charge_averted`
counter). A `rollback_state_restored` counter increments only when a rollback both flipped the slot
AND restored the snapshot — a code-only rollback would leave it un-incremented, a detectable
regression.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the 20-point contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` **X9 (golden snapshot — the integration point for
provisioning/update/backup; P67 owns the image spec, P68 co-owns the slot/backup layout; the promote
step MUST age-snapshot first)**, **§4-B (self-custody severity — CLOSED to absolute, NO break-glass in
the backup recipient set)**, X8 (backup crypto shares the primitive family with the cert chain but is
a SEPARATE, simpler mechanism), §5 W3 P68 row ·
`docs/research/OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md` (read in full — **§0/§1.1-1.3 age
envelope over rclone not rclone-crypt; §2.1-2.4 A/B-slot + symlink-flip + health-gate + `self_update`
fetch-only; §7 risk #1 migration/state rollback, risk #2 break-glass**) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` **§16.27** (encrypted auto-backup dowiz-never-
plaintext + auto-update-by-default with owner rollback), §16.29 (media/dispute boundary — not P68's),
§16.47 (self-custody, loss is the user's responsibility — the backup-key parity with the wallet),
§16.54 (AGPLv3 open-source hub binary — the restore tool), §16.14 (no central state) ·
`BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md` (the LIVE `hetzner:dowiz` remote — transport reused).
**Golden-image co-ownership:** **P67** (`Hub provisioning & claim` — owns the image spec: hub binary,
`cloudflared`+tunnel, pre-minted P59 certs, demo fixtures, shadow ingress, warm-pool/claim; **P68
co-signs the A/B slot layout + the backup-scheduler config within that image, and consumes the
claim-time vendor-recipient-pubkey handoff** — X9). **Upstream primitives (reused, never redefined):**
the kernel `pq/x25519` + `pq/keccak::shake256` + `aes-gcm` + `event_log` (chain-tip anchor +
`verify_chain`); the PgStore/W13 projection (the state-store snapshot target). **Sibling (family
shared, not merged):** **P59** capability-cert chain (X8 — same `pq` primitives, separate mechanism,
domain-separated KDF context). **Peer parity:** **P66** (data wallet — the identical primitive-reuse
finding, the identical §4-B self-custody ruling applied to wallet keys; P68 applies it to backup keys
for cross-surface consistency). Memory: `crypto-safe-first-pass-2026-07-14` (crypto reused, not
re-implemented; the hand-rolled-crypto forgery lesson → the §4.1 mandatory-review gate) ·
`rust-native-bare-metal-decision-2026-07-14` (DECART tables §3/§4.1) · `ops-reliability-arc-2026-07-13`
(3-2-1-1-0 backups, degrade-closed, resurrect-from-attic) · `worktree-remote-push-collision-avoidance`
(the confirmed-data-loss precedent that motivates real backup rigor) · `never-bypass-human-gates-
2026-06-29` (§4-B was human-gated — now closed, applied not re-opened) · `verified-by-math-2026-07-07`.
Supersedes: nothing (additive, greenfield).

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the `BackupHeader`/`StateSnapshot`/`UpdateState` types and the
  pure `decide_promote`/`decide_rollback` functions (§3) precede any adapter; the pure logic is the
  source, the `self_update`/rclone/systemd adapters are the derived shadow.
- **P2 CORRESPONDENCE** (one concept, one primitive): one crypto family (kernel `pq`, reused not
  re-vendored), one epoch anchor (the event-log tip), one snapshot mechanism (M4 — two consumers:
  off-site-encrypted + local-plaintext), one transport (the live `hetzner:dowiz` remote) — the same
  concept never gets a second implementation.
- **P4 POLARITY** (paired inverses as law): seal↔open (AES-GCM STREAM), promote↔rollback (the
  snapshot-anchored inverse that reverts code AND state together, §4.7), fetch-forward↔pin-hold.
- **P6 CAUSE-AND-EFFECT** (determinism as law): the content-addressed epoch hash makes a restore
  verifiably land at exactly the pre-promote state; the STREAM counter-nonce makes truncation
  deterministically fail — each safety property carries a falsifier (the regression tests, §6).
- **P7 GENDER** (paired verification, no self-certification): a promote is not self-certified by "the
  process started" — it is refereed by an *independent real-code-path health probe* (§4.6) before the
  flip; the hand-rolled STREAM is not self-certified — it requires an *independent adversarial crypto
  review* (§4.1). Neither the promote moment nor the crypto trusts a single unrefereed party.

(P3/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; reused `pq/x25519`+`keccak`+`aes-gcm`+`event_log`, live `hetzner:dowiz`, `self_update` forward-only, `age` pre-1.0, P67 image ownership) |
| 2 DoD | §6 |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; `UpdateState` sequence assertions, pure `decide_*` |
| 4 predefined types/consts | §3 |
| 5 adversarial/breaking tests | §4.1–4.7 (tamper/truncate/reorder/extend-STREAM, injected-dowiz-key, corrupt-blob, 200-but-corrupt-order health, crash-loop rollback, code-only-rollback teeth, deleted-previous-slot) |
| 6 hazard-safety as math | §5.1 (no-dowiz-plaintext, no-code-outruns-schema, no-half-slot, no-unhealthy-promote, no-brick, no-silent-truncation — all reachability) |
| 7 links docs/memory | §8 (P67 co-owner, P59 sibling, P66 parity, kernel primitives upstream) |
| 8 scaling axes | §5.2 (each with a named break point; incremental-backup named future) |
| 9 Linux discipline | §5.5 (all verdict classes incl. the honest self_update/rclone GAP + the STREAM-review burden) |
| 10 benchmarks+telemetry | §7 (pure crypto/decide legs benched; I/O out-of-core; the snapshot==promote safety counter) |
| 11 isolation/bulkhead | §5.3 (pure crate + out-of-core adapters; supervisor a separate process from the hub) |
| 12 mesh awareness | §5.3 (node-local; backup never on the mesh transport; auto-update-vs-version-skew is pull not gossip) |
| 13 rollback/self-heal vocabulary | §5.4 (Self-Termination + Self-Healing + Snapshot-Re-entry claimed precisely) |
| 14 error-propagation gates | §5.6 (type-first — unrepresentable promote/rollback — then CI scans + dep fence) |
| 15 living memory | §5.3 (append-only event-log epoch anchor; cold-prefix demote-not-delete; the deliberate self-custody boundary) |
| 16 tensor/spectral + eqc reuse | §5.5 (spectral honestly NOT invoked; sha3_256 + shake256 reused) |
| 17 regression ledger | §6 (six+ rows incl. the two task-mandated tests) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0/§2/§4 (`pq` primitives, `event_log`, live `hetzner:dowiz`, `self_update` fetch-only all reused; DECART §4.1; `age`/rclone-crypt/ChaCha/HKDF/self_update-rollback rejected with reasons) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Order below is the dependency order; T1–T5 are buildable today with zero network (pure logic + the
already-in-tree `pq` crypto + a mock event-log store). T6 needs the platform adapters. Nothing waits
on an operator gate — §16.27 and §4-B are closed. The two co-signed image slices (slot layout +
backup-scheduler config) are handed to **P67 by blueprint number** — P68 does not edit P67's image
spec, it supplies these two slices for P67 to bake.

1. **T1 (M1 — the sovereign backup envelope + STREAM).** Create crate `hub-supervisor` (repo root,
   `Cargo.toml` path-dep `dowiz-kernel = { path = "../kernel", features = ["pq"] }`). Write
   `backup.rs` per §3. Implement `seal`/`open` with X25519 wrap via `dowiz_kernel::pq::x25519::x25519`,
   KDF via `dowiz_kernel::pq::keccak::shake256`, AEAD via `aes-gcm` (already a kernel dep), and the
   **STREAM** counter-nonce + `STREAM_FINAL` construction. Write the round-trip + the tamper/**truncate**/
   reorder/extend adversarial tests FIRST. **BLOCK on an independent adversarial crypto review** of the
   STREAM construction before marking M1 done (§4.1; `crypto-safe-first-pass` lesson). Acceptance:
   `cargo test -p hub-supervisor` green; the truncation test proves a dropped final chunk ⇒ `Truncated`.
2. **T2 (M2 — the §4-B no-dowiz-recipient firewall).** In `backup.rs`, make `RecipientPubKey`'s only
   ctor `from_vendor_config`; make `RecipientSet` buildable only from those; the `seal` API takes no
   other key source. Write the `no_dowiz_recipient` scan FIRST (copy the `FORBIDDEN`+`concat!` pattern
   from `kernel/src/ports/payment.rs:508-560` / P66 §4.7). Write the **no-dowiz-recipient-key test**:
   recipient count == vendor config, no dowiz fixture key present, `try_open(dowiz_identity)` ⇒
   `AeadInvalid`. Acceptance: green; adding a `from_dowiz` ctor or a hardcoded stanza fails the build.
3. **T3 (M4 — the state snapshot primitive).** Write `snapshot.rs` per §3. `EpochHash` = the
   event-log chain tip (`dowiz_kernel::event_log` tip); implement `snapshot`/`restore` against a
   `StateStore` port (mock the event-log + projection in tests). RED: `snapshot_captures_chain_tip`,
   `restore_lands_at_epoch` (byte-verified), `local_snapshot_is_not_uploaded`. Acceptance: green.
4. **T4 (M5 — the A/B update machine + fetch-only + pinning).** Write `update.rs` per §3. Model
   `UpdateState` so `Promoted` is reachable ONLY through `SnapshotTaken` then `HealthPassed`. Write
   `decide_promote` (order-enforcing + pin check) and `decide_rollback` (flip AND restore). Wrap
   `self_update` behind `ReleaseSource::fetch_verified` (FETCH+VERIFY ONLY, `.no_confirm(true)`). RED:
   `fetch_stages_into_idle_slot`, `pinned_refuses_newer`, `flip_is_single_rename`, `promote_requires_
   snapshot_and_health` (unrepresentable to skip). Acceptance: green.
5. **T5 (M6+M7 — real health gate + the rollback-after-migration test).** In `update.rs`, define the
   `HealthProbe` contract that exercises a real code path (event-log verify + order read + order-machine
   transition; 503→200). Write the **real-code-path-health-gate test** (a 200-but-corrupt-order build ⇒
   NOT promoted) and the **rollback-after-schema-migration test** (forward migration runs, v2
   crash-loops, auto-rollback flips slot AND restores snapshot, tip == pre-promote epoch; the code-only-
   rollback mutation ⇒ tip ≠ epoch ⇒ RED). Acceptance: `cargo test -p hub-supervisor` green.
6. **T6 (M3 + the platform adapters — out-of-core).** New crate `hub-supervisor-adapters` (repo root,
   path-dep on `hub-supervisor`; `self_update`/rclone-shell-out/systemd/OsRng deps live HERE). Implement
   `ReleaseSource` (self_update), `SlotFs` (atomic `rename` flip), `ServiceCtl` (restart + crash-loop
   count), `HealthProbe`, `BlobSink`/`BlobSource` (rclone `copy` to `hetzner:dowiz/<hub-id>/backups/`
   or vendor-S3), `Rng` (OsRng). RED (headless/mock): explicit-put asserted, restore round-trips,
   vendor-S3 override. Acceptance: `cargo test -p hub-supervisor-adapters` green; `cargo tree -p
   hub-supervisor` shows NO `self_update`/rclone/`age` dependency (the firewall holds).
7. **T7 (image co-sign + fences + ledger).** Add the CRDT/`age`/HKDF/`self_update`-rollback dependency
   fences to `deny.toml` (bar them from `hub-supervisor`). Supply P67 (by blueprint number) the two
   co-signed image slices: the A/B slot layout (`releases/{A,B}/`, `current` symlink, the
   `dowiz-hub-supervisor` unit, unpinned-default) and the backup-scheduler config (cadence, default
   `hetzner:dowiz/<hub-id>/` remote + vendor-S3 stub, EMPTY recipient-config location — the vendor
   pubkey arrives at claim, never baked). Add the six §6 ledger rows to
   `docs/regressions/REGRESSION-LEDGER.md`; name the no-dowiz-recipient-key and
   rollback-after-schema-migration tests as permanent regressions.
