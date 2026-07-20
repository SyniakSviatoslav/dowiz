# BLUEPRINT — Telegram Ops-Observability Hub: Remaining Build Order (Items 2–7)

**Status: BLUEPRINT — items 2–7 not built. Item 1 is DONE (see §0).**
**Precedes:** `docs/design/TELEGRAM-OPS-OBSERVABILITY-CHANNEL-DESIGN-2026-07-20.md` (the design, §"Prioritized build order" — this doc formalizes items 2–7 of that list into DoD-bearing phases; it does not re-litigate the design).
**Scope filter:** same as every other synthesis this session — native-telemetry-only (no external tracker SDK), the Telegram Bot API is the one accepted external surface, every new digest is a new subcommand of the existing `topics` binary (`tools/telemetry/topics/src/main.rs`), never a new crate.

---

## 0. Item 1 — DONE, for continuity

`topics resources` (commit `9f94547ca`, this session): fetches `hetzner-exporter`'s live `/health` (disk_pct/load1-normalized/mem_pct), reads host `cpu_ticks` from `/proc/stat`, posts to the RESOURCES topic (falls back to HERMES/267 until the operator creates the real forum topic — `TELEGRAM_TOPIC_ID_RESOURCES` env override). Everything not yet wired (PMU Tier B, joules/RAPL, CO2e, efficiency ledger, span-latency histograms, GPU, network, mesh peer telemetry) renders as a named `[absent — <reason>]`, never a fabricated number — the exporter's own `-1.0` error sentinel is caught and rendered as an absence too. 6 tests, `cargo tree` confirmed zero dependency drift.

**Not yet done from item 1's own scope:** the pinned live-board `editMessageText` update mode (the design's §3 "pinned message updated in place" idea) — the shipped version posts a fresh message each run. Folded into Phase 2 below rather than left dangling.

---

## 1. Phase 2 — CI/CD & CRON topic + pinned live board (formalizes design items 2 + the item-1 pin gap)

**Scope:** one new `topics cicd` subcommand.

- **Per-run rollup.** One message per CI workflow run on `main`, not one message per job — the design's own volume-discipline law ("nothing per-event high-frequency ever reaches Telegram"). Source: GitHub Actions' own REST API (`GET /repos/{owner}/{repo}/actions/runs/{run_id}/jobs`) via `ureq`, called from a new step at the end of `.github/workflows/ci.yml` (or a scheduled poll — see Open Decision 1). Format: run SHA, trigger, total duration, one line per job (`✅`/`❌`/`⏭️` + name + duration), never 20 separate messages.
- **Daily cron-liveness scoreboard.** Compares "did the 03:17 UTC security scan and the 10-min heartbeat actually fire" against GitHub's own scheduled-workflow run history for the prior 24h — a real presence check, not an assumption. Posted once daily alongside the existing bench/resource daily rollups.
- **Pinned live board upgrade.** RESOURCES gains a second delivery mode: on first post each day, `sendMessage` + `pinChatMessage`; subsequent hourly updates call `editMessageText` on the pinned message id (persisted in a small local state file, matching the existing `.bench_state`-style pattern in `topics/src/main.rs`) instead of posting a new message. Falls back to plain `sendMessage` if the pin/edit calls fail (never a hard dependency on pin permissions).

**DoD:**
- A1: a real CI run on `main` produces exactly one Telegram message in the CI/CD topic, containing all N job verdicts.
- A2: killing/skipping the 03:17 scan for a day is detectable in the next day's scoreboard (test via a mocked/injected run-history fixture, not a live 24h wait).
- A3: RESOURCES topic shows one message being edited in place across a session, not accumulating N messages per day; verified by asserting the stored message-id is reused across calls in the same day and a fresh id is minted on day rollover.
- A4: `cargo tree -e no-dev` for the `topics` crate unchanged (no new dependency — GitHub's REST API needs only the existing `ureq`+`serde_json`).

## 2. Phase 3 — Formalize the S0/S1 mirror rule (design item 3)

**Scope:** currently the S0/S1 "any message classified S0/S1, from any category, also mirrors to topic 257" rule is implicit, expressed only in ad-hoc call sites (`heartbeat-monitor.yml`'s own inline `S0` string, `tools/ops-alert/src/fence_check.rs`'s `severity` field). This phase makes it one real, tested function.

- **`tools/ops-alert`'s existing `severity: String` field** (`fence_check.rs`, `regression_digest.rs`) becomes the single source of truth. Add a small `Severity::{S0, S1, Warn, Info}` type (or reuse a string-tag convention if a type would ripple too far — decide at implementation time, not here) and one function, `mirrors_to_ops_alerts(severity) -> bool`, used by every emitter.
- **The FDR level→severity table**, made real: `kernel::fdr::Level::{Error, Warn, Info, Debug, Trace}` (`kernel/src/fdr/mod.rs:133`, `Error=1` most severe, `Ord` derived) maps to S0/S1 per the design's own stated rule (`kind=Alarm`/`kind=PostMortem` → S1 minimum; `Warn` and below never mirror) — implemented as one small, exhaustively-tested match, not scattered `if` checks per call site.
- **`lib.sh`'s `tg_deliver`/`tg_spool`** gain the mirror behavior: any call tagged S0/S1 additionally spools a copy to topic 257, using the *existing* spool mechanism (no new delivery path).

**DoD:**
- B1: a table-driven test asserts every `(Kind, Level)` combination maps to the design doc's stated severity — this is the actual spec, encoded as an assertion, not prose.
- B2: an integration test posts one S1-tagged message from a non-257 topic and asserts TWO deliveries occur (origin topic + 257 mirror); an Info-tagged message asserts exactly ONE delivery (no mirror).
- B3: `heartbeat-monitor.yml`'s existing inline S0 handling is refactored to call the new shared function rather than duplicating the "also post to 257" logic — one implementation, not two that could drift.

## 3. Phase 4 — LOGS topic (design item 4)

**Scope:** one new `topics logs` subcommand, reading the FDR ring the same way `bench-watch`/`git-watch` poll their sources today.

- **`Error` near-live, batched ≤1/msg/min per topic** (design's own cadence rule) — poll the FDR ring, collect `Level::Error` records since the last poll, batch into one message if more than one landed in the window (never one message per error — an error storm must not become a Telegram storm, the design's explicit worry).
- **`Warn` hourly digest, top-N by name with counts** — a simple frequency table, not a raw dump; this is where the volume discipline matters most since `Warn` is expected to be noisy.
- **`CleanShutdown`/restart lifecycle events, live** — these are rare and individually meaningful, so no batching needed.

**DoD:**
- C1: injecting 50 `Error` records within one polling window produces ONE batched message (not 50), respecting the ≤1/min cap.
- C2: a synthetic hour of `Warn` records with known name/count distribution produces a digest whose top-N table matches the injected distribution exactly.
- C3: a `CleanShutdown` record produces its own message within one poll cycle, not deferred to the next Warn digest.

## 4. Phase 5 — KERNEL/MESH + MEMORY/DOCS topics (design item 5)

**Scope:** two new `topics` subcommands.

- **KERNEL/MESH:** `EventLog::append` outcome counters (`Committed`/`Duplicate` ratio — both `kernel/src/event_log.rs::AppendOutcome` variants), `Hydra::BreachAlert{node_id, group_size}` (live, S1-mirrored per Phase 3's rule) hourly + live-on-breach. **This session landed real cross-mesh replication** (`kernel/src/mesh_replication.rs`, commit `307c3ead5`) since the original design doc was written — this phase should additionally surface `MerkleLog` root-mismatch/convergence events once a real transport exists (per `docs/design/OFFLINE-RESILIENCE-SYNTHESIS-2026-07-20.md`'s Phase B) — until then, honestly render `[absent — no live mesh transport yet, reconciliation algorithm proven but unwired]`, matching the repo's named-absence discipline rather than silently omitting the topic's most interesting future content.
- **MEMORY/DOCS:** daily digest of `MEMORY.md` index changes, new/changed `docs/design/*` files (this whole session's own output — five synthesis docs today alone — is exactly the kind of activity this topic should surface), and roadmap-item status flips (RULED/LANDED/etc., cross-referencing `DECISIONS.md`'s D-series and `CORE-ROADMAP-INDEX.md`'s per-arc status language).

**DoD:**
- D1: EventLog counters reported match a real `cargo test`-driven fixture's actual Committed/Duplicate counts exactly (no estimation).
- D2: a synthetic `BreachAlert` produces a live, S1-mirrored message within one poll cycle.
- D3: MEMORY/DOCS digest correctly detects a real `git diff` between two days' commits touching `docs/design/*.md`, listing exactly the changed files (test against this session's own actual commit history as a real fixture — today's commits are a genuinely rich test case).
- D4: the mesh section renders the honest `[absent]` placeholder today, and the placeholder's exact wording is asserted by test so a future implementer notices when to replace it rather than leaving stale text after Phase B lands.

## 5. Phase 6 — Roadmap-gated enrichment (design item 6, unchanged from the design doc)

No new work specified here beyond what the design doc already states — this phase is inherently reactive (each named roadmap item, when it lands, adds one more real field to an existing topic; item 58 → efficiency section activates, item 69 + an operator-supplied grid constant → CO2e, item 62 → threaded slow-order traces, item 63 → build-provenance line, items 59/60 → ttft/frame-budget, items 55/56 → basis-honesty tags). Each enrichment is a small, independent PR against Phases 2–5's already-shipped topics, not a new topic. No DoD beyond "the specific item's own DoD, plus one assertion that the corresponding `[absent]` placeholder is replaced with a real value and the exact placeholder text this phase's tests pinned (D4 above, and equivalents in Phases 2–4) is what changes."

## 6. Phase 7 — Operator decisions queue (design item 7, restated as open decisions here — see §6 below, do not silently resolve)

---

## Open decisions for the operator

1. **CI rollup delivery mechanism (Phase 2).** A step appended to `.github/workflows/ci.yml` itself (immediate, but couples the workflow file to Telegram delivery — a failure in the Telegram step could theoretically affect CI's own exit code unless carefully isolated with `continue-on-error`), or a separate scheduled poll (`topics cicd --poll`) reading GitHub's REST API for completed runs (decoupled, adds polling latency, needs a `GITHUB_TOKEN` with Actions read scope stored alongside the existing Telegram bot token). Recommend the scheduled-poll shape for isolation, but this touches CI infrastructure and deserves explicit confirmation, not a default.
2. **GPU seam (build vs. drop) telemetry.** The design's own item 7 already named this as unresolved — now partially informed by this session's intent-interface blueprint, which found the `gpu`/`webgpu` engine features are a real, deliberate, already-off-by-default seam blocked on one named operator action (a network grant for `cargo add wgpu`). Recommend: this Telegram topic's "GPU: [absent]" line stays absent until that grant decision is made (tracked in the intent-interface blueprint's own open decisions, not duplicated here) — no new decision needed in this doc specifically, just a cross-reference.
3. **Mesh peer-telemetry blueprint.** Folded into Phase 5's mesh section above (§4) — deliberately deferred to `OFFLINE-RESILIENCE-SYNTHESIS-2026-07-20.md`'s Phase B (the real mesh transport), not re-decided here.
4. **LLM-quality bench arc.** The design doc's own finding stands: "there is no LLM-quality benchmark suite anywhere in the repo... that's a new arc, not a wiring task." This session's local-model-wiring research (parallel track, same batch of work) may produce a concrete eval-harness recommendation — if it does, this topic's "model benchmarks" line gets a real source once that arc lands; until then it stays an honest `[absent — no LLM-quality eval harness exists yet]`.

---

**Files anchoring this blueprint** (unchanged from the source design, plus this session's additions): `.github/workflows/heartbeat-monitor.yml`, `tools/telemetry/lib.sh`, `tools/telemetry/topics/src/main.rs` (now including the `resources` subcommand, commit `9f94547ca`), `tools/telemetry/topics/ZERO-DEP-ALLOWLIST.txt`, `tools/ops-alert/src/{fence_check.rs,regression_digest.rs}`, `kernel/src/fdr/mod.rs` (`Level` enum), `kernel/src/event_log.rs` (`AppendOutcome`), `kernel/src/mesh_replication.rs` (this session's cross-mesh replication landing).
