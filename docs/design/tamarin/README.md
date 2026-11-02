# The DOC seal protocol, symbolic model (W-MG-TAM)

`ein_seal.spthy` is a Tamarin model of what a DOC file carries (FORMAT-EINFILE-v2 §9.4):
genesis keys, records, SEAL, ADMIT, REVOKE, TRUNCATE over a SEG hash, FORGOTTEN, MERGE, read under
`Trust::Only`. The adversary holds the whole file, rewrites every unsigned cell, and may hold a
stolen non-root key. A root is stolen only where a lemma says so. It models
`crates/einfile-log/src/doc` as it stood on 2026-11-01.

**STATUS: THIS MODEL HAS NEVER BEEN PARSED AND NEVER PROVED**, unless a later `tamarin.yml` run shows
otherwise. As of 2026-11-01 the public API lists one run, 41302238312 on a7507f4c, with the old
workflow, and it settled nothing: the install passed, the Prove step was cancelled after 9 minutes,
the upload was skipped, and the job ended as failure. The reason for the cancel is not public (the
annotation says only "The operation was canceled", and the log needs a login), and no output survives,
so that run shows neither a parse nor a verdict. Tamarin has not run on the box. Every "expected" below
is a prediction by hand. The only other check is a script counting brackets, comments, duplicate names
and fact arities, which is not a parse, so a parse error is a likely first result. No lemma here says
anything about the Rust.

`.github/workflows/tamarin.yml` runs it on `ubuntu-24.04`, on `workflow_dispatch` and on a push to
main touching this directory, the workflow or `tools/gates/tamarin-verdicts.py`. It is separate from CI, so a red blocks no merge. Tamarin reports a
falsified lemma in its output and not in its exit code, so `tools/gates/tamarin-verdicts.py` reads the
summary after the prover: a lemma with no verdict is red, and a verdict that differs from
`expected-verdicts.txt` is red. This form of the workflow has not run yet. **That list does not exist
yet, because no verdict has ever been produced**, so the step prints the verdicts and says they are compared with nothing. To pin them,
read the first real run against the table below, then commit the `tamarin.verdicts` file of the
`tamarin-out` artifact as `expected-verdicts.txt`. Never write it from the table: the table is a prediction.
Why not on the box: the arm64 bottle was unpacked, and running it through the dynamic loader was
refused by the session's permission classifier on 2026-11-01. Nothing here has been re-tried.

To parse it by hand on x86-64, with the prover and `maude` on PATH:
`tamarin-prover docs/design/tamarin/ein_seal.spthy`. To prove, add `--prove`.

## The lemmas

Expected "verified" on an `exists-trace` lemma means the trace exists. For a `limit_` lemma that trace
is the attack.

| id | lemma | meaning | expected |
|---|---|---|---|
| E1-6 | `exec_*` | commit, admit, revoke, cut, tombstone, merge each reach an accepted view | verified |
| L1 | `sealed_by_device_or_compromised` | an ok seal of a registered key: it wrote that record, or the key was stolen | verified |
| L1b | `unregistered_sealer_needs_a_theft` | a key nobody registered seals only behind a theft | verified |
| L2 | `no_admit_above_after_seq` | under `holds`, an ADMIT by a revoked key sits at or below `after_seq` | verified |
| L2t | `no_admit_for_later_in_time` | under `holds`, a key minted after the REVOKE is not admitted by the revoked key | verified, likely needs induction |
| L3 | `truncate_held_was_signed` | under `holds`, a cut is an honest root's, or a root was stolen | verified |
| L3s | `truncate_status_was_signed_no_tomb` | at status 0, with no tombstone in the view, the same | verified |
| **L3t** | `truncate_status_was_signed` | at status 0, the same, tombstones allowed | **FALSIFIED** |
| L3f | `forgotten_held_was_signed` | under `holds`, a tombstone's FORGOTTEN was signed, or a theft | verified |
| L4 | `merge_parents_signed_uncut` | with no cut, a MERGE row's tip was written by that device, or a theft | verified |
| L5 | `non_root_cannot_cut_a_revoke` | FORMAT §14: a non-root key cannot hide a REVOKE, with the revoker's newer record in the file | verified |
| Lim1 | `limit_rollback` | ATTACK: the view without the REVOKE, and the thief goes on | verified = attack |
| Lim2 | `limit_merge_row_under_cut` | ATTACK: under a cut a MERGE row binds its seq and not its tip | verified = attack |
| Lim3 | `limit_stolen_root` | ATTACK: a stolen root's own cut | verified = attack |

**L3t was read from `count.rs`, then run on the Rust on 2026-11-01 (`tam_l3t_` in `crates/einfile-log/src/doc/tam_tests.rs`, green); the verdict in the table is still a prediction for the prover.** A tombstone's
`prev` sits under no id, so a forger with no key puts a forged TRUNCATE at seq `s` and tombstones the
genuine `s+1` with `prev` on the forgery. Nothing forks, so the forgery is sound and the root's seal
covers it, and status 75 becomes 0. `tombs_foreign` keeps `holds` false, so `holds` is not
fooled: what falls is the refusal, for a caller that reads status alone. A REVOKE or ADMIT should go the same
way; the tests build the TRUNCATE only. A forged FORGOTTEN does not close it: the tests try one under the
tombstone and one above, and each leaves a stale digest or a stale link, and a control with the digest cut
loose from `prev` holds on the same bytes. FORMAT §14 states it as a limit: a caller MUST read `holds`.

## How the fixes appear in the model

| fix | code | model |
|---|---|---|
| only a root's TRUNCATE is effective | `declare.rs:97` | `truncate_declared` asks `RootA`; `Commit_Truncate` needs `!Trusted` |
| REVOKE names a tip, `ein-revoke/2` | `keys.rs`, `declare.rs:103` | `revoke/3`; `holds_revoke_tip_stands` is `off_chain`, with its self-revoke exemption |
| a record at a forked seq declares nothing | `count.rs:112` | `sound_is_unforked`, and the no-fork clause in `revokes_bind`, `truncate_declared` |
| dense seq | `count.rs:95` | chains grow by `%1` from 1 or from a frontier row |
| a chain ends at 0^32 or a frontier tip | `count.rs:176` | `V_first`, `V_first_cut`, `V_first_onto`, `V_first_onto_cut` are the only starts |
| FORGOTTEN | `forgotten.rs` | `forgotten(id, digest)`, one row, by the author or a root |

**There is no second FORGOTTEN mark.** `FORGOTTEN_MARK` is `ein-forgotten/1`, and "FORGOTTEN v2" is wrong
wording that survives in one journal line. What that wording can mean is the body with `(id, digest)` rows, which is what is modelled.

Forks reach further than H3: neither record at a forked seq is sound, the honest one included. So at
status 0 a forger with no key silences an honest REVOKE, or makes an honest TRUNCATE read as undeclared,
by forking its seq. `forks` keeps `holds` false, and `sound_is_unforked` models it.

## What the model leaves out, so a verified lemma cannot claim these

Legacy (FNV) records. Keyless actors, and a REVOKE naming one. Blind readers, for whom a TRUNCATE
stands with no root. Anchors and time. `claimed_ms`, `flags`, `alg` as anything but bound fields.
Two rows in one segment, and the walk down `seg`'s first argument. A prev that leaves the file, which is
`unsealed`'s in `holds` and so has no rule, in a `'status` view too. The `unsettled` branch of `settle.rs`: a
contradiction has no accepted view here. Sums (`redacted = declared`). A cut's reach across devices
in file order: an honest cut here names one device's row. Byte parsing: bodies are free constructors,
so no confusion between them can appear. ML-DSA is a perfect signature and sha256 a perfect hash.

## History

**As of 2026-11-01 02:47, before the fixes**, four failures predicted against the first model ran as
traces on the real `verify`, with toy seams, and reproduced: H1 a non-root key cut another device's
REVOKE, `holds` true; H2 a revoked key back-dated an ADMIT under `after_seq`, `holds` true; H3 a forged
TRUNCATE in a fork escaped status 75, `holds` false; H4 a MERGE row under a cut named an unsigned tip,
`holds` true. `rust-traces/tam_tests.rs` is that pre-fix form and does not build against today's API.
The current traces are `crates/einfile-log/src/doc/tam_tests.rs`, asserting the opposite for H1-H3.
