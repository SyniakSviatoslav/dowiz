# Token economy & tiered routing (binding, always-on)

Provenance: 2026-09-02 session; measured -78% token volume, ~-93% cost per
gate. All agents working in this repo follow this file.

## Tier routing

| Tier | Model | Work |
|---|---|---|
| Pro | `opencode-go/deepseek-v4-pro` | reasoning, analysis, planning, debugging, spectral synthesis, gate design |
| Flash | `opencode-go/deepseek-v4-flash` (`flash-exec` agent, `small_model`) | everything else: gate execution, compile/run, hash checks, commits, journaling, probe runs |

Pro ALWAYS writes a SPEC card before any Flash-executable work: goal, exact
commands with filled vars, expected output per command, freeze criteria,
journal line. Flash executes verbatim; on any mismatch it STOPS and returns
`VERDICT: mismatch` with evidence — only Pro decides the next step.

SPEC card format:

```
SPEC <id> [TIER:F]
GOAL: one line
CMDS: exact shell commands, in order
EXPECT: expected output per command (compare every one)
FREEZE: fold == N | crc == N | std_golden X pass
JOURNAL: one-line H:DID:GOT:VERDICT
```

## Toolstack (always on)

- `rtk <cmd>` for git/ls/file output (84% measured bash compression).
- `/tmp/opencode/ctx` — orient pack: git + corpus hashes + gate status, one call.
- `tb h <path>` — crc32 content-address (== zlib bit-exact); re-read only if
  the hash changed. `tb ctx` corpus digest. `tb s <needle> <path>` — hit line
  numbers, then read only those windows. `tb c` — stdin compressor.
- `graphify query/path/explain` before grep; `graphify update .` after edits
  (AST-only, no LLM). Graph: `graphify-out/graph.json`.
- `mempalace search <words>` before re-reading history; re-mine journal after
  commits (`mempalace mine docs/exp.journal`).
- Machinery embeds: cached slices (`/tmp/opencode/spectral_machinery.bp`,
  sha256-verified) — never re-read whole std files.
- Gates: compile `>/dev/null 2>&1`; runs read `tail -1` only; one run is proof
  when fold == frozen/oracle value (deterministic integer arithmetic);
  triple-run only at first freeze.
- Journal: one line per experiment `H:... DID:... GOT:... VERDICT:...`.
- .bp programs must be str-free (R3 defect d, journal 1788288206): argv +
  cells + arithmetic; branch-free multiply-select stores (1788288197);
  no allocations inside while bodies (L8).

## Output contract (zero prose)

Status = one line. Evidence = one number/hash/diff. Explanations only when
asked. Canonical verdict line: `DID: <x> | GOT: <y> | VERDICT: <confirmed|mismatch>`.
