# BLUEPRINT — Roadmap Item 31 (investigative half): dependency audit trio

> Tier 0 of `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §A. Source table:
> `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §25 workspace dependency audit
> (rows: `engine`/cosmic-text, `tools/deep-clean`/rusqlite, `tools/native-spa-server`/sha2).
> This doc closes the three investigations; the enactment half (per-crate allowlist CI gate)
> is Tier 2 and depends on items 1 + 25.

## (a) `rusqlite` — usage read + reclassification. VERDICT: KEEP (cat-2 foreign format at a tool boundary).

**Declaration:** `tools/deep-clean/Cargo.toml:10` — `rusqlite = { version = "0.31", features = ["bundled"] }`
(comment at line 8: bundled so the crate stays offline-buildable). Sole rusqlite crate in the repo.

**Usage sites (all in the one 1061-line binary `tools/deep-clean/src/main.rs`):**
- `main.rs:92` — `vacuum`: `Connection::open(STATE_DB)` + `VACUUM;` on the Hermes `state.db`.
- `main.rs:147`, `main.rs:158` — `prune` (dry + commit): `DELETE FROM sessions/messages … ended_at < ?`,
  FTS5 `'rebuild'`, `VACUUM` — schema is Hermes's, not dowiz's.
- `main.rs:775-782` — `sqlite_integrity`: `PRAGMA integrity_check` on a scratch copy during the
  `restore-verify` backup drill (caller at `main.rs:684`); test fixture at `main.rs:962-969`.

**Reclassification (was: needs-usage-read, verdict withheld — synthesis §25 table).** Every usage
targets Hermes host tooling's `state.db`. The crate's own doc-header (`main.rs` lines ~16-22,
operator note 2026-07-16) already rules this: dowiz-owned data is SQLLESS
(BlockStore/FileEventStore, pgrust fallback); SQLite exists here **only** as host-tooling compat.
That is exactly the synthesis's §13(c) cat-2 branch — *reads existing SQLite-format files it does
not own* → **keep, minimize**. It is load-bearing (vacuum/prune/integrity-drill are real ops
functions), and it sits in a standalone operator tool that is in no kernel/engine/default build
path, so it does not violate the zero-dep kernel discipline.

**Zero-dep removal would require** either reimplementing the SQLite file format + VACUUM (a
foreign wire format orders of magnitude past `flate2`'s RFC 1951 — rejected by the synthesis's own
cat-2 rule) or amputating the three subcommands (loses real function). Neither is warranted.

**Executor one-liner:** add the ruling as a manifest comment above `tools/deep-clean/Cargo.toml:10`
in the `slot_arena.rs` ruling format (per item 25's procedure): `cat-2 foreign format (Hermes
state.db, host-compat only, never extend — see main.rs header note); KEEP bundled; audited
2026-07-19 item 31`. Then list `rusqlite` in deep-clean's per-crate allowlist when item 31's
enactment half builds the CI gate.

## (b) `cosmic-text = "*"` — wildcard pin. VERDICT: DEFECT CONFIRMED; pin to 0.19.0.

**Declaration:** `engine/Cargo.toml:30` — `cosmic-text = { version = "*", optional = true }`,
carried by feature `text = ["dep:cosmic-text"]` (`engine/Cargo.toml:65`). Off the default build
(P57 Lane B only). `wasm/Cargo.toml:33` mentions cosmic-text in a comment only — no second
declaration anywhere in the repo.

**Currently resolved:** `engine/Cargo.lock:138-141` —
`cosmic-text 0.19.0`, checksum `be17b688510d934ce13f48a2beba700e11583e281e0fda99c22bb256a14eda73`.
(No workspace-root lockfile; the engine lock is the authority for this dep. It does not appear in
`wasm/Cargo.lock` — the wildcard's blast radius is the engine crate alone.)

**Executor one-liner (direct fix, no investigation needed):** change `engine/Cargo.toml:30` to
`cosmic-text = { version = "0.19.0", optional = true }` and confirm `engine/Cargo.lock` is
unchanged (`cargo update -p cosmic-text --precise 0.19.0` is a no-op check). Caret-0.19.0 is
sufficient because items 1+13 add the lockfile-hash CI assertion; if the executor wants
belt-and-braces before that gate exists, `"=0.19.0"` is the stricter legal option — either
resolves the synthesis-flagged defect.

## (c) `sha2` vs kernel Keccak on the body digest. VERDICT: NOT a correctness defect — two digests serve two domains — but the `sha2` dep IS removable, conditionally.

**Declaration:** `tools/native-spa-server/Cargo.toml:35` — `sha2 = "0.10"`.

**Usage sites (all one crate):** `tools/native-spa-server/src/api.rs:45` (`use sha2::{Digest,
Sha256}`); helper `fn sha256` at `api.rs:647`; **body digest** at `api.rs:441`
(`sha256(&body_bytes)` — SHA-256 of raw request bytes, bound into the capability frame payload as
`(route ‖ body_digest ‖ epoch)`, minted at `api.rs:715 mint_frame`, checked at `api.rs:289-295`);
**frame replay-ring digest** at `api.rs:437` (`sha256(&cap_hdr)`). Tests re-derive it at
`tests/agent_route.rs:191` and `tests/integration.rs:198`.

**The kernel-Keccak side:** `kernel/src/pq/keccak.rs` exposes `sha3_256` (line 156) from the
audited FIPS-202 module; `pub mod pq` is unconditional (`kernel/src/lib.rs:14`), and
native-spa-server already depends on the kernel (`tools/native-spa-server/Cargo.toml:38`,
`json-api` feature) — so the replacement primitive is *already linked*. Precedent:
`tools/nfc-pod-codec/src/pod.rs:11-22` reuses `kernel::pq::keccak` explicitly under a "no new
crypto" rule. (Separately: `kernel/src/event_log.rs:67` carries its **own** local `keccak_f` for
the internal event-chain digest — a second in-kernel Keccak-f[1600] alongside `pq/keccak.rs`.
Out of item-31 scope; flagged for a future dedup pass, not this one.)

**Is SHA-256 externally pinned?** No — checked. `mint_frame` is called only from `api.rs` itself
and the two in-crate test files; there is no browser-side minter (`crypto.subtle`/SHA-256 greps of
`apps/web/src` are empty) and no webhook/HMAC handler in native-spa-server sources. Both sides of
the `x-dowiz-cap` wire format live in this one crate today.

**Honest ruling (per the synthesis's own "verify before swapping" caveat):**
1. **Not an inconsistency to "unify".** The event-chain Keccak (internal tamper-evidence) and the
   HTTP body digest (capability scope-binding at a boundary) are different domains; nothing
   requires them to share an algorithm. No defect exists in the current code's correctness.
2. **But the dep is dead weight *unless* one future door matters:** browser `SubtleCrypto`
   supports SHA-256 natively and SHA-3 not at all. Swapping to `sha3_256` forecloses zero-dep
   client-side frame minting in a browser (P59 capability-chain adjacency).
3. **Do not hand-roll SHA-256** as a third option — it violates the nfc-pod-codec "no new crypto"
   reuse discipline for zero benefit.

**Executor decision rule (one binary check, then one mechanical change):** if browser-side frame
minting is on any accepted blueprint (check P59 + the P57 SPA surface) → **keep `sha2`**, record
the ruling in the manifest, allowlist it. If not → replace both `sha256` call sites'
implementation (swap `fn sha256`'s body at `api.rs:647` for `dowiz_kernel::pq::keccak::sha3_256`,
delete the import at `api.rs:45`, update the two test digests), delete `Cargo.toml:35`. Single
crate, no cross-repo coordination, ~10-line diff either way.

## Handoff

Per `CORE-ROADMAP-INDEX.md` "every planning doc gets a row" — add:

| Phase / doc | Component | Blueprint | One-line note |
|---|---|---|---|
| Space-grade item 31 (investigative half) | CORE | [BLUEPRINT-ITEM-31-dependency-audit-2026-07-19.md](BLUEPRINT-ITEM-31-dependency-audit-2026-07-19.md) | All 3 sub-checks closed: rusqlite KEEP (cat-2, Hermes host-compat, manifest ruling to write) · cosmic-text wildcard → pin `0.19.0` at `engine/Cargo.toml:30` (lock-confirmed, direct fix) · sha2-vs-keccak NOT a defect, dep removable iff no browser-minting door (P59 check → 10-line swap to `kernel::pq::keccak::sha3_256`) |

**Direct actions for the enactment executor, in order of certainty:** (1) the cosmic-text pin —
zero-risk, do first; (2) the rusqlite manifest ruling comment — documentation only; (3) the sha2
decision — run the §(c) binary check, then either allowlist or swap. Bonus flag carried forward:
dual in-kernel Keccak-f[1600] (`event_log.rs:67` vs `pq/keccak.rs`) — file under a future item-25
procedure pass, not item 31.
