# AUDIT — Roadmap Item 31 (investigative half): dependency-audit findings

> Closes the three sub-investigations of `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md`
> §A Tier 0 · Item 31. Independently re-verifies the Fable blueprint
> ([`BLUEPRINT-ITEM-31-dependency-audit-2026-07-19.md`](BLUEPRINT-ITEM-31-dependency-audit-2026-07-19.md)),
> applies the one confirmed code fix (cosmic-text pin), and files the two docs-only rulings.
> Every citation below was re-read against live `HEAD` on 2026-07-19; **one material blueprint
> claim in finding (c) was found wrong and is corrected here** (see §(c) "Correction").

Enactment half (per-crate allowlist CI gate) is Tier 2, depends on items 1 + 25 — not this doc.

---

## (a) `rusqlite` — VERDICT: **KEEP and contain** (cat-2 foreign format at a tool boundary). Blueprint CONFIRMED.

**Independent citation-check (all re-read, all match the blueprint):**
- Declaration `tools/deep-clean/Cargo.toml:10` — `rusqlite = { version = "0.31", features = ["bundled"] }`. Sole rusqlite declaration in the repo. Comment at line 8: `Offline-buildable: rusqlite + libsqlite3-sys are in the cargo cache.` ✅
- Usage sites (all inside the one binary `tools/deep-clean/src/main.rs`):
  - `main.rs:92` — `rusqlite::Connection::open(STATE_DB)` + `execute_batch("VACUUM;")` (line 95) — `vacuum` subcommand.
  - `main.rs:147` (prune dry) / `main.rs:158` (prune commit) — `DELETE FROM messages …` / `DELETE FROM sessions …` (lines 160-161), FTS rebuild + `VACUUM`.
  - `main.rs:776` — `rusqlite::Connection::open(db)` + `query_row("PRAGMA integrity_check", …)` (line 778) — `restore-verify` drill; test fixture at `main.rs:969` / assert at `:980`.
- Crate doc-header operator note `main.rs:16-22` (2026-07-16) re-read verbatim: dowiz's own data is **SQLLESS** (content-addressed `BlockStore` + JSONL `FileEventStore`, **pgrust** as the uniform backup/fallback), never SQLite; `vacuum`/`prune`/`PRAGMA integrity_check` exist **only** because Hermes — external host tooling this crate does not own — keeps its state in `state.db`. ✅

**Ruling.** Every usage targets Hermes's `state.db`, a foreign SQLite-format file the crate reads but does not own — the synthesis §13(c) **cat-2** branch (*reads existing SQLite-format files it does not own* → keep, minimize). It is load-bearing (vacuum/prune/integrity-drill are real ops functions) and lives in a standalone operator tool that is in **no kernel/engine/default build path**, so it does not violate the zero-dep kernel discipline. Zero-dep removal would mean either reimplementing the SQLite file format + VACUUM (a foreign wire format far past `flate2`'s RFC 1951 — rejected by the cat-2 rule) or amputating three real subcommands. Neither is warranted. **KEEP bundled; never extend; audited 2026-07-19 item 31.** The enactment half (item 31 Tier 2) lists `rusqlite` in deep-clean's per-crate allowlist and may add the one-line manifest ruling comment above `Cargo.toml:10` under item 25's slot-arena ruling format; recording the decision here satisfies the investigative half.

## (b) `cosmic-text = "*"` wildcard — VERDICT: **DEFECT CONFIRMED, FIXED** (pinned to resolved `0.19.0`). Blueprint CONFIRMED + applied.

**Independent citation-check (all match):**
- `engine/Cargo.toml:30` — `cosmic-text = { version = "*", optional = true }`, carried by feature `text = ["dep:cosmic-text"]` (`engine/Cargo.toml:65`), OFF the default build (P57 Lane B only). No second declaration anywhere (`wasm/Cargo.toml` mentions it in a comment only). ✅
- Resolved version `engine/Cargo.lock:138-141` — `cosmic-text 0.19.0`, checksum `be17b688510d934ce13f48a2beba700e11583e281e0fda99c22bb256a14eda73`. The engine lock is the authority; the dep is absent from `wasm/Cargo.lock`, so the wildcard's blast radius is the engine crate alone. ✅

**Fix applied.** `engine/Cargo.toml:30` → `cosmic-text = { version = "0.19.0", optional = true }`, an exact pin to the already-resolved version. Verified in worktree `/root/dowiz-wt-space-grade-exec` (branch `exec/space-grade-tier0-2026-07-19`):
- `cargo check -p dowiz-engine --features text --offline --locked` → exit 0 (warnings only, no errors). `--locked` proves the lockfile needs no change to satisfy the new constraint.
- `git diff engine/Cargo.lock` → **empty**. The pin matched the resolved graph exactly; no version bump, no checksum churn, no new fetch.
- Committed `c2d0f306a` (`fix(engine): pin cosmic-text wildcard to resolved 0.19.0 (roadmap item 31b)`), pushed to `origin exec/space-grade-tier0-2026-07-19` (`94c29146b..c2d0f306a`).

## (c) `sha2` vs kernel Keccak on the body digest — VERDICT: **NOT a correctness defect; KEEP `sha2`.** Blueprint's *conclusion* stands, but its *removal-path reasoning was materially wrong* — corrected below.

**Independent citation-check (all match):**
- Declaration `tools/native-spa-server/Cargo.toml:35` — `sha2 = "0.10"`. ✅
- Usage (all one crate): import `api.rs:45` (`use sha2::{Digest, Sha256};`); helper `fn sha256` at `api.rs:647` (`Sha256::new()`); **frame replay-ring digest** `api.rs:437` (`sha256(&cap_hdr)`); **body digest** `api.rs:441` (`sha256(&body_bytes)`, bound into the capability-scope check); `mint_frame` at `api.rs:715` (`body_digest: &[u8; 32]` param). Tests re-derive in `tests/agent_route.rs` / `tests/integration.rs`. ✅
- **Dual in-kernel Keccak-f[1600] — CONFIRMED.** `kernel/src/event_log.rs:67` carries a **local** `fn keccak_f(s: &mut [u64; 25])` (full θ/ρ/π/χ/ι round loop, its own RC/R tables at `:44-65`) for the internal event-chain digest. Separately, `kernel/src/pq/keccak.rs:156` exposes `pub fn sha3_256(input: &[u8]) -> [u8; 32]` over the audited FIPS-202 sponge. Two independent Keccak-f[1600] permutations coexist in the kernel. ✅

**No browser-side minter exists (re-verified):** `grep -rniE 'crypto\.subtle|sha-?256|mint_frame|x-dowiz-cap' apps/web/src` → **zero matches**. P59 (`SelfSignedRoot::mint`) mints roots at **bake/provision/first-boot time, server-side** (`BLUEPRINT-P67` §9.3, `BLUEPRINT-P68` L54/L145), never in a browser. Both sides of the `x-dowiz-cap` wire format live in native-spa-server today.

### Correction to the blueprint (material)

The blueprint argued the swap is nearly free because *"`pub mod pq` is unconditional (`kernel/src/lib.rs:14`)… the replacement primitive is already linked."* **Both halves are false:**
1. `kernel/src/lib.rs:13-14` reads `#[cfg(feature = "pq")]` / `pub mod pq;` — the `pq` module is **feature-gated, not unconditional**.
2. native-spa-server enables **only** `features = ["json-api"]` (`Cargo.toml:38`), and `json-api = ["dep:serde", "dep:serde_json"]` (`kernel/Cargo.toml:24`) — it does **not** enable `pq`. So `kernel::pq::keccak::sha3_256` is **not currently linked** into native-spa-server.

The cited precedent confirms the correction rather than the claim: `tools/nfc-pod-codec/Cargo.toml:27` reaches `kernel::pq::keccak::shake256` by explicitly setting `features = ["pq"]`. Enabling `pq` on native-spa-server would pull the whole PQ crypto core — `pq = ["dep:serde", "dep:serde_json", "dep:aes-gcm", "dep:curve25519-dalek"]` (`kernel/Cargo.toml:56`) — dragging **`aes-gcm` + `curve25519-dalek`** into the server's graph just to reach one hash. Removing `sha2` (one tightly-scoped hash crate) therefore *increases* the external-crate count, not decreases it.

**Ruling.** KEEP `sha2`. It is (1) not a correctness defect — the event-chain Keccak (internal tamper-evidence) and the HTTP body digest (capability scope-binding at a boundary) are different domains and are not required to share an algorithm; (2) **not "dead weight"** — the only zero-hand-rolled-crypto removal path costs *more* dependencies than it saves and forecloses future browser-side `SubtleCrypto` frame minting (WebCrypto has SHA-256, no SHA-3); (3) hand-rolling SHA-256 as a third option is barred by the nfc-pod-codec "no new crypto" discipline. Record the ruling; allowlist `sha2` when item 31's enactment half builds the per-crate gate.

### Follow-up flagged (NOT fixed here — new scope)

The **dual in-kernel Keccak-f[1600]** (`event_log.rs:67` local `keccak_f` vs `pq/keccak.rs` FIPS-202 sponge) is real and out of item-31 scope. It warrants a **future dedup ticket under item 25's procedure** (which implementation is authoritative — likely collapse `event_log.rs` onto `pq::keccak`, gating the event-chain digest behind or alongside the audited module). This is a design decision (feature-gating the event log's digest on `pq` would change the default kernel's build graph), so it is **flagged, not silently fixed**.

---

## Handoff

- Roadmap-index row: added under `CORE-ROADMAP-INDEX.md` §7 (cross-cutting arcs), per its "every planning doc gets a row" rule.
- Execution-roadmap item 31 entry: marked complete with a pointer to this doc.
- Net code change: one line (`engine/Cargo.toml:30` pin), committed + pushed on the worktree branch. Findings (a) and (c) are docs-only rulings recorded above.
- Carried-forward tickets: (1) item-31 enactment half — allowlist `rusqlite` + `sha2`, add manifest ruling comments; (2) dual-Keccak dedup under item 25.
