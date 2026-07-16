# HARNESS IMPROVEMENT — SYNTHESIS PLAN (2026-07-16)

> **Design plan, not a fix.** Synthesizes two research passes from this session —
> [HARNESS-RESEARCH-revfactory-and-agentic-teams.md](HARNESS-RESEARCH-revfactory-and-agentic-teams.md)
> and [LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md](LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md) — plus the
> independent verifier pass [SELF-CRITIQUE-2Q-DOUBT-AUDIT.md](SELF-CRITIQUE-2Q-DOUBT-AUDIT.md) §1.1/§3
> — into ONE sequenced plan for the dowiz/openbebop agent harness. Nothing in canon or the roadmap is
> edited here; §4 lists exactly what a future amend pass must touch. All local facts below were
> verified against the working tree / live host on 2026-07-16; estimates are marked.

---

## 1. Executive summary

dowiz's telemetry *consumers* (EV/Kelly model routing, false-claim meter) are ahead of the entire
agentic-team genre but starving on hand-written data — 9 rows in `track_record.jsonl`, 3 in
`precedents.jsonl` (verified) — while the per-call token/cost data they need already sits unharvested
in `~/.claude/projects/*/*.jsonl`. Independently, THREE agents this session confirmed that gating
llama.cpp behind "GPU-unlock" was a category error: this GPU-less host (8-vCPU EPYC Milan, 32 GB RAM,
`/usr/local/lib/ollama/llama-server` already on disk, huggingface.co → 200, crates.io/wgpu → 403) can
run a CPU llama.cpp backend today — treat that as settled fact. The plan: **H1** auto-harvest the
governance ledgers from session transcripts (cheapest, highest leverage), **H2** sovereign per-repo
Telegram notification across dowiz/openbebop/hermes (cheap, config-heavy), **H3** the `LlmBackend`
Trait-as-Port with an operator-gated llama.cpp Tier-1 rollout (medium effort, biggest architectural
payoff, makes M5 hub-autonomy real for the first time).

---

## 2. The three-phase plan

Ordering is by cost-to-leverage ratio, cheapest/highest-leverage first. H1 and H2 touch disjoint
repos/files and may run as parallel lanes; H3 is gated on an operator decision (DECART) and goes last.

### H1 — Auto-harvest the governance ledgers from session transcripts

**What** (harness research R1 + R2, folded into one pass — same parse, same files):
a harvest step that parses per-call token usage out of `~/.claude/projects/*/*.jsonl` (the exact
files `tools/loop-signals/transcript_events.py` already parses) and folds each agent
dispatch/session into `gov_record {model, task_type, success, value, cost}`
(`tools/telemetry/governance.sh:39`, CLI case `:345`), writing
`tools/telemetry/governance/track_record.jsonl`. The same pass emits one
`agent_turn.jsonl` row per subagent dispatch — `{run_id, unit, model, tokens, ms, verdict,
files_touched}` — via the existing `log_event` (`tools/telemetry/lib.sh:69`), giving every ledger row
a run-correlation ID for the first time.

**Why first:** the genre's single deepest lesson (LangSmith/AgentOps/MetaGPT `CostManager`) is that
metrics exist only when captured automatically per call. dowiz built the *best consumers* in the genre
(EV = p·v − (1−p)·c routing under a ruin cap, ½-Kelly lane width, served by hermes-kernel in 4–5 ms;
false-claim meter — nobody else measures honesty) and then fed them 9 hand-written rows. This phase
turns `gov_route` from aspirational to real, and every later phase (including H3's local-model EV
measurement) consumes its output.

**Tagging:** PATTERN (zero new dependencies — python stdlib or a small addition to hermes-kernel;
same source loop-signals already reads).

**Effort:** ~0.5–1 day (estimate). One harvest script + a hook point (loop-orchestrator HARVEST step
and/or the existing hermes cron pattern, cf. deep-clean-daily).

**Done criteria (falsifiable):**
1. **Volume:** after one harvest run over existing transcripts,
   `wc -l tools/telemetry/governance/track_record.jsonl` ≥ 100 (baseline 9, verified 2026-07-16);
   every harvested row has non-null `{ts, model, task, success, value, cost}`.
2. **Ground truth:** for one named session file, the harvested token totals are reproduced by an
   independent `grep`/`jq` over that same `~/.claude/projects/*/*.jsonl` file — numbers match exactly.
3. **Idempotence:** running the harvest twice back-to-back leaves the row count unchanged
   (dedup key = session/turn identity, not wall-clock).
4. **Consumer flip test:** injecting one synthetic row (model X, success=0, high cost) flips
   `gov_route`'s survivor away from X; removing it flips the route back. (Proves the ledger actually
   drives routing, not just accumulates.)
5. **Correlation:** a single `run_id` query over `agent_turn.jsonl` returns every dispatch row of one
   swarm wave — the join that today does not exist.
6. **Negative scope:** the harvester makes zero LLM calls and zero network calls; `TELEMETRY_NO_TG=1`
   is honored end-to-end.

### H2 — Telegram notification coverage: dowiz · openbebop · hermes

**What:** every one of the three operator-named repos can post a notification to the operator's
Telegram group (`-1003901655568`) with per-repo topic routing — each via its **own** sender
(recommendation and reasoning in §3; short form: sovereign-per-repo, shared *contract* not shared
*code*).

- **dowiz — done, becomes the reference.** `tg_send`/`tg_spool`/`tg_deliver` in
  `tools/telemetry/lib.sh` (spool queue, flock-atomic `TG_MIN_GAP` throttle, 6-attempt 429-aware
  retry honoring `retry_after`, topic routing via `TELEGRAM_TOPIC_ID`, token never logged, default
  chat `-1003901655568`). No code change; its behavior is written down as the contract.
- **openbebop (`/root/bebop-repo`) — generalize what exists.** `scripts/notify-telegram.mjs` is a
  working sender with a hard-coded report baked in. Replace with `scripts/tg-notify.mjs`: message
  from argv/stdin, env `TG_BOT_TOKEN`/`TG_CHAT_ID`/optional `TG_TOPIC_ID`
  (`message_thread_id`), bounded 429-aware retry, exit 2 + print-report-to-stdout fallback when
  unconfigured (all behaviors the current script already half-has). Node stdlib `fetch` only.
  **Do NOT touch `bebop2/ports/telegram/`** — that is DK-02, a product-layer `wasm32-wasip2`
  NotificationPort with zero-ambient-authority guarantees; dev-notification must stay out of the
  protocol surface.
- **hermes (`/root/hermes-agent-kernel-rewrite`) — config, not code.** Hermes ships a full native
  Telegram platform adapter (`plugins/platforms/telegram/`, python-telegram-bot) with cron delivery
  (`.env.example:336–348`: `TELEGRAM_BOT_TOKEN`, `TELEGRAM_HOME_CHANNEL`,
  `TELEGRAM_CRON_THREAD_ID`). Coverage = setting those envs in the operator deployment so hermes
  cron/agent output lands in the group. Adding a parallel shell sender would duplicate a
  production-grade adapter the repo already maintains.

**Tagging:** PATTERN (bebop: generalize an existing zero-dep script; hermes: configuration;
dowiz: documentation of existing behavior).

**Effort:** ~0.5 day total (estimate): bebop script ~1–2 h, hermes env wiring ~30 min, contract
paragraph ~30 min. Parallel-safe with H1 (disjoint repos).

**Done criteria (falsifiable):**
1. **bebop send:** `TG_BOT_TOKEN=… TG_CHAT_ID=… node scripts/tg-notify.mjs "bebop test $(date -u +%s)"`
   exits 0 **only** on `"ok":true` from the Telegram API, and the message is visible in the group;
   with envs unset it exits 2 and prints the message to stdout (never a silent drop).
2. **bebop hygiene:** `git diff` for the script shows zero additions to `package.json` dependencies;
   `grep -c "shipped to main" scripts/tg-notify.mjs` = 0 (no baked-in report text); the token never
   appears in any output path.
3. **hermes delivery:** one existing hermes cron job (e.g. deep-clean-daily) is observed delivering
   into the configured topic (`TELEGRAM_CRON_THREAD_ID`) — a dated message in the group is the proof
   artifact.
4. **Sovereignty check:** `grep -rn "/root/dowiz" <bebop notification script> <hermes telegram config>`
   returns nothing — no repo's notification path references another repo's checkout.
5. **Topic separation:** three test messages (one per repo) land in the group with distinguishable
   topic/thread routing, so the operator can mute per-repo.

### H3 — `LlmBackend` Trait-as-Port + un-gated llama.cpp Tier-1 (operator-gated)

**What** (LLM-infra research §5, verdict triple-confirmed by HARNESS session agents + SELF-CRITIQUE
§3 live probes — settled fact, not re-litigated here):

1. **Port:** `ports/llm.rs` — `trait LlmBackend { id, caps, chat, embed, rerank, health }`, kernel
   sees only `&dyn LlmBackend`; adapters live in a separate crate the kernel never imports
   (compile-firewall).
2. **One transport, three adapters:** a single `OpenAiCompatTransport` with per-backend `Quirks`
   (the Hermes-proven pattern, `plugins/model-providers/custom/`):
   - `ManagedApiAdapter` — headroom proxy → OpenRouter. **Tier 0, DEFAULT, live now.**
   - `LlamaCppAdapter` — `127.0.0.1:8080`, llama-server as a single static binary + systemd unit
     (S1-native, zero OCI). **Tier 1 — operator-unlockable NOW** (gate = DECART report + operator
     go; **no GPU condition** — the E13-cpu gate per LLM-INFRA §4.3).
   - `VllmAdapter` — local GPU or Modal H100 burst. **Tier 2 — stays gated on real GPU-unlock.**
3. **Hub choice:** selected per-hub via `HubPolicy.llm_backend` / EnvFile `LLM_BACKEND=` +
   `LLM_BASE_URL=` — **swap is config, never a kernel change**. This is M5 ("every hub may use any
   models/API at its discretion") made real for the first time; no dev-time gate may block a
   runtime hub from switching (SCOPE RULE).
4. **Backend-independent LOCKs bind regardless:** F3/F27 sha3 verify-or-deny on GGUF weights;
   F6/E19 `TokenBucket`/`Budget` ceiling with typed refusal; M8 local-only telemetry (llama-server
   `/metrics` never leaves the host); M12 red-line scopes unchanged; E21 honest `Err` when the
   backend is absent.
5. **First Tier-1 consumers (value, not capability theater):**
   (a) VERIFIABLE-COGNITION §3.3 semantic-leakage gate — deferred *solely* for lack of an embeddings
   bridge; llama-server `/v1/embeddings` is that bridge, at $0/call;
   (b) sovereign advisory judge — `eval-layer/openrouter_judge.py` already honors
   `OPENAI_BASE_URL`; point it at llama-server (advisory-only per VC §3.5 — never the gate).

**Tagging:** DECART (new binary + model weights at a trust-relevant surface → dated DECART report +
operator go BEFORE adoption, per the rust-native rule). The port/trait itself is PATTERN.

**Effort:** ~2–4 days (estimate) after operator go: trait + transport + 3 adapters + systemd unit +
GGUF manifest path + the two consumers. Realistic host expectation (verified class numbers):
~10–20 tok/s on 8B Q4, ~15–30 tok/s on 3–4B Q4 — single-agent workloads, not high-concurrency.

**Done criteria (falsifiable):**
0. **Gate first:** a dated DECART report exists and the operator's go is recorded before any binary
   or weights are fetched. (This phase does not self-start.)
1. **Compile firewall:** `cargo tree -p <kernel-crate>` shows no HTTP client and no adapter crate;
   `grep -rn "reqwest\|llama\|openai" kernel/src/` matches only the trait/port definitions.
2. **Verify-or-deny:** loading a GGUF whose sha3 differs from its manifest entry is refused with a
   typed error — a test plants a corrupted blob and proves the deny path RED→GREEN.
3. **Swap-is-config:** flipping `LLM_BACKEND=managed` → `llamacpp` in the EnvFile changes the live
   backend with zero recompile (same binary hash before/after); `id()` reflects the new backend.
4. **Honest absence:** with llama-server stopped, `health()` and `embed()` return typed `Err`
   (E21) — never a mock, never a silent fallback to the managed tier.
5. **Consumer (a):** the §3.3 leakage gate rejects a planted near-duplicate eval item
   (cosine > 0.9 via local `/v1/embeddings`); unplugging llama-server makes the gate fail CLOSED
   (typed `Err`), not pass silently.
6. **Consumer (b):** one eval-layer run completes with `OPENAI_BASE_URL=http://127.0.0.1:8080/v1`
   at $0 marginal API cost, and its output is labeled advisory (no gate consumes it).
7. **Budget:** an over-budget local call is refused with a typed refusal (F6/E19), proven by a test
   that exhausts the `TokenBucket`.
8. **EV loop closure (ties back to H1):** the local backend's calls appear as harvested
   `track_record.jsonl` rows (cost = CPU-time/token proxy), so `gov_route` can price
   local-vs-managed on measured data, not vibes.

---

## 3. Telegram cross-repo recommendation (the decision, with reasoning)

**Recommendation: sovereign-per-repo senders sharing a one-page CONTRACT — reject the cross-repo
call into dowiz's `lib.sh`.** Not "both options are fine": the cross-repo option loses on every axis
this project already ruled on.

Why the cross-repo call loses:

1. **Path coupling = central SPOF shape.** Sourcing `/root/dowiz/tools/telemetry/lib.sh` from bebop
   and hermes makes two repos silently depend on a third repo's checkout path, its `.env` loading,
   and its `/tmp` spool state. That is exactly the central-dependency topology the canon already
   rejected (C4 "PER-HUB REPLICATED — no central SPOF", M6 zero-dep at trust boundaries, the whole
   mesh-foundation pivot). Adopting it for notifications-of-all-things would be the house bias
   inverted for convenience.
2. **Language mismatch makes the "reuse" fake.** `lib.sh` is bash sourcing dowiz's `.env`; bebop is
   Node/Rust, hermes is Python. The cross-repo call would be `bash -c 'source …; tg_send …'` shelled
   out of foreign runtimes — all of the coupling, none of the integration.
3. **The cost of independence is nearly zero.** bebop already has 80% of a sender
   (`scripts/notify-telegram.mjs` — it needs generalizing, not writing); hermes already has a
   *production-grade* Telegram platform adapter + cron delivery and needs only env configuration.
   Total new code across all three repos: one ~60–80-line Node script.
4. **Independent failure domains.** A dowiz refactor, a moved checkout, or a spool-dir permission
   change must not be able to silence bebop's or hermes's notifications.

What IS shared (deliberately): the **contract, not the code** — env-var names
(`TG_BOT_TOKEN`/`TELEGRAM_BOT_TOKEN`, chat `-1003901655568`, per-repo topic ID), and four behavior
invariants every sender must satisfy: (i) token never logged; (ii) bounded retry honoring Telegram's
`retry_after`; (iii) unconfigured ⇒ non-zero exit + message printed to stdout, never a silent drop;
(iv) rate-respecting (dowiz's `TG_MIN_GAP` throttle is the reference). dowiz's `lib.sh` is the
richest implementation and serves as the contract's normative example. This mirrors the project's
own protocol-not-platform ethos: interoperate on the wire (same bot, same group, same invariants),
stay sovereign in the implementation.

---

## 4. Exact roadmap documents/lines this plan corrects (for a future amend pass — NOT edited here)

All corrections stem from H3's now-settled finding (E13 CPU tier was never GPU-dependent). The amend
pass belongs in the operator's canon merge (BLUEPRINT-P02's canon-diff is the natural vehicle).

| # | File | Location | What to amend |
|---|---|---|---|
| 1 | `/root/dowiz/docs/design/ARCHITECTURE.md` | line 34 | "managed-advisory until GPU-unlock" → "managed-advisory default; **E13-cpu (llama.cpp CPU tier) unlockable by operator DECART now — no GPU condition**; E13-gpu (vLLM/Modal) until GPU-unlock". |
| 2 | `/root/dowiz/docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P05-routing-organism-wiring.md` | §8, lines 253–262 (the "gated on **GPU-unlock**" corollary at line 256) | Split the corollary: E13-cpu gate = egress (verified satisfied 2026-07-16) + DECART + operator go; only E13-gpu remains on GPU-unlock. |
| 3 | `/root/dowiz/docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P15-living-organism-unbounded.md` | §9 lines 311–336 (incl. line 333 "gated on O18") and §10 acceptance item at line 369 ("execution remains gated on O18 / Phase 17") | Re-scope: the §9 port design survives unchanged; only the activation trigger splits into E13-cpu (operator-gated now) vs E13-gpu (O18). Acceptance item 10 must stop asserting O18 for the CPU tier. |
| 4 | `/root/dowiz/docs/design/sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md` | lines 81 (Phase-5 row: "self-host llama.cpp/vLLM execution is gated on GPU-unlock"), 91 (Phase-15 row: "E13-execution additionally on GPU-unlock"), 105 & 107 (dependency edges "[+GPU-unlock for E13-exec]" / "[+GPU-unlock, operator]"), 162 (the O18 row itself) | Split **O18** into two triggers per SELF-CRITIQUE §3: `graphics-unlock` (= `cargo add wgpu` reachable; today **red**, 403) gating P17 wgpu/video/F14/E23, and `model-weights-unlock` (= GGUF fetched + runnable llama-server; today **green-ish** — server on disk, HF → 200) gating E13-cpu. Rewire lines 81/91/105/107 to the correct trigger each. |
| 5 | `/root/dowiz/docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` | lines 120, 138 ("+GPU-unlock" on P17 — correct, keep), line 169 (O10–O18 operator-decision row) | Reflect the O18 split in the operator-decision table; P17's own +GPU-unlock stays. |
| 6 | Stale vLLM descriptor (same merge, cosmetic) | `docs/design/STRATEGIC-VECTORS-LOCKED-2026-07-16.md:80`, `docs/design/ecosystem-strategy/RESEARCH-CONSPECT.md:18`, `docs/design/ecosystem-strategy/ECOSYSTEM-STRATEGY-PLAN.md:52,143`, `docs/design/sovereign-roadmap-2026-07-16/OPEN-SOURCE-CREDITS-LIST.md:298` | "PagedAttention" was removed upstream in vLLM v0.25.0 (verified 2026-07-16); retire the descriptor to "V1 engine / Model Runner V2". Historical-context files may keep it with a dated note. |

Not a correction but a **feed**: H1 is a direct down-payment on R2 line 81's Phase-5 target
("`governance.sh` never calls `classify_complexity`/`rank_models`; lane width never fed by live
telemetry") — the auto-harvest supplies the live telemetry that row says is missing. A future P05
implementer should treat H1 as already-specified prerequisite work, not re-plan it.

---

## 5. What this plan does NOT cover (honest scope boundary)

- **P17's actual GPU/wgpu/video work is untouched.** Only the *LLM-serving* gating is corrected.
  `cargo add wgpu` is genuinely blocked (403, verified), the W21 wgpu offline-ceiling stands, and
  graphics-unlock remains a real external trigger. Nothing here schedules video, splats, F14
  topology viz, or webgpu.
- **vLLM Tier-2 stays gated.** vLLM CPU-only is a compatibility fallback that loses to llama.cpp on
  identical hardware (LLM-INFRA §2); its GPU gate is *correct* and is not loosened.
- **No model choice, no DECART verdict pre-empted.** H3 explicitly starts with the DECART report and
  the operator's go; this plan does not pick the GGUF model, fetch weights, or claim the gate will
  pass.
- **WANDR transfers are out of scope here.** The soft/hard hierarchical-F1 rollup and the
  `{item, source, excerpts, answer}` evidence-record port into `kernel/src/evals.rs` (LLM-INFRA
  §1.3) are recommended but separately schedulable — they touch the eval layer, not the harness, and
  deserve their own small task. WANDR-as-benchmark is rejected (LLM-judge-as-gate violates
  VERIFIED-BY-MATH / VC §3.5) — do not claim leaderboard comparability.
- **Harness research R3–R5 are follow-ons, not phases here:** the six-topology taxonomy page
  (pipeline / fan-out-fan-in / expert-pool / producer-reviewer / supervisor / hierarchical-delegation)
  + orchestrator CLASSIFY, the machine-checkable Owns-lanes diff gate, and the hard budget abort.
  R5 in particular depends on H1's live spend data — sequence it after H1 lands. The
  manifest-generator meta-skill (revfactory's one real trick) likewise waits.
- **No dependency adoption.** revfactory/harness, harness-100, claude-flow, LangSmith/AgentOps/OTel
  sinks all remain DECART-required, with likely-reject verdicts recorded in HARNESS-RESEARCH §5.
- **The §2 demand-gate meta-critique (G11 "first real order" as a Wave-0 gate) is not resolved
  here.** It is an operator-level charter question, orthogonal to these three phases; this plan
  neither depends on it nor prejudges it.
- **This plan edits no canon.** §4 is an amend list for the operator's canon merge; until that merge,
  P05/P15/R2/ARCHITECTURE read as they read today.

---

*Companion to: HARNESS-RESEARCH-revfactory-and-agentic-teams.md §4–5 (gap table, R1–R5),
LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md §4–5 (gating verdict, port design),
SELF-CRITIQUE-2Q-DOUBT-AUDIT.md §1.1/§3 (independent confirmation, O18 split),
BLUEPRINT-P02 (canon-diff vehicle), BLUEPRINT-P05 §8, BLUEPRINT-P15 §9, R2 §4 O18.*
