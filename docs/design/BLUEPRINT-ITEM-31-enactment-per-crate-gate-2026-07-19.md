# BLUEPRINT — Item 31 (enactment half): per-crate zero-dep gate + shared kernel JSON-parse primitive

Date: 2026-07-19 · Tier 2 (§C) · Author: planning agent (Fable) · Status: PLANNING ARTIFACT (no code changed)
Depends on (both CLOSED this session, read not re-derived):
- Items 1+13 — `scripts/zero-dep-gate.sh` + `kernel/ZERO-DEP-ALLOWLIST.txt` (now EMPTY: kernel default
  no-dev tree = zero external crates), CI job `zero-dep-gate` at `.github/workflows/ci.yml:271`
  (worktree `/root/dowiz-wt-space-grade-exec`, branch `exec/space-grade-tier0-2026-07-19`).
  Design doc: `BLUEPRINT-ITEMS-01-13-ci-zero-dep-gate-2026-07-19.md`.
- Item 25 — `PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md` (binding ten-step procedure).
- Item 31 investigative half — `AUDIT-ITEM-31-dependency-findings-2026-07-19.md`: `rusqlite`
  KEEP-and-contain (cat-2), `cosmic-text` pinned `0.19.0` (`c2d0f306a`, already applied),
  `sha2` KEEP (swap to kernel Keccak would ADD `aes-gcm`+`curve25519-dalek` via the `pq` feature).

Roadmap text (§C line 201): *"per-crate allowlist CI gate + shared kernel-side JSON-parse primitive
for the seven serde carriers + manifest-recorded rulings."* Synthesis item 31 proof clause (line 462):
allowlist exists and only shrinks (CI-asserted); a test diff adding an unlisted crate to ANY manifest
fails; rewrite-candidate rulings recorded in each manifest in the `slot_arena.rs` format; the
default-build count of crates carrying serde reported before/after and does not increase.

---

## 1. Workspace dependency inventory — the real baseline (every manifest read 2026-07-19)

**Correction to synthesis §25(a) first.** It states "the workspace holds 20 crates — kernel plus the
19 audited below." The live tree holds **26 crates**: the synthesis table missed all six
`tools/telemetry/*` crates. Two of the missed six carry `serde_json` unconditionally (see §3) — the
blind spot is material, not cosmetic. (`.worktrees/` copies and the skillspector `.venv` vendored
crates are not workspace crates and are excluded.)

Direct external `[dependencies]` only (dev-deps excluded — the gate's proof surface is `-e no-dev`,
matching items 1+13). Path deps are internal and listed separately. Every crate has its own
`Cargo.lock` (verified: 26/26 present), so Gate C parametrizes cleanly.

| Crate dir | Direct external deps (default build) | Path deps | Gate note |
|---|---|---|---|
| `kernel` | *(none — default no-dev tree is empty; optional: wasm-bindgen, serde, serde_json, serde_yaml, sqlx, tokio, aes-gcm, curve25519-dalek, wgpu, pollster, thunderdome)* | — | DONE (items 1+13/4+29/5); allowlist empty, shrink-only |
| `engine` | *(none default; optional: serde, cosmic-text =0.19.0 pinned)* | dowiz-kernel | zero-dep default; allowlist empty |
| `wasm` | wasm-bindgen =0.2.95, serde, serde_json | dowiz-engine (serde feature), dowiz-kernel | serde carrier |
| `apps/courier` | *(none)* | dowiz-engine | allowlist empty |
| `agent-adapters` | serde, serde_json; wasmtime 46 (optional, off-default) | dowiz-kernel | serde carrier |
| `agent-facade` | serde, serde_json | dowiz-kernel | serde carrier |
| `agent-loop` | *(none)* | agent-facade, llm-adapters | allowlist = closure via path deps (serde etc. arrive transitively — see §2.4) |
| `agent-governance-wasm` | wasm-bindgen | bebop2-core (**absolute path** `/root/bebop-repo/...`) | CI-ungateable as-is — §2.5 |
| `llm-adapters` | ureq (tls+json), serde, serde_json | dowiz-kernel | serde carrier |
| `mesh-adapter` | *(none)* | dowiz-kernel, bebop-delivery-domain, bebop-proto-cap (relative `../../bebop-repo/...`) | gate rides the existing `mesh-adapter` CI job — §2.5 |
| `tools/async-spool` | ureq (tls+json), serde, serde_json | — | serde carrier |
| `tools/ci-truth` | *(none)* | — | allowlist empty |
| `tools/deep-clean` | rusqlite 0.31 (bundled) | — | ruled KEEP-and-contain; allowlist = rusqlite closure |
| `tools/eqc-rs` | *(none)* | — | allowlist empty |
| `tools/native-spa-server` | axum 0.8, tokio (full), tower-http, hyper-util, tokio-rustls, rustls-pemfile, flate2, clap, base64, sha2, serde, serde_json | dowiz-kernel (json-api) | serde carrier; largest closure; `sha2` ruled KEEP |
| `tools/nfc-pod-codec` | *(none)* | dowiz-kernel (pq) | allowlist = kernel-pq closure (aes-gcm, curve25519-dalek, serde, serde_json arrive via the feature) |
| `tools/nfc-pod-flipper` | flipperzero, flipperzero-sys, flipperzero-rt (0.16.0) | — | device-SDK boundary; frozen set |
| `tools/ops-alert` | *(none)* | — | allowlist empty |
| `tools/shell-spike` | *(none)* | dowiz-engine | allowlist empty |
| `tools/skillspector-rs` | regex, serde, serde_json | — | serde carrier; `regex` is a named rewrite candidate (synthesis §25 row) — separate from this item |
| `tools/telemetry/hetzner-exporter` | *(none)* | — | **missed by synthesis table**; allowlist empty |
| `tools/telemetry/native-ser` | *(none)* | — | missed by synthesis table; allowlist empty |
| `tools/telemetry/native-trackers` | *(none)* | — | missed by synthesis table; allowlist empty |
| `tools/telemetry/rust-spool` | ureq (tls+json), serde, serde_json | — | missed by synthesis table; serde carrier; **superseded** — `tools/async-spool`'s own manifest declares itself "the generalized successor to tools/telemetry/rust-spool" |
| `tools/telemetry/swarm-proof` | *(none)* | — | missed by synthesis table; allowlist empty |
| `tools/telemetry/topics` | ureq (json), serde_json | — | missed by synthesis table; serde_json carrier (no serde) |

Baseline summary: **12 of 26 crates are already zero-external-dep in their default build** (empty
allowlists — the gate freezes that as a floor). The full transitive-closure name lists per crate are
generated at execution time by the gate's own pipeline (§2.3), not hand-transcribed here — the
item-1+13 precedent (blueprint §4 step 6) is to regenerate from the pipeline, and hand-copied lists
would go stale against 26 lockfiles.

## 2. Per-crate gate design — extend items 1+13 exactly, invent nothing

Items 1+13's blueprint **pre-decided the schema** (§2.1): *"One file, next to the manifest it
governs (so item 31's later per-crate extension is 'add `<crate>/ZERO-DEP-ALLOWLIST.txt`', not a
schema change)."* We follow it: **one allowlist file per crate directory, same format** (names only,
`#` comments, shrink-only), **one parametrized script**, one roster. No consolidated manifest — a
single consolidated file would break Gate B's per-file `git show origin/main:` diff semantics and
the "next to the manifest it governs" locality that makes rulings discoverable.

### 2.1 Script change — `scripts/zero-dep-gate.sh [<crate-dir>]`

Backward-compatible parametrization (no-arg = `kernel`, so the existing CI line keeps working):

```bash
CRATE="${1:-kernel}"                      # crate directory relative to repo root
ALLOW="$CRATE/ZERO-DEP-ALLOWLIST.txt"
LOCK="$CRATE/Cargo.lock"                  # Gate C hashes this (all 26 crates have one — verified)
# tree pipeline: filter path deps BEFORE awk strips the "(...)" marker
cargo tree --manifest-path "$CRATE/Cargo.toml" -e no-dev --locked --offline --prefix none \
  > /tmp/zdg-raw.txt
{ grep -v ' (/' /tmp/zdg-raw.txt || true; } | awk '{print $1}' | sort -u > /tmp/zdg-actual.txt
```

The **one substantive mechanical change** is the root/path-dep filter. The current script drops only
the literal name `dowiz-kernel` (`zero-dep-gate.sh:27`). Per-crate, internal path deps
(`dowiz-kernel`, `dowiz-engine`, `agent-facade`, `llm-adapters`, `bebop2-core`, …) appear in
consumers' trees and are not external crates. Hardcoding a name list would rot; instead: with
`--prefix none`, `cargo tree` renders every **path** dependency (and the root) as
`name vX.Y.Z (/abs/path)` and every registry crate as `name vX.Y.Z` — so `grep -v ' (/'` removes the
root and all in-tree/sibling path deps in one principled step. `(proc-macro)`/`(*)` markers don't
match ` (/` and pass through to awk unchanged. The `|| true` guard preserves the item-5 fix (empty
result must report empty, not abort under pipefail). Gates A/B/C logic is otherwise **unchanged**,
just `$ALLOW`/`$LOCK`-parametrized; Gate B's first-commit vacuous-green already handles the 25 new
allowlist files landing.

Feature-resolution note (state in the script header): externals arriving **through a path dep's
feature** belong in the consumer's allowlist — e.g. `serde`/`serde_json` appear in
`native-spa-server`'s tree via `dowiz-kernel (json-api)` and in `nfc-pod-codec`'s via
`dowiz-kernel (pq)`. That is correct per-consumer truth (those builds genuinely link them) and does
not conflict with the kernel's own empty allowlist — different proof surfaces, different feature
resolutions.

### 2.2 Roster — `scripts/zero-dep-crates.txt` (new)

One crate dir per line, `#` comments. Contents: the 24 in-repo-resolvable crates of §1 (kernel
included — the existing kernel invocation folds into the loop). Two exclusions, recorded IN the
roster as comments with reasons (§2.5).

### 2.3 Allowlist generation + file headers

Each `<crate>/ZERO-DEP-ALLOWLIST.txt` is generated by the §2.1 pipeline at enactment time (exact
command recorded in each file's header), then frozen. Header states the crate's ruled terminal state
per the item-25 procedure, e.g.:
- `tools/deep-clean`: "rusqlite = cat-2 foreign-format boundary, KEEP-and-contain, never extend —
  AUDIT-ITEM-31 §(a), 2026-07-19."
- `tools/native-spa-server`: "sha2 = KEEP (kernel-Keccak swap costs more deps than it saves —
  AUDIT-ITEM-31 §(c)); axum/tokio stack = §13(c) cat-1+3 composition root, surface-minimization
  target only."
- Zero-dep crates: the kernel's "no external crates permitted" closing line, verbatim pattern.

Names-only + full transitive closure, exactly as items 1+13 argued (version bumps don't churn the
file; a new transitive crate through `cargo update` must go RED). Honest cost: `native-spa-server`'s
closure is large (order 10² names). That is the true dependency surface being frozen; hiding it
would defeat the gate.

### 2.4 Gate semantics per crate class

Same three gates everywhere; what differs is the fixed point:
- **kernel**: allowlist EMPTY, contract = zero (unchanged).
- **zero-dep crates (12)**: allowlist empty from day one — the gate turns their current cleanliness
  into an enforced floor (today nothing stops a dep landing in `eqc-rs`).
- **boundary/carrier crates**: allowlist = today's frozen closure, shrink-only. The serde-primitive
  work (§4) then SHRINKS lists — Gate B makes each shrink permanent for free, which is precisely
  the synthesis proof clause "count … does not increase" made mechanical.

### 2.5 Exclusions (recorded, not silent)

- `agent-governance-wasm` — depends on bebop2-core by **absolute path** (`/root/dowiz/
  agent-governance-wasm/Cargo.toml:15`: `path = "/root/bebop-repo/bebop2/core"`); unresolvable on
  any runner layout. EXCLUDE from the CI loop; flag the absolute-path manifest as a portability
  defect (own small follow-up ticket — out of item-31 scope to fix).
- `mesh-adapter` — relative `../../bebop-repo/...` paths resolve only in the dual-checkout layout
  the existing `mesh-adapter` CI job (`ci.yml:368-382`) already builds (dowiz under `dowiz/`, bebop
  under `bebop-repo/`). Enactment adds one `zero-dep-gate.sh` invocation **inside that job** (cwd
  `dowiz/`), rather than into the main loop. Its external set is empty today (path deps only), so
  the allowlist is empty — cheap and worth gating.

### 2.6 CI wiring + red-proofs

Extend the existing `zero-dep-gate` job (worktree `ci.yml:271`) — do not add a new job:
- Pre-namespace fetch: loop `cargo fetch --locked --manifest-path "$c/Cargo.toml"` over the roster.
- Gate step: single `sudo -E env "PATH=$PATH" unshare -n bash -c` invocation looping
  `bash scripts/zero-dep-gate.sh "$c"` over the roster (fail-fast; `set -euo pipefail` inherited).
  One job, no matrix — `cargo tree` is metadata-only, the whole loop is seconds.
Red-proofs at enactment (mirror items 1+13 §2.4): (1) throwaway branch adds a dummy dep to ONE tool
manifest → RED (this is the synthesis's literal proof clause "a test diff adding an unlisted crate
to any manifest demonstrably fails"); (2) throwaway growth of one allowlist → RED; (3) green run
prints per-crate counts + stable lock hashes; (4) `nfc-pod-flipper`/`wasm` resolve offline on the
runner cache (verify once — both have lockfiles and have built offline before).

Adjacent, untouched: `deny.toml`/`supply-chain` (licenses/advisories; runs kernel+engine only),
`decart-dep-lint` (direct-dep diff prose gate), `firewall-agent-loop` — complementary layers, per
the items-1+13 scope rule.

## 3. The "seven serde carriers" — real count: NINE (not forced to seven)

Synthesis §25(b) claims the serde/serde_json pair is "carried unconditionally by **seven** crates":
agent-adapters, agent-facade, llm-adapters, wasm, async-spool, native-spa-server, skillspector-rs.
Those seven are real and confirmed. But the count inherits the §25(a) table's blind spot (§1): the
live tree has **eight crates carrying the pair unconditionally** (the seven + `tools/telemetry/
rust-spool`) **plus one carrying `serde_json` alone** (`tools/telemetry/topics`) = **nine
unconditional serde_json carriers**. Kernel and engine carry serde only behind opt-in features —
correctly not counted.

Actual parse-side call sites (`serde_json::from_str/from_slice/from_value/from_reader`), per carrier:

| Carrier | Sites | What is parsed | Trust class |
|---|---|---|---|
| `agent-adapters` | 4 (dispatch.rs, mcp.rs, transport.rs ×2) | JSON-RPC 2.0 / MCP protocol frames | foreign protocol, arbitrary nesting |
| `agent-facade` | 1 (lib.rs:84) | tool-call args | LLM-originated, bounded schema |
| `llm-adapters` | 1 (dispatch.rs) + ureq `send_json`/`into_json` ×4 (transport.rs:119-169) | LLM API responses | foreign network, drifting schemas |
| `wasm` | 5 (lib.rs:143 SpineDoc array, :490 SemanticScene; 3 test-only) | JS-embedder-supplied docs | trusted embedder, fixed schema |
| `tools/async-spool` | 1 + ureq `send_json`/`into_json` (main.rs:286-310) | own spool envelope `{dest,…}` + passthrough payload | self-owned format |
| `tools/native-spa-server` | 7 (api.rs) | **HTTP request bodies, capability frames** | attacker-controlled |
| `tools/skillspector-rs` | 1 (main.rs:75 → `Value`) | JSON-RPC serve-mode stdin lines | local dev tooling |
| `tools/telemetry/rust-spool` | 1 | Telegram spool | superseded by async-spool |
| `tools/telemetry/topics` | 1 | Telegram API response | dev tooling |

(Kernel's own 30 from_* sites — json_api.rs, wasm.rs, evals.rs, living_knowledge.rs, pq tests — are
all feature-gated out of the default build; not carriers.)

**Consolidation finding (flag, don't force):** `rust-spool` is declared superseded by its successor's
own manifest and is referenced by no script/workflow (grepped `scripts/`, `.github/workflows/` —
zero hits). Under the delete-dead-legacy discipline, the enactment should propose **deletion** of
`rust-spool` (and evaluate `topics`) after verifying no host-side systemd/cron unit invokes the
binaries — deletion, not primitive-cutover, is the correct serde-count reduction for these two.

## 4. Shared kernel JSON-parse primitive — honest scope assessment

### 4.1 `kernel::fdr::json` is NOT the home — by its own charter

The module built this session is the **write** authority only, and says so explicitly
(`kernel/src/fdr/json.rs:12-13`): *"Scope: this is the \*serialize\* side only. Parse-side JSON
(`json_api.rs`, the serde carriers) is item 31's scope and is untouched."* Its escaper is
deliberately minimal (5 escapes, `esc()`-byte-compatible, golden-pinned) and its `JsonWriter` is a
fixed-schema object builder. Bolting a parser onto it would break the golden-pinned narrowness that
is its whole value. **Ruling: the parse primitive gets its own module — `kernel/src/json/`
(proposed: `json::value` with a bounded `Value` enum + `json::parse`), always-compiled, pure `std`.**
It hand-rolls, so the kernel's empty allowlist stays empty; `serde_json` is added as a
**dev-dependency differential oracle** — dev-deps sit outside the `-e no-dev` proof surface, so the
oracle costs the zero-dep gate nothing. (Whether `fdr::json`'s writer later moves under `kernel::
json` as `json::write` is cosmetic; do not churn it in this item.)

### 4.2 The synthesis's complexity claim, corrected

Synthesis §25(b): a parse-side hand-roll is "bounded (a JSON parser is well inside the Keccak
complexity class)." **True for algorithmic size, wrong as a risk statement.** Keccak is verified
against a *complete, closed* oracle (NIST KAT vectors); a JSON parser at a trust boundary faces an
*adversarial, unbounded* input space — deep-nesting stack exhaustion, `\uXXXX` surrogate pairs,
number-grammar edge cases, duplicate keys, invalid UTF-8 — and the incumbent (`serde_json`) has a
decade of fuzz hardening. Replacing it **at the attacker-facing HTTP boundary** would be a security
regression until the hand-roll has carried equivalent differential-fuzz load. The roadmap's one-line
framing under-states this; the blueprint says so plainly.

### 4.3 The tree-win analysis — where cutover actually shrinks anything (verified with `cargo tree -i`)

- `native-spa-server`: `serde_json v1.0.150` is pulled by **axum 0.8.9** (default `json` feature)
  AND `dowiz-kernel (json-api)` AND directly (inverse tree run 2026-07-19). Removing the direct dep
  removes **nothing** from the tree.
- `llm-adapters`: pulled by **ureq 2.12.1** (`json` feature — and its `send_json`/`into_json`
  helpers are actively used) AND directly. Same for `async-spool`, `rust-spool`, `topics`. Removing
  direct serde without dropping ureq's `json` feature removes **nothing**.
- Genuine tree shrink is possible in exactly **four** carriers: `agent-facade`, `agent-adapters`,
  `wasm`, `skillspector-rs`.

### 4.4 Scoped-down plan (the honest replacement for "one primitive replaces seven copies")

**Phase A — build the primitive, cut over the safe three.** Per item-25 step 4, `kernel::json` is
brought to compile-checked, test-passing state BEFORE any carrier ruling: bounded recursive-descent
RFC 8259 parser (explicit max depth + max input length, `Result` degrade-closed, never panics),
differential-tested against the serde_json dev-oracle (proptest + adversarial corpus incl. nesting/
surrogates/numbers), round-trip-tested against `fdr::json`'s writer. Then cut over the fixed-schema,
non-attacker-facing carriers: **`wasm`** (2 prod sites, trusted embedder), **`agent-facade`** (1
site; malformed → `ToolError::BadArg`, already degrade-closed), **`skillspector-rs`** (1 site, local
tooling; needs the generic `Value`). Each cutover shrinks that crate's allowlist (Gate B locks it)
and drops `serde`+`serde_json` from its manifest. Before/after serde-carrier count: **9 → 6**
(satisfying the synthesis's "does not increase" clause with an actual decrease).

**Phase B — explicitly DEFERRED, with named reopening triggers (item-25 step 10):**
- `native-spa-server`: attacker-facing + zero tree win (axum retains serde_json). Reopen only if
  axum is ever configured `default-features = false` without `json` — until then serde_json stays
  ruled KEEP as part of the composition-root boundary.
- `llm-adapters` / `async-spool`: zero tree win via ureq `json` + foreign drifting schemas. Reopen
  if the transport ever drops ureq's json helpers.
- `agent-adapters`: real tree win but the largest parse surface (full JSON-RPC 2.0/MCP, arbitrary
  nesting, foreign peers). Reopen after the Phase-A primitive has carried real load with its
  differential-fuzz corpus green — the `master_seed()` never-silently-replace discipline applied to
  a parser.
- `rust-spool` / `topics`: not cutover targets — the §3 deletion/consolidation question instead.

Every Phase-A/B ruling is recorded in the crate's manifest in the `slot_arena.rs` format (item-25
step 9), alongside the §2.3 allowlist headers.

## 5. Execution order + proof map

1. Land §2 (script parametrization + roster + 24 generated allowlists + mesh-adapter job hook +
   red-proofs). This alone discharges the gate half of the synthesis proof clause.
2. Add the three manifest ruling comments carried forward from the audit (rusqlite / sha2 /
   cosmic-text) in the same change as their allowlist headers.
3. Phase A of §4 (primitive + three cutovers) as its own change; allowlist shrinks ride Gate B.
4. File the flagged follow-ups, separately: `agent-governance-wasm` absolute-path dep;
   `rust-spool`/`topics` deletion check; (pre-existing, reaffirmed) dual-Keccak dedup.

| Synthesis item-31 proof clause | Discharged by |
|---|---|
| allowlist exists, only shrinks, CI-asserted | §2.1 Gates A+B per crate, roster loop §2.6 |
| unlisted crate in ANY manifest → demonstrable fail | §2.6 red-proof (1) |
| rulings in manifests, slot_arena.rs format | §2.3 headers + §4.4 ruling comments |
| serde-carrying crate count before/after, non-increasing | §3 baseline 9 → §4.4 Phase A target 6 |
