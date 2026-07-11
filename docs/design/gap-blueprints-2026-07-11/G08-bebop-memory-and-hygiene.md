# G08 — bebop-repo Living Memory + Cross-Repo Hygiene + WIP Protection

> Gap blueprint, 2026-07-11. Research + execution plan only — **no file in either repo was
> modified** producing this document; both working trees left exactly as found. Sources: audit
> `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md` (§1 risk 5, §2.2, §6.1, §6.5, §7.6,
> §9 rec 6), full bebop git log (104 commits on HEAD / 110 across refs), `.review/` artifacts,
> both repos' hooks, the dowiz memory corpus (152 files), and the dowiz-living-memory skill.
> Every claim below re-verified live 2026-07-11 ~16:40Z unless marked (audit).

---

## 1. Gap & evidence

**G08: the discipline that makes dowiz auditable does not exist for the repo getting all the
attention.** Four sub-gaps, all verified live:

| # | Sub-gap | Evidence (verified 2026-07-11) |
|---|---------|-------------------------------|
| a | **Zero living memory for bebop-repo** | `/root/.claude/projects/-root-bebop-repo/` has **no `memory/` directory at all** and 27 session transcripts (16 mtime 07-10 10:45–21:10, 11 mtime 07-11 03:13–16:14, ~8.7 MB). bebop's own README line 24 advertises "**Living memory**" as a headline feature. The dowiz corpus's newest entry is 07-10 12:24 (`cross-branch-todo-map-2026-07-10.md`) — the entire bebop2 crypto pivot is invisible to memory on both sides. Root cause found: `/root/bebop-repo/.claude/settings.json` **denies `Edit`/`Write` outright** for every bebop session — an agent working in bebop *cannot* write memory files even if it wants to. The gap is structural, not behavioral. |
| b | **Cross-repo detritus recurred within 24h of being "fixed"** | Todo-map T1 was closed "DONE 2026-07-10" (relocation commit `4fa0839`, 07-10 12:26). By 07-10 17:02–07-11 09:28, **12** bebop-topic files were back in `/root/dowiz`: 11 untracked (`chacha.pdf`, `poly1305.pdf`, `xsalsa.pdf`, `xchacha2.pdf`, `xchacha_draft.html`, `crypto-primitives-research.md`, `hybrid-routing-sota.md`, `platform-vs-protocol-logistics.md`, `web3-logistics-postmortem.md`, `docs/design/five-tool-integration-report.md`, `docs/design/integration-research-report.md`) **plus `rfc8439.txt`, which is invisible to `git status` because dowiz `.gitignore:86` has `/*.txt`**. The standing rule is prose in three places (MEMORY.md, todo map, skill) and has failed twice. The repo's own philosophy (§0·GP): deterministic gates, not prose. |
| c | **Largest unprotected WIP in either repo** | bebop `feat/wire-native-core`: **staged** `bebop2/core/src/kdf.rs` +618 (complete Argon2id, RFC 9106, in-tree BLAKE2b), `pq_dsa.rs` +698 (complete ML-DSA-65, FIPS 204), `pq_kem.rs` +4/−2 (Keccak XOF expose) = +1,314/−6; **unstaged** `AGENTS.md` +10 (§0 multipilot-native). Branch 5 commits ahead of `origin/feat/wire-native-core` (`cc265f8`…`8012b57`); `main` also 5 ahead of `origin/main` (`ca6b030`…`76b0c58`). No stash, no bundle, one machine. **New since the audit:** the crypto files were staged (~16:06Z), a 3-model review record was prepared (`.review/staged.json`, builder `hermes-tencent-hy3`, reviewer/overlap **null**), and an overlap review (`.review/findings-overlap.md`, 16:26Z) found **two real bugs in the staged ML-DSA-65** (C3a/C3b, §2.4 below). So the crown jewels are uncommitted, unpushed, *and* known-defective-but-fixable — protection and correctness are now entangled. |
| d | **bebop doc drift** | `docs/ARCHITECTURE.md` describes the retired TypeScript runtime present-tense ("Bebop is a TypeScript agent shell…", `bebop.ts`/`guard.ts`/`copilot.ts`); `llms.txt:9` + `llm-manifest.json:19` say the shell is TypeScript; README:128 and AGENTS.md:56 claim "294 Rust tests" vs 384 live full-workspace; CHANGELOG stops at 0.4.0 (07-09) — the whole native-core + bebop2 arc is unrecorded; `bebop2-roadmap-2026-07-10.md` still says ML-DSA/Argon2id are "STILL STUB" (now stale in the *other* direction). Full list §2.6. |

---

## 2. Research findings

### 2.1 The reconstructed pivot narrative (seed content for the memory corpus)

All commit refs verified in `git log --all`. Five phases in 3.5 days:

**Phase 1 — "Your own coding agent" (2026-07-08, one day).**
`2d8ccf7` 14:17 first commit: *"Bebop — your own coding agent (AGPL-3.0, guard OS + living memory
+ PQ identity + telemetry governor)"*. Same day: full OSS scaffolding (`bbc0e46` CITATION/SECURITY,
`d1e07fd` governance+CI, `4b18faa` MCP server + in-repo wiki, `8db1196` maintainer note "blocked
from money platforms" + SUPPORT.md, `2fe78c2` agent parity — slash cmds/plan mode/headless
JSON/hooks/subagents/skills, `369c7cd` Rust/WASM guard kernel + free-LLM default, `0578901`/`f2bf8ef`
security closes, `265dc5e` cosmo-noir launch animation), `897d2b6` **v0.2.0** 18:22.

**Phase 2 — Sovereign node + physics/L5 layer (07-08 late → 07-09).**
`af802f1` backup-WIP branch; `21def8a`+`ad57fb0` **Sovereign Node 0.3.0** — zkVM journal +
TigerBeetle-style money in one kernel gate; `e01220f` v0.3.1 red-team hardening (gate-drift,
optical poisoning, money fail-open); v0.3.2–v0.3.5 CI/red-team fixes (07-09 morning). Then the
physics arc: `77eb8b2`…`0806f74` D1–D6 flag-OFF (telemetry-shadow, RAG noise-clean, zenoh mesh),
`6e4313c` N1 open-system symmetry + N2 liveness watchdog, `501b88f`–`19f547c` L5 phases +
reverse-engineering loop, `2504a03` PDDL-INSTRUCT plan verifier, `9b7a990` tensor+graph field
theory, **`af9a490` Rust→WASM graph-PDE core** (rust-core is born, replacing the JS field sim),
`9575bee` memory-discipline perf, `e3a578e` **v0.4.0** "Multipilot + the new outfit" 20:27
(`baa2329` tag; `f6a4cbc` = today's `origin/main` tip).

**Phase 3 — Kill the TS runtime + decentralized delivery-protocol research (07-10 day).**
`2eaa82c` 06:16 *"eliminate TS runtime, wire native Rust core"* opens `feat/wire-native-core`.
`1649873` hybrid FIPS-203/204 vault (PQ identity, host crates). Morning: gap engines + MCP
(`abca38d`, `3750ba7`), `5619f91` native-core phase complete, stabilizer/Lyapunov/agentic-git/
mathx (`1e238c9`…`18933a3`), **`4fa0839` 12:26 detritus relocation #1** ("moved from /root/dowiz
(cross-repo detritus)"). Afternoon: wavefield connection-graph + damped graph-wave change-impact
gate (`ed9d4c3`…`40004b1`), then the **delivery-protocol turn**: `a914b8a` 17:25 Hybrid Cost-Aware
Engine (k-d+BFS+A*/CH) + *decoupled-matcher protocol doc*, `e892f39` PROTOCOL-CENTRALIZATION-MAP
(matcher/sequencer, oracle, SDK, identity gaps), `3c3b900` 18:05 open replicable dispatch API
("kills DANGER #1"), `28fc67b`/`e96d817` SYSTEM-ARCHITECTURE-AUDIT — thesis: **"trust is the
binding constraint"**; `f704a30`/`b41d577`/`0136b29` fable adversarial closes + core-RE-loop.

**Phase 4 — The first-principles pivot → bebop2 (07-10 evening).**
`d6565db` 20:00 operator pivot: **First-Principles + Physicality-as-Truth global rules** (AGC/LVDC
research, "impossibility of corruption at the bytecode level, not software" —
`docs/design/pivot-first-principles-physicality-2026-07-10.md`). 23 minutes later `669bdea`
**bebop2 greenfield zero-dep post-quantum core** — skeleton + architecture ("NOT a refactor — a
parallel implementation that at the end simply REPLACES the old one; old = oracle"). `90cbf33`/
`9e0a6aa` pillars 3+4 (latency envelope; better math per function — both operator directives).
`ae86776` 22:26 pq_kem schoolbook rewrite (NTT proven broken by 3 independent audits → pivot to
coefficient-domain schoolbook) = today's `origin/feat/wire-native-core` tip.

**Phase 5 — Hand-rolled crypto vs. spec KATs (07-11).**
`cc265f8` 04:08 SHA-512 + SHA3 (FIPS 180-4/202); `0de78a1` 04:36 ChaCha20 CSPRNG + HChaCha20
(RFC 8439); `cccec00`+`5f988a6` 09:38 Poly1305 per-block hibit fix per RFC 8439 §2.5.1 + §A.3.1
KAT (the "green-for-the-wrong-reason roundtrip" bug that birthed the 3-model gate); `8012b57`
14:17 **Ed25519 RFC 8032 §7.1 KAT green + 3 RFC deviations closed** (reject S≥L, reject
non-canonical y, wrong-pubkey RED KAT — the overlap reviewer had REJECTED the first version for
malleability; builder fixed; APPROVE recorded in `.review/{reviewer,overlap}-findings-sign.md`).
HEAD, 5 unpushed. Uncommitted on top: Argon2id + ML-DSA-65 (§1c), with a fresh overlap review
finding 2 bugs in the staged ML-DSA (§2.4).

**Meta-observation** (belongs in memory): the same serial-pivot gravity the audit flagged for
dowiz (§7.1) is running faster here — coding agent → physics planner → delivery protocol → PQ
crypto library in 3.5 days, each pivot documented and internally reasoned, none closed, zero users.

### 2.2 Session-transcript inventory (dates only, not read)

27 `.jsonl` transcripts in `/root/.claude/projects/-root-bebop-repo/`:
- **2026-07-10**: 16 files (mtimes 10:45, 10:46, 18:38×2, 18:48, 18:56, 19:07, 19:16, 19:37,
  19:47, 20:37×2, 20:50, 20:52, 20:59, 21:10) — clusters match the native-core morning and the
  protocol→pivot evening.
- **2026-07-11**: 11 files (03:13, 03:14, 03:15, 04:05×2, 04:28, 04:29, 15:47, 15:48, 16:14×2) —
  the overnight hash/rng KAT push and today's review sessions.
- None dated 07-08/07-09 despite the repo existing then — earliest sessions either ran with a
  different cwd (e.g. `/root/dowiz`, where bebop was extracted from) or mtimes reflect last write.
  Either way: ~27 sessions of context with **zero distillate**.

### 2.3 The bebop commit gate — what a commit actually requires

`.git/hooks/pre-commit` (real, executable) runs three blocking stages:
1. `scripts/verify-doc-claims.mjs` — 8+ falsifiable doc-claim checks incl. **check F: runs
   `cargo test --quiet --lib --workspace` (300s timeout) and requires README's "N Rust tests" and
   AGENTS.md's "cargo test — N Rust tests" to equal the live pass count with 0 failures.**
   `bebop2/core` IS a workspace member, so its lib tests count. The staged diff adds **8 new
   `#[test]` fns** → the live count rises → **committing the crypto work without also updating
   README.md:128 + AGENTS.md:56 to the new number will be blocked by the hook.** (Doc truth-pass
   and WIP protection are entangled by design.)
2. `scripts/guardrail-falsifiable-proof.mjs` — every test file needs a RED path (staged tests have
   tamper/RED asserts; expected green).
3. `scripts/three-model-review.sh` — refuses any commit with staged changes unless
   `.review/staged.json` carries `builder` ≠ `reviewer.agent` ≠ `overlap.agent`, each attestation
   with **non-empty findings**. It checks *distinctness and non-emptiness*, not verdict text.
   The record is consumed (`rm staged.json`) after each commit. Bypass `CI_THREE_MODEL_REVIEW=allow`
   exists but is declared red-line ("should normally be OFF").
   Current state: `staged.json` = `{builder: "hermes-tencent-hy3", reviewer: null, overlap: null}`
   — prepared, unattested.

### 2.4 State of the staged crypto work (the thing being protected)

`.review/findings-overlap.md` (2026-07-11 16:26, independent overlap checker, reproduced findings
in a standalone program + ran the real suite):
- **C3a (BUG, critical for FIPS-204 compliance)**: `pq_dsa.rs:351` applies `highbits` a second
  time to already-highbits values → the entire commitment `w1` encodes as **0** → the Fiat-Shamir
  challenge binds only to `mu`, not to `w = A·y`. Sign and verify share the broken path, so the
  roundtrip test is GREEN — the exact shared-blind-spot class the 3-model gate exists for.
  Fix: `let t = (poly[i] & 0x0f)` (one line).
- **C3b (BUG, latent)**: `pq_dsa.rs:578-585` passes `c·t0·2^d − c·s2` to `make_hint` instead of
  `c·t0·2^d`; masked by C3a, will surface the moment C3a is fixed. Fix: drop the `sub_mod(..., cs2)`.
- **C4 coverage gap**: Argon2id §5.3 official KAT **passes** (implementation almost certainly
  correct); the widely-published `password`/`somesalt`/`510fd3b7…` vector is determinism-checked
  but not KAT-asserted — add one `assert_eq!`.
- C1/C2/C3c/C5 all CORRECT (rejection bounds, decompose, hint pair, ML-KEM dual-impl bit-exact).
- The prior Ed25519 cycle (committed `8012b57`) shows the gate working end-to-end: overlap
  REJECT(conditional) → builder fixed 3 deviations → reviewer + overlap APPROVE on record.

Implication: **the staged work is protect-worthy but not commit-worthy as-is** under the repo's
own quality bar. Phase 1 therefore leads with a zero-risk filesystem-level safety net that needs
no gate at all, and only then the gate-honoring commit path.

### 2.5 How dowiz wires deterministic guards (the template for the detritus guard)

- **Hooks**: `.claude/settings.json` registers 9 hooks; blocking convention = **exit 2** with a
  `BLOCKED:`/`RED-LINE:` message on stderr (`protect-paths.sh`, `guard-bash.sh`); hooks parse
  `tool_input` JSON from stdin with a jq/python3/node fallback chain; docs and the harness layer
  are carved out of content red-lines (`post-edit-gates.sh` — never flag a doc for *naming* a
  pattern).
- **Pre-commit** (`.husky/pre-commit`): cheap whole-tree guardrails **always run** (steps
  1.4–1.4g, incl. `run-armaments.sh` and `guardrail-sandbox-staleness.mjs`); heavy steps are
  dynamically scoped to build-relevant staged paths.
- **VbM pattern**: guards ship with a companion RED test — precedent
  `scripts/guardrail-sandbox-staleness.test.mjs` next to its guard.
- **CI**: `pnpm verify:all --ci` (`.github/workflows/ci.yml:41`) → `scripts/verify-all.ts` runs
  the same guard family.
So the detritus guard should be: one `scripts/guardrail-cross-repo-detritus.mjs` + its `.test.mjs`,
wired into (a) `.husky/pre-commit` cheap section, (b) `verify-all.ts`, (c) optionally the
PostToolUse edit path. Spec in §4 Phase 3.

### 2.6 Stale-claim inventory for bebop (Phase 4 input)

| # | File:line | Stale claim | Correction | How verified |
|---|-----------|-------------|------------|--------------|
| 1 | `README.md:128` | "`cargo test` # 294 Rust tests" | Update to the live `cargo test --lib --workspace` count after the Phase-1 commit (294 + 8 staged tests ⇒ expected 302; **use the number the gate prints**, don't hand-compute) | `verify-doc-claims.mjs` check F exits 0 |
| 2 | `AGENTS.md:56` | "`cargo test` — 294 Rust tests" | Same number, same commit | same |
| 3 | `docs/ARCHITECTURE.md` (whole doc) | "Bebop is a TypeScript agent shell…" + present-tense `bebop.ts`/`guard.ts`/`router.ts`/`copilot.ts`/`core-wasm.ts` diagrams | Rewrite for the native runtime (crates/bebop + rust-core + bebop2), or minimally add a dated banner: "Describes the retired TS runtime, archived to `archive/` on 2026-07-10 (`2eaa82c`); live architecture = …" — contradicts README:134 "no TypeScript in the live path" | grep `TypeScript` in doc vs README claim |
| 4 | `llms.txt:9` | "A TypeScript shell over a Rust/WASM guard kernel (`crates/core` → `src/bebop_core.wasm`)" | Native Rust host; `crates/core` no longer exists (renamed `crates/core-legacy`, excluded from workspace); wasm artifact lives at `rust-core/bebop_core.wasm` | path grep |
| 5 | `llm-manifest.json:19` | `"shell": "TypeScript (Node)"` | `"shell": "Rust (native)"` (or equivalent) | grep |
| 6 | `README.md:85,123` | "run `npm test`" / "`bebop self maintain` runs the full test suite (`npm test` now covers `src/integration/**`)" | TS-era invocations; live suite is `cargo test --workspace`. Update or mark archived | grep `npm test` README |
| 7 | `CHANGELOG.md` | Last entry 0.4.0 (2026-07-09); nothing on TS-runtime elimination, wire-native-core, the delivery-protocol docs, the first-principles pivot, bebop2, the 3-model gate | Add an `[Unreleased]`/0.5.0-dev section summarizing 07-10→07-11 (content = §2.1 phases 3–5) | new section exists |
| 8 | `docs/design/bebop2-roadmap-2026-07-10.md` | "ML-DSA-65 … **STILL STUB** (pq_dsa.rs, 2 lines)"; "XChaCha20-Poly1305, Argon2id, SHA-512/SHA3, Ed25519, CSPRNG — **ALL STILL STUBS**" | Stale in the opposite direction: all are now implemented (aead/hash/rng/sign committed; kdf/pq_dsa staged). Update milestone table after Phase-1 commit, incl. C3a/C3b disposition | grep "STILL STUB" |
| 9 | `docs/wiki/*`, `docs/VERIFICATION-MATRIX.md`, `docs/architecture.md` | Not exhaustively audited here; wiki pages describe 0.4.0-era TS features | Sweep with `grep -rn "TypeScript\|npm test\|433\|294" docs/` during the same truth-pass | grep sweep exits clean or each hit annotated |
| 10 | (dowiz side) `/root/.hermes/skills/productivity/dowiz-living-memory/SKILL.md` + `references/memory-map.md` | "expect 202 pass: 186 bebop + 16 rust-core" | Live is 384 full-workspace (275+19+90) and moving; recommend the skill say "match the count `verify-doc-claims.mjs` prints" instead of a literal | skill file update (dowiz-side, not bebop) |

Note the pre-commit hook makes #1/#2 self-enforcing; #3–#9 are *not* covered by any live check —
a candidate follow-up is extending `verify-doc-claims.mjs` with a "no present-tense TS-shell
claim" check (one regex), which fits the repo's Constant-Doubt pattern.

### 2.7 Detritus destination research

bebop-repo conventions: research/design docs live flat in `docs/design/` named
`topic-YYYY-MM-DD.md`; protocol docs in `docs/design/delivery-protocol/`; the precedent relocation
(`4fa0839`) landed at `docs/design/research-12tool-ev-2026-07-10.md`. There is no `docs/specs/`
yet; KAT vectors live in `bebop2/core/kat/`. Full mapping in §4 Phase 3.

---

## 3. Options & tradeoffs

**O1 — WIP protection.**
- *A. Filesystem snapshot first (tar of the whole repo dir incl. `.git`, excl. build dirs), then
  gate-honoring commit.* Zero risk, zero gate, one command, preserves the exact staged/unstaged
  split and `.review/` state. **Chosen.**
- *B. Commit immediately with `CI_THREE_MODEL_REVIEW=allow`.* Fast, but violates the repo's own
  red-line ("should normally be OFF") and precisely the rule-shopping the audit warned about
  (§7.10). Rejected as default; listed as last-resort operator override.
- *C. Commit the staged work as an explicit WIP with honest REJECT findings attested.* Mechanically
  passes the gate (it checks distinctness + non-empty findings, not verdicts) and is honest — but
  ships a known-broken ML-DSA onto the branch and stretches the gate's spirit. Viable fallback if
  the C3a/C3b fixes can't be done same-day; requires the README/AGENTS count bump anyway.
- *D. `git stash` / new backup branch.* Stash mutates the working tree (risk to the precious WIP,
  loses staged-vs-unstaged split); a branch commit still requires the pre-commit gate (or
  `--no-verify`, same objection as B). Rejected.

**O2 — Memory bootstrap location/mechanism.**
- *A. Canonical Claude-Code auto-memory corpus at `/root/.claude/projects/-root-bebop-repo/memory/`
  mirroring dowiz's exact format (frontmatter `name`/`description`/`metadata.node_type: memory`,
  `[[links]]`, MEMORY.md index, ATTIC when it grows).* Consistent with the operator's 2026-07-10
  "ONE rule" (canonical = the corpus). **Chosen.** Note: files live *outside* the repo, so bebop's
  read-only settings don't block writing them from a dowiz/root session; but bebop-cwd sessions
  can't maintain them until the settings carve-out (decision D4).
- *B. In-repo `docs/memory/`.* Versioned + review-gated, but diverges from the established
  canonical-store rule and makes every memory write pay the 3-model gate. Rejected for now.

**O3 — Detritus guard shape.**
- *A. Sweep-style guardrail script (scans untracked files) wired into pre-commit + verify:all +
  PostToolUse.* Catches ALL arrival paths — including `curl`/`wget` downloads via Bash, which is
  how the PDFs most likely arrived and which an Edit/Write-only hook would never see. **Chosen.**
- *B. PreToolUse Edit/Write hook only.* Cheap, but structurally blind to Bash-created files —
  would not have caught 6 of the 12 current offenders. Rejected as sole mechanism (fine as an
  optional early-warning addition).
- *C. Content-hash allowlist of the repo root.* Too rigid; every legitimate new file needs a rule
  edit. Rejected.

**O4 — Doc truth-pass depth.** Minimal banner-patches vs full ARCHITECTURE rewrite. Chosen:
banner + targeted line fixes now (cheap, unblocks honesty), full rewrite queued as ordinary work —
consistent with "honest docs beat complete docs."

---

## 4. Recommended execution blueprint

Format per step: **Action / Gate / Falsifiable proof / Effort**. Gate markers:
`[SAFE]` = no approval needed; `[OPERATOR]` = human runs or signs it; `[RED-LINE]` = bebop
red-line class (crypto-constant / gate config) — per-change confirmation.

### Phase 1 — Protect the WIP (same day, ~30 min operator time)

**1.0 Snapshot everything, before anything else.** `[OPERATOR]` `[SAFE]` — effort: 2 min.
```bash
tar --exclude='bebop-repo/target' --exclude='bebop-repo/node_modules' \
    --exclude='bebop-repo/.venv' --exclude='bebop-repo/.venv2' --exclude='bebop-repo/.venv-render' \
    -czf /root/bebop-backup-2026-07-11.tgz -C /root bebop-repo
cd /root/bebop-repo
git diff --cached > /root/bebop-staged-2026-07-11.patch      # Argon2id + ML-DSA-65 + XOF (+1314/−6)
git diff          > /root/bebop-unstaged-2026-07-11.patch    # AGENTS.md §0 (+10)
git bundle create /root/bebop-all-refs-2026-07-11.bundle --all
```
This captures `.git` (all 110 commits incl. the 5+5 unpushed), the index (staged split), the
working tree, `.review/`, and the untracked research docs. Copy the four artifacts off-machine
(any channel the operator trusts) — until then, bus-factor-1 stands.
*Proof:* `git bundle verify /root/bebop-all-refs-2026-07-11.bundle` prints OK;
`tar -xzf … -O bebop-repo/bebop2/core/src/kdf.rs | diff - bebop2/core/src/kdf.rs` → empty.
*RED case:* truncate the tgz (`truncate -s 1000 copy.tgz`) → `tar -tzf` fails; corrupt bundle →
`git bundle verify` fails.

**1.1 Decide the commit path (see §6 D1).** Recommended: **fix-then-commit**, because the fixes
are two one-liners already specified by the overlap review, and the alternative commits a
known-degenerate signature scheme.

**1.2 Builder session applies the three review follow-ups.** `[OPERATOR dispatches; RED-LINE:
crypto]` — effort: 30–60 min agent time.
1. `pq_dsa.rs:351` → `let t = (poly[i] & 0x0f)` (kill the double-`highbits`).
2. `pq_dsa.rs:578-585` → hint arg = `ct0s` (drop `sub_mod(ct0s, cs2)`).
3. `kdf.rs` → add `assert_eq!` of the `510fd3b7…` tag for `password`/`somesalt`/t=2/m=16/p=4.
4. `cargo test -p bebop2-core` — the roundtrip now exercises the real commitment + live hint path.
*Proof:* suite green **and** the C3a reproduction from the overlap report
(`w1_encode([0,2,4,6,8,10,12,14]) == [32,100,168,236]`, not `[0,0,0,0]`) asserted as a new RED+GREEN
test. *RED case:* revert fix 1 → the new w1_encode test fails.

**1.3 Update the entangled doc counts in the same commit.** `[SAFE]` — effort: 5 min.
README.md:128 + AGENTS.md:56 to the count `verify-doc-claims.mjs` reports (expected 302; trust the
gate's printout). *Proof:* pre-commit check F prints `✓ test count honest` twice.
*RED case:* leave 294 in place → check F exits 1 (this is the hook's own falsifiability).

**1.4 Satisfy the 3-model gate honestly.** `[OPERATOR orchestrates 3 distinct agents]` — 20 min.
```bash
bash scripts/three-model-review.sh prepare <builder-id>          # re-prepare: post-fix diff differs
# reviewer (agent ≠ builder):
bash scripts/three-model-review.sh attest reviewer <reviewer-agent> <reviewer-findings.md>
# overlap (agent ≠ both):
bash scripts/three-model-review.sh attest overlap  <overlap-agent>  <overlap-findings.md>
```
The existing `findings-overlap.md` is the natural seed for the overlap findings (updated with the
post-fix verification). *Proof:* `three-model-review.sh` prints `✓ 3-model review gate satisfied
(builder ≠ reviewer ≠ overlap)`. *RED case:* attest overlap with the builder's id → gate fails
with "must be a DIFFERENT agent".

**1.5 Commit + push both branches.** `[OPERATOR]` — 5 min.
```bash
git add bebop2/core/src/kdf.rs bebop2/core/src/pq_dsa.rs bebop2/core/src/pq_kem.rs AGENTS.md README.md
git commit -m "feat(bebop2): Argon2id (RFC 9106) + ML-DSA-65 (FIPS 204) — overlap C3a/C3b closed, KATs green"
git push origin feat/wire-native-core        # clears the 5-unpushed exposure
git push origin main                          # fast-forward, 5 lounge/CLI commits from 07-09/10 (D2)
```
*Proof:* `git rev-parse origin/feat/wire-native-core` == local HEAD; `git status` clean except
intentionally-untracked docs. *RED case:* `git log origin/feat/wire-native-core..HEAD` non-empty →
push didn't land. Effort: minutes (pre-commit runs `cargo test --lib --workspace`; keep the target/
cache warm — see §5 R3).

**1.6 Fallback if 1.2–1.4 can't complete same-day.** `[OPERATOR]`
Step 1.0 already guarantees zero loss. Optional stronger interim: commit the staged diff as
explicit WIP with honest attestations whose findings state "REJECT-as-final / APPROVED-as-WIP-
checkpoint: C3a/C3b open, see .review/findings-overlap.md" — mechanically passes the gate without
forging anything, and the commit message must carry the defect list. Do **not** use
`CI_THREE_MODEL_REVIEW=allow` or `--no-verify` (repo red-line; the audit's rule-shopping warning).

### Phase 2 — Bootstrap the bebop living-memory corpus (same day, ~20 min)

**2.1 Create the corpus dir + 4 files.** `[OPERATOR or any non-bebop-cwd session]` `[SAFE]` —
10 min. `mkdir -p /root/.claude/projects/-root-bebop-repo/memory/` then write the four drafts
below verbatim (they follow dowiz's exact conventions: YAML frontmatter with `name`/`description`/
`metadata.node_type: memory`, `[[wiki-links]]`, dated filenames, index with one-line summaries).
*Proof:* `ls /root/.claude/projects/-root-bebop-repo/memory/*.md | wc -l` ≥ 4; every `](file.md)`
target in MEMORY.md exists (`grep -o '](.*\.md)' MEMORY.md` loop). *RED case:* delete one seed →
link check fails.

**2.2 Cross-link from the dowiz corpus + resync the mirror.** `[SAFE]` — 5 min.
Append one line to `/root/.claude/projects/-root-dowiz/memory/MEMORY.md` under "Cross-branch todo
map" (draft E below), then `cd /root/dowiz && node scripts/sync-memory-to-hermes.mjs` (HERMES.md is
a derived mirror — never hand-edit). *Proof:* `grep bebop2 /root/dowiz/HERMES.md` hits after sync.
*RED case:* edit HERMES.md by hand instead → next sync clobbers it (this is why the rule exists).

**2.3 Unblock future in-session memory writes.** `[OPERATOR]` `[RED-LINE: gate config]` — decision
D4. Minimal carve-out in `/root/bebop-repo/.claude/settings.json`: replace the blanket
`"deny": ["Edit","Write",…]` with denies scoped to the repo tree, or add
`"additionalDirectories"`-style allowance for the memory path — operator's governance file,
operator's edit. Until then, memory maintenance runs from dowiz-side sessions.
*Proof:* a bebop session can `Write` a memory file but still cannot `Edit` `bebop2/core/src/*`.
*RED case:* attempt an in-repo Write → still denied.

---
**Draft A — `/root/.claude/projects/-root-bebop-repo/memory/MEMORY.md`**

```markdown
# Project Memory — bebop / bebop-repo

> Canonical living-memory corpus for /root/bebop-repo (operator ONE-rule 2026-07-10: the Claude
> Code auto-memory corpus is THE store; nothing else is canonical). Bootstrapped 2026-07-11 from
> the G08 blueprint after 27 sessions ran with zero memory files. Keep the index lean; move
> closed topics to an ATTIC file once this exceeds ~40 lines (dowiz pattern).

## Ground state (verified 2026-07-11)
- Repo born 2026-07-08 14:17 (`2d8ccf7`); 104 commits on HEAD in 3.5 days; AGPL-3.0; remote
  git@github.com:SyniakSviatoslav/bebop.git; tags v0.2.0→v0.4.0.
- Branches: `feat/wire-native-core` (live work), `main` (76b0c58), both were 5 ahead of origin
  until the 2026-07-11 protection push (see [[bebop2-pivot-2026-07-10]] §WIP).
- Workspace: `rust-core` (graph-PDE/VSA field core, wasm) + `crates/bebop` (host agent CLI) +
  `bebop2/core` (zero-dep PQ core); `crates/core-legacy` excluded; TS runtime archived to
  `archive/` since `2eaa82c` (2026-07-10).
- Test truth: match the number `scripts/verify-doc-claims.mjs` prints (pre-commit check F pins
  README/AGENTS to `cargo test --lib --workspace`); full `cargo test --workspace` was 384/384 on
  2026-07-11 (audit-verified).

## Active arcs
- [🧬 bebop2 pivot — coding agent → physics → delivery protocol → zero-dep PQ core](bebop2-pivot-2026-07-10.md) — the full 5-phase history with commit refs; current center of gravity; WIP state + protection status.
- [📦 Delivery-protocol thread — the only genuine dowiz∩bebop intersection](delivery-protocol-thread-2026-07-10.md) — matcher/PoD/reputation/ledger primitives + protocol docs; strategy bridge to dowiz is UNSIGNED.
- [⚖️ Review-gate philosophy fork vs dowiz](review-gate-philosophy-fork-2026-07-11.md) — bebop mandates 3-model proxy review; dowiz purged proxy review (§0·GP). Unreconciled; do not rule-shop.

## Standing rules (binding)
- Three-model review on every commit (AGENTS.md §1; `.git/hooks/pre-commit`): builder ≠ reviewer ≠
  overlap, non-empty findings, record in `.review/staged.json`. `CI_THREE_MODEL_REVIEW=allow` is
  red-line OFF.
- Verified-by-Math: RED case ships with every green (AGENTS.md §2).
- Red-lines: auth/money/RLS/migrations/bulk-edit/**crypto-constant changes** → per-change human
  confirmation (AGENTS.md §3).
- Cross-repo hygiene: files referencing bebop/`feat/wire-native-core`/bebop2/crypto specs belong
  in /root/bebop-repo, NEVER in /root/dowiz. Enforced (dowiz side) by
  `scripts/guardrail-cross-repo-detritus.mjs` — do not rely on this prose line.
- Feature branch only; never build directly on `main` (bebop2-roadmap constraint).

## Cross-repo
- dowiz corpus: /root/.claude/projects/-root-dowiz/memory/ (152+ files) — product/DeliveryOS
  state lives THERE; this corpus covers bebop only. The naming hazard is real: "bebop" also means
  the dowiz brand skin, dowiz `tools/bebop` governor, and `dowiz/rebuild/crates/bebop` (audit §2.2).
```

**Draft B — `bebop2-pivot-2026-07-10.md`**

```markdown
---
name: bebop2-pivot-2026-07-10
description: "The bebop pivot history 2026-07-08→11: coding agent (v0.2) → sovereign-node/physics
  L5 (v0.3–0.4) → decentralized delivery-protocol research → First-Principles/Physicality pivot
  (d6565db) → bebop2 greenfield zero-dep post-quantum core (669bdea). Includes the 2026-07-11
  Argon2id/ML-DSA WIP state, its overlap-review defects (C3a/C3b), and protection status."
metadata:
  node_type: memory
---

Five phases, all commit-ref'd (verify with `git log --all` in /root/bebop-repo):

1. **Coding agent (07-08).** `2d8ccf7` 14:17 "Bebop — your own coding agent" + same-day OSS
   scaffolding (governance/CI/MCP/wiki, maintainer note "blocked from money platforms" `8db1196`),
   Rust/WASM guard kernel `369c7cd`, v0.2.0 `897d2b6`.
2. **Sovereign node + physics/L5 (07-08→09).** zkVM journal + TigerBeetle money in one kernel gate
   (0.3.0 `21def8a`/`ad57fb0`), red-team hardening v0.3.1–0.3.5, D1–D6 + N1/N2 flag-OFF arcs,
   PDDL-INSTRUCT `2504a03`, Rust→WASM graph-PDE field core `af9a490` (rust-core born),
   v0.4.0 "Multipilot + the new outfit" `e3a578e`/`baa2329`. origin/main sits at `f6a4cbc`.
3. **Native core + delivery protocol (07-10 day).** `2eaa82c` eliminates the TS runtime
   (archive/); PQ hybrid vault `1649873`; wavefield + damped graph-wave gate `ed9d4c3`…`40004b1`;
   hybrid k-d+BFS+A*/CH engine + decoupled-matcher `a914b8a`; MATCHER-API "kills DANGER #1"
   `3c3b900`; SYSTEM-ARCHITECTURE-AUDIT "trust is the binding constraint" `28fc67b`. See
   [[delivery-protocol-thread-2026-07-10]].
4. **First-principles pivot → bebop2 (07-10 evening).** Operator: First-Principles +
   Physicality-as-Truth global rules `d6565db` (AGC/LVDC envelope; docs/design/
   pivot-first-principles-physicality-2026-07-10.md); 23 min later `669bdea` bebop2 skeleton —
   from-scratch, zero-dep, no_std+alloc, empty-import wasm, "old bebop = oracle, then swapped".
   Pillars 3+4 by operator directive (`90cbf33`, `9e0a6aa`). pq_kem NTT → schoolbook `ae86776`.
5. **Crypto KATs (07-11).** hash `cc265f8`, rng `0de78a1`, Poly1305 §2.5.1 hibit + §A.3.1 KAT
   `cccec00`/`5f988a6` (the shared-blind-spot bug that birthed the 3-model gate), Ed25519 RFC 8032
   §7.1 KAT green `8012b57` (overlap REJECTED v1 for malleability; fixed; APPROVE on record).

**WIP state (2026-07-11 16:40Z):** staged +1,314/−6 — complete Argon2id (RFC 9106, kdf.rs,
§5.3 official KAT GREEN) + complete ML-DSA-65 (FIPS 204, pq_dsa.rs) + pq_kem XOF expose; unstaged
AGENTS.md §0 multipilot-native (+10). `.review/staged.json` prepared (builder hermes-tencent-hy3),
attestations pending. **Overlap review (.review/findings-overlap.md) found 2 real bugs in the
staged ML-DSA:** C3a `w1_encode` double-highbits → zero commitment (roundtrip green for the wrong
reason); C3b `make_hint` wrong argument (latent). Fixes are one-liners (pq_dsa.rs:351, :578-585)
+ one Argon2id KAT assert (510fd3b7 vector). Protection: tar + bundle + patch trio taken
2026-07-11 per G08 Phase 1; commit/push status → check `git log origin/feat/wire-native-core..HEAD`.

**Meta:** 4 identities in 3.5 days (agent → physics → protocol → crypto lib) — the dowiz
serial-pivot pattern (audit §7.1) running faster. Each pivot documented, none closed, zero users.
Related: [[review-gate-philosophy-fork-2026-07-11]].
```

**Draft C — `delivery-protocol-thread-2026-07-10.md`**

```markdown
---
name: delivery-protocol-thread-2026-07-10
description: "Decentralized delivery-protocol research thread in bebop-repo — the only place the
  dowiz and bebop futures genuinely intersect. Primitives are tested library code; network layer
  does not exist; the dowiz↔protocol strategy bridge is research-only and UNSIGNED."
metadata:
  node_type: memory
---

**Thesis** (docs/design/delivery-protocol/SYSTEM-ARCHITECTURE-AUDIT.md, `28fc67b`/`e96d817`):
trust is the binding constraint of dispatch platforms; decentralize the matcher/sequencer or the
platform re-centralizes. "Protocol, not platform."

**What exists as tested code** (crates/bebop/src/): `matcher.rs` — pure deterministic fail-closed
`match_orders()` + open replicable dispatch API + reference client (`3c3b900`, "kills DANGER #1");
`pod.rs` proof-of-delivery attribution; `reputation.rs`; `ledger.rs` (Σbalance==0 conservation);
`zkvm.rs` commit/verify boundary; `zenoh.rs` mesh seam. All flag-OFF library code.

**What exists as design only:** DECOUPLED-MATCHER.md, MATCHER-API.md,
PROTOCOL-CENTRALIZATION-MAP.md (`e892f39` — matcher/sequencer, oracle, SDK, identity gaps),
fable-protocol-2026-07-11/ 10-angle review. `delivery/` dir is EMPTY — no network layer, no nodes,
no deployment.

**Research corpus** (relocated from /root/dowiz where it sat as untracked detritus, G08 Phase 3):
docs/design/delivery-protocol/{crypto-primitives-research,platform-vs-protocol-logistics,
web3-logistics-postmortem}-2026-07-10.md + docs/design/hybrid-routing-sota-2026-07-10.md.
The postmortem catalogues why prior Web3 logistics plays died — read before adding any token.

**Open strategic fact:** if the intent is that bebop2 hosts a sovereign protocol that dowiz plugs
into, that bridge exists ONLY as these research docs + dowiz/docs/design/dowiz-agent-cli/CORE.md.
No operator-signed decision ranks this against the dowiz rebuild/MVP/OSS arcs (audit §7.1 —
"no single spine"). Next concrete steps per audit §5.7: dispute arbitration (F2),
hidden-centralization removal (F3), 50%-courier-drop stress test (F4).
Related: [[bebop2-pivot-2026-07-10]].
```

**Draft D — `review-gate-philosophy-fork-2026-07-11.md`**

```markdown
---
name: review-gate-philosophy-fork-2026-07-11
description: "Unreconciled governance fork: dowiz purged proxy review (§0·GP, f1255ad5,
  2026-07-07) while bebop MANDATES three-model proxy review on every commit (pre-commit gate,
  operator standing rule 2026-07-11). Both are 'binding'. Record of evidence on each side +
  proposed reconciliation. Do not rule-shop between repos."
metadata:
  node_type: memory
---

**dowiz side (2026-07-07):** operator removed council + critic/review proxy agents + advisory
hooks (`f1255ad5`); kept deterministic gates only. Rationale: proxies produced false greens and
noise; "an unverified opinion is a PROXY mistaken for GROUND TRUTH." See dowiz corpus
[[ground-truth-over-proxy-2026-07-07]].

**bebop side (2026-07-11):** AGENTS.md §0–1 + `.git/hooks/pre-commit` → three-model-review.sh:
builder ≠ reviewer ≠ overlap, non-empty findings, attestation record consumed per commit.
Evidence it earns its cost — two real catches in 24h that deterministic KATs missed:
1. Ed25519 v1: overlap REJECTED for malleability deviations (accept S≥L, non-canonical y);
   fixed in `8012b57`, APPROVE on record (.review/{reviewer,overlap}-findings-sign.md).
2. Staged ML-DSA-65: overlap found `w1_encode` double-highbits → zero Fiat-Shamir commitment,
   GREEN roundtrip test hiding it (sign+verify shared the broken path), + latent make_hint arg bug
   (.review/findings-overlap.md, 2026-07-11).

**Why both can be right (proposed reconciliation, needs operator signature):** §0·GP itself says
a proxy is used ONLY where no deterministic check exists. For dowiz product code a ground-truth
oracle nearly always exists (staging Playwright, DB re-read, migrations diff) → proxies were net
noise. For from-scratch crypto there is often NO independent oracle reachable (NIST vectors
unreachable offline; a self-written roundtrip shares the implementation's blind spots) → an
independent re-derivation by a different model IS the nearest available ground truth. Scope rule:
proxy review binding where the artifact has no external oracle (crypto, novel math); deterministic
gates everywhere an oracle exists. Until signed, apply each repo's own rule inside that repo and
never import the other's exemptions.
Related: [[bebop2-pivot-2026-07-10]].
```

**Draft E — one-line addition to the dowiz corpus index** (append under "Cross-branch todo map"
in `/root/.claude/projects/-root-dowiz/memory/MEMORY.md`, then resync):

```markdown
- [🧬 bebop-repo corpus BOOTSTRAPPED (2026-07-11)](../../-root-bebop-repo/memory/MEMORY.md) — bebop now has its own canonical corpus (bebop2 pivot narrative, delivery-protocol thread, review-gate fork). bebop state lives THERE; this corpus stays dowiz-only. Detritus rule is now a deterministic gate: `scripts/guardrail-cross-repo-detritus.mjs`.
```

### Phase 3 — Detritus relocation + the deterministic guard

**3.1 Relocation map (12 files).** `[OPERATOR]` `[SAFE — all sources are untracked in dowiz; no
git history is lost]` — 5 min.

| Source (in /root/dowiz) | Destination (in /root/bebop-repo) | Rationale |
|---|---|---|
| `chacha.pdf` | `docs/specs/chacha.pdf` (new dir) | RFC/spec reference for bebop2 rng/aead |
| `poly1305.pdf` | `docs/specs/poly1305.pdf` | ditto (Poly1305 §2.5.1 work) |
| `xsalsa.pdf` | `docs/specs/xsalsa.pdf` | ditto |
| `xchacha2.pdf` | `docs/specs/xchacha2.pdf` | ditto |
| `xchacha_draft.html` | `docs/specs/xchacha_draft.html` | draft-irtf-cfrg-xchacha reference |
| `rfc8439.txt` **(gitignore-masked!)** | `docs/specs/rfc8439.txt` | THE Poly1305/ChaCha KAT anchor (AGENTS.md names it) |
| `crypto-primitives-research.md` | `docs/design/delivery-protocol/crypto-primitives-research-2026-07-10.md` | protocol crypto/mechanism-design research |
| `hybrid-routing-sota.md` | `docs/design/hybrid-routing-sota-2026-07-10.md` | SOTA brief for the k-d+BFS+A*/CH engine (`a914b8a`) |
| `platform-vs-protocol-logistics.md` | `docs/design/delivery-protocol/platform-vs-protocol-logistics-2026-07-10.md` | protocol strategy research |
| `web3-logistics-postmortem.md` | `docs/design/delivery-protocol/web3-logistics-postmortem-2026-07-10.md` | protocol failure-mode research |
| `docs/design/five-tool-integration-report.md` | `docs/design/five-tool-integration-report-2026-07-10.md` | targets "bebop coding-agent", names `/root/bebop-repo` in its own header |
| `docs/design/integration-research-report.md` | `docs/design/integration-research-report-2026-07-10.md` | companion report, same backlog (audit §5.7) |

Naming follows bebop's `topic-YYYY-MM-DD.md` convention and the `4fa0839` relocation precedent.
One command block:
```bash
mkdir -p /root/bebop-repo/docs/specs
cd /root/dowiz
mv chacha.pdf poly1305.pdf xsalsa.pdf xchacha2.pdf xchacha_draft.html rfc8439.txt /root/bebop-repo/docs/specs/
mv crypto-primitives-research.md      /root/bebop-repo/docs/design/delivery-protocol/crypto-primitives-research-2026-07-10.md
mv platform-vs-protocol-logistics.md  /root/bebop-repo/docs/design/delivery-protocol/platform-vs-protocol-logistics-2026-07-10.md
mv web3-logistics-postmortem.md       /root/bebop-repo/docs/design/delivery-protocol/web3-logistics-postmortem-2026-07-10.md
mv hybrid-routing-sota.md             /root/bebop-repo/docs/design/hybrid-routing-sota-2026-07-10.md
mv docs/design/five-tool-integration-report.md /root/bebop-repo/docs/design/five-tool-integration-report-2026-07-10.md
mv docs/design/integration-research-report.md  /root/bebop-repo/docs/design/integration-research-report-2026-07-10.md
```
Leave them untracked in bebop and fold them into the next gated docs commit (e.g. together with
Phase 4), so the relocation itself doesn't pay a 3-model review round. PDFs+txt ≈ 2.4 MB total —
track vs `.gitignore` is decision D3.
*Proof:* `git -C /root/dowiz status --porcelain | grep -ciE 'chacha|poly1305|xsalsa|xchacha|crypto-primitives|hybrid-routing|platform-vs-protocol|web3-logistics|five-tool|integration-research'` → 0, **and** `ls /root/dowiz/rfc8439.txt` → no such file (the porcelain check alone cannot see it).
*RED case:* `touch /root/dowiz/xsalsa.pdf` → count > 0 (and the 3.2 guard goes red).

**3.2 Deterministic cross-repo-detritus guard — spec.** `[OPERATOR approves the new gate;
implementation is ordinary dowiz work]` `[RED-LINE-adjacent: touches .husky/pre-commit + hooks]` —
effort: ~1h build + test.

New file `dowiz/scripts/guardrail-cross-repo-detritus.mjs` (sweep-style, like
`guardrail-sandbox-staleness.mjs`):

- **Triggers (three, layered):**
  1. `.husky/pre-commit` — in the "cheap whole-tree guardrails always run" section (after 1.4g):
     `node scripts/guardrail-cross-repo-detritus.mjs || exit 1`.
  2. `scripts/verify-all.ts` — same invocation (CI parity via `pnpm verify:all --ci`).
  3. (optional, D5) `post-edit-gates.sh` PostToolUse — call with `--file "$REL"` for immediate
     feedback on Edit/Write; note this arm alone is insufficient (Bash `curl -o` bypasses it —
     that is how the PDFs arrived), which is why the sweep arms are primary.
- **Candidate set (deterministic):**
  `git ls-files --others --exclude-standard` (untracked, non-ignored) **UNION** root-level
  ignored files caught by `git status --porcelain --ignored=matching`, filtered to depth ≤ 2 and
  extensions `md|pdf|txt|html|rs|toml` — the ignored arm exists because `.gitignore:86 /*.txt`
  masked `rfc8439.txt` from every porcelain-based check.
- **Detection predicate (per candidate → RED if `name_hit || content_hit`, unless allowlisted):**
  - `name_hit`: basename matches
    `/(chacha|poly1305|x?salsa|ml[-_]?kem|ml[-_]?dsa|argon2|kyber|dilithium|rfc[0-9]{4}|fips[-_ ]?20[234])/i`.
  - `content_hit` (text files only, first 400 lines): matches ≥1 of
    `\/root\/bebop-repo`, `feat\/wire-native-core`, `\bbebop2\b`, `crates\/bebop\/src`,
    `bebop-repo`, `research_patterns\.rs` — **AND** matches none of the dowiz-legitimacy markers
    `data-skin="bebop"`, `tools\/bebop`, `rebuild\/crates\/bebop` (the four-meanings naming hazard,
    audit §2.2, is why bare "bebop" must NOT be a trigger).
  - Allowlist: `scripts/cross-repo-detritus.allow` — one repo-relative path or glob per line;
    ships with `docs/research/*audit-dowiz-bebop*.md` and `docs/design/gap-blueprints-*/**`
    (documents *about* both repos legitimately live in dowiz; same carve-out philosophy as
    `post-edit-gates.sh` docs-name-a-pattern rule). Tracked files are never scanned — anything
    already committed passed a human/gate once.
- **RED behavior:** exit 1 (pre-commit/verify context) printing per file:
  `DETRITUS: '<path>' looks bebop-topic (matched: <pattern>). Files referencing bebop belong in
  /root/bebop-repo — suggested: mv '<path>' /root/bebop-repo/docs/design/  (allowlist:
  scripts/cross-repo-detritus.allow)`. In the optional hook arm: exit 2 + `BLOCKED:` on stderr
  (dowiz hook convention). Fail-open on git/parse errors (guard-bash convention) — a broken guard
  must not brick commits.
- **Falsifiable proof (VbM, ships with the guard):** `scripts/guardrail-cross-repo-detritus.test.mjs`
  (pattern: `guardrail-sandbox-staleness.test.mjs`):
  - GREEN: on a tree with no candidates → exit 0.
  - RED 1 (name): create `tmp-fixture-poly1305-notes.md` untracked at repo root → guard exits 1
    and names the file; delete → exit 0.
  - RED 2 (content): create `tmp-fixture-innocuous.md` containing `/root/bebop-repo` +
    `feat/wire-native-core` → exit 1; delete → exit 0.
  - RED 3 (ignored-mask): create `tmp-fixture-rfc9999.txt` at root (gitignore-masked by `/*.txt`)
    → exit 1 — this is the regression test for the rfc8439.txt blind spot.
  - FP guard: fixture containing only `data-skin="bebop"` → exit 0 (naming-hazard false-positive
    stays green).
  Register the guard in `run-armaments.sh` so `guardrail-gate-armament.mjs` proves it is armed,
  and add a REGRESSION-LEDGER row (dowiz self-improvement rule: fix ≠ done without red→green).
- **Effort:** guard ~120 LOC, test ~80 LOC, wiring 3 lines. No new deps.

### Phase 4 — bebop doc truth-pass

**4.1 Apply the §2.6 table** (items 1–2 land inside the Phase-1 commit; items 3–9 as one
`docs(truth-pass)` commit through the normal 3-model gate; item 10 is a dowiz-side skill edit).
`[OPERATOR dispatches; SAFE content-wise]` — effort: ~1h agent time + review round.
*Proof:* `node scripts/verify-doc-claims.mjs` exits 0 AND a one-shot sweep
`grep -rn "TypeScript shell\|npm test\|STILL STUB" README.md AGENTS.md llms.txt llm-manifest.json docs/ARCHITECTURE.md docs/design/bebop2-roadmap-2026-07-10.md` returns only annotated/historical hits.
*RED case:* re-insert `"shell": "TypeScript (Node)"` → the sweep (and, if extended per below,
check I) goes red.

**4.2 (Recommended ratchet)** extend `verify-doc-claims.mjs` with **check I**: fail if
`docs/ARCHITECTURE.md`/`llms.txt`/`llm-manifest.json` claim a TypeScript live path while
`archive/` holds the TS tree — turns this truth-pass from a one-time sweep into a standing gate
(the repo's own Constant-Doubt pattern). `[RED-LINE-adjacent: gate change → per AGENTS.md §3 flag
for human confirmation]` — 20 min.

---

## 5. Risks & rollback

| # | Risk | Mitigation / rollback |
|---|------|----------------------|
| R1 | Phase-1 fixes (C3a/C3b) introduce a new defect in ML-DSA | The tar/patch trio from step 1.0 restores the exact pre-fix staged state (`git apply /root/bebop-staged-2026-07-11.patch` on a clean checkout). The overlap re-review in 1.4 is the catch net; the new w1_encode RED test pins the fix direction. |
| R2 | `git add`/commit mangles the carefully staged split | Step 1.0 snapshots `.git` including the index; `tar -xzf` to a scratch dir recovers it byte-exact. Patches are the belt-and-braces copy. |
| R3 | Pre-commit check F flakes: `cargo test --lib --workspace` has a 300s timeout and bebop2 KATs are compute-heavy (audit measured 448s debug / 32.8s release for the full suite) | Warm the cache first (`cargo test --lib --workspace` once, then commit immediately); if the count parses partial output and mismatches, re-run — the failure mode is a visible red, not a silent wrong commit. Do NOT respond with `--no-verify`. |
| R4 | WIP-commit fallback (1.6) normalizes committing known-broken crypto | Commit message must carry the defect list + `.review` pointer; MEMORY.md draft B records C3a/C3b as open so no later session mistakes the branch tip for done. |
| R5 | Memory corpus drifts stale like the docs did | Corpus bootstrap includes the standing-rules section; the real fix is behavioral (session-end memory writes) — unblocked structurally by D4. If it rots, files are additive-only; nothing to roll back. |
| R6 | Detritus guard false-positives block legitimate dowiz work | Allowlist file + naming-hazard negative markers + fail-open on errors; the FP fixture in the test suite pins the `data-skin="bebop"` case green forever. Rollback = remove 3 wiring lines (guard is sweep-only, holds no state). |
| R7 | Relocated PDFs bloat bebop git history if tracked (D3=track) | 2.4 MB one-time; if unwanted later, they are leaf blobs on a young repo — or choose D3=ignore and keep them untracked in `docs/specs/` (they are reproducible from public RFC sources). |
| R8 | Doc truth-pass commit stalls on the 3-model gate (3 agents needed for a docs change) | Batch it with the Phase-3 relocation into one review round; the gate cost is per-commit, not per-file. |
| R9 | This blueprint's own drafts go stale before application | Every draft carries its as-of timestamp and verification command; appliers re-run the one-liners (`git status`, `bundle verify`, `verify-doc-claims`) rather than trusting the prose — the corpus's own don't-trust-pasted-state rule. |

Nothing in Phases 1–4 deletes anything anywhere; every step is additive or a move of untracked
files, and every move has a listed inverse.

---

## 6. Operator decision points

| # | Decision | Options | Recommendation |
|---|----------|---------|----------------|
| D1 | Commit path for the staged crypto WIP | (a) fix C3a/C3b then commit; (b) WIP-commit with honest REJECT-as-final findings; (c) snapshot-only, commit later | **(a)** — fixes are two one-liners already specified by the overlap review; (b) acceptable if same-day capacity is short; snapshot (1.0) happens first in all cases |
| D2 | Push local `main` (5 commits, fast-forward) to origin | push / hold | **Push** — plain fast-forward of 07-09/10 lounge+CLI work; halves the unpushed exposure. (bebop has no push-to-main hook block; the "feature branch only" roadmap rule governs *new* work, not syncing history) |
| D3 | Track the relocated spec PDFs/txt in bebop git | track in `docs/specs/` / gitignore + keep untracked | **Track** — 2.4 MB one-time; the repo's whole ethos is committed anchors (KAT vectors are already in-tree); if size-sensitive, ignore the 1.5 MB poly1305.pdf only |
| D4 | Carve a memory-write allowance out of bebop's read-only `.claude/settings.json` | scoped allow for the corpus path / keep deny + maintain memory from dowiz-side sessions | **Scoped allow** — 0 memory files after 27 sessions is structural; keep the in-repo Edit/Write deny intact |
| D5 | Detritus guard placement | pre-commit+verify only / + PostToolUse hook arm / + a mirror guard in bebop | **pre-commit+verify now**, hook arm optional later; a bebop-side mirror is low-value (drift ran dowiz-ward both times) |
| D6 | The review-gate philosophy fork | sign the domain-scoped reconciliation in Draft D / keep both repos' rules as-is with a no-rule-shopping note | **Sign the scoped rule** — it is §0·GP-consistent ("proxy only where no deterministic oracle exists") and the fork otherwise invites rule-shopping (audit §7.10) |
| D7 | Backup artifact custody | where the tar/bundle/patches get copied off-machine | Operator-only knowledge; the blueprint deliberately does not pick a channel |

---

*G08 blueprint produced 2026-07-11 by a read-only research session. The only file created is this
document. Both repos' working trees, indexes, branches, stashes, and `.review/` state were left
exactly as found.*
