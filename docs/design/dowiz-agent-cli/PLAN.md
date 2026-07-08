# Bebop — Build Plan (conductor-first, model/OS-agnostic, PQ-ready)

> Companion to RESEARCH.md. Build order leads with the conductor core (Bebop's actual value),
> then the 5-axis selector, then the PQ vault, then memory/orchestration/looks.
> Every phase ships with RED+GREEN proof (Verified-by-Math). Foundation already verified:
> `tools/bebop` 14/14 tests pass; `rebuild/crates/bebop` is BROKEN at HEAD (not a runtime dep).

## Principles (non-negotiable)
- **Conductor, not competitor.** Bebop dispatches to connected CLIs; never locks the user to one.
- **One `Profile` object** drives all 5 axes — data, not branching code.
- **No phone-home, BYOK-only.** Provider keys live in the user's local PQ vault. Zero default keys.
- **Reuse the verified modules** (`loop.ts`, `guard.ts`, `router.ts`, `theme.ts`, `voice.ts`,
  `knowledge.ts`). Don't rewrite them; extend them.
- **RED+GREEN on every gate.** No false-green metrics. Ship the red case with the green.

---

## Phase 0 — Conductor core (THE value; build first)
Promote `scripts/agents-mesh.sh` + `hermes-fallback.sh` into typed TypeScript.

- `src/backend.ts`
  - `Backend` enum: `opencode | codex | claude | hermes | aider | goose | native`.
  - `BackendAdapter` interface: `binary`, `detect()` (is it installed?), `requiredEnv()` (which keys),
    `buildArgs(task, opts)`, `parse(stdout)` → `{ ok, summary }`.
  - Shipped adapters implement the same CLI flags the mesh script already uses
    (e.g. `opencode run "<task>"`, `codex exec`, `hermes chat -q`, `goose run -t`,
    `aider --message`, `claude -p`). `native` = Bebop's own `runLoop`.
- `src/routing.ts`
  - `selectBackend(profile, taskClass, healthy)` → picks from `profile.origin.backendOrder`,
    skipping backends that fail `detect()` or lack `requiredEnv()`; honors YOLO gating.
  - `rotate(onFailure)` → next healthy backend (the mesh fallback, now code).
  - `healthProbe(backend)` → cheap `binary --version` / `--help` check (no task run).
- `src/loop.ts` gains a `dispatch <backend> <task>` tool that shells out via the adapter and records
  an **envelope** (the kernel law) so the session is replayable across backends.
- **Exit proof (RED+GREEN):**
  - GREEN: with `opencode`/no backends, `dispatch` to `native` runs the loop and records an envelope.
  - RED: a backend listed in `profile` but not installed is skipped (not crashed) and rotation picks
    the next; an explicitly-red-line task is still denied by `guard.ts` before dispatch.

## Phase 1 — 5-axis `init` wizard + `Profile`
- `src/profile.ts`: typed `Profile { origin, classKind, narration, patrons, looks }` + schema defaults
  (the "Bebop" native preset is the default).
- `src/init.ts` (or `bebop.ts init`): interactive TTY selector for the 5 axes; non-TTY → `--preset
  bebop` / `--json` for scripting; writes `~/.bebop/settings.json`. Idempotent re-run.
- Axis → behavior mapping (data-driven):
  - origin → `profile.origin.backendOrder` + enabled backends (+ default model lane per `router.ts`)
  - classKind → tool/skill allowlist + system-prompt module selection
  - narration → `voice.ts` tone profile key
  - patrons → reasoning-style flag consumed by `provider.ts` (temp/system shaping)
  - looks → `theme.ts` palette key + `mascot.ts` sprite id
- **Exit proof:** GREEN: `init --preset bebop` writes a valid `Profile` that `selectBackend` reads.
  RED: a corrupt `settings.json` is rejected with a clear error (no silent bad default).

## Phase 2 — Post-quantum vault (`vault.ts`)
- Local encrypted store at `~/.bebop/vault` (AES-256-GCM body; ML-KEM/Dilithium via vetted WASM lib
  OR Argon2id-derived device key wrapping the file key). Holds BYOK keys + memory index + `Profile`.
- `unlock(passphrase)` / `lock()` / `get(key)` / `set(key, val)`. Wrong passphrase → fail loud.
- Security red-line area → ship with explicit tests; never logs keys.
- **Exit proof:** RED: tampered vault file fails to open; wrong passphrase fails. GREEN: round-trip
  set→get with correct passphrase. (Provider keys never touch disk unencrypted.)

## Phase 3 — Deterministic agent log (kernel law in TS)
- Thin port of `domain` `decide/fold/replay`: every dispatch/action → `Envelope{seq, cause, event}`.
  `replay(log)` reconstructs state. Byte-stable `exportLog()`.
- **Exit proof:** RED: same inputs → identical bytes; illegal action → violation, no event emitted.

## Phase 4 — Living memory
- `knowledge.ts` already shells to the deterministic retriever (recall@5=1.0). Add a local
  JSONL/SQLite memory store (encrypted in vault) for cross-session notes. No external service.

## Phase 5 — Provider passthrough (last)
- `provider.ts`: OpenAI-compat + Anthropic-compat + local. Injected like the existing `llm` stub,
  used ONLY when backend=`native` and keys present in the vault. Not required for orchestration.

## Phase 6 — Looks + animated pixel mascot
- `theme.ts` palettes: `bebop | claude | opencode | codex | custom`. `mascot.ts`: tiny ANSI/sixel
  sprite frames, selectable + "create your own" (user-supplied frames).

## Phase 7 — Packaging & autonomy
- Single binary: `bun build --compile` or esbuild → npm bin (`bebop`). `npm i -g @deliveryos/bebop`
  or `npx`. `bebop update` pulls a signed release from a user-overridable URL. No telemetry by default.

---

## Build order for THIS session
1. Phase 0 (conductor core) — highest value, reuses mesh logic, fully testable offline.
2. Phase 1 (init wizard + Profile) — the novel surface you asked for.
3. A smoke build + `boot` + `run` to prove it works end-to-end, then commit on the feature branch.

Phases 2–7 follow in later sessions (vault = security red-line, needs its own approval gate).
