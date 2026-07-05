---
TRIGGER: apps/api/src/**/*.test.ts
CAUSE: >
  A completion/success signal (a status field, a "restore verified" report, a security red-arm
  test) was derived from "the step executed without throwing" or "the check has the right shape"
  instead of from an independent re-read of the effect it claims. Four instances shipped in the
  same 2026-07-03 audit remediation, across production code and tests alike: the GDPR anonymizer
  wrote `completed` without re-reading `anonymized_at` (commit `69ad3074`); the backup restore
  drill compared row-count heuristics against the wrong pool and a mismatched checksum encoding
  (same commit); LC9's fabricated-fallback red-arm never ran its detector against a bugged fixture,
  and LC2's IDOR proof pinned a SQL string instead of a cross-tenant read (commit `aaa0b182`). All
  four passed clean until a decorrelated verification sweep forced each one to answer "can this
  actually go RED on the real defect?" — none of them could, until fixed (ledger #61/#64/#67).
ACTION: >
  Before treating a completion status, a "verified" report, or a negative/security-path test as
  done: ask "what pre-fix state must this see, and does it actually observe that state, not just
  its own success path?" Concretely — (1) a completion write on a red-line surface (GDPR, backup,
  payments, auth) must be gated on an independent read-back of the effect (re-query the row/table
  it claims to have changed), never on "no exception was thrown"; (2) a security/negative-path test
  (IDOR, fabricated-fallback, permission check) must construct the actual adversarial condition and
  be git-verified RED against the pre-fix parent commit — a test that would pass whether or not the
  bug exists is not a proof, per CLAUDE.md's Mandatory Proof Rule ("an assertion that fails when the
  code is wrong").
LINK: docs/regressions/REGRESSION-LEDGER.md #61, #64, #67 ;
  docs/reflections/INBOX/2026-07-05-gdpr-backup-completion-was-unconditional.reflection.md ;
  docs/reflections/INBOX/2026-07-05-proof-hardening-duplicated-invariants.reflection.md
SCOPE: Completion/status writes and negative-path (security/verification) test proofs on red-line
  or otherwise high-consequence surfaces. Not a general "write more tests" nudge — the gap here was
  proofs that already existed and looked complete but structurally could not fail.
STATUS: active
---

# A proof that can't fail on the real bug is not a proof

The 2026-07-03 audit remediation (commits `69ad3074`, `aaa0b182`) fixed four independent instances
of the same shape of bug, in the same two-commit batch: a success signal — a `completed` status, a
"restore verified" drill result, a security red-arm test — that would report success (or pass)
regardless of whether the underlying invariant actually held.

- The GDPR anonymizer worker wrote `completed` after running, without re-reading `anonymized_at`
  from the row it had just supposedly anonymized — a transient failure could strand a data-subject
  erasure while every dashboard reported it done (Art.17 risk, ledger #61).
- The backup restore drill's `checkRowCounts` used a `count===0 || count>base*10` heuristic against
  the wrong (arg-discarded) pool, comparing a ciphertext checksum to a plaintext one — four
  independent ways it could report "restore verified" while verifying nothing (ledger #64).
- LC9's `no-fabricated-fallback` red-arm never actually fed its detector a fabricated fixture — it
  was self-referential, so it would pass whether or not the detector worked (ledger #67).
- LC2's PATCH-IDOR proof pinned the literal SQL string rather than exercising a real cross-tenant
  read — a refactor that kept the string but broke the tenant check would still pass (ledger #67).

All four went undetected until a **decorrelated verification sweep** — an adversarial third pass
asking specifically "can this fail?" — forced each one to answer no. That's the generalizable
takeaway: authoring a proof and adversarially trying to defeat it are two different mental modes,
and the first alone reliably misses this class of bug. CLAUDE.md's Mandatory Proof Rule already
states the destination ("an assertion that fails when the code is wrong"); this lesson is the
concrete reminder, backed by four real repo instances, to actually check that a completion write or
a negative-path test satisfies it — re-read the effect, don't trust the exit code; construct the
real adversarial condition, don't pin the current shape.
