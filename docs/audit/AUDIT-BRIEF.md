# Audit brief: the bebop DOC file, its seals and its readers

For an external cryptography and security auditor, day one. Written 2026-11-01 from reading the tree; **nothing was compiled or run to write it**.

- **Commit under audit:** `<COMMIT: maintainer fills this in; the text was read from the unmerged working tree of branch mg-batch7>`. Full paths are relative to the repository root; a short name such as `doc/count.rs` or `dsa.rs` is the file of that name in the crate the same row or section names (§1, §9).
- **Evidence rule.** Each claim names the file or test it comes from. A mark **[lane record]** means the number comes from a working note that is not in the repository: treat it as a pointer and reproduce it. A test named here exists; that it passes at your commit is what §11 is for.
- **Normative text:** `docs/FORMAT-BEBOP-IMAGE-v2.en.md` (below "FORMAT"), which rules over the code (FORMAT header, "Authority"). Status vocabulary: `docs/STATUS-CODES.md`. Reporting channel: `SECURITY.md`.

## 1. Scope

**In scope**

| Area | Where |
|---|---|
| The format: container, DOC, SEAL, KEY, SEG, ANCH, SPEC | FORMAT §1-§14 |
| The verifier: counters, `holds`, admission, revocation, declarations, budgets | `crates/bebop-log/src/doc/` (`verify`, `count`, `seals`, `settle`, `declare`, `budget`, `frontier`, `seg`, `forgotten`, `merge`, `cut`) |
| Container and KV readers | `crates/bebop-image`, `crates/bebop-kv`, `crates/bebop-log/src/lib.rs` |
| The hub's filling of the seams: ids, signer, verifier, sealed writes | `crates/dowiz-hub/src/doc_seal.rs`, `doc_seal/`, `chain/scheme.rs` |
| The signature and hash primitives the seal rests on | `crates/dowiz-core/src/pq/dsa.rs`, `pq/keccak.rs`, the sha256 code of §9 |
| The auditor's tool | `crates/dowiz-hub/src/bin/doc_verify.rs` |
| Readers that carry no signature code | `tools/verify/py/*.py` (assembled to `bebop-verify.py`), `crates/bebop-wasm-reader`, `crates/bebop-wasm/oracle.py` |
| The self-reading archive and its host | FORMAT §9.5, `examples/read.mjs`, `tools/verify/bebop-verify.mjs` |
| Level 1 anchors | `crates/dowiz-hub/src/doc_seal/{ots,tsa,roughtime}.rs`, FORMAT §9.4 ANCH |

**Out of scope for this engagement unless agreed**

- The live service: Worker routes, tenant isolation, payments, tokens. `SECURITY.md` lists those controls.
- A public chain, witnesses or level 2 settlement: `docs/research/R-MAINGOAL-A.md` §4 calls it a specification, not code, until a second organisation exists.
- Traffic and storage confidentiality. Nothing in the file is encrypted by this design; `SECURITY.md` makes no such claim for live data.
- `crates/dowiz-core/src/pq/hybrid_signing.rs`, except as stated in §9: nothing on the seal path may use it.

**What is live.** Be exact about this. `SECURITY.md`: nothing on the live request path calls `pq/`; tokens are HMAC-SHA256 and transport is Cloudflare TLS; the backup seal is built and switched off until `BACKUP_SEAL_PK` is set. A search of `workers/` for `doc_seal`, `SealedHub` or `seal_hub` finds nothing, so the sealed file has no caller there. `tools/gates/maingoal.sh`: "no platform key exists in production and no Bitcoin attestation is involved". FORMAT header: nothing has been released, and the text is not frozen. So findings on the format can still change bytes at no migration cost.

## 2. Assets

| Asset | What it is | Protected by | Stated in |
|---|---|---|---|
| Record history of a device | `(actor, seq, prev, flags, payload)` under a sha256 id | id formula, ML-DSA-65 seal over the tip | FORMAT §9.4 "Record id" |
| Device identity | 32-byte `actor_id = sha256(pk)` | `actor_of` check; the verifier's own root list | FORMAT §9.4 |
| Device seed | 32 bytes from which the key derives | outside the file; custody is not specified | `doc_seal.rs:124` "the seed is what a device keeps" |
| Revocation state | REVOKE records; the table cell `revoked_at` is an index nobody signs | derivation from records, `keys_disagree` | FORMAT §9.4 "Revocation" |
| Erasure of a person | tombstone, nonce erased, FORGOTTEN with digests | `tombs_foreign`, `redacted + cut_redacted = declared` | FORMAT §9.4 |
| Cut history | SEG chain, TRUNCATE, `seg_hash` | root-only effective TRUNCATE, status 75 | FORMAT §9.4 |
| Reader's host | a stranger's CPU and memory, a terminal, the auditor's verdict | walk bound, heap bound, module outline, pin | FORMAT §9.0, §9.4, §9.5 |
| The verdict itself | the words a reader prints | `seals COUNTED, not CHECKED:` rule | FORMAT §9.4 |

## 3. Trust model

The sentence FORMAT calls its most important one is in §9.4. In short:

| | `Trust::Genesis` | `Trust::Only(ids)` |
|---|---|---|
| Starts from | keys the file calls genesis (`ref ADMIT_REC` reads as null) | exactly the actor ids the verifier brings from outside |
| Who controls the start | whoever edits the file: the table is unsigned | the verifier |
| An ok result means | "signed by the keys this file names as its roots" | signed by keys descending, through signed admission, from those ids |
| Evidence of who wrote the file | **none** | yes, for as far as §5 and §6 allow |
| Code | `verify.rs:157`, `:240` | `verify.rs:162`, `:241` |

Consequences that bind this brief:

- **Every statement of what a thief cannot do is a statement under `Trust::Only`** (FORMAT §9.4, §14). A forger writes his own key in as a root under `Trust::Genesis`. Test `only()` in `crates/dowiz-hub/tests/atk_holes.rs` runs that way.
- `Trust::Only` brings ids, not keys, so the verifier recomputes `actor_of(pk)` for every key, roots included (FORMAT §9.4). The KEY row is unsigned. Before ATK hole 1 this comparison did not exist.
- There are no roles: "ANY key valid at its own seq may revoke ANY key, itself and a root included", and a REVOKE is final (FORMAT §9.4).
- A TRUNCATE is effective only from a root of the fold's trust (`declare.rs:97`). A FORGOTTEN counts from the record's author or a root (FORMAT §9.4 "Who may erase whom").
- A **blind reader** (no signature code) has `checked = 0` and `holds` false whatever else it prints, and must say so in words. Python, wasm32 and the oracle are blind; `doc_verify` is the only reference tool that judges signatures (`crates/bebop-wasm-reader/src/ids.rs` header; FORMAT §9.0).
- The embedded wasm reader is testimony, not authority (FORMAT §9.5): it runs only under a host's pin.

## 4. Threat model

The adversary always holds the whole file and may rewrite every unsigned cell. Unsigned: HEADS, KEYS, SEG rows outside `seg_hash`, order across devices, `ref PREV_SEAL`, the ANCH chain, `revoked_at`, h0 of older objects (FORMAT §5, §9.4, §14).

| Adversary | Has | Should fail to | Where tested |
|---|---|---|---|
| **A0 Forger** | the file, no key of it | change, add, remove or reorder anything `holds` covers | `atk_holes`, `atk_caught`, `atk2_seq` |
| **A1 Thief of a non-root key** | one admitted key | hide a REVOKE, or cut | `crates/bebop-log/src/doc/settle/tests.rs`, `doc/tam_tests.rs` (`h1_*`) |
| **A2 Holder of a revoked key** | a key past its `after_seq` | sign past `after_seq`, back-date an ADMIT | `doc/tam_tests.rs` (`h2_*`), `settle/tests.rs` |
| **A3 Thief of a root** | a root key | nothing is promised: §14 "A stolen root" | pinned `limit_a_stolen_root_cuts_a_revoke_away` |
| **A4 Author of a hostile file** | any bytes | crash, hang, out-allocate or disagree among readers | `atk2_dos`, `atk5_dos`, `tools/gates/atk3*` |
| **A5 Author of a hostile archive** | any bytes, embedded wasm | run unpinned code, write to the terminal | `tools/gates/atk3.sh`, `atk3_bootstrap.py` |
| **A6 Holder of an older copy** | a true earlier file | nothing is promised in one file: §14 "Rollback" | pinned `limit_the_file_before_the_revoke_knows_nothing_of_it` |
| **A7 Honest offline devices** | no malice | lose a record or a signature, or hide a conflict | `atk5_*`, `m2_merge` |

**Assumptions.** ML-DSA-65 unforgeability for a key whose holder is honest, and collision resistance of sha256 over the records at issue, each stated as a hypothesis in `bebop-lang/formal/Bebop/DocLog.lean`, never as an axiom. A stolen key signs and nothing can tell.

**Not modelled or claimed.** Side channels (§9). A signature's `claimed_ms`: FORMAT §9.4 says an ok seal does not say the time is right, nor that the event happened or a quantity is true.

## 5. What the file promises

**An ok seal says** only: "the holder of key K signed this tip at seq s of its chain, and nothing under the tip has changed since" (FORMAT §9.4).

**`holds`** is the conjunction (`verify.rs:149`, FORMAT §9.4): `broken = 0`, `seals_bad = 0`, `forks = 0`, `seals_ok > 0`, `redacted + cut_redacted = declared`, `keys_disagree = 0`, `tombs_foreign = 0`, `unsealed = 0`, `merges_bad = 0`, `tree_unsigned = 0`. Under `Trust::Only`, that is: every record vouched for by an ok seal, every chain ending at `0^32` or a frontier tip, and no tree outside the signatures.

- **`holds` is not status 0.** A file can open and print status 0 with `holds` false. A caller that reads the status alone gets less, and a forged TRUNCATE can sit there (FORMAT §14). `tombs_foreign`, `tree_unsigned`, `revokes_unsettled` and `admitted` are not in the printed line; four terms of `holds` cannot be read off it (FORMAT §9.4 "What is not in the line").
- **Refusals.** Status 1, 2, 3, 4, 10, 12, 41, 75 for a read or `verify`; 76, 77, 78 for a host (FORMAT §9.0). A last-commit failure of bytes handed whole is a refusal, never a fallback (FORMAT §7).
- **Bounds.** One walk bound `n`, the cells that arrived (§9.0). One `verify` holds at most 8 × the file size of heap above the image, and time is O(n log n) plus settle rounds × admission passes × distinct seals (§9.4 "Bounds of one `verify`"). A reader must not pad, panic, or allocate from an unbounded value (§1, §3.1).
- **Erasure.** A tombstone erases the nonce, so a kept id does not confirm a guessed payload (§9.4, `atk4_nonce`).
- **Merge.** Loses no record and no signature, and refuses two different chains over the same records (`SealFork`) (§9.4).
- **Crash.** A crash between the three `fdatasync` steps leaves generation k readable (§8). See limit L18.
- **Host.** It computes the sha256, checks the module outline, requires the DOC's `ref SPEC` at cell 1024, and runs only the reader it pins (§9.5).

## 6. Every limit, by name

**One file cannot decide these** (FORMAT §14, as named there; its seventeenth entry, "Status 0 is not `holds`", is below the table, so these numbers stay):

| # | Limit | # | Limit |
|---|---|---|---|
| L1 | Trust | L9 | Legacy records |
| L2 | Rollback | L10 | A tombstone's bytes under a cut |
| L3 | Whole-tail removal | L11 | Time |
| L4 | A stolen root | L12 | MERGE strip |
| L5 | A blind reader | L13 | A conflict list under a cut (`doc_verify` exits 4, `NOT CHECKED`) |
| L6 | Removing an anchor | L14 | A MERGE row or a REVOKE under a cut (fixes the seq, not the tip) |
| L7 | Swapping files of one owner | L15 | What a MERGE saw |
| L8 | Order across devices | L16 | After a rotation |

Also in FORMAT §14: **Status 0 is not `holds`.** A forged TRUNCATE can sit at status 0 through a tombstone's unbound `prev`, with `holds` false. It is the model's trace L3t, and `tam_l3t_` in `crates/bebop-log/src/doc/tam_tests.rs` builds it.

L9 matters most for a first read: "Live data is all legacy, so a `seal_hub` of a real venue is exposed on its pre-seal records until those records leave the file", and FNV preimages are untested. L2 and L3 are theorems of the model (`DocLimits.rollback_holds`, `no_file_check_sees_rollback`).

**Further limits stated elsewhere in FORMAT:**

| # | Limit | Where |
|---|---|---|
| L17 | Whether superblock plus PartTab is one atomic sector is **not verified**; the only integrity argument is the crc | §2 |
| L18 | A kill-anywhere test for the Rust write path does not exist | §11 D4 |
| L19 | The crc covers the payload, not the header; h0 of an older object is protected only by the anchor over the last commit | §5 |
| L20 | A lazy open that stops at the anchor does not catch a corrupt object of an older commit, by design | §7 |
| L21 | `verify` reads no tree; a file naming a TREE does not hold | §9.4 `tree_unsigned` |
| L22 | A KEY object's crc is not read | §9.4 |
| L23 | Unsettled REVOKEs are not resolved: a seal goes bad and a person decides | §9.4 |
| L24 | Memo kind 4: a crc32 collision in a rewrite is **NOT COVERED** | §9.3 |
| L25 | Key and non-UTF-8 divergence: recorded, not changed | §11 D6 |
| L26 | Bounds not tested: `MAX_PAD_CELLS` at the bound, `MAX_VALUE_BYTES` at its real value, `MAX_WIDTH` at 64/65; images above 512 MiB through a file not measured | §10 |
| L27 | Three open reader questions: a v1-v3 TREE in a DOC, the name of status 2, cells 13-14 | §12 |
| L28 | The SPEC sha256 is a claim of the file; only a host binds it | §9.4, §9.5 |

## 7. Attack history

FORMAT §14: "No attack round that tested this format came back empty: each found defects, so the list above is what is known, not all there is." Read the table the same way. Tests under `crates/dowiz-hub/tests/` unless a path says otherwise. Counts of defects are from the test files; a figure marked [lane record] is not.

| Round | Attacked | Found | Closed by | Test files |
|---|---|---|---|---|
| **W-MG-ATK** | `verify` with no private key, `Trust::Only`, real ML-DSA-65 | Holes 1-9 with sub-cases: root pk unbound to its id; `flags` outside the id; tombstone content and identity unbound; `covered` vs `unsealed` over a device's first record; frontier rows unsigned; older SEG counts unsigned; declaration read from an order; TREE unread; actor free under a many-record seal. All forged files printed `HOLDS`, rc 0 [lane record] | **W-MG-HOLES**: id v2 over every cell, `actor_of`, `seg_hash`, FORGOTTEN digests, `tombs_foreign`, `tree_unsigned` | `atk_holes.rs` + `atk_holes/{forger,segments,unbound}.rs` (13 tests); `atk_caught.rs` + `atk_caught/limits.rs` (9, the attacks that were not holes); helper `atk_forge/` |
| **W-MG-ATK2** | Seq density; cost on hostile files | Seqs never asked to climb, so a keyless record at an unused seq could admit a key; four allocation or time shapes (seal ring, nested SEG, nested ANCH, one KEY on many rows). Worst measured 1678× heap over file on a 156 KB file [lane record] | **W-MG-HOLES2**: `doc/count.rs` dense seq, `doc/budget.rs`, `doc/seals.rs`, `doc/frontier.rs` | `atk2_seq.rs` (5), `atk2_dos.rs` + `atk2_dos/shapes.rs` (11), `h2_honest.rs` (5) |
| **W-MG-ATK3** | Disagreement between readers on one file; the host | Reader disagreements on `as usize` wrap on wasm32, padding, duplicate keys and rows, missing dense rule; a Python reader with no budget; the archive host (no pin, unbounded memory, raw output). First run 53 HOLE over 66 images, 11 more on the host bootstrap [lane record] | **W-MG-READ**, **W-MG-HOST**, HOLES2 stage 3, **W-MG-FIX**: no reader pads; `tools/gates/cell-usize.sh`; host pin and module outline (§9.5) | `tools/gates/atk3_run.py`, `atk3_cases*.py`, `atk3_bootstrap.py`, `atk3.sh`, `crates/bebop-wasm-reader/tests/atk3_native.rs`. Accepted differences: `NAMED` in `atk3_run.py` |
| **W-MG-ATK4** | The sha256-id migration: expand, flip, rollback, tablets | Holes in rollout and rollback: a flagged record on an unflipped log, a rotation breaking one-way order, unarmed constructors, deploy parsing, a lost flip mark, double adoption on retry | **W-MG-TRANS** | `atk4_expand.rs` (7), `atk4_rotate.rs` (6), `atk4_nonce.rs` (3), `atk4_s2.rs` (10), `tools/deploy/atk4-{rollback,deploy,seams}.test.sh`; `docs/design/MIGRATION-sha256-ids.md` |
| **W-MG-ATK5** | MERGE and conflict naming | 13 holes [lane record]: a named conflict locking the file for ever; conflicts reading as none; `recheck` depending on file order; a cut laundering a false list; a guard blaming the wrong file; a cut inside one sync; audit cost | **W-MG-MERGE2**: `StockLedger::history`, `CutInsideSeal`, exit 4 `NOT CHECKED`, canonical order from signed edges (`doc_seal/sides.rs`) | `atk5_{stuck,silent,recheck,guard,dos,cutsync,rate,held}.rs` (36), helper `atk5_common/` |
| **W-MG-TAM** (traces) | Four failures the model predicted, run on real `verify` with toy seams | H1 non-root cut of another device's REVOKE; H2 back-dated ADMIT, REVOKE named no tip; H3 forged TRUNCATE at a forked seq escaping status 75; H4 a MERGE row under a cut names an unsigned tip | H1 root-only TRUNCATE; H2 `bebop-revoke/2` with `tip`; H3 a forked seq declares nothing; **H4 stays a limit** (L14). All W-MG-MERGE2 | `crates/bebop-log/src/doc/tam_tests.rs` (flipped to law); original traces in `docs/design/tamarin/rust-traces/tam_tests.rs` |
| **W-MG-LEAN** (model) | Disagreements between `holds` and its Lean model | Three: a tombstone ring at seq 0; a keyed device starting at any seq on a foreign frontier tip; `merge.rs` folding a non-effective REVOKE | Same lane and MERGE2 (`revoked()` from the effective set) | `crates/bebop-log/src/doc/lean_tests.rs` (`da_`, `db_`, `dc_`) |
| **W-MG-ATK6** | Planned: the newest fixes | **Stopped with nothing tested.** FORMAT §14: "no evidence either way". Candidates in its notes are unproven [lane record] | Nothing | None: no `atk6_*` file exists |

**Before those rounds.** W-MG-REVOKE made revocation a fold over records: alternating fixpoint, `keys_disagree`, `tombs_foreign` (`doc/settle.rs`, `doc/settle/tests.rs`). The 2026-09-27 audit's S1/S2 are the origin of the record walk budget (`docs/audit/2026-09-27-AUDIT.md`, `crates/dowiz-hub/src/store_tests.rs`).

**Method note.** The repository has no fuzzers, by operator rule: every case is named and one-off. Coverage is thus exactly the cases written. That is the strongest reason to look past the table. Mutation runs were done per rule by the closing lanes [lane record]; `.github/workflows/mutations.yml` runs over kernel files, and no workflow line read here names `doc/`.

**Reading a test header.** Several headers still say "RED ON PURPOSE until `verify` closes it" (`atk_holes.rs`, `atk2_seq.rs`, `atk2_dos.rs`). Those are historical wording: the same files carry the closed rule now. Run them.

## 8. Six named cases never tested

By title only, from FORMAT §14. No test exists for any. Please treat them as first-priority work and record your method with your finding.

1. A thief rewriting a tombstone's actor
2. A FORGOTTEN listing an id twice or a ghost id
3. A replayed older TRUNCATE
4. Moved cut fields
5. A flagged record shorter than its nonce
6. Two-root trust

## 9. Primitives and provenance

**No third-party crypto crate is on this path** (`SECURITY.md`; `crates/dowiz-core/src/pq/keccak.rs` "no `sha2`/`tiny-keccak`/`pqc` crates allowed"). **All of it is our own code, and none of it has had an external audit that this tree records.** The vectors below are known-answer tests: they establish agreement on those vectors and nothing about secret-dependent timing or fault behaviour.

| Primitive | File, licence | Provenance as stated | Vectors | Stated not done |
|---|---|---|---|---|
| **ML-DSA-65** (FIPS 204) | `crates/dowiz-core/src/pq/dsa.rs`, AGPL-3.0-or-later | The header says both "implemented from scratch" and "a faithful, byte-exact port of the pq-crystals reference"; ported from `bebop2-core pq_dsa.rs` on 2026-07-12. An earlier version was not interoperable. **Resolve this wording with the maintainer** | NIST ACVP FIPS204 vsId 42 in `pq/kat/acvp/{key-gen,sig-gen,sig-ver}.json`, run by `pq/dsa/dsa_acvp_tests.rs`: 60 ML-DSA-65 cases, one `#[test]` per tcId (25 keyGen, 20 sigGen, 15 sigVer). The files also hold ML-DSA-44 and -87, which are not run | `dsa.rs` makes no constant-time, side-channel or zeroisation statement (grep finds none) |
| **ML-KEM-768** (FIPS 203) | `crates/dowiz-core/src/pq/kem.rs`, `kem/ntt.rs`, AGPL | "Algorithms 5-18 as written"; rewritten in P91.2, before which keys could not match the standard | 80 ACVP vectors in `kem/acvp_tests.rs`, provenance `kat/acvp/README-mlkem768.md` | Header: "No constant-time or side-channel review"; only `_internal` exists; no 512/1024. **Not used by seals**; only the backup seal, which is off |
| **Keccak, SHAKE** (FIPS 202) | `crates/dowiz-core/src/pq/keccak.rs`, AGPL | "Inlined on purpose" | FIPS 202 KAT in the file | Kani proofs under `cfg(kani)` for panic and overflow; "does NOT prove codegen-level constant-time". Whether Kani runs in CI was not checked |
| **sha256 in the reader** | `crates/bebop-wasm-reader/src/sha256.rs`, MIT OR Apache-2.0 | Own, "~60 lines" | `tests/doc.rs` `sha256_matches_the_fips_vectors` | Its header gives the M0 formula `sha256(prev ‖ payload)`, which is out of date: `ids.rs` uses `id_message` |
| **sha256 in the hub** | `crates/bebop-store/src/nodekey.rs`, used by `chain/scheme.rs:22` and `doc_seal.rs:61` | Own, "written out below because this crate has zero dependencies" | `nodekey/tests.rs` | Cross-check against the reader's is not named as a test |
| **sha256, fast** | `crates/bebop-store/src/sha_fast.rs`, `sha_fast/scalar.rs` | Own; AArch64 SHA2, x86_64 SHA-NI, scalar | `sha_fast/tests.rs` `nist_vectors_on_the_dispatching_and_the_scalar_path` | |
| **sha256, integrity only** | `crates/dowiz-core/src/sha256_hw.rs` | Header: "NOT for cryptographic MACs, signatures" | | Confirm no seal path reaches it |
| **Merkle tree** | `crates/bebop-store/src/merkle.rs` | RFC 9162 domain separation | `merkle/tests.rs` | SEG `root` can be checked only with the archive |
| **Not cryptographic** | CRC-32, FNV-1a 64 | Integrity and fold values. `legacy_id` is FNV-1a ×4; `K64` is "never a proof" (`nodekey.rs`) | | L9 |

**How the seal uses them.**

- `MlDsa65` and `DeviceKey` live in `crates/dowiz-hub/src/doc_seal.rs`; the crates `bebop-log` and `bebop-wasm-reader` hold no signature code (`doc/seams.rs`). `dowiz-core` is a dev-dependency of `bebop-log` only.
- Seals are external pure ML-DSA-65 with an empty context, `M' = 00 00 ‖ msg`, `rnd = 0^32` (FORMAT §9.4). Any FIPS 204 library's outer interface produces and checks them.
- The `alg` registry has one entry, 65. Any other number or length does not verify. Messages carry ASCII tags: `bebop-seal/1`, `bebop-anchor/1`. Records: `bebop-record/1`. Segments: `bebop-seg/1`.

**What `hybrid_signing` was.** `crates/dowiz-core/src/pq/hybrid_signing.rs` is still in the tree and is **not** a usable scheme. Its header calls it Ed25519 + ML-DSA-65 + SLH-DSA. By its own code the classical leg is not Ed25519: `classical_sign` is `sig = secret XOR H(DOMAIN_SIG || msg)` (`:295-310`), so one signature gives up its secret, as `classical_verify` itself shows (`:314`), and `doc_seal.rs:8` says so. The "SLH-DSA" leg is a domain-separated ML-DSA stub, by its header. `doc_seal.rs` and `doc_seal/history.rs` forbid it for seals. It remains reachable from `crates/dowiz-core/src/ports/agent/cap.rs` (`HybridPolicy`) and `kernel/src/pq/mod.rs`; those callers were not audited here.

**Separate question, pending.** A provenance study of whether to replace our ML-DSA (lane W-MG-PQ, planned as docs/research/R-MLDSA-PROVENANCE.md) was commissioned and that report is not in the tree at the time of writing. Seals are unreleased, so a change is free now and not after the first release [lane record].

## 10. Formal artefacts

### Lean (`bebop-lang/formal/Bebop/`, Lean 4.33.1)

**They model the Rust; they do not prove it.** Each header says so: "NOTHING HERE SAYS THE RUST IS PROVED. A theorem below is about the structure `Holds`; that the Rust counters compute `Holds` is a reading." The link is by reading, plus the twin tests in `doc/lean_tests.rs`.

| File | Theorems |
|---|---|
| `DocLog.lean` | `signed_for`, `reach_own_chain`, `signed_for_honest`, `signer_saw`, `holds_gives`, `admission_signed`, `chain_ends` |
| `DocMerge.lean` | `union_chain`, `union_holds`, `fork_breaks_union`, `merged_holds_iff`, `merges_iff`, `union_keeps`, `union_comm_front`, `union_assoc_front`, `union_idem_front`, `last_verify_is_collision`, `union_nothing_new`, `union_front_max` |
| `DocLimits.lean` | `rollback_holds`, `no_file_check_sees_rollback`, `rollback_hides_revoke`, `past_revoke_blocks_union`, `unsealed_prefix_fails`, `union_holds_side_does_not`, `ring_refused`; fixtures `fOne_holds`, `fTwo_holds`, `fRevoked_holds` show `Holds` is inhabited |
| `Merkle.lean` | 8 theorems; the hash is a parameter, collision resistance a hypothesis `hinj` |
| `Sync.lean` | 35 theorems, including a frontier section |
| `Sha256.lean` | no theorems: it computes digests for the `cas://` form, and is unrelated to a security claim |

**Outside the model** (`DocLog.lean` header): pre-seal actors; legacy ids and tombstones; FORGOTTEN and TRUNCATE accounting; the fixpoint of `settle.rs` and `revokes_unsettled`; `KeyClash`; bytes, crc, HEADS rows, budgets, `claimed_ms`, TREE; u64 is Nat. `Admitted` takes any ADMIT in the file, which the header argues gives the weaker conclusion. `merged_holds_iff` carries hypotheses that the sentence "holds iff both hold and neither forked" lacks; that sentence is false, by theorem (`DocMerge.lean` header).

The Doc files state no `axiom` and no `sorry`. `tools/gates/lean.sh` enforces this by census against `tools/gates/lean.axioms`, whose entries are mostly syscall specifications for the language model.

### Tamarin (`docs/design/tamarin/bebop_seal.spthy`, `README.md`)

**Status: never parsed, never proved, unless a later run shows otherwise.** Every "expected" is a prediction by hand, and a parse error is a likely first result (README). The prover was unpacked on the development box and running it was refused there.

- `.github/workflows/tamarin.yml` is separate from CI and blocks nothing. One run exists, 41302238312 on a7507f4c (2026-11-01, the old workflow): the Prove step was cancelled after 9 minutes, the reason is not public, and no output survives, so it shows neither a parse nor a verdict.
- **The model carries the fixes.** The second pass has `revoke/3` with a tip and root-only `truncate_declared` (README, "How the fixes appear in the model"). It adds a prediction run on the Rust (`tam_l3t_`) and not yet on the prover: L3t, a forged TRUNCATE at status 0 through a tombstone's unbound `prev`, with `holds` false (FORMAT §14).
- A verified lemma could not cover: legacy records, keyless actors, blind readers, anchors and time, sums, byte parsing (ambiguity between bodies is excluded by free constructors, so "no confusion attack can appear"). ML-DSA is a perfect signature and sha256 a perfect hash there (README, "What the model leaves out"). The limit lemmas `limit_rollback` and `limit_stolen_root` read inverted: "verified" means the attack exists.

## 11. Reproducing everything

Not run for this brief. Rust 1.96.1 (`rust-toolchain.toml`). **There is no workspace: `cd` into each crate.** Python 3 and Node needed; Lean by `elan` from `bebop-lang/formal/lean-toolchain`.

```sh
# The library and the seal
(cd crates/bebop-image && cargo test)
(cd crates/bebop-kv    && cargo test && cargo test --features tree)
(cd crates/bebop-log   && cargo test)                 # incl. doc/tam_tests, lean_tests, settle/tests
(cd crates/bebop-io    && cargo test && cargo test --features tree)
(cd crates/bebop-store && cargo test)
(cd crates/bebop-wasm-reader && cargo test)
(cd crates/bebop-wasm  && cargo test --features decide)
(cd crates/dowiz-hub   && cargo test)                 # every atk*, h2_honest, m2_merge
# One round
(cd crates/dowiz-hub && cargo test --test atk_holes --test atk_caught)
# Primitives
(cd crates/dowiz-core && cargo test acvp)             # ML-DSA-65 and ML-KEM-768 vectors
(cd crates/bebop-wasm-reader && cargo test --test doc sha256_matches_the_fips_vectors)
(cd crates/bebop-store && cargo test sha_fast)
# Readers agree, from a clean directory
sh tools/gates/conformance.sh && sh tools/gates/conformance.sh --prove
sh tools/gates/verify-file.sh && sh tools/gates/verify-file.prove.sh
sh tools/verify/gate.sh --prove
# Hostile files, every reader
python3 tools/gates/atk3_run.py write "$D"
(cd crates/bebop-wasm-reader && ATK3_DIR="$D" cargo test --test atk3_native)
python3 tools/gates/atk3_run.py run "$D"
sh tools/gates/atk3.sh && sh tools/gates/atk3.prove.sh
sh tools/gates/cell-usize.sh && sh tools/gates/cell-usize.prove.sh
# The demonstration: real files, real ML-DSA-65, 4 scenarios, each with a named kill
sh tools/gates/maingoal.sh --build && sh tools/gates/maingoal.prove.sh
# The migration
sh tools/gates/migration-law.sh
bash tools/deploy/rollback.test.sh && bash tools/deploy/test.sh
bash tools/deploy/atk4-rollback.test.sh; bash tools/deploy/atk4-deploy.test.sh; bash tools/deploy/atk4-seams.test.sh
# Models
sh tools/gates/lean.sh && sh tools/gates/lean.prove.sh
# Tamarin, x86-64 Linux: parse first (drop --prove), then the command in docs/design/tamarin/README.md
# Everything CI's gates job runs
sh tools/gates/run-all.sh --cargo
```

**Read one file as the third party does**, with no repository:

```sh
doc_verify <image> --trust <actor hex> [--platform-pk <hex>] [--headers <file>]
    # built from crates/dowiz-hub; exit 0 holds, 3 does not, 4 NOT CHECKED, 2 not a sealed image
python3 -I tools/verify/bebop-verify.py <file>        # blind: counts, does not check
node examples/read.mjs <archive.store>                # runs only the pinned reader
```

Fixtures and expected answers: `crates/bebop-wasm/fixtures/`, listed with sha256 in FORMAT §13. `.github/workflows/ci.yml` is the list CI actually runs. dowiz-hub links are large; `atk2_dos` takes `ATK2_FULL=1` for the 1 MB numbers.

## 12. Open questions, ranked

1. **Is our ML-DSA-65 sound beyond 60 vectors?** Own code, no recorded external review, no side-channel statement. Deterministic signing with `rnd = 0^32`, `_internal` with no context, key material handling, `sigVer` malformed-input paths.
2. **The six cases of §8**, then the areas ATK6 never reached (`doc/budget.rs`, `count.rs`, `seals.rs`, `frontier.rs`, the `settle.rs` fixpoint at scale, `valid_from`) [lane record]. `valid_from` is compared with the ADMIT (`settle.rs:159`); say whether anything more needs it.
3. **Does `holds` mean what §5 says?** The conjunction has no proof against the Rust. Find a file that holds under `Trust::Only`, with no root stolen, and outside L1-L16.
4. **The revocation fixpoint.** Is the answer independent of file order at every shape? `settle.rs` is outside both models.
5. **Encoding.** Are the id, seal, REVOKE, TRUNCATE, MERGE and FORGOTTEN bodies injective, across the mark, nonce and kind-byte framing of §9.4? Tamarin excludes this by construction; ATK hole 7 was of this class.
6. **Reader divergence.** Five readers and a host must agree. L27 lists three known gaps; which are security relevant?
7. **Legacy exposure (L9).** What can be done with FNV records under a `seal_hub`, and should sealing a legacy log be allowed?
8. **Anchor adapters.** `ots.rs`, `tsa.rs`, `roughtime.rs` parse hostile proofs, yet an anchor changes no counter. Is an adapter's verdict sound, and is an offline `Unchecked` always distinguishable from ok?
9. **Tamarin.** Bring the model to the code (REVOKE v2, root-only TRUNCATE, forked seq), parse it, run it, report each lemma.
10. **Key custody and roots.** Seeds, rotation, no k-of-n (L16, `R-MAINGOAL-A` §2e). What must a first release specify?
11. **Crash behaviour.** L17 and L18.
12. **The migration.** Scheme state per log, one-way flip, rollback floor (`docs/design/MIGRATION-sha256-ids.md`).

## 13. Traps in the tree's own text

Stale at the time of writing; the code and FORMAT rule.

| Where | Says | True |
|---|---|---|
| `crates/bebop-wasm-reader/src/sha256.rs` header | `id = sha256(prev ‖ payload)` | id v2, `doc::id_message` |
| `docs/research/R-MAINGOAL-A.md` | DOC of 8 cells, `seq` scheme, sizes | dated 2026-10-30, a design: FORMAT §9.4 rules |
| `docs/design/MIGRATION-sha256-ids.md` status | prod at `ac5ff0c4` | dated 2026-10-31; ask for the current state |
