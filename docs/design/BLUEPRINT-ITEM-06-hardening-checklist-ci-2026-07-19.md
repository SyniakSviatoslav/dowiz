# BLUEPRINT — Item 6: §4 Hardening Checklist Codified + CI Enforcement (with §10/P7 built in)

- **Date:** 2026-07-19 · **Tier:** 2 (roadmap §C) · **Status:** BLUEPRINT (planning artifact, no code)
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §C Item 6;
  `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §1.6, §4, §9.6, §10 (P5/P6/P7);
  worktree `/root/dowiz-wt-space-grade-exec` @ `6605166cd` (branch `exec/space-grade-tier0-2026-07-19`)
  — ground truth for all citations below (main has not absorbed all of it).
- **Downstream dependents:** item 7 (Kani — upgrades checklist item 4 to deterministic) and item 8
  (GCRA decision package — the gate's first full end-to-end customer). This blueprint sets the
  standard both build on.

---

## 1. The checklist, exactly as §4 states it

§4's mandate: any new or changed **algorithmic hot path (crypto or not: graph algorithms, scheduler
math, GCRA arithmetic all qualify)** must ship with:

> 1. **An oracle** — exhaustive where the input space permits (the FSM's 12 states permit it), or a
>    large randomized corpus differentially checked against a simple reference implementation (the
>    NTT's schoolbook, the token bucket's mutex version), with the reference retained forever as a
>    test-only crate-internal module.
> 2. **A dudect-style gate** where secret-dependent timing is conceivable — including a planted-leak
>    self-test proving the gate itself works, per today's `ntt_ct_gate` standard. (CI-time harness,
>    not linked.)
> 3. **Debug-mode differential cross-check** (`debug_assert_eq!` against the oracle) compiled out of
>    release — continuous verification at zero production cost, per today's `ring_mul` standard.
> 4. **A binary/assembly spot-check on every compiler version bump** for the branch-free paths —
>    grounded in the arXiv:2410.13489 incident class. PMU counters remain optional/experimental,
>    honestly labeled as the least-precedented element.

Deliverable per §4: "one markdown checklist plus a CI job that fails when a diff touches a
designated hot path without the corresponding artifacts." Proof per §9.6: "a diff touching a
designated hot path without oracle/dudect/differential artifacts fails CI; the NTT work passes
retroactively without modification." (The NTT precedent lives in bebop — `bebop2/core/src/pq_kem.rs`,
commit `cf1fc90` on `openbebop/main`, per §1.6. Item 6's job is the **dowiz kernel**, where the
retroactive-pass clause maps onto the surfaces inventoried in §3 below.)

**The §10/P7 correction — the load-bearing design constraint:**

> §4's CI enforcement as drafted fails a diff that lacks "the corresponding artifacts" —
> presence-checking is self-certifiable (an empty oracle file passes). The receptive half must
> **re-execute**: CI runs the oracle, runs the dudect gate including its self-test. Otherwise the
> checklist reproduces the hermetic doc's Gender V-1 failure (the gate consumes author-supplied
> evidence) one layer up.

Two sibling corrections also bind this item: **P5(b)** (compiler-bump detection must be structural,
not human-noticed — already landed as `toolchain-bump-gate`, see §3.7) and **P6** (the gate itself
must be deterministic: `--locked --offline`, no registry/network dependence).

---

## 2. §10/P7 made mechanical — what "re-execute, never presence-check" means here

1. **No file is evidence.** The CI job never decides GREEN by reading a report/artifact file
   (`test -f dudect_report.txt` is banned by construction). Every verdict comes from a live process
   exit code plus parsed live test counts, in the CI run itself.
2. **Named-filter re-execution with a minimum-count assertion.** `cargo test <filter>` with a filter
   matching **zero** tests exits 0 — bare exit-code checking is a presence-check one level down
   (delete or rename the oracle test and the gate stays green). Every manifest row therefore carries
   `min_tests`, and the job parses cargo's `N passed` and asserts `N >= min_tests`. This is the
   anti-forgery core.
3. **Self-tests run in the same invocation.** The dudect gate (once it exists, §5) runs *including*
   its planted-leak self-test: a deliberately leaky comparator must be rejected by the same Welch-t
   machinery in the same CI step, or the whole step is RED. Per P7, the verifier proves it can
   reject before its acceptance means anything.
4. **The gate itself has a proven RED path** (P7 applied one more layer up, matching the
   `toolchain-bump-gate` precedent whose bite was demonstrated with a simulated
   bump-without-artifact → exit 1): before first merge, the executor must demonstrate (a) a
   synthetic diff touching a designated hot path with no manifest row → exit 1, and (b) a manifest
   row whose filter matches zero tests → exit 1.
5. **One sanctioned exception, named honestly:** `toolchain-bump-gate` (ci.yml:414–449) checks for
   the presence + required headings of `docs/audits/toolchain/spot-check-<ver>.md`. That artifact
   is a *human judgment record* (an assembly audit cannot be re-executed by grep); the presence
   check is acceptable there and only there. Item 7 (Kani) is what eventually upgrades checklist
   item 4 from "human artifact accompanies the bump" to "deterministic check re-executes."

---

## 3. What already exists in the dowiz kernel — re-executing today, with citations

All of the following are `#[test]`s that run under plain `cargo test`, and CI already re-executes
them **unconditionally** via the `cargo-test` job (`.github/workflows/ci.yml:128–144`: per-crate
`cd kernel && cargo test --offline`, real exit code, no path filter — its header at ci.yml:118–127
states the planted-`assert!(false)`-goes-RED property explicitly). So for everything below, the P7
re-execution property already holds; what is missing is the *diff-triggered obligation* and the
*named-filter min-count* layer (§4 of this doc).

### 3.1 Checklist item 1 (oracle) — substantially present

| Surface | Oracle that exists | Citation (worktree) |
|---|---|---|
| ML-DSA-65 (sign/verify/keygen) | Official NIST ACVP FIPS-204 vectors, byte-exact, **one discrete `#[test]` per tcId** (keyGen/sigGen/sigVer), plus count-pin tests that fail if vendored vectors shrink | `kernel/src/pq/dsa.rs:1008–1013` (module decl); `kernel/src/pq/dsa/dsa_acvp_tests.rs:209,238,268` (generated per-tcId tests), `:299–316` (`acvp_mldsa65_{keygen,siggen,sigver}_count`); vectors `kernel/src/pq/kat/acvp/{key-gen,sig-gen,sig-ver}.json` |
| ML-DSA-65 differential probe | `mldsa_diff_probe` — emits FIPS-204 intermediates for byte-level diffing vs pq-crystals reference (G10) | `kernel/src/pq/dsa.rs:1086–1110` |
| Keccak-f[1600] copy A | FIPS-202 KATs: `kat_shake256_empty`, `kat_shake128_empty`, `kat_shake256_abc`, `kem_debug_sha3_kat` | `kernel/src/pq/keccak.rs:223–269` |
| Keccak-f[1600] copy B (`event_log.rs`) | `sha3_256_empty_known_answer`, `sha3_256_abc_distinct` | `kernel/src/event_log.rs:609–628` (permutation at `:67`) |
| x25519 | RFC 7748 §6.1 KATs `kat_x25519_vector1/2` + `kat_x25519_associative` | `kernel/src/pq/x25519.rs:37,46,55` |
| ML-KEM-768 | Self-consistency, two-seed-FIPS keygen, tamper red-gate, random-seed soak — **but not official ACVP vectors** (deferred, see §5.2) | `kernel/src/pq/kem.rs:490–586` (`kem_two_seed_keygen_matches_fips:532`, `kem_self_consistency:554`, `kem_tamper_red_gate:565`, `kem_soak_random_seeds:579`); deferral self-documented at `kem.rs:5` |
| Hybrid KEM | Roundtrip + tamper + wrong-peer + no-classical-fallback red/green set | `kernel/src/pq/hybrid.rs:131–165` |
| Order FSM (12 states) | **The exhaustive case §4 names**: `FSM_GOLDEN_SIGNATURE` drift gate (`verify_fsm_signature`), live-vs-golden test, red drift tests, independent `spectral_radius_oracle`; 25 `#[test]`s total | `kernel/src/order_machine.rs:502` (const), `:532` (verify fn), `:932` (`green_live_signature_matches_golden`), `:944` (`red_divergent_report_reports_drift_field`), `:959` (`spectral_radius_oracle`) |
| Eigen surface | Bit-capture oracle (`eig2x2_bit_capture_oracle`) + parity vs Faddeev (`r3_topk_symmetric_parity_p3`) | `kernel/src/householder.rs:1008–1065`; `kernel/src/spectral.rs:1331` |
| Token bucket | 3 behavioral tests incl. the F33 never-over-grants falsifier — **no differential oracle** (see §5.4) | `kernel/src/token_bucket.rs:128–160` |

### 3.2 Checklist item 2 (dudect gate) — **absent entirely**

`grep -ri "dudect\|ct_gate\|planted" kernel/src/` → zero hits. The `ntt_ct_gate` standard exists
only in bebop. No timing gate of any kind exists in the dowiz kernel. (`leak_gate.rs` is a semantic
cosine-similarity gate — unrelated to timing.)

### 3.3 Checklist item 3 (debug-mode `debug_assert_eq!` cross-check) — **absent entirely**

`grep -rn "debug_assert" kernel/src/pq/ kernel/src/token_bucket.rs kernel/src/order_machine.rs` →
zero hits. The `ring_mul` pattern (release runs fast path, every debug/test run oracle-checks) has
not been ported to any dowiz hot path.

### 3.4 Checklist item 4 (binary/assembly on compiler bump) — structurally enforced, content partial

- **Enforcement (P5(b) fix) — landed as item 14:** `toolchain-bump-gate`
  (`.github/workflows/ci.yml:414–449`) derives "bump happened" from `rust-toolchain.toml` changing
  and fails the bump diff unless `docs/audits/toolchain/spot-check-<new>.md` with required headings
  accompanies it. Its RED path was demonstrated (simulated `1.96.1→1.96.2` without artifact → exit 1).
- **Baseline artifact:** `docs/audits/toolchain/spot-check-1.96.1.md` (worktree). Its own verdict:
  **"Partial / method-demonstrated, not exhaustive"** — source-level constant-time review real and
  performed across all six secret-dependent surfaces; one symbol disassembled as method demo; the
  exhaustive per-branch taint proof **explicitly deferred to Tier 2 item 7 (Kani)**. Items 6+7 are
  the follow-through on that recorded deferral. It also put two real findings on the ledger — see §5.3.

### 3.5–3.7 Supporting gates already live (context, not scope)

- `cargo-test` unconditional re-execution floor — ci.yml:128–144 (the P7 backbone).
- `zero-dep-gate` — ci.yml:271–281 + `scripts/zero-dep-gate.sh` (items 1+13; GREEN at 0 external
  crates since item 5 landed this session).
- `bench-regression` fail-closed A/B gate — ci.yml:160–178. `v5c-reexec` independent re-execution —
  ci.yml:88–116.

---

## 4. CI job design — `hardening-gate`

### 4.1 Deliverables (three files)

1. **`docs/audits/hardening/CHECKLIST.md`** — the §1 checklist verbatim as standing law, plus the
   designation rule ("algorithmic hot path, crypto or not") and the P7 clause. One page.
2. **`docs/audits/hardening/HOT-PATHS.tsv`** — the machine-read manifest. One row per designated
   hot path: `path-glob · checklist-coverage(1..4) · cargo-test filter(s) · min_tests · gap-owner`.
   Missing coverage is **ledgered in the row** (`MISSING(P91.2)`, `MISSING(item-7)`,
   `MISSING(item-8)`, `KNOWN-RED(P91.2)`) — gaps are visible in the gate's own input, never hidden.
3. **`.github/workflows/ci.yml` — new job `hardening-gate`** (modeled on `toolchain-bump-gate` for
   diff detection, `cargo-test` for execution discipline).

### 4.2 Job steps

```
hardening-gate:
  A. Diff scope     : git diff --name-only $BASE...HEAD  ∩  HOT-PATHS.tsv path-globs
                      ($BASE = merge-base with main / before-SHA, per toolchain-bump-gate ci.yml:421+)
  B. Row check      : every touched hot path MUST have a manifest row → missing row = exit 1
  C. Re-execute     : for each touched row:
                        cd kernel && cargo test --offline --locked <filter>
                      parse "N passed"; assert N >= min_tests   (zero-match forgery = RED)
  D. Oracle floor   : UNCONDITIONALLY (every run, even with no hot-path diff) re-execute the
                      manifest's named oracle filters with the same min-count assertion —
                      deleting/renaming an oracle test goes RED on the next run, any diff
  E. dudect step    : once §5.1 lands: cargo test --offline --locked --release <ct_gate filter>
                      — runs the gate INCLUDING its planted-leak self-test in the same step
  F. Gap ledger     : rows marked MISSING/KNOWN-RED are printed as ::warning:: with owner —
                      visible every run, never silently green
```

Determinism (P6): `--locked --offline` on every cargo invocation; the job asserts the `Cargo.lock`
hash is unchanged after the run (same discipline the zero-dep gate adopted per §10/P6).

### 4.3 Initial hot-path designation (manifest seed — real filters, from §3.1)

| Path | Filters (real test names) | min_tests |
|---|---|---|
| `kernel/src/pq/dsa.rs`, `kernel/src/pq/dsa/`, `kernel/src/pq/kat/` | `acvp_` (+ the 3 `_count` pins) | count pins already assert vector counts; job asserts ≥ 3 count tests + ≥ 1 per-tcId batch |
| `kernel/src/pq/keccak.rs` | `kat_shake256_empty kat_shake128_empty kat_shake256_abc kem_debug_sha3_kat` | 4 |
| `kernel/src/event_log.rs` | `sha3_256_empty_known_answer sha3_256_abc_distinct` (+ hash-chain tests) | 2 |
| `kernel/src/pq/x25519.rs` | `kat_x25519_` | 3 |
| `kernel/src/pq/kem.rs` | `kem_two_seed_keygen_matches_fips kem_self_consistency kem_tamper_red_gate kem_soak_random_seeds` | 4 · plus row `MISSING(P91.2): ACVP vectors` |
| `kernel/src/pq/hybrid.rs` | `green_roundtrip_both_legs red_tampered_kem_ct_rejected red_wrong_peer_rejected red_no_classical_fallback` | 4 · plus `KNOWN-RED(P91.2)` for the tag compare (§5.3) |
| `kernel/src/order_machine.rs` | `order_machine::` module filter | 25 (exhaustive-oracle clause satisfied) |
| `kernel/src/householder.rs`, `kernel/src/spectral.rs` | `eig2x2_bit_capture_oracle r3_topk_symmetric_parity_p3` (+ parity set) | 2+ |
| `kernel/src/token_bucket.rs` | `token_bucket_` | 3 · plus `MISSING(item-8): GCRA differential oracle` |
| `kernel/src/retrieval/pattern.rs` (item 5's hand-rolled matcher — "crypto or not" clause) | pattern/matcher test set (executor enumerates from `retrieval/tests.rs`) | ≥ its current count |
| `kernel/src/fdr/json.rs` (item 4's escaping surface, §1.2-class) | fdr json escaping tests | ≥ its current count |

Retroactive-pass check (§9.6 analog): the gate applied to the current tree must go GREEN on every
row except the ledgered MISSING/KNOWN-RED warnings — no existing surface should need modification
to pass, mirroring "the NTT work passes retroactively without modification."

---

## 5. Honest gap list — real scope for the Opus executor (build, or defer with reason)

1. **dudect-style harness for the dowiz kernel — does not exist (checklist item 2, §3.2).**
   RECOMMEND BUILD in item 6: a minimal zero-dep Welch-t harness as a test-only kernel module
   (bebop's `ntt_ct_gate` is the pattern precedent — |t| < 4.5, planted-leak self-test), wired
   first on the expected-GREEN surfaces (Keccak permutation, DSA NTT/reduction paths). Without it,
   the §10/P7 correction has no teeth for checklist item 2. If deferred instead, the manifest rows
   stay `MISSING` and the reason must be recorded here.
2. **ML-KEM-768 ACVP official-vector KATs — deferred, self-documented** (`kem.rs:5`: "P91.2 ACVP
   KAT + 3-model review gate is DEFERRED"). Owner: P91.2, not item 6. Manifest row carries it.
3. **Two known variable-time `!=` compares** — `kem.rs:~468` (FO implicit-rejection tag compare)
   and `hybrid.rs:~101` (confirm-tag compare), both found and ledgered by the item-14 audit
   (`spot-check-1.96.1.md`), both pre-existing and compiler-independent, both owed to P91.2.
   A dudect gate over these surfaces would honestly go RED today. **Do NOT silently fix crypto
   inside item 6** — designate the rows `KNOWN-RED(P91.2)`; the constant-time fix then becomes the
   gate's first customer, passing through the very checklist it triggered.
4. **GCRA differential oracle (atomic vs mutex) + interleaving check — not present**
   (`token_bucket.rs` has 3 behavioral tests only). Owner: item 8 (ruling: ADOPT). Item 6's job is
   solely to designate `token_bucket.rs` a hot path now, so item 8's swap cannot merge without the
   oracle. Note one honest wrinkle for checklist item 1's "reference retained forever" clause: the
   mutex implementation must survive as a test-only module after the swap (§10/P2's parity-pin).
5. **Debug-mode `debug_assert_eq!` differential cross-checks — none exist (checklist item 3,
   §3.3).** Cheap to add where a per-call oracle exists (e.g. FSM transition vs table, future
   GCRA-vs-mutex). For ACVP-style corpus oracles there is no per-call reference to assert against —
   the executor should apply item 3 only where a callable reference exists, and record
   `N/A(corpus-oracle)` in the manifest otherwise, not fake it.
6. **Exhaustive assembly/taint audit — deferred to item 7 (Kani)**, per the spot-check's own
   honesty preface. Item 6 encodes the deferral in the manifest (`MISSING(item-7)` on checklist
   item 4's exhaustive form); the structural bump-trigger half is already live (§3.4).
7. **A regex-reference footnote for item 5's matcher:** the original differential reference (the
   `regex` crate) was removed to reach zero-dep — the "reference retained forever" clause is
   satisfied there by the vendored test corpus, not a live reference implementation. Record this
   in the manifest row rather than pretending the clause is met in its strong form.
