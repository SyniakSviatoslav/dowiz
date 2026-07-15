# Disk Reclaim Audit — 2026-07-15

> `/` is at 91% (6.8GB free of 75G). This is a plan, not an execution — nothing below has been
> deleted. Every "safe" item was verified, not assumed (SHA-level checks against live git history
> for anything in `cleanup-safety`, not just "it's named backup so it's probably junk").

## Tier A — safe to delete, zero real workflow cost (all regenerable build cache / verified-redundant)

| Item | Size | Why it's safe |
|---|---|---|
| `/root/bebop-repo/target` | 3.8G | Rust build cache — `cargo build` regenerates |
| `/root/dowiz/kernel/target` | 2.4G | same |
| `/root/dowiz-pq/target` | 1.7G | same |
| `/root/dowiz/tools/native-spa-server/target` | 1.0G | same |
| `/root/dowiz-b1-kalman/kernel/target` | 480M | same |
| `/root/dowiz/agent-governance-wasm/target` | 360M | same |
| `/root/bebop-repo/bebop2/ports/github/target` | 263M | same |
| `/root/.claude/jobs/c6a4c73f/tmp/bebop2-pwn/target` | 186M | orphaned job-scratch build cache from a completed task |
| `/root/dowiz/engine/target` | 25M | same |
| `/root/bebop-repo/bebop2/ports/telegram/target` | 5.5M | same |
| `/root/.npm/_npx` | 731M | npx re-downloads packages on demand, always safe to clear |
| `/root/.cache/typescript` | 32M | TS server cache, regenerates |
| `__pycache__` (all, across /root) | 131M | Python bytecode cache, regenerates |
| `/root/dowiz/attic/apps-api/node_modules` + `apps-worker/node_modules` | 7.4M | attic = intentionally quarantined/archived code (per project convention), not actively built; `pnpm install` would restore if ever resurrected |
| **`cleanup-safety` — verified-redundant subset** | | |
| `/root/cleanup-safety/dowiz/` (26 per-branch `.bundle` files) | 2.0G | **Checked, not assumed**: extracted all SHAs from these 26 bundles, compared against `dowiz-all.bundle`'s 88 refs — **0 SHAs unique to the per-branch bundles**. They're a redundant per-branch breakdown of exactly what `dowiz-all.bundle` already has in one file. |
| `/root/cleanup-safety/bebop-all.bundle` | 12M | **Checked**: all 70 refs confirmed present as live objects in `/root/bebop-repo` right now. Fully redundant with the actual repo. |
| `/root/cleanup-safety/dowiz-stashes.bundle` | 119M | **Checked**: the one stashed commit it captures is confirmed present in dowiz's live object database. Redundant. |
| `dowiz.log` + `bebop.log` | 0 bytes | empty files |
| **Tier A total** | **≈13.1G** | |

### Tier A — explicitly KEPT, not a candidate

- **`/root/cleanup-safety/dowiz-all.bundle` (238M)** — checked the same way as everything else, and it's
  NOT redundant: **14 of its 88 ref SHAs are missing from the live dowiz repo's object database** —
  meaning ~14 branches (including `docs/plane-status-2026-07-02`, `backup-wip-2026-07-08`, and others)
  exist ONLY in this bundle. This is real, unique history. Do not delete without reviewing what those
  14 branches actually contain first.
- **`/root/hermes-agent-kernel-rewrite/hermes-kernel/target` (123M)** — excluded because the Opus
  rewrite agent is actively building here right now (confirmed via mtime — modified within the last
  few minutes). Revisit once that task completes.

## Tier B — safe to delete, but has an immediate practical cost worth confirming first

These are also 100% regenerable (nothing unique is destroyed), but deleting them means the *next*
command in that specific directory (`pnpm install`, `pnpm build`, etc.) will be slower and needs
network access — a real if minor workflow interruption, including possibly in this very session.

| Item | Size | Note |
|---|---|---|
| `/root/dowiz/node_modules` | 1.3G | dowiz's main active dependency tree — needed for the next `pnpm build`/`lint`/`typecheck` in this repo |
| `/root/dowiz/spikes/living-knowledge/node_modules` | 977M | `spikes/` was committed 13 minutes before this audit — actively-touched, not stale, but its `node_modules` is still just a cache |
| `/root/dowiz/rebuild/web/node_modules` | 254M | `rebuild/` was committed 30 hours ago — also recent |
| `/root/dowiz/web/node_modules` | 163M | a third, separate "web" node_modules within dowiz (distinct from `apps/web` and `rebuild/web`) — worth confirming which of these three is actually the live frontend before treating any as disposable-and-forgettable |
| `/root/dowiz/tools/bebop/node_modules` | 46M | |
| **Tier B total** | **≈2.7G** | |

## Tier C — needs a decision, not just a click

| Item | Size | Question |
|---|---|---|
| `.opencode` (×3: `/root/.opencode`, `/root/.config/opencode`, `/root/dowiz/.opencode`) | 189M | A separate AI coding CLI tool. Binary last touched 2026-07-08 (7 days before this audit) — not recently active, but I can't confirm from disk state alone whether it's abandoned or just not used this week. Confirm before removing. |
| `/root/claude-fixed-backup` | 17M | Looks like a point-in-time `.claude/` config backup (CLAUDE.md, skills, hooks, settings). Small enough that reclaiming it barely matters — flagging for completeness, not urgency. |
| `/root/restore` | 4.0K | Empty directory. Trivial, harmless either way. |

## What this does NOT include (deliberately out of scope)

Per your own architectural direction (repeated ~8 times in the Hermes usage log this session's audit
surfaced: "why are there still ts/nodejs remnants... rust/kernel/wasm only") it might be tempting to
read this task as "also rip out the TS/React frontend." I did not include that. Dowiz's own `CLAUDE.md`
still lists Node.js/TypeScript as the live entry-point stack (`apps/web`, `apps/api`), and `spikes/`
and `rebuild/` show very recent commit activity — these are working files, not dead ones. Disk
reclaim (removing regenerable caches and verified-redundant backups) is a completely different,
much safer action than a frontend migration decision, and I kept them strictly separate.

## Recommended order, if you approve

1. Tier A now — zero practical downside, ~13.1G back (would take free space from 6.8G to ~19.9G).
2. Tier B whenever convenient — same safety profile, just budget a `pnpm install` afterward in
   whichever of those directories you next work in.
3. Tier C after a quick confirm on `.opencode`'s actual status.
4. Revisit `dowiz-all.bundle`'s 14 unique-only branches separately — worth a look before their only
   copy is ever removed, unrelated to this disk-reclaim pass.
