# Reflection — completion status was written unconditionally, not re-derived from data

- **Date:** 2026-07-05
- **Trigger:** meta-loop backlog item 6 (autonomous-continuation task) — retrospective read of
  commit `69ad30743fc0372225e0003781ca5fd67d8cbe42` ("GDPR erasure fail-loud backstop + LC4
  stranding, LC3 cancel proof, LC7 restore-drill integrity"), a red-line (GDPR/backup) remediation
  batch from the 2026-07-03 audit's verification sweep.
- **Class:** self-improvement loop / test-integrity false-green family (same family as ledger #67).

## WHAT happened

Three unrelated red-line surfaces were fixed in one commit, and all three turn out to share one
shape of bug: **a status field was set to a terminal "success" value without re-reading the data
it claims to describe.**

- GDPR worker (N1/LC4): the anonymizer marked a job `completed` unconditionally after running,
  instead of re-reading `anonymized_at` from the row it just wrote and only completing if that
  read confirms the erasure actually landed. A retryable failure could silently report success
  while data-subject rows stayed un-anonymized — an Art.17 violation invisible to every dashboard
  that trusts the `completed` status.
- Backup restore-drill (LC7): `checkRowCounts` used a `count===0 || count>base*10` heuristic
  against a 9-table baseline, queried the wrong pool (arg-discarded, hit prod instead of the
  scratch target), and compared a ciphertext checksum to a plaintext one — four independent ways
  the drill could report "restore verified" while verifying nothing.
- Customer cancel (LC3, same commit): fixed here but its false-green (see next reflection's
  sibling ledger row #67) was an *owner*-token 403 test standing in for a *customer*-token
  happy-path proof — again, a check that asserts adjacent to the real invariant instead of on it.

## WHERE

`apps/api/src/workers/anonymizer-gdpr.ts` · `apps/api/src/workers/backup/backup-verify.ts` ·
`customer-cancel-after-dispatch.test.ts` · ledger rows #61 and #64 (deterministic proof already
lives there).

## WHY (causal root, not just location)

Both bugs are the same shape at different layers: **a completion signal was derived from "the step
ran" rather than from "the step's effect is independently observable."** The GDPR worker treated
"the anonymize function returned" as equivalent to "the row is anonymized." The restore drill
treated "row counts are non-zero and roughly baseline-shaped" as equivalent to "this is a correct
restore of the same data" — without checking it queried the right database, the right encoding, or
an actually-matching count. Neither bug is a logic error in the *forward* path; both are missing
independent re-verification of the *result*, which is exactly the gap a legal/compliance red-line
(GDPR) and a disaster-recovery red-line (backup restore) can least afford, because their failure
mode is silent and their audit trail is the status field itself.

This is upstream-identical to ledger #67's "false-green proof" class (a test that can't fail on
the real bug) — except here the unverified-completion pattern is in *production code*, not just
its test. The proof gap and the production gap are the same root: **trusting that an operation
succeeded because it executed, instead of re-observing its effect.**

## CONFIDENCE

High — both fixes are read directly from the commit diff and corroborated by ledger rows #61
(LC4/N1) and #64 (LC7), which independently document the same "verified nothing it claimed"
pattern with concrete before/after proof (6/6 and 5/5 red→green).

## NEXT-TIME

For any status/completion field on a red-line surface (GDPR, backups, payments, auth), the
completion write should be gated on an independent read-back of the effect it claims, not on the
absence of a thrown exception. When reviewing a "drill" or "verification" script, check that its
data source (pool/host/table) and its comparison basis (encoding, unit, baseline) are asserted
in the test, not assumed from the script's name.

## PROPAGATE (candidate — advisory; librarian/ratchet decides)

Feeds the curated lesson `docs/lessons/2026-07-05-proof-must-observe-the-effect.md` (this run),
which generalizes across this reflection and its sibling (`2026-07-05-proof-hardening-duplicated-invariants.reflection.md`).

## LINK

`docs/regressions/REGRESSION-LEDGER.md` #61, #64, #67 · commit `69ad3074` ·
[[2026-07-05-proof-hardening-duplicated-invariants.reflection.md]]
