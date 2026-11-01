# The DOC seal protocol, symbolic model (W-MG-TAM, 2026-11-01)

`bebop_seal.spthy` is a Tamarin model of what a DOC file carries (FORMAT-BEBOP-IMAGE-v2 §9.4):
genesis keys, records, SEAL, ADMIT, REVOKE, TRUNCATE over a SEG hash, FORGOTTEN, MERGE, read under
`Trust::Only`. The adversary holds the whole file, rewrites every unsigned cell, and may hold a
stolen non-root key. A root is stolen only where a lemma says so.

**STATUS: THE MODEL HAS NEVER BEEN PARSED OR PROVED.** Tamarin 1.12.0 (homebrew `arm64_linux` bottle) and
Maude 3.5.1 (Ubuntu arm64) were unpacked on the box, and running them was refused by the session's
permission classifier (below). Every "EXPECTED" in the model is a prediction by hand. A parse error
is the likely first result of the first real run. Its one check so far is a script that counted brackets,
comments, duplicate names and fact arities, which is not a parse.

What did run: the four predicted failures, as traces on the real `verify`, in
`rust-traces/tam_tests.rs`. Those are executions of one file each, with toy hash and signature seams.
They show those four files pass or are not refused. They prove nothing about the Rust in general,
and no lemma here does either.

## Run it

x86-64 Linux, from the repo root, one command:

```sh
curl -sL https://github.com/tamarin-prover/tamarin-prover/releases/download/1.12.0/tamarin-prover-1.12.0-linux64-ubuntu.tar.gz | tar xz && sudo apt-get install -y maude graphviz && ./tamarin-prover --prove docs/design/tamarin/bebop_seal.spthy +RTS -N2 -RTS
```

The tarball's inner layout was not inspected: if `./tamarin-prover` is not at its top, use the path
`tar tzf` shows. First parse only, which is the step to do first: drop `--prove`.

CI job proposal, NOT added (`.github/workflows` is main's):

```yaml
tamarin-seal:
  runs-on: ubuntu-24.04
  timeout-minutes: 120
  steps:
    - uses: actions/checkout@v4
    - run: |
        curl -sL https://github.com/tamarin-prover/tamarin-prover/releases/download/1.12.0/tamarin-prover-1.12.0-linux64-ubuntu.tar.gz | tar xz
        sudo apt-get install -y maude graphviz
        ./tamarin-prover --prove docs/design/tamarin/bebop_seal.spthy +RTS -N2 -RTS | tee tamarin.out
        # A falsified lemma stays here on purpose until its hole closes: pin the expected verdict
        # of each lemma against tamarin.out, do not gate on "all verified".
```

On this box, should the operator allow it: the bottle's ELF interpreter is the unsubstituted
`@@HOMEBREW_PREFIX@@/lib/ld.so`, so it can only start with `/lib/ld-linux-aarch64.so.1` as the launcher, and
it finds `maude` through PATH with `LD_LIBRARY_PATH` at the unpacked debs. That launcher command is what was
refused. It was not retried in another shape.

## The lemmas

| id | lemma | meaning | expected | evidence |
|---|---|---|---|---|
| E1-6 | `exec_*` | commit, admit, revoke, cut, tombstone, merge all reach an accepted view | verified | not run |
| L1 | `sealed_by_device_or_compromised` | an ok seal of a registered key: it wrote that record, or the key was stolen | verified | not run |
| L1b | `unregistered_sealer_needs_a_theft` | a key nobody registered seals only behind a theft | verified | not run |
| L2 | `no_admit_above_after_seq` | an effective ADMIT by a revoked key sits at or below `after_seq` | verified | not run |
| **L2t** | `no_admit_for_later_in_time` | a key MINTED AFTER the REVOKE is not admitted by the revoked key | **FALSIFIED** | **H2, trace ran** |
| L3 | `truncate_held_was_signed` | under `holds`, a TRUNCATE was signed, or a theft | verified | not run |
| **L3s** | `truncate_status_was_signed` | at status 0 alone, the same | **FALSIFIED** | **H3, trace ran** |
| L3f | `forgotten_held_was_signed` | under `holds`, a tombstone's FORGOTTEN was signed, or a theft | verified | not run |
| L4 | `merge_parents_signed_uncut` | with no cut, a MERGE row's tip was written by that device, or a theft | verified | not run |
| **L4c** | `merge_parents_signed` | the same with a cut in the view | **FALSIFIED** | **H4, trace ran** |
| **L5** | `non_root_cannot_cut_a_revoke` | FORMAT §14's sentence, read as written | **FALSIFIED** | **H1, trace ran** |
| L5s | `same_device_cut_takes_the_admit` | the case the sentence does cover: revoker = admitter | verified, may need induction | not run; pinned by `a_thief_with_an_admitted_key_cannot_cut_its_revoke_away` |
| Lim1 | `limit_rollback` | ATTACK EXISTS: the view without the REVOKE, thief goes on | verified (= attack) | not run; pinned by `limit_the_file_before_the_revoke_knows_nothing_of_it` |
| Lim2 | `limit_stolen_root` | ATTACK EXISTS: a stolen root's cut | verified (= attack) | not run; pinned by `limit_a_stolen_root_cuts_a_revoke_away` |

The card's lemma "a key revoked effectively cannot admit a key for later" splits in two, because the
Rust's `valid_at` means seq and the card means time. L2 is what the code enforces. L2t is what was asked.

## The four holes

**H1, real, and it contradicts the format.** An admitted NON-ROOT key cuts away a REVOKE written by a
device other than the one that admitted it, and `holds` is true under `Trust::Only` with no root
stolen. Root A admits T; root C writes REVOKE(T) then one more record; T's holder signs a cut of the
one REVOKE record, keeps its own ADMIT on A's chain, and goes on signing past `after_seq`. C's newer
record stays in the file, so this is NOT the named rollback: the file is not a shorter true file.
Wrong words: FORMAT §14 "a cut through a REVOKE takes that key's older ADMIT with it"; `settle.rs:38`.
That holds only on one chain. `cut.rs:127` `TakesRevoke` is a writer's refusal and a thief has his own
writer; `declare.rs:152` `check_segments` asks only for an effective TRUNCATE, `declare.rs:82` lets any
admitted key sign one, and nothing compares a SEG with what left, because only the archive can.
Cross-device order is unsigned (§14), so the cut need not be a prefix of the true file.
The revoker need not be a root: any valid key may revoke any key, and revoker ≠ admitter is enough.

**H2, real, unnamed in §14.** REVOKE names `after_seq` and no tip (`keys.rs:21`), and `valid_at` reads
only the seq (`keys.rs:74`). So the holder of a revoked key rewrites its own chain at or below `after_seq`
into an ADMIT of a key made after the revocation, re-seals it, and that key is admitted, unrevoked,
and outlives the REVOKE. `holds` is true. The REVOKE is the same id and its seal the same bytes in
both files. `settle.rs:158` `by_valid` and `count.rs` density both pass: no seq is reused inside the
one file. `Doc::merge_into` against an honest copy refuses with `Fork` at that seq, so a second file
sees it, and an anchor over the old tip would. `after_seq` = 0 (REVOKED_ALL) closes it for that key.
It is the "back-dated operation" R-MAINGOAL-A:146 quotes as unsolved by Keyhive.

**H3, real, narrower.** No key at all. `dense` is true for any record whose seq is forked
(`count.rs:95`), so a forged record at a used seq on top of a device's chain is `sound`, and that
device's seal covers its seq (`declare.rs:82`). A forged TRUNCATE there turns status 75 into status 0.
`forks` = 1, `unsealed` = 1, `holds` is false, so `holds` is not fooled. What falls is the refusal, and
`declare.rs:151` "no record leaves a file without a key" read as a refusal. A forged REVOKE or ADMIT goes the
same way (`declarations`, `declare.rs:38`). Any caller that reads status and not `holds` is exposed.

**H4, real, already written in §9.4 as behaviour.** `placed` (`declare.rs:100`) reads no tip at or under
a frontier row, so a MERGE row there names a tip nobody signed, and `holds` is true. §9.4 says "a seq at or
below the newest frontier row", so the text and the code agree; what is missing is a limit in §14:
under a cut, "the rows fix both sides" holds for seq, not for tip.

## What the model leaves out, so a verified lemma cannot claim these

Legacy (FNV) records. Keyless actors. Blind readers. Anchors and time. `claimed_ms`, `flags`, `alg`
as anything but bound fields. Two rows in one segment (one row per TRUNCATE here; a chain of hashes is
`seg`'s first argument and is not walked). A device's first record chaining onto the log: here it
is over `'zero` at seq 1. The `unsettled` branch of `settle.rs`: a contradiction has no accepted view
here, which matches `holds` false and models nothing about its count. Sums (`redacted = declared`).
Byte parsing: ambiguity between bodies, `Ids::body` framing and the nonce are free constructors here,
so no confusion attack can appear. ML-DSA is a perfect signature here and sha256 a perfect hash.
`Rec` of an admitted `b`'s REVOKE binds in every view: a seal past it is not ok, as in the Rust.

The Rust traces use `ToyIds`, `ToySigner`, `ToyVerifier`, so `actor_of` is `None`, not the hub's seams.
No step forges a signature, so H1, H2 and H4 should carry over, and that was not run on `dowiz-hub`.
