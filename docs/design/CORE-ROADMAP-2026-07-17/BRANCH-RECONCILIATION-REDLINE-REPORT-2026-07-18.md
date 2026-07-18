# Branch Reconciliation Report — RED-LINE branches (money · signing · PQ crypto), 2026-07-18

Branch: `fix/reconcile-redline-branches-2026-07-18` (off `main` @ `ac6751108`, which carries the
full §16–§18 axis + the prior reconciliation pass's merges). Scope: the three red-line-sensitive
feature branches named in the task (`feat/p47-payment-rail`, `feat/p06-v1-real-signer`,
`feat/pq-crypto-tier1`) plus the stale twin `feat/pq-crypto-tier1-snapshot-2026-07-18`.
Methodology and tone follow the prior pass (`BRANCH-RECONCILIATION-REPORT-2026-07-18.md`):
investigate real content before merging, `--no-ff` real merges (no squash/rebase), resolve
conflicts by understanding both sides, flag anything too large/stale/risky for archival instead
of merge. **Every conflict here was treated with adversarial-review rigor, not routine
doc-merge rigor.** **Nothing here is merged into `main` — that decision is the operator's.**

## 0. First finding: the task brief's ahead/behind numbers were inverted (same as the prior pass)

The brief records "p47 = 83 ahead / 1 behind", "p06 = 147 ahead / 1 behind",
"pq-tier1 = 479 ahead / 23 behind", "snapshot = 1069 ahead / 613 behind". Live
`git rev-list --count` at reconciliation start showed the **inverse** — `main` had already
absorbed the bulk of the small groups via the same-day merge wave:

| Branch | Brief claim | Live (this pass, vs `origin/main`) | Merge-base with `main` |
|---|---|---|---|
| `feat/p47-payment-rail` | 83 ahead / 1 behind | **1 ahead / 83 behind** | `b7577e228` (the commit that *added* `payment_capability.rs`) |
| `feat/p06-v1-real-signer` | 147 ahead / 1 behind | **1 ahead / 147 behind** | `b1e5b723c` |
| `feat/pq-crypto-tier1` | 479 ahead / 23 behind | **23 ahead / 479 behind** | `dec8689e4c` (2026-07-12) |
| `feat/pq-crypto-tier1-snapshot-2026-07-18` | 1069 ahead / 613 behind | **613 ahead / 1069 behind** | `129f73a42` (early-June — *same base as the archived `kalman-organ`*) |

**Trap avoided (local != origin for the pq pair):** in this worktree the *local* checkout
`feat/pq-crypto-tier1` points at `884a79b2de` — which is the **snapshot's** SHA, **not** the real
`origin/feat/pq-crypto-tier1` (`60309b653a`). The local `pq-crypto-tier1` and
`origin/pq-crypto-tier1-snapshot-2026-07-18` are the *same commit*; the live tier1 branch is a
different SHA. All work below was done against **`origin/` refs explicitly** to avoid merging the
stale snapshot by mistake. (p47 and p06 had `local == origin`, verified.)

---

## 1. `feat/p47-payment-rail` (money) — MERGED CLEAN

- **Unique work:** one commit `0e7b98cf8 backup(p47): uncommitted payment_capability.rs change`.
  Diff = **`kernel/src/ports/payment_capability.rs` only, 10 ins / 10 del** — a purely cosmetic
  doc/comment reword: "client" → "transport" / "provider" / "object" in the module doc and in the
  trailing comments of the `FORBIDDEN_MARKERS` test.
- **Money-safety check (re-read of the merged file):**
  - No float money, no arithmetic *at all* — the module is a pure capability enum
    (`PaymentRail { Fiat, Crypto, Stripe, GoogleApplePay, OtherLater }`) + a feature-flag record
    (`PaymentCapability { rail, enabled }`) + `validate()`. The only `unwrap()`s are test
    rail-name parses (`"STRIPE".parse::<PaymentRail>().unwrap()`), not money math.
  - The RED-LINE is *preserved*: the actual banned strings in `FORBIDDEN_MARKERS`
    (`req`+`west`, `ur`+`eq`, `std::env`, `secret`, `key`, …) are **unchanged** — only their
    trailing comments were reworded. The red-line self-scan test (`red_line violation`,
    `capability declaration`, `ENABLED`) is intact. The module still constructs no client,
    reads no credentials, makes no network call, moves no money.
- **§16–§18 / P60 overlap check (required by the brief):** **complementary, not superseded.**
  `BLUEPRINT-P60-payment-adapter-core.md` (a same-day *planning* doc that writes no product code)
  **explicitly cites `payment_capability.rs` as the gate its adapter sits behind** and states
  verbatim: *"this is the feature-flag that GATES P60's adapter; P60 cites it, never redefines
  it"* (P60 §0 table) and *"`payment_capability.rs`'s `validate()` is the gate; P60 does not
  [redefine it]"* (§ non-goals). P60 adds a **sibling** `payment_provider.rs` port behind this
  gate. p47's code is the *foundation* P60 builds on; the comment-reword touches none of the API
  surface (enum / `validate()` / red-line test identifiers) P60 cites. No conflict, no
  supersession.
- **Merge commit: `0f8ef832a`** (`--no-ff`, zero conflicts, `ort` strategy, one file 10/10).

---

## 2. `feat/p06-v1-real-signer` (cryptographic signing) — **NOT MERGED. BLOCKER: human review required.**

This is the one the brief flagged for "stop, do not commit, report as a blocker" if the merge
itself could create a vulnerability. It can. **The trial merge was run `--no-commit` and then
`--merge --abort`ed; nothing was committed.**

### What the branch actually is
- **Unique work:** one commit `d250025790 fix(ci-truth,P06): complete HybridSigner compile
  (sig fields + verify_message + gate arity)` — `tools/ci-truth/src/{main.rs,v1.rs}`, +610/−69.
- **The decisive structural fact:** the branch's commit and `main`'s current P06 close are **two
  parallel children of the same base** `b1e5b723c`, developed by concurrent same-author sessions
  on the same day, and **the branch does NOT contain `main`'s close**
  (`git merge-base --is-ancestor 58987d79d <branch>` → false):

  | Commit | When | Design | Status |
  |---|---|---|---|
  | base `b1e5b723c` | 07-18 00:31 | HybridSigner slot + acceptance tests | common parent |
  | `main`'s `58987d79d` | 07-18 **10:29** | sig as a **parsed struct field** (`attest.sig`, `verdict.sig`; `signing_bytes()` excludes it) | **canonical, e2e GREEN, memory-recorded CLOSED** |
  | branch `d250025790` | 07-18 **14:30** | sig **TLV-embedded in raw note bytes** (`SIG_TAG_K=0x07` / `SIG_TAG_V=0x08`, split via `split_note_sig`) + `verify_message`, `v1-probe`, telemetry, `measure()` | parallel fork, never reconciled back to `main` |

### Why merging is a real red-line risk (not routine)
- The trial merge auto-merged `main.rs` but produced **19 conflict regions across `v1.rs`** (a
  1733-line red-line crypto file), including ~96- and ~115-line blocks. These are **two
  incompatible rewrites of the entire sign/verify path**: same field renamed (`sig` vs `sig_k`),
  different encode semantics (unconditional `tlv_put(0x07,…)` vs conditional
  `if !sig_k.is_empty()`), and a whole parallel `split_note_sig`/`verify_message` verification
  path vs `main`'s parsed-struct-field path.
- **Both sides are individually correct** on the two invariants I checked:
  - *Hybrid AND-semantics preserved on both* — each delegates `RequireBoth`
    (Ed25519⊕ML-DSA-65) to the external `bebop2-kv verify` CLI, and each is **fail-closed** via
    short-circuit early-return: `GateVerdict::Green` is reached **only if both** the key_K
    DiffAttestation **and** key_V Verdict signatures verify. Neither collapses to OR-semantics.
    (The branch even *adds* a cross-role hardening: the anchor line must `ends_with("role=K")` /
    `role=V`, blocking cross-role attestation.)
  - Both agree at the *wire* level (TLV `0x07`=K sig, `0x08`=V sig, `signing_bytes` = TLV
    `0x01..0x06` excluding the sig).
- **But a textual splice of two incompatible sig-representations across 19 crypto conflict
  regions is exactly the class of merge that can silently break verification** — e.g. computing
  `signing_bytes()` `main`'s way while splitting the sig the branch's way, so the signature
  commits to different bytes than the verifier re-derives. That is a vulnerability *created by
  the merge*, not present in either side. Per the brief's red-line rule, I **stopped**.

### Why this is a canonicalization decision, not a conflict resolution
- `main`'s `58987d79d` is the version that is **e2e GREEN and memory-recorded CLOSED**
  ("P06 key_V HybridSigner CLOSED 2026-07-18 (commit `58987d79d`)").
- `BLUEPRINT-P59-capability-cert-chain.md` — the same-day blueprint the brief asked me to check
  for overlap — **explicitly names p06's `HybridSigner`** at `tools/ci-truth/src/v1.rs:11-12` and
  pins it to **`main`'s `58987d79d`**: *"A dev-tooling CI adapter, out of the kernel graph. This
  is the P06 `key_V` closure (memory: commit `58987d79d`, 2026-07-18)."* So the *live blueprint
  program already treats `main`'s struct-field version as the canonical P06 closure* — merging
  the branch's alternative TLV design would actively contradict P59's stated citation.
- **P59 / p06 overlap verdict: they overlap only in name and shared crypto doctrine, not in
  surface.** p06 is the **CI merge-gate truth-attestation signer** (dev tooling in
  `tools/ci-truth`, shells `bebop2-kv`). P59 builds on the **kernel runtime** identity substrate
  — `kernel/src/ports/agent/cap.rs` (`SignatureVerifier`, `HybridPolicy::RequireBoth` = "the ONLY
  policy; there is no weaker code point", `verify_chain`, `Delegation`) + `pq/dsa.rs` +
  `pq/root_delegation.rs`. Different files; P59 uses the P06 closure only as a reference point.

### Recommended disposition (operator's / human reviewer's call)
1. **Do not machine-merge.** Treat `main`'s `58987d79d` as the canonical P06 key_V gate. The
   branch's `d250025790` is a **superseded parallel design** of the same feature.
2. **Carve-outs worth a human re-deriving onto `main`'s struct-field design** (each is a genuine
   improvement `main`'s version lacks; each touches the signed-bytes path so must be done by
   hand, not spliced):
   - `v1-probe` subcommand — signs+verifies a known payload via `bebop2-kv` and proves a 1-bit
     corruption is rejected (a real self-test).
   - the local telemetry sink (`V1_TELEMETRY = docs/ledger/v1-sigverify-telemetry.jsonl`,
     `record_telemetry`, `measure()`) — mandatory-telemetry-doctrine nicety, never logs key
     material.
   - the cross-role anchor check (`anchor ends_with role=K` / `role=V`) — extra "no cross-role
     attestation" hardening.
3. Keep the pushed ref `origin/feat/p06-v1-real-signer` as the record of the alternative design.

---

## 3. `feat/pq-crypto-tier1` (post-quantum crypto) — **NOT MERGED. Recommendation: archive** (kalman-organ pattern)

- **Unique work:** 23 commits (`669c77953` … `60309b653`) — WAVE1 PQ crypto tier-1 (ML-DSA/KEM
  codesign+dsa+kem+hybrid+x25519+volume+entropy + ACVP KAT), a DTN L2 node, S1 bp7-rs (RFC 9171)
  codec, S2+S3 QUIC/TLS1.3 bearer, S4 e2e sim, and a **`server/` → `node/` rename**. Diff vs the
  July-12 base: 55 files, +10 261/−2 189.
- **Investigation verdict — its fresh work already landed on `main`, independently:**
  - `kernel/src/pq/kem.rs`, `volume.rs`, `x25519.rs`, `entropy.rs` are **byte-identical to
    `main`** (same blob SHAs). `main`'s pq tree is a **superset**: it additionally carries
    `codesign.rs`, `envelope.rs`, `fractal.rs`, `hybrid.rs`, `keccak.rs`, **`root_delegation.rs`
    (which the branch lacks)**, `dsa/dsa_acvp_tests.rs`, and the ACVP KAT corpus — "107 pq-KAT
    tests" per memory. `dsa.rs`/`mod.rs` differ, with `main`'s the more-evolved versions.
  - None of the branch's 23 commits are ancestors of `main` (the identical files arrived via a
    different merge path / re-derivation).
- **Money-safety of the branch's red-line `money.rs` change — superseded by a *safer* `main`
  version, not a regression:** the branch removes two guards in `to_minor_unit` /`apply_tax`
  (`amount != amount`, `subtotal % 1 != 0`). Both are **provably dead on `i64`** (a NaN-check and
  an integer-mod-1, always false) — vestigial from the TS→Rust float-era port; removing them is
  correct. **But `main` already did this same removal *and went further*** (BP-17 comment:
  "the two dead guards … dead on `i64`") — `main`'s `money.rs` now has `checked_add` / `checked_neg`
  / `i64::try_from` range-checking the branch never had. **Merging risks *regressing* `main`'s
  checked-arithmetic hardening if a conflict were resolved toward the branch.**
- **Trial merge (aborted, nothing committed):** 13 conflicts — `.gitignore`, `kernel/Cargo.{toml,lock}`,
  `kernel/benches/*` (add/add), `domain.rs`, `lib.rs`, `money.rs`, `pq/dsa.rs` + `pq/mod.rs`
  (add/add), and a `server/Cargo.lock → node/Cargo.lock` **rename/delete vs `main`'s deletion of
  `server/`**. Every kernel/crypto/money conflict resolves to "take `main`" (more-evolved side).
  **Merging adds no value and carries the money-hardening-regression risk above.**
- **Only genuinely-unique surface:** the **12-file `node/` crate** (server→node rename + DTN
  store-and-forward, QUIC/TLS1.3 bearer, bp7-rs BPv7 codec, ActivityPub/MCP/Nostr adapters,
  roles/sim/store). `main` has **no `node/`** — it retired `server/` entirely. This work is built
  against the **July-12 architecture**; whether a standalone `node/` DTN crate should be
  re-derived onto the current mesh-hub architecture (P34 / P67–P68) is an **architecture decision
  for the operator**, not a mechanical merge.
- **Recommended disposition:** **archive** (identical rationale to the prior pass's `kalman-organ`
  call — fresh work already on `main`, strict-subset stale kernel, unique surfaces built against a
  retired architecture). Keep `origin/feat/pq-crypto-tier1` pushed as the record; optionally retag
  `archive/pq-crypto-tier1-2026-07-18`. Flag the `node/` DTN/QUIC/adapters crate as the sole
  re-derivation carve-out. Do **not** delete; do **not** reconcile.

---

## 4. `feat/pq-crypto-tier1-snapshot-2026-07-18` — **investigate-only (per brief). Recommendation: archive** (kalman-organ class)

- **It is NOT an identical twin of the live tier1 branch** (the brief warned about this). It
  shares only the **early-June merge-base `129f73a42`** with `main` — the **exact same base as the
  already-archived `kalman-organ`** — and is **fully diverged** from the live tier1 (613 vs 613
  commits either side of that early-June ancestor).
- **Profile = kalman-organ class:** **2 637 files changed, +575 002 / −16 798** vs `main`;
  **1 209 junk-tree files** (`src/screens/*` TS mocks, `e2e/`, `attic/`, `packages/`,
  `eslint-plugin*`, and — critically — `.claude/` + `.agents/` governance trees that must never
  be bulk-merged from a feature branch). It is **1 069 behind `main`** — *staler than
  `kalman-organ`* (1 057 behind).
- Its commit **subjects mirror the live tier1's** (same "WAVE1 PQ crypto tier-1", "S2+S3
  QUIC/TLS1.3", "s1 bp7-rs", "s4 e2e sim", "P1/P2/P5 merges") but with **entirely different SHAs**
  (`c260eb4be` vs `da99e4e48`, etc.) — it is a **parallel rewrite of the same PQ work on the old
  early-June base**, whereas the live tier1 rebuilt that work on the fresher July-12 base.
- **There is nothing in the snapshot the live tier1 lacks in a fresher form** — it is strictly
  *worse* (same PQ work, older base, 1 209 junk files, missing `root_delegation.rs`). Everything
  of value either already landed on `main` byte-identically or is superseded there.
- **Recommended disposition:** **archive**, same as `kalman-organ`. Keep the pushed ref
  `origin/feat/pq-crypto-tier1-snapshot-2026-07-18` as the archival record of the early-June
  alternative history; optionally retag `archive/pq-crypto-tier1-snapshot-2026-07-18`. Do **not**
  merge, do **not** delete, do **not** attempt reconciliation.

---

## 5. Summary table

| Branch | Divergence (live) | Disposition | Result |
|---|---|---|---|
| `feat/p47-payment-rail` (money) | 1 ahead / 83 behind | **MERGED** `--no-ff` | commit `0f8ef832a`; money-safety verified; complementary to P60 |
| `feat/p06-v1-real-signer` (signing) | 1 ahead / 147 behind | **NOT MERGED — BLOCKER** | 19-region crypto conflict; two incompatible parallel P06 designs; `main`'s `58987d79d` is canonical (cited by P59); carve-outs flagged for human re-derivation |
| `feat/pq-crypto-tier1` (PQ crypto) | 23 ahead / 479 behind | **NOT MERGED — archive** | PQ work already on `main` (4 modules byte-identical, rest superseded); `money.rs` change superseded by `main`'s safer checked-arith; unique `node/` DTN crate = carve-out |
| `feat/pq-crypto-tier1-snapshot-2026-07-18` | 613 ahead / 1069 behind | **NOT MERGED — archive** | kalman-organ-class stale fork (early-June base, 2 637 files, 1 209 junk); strictly worse than live tier1 |

## 6. Not done here (deliberately)
- No merge of `fix/reconcile-redline-branches-2026-07-18` into `main` — **operator gate**, same as
  the prior pass.
- No merge of the p06 crypto branch — **blocked on human review** of a red-line
  canonicalization decision (which sig-representation design is canonical).
- No merge of either pq branch — archive recommendations.
- No deletion, retag, or force-push of any investigated branch.
- The only code change committed on this branch is the p47 cosmetic comment-reword merge
  (documentation-grade; no money arithmetic, no crypto logic).
