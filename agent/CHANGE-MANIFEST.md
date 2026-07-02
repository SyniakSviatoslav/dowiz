# CHANGE-MANIFEST

CLASSIFICATION: build   # one of: spike | build | audit | challenge  (§1 — routes the governance mode)

FINDING-id: p0-gate-rearm-2026-07-02
Intent: fix — enact P0 of the meta-loop audit (human-approved 2026-07-02): rearm the silently
disarmed governance gates, govern the Bash lane, wire verify:all --ci into CI, broaden default
agent permissions.

Touched files:
- .claude/hooks/serious-gate.sh — clearance becomes per-line `slug|expiry-epoch` (legacy bare
  slugs = expired); the 7-slug accumulated serious-cleared no longer holds the gate open.
- .claude/hooks/red-line-doubt-gate.sh — redline-confirmed releases the irreversible gate only
  while <60 min old (the 2026-06-23 confirmation held it open for 9 days).
- .claude/hooks/guard-bash.sh + .claude/settings.json — Bash lane governed: protect-paths parity
  for Bash mutations, human-only override files, push-main/prod-deploy blocks; registered as a
  Bash PreToolUse matcher. Staging deploy stays allowed (Ship Discipline) — the old verbatim
  guard blocked it, which is why it was unregistered.
- .claude/commands/council.md — GO step appends `slug|expiry(+72h)` so clearance self-expires.
- .claude/state/{serious-cleared,redline-confirmed} — truncated / retired (stale since 06-21/06-23).
- scripts/guardrail-hook-matchers.mjs — also asserts guard-bash.sh registered under Bash (red→green).
- scripts/guardrail-gate-armament.mjs (new) — hermetic hook simulation: DENY on stale clearance,
  ALLOW on fresh, Bash protected-write blocked. Wired into verify-all.ts (ci:true).
- .github/workflows/ci.yml — validate job runs `pnpm verify:all --ci` (was wired nowhere).
- .claude/settings.json permissions.allow + .claude/agents/librarian.md tools — broader defaults
  (WebFetch/WebSearch/Agent/MCP servers; librarian gets Write+Edit within its existing path contract).

NOTE: .claude/ and .github/ edits are protect-paths zones — applied via staged copy under explicit
per-change human approval ("yes, enact P0", 2026-07-02), the manual-approval path the hook mandates.

Proof: guardrail-hook-matchers red→green; guardrail-gate-armament green; pnpm verify:all --ci green.

# Reminder (§5): a well-proven FAIL / MISSING / BLOCKED is a SUCCESSFUL run, equal to PASS.
