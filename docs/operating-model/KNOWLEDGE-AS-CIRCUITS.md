# KNOWLEDGE-AS-CIRCUITS

Operator directive 2026-07-05: our accumulated knowledge — error-patterns, lessons, core programming
rules, system-design rules, and library best-practices — must be **hard mechanical circuits**, not
advisory memory. A circuit fires deterministically on violation; reasoning and skills are the last
line of defense, never the first. Less memory, less reasoning; more machinery.

## The mechanism (built, proven)

- **Registry** — `docs/operating-model/circuits/registry.json`. Each entry is one circuit:
  ```
  { id, source, severity: "red-line"|"warn", glob, type, pattern, [required], [flags], message }
  ```
  - `type:"forbid"` — `pattern` must NOT appear in a matching file (e.g. a float in the money core).
  - `type:"require_together"` — if `pattern` appears, `required` must also appear (e.g. ENABLE RLS ⇒
    FORCE RLS; a new dependency ⇒ its cached `docs/libraries/<name>.md`).
- **Runner** — `scripts/run-circuits.mjs [file …] | --staged`. Strips comments (so a rule never trips
  on prose that merely mentions the token), matches every circuit whose glob matches, prints
  violations, exits **2** on any red-line, **1** on warn-only, **0** clean. No LLM, no skill — a
  pattern matches or it does not.
- **Automatic wiring** (staged, see APPLY): `run-circuits.mjs --staged` in `.husky/pre-commit`
  (blocks a commit that reintroduces a pattern) + a PostToolUse `circuit-guard.sh` (immediate signal
  the moment an edit lands). Neither depends on the agent remembering.

## Promotion (this is how the registry grows — mandatory)

The self-improvement loop already says "a fix ≠ done without a deterministic guardrail red→green."
KNOWLEDGE-AS-CIRCUITS makes the guardrail concrete and uniform: **the registry is that guardrail.**

- Trigger (any): a repeated error-pattern (loop N≥2 of the same signature), a red-line touch
  (auth/money/RLS/migrations), or a lesson that generalizes to a mechanical check.
- Action: add a circuit to `registry.json` (red→green — prove it trips on the bad shape and passes on
  the fixed code), cite its `source` (lesson/ADR/ledger row), and reference it from the lesson.
- The librarian/harness enacts this at stage-close; it never weakens a circuit, only adds/prunes.
- Loop-shaped: seeding the existing lesson/error-pattern store into circuits is per-item work — run it
  as a loop (`/loop-orchestrator`), one lesson → one circuit per pass, until the store is covered.

## Library / framework / language protocol (research-first, cache, circuit-per-best-practice)

Before USING any new library/framework/language — no exceptions, not from memory:
1. **Research** its official documentation + current best practices (WebFetch/WebSearch the canonical
   source; do not answer from training memory — versions drift).
2. **Cache** a distilled note at `docs/libraries/<name>.md` (name, version researched, the 5–10
   load-bearing best-practices, known anti-patterns/deprecated APIs, links). This is the local
   memory/docs copy — future use reads the cache, never re-researches from scratch.
3. **Circuit-per-best-practice**: for every best-practice whose VIOLATION is mechanically detectable,
   add a circuit to the registry (a deprecated call, an unsafe default, a banned import). A
   `require_together` circuit also gates that `docs/libraries/<name>.md` exists before the dependency
   is introduced — so "research-first" is itself enforced, not trusted.

Examples of detectable best-practice circuits: `forbid` a deprecated API signature; `forbid` a
sync/blocking call in an async runtime; `require_together` that adding a crate/pkg to a manifest
co-occurs with its cached doc.

## Honest scope / limits

- **Regex circuits are lexical**, not semantic — they catch shapes, not deep logic bugs (those stay
  with tests + the doubt/council layer). Comment-stripping reduces false positives; strings can still
  trip a greedy pattern — write patterns tight (token-anchored), and prefer `warn` until a circuit is
  proven low-noise, then promote to `red-line`.
- **Circuits complement, don't replace**, the existing specific guardrails (protect-paths, guard-bash,
  clippy/eslint analogs, the sovereign gate). The registry is the UNIFORM place a lesson becomes a
  check; a heavier guardrail (a compiler lint, a test) is still preferred where it fits.
- Seed set today: `money-no-float-in-core`, `no-raw-any-ts`, `no-process-exit-ts`, `rls-force-on-enable`.
  The rest is promotion loop-work.
