# Bebop — Agentic CLI Research (Verified)

> Authoritative research for the customizable, model/OS-agnostic, post-quantum-ready agentic CLI.
> Every claim below was verified against the live repo and the public landscape on 2026-07-08.
> Companion: PLAN.md (what we build and in what order).

---

## 0. Chain of thought (how I reasoned, so you can audit it)

1. **Before designing anything, read what exists.** Prior sessions already started Bebop in two
   places: `tools/bebop` (TypeScript) and `rebuild/crates/bebop` (Rust). I read all of it.
2. **Verify, don't trust the memory.** I ran the TS tests (14/14 pass) and tried to compile the Rust
   crate (it FAILS at HEAD — `PriceInputs` was removed from `domain`). That single fact changes the
   plan: the Rust crate is not shippable today; the TS implementation is.
3. **Is "cog/kernel" generic or order-specific?** The `domain` kernel (`decide`/`fold`/`validate`) is
   the *order lifecycle* state machine (10-state relation, money composition). It is NOT a generic
   agent kernel. So "uses cog/kernel for the 100% core" must be reinterpreted: the deterministic,
   event-sourced, provable *shape* of the kernel is what we reuse (a `Decide`/`Fold`/`Replay` law for
   the agent's own command log), not the order transitions themselves.
4. **What does dowiz already have that maps to the ask?** Guard/red-lines (✅ both TS+Rust), token
   router + model routing (✅ TS `router.ts`), living-knowledge retriever (✅ `spikes/living-knowledge`,
   recall@5=1.0, deterministic), orchestration/fallback/mesh (✅ `scripts/ORCHESTRATION.md`,
   `agents-mesh.sh`, `hermes-fallback.sh`, `guardrail-subagent-return-guard.mjs`), brand/voice (✅
   `BRAND-BIBLE.md`), themes (✅ both `theme.ts` and `brand.rs` use the exact Cowboy Bebop teal
   `#46B0A4`). Post-quantum encryption (❌ NONE EXISTS — must be built). Settings/onboarding (❌ NONE
   EXISTS — the 5-axis selector is net-new).
5. **Competitive landscape.** OpenCode (OS, provider-agnostic, JSON config, TUI), Claude Code
   (Anthropic-locked, polished), Codex CLI (OpenAI, sandbox-first), Aider (git-native, diff model),
   Goose (MCP-native), Hermes (this agent — multi-tool, skills, memory). None ship the 5-axis
   personalization + post-quantum vault + standalone-node autonomy you want. That is the wedge.
6. **Iterate the recommendation.** First cut: "port everything to Rust." Rejected — Rust core is
   broken and over-engineered for a CLI that must be trivial to install across OSes. Better: ship a
   **TypeScript-first** CLI (one `npm i -g` / `npx` / `cargo install` later) that reuses the existing
   TS modules, and keep the Rust `domain` kernel as the *reference implementation* of the
   deterministic log law (not a runtime dependency). This honors "model/OS-agnostic, easy install,
   standalone node" while not blocking on the broken Rust build.

---

## 1. What already exists (verified, reuse-don't-rebuild)

| Capability | Location | Status | Verified |
|---|---|---|---|
| Agentic loop (read/edit/run/grep/done + guard gate) | `tools/bebop/src/loop.ts` | works | ✅ 14/14 tests |
| Red-line + scope guard (falsifiable RED+GREEN) | `tools/bebop/src/guard.ts` | works | ✅ selfTest PASS |
| Token router / model routing (haiku/sonnet/opus) | `tools/bebop/src/router.ts` | works | ✅ unit-tested |
| Living-knowledge §0·GP retriever seam | `tools/bebop/src/knowledge.ts` + `spikes/living-knowledge` | works (shells out) | ✅ recall@5=1.0 |
| Warm Cosmo-Noir voice + microcopy | `tools/bebop/src/voice.ts`, `brand.rs` | works | ✅ matches BRAND-BIBLE |
| Theme (Cowboy Bebop teal `#46B0A4`) | `theme.ts`, `brand.rs` | works | ✅ exact hexes |
| Orchestration / fallback / mesh / subagent guard | `scripts/ORCHESTRATION.md`, `agents-mesh.sh`, `hermes-fallback.sh`, `guardrail-subagent-return-guard.mjs` | exists (shell/Node) | ✅ read |
| Deterministic event-sourced kernel (reference) | `rebuild/crates/domain` (`decide`/`fold`/`replay`) | compiles (domain only) | ✅ cargo 1.96 present |
| Rust Bebop host (CLI over kernel) | `rebuild/crates/bebop` | **BROKEN at HEAD** | ❌ `PriceInputs` removed |

**Conclusion:** `tools/bebop` is the live foundation. Build the 5-axis selector + PQ vault + autonomy
on top of it. Treat `rebuild/crates/bebop` as a future native binary, not a blocker.

---

## 1.5 Architecture reframe — Bebop is the abstract CONDUCTOR layer (operator clarification)

Bebop is NOT a from-scratch model-caller competing with OpenCode/Claude/Codex. It is an **abstract
layer above** them: it presents the established agentic-CLI UX users already know, but underneath it
**dispatches commands to any connected agentic CLI**, rotates between them, and orchestrates them —
so the user is never held hostage to one tool, one vendor, or one model.

- **Backends = pluggable adapters.** Each adapter knows how to invoke one CLI: its binary, flags,
  stdin/stdout or JSON-RPC protocol, and how to parse results. Shipped adapters: `opencode`,
  `codex`, `claude`, `hermes`, `aider`, `goose`, plus a `native` one (Bebop's own deterministic
  loop from `loop.ts`, used when no external CLI is connected or as a fallback).
- **Dispatch is a tool.** The agent loop's `tools` gain a `dispatch <backend> <command>` that shells
  out to the chosen CLI. Every dispatch is recorded as an **envelope** in the deterministic log
  (the `kernel` law) — so the whole session is replayable and auditable across backends.
- **Rotation/orchestration = the routing spine.** Per task, `routing.ts` (the token router + health
  probe + fallback) picks a backend. If OpenCode is wedged, it rotates to Codex/Hermes/Aider. This
  is exactly `scripts/agents-mesh.sh` (Hermes→OpenCode→Goose→Aider→OpenHands) + `hermes-fallback.sh`
  **promoted to a first-class, configurable core** — not a shell script we run by hand.
- **No single point of failure.** If no backend is connected, Bebop still runs its own `native` loop
  (or says so plainly). The user copies Bebop, points it at their own CLIs + BYOK keys in the PQ
  vault, and owns the whole stack — matching the "standalone autonomous node" goal.
- **The 5-axis selector tunes the conductor.** Origin → default backend rotation order + which
  backends are enabled. Class → tool allowlist + skill set. Narration → voice. Patrons → reasoning
  style. Looks → theme + mascot. All of it is data in one `Profile`, not branching code.

This reframe changes the build order: the **backend-adapter + routing core comes before** any live
model wiring, because Bebop's value is orchestration, not yet another model caller.

---

## 1.6 Governing principle — cross-cutting layers are Bebop's, applied ONCE to every backend

The operator's hard rule: **token accounting, orchestration/rotation, model selection, the agentic
guard rules (red-lines + scope), and living memory are owned by Bebop and applied identically to EVERY
connected CLI** — Claude Code, Hermes, OpenCode, Codex, Aider, Goose, or the native loop. They are NOT
re-implemented per tool. Consequences for the design:

- **Bebop governs; backends execute.** Each backend adapter is THIN: it only translates Bebop's
  canonical task + envelope into that CLI's invocation flags and parses its stdout. It does not decide
  routing, does not hold red-line logic, does not meter tokens itself.
- **The guard wraps `dispatch`.** `guard.ts` (red-line + scope, falsifiable RED+GREEN) runs BEFORE any
  backend is called, for every backend equally. No backend can opt out.
- **Token accounting is central.** A single `token.ts` tallies usage across all backends into one
  ledger (the operator's token-economy rule), regardless of which CLI spent it.
- **Model selection is Bebop's call.** `router.ts` chooses the model lane (haiku/sonnet/opus) and the
  backend; the backend just receives the chosen model via its flag. Selection is never delegated to the
  tool.
- **Living memory is shared.** `knowledge.ts` + the local memory store feed the SAME context to every
  backend. A fact learned through OpenCode is available to Claude Code next session.
- **Rotation is uniform.** When a backend wedges or fails, `routing.ts` rotates to the next healthy
  backend — same logic, any backend. No special-casing.

So a "connected agent" is a dumb executor behind a uniform policy envelope. This is what makes Bebop
backend-agnostic and why a user is never locked to one vendor: the intelligence lives in Bebop, not in
the tool it drives.

---

## 1.7 Zero-cloud determinism — the core is kernel + cog, PQ-encrypted, provider-free

The operator's governing constraint: **Bebop's own logic must be 100% reliable and deterministic —
"kernel one", expanded with "cog", protected by post-quantum (psq) cryptography, with NO reliance on
cloud or any provider.** This is the red line that separates Bebop from hosted agent products.

- **kernel** = the deterministic decision law. In this repo it already exists as
  `rebuild/crates/domain/src/kernel.rs` (`decide` / `validate` / event-sourced log) — pure, total,
  IO-free. Bebop ports the *shape* of that law into TypeScript (`loop.ts` envelopes + `guard.ts` gates
  + `token.ts` ledger): every action is a recorded, replayable, deterministic event. No RNG, no clock,
  no network in the recording path — exactly like the Rust kernel.
- **cog** = cognition/orchestration built ON the kernel: `routing.ts` (select/rotate), `profile.ts`
  (the 5-axis brain), `knowledge.ts` (living-memory seam), `init.ts` (personalization). All local,
  all deterministic given the same inputs. The conductor may *dispatch* to BYOK backends, but Bebop's
  brain adds zero cloud calls of its own.
- **psq cryptography** = the local vault (`vault.ts`, Phase 2) wraps the `Profile` + BYOK keys in ML-KEM
  / Dilithium (via a vetted WASM lib, e.g. `@noble/post-quantum` once stable, or an `age`-style
  AES-256-GCM body keyed by an Argon2id device passphrase). Everything at rest is encrypted; in use,
  keys are handed to the chosen backend's process env only, never transmitted to Bebop's servers —
  there are no Bebop servers.
- **No provider reliance** = the default build ships with NO default API keys, NO telemetry, NO
  update-phone-home (only an explicit, signed, user-overridable `bebop update`). A fresh `npx bebop`
  runs fully offline on the `native` backend with the deterministic stub. Connectivity is opt-in BYOK.

Consequence for architecture: the **deterministic core (kernel + cog + vault) is the product**. The
pluggable backends are interchangeable executors behind it. If every external CLI vanished tomorrow,
Bebop would still boot, still enforce its guard law, still replay its log, still manage its memory —
just without a remote model to call. That is the autonomy guarantee.

---

## 2. The 5-axis personalization (your spec, decoded)

| # | Axis | Values | Repo hook |
|---|---|---|---|
| 1 | **Origin** (behavior baseline) | claude / opencode-hermes / codex | maps to loop style + default tool set + model defaults |
| 2 | **Class** (focus) | multi / marketing / sales / automation / research / osint | maps to system-prompt module + skill set + tool allowlist |
| 3 | **Narration** (voice) | dry-cosmo-noir / plain / sarcastic / corporate-killer | maps to `voice.ts` tone profile |
| 4 | **Patrons** (reasoning archetype) | rock (cold logic) / garden (warm gods) / hybrid | maps to reasoning-temperature + explanation style |
| 5 | **Looks** (theme + mascot) | bebop / claude / opencode / codex / custom + pixel-mascot | maps to `theme.ts` palette + animated mascot frames |

**Native recommended preset = "Bebop"**: origin=claude-opencode-hybrid, class=multi, narration=bebop
(dry, truthful, slightly offensive), patrons=hybrid, looks=bebop (cosmo-gothic-noir jazz, teal signal).

---

## 3. Post-quantum encryption — current state: NONE

Searched the whole repo for `kyber|ml-kem|postquantum|secretbox|chacha|argon2|age.crypt` → **zero
hits**. So the "encrypted for the post-quantum age" vault is net-new. Recommendation:

- Use **ML-KEM (Kyber, FIPS 203)** for key encapsulation + **ML-DSA (Dilithium, FIPS 204)** for
  signing, via a vetted WASM lib (`liboqs` WASM build, or `@noble/post-quantum` once stable). Never
  roll your own.
- Vault layout: a local encrypted store (`~/.bebop/vault.age` style) holding provider keys + memory
  index + settings. PQ-KEM wraps a symmetric file key (AES-256-GCM or XChaCha20-Poly1305); the
  symmetric key is what's PQ-encapsulated per-device so the vault replicates across devices without
  re-encrypting the body.
- Derive the device key from a passphrase with **Argon2id** (memory-hard) — not PBKDF2.
- This is a security red-line area (auth/keys) → ship behind a clear, tested boundary; provide a
  RED-proven test (tampered vault fails to open; wrong passphrase fails).

---

## 4. "Standalone autonomous node" — what it really means

Your differentiator vs OpenCode/Claude/Codex: **a Bebop user copies the binary, points it at their own
provider keys (BYOK), and never depends on your infra, updates, or API.** Concretely:

- **No phone-home.** No telemetry to dowiz servers by default. (Telemetry is opt-in, local, and the
  §0·GP retriever already runs fully offline.)
- **BYOK-first.** Provider keys live ONLY in the user's local PQ vault. Bebop ships zero default keys.
- **Self-updating optional.** `bebop update` pulls a signed release from a URL the user controls
  (default: the public repo, but overridable). Never auto-mutates without consent.
- **Model/OS-agnostic.** One TypeScript core compiled to a single binary (bun/deno compile, or
  `pkg`/esbuild → npm bin). Runs on Linux/macOS/Windows, any model behind an OpenAI-compat or
  Anthropic-compat endpoint.
- **Open source.** AGPL-3.0-or-later (matches `rebuild` workspace license) + DCO + trademark note.

---

## 5. My iterated recommendations (what to actually build — conductor-first)

1. **Ship TypeScript-first**, not Rust-first. Reuses the verified 14/14 module set; trivial install;
   OS-agnostic. Keep the Rust `domain` kernel as the *reference* for the deterministic log law.
2. **Build the conductor core FIRST** (the reframe in §1.5). A `backend.ts` with pluggable adapters
   (`opencode`/`codex`/`claude`/`hermes`/`aider`/`goose`/`native`) + a `dispatch` tool that shells out
   and records an envelope. Promote `scripts/agents-mesh.sh` + `hermes-fallback.sh` into `routing.ts`
   (health probe → pick → rotate on failure). This is Bebop's actual value, so it leads.
3. **Build the 5-axis `init` wizard next** — the novel surface you explicitly asked for. It writes
   `~/.bebop/settings.json` (a typed `Profile`) consumed by every other module: origin sets the
   default backend rotation + enabled backends; class sets tool/skill allowlist; narration sets voice;
   patrons sets reasoning; looks sets theme + mascot.
4. **Add a PQ vault module** (`vault.ts`) — ML-KEM/Dilithium + Argon2id + AES-256-GCM. With RED+GREEN
   tests. Holds BYOK provider keys + memory index + settings, encrypted per-device. Security
   differentiator; build carefully behind the auth red-line.
5. **Make the kernel law reusable**: extract `Decide/Fold/Replay` for the agent's own command log
   (every dispatch/action recorded as an envelope, replayable, deterministic) — a thin TS port of the
   `domain` kernel, not a Rust dependency. This is what makes cross-backend sessions auditable.
6. **Living memory** = wire `knowledge.ts` to the local retriever (already deterministic) + a plain
   JSONL/SQLite memory store encrypted in the vault. No external service required.
7. **Provider passthrough (optional, later)** — when a backend is `native`, a `provider.ts` adapter
   (OpenAI-compat + Anthropic-compat + local) injects a live model like the existing `llm` stub. This
   is the LAST piece, not the first; orchestration of existing CLIs is the point.
8. **Looks/mascot** = `theme.ts` already supports palettes; add a `mascot.ts` with a tiny animated
   pixel sprite (ANSI frames / sixel) selectable per the "Looks" axis, plus "create your own."

---

## 6. Risks & honest limits

- Rust `bebop` crate is broken; do not depend on it at runtime.
- `domain` kernel is order-specific — only its *shape* (decide/fold/replay) is reusable for the agent.
- PQ libs in pure TS/WASM are younger than Rust `liboqs`; pin versions, test RED+GREEN, allow a
  Rust-binary vault backend later.
- The 5-axis selector is preference, not correctness — keep it as data, not as branching code paths
  that rot. A single `Profile` object drives switches.
- I have NOT yet built the live provider call (no keys, and auth is a red-line). The `llm` injection
  seam is the safe path; wiring a real endpoint is a later, explicitly-approved step.

See PLAN.md for the build sequence and exit criteria.
