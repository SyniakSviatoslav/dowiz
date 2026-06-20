# Loop · backend-contract-convergence

**Family:** Error · **Status:** DRAFT (→ verify) · **Trigger:** `/converge-server [scope]` · **When built →** `loops/backend-contract-convergence.yaml`.

**When:** server↔contract/spec mismatch — handler/Zod/migration violates invariants. **Inversion of error-fix:** the *server is the target* (read-write); touching UI to mask a server gap is out of scope.

```yaml
id: backend-contract-convergence
version: 0.1
status: DRAFT
intent: "bring the server into full compliance with the contract/spec; invariants honored"
problem_signature: "server↔contract mismatch; handler/Zod/migration violates invariants"
trigger: "/converge-server [scope]"
role_mindset: "contract engineer of the server: contract = spec, server conforms to it"
preconditions: ["backend boots locally", "contract layer (Zod) + ADR available", "verify:db/verify:rls/migrate:up work"]
execution_skills: [contract-diff, server-test-run, db-verify, minimal-server-fix, engineer-review]
goal: "every server contract = spec; RLS FORCE/integer-money/idempotency/RS256 honored; server tests green"
verification: "server tests green; verify:db/verify:rls/migrate:up green; cross-tenant SELECT=0; idempotency+state-machine guarded; contract diff deliberate"
iron_principles: [contract-is-spec, forward-only-migrations, rls-force-every-tenant-table, integer-money, idempotency-in-pg, jwt-rs256, no-fake-green]
loop_body: [SENSE, DIAGNOSE, ACT, VERIFY, REPEAT]
exit_conditions: "all: contract parity proven; verify:db/rls green; migrate:up clean; cross-tenant=0; idempotency+state-machine tests green; 0 flaky"
gates: [STOP-CONTRACT-MAP, STOP-MIGRATION]
proof_artifacts: [contract-parity-table, verify-db-rls-output, migration-diff, server-test-run]
out_of_scope: [change-UI-to-mask-a-server-gap, edit-existing-migrations]
escalation: "contract change that breaks clients → serious change → /council before code"
skills_required: [repo-access, db-access, test-runner]
memory_file: loops/memory/backend-contract-convergence.md
verification_report: loops/reports/backend-contract-convergence-0.1.md
```
