# Loop · build-stage

**Family:** Build · **Status:** DRAFT (→ verify) · **Trigger:** `/build-stage <N>` · **When built →** `loops/build-stage.yaml`.

**When:** execute one roadmap stage exactly to its ✅ checkpoint. The checkpoint IS the stage's DoD. Scope discipline, forward-only migrations, RLS FORCE, zero feature-creep. A roadmap stage is already-approved design (build-prompt + ADR = plan-truth) → on STOP-SCOPE a human may clear the require-classification gate for that stage; a NEW architectural need not in the stage spec → `/council` first.

```yaml
id: build-stage
version: 0.1
status: DRAFT
intent: "execute one roadmap stage exactly in scope to its ✅ GATE checkpoint"
problem_signature: "need to implement the next stage (Stage N) to its checkpoint"
trigger: "/build-stage <N>"
role_mindset: "build engineer: exactly the stage scope, nothing more; checkpoint = stage DoD"
preconditions: ["stage has a build-prompt/spec", "deps of prior stages green", "migrate:up/verify:db/verify:rls work"]
execution_skills: [stage-spec-read, implement-to-spec, migration-author, db-verify, test-run, engineer-review]
goal: "Stage N implemented exactly in scope; every ✅ Checkpoint item green"
verification: "each ✅ Checkpoint item green; migrate:up clean; verify:db/verify:rls green; cross-tenant=0 where new tables; tests green; zero out-of-scope drift"
iron_principles: [scope-discipline, schema-rich-runtime-minimal, forward-only-migrations, rls-force, integer-money, checkpoint-is-DoD, no-feature-creep, no-fake-green]
loop_body: [SENSE, DIAGNOSE, ACT, VERIFY, REPEAT]
exit_conditions: "all: stage checkpoint fully green; migrations clean+RLS; tests green; cross-tenant=0; zero scope-drift; deps satisfied"
gates: [STOP-SCOPE, STOP-CHECKPOINT]
proof_artifacts: [checkpoint-evidence, migrate-verify-output, test-run, scope-diff]
out_of_scope: [enable-deferred-post-MVP, pull-next-stage, runtime-beyond-scope]
escalation: "NEW serious architectural need within the stage → /council; contract gap → MISSING"
skills_required: [repo-access, db-access, test-runner]
memory_file: loops/memory/build-stage.md
verification_report: loops/reports/build-stage-0.1.md
```
