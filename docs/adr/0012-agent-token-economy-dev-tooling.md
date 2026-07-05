# ADR-0012 — Agent token-economy: AST code-search + git-versioned fix-memory (Area C, dev-only)

- Status: **Proposed** (design-time)
- Date: 2026-06-26
- Deciders: DeliveryOS Triadic Council
- Scope: **dev-only tooling — NEVER shipped to prod.**

## Context

Agents re-derive the same invariants every session and lack a fast semantic code map. Two cheap,
boring wins exist without adding runtime surface.

## Decision

1. **C1 — `ccc` (cocoindex-code) AST-semantic search** as a lightweight MCP/skill — **NOT** full
   CocoIndex (no pgvector, no daemon). Dev-only. Must **consult ignore rules BEFORE reading file bytes**
   (B10 — a read-then-filter walker would index `.env`, which is on disk), **never indexes secrets**,
   **zero index artifacts in `dist`**. The secret-scan test is a **merge gate**; until it passes, C1 stays
   disabled.
2. **C2 — git-versioned agent fix-memory, librarian-gated (Counsel 5c, Breaker B11).**
   `docs/agent-rules/INVARIANTS.md`, linked from CLAUDE.md, rephrasing §16 agent-facing
   (integer-money/no-float, never-bypass-RLS, never-edit-applied-migration, WS-only-`useWebSocket`,
   zero-hardcoded-token, `crypto.randomUUID`, RS256). **Authority (B11): the executable guardrail
   (lint rule/test) is authoritative; `INVARIANTS.md` LINKS to each guardrail and must not restate a rule
   with no gate** — no two sources of one rule. **Append path (5c): a correction does NOT raw-write
   `INVARIANTS.md` in-session** (it is linked from an OVERRIDE-authority CLAUDE.md). It routes through the
   **librarian promotion gate** (reflection/lesson → human-reviewed guardrail → linked index entry),
   preserving the ratchet (memory advisory; guardrail/human authoritative).

## Consequences

- (+) Fewer tokens re-deriving context; invariants travel with the repo, gated by the ratchet.
- (−) A dev dependency + a curated index that must point at real guardrails.
- **Never shipped:** no prod image inclusion, no runtime path, no secret indexing.
- **Governance:** no self-authored authority-bearing rule reaches the next agent without the librarian/
  human gate (closes the convergence-theater / self-drift pathology Counsel §5 raises).

## Proof

`ccc` secret-scan **merge gate**: over a fixture tree (on-disk `.env` + a `.gitignore`d secret) it
indexes neither, consulting ignore rules before read; zero `dist/` artifact. C2: an in-session correction
produces a librarian-gated reflection (not a raw `INVARIANTS.md` write); each `INVARIANTS.md` entry links
to an executable guardrail.
