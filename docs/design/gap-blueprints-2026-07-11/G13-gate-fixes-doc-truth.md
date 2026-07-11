# G13 — Gate/doc drift: land the uncommitted gate fixes, run the doc truth-pass

> Gap blueprint, 2026-07-11. Read-only research session; nothing in the tree was modified and the
> three uncommitted diffs remain exactly as found on `feat/paleo-dinosaur-digs`.
> Grounds: audit `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md` (§6.1, §7.2, §7.5,
> §7.7, §9 rec 3), GH issue #9, memory (`council-gate-disabled-2026-07-05`,
> `ground-truth-over-proxy-2026-07-07`), `docs/operating-model/fable-audit-findings-2026-07-07.md`,
> and direct verification performed for this blueprint — every fresh claim below carries a
> file:line, commit hash, or an executed read-only probe. Sibling blueprints: G06 (MVP exit gate,
> owns the "SHIPPING-READY" substance), G07 (program spine / lineage merge), G12 (stale-worktree
> disposition — this doc only cites worktree evidence, it does not plan their fate).

---

## 1. Gap & evidence

**The gap (G13).** Three coherent, finished-looking diffs sit **uncommitted** in the working tree
(`git status`: `scripts/plane-guard.mjs` +28/−1, `scripts/verify-all.ts` +8/−3,
`apps/web/src/lib/reactAction.test.ts` +28/−15 net −2), while the "authoritative" entry-point docs
still describe states that were deliberately changed:

- **Gate side:** the committed tip `e5eb3d03` still carries the OLD plane-guard P7 row
  (`['P7 council-before-code', '.claude/hooks/serious-gate.sh', …]`) which asserts a hook that was
  deliberately deleted on 2026-07-07 (`f1255ad5`, operator directive "ground truth over proxy
  reasoning"). Result: `node scripts/plane-guard.mjs` **hard-fails P7 on every clean checkout** of
  this lineage — verified: the pushed `origin/feat/sovereign-core-phase-zero` tip (`330ff4ed`) has
  the old P7 row *and* no `serious-gate.sh`. `verify:all` step 54 runs plane-guard, so
  `pnpm verify:all --ci` is red on those checkouts. (Mitigating fact: `.github/workflows/ci.yml`
  triggers only on push/PR to `main`, and pre-commit does not run plane-guard — so nothing is
  red in CI *today*; the red fires on local verify:all runs and on any future PR toward main.)
- **Doc side:** `docs/design/harness/META-CONTROLLER.md:33` still lists `serious-gate` among the
  immutable authority hooks; `DeliveryOS-As-Built-Summary-v1.md` (audit date 2026-06-04) claims
  HS256 JWT and 67 migrations; `CONTEXT-INDEX.md` ("Updated: 2026-06-08") crowns that As-Built doc
  "START HERE … always loaded first … trumps planning docs" and points at retired machinery;
  `project-state-2026-07-08.md` (which lives OUTSIDE the repo, see §2.3) self-labels
  "Authoritative … MVP is SHIPPING-READY" against the same week's HANDOFF "MVP ~40%".
- **Claimed-unlanded follow-ups:** audit §7.7 asserts the fable-audit follow-ups (guard-bash 83%
  false-positive fix, loop-registry parity circuit) show "no evidence landed". **This research
  falsifies that claim** — see §2.4. Phase 4 becomes a record-correction, not a re-scope.

This is GH #9's class (gates referencing machinery that a given checkout doesn't have) recurring in
the opposite direction: #9 was "gate demands scripts main lacks"; G13 is "gate demands a hook the
current lineage deliberately removed".

---

## 2. Research findings

### 2.1 My own audit of the three uncommitted diffs

**(a) `scripts/plane-guard.mjs` (+28/−1) — P7 rewrite. Verdict: CORRECT IN DESIGN, land it, but
one regex leg of its RED case is dead — amend 2 lines before (or immediately after) landing.**

What it does: removes the P7 row from the `wired` existence-check array and replaces it with an
inline predicate (new lines 53–78): GREEN = `serious-gate.sh` absent AND not functionally
referenced; RED = present-but-unregistered (resurrected-unwired) OR absent-but-still-referenced
(dangling pointer). Functional reference = a `"command":` hook registration in
`.claude/settings.json` or a `cmd:` step in `verify-all.ts`; prose/comments deliberately excluded.

Verified GREEN/RED behavior (predicate replicated read-only, exact same regexes, executed in this
session):

| Tree | councilPresent | hookRegistered | stepInvoked | p7ok |
|---|---|---|---|---|
| `/root/dowiz` (paleo, diff applied) | false | false | false | **true (GREEN)** |
| `/root/dowiz-wt-phase0` (07-02 worktree) | true | false | false | **false (RED)** |
| `/root/dowiz-wt-phase5` (07-02 worktree) | true | false | false | **false (RED)** |

So the headline RED case ("serious-gate reappears unwired") demonstrably fires — the two stale
worktrees are live RED fixtures. **However, my audit found a genuine falsifiability hole:**

- **BUG (dead RED leg):** `councilHookRegistered = /"command"\s*:\s*[^"]*serious-gate\.sh/` can
  **never** match this repo's real registration syntax. The historical registration (still present
  at `/root/dowiz-wt-phase0/.claude/settings.json:39`) is
  `"command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/serious-gate.sh\""` — after `"command":`
  the value opens with `"`, so `[^"]*` stops immediately and the regex fails. Proven by executing
  the regex against that exact file: `false`, while `includes('serious-gate.sh')` is `true`.
  Consequence: the declared RED case "absent BUT still referenced in settings.json" is
  **unreachable** — if someone deleted the script but left (or restored) the hook registration,
  P7 would read false-GREEN. That is precisely the "half-removed dangling pointer" state P7 claims
  to catch, and a VbM violation inside a comment block that invokes VbM.
- **Minor message bug:** the detail ternary prints "present but NOT wired" for *any*
  `councilPresent` state — including present-AND-wired (a full deliberate resurrection). The
  verdict (RED) is still right for that case — re-enabling council must be a reviewed diff that
  rewrites P7 too, which is coherent ratchet behavior — but the printed reason would be false.
- **Also verified:** whether serious-gate is wired or unwired, present ⇒ RED. So a deliberate
  operator re-enable (per `council-gate-disabled-2026-07-05.md` "To re-enable: re-add the entry")
  now additionally requires rewriting P7 in the same diff. This should be stated in
  META-CONTROLLER.md (Phase 2) so it is a documented property, not a surprise.

Exact amendment (2 changed lines + comment; apply to the working-tree file before commit A):

```js
// settings.json is JSON — comments are impossible, so ANY occurrence of the hook filename in it is
// functional registration debris. (A stricter "command"-scoped regex was RED-case-dead against the
// repo's real registration syntax — bash "$CLAUDE_PROJECT_DIR/…/serious-gate.sh" — proven vs the
// 07-02 worktrees' settings.json:39.)
const councilHookRegistered = settingsBody.includes('serious-gate.sh');
```

```js
    : councilPresent
      ? (councilReferenced
          ? 'present AND still registered — council resurrected; if deliberate, rewrite P7 in the same reviewed diff'
          : 'present but NOT wired in settings.json/verify-all — resurrected without arming (half-state)')
      : 'absent BUT still referenced in settings.json/verify-all — half-removed (dangling pointer)');
```

**(b) `scripts/verify-all.ts` (+8/−3) — comment/name truth for the same removal. Verdict: CORRECT,
land it; note it is NOT load-bearing for P7.** Verified: the old step name/comment
(`'gate armament (serious/red-line/bash hooks …)'` + the "held serious-gate + red-line-gate open"
comment) does **not** match the new P7 `cmd:`-scoped regex (executed against `HEAD:scripts/verify-all.ts`:
`false`). So P7 would go GREEN even without this diff — the diff is honest-labeling hygiene, which
is exactly the doc-truth theme of this gap. Keep it in the same commit.

**(c) `apps/web/src/lib/reactAction.test.ts` — non-null assertions. Verdict: CORRECT, verified
green.** `tsconfig.base.json:8` sets `"noUncheckedIndexedAccess": true`, making every
`res.trace[0].ok` access a TS18048/TS2532 error; the diff adds `!` on 14 indexed accesses. The RED
side is already on record: commit `77811204` (07-08) had to use `--no-verify` citing "pre-existing
reactAction.test.ts type errors". Executed in this session with the diff applied:
`pnpm exec tsc -p tsconfig.json --noEmit` in `apps/web` → **exit 0**;
`pnpm exec tsx --test src/lib/reactAction.test.ts` → **6/6 pass**. One cosmetic note: the diff also
deletes a 2-line explanatory comment ("And maxAttempts=2 DOES let it land…") — harmless, the
assertion message still carries the intent.

**Would committing make verify-all pass on a clean checkout?** For the P7 failure — yes, traced
above (absent + unreferenced ⇒ GREEN; nothing else in the paleo tree references serious-gate
functionally: `.claude/settings.json` registers 9 hooks, none of them serious-gate; remaining greps
are comments in `guardrail-gate-armament.mjs:4,53`, `guardrail-hook-matchers.mjs:5,22`,
`meta-controller.mjs`, a stale backup `.claude/settings.json.bak-selfimprove`, and a log). Whole
`verify:all --ci` green is not claimable from static analysis alone (other steps run typecheck-class
work and this environment has a known-broken `pnpm lint:gates`); Phase 1 includes the run as the
landing proof.

### 2.2 GH #9 mapped against today's trees — headline claim now STALE

Issue #9 (OPEN, filed 2026-07-02 by the plane-maintainer against `origin/main @ a84f6d7`):
"verify:all / plane-guard reference 9 guardrail scripts missing from main — 3 real hard fails,
red-line adjacent"; 6 of 9 lived only on `chore/design-system-prune`, 3 were found nowhere.

Verified today (`git cat-file -e origin/main:<path>` for each, `origin/main` = `c8b2d5a0`,
2026-07-03): **all 9 exist on origin/main** — guardrail-deliver-v2, guardrail-corpus-reachability,
guardrail-license, guardrail-hook-matchers, guardrail-definer-search-path, guardrail-gate-armament,
guardrail-ledger-integrity, loops-registry-sync, agent-health-pass. Additionally on main:
`verify-all.ts` present, `plane-guard.mjs` present with the OLD P7 row, **and**
`.claude/hooks/serious-gate.sh` present with the `gate-armament` token in main's verify-all — so
old-P7 is coherent and green *on main*. The design-system-prune landing between `a84f6d7` and
`c8b2d5a0` resolved the issue's 3 hard fails. **Issue #9 should be closed with proof** (draft
comment in Phase 1.6). The only surviving instance of its class is this gap's uncommitted P7 fix
on the sovereign/paleo lineage — and the fixed P7 reaches `main` only via the rewrite-aware
lineage merge (audit §7.4; G07 territory).

### 2.3 Doc-staleness inventory (each claim checked against ground truth)

| Doc | Stale claim | Ground truth (verified) | Verdict |
|---|---|---|---|
| `DeliveryOS-As-Built-Summary-v1.md` (repo root; "Audit date 2026-06-04") | "HS256 JWT" (lines 28, 57, 92); "67 migrations — 001…067" (lines 51, 78); Supabase-Free/N=1 stack | Auth is **RS256, alg-pinned** (`packages/platform/src/auth/jwt.ts:15,22,56,106,109-110` — verify rejects non-RS256); **162 files** in `packages/db/migrations/` (highest `1790000000086_…`); a Rust rewrite lives under `rebuild/` | Two product-generations stale; keep as history, banner + demote (Phase 3.1) |
| `CONTEXT-INDEX.md` (root; "Updated 2026-06-08") | As-Built = "**START HERE** … always loaded … trumps planning docs"; points at `graphify-out/` (**MISSING** — verified, only dead pointer in its table; all 15 other paths exist), `mempalace` (never installed per MEMORY-MAP), `.agents/rules`+`skills` (superseded by the `.claude/` harness) | Live entry points are README / docs/ARCHITECTURE.md / AGENTS.md / .claude/CLAUDE.md; live memory is the `.claude` corpus (152+ files) | Rewrite in place (Phase 3.2) |
| `MEMORY-MAP.md` (root; "Updated 2026-06-08") | As-Built "authoritative for code reality"; memory = this markdown map | Same as above; the living-memory corpus + MEMORY.md index is the real memory system | Banner (Phase 3.3) |
| `project-state-2026-07-08.md` — **actual location** `/root/.hermes/skills/software-development/dowiz-operating-system/references/project-state-2026-07-08.md` (OUTSIDE the repo — it is a Hermes skill reference, not a repo/memory file) | "Authoritative Project State … MVP is SHIPPING-READY … READY TO SHIP" | `HANDOFF-2026-07-07-SESSION.md:71`: "MVP is only ~40% complete (5 of 12 phases done)"; exit gate never closed — full adjudication already exists in blueprint **G06 §1** | Correction banner only; substance is G06's (Phase 3.4) |
| `docs/design/harness/META-CONTROLLER.md:33` | Lists `serious-gate` among immutable authority hooks | Hook deleted `f1255ad5` 07-07; only the paleo worktrees still carry it | Exact replacement line ready (Phase 2) |
| `scripts/meta-controller.mjs:46,68,298` (code echoes of the same) | Immutable-core regex + systems-map row still name `serious-gate` | Same removal | **Recommend KEEP the regex** — refusing `serious-gate.sh` as a proposal target is now an anti-resurrection tripwire that *supports* P7; annotate with comments only (Phase 2, decision point 3). `meta-controller.test.mjs` does not probe the serious-gate path, so either choice keeps 9/9 green |
| `scripts/run-armaments.sh` (label, ~line 27) | Echo label "gate-armament (serious/red-line/guard-bash armed…)" | serious-gate removed; the guardrail itself was updated in `f1255ad5` | 1-word label fix, ride along in commit A (Phase 1.2) |
| `.claude/settings.json.bak-selfimprove` | Stale backup still registering serious-gate | Orphan-state class (fable-audit #13 deleted its siblings) | Delete (Phase 3.5, operator-applied) |
| Spot-checked OK (no action) | — | `README.md` (current positioning, links TRADEMARK.md, no dead stack claims found), `docs/ARCHITECTURE.md` (current-gen contributor map → `docs/adr/`), `AGENTS.md` (continuously updated, rules through 07-05), `HERMES.md` (re-synced 07-10, `e5eb3d03`) | — |

**EXPANSION-PLAN alignment:** Layer 0.3 (`docs/design/dowiz-brand/EXPANSION-PLAN.md:24`) =
"README / SECURITY.md truth-pass … zero dead links; zero marketing; agent-readable arch".
Phase 3 here IS the entry-point half of that gate (dead links + agent-readable truth); note that
**SECURITY.md does not exist anywhere in the repo yet** (verified) — authoring it stays in 0.3
proper and is NOT annexed by G13. Phase 3's proof artifact (the link-existence check) should be
cited when 0.3 is closed so the work counts once.

### 2.4 The "unlanded follow-ups" are LANDED — audit §7.7 is falsified on this point

- **guard-bash 83%-FP fix: LANDED.** `.claude/hooks/guard-bash.sh:69-96` is the target-based
  rewrite (PROTECTED matched against write TARGETS only — redirect destinations + mutator
  path-args on a quote-stripped skeleton; `/tmp/claude-*` whitelisted; the ~83% FP history is
  documented in the file itself at line 69). Landed in `4077c11d` ("feat(harness): Fable-audit
  top-5 + Verified-by-Math + …"), which `git merge-base --is-ancestor` confirms is an **ancestor of
  the paleo HEAD**. Measured proof recorded in
  `docs/operating-model/fable-audit-findings-2026-07-07.md` STATUS ✅3: "0/7 FP on the over-block
  corpus, 0/5 missed real blocks (`scripts/probe-system-comparison.mjs`)", fixtures in
  `guardrail-gate-armament.mjs`.
- **loop-registry parity circuit: LANDED.** `scripts/guardrail-loop-registry-parity.mjs` exists
  (6,186 bytes) and is wired TWICE in `scripts/run-armaments.sh` (self-test + live), plus the
  red-line forbid-circuits for removed-machinery refs; STATUS ✅4 records rows 13/21 demoted to
  DRAFT, 10 bogus citations blanked, 14 files fixed. Same commit `4077c11d`.
- **Why the audit missed it:** unclear — possibly checked `origin/main` (where neither exists;
  main tip 07-03 predates them) or trusted the "NEXT-SESSION top-5" framing in
  `ground-truth-over-proxy-2026-07-07.md` without reading the findings doc's later STATUS section.
  Either way §7.7's sentence "guard-bash 83% FP fix, loop-registry parity circuit — no evidence
  landed" is **CONTRADICTED** by tree + ancestry + recorded measurements.
- **Honest residual:** both live only on the sovereign/paleo lineage, not on `origin/main` — the
  same lineage-merge dependency as everything else (G07).

### 2.5 Stale 07-02 worktrees (G12 coordination point — evidence only, no disposition here)

Both confirmed carrying the deleted hook: `/root/dowiz-wt-phase0/.claude/hooks/serious-gate.sh`
and `/root/dowiz-wt-phase5/.claude/hooks/serious-gate.sh` (each 4,660 bytes, mtime Jul 2;
branches `feat/phase0-hardening @ 7a4f7aca` / `feat/phase5-adaptive-gps @ 07894df1`), and both
**register it** at their `.claude/settings.json:39`. Under the new P7 both trees evaluate RED
(demonstrated, §2.1) — which is correct and useful: they are the standing RED fixtures for P7's
falsifiability proof. Harvest-or-prune is G12's blueprint; the only G13 ask of G12: do not delete
both worktrees before Phase 1's RED-case proof is captured (or capture it from this doc's table).

---

## 3. Options & tradeoffs

**A. Land the three diffs as-is (no amendment).**
Fast, minimal review surface. Keeps the dead RED leg (§2.1a) — a documented VbM hole in a gate
whose own comments invoke VbM — and a misleading failure message. Would need a follow-up commit
and a second ledger row anyway.

**B. Amend the 2 P7 lines, then land (RECOMMENDED).**
One extra minute of review; the gate's declared RED cases all become reachable; the worktree
settings.json provides a free real-world RED fixture. Slightly enlarges an already-reviewed diff —
acceptable because the amendment is itself proven by an executed red/green probe.

**C. Split verify-all.ts out of commit A.**
No value — it is documentation of the same removal and not load-bearing; splitting doubles gate
runs.

**Doc dispositions (per doc, chosen in §2.3):** update-in-place (CONTEXT-INDEX — small, heavily
habit-referenced, a rewrite is cheaper than teaching everything a new path), archive-with-banner
(As-Built, MEMORY-MAP — historically valuable v1-era records; deleting would orphan inbound links
incl. CONTEXT-INDEX history and the 06-19 update chain), correction-banner-only
(project-state — the substance belongs to G06; G13 only stops the false label), delete
(`.claude/settings.json.bak-selfimprove` — pure orphan debris, its content is recoverable from git
history of settings.json). Deleting As-Built outright was rejected: audit §7.2 needs it citable.

**Follow-ups disposition:** close-as-landed with re-run proof (recommended) vs re-scope
(unwarranted — measurements exist and re-ran green as recently as the 14/14→17/17 armament suite)
vs drop-with-ledger-entry (wrong — nothing to drop).

---

## 4. Recommended execution blueprint

Format per step: **Action / Gate marker / VbM proof / Effort**. All commits on
`feat/paleo-dinosaur-digs` (already dirty with exactly these changes; no branch dance needed).
Contextual-commit style per repo convention. Nothing here touches red-line globs
(auth/money/RLS/migrations) — the edited files are harness scripts, a test, and docs.

### Phase 0 — pre-flight (before any staging)

- **0.1** Run `bash scripts/run-armaments.sh` (17 armaments, incl. falsifiable-proof +
  loop-registry-parity + gate-armament with the guard-bash fixtures) and `node
  scripts/plane-guard.mjs` on the dirty tree.
  **Gate:** all armaments green; plane-guard verdict PASS with P7 line "absent + unreferenced".
  (Note: plane-guard writes a run artifact to `loops/runs/` — gitignored scratch, expected.)
  **VbM:** plane-guard exit code 0 is the assertion; its P7 RED case is §2.1's worktree table.
  **Effort:** 5 min.
  (Verified in advance for this blueprint: `guardrail-falsifiable-proof.mjs` scans only the
  proofs enumerated in `run-armaments.sh` — plane-guard is not on that surface, so the P7 rewrite
  cannot trip it; plane-guard retains `process.exit(1)` paths regardless.)

### Phase 1 — land the three diffs

- **1.1 Amend P7 (Option B).** Apply the 2-line regex + message fix from §2.1a to
  `scripts/plane-guard.mjs`.
  **Gate:** none yet (working tree).
  **VbM proof (red AND green, both read-only one-liners):**
  green — the §2.1 predicate against `/root/dowiz` → `p7ok:true`;
  red (dangling-registration leg, previously dead) —
  `node -e "const s=require('fs').readFileSync('/root/dowiz-wt-phase0/.claude/settings.json','utf8'); console.log(s.includes('serious-gate.sh'))"`
  → `true` while the script is absent in a synthetic fixture dir ⇒ p7ok false. Paste both outputs
  in the commit body.
  **Effort:** 10 min.
- **1.2 Commit A — harness truth:** `scripts/plane-guard.mjs` + `scripts/verify-all.ts` + the
  1-word `scripts/run-armaments.sh` label fix (§2.3). Suggested subject:
  `fix(plane-guard): P7 asserts serious-gate CLEANLY REMOVED, not present (council purge f1255ad5); de-red every post-purge checkout`.
  None of these paths match the pre-commit BUILD_RELEVANT regex (`^(apps|packages)/…`), so the
  dynamic-scope pre-commit runs only the cheap whole-tree guardrails — no typecheck/build/Docker.
  **Gate marker:** pre-commit PASS **without** `--no-verify` (eslint staged, corpus-reachability,
  license, hook-matchers, run-armaments 17/17, definer-search-path, no-set-cookie,
  sandbox-staleness).
  **VbM proof:** post-commit `node scripts/plane-guard.mjs` → verdict PASS, P7 green; RED case =
  §2.1 worktree table + the 1.1 fixture, cited in the commit body.
  **Effort:** 15 min.
- **1.3 Commit B — test strictness:** `apps/web/src/lib/reactAction.test.ts`. Suggested subject:
  `test(web): reactAction non-null assertions for noUncheckedIndexedAccess (closes the 77811204 --no-verify debt)`.
  This path IS build-relevant ⇒ full pre-commit (typecheck/build/Fly/Docker — the known >8-min
  class, audit §6.4). Run it in an environment with Docker available, or accept the wait.
  **Gate marker:** full pre-commit PASS without `--no-verify` (this retires the recorded
  `77811204` bypass debt). Fallback ONLY if the environment cannot run Docker: `--no-verify` with
  the justification line in the commit body + compensating proof pasted (next line) — decision
  point 2.
  **VbM proof:** `cd apps/web && pnpm exec tsc -p tsconfig.json --noEmit` → exit 0 AND
  `pnpm exec tsx --test src/lib/reactAction.test.ts` → 6/6 pass (both already demonstrated in this
  research); RED anchor = `77811204`'s recorded pre-existing type errors.
  **Effort:** 10 min + pre-commit wall-clock (up to ~10 min).
- **1.4 Ledger row.** Append to `docs/regressions/REGRESSION-LEDGER.md` (last row = #89):
  `| 90 | **plane-guard P7 asserted the EXISTENCE of deliberately-removed machinery (serious-gate.sh), hard-failing every post-f1255ad5 checkout — a gate that reds on the correct state is a false-positive metric (VbM); its replacement's settings.json regex was RED-case-dead against the real registration syntax** | P7 rewritten to assert clean removal (absent + unreferenced); registration check = JSON substring (comments impossible in JSON); RED fixtures = 07-02 worktrees (present) + dangling-registration fixture | <commit A hash> |`
  **Gate marker:** `node scripts/guardrail-ledger-integrity.mjs` green (unique #N).
  **VbM:** the guardrail is the proof.
  **Effort:** 5 min. (Docs-only ⇒ cheap pre-commit; may be folded into commit A instead.)
- **1.5 Push the branch** (`git push -u origin feat/paleo-dinosaur-digs`). Protects the only copy
  of these fixes; also supersedes the red pushed `origin/feat/sovereign-core-phase-zero` tip
  (`330ff4ed` — old P7, no serious-gate) since paleo contains it. No force-push anywhere.
  **Gate marker:** operator consent to push (repo rule: commit/push on ask).
  **VbM:** `git ls-remote origin feat/paleo-dinosaur-digs` returns the new tip hash.
  **Effort:** 2 min.
- **1.6 Close GH #9** with an evidence comment (draft):
  > Status 2026-07-11: all 9 referenced scripts now exist on `origin/main` @ `c8b2d5a0` (verified
  > per-path via `git cat-file`); `verify-all.ts`, `plane-guard.mjs`, the gate-armament wiring
  > token and `.claude/hooks/serious-gate.sh` are all present on main, so the 3 hard fails
  > reported here are resolved on the current main tip (the design-system-prune landing between
  > `a84f6d7` and `c8b2d5a0` closed the gap). The surviving instance of this class was the
  > post-council-removal P7 drift on the dev lineage — fixed by <commit A>. Residual: the fixed P7
  > reaches main only via the rewrite-aware lineage merge (2026-07-11 audit §7.4, blueprint G07).
  > Closing as resolved-on-main.
  **Gate marker:** issue state CLOSED.
  **VbM:** the per-path `git cat-file -e origin/main:<script>` list (9/9 YES) pasted in the comment.
  **Effort:** 5 min.

### Phase 2 — META-CONTROLLER.md correction (exact text, ready to apply)

- **2.1** Replace `docs/design/harness/META-CONTROLLER.md` line 33 — current:
  ```
  | **Authority hooks** (`protect-paths`, `red-line-doubt-gate`, `serious-gate`, `guard-bash`, `require-classification`) | The deterministic enforcement layer. Advisory signals never rewrite enforcement. |
  ```
  with:
  ```
  | **Authority hooks** (`protect-paths`, `red-line-doubt-gate`, `guard-bash`, `require-classification`) | The deterministic enforcement layer. Advisory signals never rewrite enforcement. (`serious-gate` was DELIBERATELY REMOVED 2026-07-07 — operator `f1255ad5`, "ground truth over proxy reasoning". plane-guard P7 now asserts it stays cleanly removed, and the controller still REFUSES `serious-gate.sh` as a proposal target — an anti-resurrection tripwire, not a live hook. Re-enabling council therefore requires one reviewed diff that re-adds the hook AND rewrites P7.) |
  ```
- **2.2 (optional, decision point 3)** Comment-only annotations in `scripts/meta-controller.mjs`:
  at line 46 append `// serious-gate REMOVED f1255ad5 2026-07-07 — kept in this regex as an
  anti-resurrection tripwire (proposals targeting it are refused), NOT a live hook`; align the
  line-298 systems-map row text similarly. **No behavioral change** — the regex stays.
- **Gate marker:** docs/comment-only ⇒ cheap pre-commit; `node --test
  scripts/meta-controller.test.mjs` → 9/9 (verified: the test suite does not probe the
  serious-gate path, so it stays green either way).
  **VbM proof:** `grep -n "serious-gate" docs/design/harness/META-CONTROLLER.md` returns ONLY
  lines containing `REMOVED`/tripwire annotation (a checkable predicate; goes RED if the old row
  text survives).
  **Effort:** 15 min total.

### Phase 3 — doc truth-pass (counts toward EXPANSION-PLAN Layer 0.3)

- **3.1 As-Built: banner + keep (archive-in-place).** Insert at the very top of
  `DeliveryOS-As-Built-Summary-v1.md`:
  ```
  > ⚠️ HISTORICAL — SUPERSEDED (annotation 2026-07-11). This v1 audit describes the 2026-06-04
  > codebase, two product-generations ago. Known-stale claims: "HS256 JWT" (auth is RS256,
  > alg-pinned — packages/platform/src/auth/jwt.ts); "67 migrations" (now 162 in
  > packages/db/migrations/); the §2 stack (a Rust rewrite lives under rebuild/). Do NOT use as
  > "code reality" and do NOT treat as START-HERE. Current entry points: README.md,
  > docs/ARCHITECTURE.md, AGENTS.md; latest verified ground truth:
  > docs/research/2026-07-11-full-project-audit-dowiz-bebop.md. Kept unedited below as the v1
  > pilot-readiness record (the 2026-06-19 update note is likewise historical).
  ```
  **Gate:** cheap pre-commit. **VbM:** `head -3` of the file matches the banner; the audit §7.2
  citation chain stays intact. **Effort:** 10 min.
- **3.2 CONTEXT-INDEX.md: rewrite in place** (it is the doc whose job is to be current). Proposed
  full replacement body:
  ```
  # DeliveryOS — Context Index

  > Thin entry-point index. Rewritten 2026-07-11 (truth-pass, EXPANSION-PLAN Layer 0.3).
  > The 2026-06-08 version pointed at retired machinery (graphify, mempalace, .agents/, the v1
  > As-Built as "START HERE") — see git history.

  ## START HERE
  1. README.md — what the product is (0-commission branded storefronts).
  2. docs/ARCHITECTURE.md — system map; docs/adr/ for the why.
  3. AGENTS.md + .claude/CLAUDE.md — agent rules, ethics charter, gates (the harness).
  4. docs/research/2026-07-11-full-project-audit-dowiz-bebop.md — latest verified ground truth.
  5. docs/design/dowiz-brand/EXPANSION-PLAN.md — declared north star;
     docs/design/gap-blueprints-2026-07-11/ — execution blueprints per audit gap.

  ## Quick lookup
  | You need... | Go to... |
  |---|---|
  | Code reality / is it built | the 2026-07-11 audit §4 (evidence-linked) |
  | Harness / gates | scripts/verify-all.ts · scripts/plane-guard.mjs · .claude/settings.json hooks · scripts/run-armaments.sh |
  | Living memory | /root/.claude/projects/-root-dowiz/memory/ (MEMORY.md index; HERMES.md mirror) |
  | Migrations | packages/db/migrations/ (162; drafts 085–089 under docs/design/*/migration-drafts/) |
  | Security posture | audit §7.9 + docs/audit/vulnerabilities.md (historical) |
  | Historical v1-era docs | DeliveryOS-As-Built-Summary-v1.md · MEMORY-MAP.md (both banner'd SUPERSEDED) |
  ```
  **Gate:** cheap pre-commit.
  **VbM (the 0.3 "zero dead links" predicate):** run a link-existence loop over every path named
  in the new file (the same probe used in this research — 15/16 existed, `graphify-out/` was the
  one dead pointer and is now gone): `for p in <paths>; do test -e "$p" || echo "DEAD: $p"; done`
  → empty output; RED case: add a bogus path → non-empty. Paste output in the commit body and cite
  it when Layer 0.3 closes. **Effort:** 20 min.
- **3.3 MEMORY-MAP.md: banner** (same pattern as 3.1):
  ```
  > ⚠️ SUPERSEDED (2026-07-11). Memory lives in the .claude living-memory corpus
  > (/root/.claude/projects/-root-dowiz/memory/ — MEMORY.md index, HERMES.md in-repo mirror), not
  > in this map. The "As-Built = authoritative for code reality" rule below is retired (that doc
  > is banner'd historical). Kept as the 2026-06-era record.
  ```
  **Gate/VbM/effort:** as 3.1; 5 min.
- **3.4 project-state-2026-07-08.md (Hermes skill file, OUTSIDE the repo — no repo gates apply).**
  Prepend:
  ```
  > ⚠️ CORRECTION (2026-07-11): "SHIPPING-READY" below is CLAIMED-UNVERIFIED. The same week's
  > HANDOFF-2026-07-07-SESSION.md records "MVP ~40% (5 of 12 phases)" and the GRAND-PLAN exit gate
  > was never closed. Adjudication + closure plan:
  > /root/dowiz/docs/design/gap-blueprints-2026-07-11/G06-sovereign-core-exit-gate.md.
  ```
  **Gate marker:** none (out-of-repo); note in the session log.
  **VbM:** `head -1` of the file contains `CORRECTION` — trivially checkable.
  **Effort:** 5 min.
- **3.5 Orphan hygiene (operator-applied):** delete `.claude/settings.json.bak-selfimprove` (stale
  backup still registering serious-gate — same class as the fable-audit #13 orphans already
  deleted). If protect-paths blocks the deletion, use the stage-and-apply human pattern.
  **Gate:** protect-paths/human. **VbM:** `test ! -e .claude/settings.json.bak-selfimprove`.
  **Effort:** 2 min.

### Phase 4 — disposition of the "unlanded" follow-ups: CLOSE AS LANDED (no re-scope, no drop)

- **4.1** Re-run the live proof: `bash scripts/run-armaments.sh` → all 17 green, which exercises
  BOTH follow-ups (loop-registry-parity self-test = lying-cert + bogus-citation go RED;
  gate-armament carries the guard-bash FP/true-block fixtures).
  **Gate marker:** run-armaments exit 0. **VbM:** each armament's self-test asserts its own RED
  case by construction (enforced by `guardrail-falsifiable-proof.mjs`). **Effort:** 5 min.
- **4.2** Correct the record so §7.7's error does not propagate into other gap work: append a
  dated erratum to the audit doc (decision point 5):
  ```
  > ERRATUM (2026-07-11, G13 research): §7.7's "guard-bash 83% FP fix, loop-registry parity
  > circuit — no evidence landed" is WRONG. Both landed 2026-07-07 in `4077c11d` (ancestor of the
  > paleo tip): guard-bash.sh:69-96 target-based matching (measured 0/7 FP, 0/5 missed —
  > fable-audit-findings STATUS ✅3) and scripts/guardrail-loop-registry-parity.mjs wired twice in
  > run-armaments.sh (STATUS ✅4). Neither is on origin/main yet — that is the lineage-merge gap
  > (§7.4), not an unbuilt follow-up.
  ```
  **Gate:** docs-only pre-commit. **VbM:** grep for `ERRATUM` + the commit hash in the audit doc.
  **Effort:** 5 min.
- **4.3** Ledger/memory: no new guardrail needed (both follow-ups already carry fixtures + wiring);
  add one line to the G13 session memory noting CLOSED-VERIFIED-ON-LINEAGE with the `4077c11d`
  citation, residual tracked under G07 (reach main via the rewrite-aware merge). **Effort:** 5 min.

**Total effort: ~2 hours wall-clock** (dominated by commit B's full pre-commit), all reversible,
zero red-line surfaces.

---

## 5. Risks & rollback

- **R1 — P7 semantic inversion is a one-way door for casual re-enable.** After commit A, restoring
  council (per `council-gate-disabled-2026-07-05.md`) reds P7 until P7 is rewritten in the same
  diff. This is intended ratchet friction and is now documented in META-CONTROLLER.md (Phase 2).
  Rollback: `git revert <commit A>` restores the old existence-assertion (which then hard-fails
  again until serious-gate.sh returns — reverting alone does NOT produce a green tree; revert only
  as part of a full council resurrection).
- **R2 — commit B's full pre-commit (>8-min class, Docker/Fly steps).** Known hang class (audit
  §6.4). Mitigation: commits split so only B pays it; run B where Docker exists. Fallback
  `--no-verify` ONLY with pasted compensating proof (tsc exit 0 + 6/6 tests) — precedent
  `77811204`, and this commit retires that very debt, so prefer the clean path.
- **R3 — guardrail interaction surprises.** Pre-flight (Phase 0) runs the entire armament suite on
  the dirty tree BEFORE staging, so any interaction (falsifiable-proof, no-orphan, circuits
  --staged) fires pre-commit-shaped, not mid-commit. Verified in advance: falsifiable-proof's
  surface is run-armaments.sh entries only; plane-guard is not on it.
- **R4 — substring check false-RED risk (amended P7).** `settingsBody.includes('serious-gate.sh')`
  reds on ANY occurrence in `.claude/settings.json`. JSON admits no comments, so a benign-prose
  occurrence is impossible in a parseable settings file; documented in the code comment.
- **R5 — protect-paths on doc/state edits.** META-CONTROLLER.md lives under `docs/design/harness/`
  and 3.5 deletes a `.claude/` file; if protect-paths denies, use the designed stage-and-apply
  human pattern (agent stages a proposed copy, operator `cp`s it — same as the 07-05 settings
  change). No gate is ever bypassed.
- **R6 — worktrees read RED under new P7.** Expected and correct (they hold the deleted hook);
  not a regression. G12 owns their fate; capture the RED transcript before any prune (§2.5).
- **R7 — none of this reaches origin/main.** True by design: main is hash-bifurcated (audit §7.4).
  G13 makes the dev lineage internally truthful; the merge itself is G07's blueprint. Do not
  attempt a partial cherry-pick to main from here.
- **Rollback general:** every phase is an isolated, single-purpose commit; `git revert` any of
  them independently. Banners are additive (no original text destroyed); CONTEXT-INDEX's old body
  survives in git history and is referenced from the new header.

---

## 6. Operator decision points

1. **P7 amendment before landing (Phase 1.1)** — Option B (amend the dead regex leg + message,
   recommended) vs land-as-is + follow-up. Default: B.
2. **Commit B pre-commit path** — wait out the full Docker/Fly pre-commit (recommended, retires
   the 77811204 `--no-verify` debt cleanly) vs `--no-verify` + pasted compensating proof if this
   environment cannot run Docker. Default: full path.
3. **meta-controller.mjs serious-gate regex (Phase 2.2)** — KEEP as anti-resurrection tripwire
   with comment annotation (recommended; zero behavior change, test stays 9/9) vs prune the token
   from the regex (cleaner text, loses the tripwire). Default: keep.
4. **As-Built / MEMORY-MAP disposition** — banner-in-place (recommended) vs move to a
   `docs/attic/`. Default: banner (preserves inbound links and the audit's citations).
5. **Audit-doc erratum (Phase 4.2)** — append the dated ERRATUM block to the 2026-07-11 audit
   (recommended: other gap blueprints are actively citing §7.7) vs memory-note only. Default:
   append.
6. **Push + GH #9 closure (Phases 1.5/1.6)** — push `feat/paleo-dinosaur-digs` and close #9 with
   the evidence comment. Push needs your explicit go per repo rule. Default: yes to both.
7. **EXPANSION-PLAN 0.3 credit** — when 0.3 is worked, count Phase 3's link-existence proof toward
   its "zero dead links" criterion (this blueprint's §2.3/§3.2 are the citation); SECURITY.md
   authoring remains inside 0.3, not G13.
