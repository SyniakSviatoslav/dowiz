# Loop · error-fix-convergence (canonical)

**Family:** Error · **Status:** CERTIFIED · **Trigger:** `/converge-loop [scope]` · **When built →** `loops/error-fix-convergence.yaml`.

**When:** code "seems to work" + server green, but flows are unverified/red. Brings UI↔server into full correspondence — every flow green — by driving it **live** in Playwright. Server is **read-only** here (contract gap → MISSING/BLOCKED-contract).

**Phases:** A (MATRIX.md all-RED + Playwright harness 390/768/1280, trace/video/screenshot, retries:0 + deterministic seed) → STOP-A → B (RUN → DIAGNOSE → FIX(minimal, single-source) → RE-VERIFY → REPEAT until red/flaky=0) → STOP-B (matrix 100% GREEN ×3 consecutive + X1–X11).

```yaml
id: error-fix-convergence
version: 1.0
status: CERTIFIED
intent: "bring UI↔server into full correspondence, every flow green"
problem_signature: "code 'seems to work' + green server, but flows unverified/red"
trigger: "/converge-loop  (attach: Context-Handoff-v4_5, Service-Build-Plan-v4_4, Architecture-Update-v3_1, component inventory)"
role_mindset: "convergence engineer: test = spec, not a mirror of code"
preconditions:
  - "frontend boots; backend green locally"
  - "Zod contracts + design tokens extracted"
execution_skills: [playwright-run-headed, root-cause-trace, minimal-fix, engineer-review]
goal: "every (screen×flow×role×breakpoint×locale) green"
verification: "MATRIX.md 100% GREEN; X1–X11; 3× consecutive; 0 flaky; separate-agent approve"
iron_principles: [test-is-spec, no-fake-green, server-read-only, live-real-backend, zero-flaky, refactor-keeps-contract]
loop_body: [RUN, DIAGNOSE, FIX, RE-VERIFY, REPEAT]
exit_conditions: "all at once: matrix 100% ×3 + X1–X11 + BLOCKED carved out + 0 contract/design regression"
gates: [STOP-A, STOP-B]
proof_artifacts: [trace, video, screenshots-390/768/1280, MATRIX.md]
out_of_scope: [server-contracts, schema, migrations, weakening-tests, new-features]
escalation: "contract gap → MISSING / BLOCKED-contract"
skills_required: [headed-browser, playwright, repo-access]
memory_file: loops/memory/error-fix-convergence.md
verification_report: loops/reports/error-fix-convergence-1.0.md
```

> Note: the launch work this session used the same discipline (Playwright proof against the deployed app, no fake-green). This loop generalizes it. The repo's existing `/reliability-gate` skill is a sibling artifact-proof harness for the whole order lifecycle.
