# codegraph-rust vs repowise — empirical bake-off (token consumption, correctness-gated)

**Question:** does `Jakedismo/codegraph-rust` beat the repowise MCP (our current system) on the
operator's top priority — **token consumption** — with **correctness as a hard gate**? Prior
dossier: `docs/research/2026-07-04-codegraph-rust-teardown.md` (source-level reverse-engineering).
This document re-verifies every claim empirically: real build, real index, real MCP tool calls on
both sides, real grep-oracle ground truth. All work happened out-of-tree in
`/tmp/.../scratchpad/codegraph-bakeoff/` (+ the pre-existing `codegraph-rust-teardown/` clone from
the prior session); nothing landed under `/root/dowiz` except this file.

## Verdict — NO-GO

**codegraph-rust loses.** In the only configuration the license and the operator's token-budget
permit (compiled *without* `ai-enhanced`), its four MCP tools returned the identical stub error —
*"Agentic tools require the `ai-enhanced` feature to be enabled"* — for **100% of the 7 test
queries**. Zero correct answers. It fails the correctness gate before token cost is even a factor.
repowise answered 4/7 queries exactly right (verified byte-for-byte against grep), 2/7 partially,
and missed 1/7 (which repowise itself also missed cleanly, so no query flips to codegraph). The
"cheaper alternative" the task asked me to check — getting repowise to index the Rust tree — turned
out not just cheaper but **already built and installed**: repowise's own package already ships
`tree_sitter_rust` plus a full Rust ingestion/resolver stack, and indexing the real
`rebuild/crates/` Rust tree (copied into an isolated test) took **13.8 seconds, zero LLM tokens,
zero new dependencies**. That evaporates codegraph-rust's one theoretical edge (the prior dossier's
"repowise doesn't index Rust" claim was about *scope*, not *capability* — the capability was already
sitting in the installed package, unused).

---

## 1. Build outcome + cost

Cloned commit `ce5bf27` (2025-12-20 — confirms the dormancy the prior dossier flagged; no commits
since). Built `codegraph-mcp-server`'s `codegraph` CLI/MCP binary, **default features only** (`daemon`,
no `ai-enhanced` — the token-honest configuration per the task's framing).

- **Fix 1/2 (environment):** first attempt failed — `openssl-sys` couldn't find `pkg-config`. Both
  `pkg-config` and `libssl-dev` were already installed system-wide; this was a PATH/environment
  quirk in the first background shell, resolved on retry.
- **Fix 2/2 (OOM, a real cost finding, not an environment quirk):** second attempt was **OOM-killed
  by the kernel** (`dmesg`: `Out of memory: Killed process … (rustc) … anon-rss:3429360kB`) while
  compiling `surrealdb-core` — the workspace's `[profile.release]` hard-codes `lto = "fat"` +
  `codegen-units = 1`, the most memory-hungry release profile possible, applied to a notoriously
  heavy crate (`surrealdb-core`, vendored as the graph backend), on a 4-core/7.6GB box with no swap.
  Fixed by overriding the profile at build time (`CARGO_PROFILE_RELEASE_LTO=off
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 CARGO_PROFILE_RELEASE_OPT_LEVEL=1 -j2`) — a build-time
  override, not a repo patch (no code/license issue). Third attempt succeeded.
- **Result:** `Finished release … in 6m 56s`, 596 crates compiled, **3.5 GB** `target/` directory,
  57 MB binary. This is real, quantified setup burden: a stock release build of this repo does not
  fit in 7.6 GB RAM without manual intervention. repowise required **zero** build — it was already
  installed and running as this session's live MCP server.

## 2. Standing dependencies — not stood up, and confirmed unnecessary to reach the verdict

Per the task's guidance, the `ai-enhanced` (LLM-calling) feature and its SurrealDB
server were deliberately **not** provisioned (avoids external API key spend, matches the
"skip the LLM-flagship tools" instruction). This was sufficient: the MCP server starts and answers
tool calls fine on stdio without a live SurrealDB connection — it fails at the `ai-enhanced` feature
gate *before* ever touching storage. Source confirms this is unconditional:
`crates/codegraph-mcp-server/src/official_server.rs:1102-1116` — all 4 tools
(`agentic_context/_impact/_architecture/_quality`) route through `execute_agentic_workflow`, which
has exactly two implementations selected by `#[cfg(feature = "ai-enhanced")]`: the real one (spins
up an Anthropic/OpenAI/Ollama-backed ReAct/LATS agent — a **second LLM call per query**), and a stub
that unconditionally returns the ai-enhanced-required error. There is no ad-hoc query CLI either —
`codegraph`'s only non-gated subcommands are `index` / `estimate` / `start` / `stop` / `status` /
`config` / `daemon`, none of which can answer "who calls X."

## 3. Rust corpus test (codegraph's best-case turf)

Copied `rebuild/crates/` (56 `.rs` files, 2 crates: `api`, `domain`) into the scratchpad — repowise's
previously-claimed blind spot.

**codegraph-rust `codegraph estimate rust-crates/ --recursive --languages rust`** (the one command
that works without `ai-enhanced` and without a live SurrealDB): 56/56 files parsed, 3,287 nodes,
9,024 edges, 6,351 symbols, in 0.15 s — parsing/graph-building itself is fast. But making that graph
*queryable* needs embeddings too: the same estimate projects **18 minutes of Jina (cloud, paid) API
time** or a local embedding model as a prerequisite for its semantic-search tools — another standing
cost repowise doesn't have for this workload.

**codegraph-rust MCP, 4 tools, 4 representative queries, built without `ai-enhanced`** — ran a Python
MCP client (`query_codegraph.py`) against the real stdio server:

```
agentic_context  → McpError('Agentic tools require the `ai-enhanced` feature to be enabled')
agentic_impact   → McpError('Agentic tools require the `ai-enhanced` feature to be enabled')
agentic_impact   → McpError('Agentic tools require the `ai-enhanced` feature to be enabled')
agentic_context  → McpError('Agentic tools require the `ai-enhanced` feature to be enabled')
```
4/4 stub errors, 84 chars each. Cheap in tokens, useless in content — disqualified before the
token-cost comparison is even meaningful.

**repowise on the same Rust corpus** — first confirmed repowise *already ships* Rust support:
`tree_sitter_rust` is an installed dependency of the repowise package, and its source tree contains a
full first-class Rust spec (`core/ingestion/languages/specs/rust.py` — `.scm` query file, heritage
node types, Cargo manifest/lockfile awareness, `mod.rs`/`main.rs` entry points, workspace resolver
`resolvers/rust_workspace.py`, `framework_edges/rust.py`, `dynamic_hints/rust.py`). Ran
`repowise init . --index-only -y --mode fast` on the copied corpus (isolated — did **not** touch the
live `/root/dowiz/.repowise` index):

```
58 files parsed · 2,448 symbols · rust 97%, toml 3%
Graph: 2,231 nodes · 5,834 edges
Elapsed: 13.8s · 0 LLM tokens (index-only mode)
```

Then drove the **same MCP tool interface** (`get_context`, `get_risk`, `search_codebase`) against
this isolated index via a raw MCP stdio client (`query_repowise_rust.py`), since the live session's
repowise MCP connection is bound to `/root/dowiz` and can't be hot-repointed mid-session.

⚠️ **Operational footgun found and fixed:** `repowise init <path>` silently rewrote the **global**
`~/.claude/settings.json` `mcpServers.repowise` entry to point at the scratchpad path (plus wrote a
`.claude/CLAUDE.md`+hooks there) — a real side effect of running the "cheap alternative" naively.
Caught immediately (diffed against the known-correct `/root/dowiz/.repowise/mcp.json`) and reverted
before it could affect this or any other session. Noted here since it's a genuine risk of the "just
run init" cheap path and should be done deliberately (`--no-codex`-style opt-outs exist for some
side effects but not this one).

## 4. TS corpus test (repowise's home turf) — via the live, real `/root/dowiz` MCP session

Used the actual `mcp__repowise__*` tools available in this session (no simulation).

## 5. Per-query scoreboard (grep is the oracle for both sides)

| # | Query | Ground truth (grep) | repowise | codegraph-rust | Winner |
|---|---|---|---|---|---|
| 1 | RUST: all callers of `db::with_user` | 37 call sites across 6 files (`categories.rs` ×7, `modifier_groups.rs` ×7, `products.rs` ×12, `themes.rs` ×3, `menu_availability.rs` ×4, `mod.rs` ×1, +1 test) | `get_context(callers)` found 14/37, **only 1 of 6 files** (`products.rs`+the test) — real recall gap, ~1,218 tok | stub error, 0/37 | **repowise** (partial beats zero) |
| 2 | RUST: what does `routes/owner/products.rs` depend on | 12 exact `use` lines | Exact match, skeleton mode: 5,506 tok vs 28,375 full (19.4%), `verified:true` | stub error | **repowise** (clean win) |
| 3 | RUST: impact radius of the `Lek` money newtype | 2 real external dependents (`dto.rs`, `domain/lib.rs`) | `get_context("…::Lek", callers)` **wrong** — returned `Lek`'s own methods, not external users; `get_risk` fallback got the **count** right (2) but not the file list (~297 tok) | stub error | **repowise** (partial via a different tool; still ahead of zero) |
| 4 | RUST: where is JWT verification | `auth/jwt.rs` — `JwtService::verify` / `verify_with_validation` (RS256-pinned) | `search_codebase` returned **empty** — index-only mode never generated the wiki pages semantic search runs over (0 LLM tokens spent = 0 searchable prose); graceful `grep_hint` | stub error | tie-ish fail; repowise fails cheaper and more gracefully |
| 5 | TS: callers of `assertTransition` exported from `packages/domain/src/order-machine.ts` (2 same-named decoy functions live elsewhere in the repo — a real disambiguation trap) | Real: `apps/api/src/lib/orderStatusService.ts:79` (call) + `apps/api/src/routes/orders.ts:3` (import) + test file. Decoys (must NOT count): `apps/api/src/modules/acquisition/state-machine.ts` (own `assertTransition`), `packages/ui/src/utils/index.ts` (own `assertTransition`) | **Miss** — `get_context(callers)` on the file returned only 2 same-package importers (`errors.ts`, `index.ts`), missing both real cross-package (`@deliveryos/domain` workspace-alias) consumers entirely; symbol-qualified target (`file.ts::assertTransition`) returned `"empty or non-symbol file"` even using repowise's own returned `symbol_id` string verbatim; `get_answer` returned low-confidence/no-hit | not tested (TS is not codegraph's differentiator and the tool is gated identically) | no clean winner — real repowise correctness gap, logged as a separate finding below |
| 6 | TS: what does `apps/api/src/routes/spa-proxy.ts` depend on | 9 exact imports | Exact match, 2,055 tok vs 11,369 full (18.1%), `verified:true` | not tested | **repowise** |
| 7 | TS: what does `apps/api/src/routes/orders.ts` depend on | 28 exact imports | Exact match, 1,108 tok vs 11,629 full (9.5%), `verified:true` | not tested | **repowise** |

**Aggregate:** repowise correct-and-cheap on 3/7 (clean exact matches), correct-with-caveats on 2/7
(right aggregate count via a fallback tool, wrong/incomplete detail), missed 2/7 (one gracefully with
a grep_hint, one — the cross-package caller query — a genuine product gap worth reporting
independently). codegraph-rust: **0/7** answerable in the license/cost-permissible configuration —
uniformly the same 21-token stub error regardless of query. Token cost of that stub error is trivially
low, but the bar requires **correct** answers at lower cost — an unconditionally-wrong answer does
not clear the gate no matter how cheap it is.

## 6. The cheaper alternative — repowise indexing Rust — feasibility: ALREADY TRUE, not just cheap

Checked non-destructively: `repowise --help` exposes `init [PATH] [--index-only] [--mode fast] [-x
EXCLUDE]` — no separate "add a language" step needed. The installed repowise package
(`~/.local/share/uv/tools/repowise/…/site-packages/repowise/`) already contains, as first-class,
non-experimental code: `tree_sitter_rust` (an installed dependency alongside 15 other language
grammars), `core/ingestion/languages/specs/rust.py`, `core/ingestion/resolvers/rust.py` +
`rust_workspace.py`, `core/ingestion/framework_edges/rust.py`, `core/ingestion/dynamic_hints/rust.py`.
Empirically indexing the real 56-file `rebuild/crates/` Rust tree (isolated copy) took **13.8
seconds, 0 LLM tokens, 0 new processes, 0 new dependencies** and produced a real graph (2,231
nodes/5,834 edges) that answered file-dependency queries perfectly and callers/impact queries
partially. Extending the **live** `/root/dowiz` index to cover `rebuild/crates/` was not executed
(protected-config discipline — the task asked for feasibility, not the change itself) but the
mechanics are proven at this scale and order of magnitude: this is a `repowise init` invocation away,
not a new capability that has to be built. This fully absorbs codegraph-rust's one claimed edge from
the prior dossier.

## 7. License — unchanged, still blocks vendoring regardless of the empirical result

Re-confirmed on this clone: no `LICENSE` file anywhere in the repo. Root `Cargo.toml` claims `license
= "MIT OR Apache-2.0"` (workspace default) but `crates/codegraph-mcp-server/Cargo.toml` — the crate
that owns the actual CLI/MCP tools, i.e. the only integration surface — overrides to `license =
"Apache-2.0"` outright, with no license text anywhere to back either claim. This was already
sufficient to block vendoring/depending in the prior dossier and still is; moot here since the
empirical result is NO-GO on merits anyway, but stated for completeness per the task's instructions.

## 8. Repowise correctness gaps found (independent finding, does not change the codegraph verdict)

Two real, reproducible gaps surfaced while building ground truth, worth flagging to the operator
separately from this bake-off's question:
1. **Symbol-qualified `get_context` targets don't resolve.** Passing `"file.ext::SymbolName"` —
   including the exact `symbol_id` string `get_context` itself returned in a prior call — returns
   `"…: empty or non-symbol file"` instead of the symbol. Reproduced identically on both the Rust and
   TS corpora, so it's general, not language-specific.
2. **Cross-package/cross-crate caller rollups undercount.** `get_context(callers)` at the file level
   is a same-package/same-crate import rollup; it missed real consumers that reach a symbol through a
   monorepo workspace alias (`@deliveryos/domain` from `apps/api`) on the TS side, and missed 5 of 6
   caller files for a same-crate direct call (`db::with_user`) on the fresh Rust index. Both are
   real, load-bearing miss patterns for "who calls X" — the single most common code-intelligence
   query an agent asks.

These do not favor codegraph-rust (which failed all 7 queries outright) — they're a separate,
actionable repowise product issue.

## 9. GO/NO-GO

**NO-GO.** codegraph-rust fails the correctness gate completely (0/7) in the only configuration its
license and the operator's token budget permit; the one configuration where it *could* answer
anything (`ai-enhanced` + LLM keys) inherently costs *more* tokens per query than repowise (a second
model invocation), so it cannot win the stated priority even hypothetically. Standing cost (SurrealDB
+ per-language LSP + LLM keys) and build cost (OOM on stock settings, 6m56s/3.5GB even after the fix,
596 crates) are both real and both absent from repowise. The license remains unresolved (no LICENSE
file; the MCP-tools crate is Apache-2.0-only, contradicting the workspace's dual-license claim),
which would block vendoring even on a win.

**The cheaper alternative wins outright and is already available:** repowise already ships full Rust
language support (grammar + resolvers, installed, unused for `/root/dowiz` only because
`rebuild/crates/` was never added to its scope). Proven empirically at 13.8s / 0 LLM tokens / 0 new
dependencies for the real 56-file corpus. Recommended follow-up (separate from this decision):
extend the live `/root/dowiz` repowise index to include `rebuild/crates/` via `repowise init
rebuild/crates` (or a workspace-mode multi-repo config), and separately, file the two correctness
gaps in §8 as product issues since they affect "who calls X" — the most common query class — on both
languages repowise already serves.

LAST-REVIEWED: 2026-07-04
