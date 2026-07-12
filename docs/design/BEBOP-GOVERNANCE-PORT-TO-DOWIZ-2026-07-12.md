# bebop governance → dowiz port — 2026-07-12

> Operator (verbatim, uk): "продовжуй працювати над планом, використовуючи усі добавлені
> правила та фічі bebop тут також — поки не буде зроблено, повний автопілот на твій розсуд."
>
> Meaning: continue the plan, apply ALL bebop governance/identity features **here (dowiz) too**,
> full autopilot until done.

## Source of truth
bebop's governance primitives live in `/root/bebop-repo/crates/bebop/src/`:
`agent_profile.rs`, `gender.rs`, `settings.rs`, `drift.rs`, `error_patterns.rs`
(546 Rust tests, 0 fail — verified green this session).

## What was ported (DONE + verified)
Isolated, zero-dependency TS module `agent-governance/index.ts` (+ `index.test.ts`),
10 RED+GREEN `node:test` (run via `npx tsx --test`, all green). One consolidated file
(ponytail: fewer files than a 1:1 bebop module split, same behavior).

| Axis | Default | Notes |
|------|---------|-------|
| Gender (R) | masculine | parser ua/en; grammatical + style |
| Profanity | poderviansky (Подерв'янський) | 3 levels: dosed / forbidden / poderviansky |
| Archetype | corpo (DEFAULT antagonist) | reptiles/contrabandists/aliens = collaborative; witches/KPT/karma = DISABLED-by-default opt-in |
| **Voodoo** | **HARD BAN** | NOT in settings dict → cannot be toggled; author deems voodoo users "хуєсосами" |
| GodRelation | serves God | configurable; custom free-text allowed |
| Settings dictionary | — | self-service `get/set`; voodoo deliberately absent |
| Drift detector | ON | systems-thinking/architecture DRIFT (new-global-dep / layer-bleed / god-module / boundary-removed / loop-ignored) |
| Error-patterns | — | scan at session/loop/debug END → persisted JSON → summary block (auto-learning) |

Default profile is voiced in **dowiz's brand** (Warm Cosmo-Noir + dry Ukrainian irony,
"Hybrid is a feature, not a bug") — not bebop's. So it is "тут також", not a blind copy.

## Verification
- `npx tsx --test agent-governance/index.test.ts` → **10 pass, 0 fail**.
- Mirrors bebop RED+GREEN cases: voodoo hard-ban + absent-from-settings, witches/KPT/karma
  disabled-not-banned, god serves-by-default, settings validation, drift detect, error
  scan/learn/persist.
- No new deps. No touch to the 181 in-flight files on `wave3/integrate`.

## Branch / delivery
- New branch `feat/bebop-governance-port` (isolated; does not disturb `wave3/integrate`).
- Files: `agent-governance/index.ts`, `agent-governance/index.test.ts`, this doc.
- Memory-first: written to dowiz corpus MEMORY.md.

## Remaining (not required by the ask — deferred)
- Wire `defaultAgentProfile()` into the e2e `agent-driver.ts` persona + Astro/owner text.
- CLI surface (`bebop settings`/`drift`/`errors` equivalents) — optional; TS product has no CLI binary.
- TUI identity panel — deferred to G11 swap.
