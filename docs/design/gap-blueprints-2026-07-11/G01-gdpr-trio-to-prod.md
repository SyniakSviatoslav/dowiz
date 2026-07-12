# G01 — Ship the GDPR/Webhook Trio to Prod (Execution Blueprint)

> **Status:** DESIGN — nothing executed. Read-only research session 2026-07-11; every claim below
> re-verified against git objects, the live GitHub API, and live prod probes **today**.
> **Source audit:** `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md` §4.3, §7.3, §7.4, §9 rec 1.
> **Severity:** highest in the project — live GDPR Art.17 legal exposure on prod (ETHICAL-STOP class).

---

## 1. Gap & evidence

Three staging-validated fixes are **not ancestors of `origin/main`** (prod deploys from main tip
`c8b2d5a0`, live as image v410+ lineage, `dowiz.fly.dev`):

| Commit | What it fixes | Prod today (verified in `origin/main` tree) |
|---|---|---|
| `5ded9f19` (2026-07-04) | GDPR order erasure never purged `delivery_photo_key` column NOR the R2 doorway-photo object — customer photo survives its own erasure, public-by-key, forever. Ledger #74, S4 council ETHICAL-STOP lift REV-S4-7. | `git show origin/main:apps/api/src/lib/anonymizer/index.ts` → **0** hits for `delivery_photo_key` |
| `58caf4f4` (2026-07-05) | Art.17 customer erasure never reached the subject's own orders (GAP-A); `orders.delivery_lat/lng` (home GPS, GAP-B) and `order_ratings.feedback` (GAP-C) erased by NO path; completion gate re-read only `customers.anonymized_at` → false `completed`; `gdpr_erasure_requests.subject_phone` never erased (REV-S9-5). Ledger #76, 3-seat S9 council. | `origin/main:apps/api/src/workers/anonymizer-gdpr.ts` writes `completed` with **no** subject-graph re-read; no fan-out in `anonymize()` |
| `d6b3473e` (2026-07-05) | Telegram webhook header secret failed **OPEN** on missing header + logged the received token + non-constant-time compare. Ledger #75. | `origin/main:apps/api/src/routes/telegram-webhook.ts:59` — literally `// Process the request anyway for backward compat` |

Non-ancestry proven: `git merge-base --is-ancestor <sha> origin/main` → **NOT ancestor**, all three
(run 2026-07-11). All three exist on GitHub (`gh api repos/SyniakSviatoslav/dowiz/commits/<sha>` → 200)
on branch `fix/audit-remediation`.

Additionally **anonymizer GDPR gap #1** — the `client_ip_hash` copy inside `orders.metadata` — is
still open **everywhere** (main, fix branch, paleo). Verified: `apps/api/src/lib/order-persistence.ts:103`
writes `JSON.stringify({ otp_verified…, client_ip_hash: input.clientIpHash, channel… })` into
`orders.metadata` at create; no erasure path touches `orders.metadata`; ledger row #74's own text calls
the photo purge "anonymizer-completeness gap **#2** (after the orders.metadata ip-hash copy)".

Prod baseline (probed 2026-07-11, this session): `livez` 200 / 0.23s; `/health` → `postgres ok,
workers ok, messageBus ok, telegram ok, r2 ok, anonymizer ok, backup ok; degraded` only on `fallback`.

---

## 2. Research findings (all verified this session)

### F1. The "rewrite-aware merge" problem does NOT apply to this gap — the trio lives on a clean, same-lineage branch

- `fix/audit-remediation` forks **exactly** at origin/main's tip: merge-base = `c8b2d5a0` = origin/main
  tip; GitHub compare `main...fix/audit-remediation` → **ahead 97 / behind 0** (verified live via
  `gh api .../compare`). `origin/main` **is an ancestor** of all three fix commits.
- The 500+ add/add conflict problem (HANDOFF-2026-07-07) exists only for the **post-scrub** branches:
  `git merge-base origin/main HEAD(feat/paleo-dinosaur-digs)` → **no common ancestor**. Those branches
  are NOT needed as a source: the paleo tree's copies of the fixed files are **byte-identical** to the
  fix commits (see F2), so the pre-scrub `fix/audit-remediation` is a complete, hash-compatible source.

### F2. The fixed files have not drifted anywhere — blob-level identity across three lineages

| File | origin/main | post-fix (58caf4f4 / d6b3473e) | fix-branch GitHub tip `914acf8a` (07-11) | paleo HEAD |
|---|---|---|---|---|
| `apps/api/src/lib/anonymizer/index.ts` | `22ff4f84` | `26618551` | `26618551` (via contents API) | `26618551` |
| `apps/api/src/workers/anonymizer-gdpr.ts` | `c53b4faf` | `7f77d6f5` | `7f77d6f5` | `7f77d6f5` |
| `apps/api/src/routes/telegram-webhook.ts` | `94ce9e6c` | `50d49864` | `50d49864` | `50d49864` |

No later commit anywhere supersedes the fixes. (Note: the local remote-tracking ref
`origin/fix/audit-remediation` is stale at `c8723947`; GitHub's real tip is `914acf8a`, 2026-07-11 —
a plane-maintainer docs commit; file blobs unchanged. **Fetch before executing.**)

### F3. Straight cherry-picks CONFLICT — proven analytically with `git merge-tree` (no worktree writes)

- `git merge-tree --write-tree --merge-base=d6b3473e~1 origin/main d6b3473e` → exit 1, content
  conflicts in `telegram-webhook.ts` + `REGRESSION-LEDGER.md`.
- `git merge-tree --write-tree --merge-base=5ded9f19~1 origin/main 5ded9f19` → exit 1, content
  conflict in `anonymizer/index.ts`, modify/delete on `docs/design/rebuild-media-s4-council/resolution.md`,
  content conflict in the ledger.
- Root cause: the trio sits on top of intermediate commits absent from main —
  dependency chains per file (`git log c8b2d5a0..fix-tip -- <file>`):
  - anonymizer: `bd94083b` (35 files, +2513 — **money/authz/state-machine, red-line, must NOT ride along**) → `5ded9f19` → `58caf4f4`
  - worker: `bd94083b` → `69ad3074` (13 files — adds the #61 fail-loud backstop the S9 fix extends) → `58caf4f4`
  - webhook: `dc0d8aab` (30 files, +2779 — dark TMA feature; module `apps/api/src/notifications/telegram-mini-app.ts` is **ABSENT on main**) → `d6b3473e`

### F4. The curated file-set IS safe to drop onto main verbatim (except the webhook — surgical diff there)

- Anonymizer blob `26618551` + worker blob `7f77d6f5`: every import resolves on main
  (`StorageProvider` in `apps/api/src/ports.ts:18` with `delete()`; `BUS_CHANNELS.ANONYMIZER_GDPR_FAILED`
  at `origin/main:apps/api/src/lib/registry.ts:43`; `CUSTOMER_ANONYMIZED`, `ORDER_ANONYMIZED`,
  `QUEUE_NAMES.ANONYMIZER_GDPR`, `dashboardChannel` — all verified key-by-key). All callers compatible:
  the GDPR worker passes `subject.locationId` (`origin/main:.../anonymizer-gdpr.ts:62`); the retention
  worker blob is **identical in all four lineages** (`f2e8ae58`); `test-stage30.ts` blob identical in all
  lineages and passes `locationId`. The blob's extra `AnonymizeOptions` fields are optional-additive.
- Webhook blob `50d49864` **cannot** be taken verbatim (imports the absent TMA module). But
  `d6b3473e`'s own diff is self-contained: `crypto` import + `secretTokenMatches()` (constant-time,
  length-guarded) + replacement of the fail-open block — and the fail-open pre-image block exists
  verbatim on main. Apply that one diff surgically.
- The 5 guardrail test files are absent on main (add-new, no conflict) and import only
  `node:test` + the two shipped source modules — self-contained.

### F5. **NEW finding beyond the audit — the R2 purge is wired to a no-op in EVERY lineage (trio alone does not close the ETHICAL-STOP)**

- `apps/api/src/server.ts:305-310` builds `storage` (R2 when `R2_BUCKET`+`R2_ENDPOINT` set), but
  `server.ts:348` calls `startBackgroundWorkers({ pool, backupPool, queue, messageBus, notifyWorker })`
  **without it**, and `apps/api/src/bootstrap/workers.ts:100` constructs
  `new AnonymizerService(pool, messageBus)` — **no third argument**. Blob `4520d89c` identical on
  origin/main, `58caf4f4`, fix-branch tip, and paleo HEAD.
- Consequence: in the real worker path, `if (this.storage && deliveryPhotoKey)` is always false →
  the DB column gets NULLed but the **R2 object survives**; `storagePurged` stays 0. (The pre-existing
  `avatar_key` purge has been no-oping the same way.) The commit's unit test passes because it injects
  a mock storage directly.
- Fix is ~6 lines: add `storage: StorageProvider` to `BackgroundWorkerDeps`, pass it at `server.ts:348`,
  use it at `workers.ts:100`. R2 is configured and healthy on prod (**live `/health` → `r2: ok`**),
  so the wiring becomes effective immediately.
- **This blueprint therefore ships FOUR changes, not three.** Without #4 the photo-purge fix is
  cosmetic at the object level.

### F6. Zero migrations, zero env/config changes required

- `git diff --stat c8b2d5a0 fix-tip -- packages/db/migrations/` → **empty**. All columns the fixes touch
  already exist in origin/main's migration set (`1790000000039_order-entry-photo.ts` →
  `delivery_photo_key`; `1780310074262_orders.ts` → `delivery_lat/lng`;
  `1780421100060_anonymization-seam.ts` → `subject_phone`; `1790000000025_order-ratings.ts`), and prod
  is migrated through main's full set (CI deploy `Migrate Database` step succeeded on the v410 saga and
  the two later green runs). Rollback therefore never involves schema.

### F7. CI on origin/main is GREEN today; GH issue #9 is stale at the current tip; push-to-main IS the prod deploy trigger

- Last two CI runs on main: `f7284c23` and tip `c8b2d5a0` → **conclusion: success** (run 28655643050),
  including the `deploy` job (migrate → `flyctl deploy` → post-deploy e2e). Verified via
  `gh api .../actions/runs?branch=main`.
- Issue #9 ("guardrail scripts missing from main — 3 real hard fails") was filed against pre-merge tip
  `a84f6d7`. At `c8b2d5a0` **all nine** scripts exist (`git ls-tree origin/main scripts/` — verified
  file-by-file, incl. `guardrail-ledger-integrity.mjs`, `loops-registry-sync.mjs`;
  `verify-contrast/i18n` live under `apps/api/scripts/`). Every `verify-all.ts` step reference resolves.
  Recommend closing/annotating #9 after this ships.
- `ci.yml`: PRs run `validate` + `fresh-provision` only; **push to main additionally runs `deploy`**
  (migrate + `flyctl deploy --remote-only` + post-deploy e2e). So the operator gate must sit **before
  the merge-to-main push** — merging IS deploying. Deploy-health landmine is closed: boot-fix
  `db30d273`/`5cee7611` (WORKER_BOOT_BUDGET_MS 3s) is on main with two clean auto-deploys after it.
- Post-deploy e2e vs prod is prod-safe: `e2e/tests/telegram-webhook.spec.ts` **skips every test on prod**
  (`f7284c23`), and — key — main's spec **already expects the fail-closed behavior**
  (WEBHOOK-2 `missing secret returns 401`, spec line 53). The spec runs full against staging.

### F8. Ledger mechanics

`scripts/guardrail-ledger-integrity.mjs` asserts **uniqueness only** (gaps fine — prints
"all numbers unique (max #N)"). Main's ledger tops out around #52/#56. Appending rows **#61, #74, #75,
#76** verbatim (four rows; #61 is the fail-loud backstop that ships inside the worker blob) keeps the
guardrail green and preserves the cross-reference numbering used by commit messages and test comments.

### F9. Gap #1 (`orders.metadata.client_ip_hash`) — current state and cheap closure

Open everywhere (F. §1). A salted IP hash is pseudonymized personal data (GDPR Rec. 26) tied to an
erased subject's order. Closure is one line inside `anonymizeOrder`'s existing PII UPDATE:
`metadata = metadata - 'client_ip_hash'` (JSONB key-delete; `otp_verified`/`channel` are not personal
data and stay) + one test arm asserting the key is gone post-erasure and other keys survive.
Recommended to ride in the same vehicle (Phase 1 step 1.5, operator scope decision D5).

### F10. G02 (remote history scrub) interaction — ordering facts

- The 4 secret-bearing commits (`72fde8a6`, `bccfd324`, `d5eef9cb`, `84b95d66`) **are ancestors of
  origin/main** (verified) → the eventual scrub MUST rewrite origin/main itself.
- The 2026-07-05 local scrub twins are **stale and dangerous as push sources**: local `main` is
  `2be9e692` (2026-06-25) — 8 days and the entire 275-commit prod merge **behind** origin/main. A blind
  `git push --force --all` would roll prod's source back weeks and trigger a bad CI auto-deploy.
- GitHub currently has **35 branches** (5 plane-maintainer branches created after the local scrub;
  local remote-tracking refs are stale). The scrub must be **regenerated from a fresh clone of current
  origin**, not replayed from the stale local twins.
- `fix/audit-remediation` (GitHub) holds the trio's provenance — do not prune it before G01 lands.

---

## 3. Options & tradeoffs

| Option | Mechanics | Verdict |
|---|---|---|
| **A. Merge `fix/audit-remediation` → main wholesale** | Clean by lineage (0 behind), one merge | **Reject.** Drags 97 commits: S1–S10 dark Rust surfaces, cutover harness, TMA dark features, `bd94083b`'s 35-file money/authz batch — a massive, council-unratified prod surface change as the vehicle for a legal fix. Blast radius and review burden are wrong by an order of magnitude. |
| **B. Cherry-pick the trio (`git cherry-pick 5ded9f19 d6b3473e 58caf4f4`)** | 3-way replay | **Reject.** Proven conflicts (F3) on every commit; hand-resolving them converges on option C's end state anyway, with murkier provenance and a risk of silently importing `bd94083b` context. |
| **C. Curated file-set commit(s) — "merge by tree, not by hash"** (audit §9 rec 1) | New branch off origin/main; set the 2 anonymizer files to their proven blobs; surgically apply `d6b3473e`'s diff to the webhook; add the 5 guardrail tests; append 4 ledger rows; + the F5 storage wiring fix; 3 small commits, one PR | **RECOMMENDED.** Conflict-free by construction, byte-provably identical to the staging-validated code, zero migrations, independently revertable commits, minimal blast radius. |
| **D. Rebase a subset of the fix branch onto main** | `git rebase --onto` a curated range | Reject — same conflicts as B plus interactive-surgery risk; no benefit over C. |
| **E. Wait for the "big merge" (paleo/sovereign reconciliation)** | Ship when the rewrite-aware program merge is designed | **Reject as the primary plan.** That is the unsolved 500+-conflict problem (F1); the legal exposure is live now. C is independent of and does not complicate the eventual reconciliation (same blobs on both sides ⇒ the future merge sees identical content, not a conflict). |

Sub-decision inside C: **three commits, not one squash** — (1) webhook fail-closed, (2) GDPR erasure
set, (3) worker storage wiring — so each is independently `git revert`-able in prod.

---

## 4. Recommended execution blueprint (Option C, phased)

> Standing rules honored: **Verified-by-Math** (every step has a falsifiable proof with a stated RED
> case), **Ship Discipline** (feature branch → CI → staging rehearsal → prod only on explicit operator
> approval), **red-line marking** (GDPR/legal data-handling + prod deploy + prod DB writes = 🔴 OPERATOR).
> Executor session: normal write permissions on a feature branch; never touch main directly until Phase 4.

### Phase 0 — Preflight (read-only, ~20 min)

| # | Action | Gate | VbM proof (GREEN) | RED case → response |
|---|---|---|---|---|
| 0.1 | `git fetch origin` then re-verify sources: `git rev-parse origin/main` = `c8b2d5a0` (or note the new tip); trio still on GitHub; blobs at fetched `origin/fix/audit-remediation` for the 3 files = `26618551`/`7f77d6f5`/`50d49864` | — | Exact hash equality, 3/3 | Any blob differs → someone changed the fixes upstream → STOP, re-run §2 research before proceeding |
| 0.2 | Prod baseline: `curl /livez` + `/health` | — | livez 200; `r2: ok`, `anonymizer: ok`, `workers: ok` | `r2` not ok → the storage wiring fix would no-op in prod; halt and fix R2 first |
| 0.3 | `gh api .../actions/runs?branch=main&per_page=1` | — | Latest main run `success` | Red main → fix main first; do not stack a legal fix on a red base |
| 0.4 | Confirm prod Telegram webhook registration carries `secret_token` (see Risk R4). Operator checks `getWebhookInfo` (out-of-band; standing memory restricts the agent to `sendMessage` only) or simply plans to re-issue `setWebhook` with `secret_token=$TELEGRAM_BOT_SECRET` in the Phase 4 window | 🔴 OPERATOR | Operator confirms `has_custom_certificate/…/secret_token` semantics: header will be present on Telegram's calls | Unknown/absent → schedule the `setWebhook` re-issue INSIDE Phase 4.2, immediately after deploy; without it the fail-closed check would 401 every real update |

### Phase 1 — Branch construction + local proofs (~1–2 h)

Branch: `git checkout -b fix/gdpr-trio-to-prod origin/main` (record base SHA in the PR body).

| # | Action | Gate | VbM proof (GREEN) | RED case → response |
|---|---|---|---|---|
| 1.1 | **Commit 1 — webhook fail-closed:** `git show d6b3473e -- apps/api/src/routes/telegram-webhook.ts \| git apply -3` (or hand-apply the 3 hunks: crypto import; `secretTokenMatches()`; check-block replacement) | — | `grep -c 'Process the request anyway' apps/api/src/routes/telegram-webhook.ts` = **0**; `grep -c secretTokenMatches` = 2; `grep -c telegram-mini-app` = **0** (no TMA leakage); `pnpm --filter api typecheck` clean | apply conflict → resolve manually per the three hunks; TMA import present → wrong source used, redo from the DIFF not the blob |
| 1.2 | **RED arm for commit 2 (falsifiability first):** add ONLY the 5 test files from `58caf4f4` (`git checkout 58caf4f4 -- apps/api/tests/anonymizer-fail-closed.test.ts apps/api/tests/anonymizer-gdpr-backstop.test.ts apps/api/tests/anonymizer-gdpr-worker-provenance.test.ts apps/api/tests/anonymizer-order-photo-purge.test.ts apps/api/tests/gdpr-erasure-completeness.test.ts`), run them against main's untouched source | — | **Tests FAIL in the documented pattern** (photo-purge ~3/5 arms red: delete never called, column absent from UPDATE SQL, storagePurged 0≠1; completeness ~2/3 red: false `completed`, un-erased order + subject_phone). This reproduces the historical RED→GREEN on main's own lineage — the proof can fail, therefore it proves | Tests PASS against main's code → the gap doesn't exist / research invalid → STOP, escalate to operator with findings |
| 1.3 | **Commit 2 — GDPR erasure set:** `git checkout 58caf4f4 -- apps/api/src/lib/anonymizer/index.ts apps/api/src/workers/anonymizer-gdpr.ts`; hand-append ledger rows **#61, #74, #75, #76** (verbatim from `origin/fix/audit-remediation:docs/regressions/REGRESSION-LEDGER.md`) to main's ledger; optionally add the two council resolution docs (`rebuild-media-s4-council/resolution.md`, `rebuild-gdpr-s9-council/*`) for provenance | — | `git rev-parse :apps/api/src/lib/anonymizer/index.ts` = `26618551…` and worker = `7f77d6f5…` (**byte-identity with the staging-validated blobs — the strongest equivalence proof available**); the Phase-1.2 suite now **all green**; `node scripts/guardrail-ledger-integrity.mjs` green (uniqueness incl. #61/74/75/76) | blob hash mismatch → wrong source ref; any test still red → do NOT hand-patch tests (test-integrity rule), investigate |
| 1.4 | **Commit 3 — storage DI wiring (F5):** add `storage: StorageProvider` to `BackgroundWorkerDeps` (`apps/api/src/bootstrap/workers.ts:32`), destructure it at `:50`, pass to `new AnonymizerService(pool, messageBus, storage)` at `:100`; pass `storage` at the `server.ts:348` call site. Add a wiring guardrail arm (e.g. in `anonymizer-order-photo-purge.test.ts`: source-level assert that `workers.ts` constructs `AnonymizerService` with a storage argument — same technique as `test-stage30.ts` R1.2) | — | New arm RED before the edit, GREEN after (run it before committing the wiring to capture RED); `pnpm --filter api typecheck` + `pnpm -r build` clean | typecheck error → `BackgroundWorkerDeps` consumers elsewhere need the param — extend, don't `any` |
| 1.5 | *(scope decision D5)* **Gap #1 closure:** inside `anonymizeOrder`'s PII UPDATE add `metadata = metadata - 'client_ip_hash'`; extend the photo-purge test: seed `metadata` with `{otp_verified, client_ip_hash, channel}`, assert post-erasure the key is gone AND `channel`/`otp_verified` survive | 🔴 OPERATOR (scope) | Test arm RED before (key survives), GREEN after | If operator defers: file it as a ledger TODO row so it stops being silently carried (audit §9 rec 10 pattern) |
| 1.6 | Full local gate: `pnpm -r typecheck && pnpm -r build && pnpm verify:all --ci && pnpm lint && node --test --import tsx apps/api/tests/*.test.ts` | — | All green (fix-branch history: 638/638 unit; main tip was CI-green so the base is clean) | verify:all failure in an untouched area → base drift, bisect against `c8b2d5a0` |

### Phase 2 — PR + CI (~30 min wall-clock)

| # | Action | Gate | VbM proof (GREEN) | RED case → response |
|---|---|---|---|---|
| 2.1 | Push branch; open PR → `main`. PR body: this doc's link, the 3 source commits, blob hashes, the F5 finding. **PR does NOT deploy** (deploy job gated on `refs/heads/main`) | — | CI `validate` + `fresh-provision` green on the PR. `fresh-provision` boots the API with the new DI wiring against a from-scratch DB — a real boot-level proof of commit 3 | Any red → fix on the branch; never `--no-verify`, never merge red |

### Phase 3 — Staging rehearsal (~1–2 h) — Ship Discipline's staging gate

| # | Action | Gate | VbM proof (GREEN) | RED case → response |
|---|---|---|---|---|
| 3.0 | **Staging overwrite consent:** staging currently hosts the parked sovereign-core/cutover lineage (audit §7.4/§9 rec 7 — its flag state is already 6 days stale). Record current staging image (`flyctl releases -a dowiz-staging`) as the restore point | 🔴 OPERATOR | Restore point SHA/image recorded in the PR thread | Operator declines → NO-GO; there is no other rehearsal environment (Ship Discipline requires staging) |
| 3.1 | Deploy the PR branch to staging via `scripts/deploy-staging.sh` (canonical — carries the VITE build-args). Migration release-command should **no-op** (candidate's migration set ⊆ staging's applied set; staging has 5 extra sovereign migrations whose absence from the candidate dir node-pg-migrate ignores) | — | Deploy green; staging `/health` all-ok; **pre-flight:** release-command logs show 0 pending migrations | Migration step errors on the applied-but-missing names → abort deploy, capture log, redesign with `--no-check-order` analysis; do NOT hand-edit staging's `pgmigrations` |
| 3.2 | **Webhook proof (staging):** run `e2e/tests/telegram-webhook.spec.ts` vs staging (full — spec only skips on prod). Manual arms: POST valid-URL-secret + **no header** → 401; wrong header → 401; correct header → 200 | — | WEBHOOK-1/2/3 green. (Historical RED for WEBHOOK-2 exists at `d6b3473e`; this session's RED equivalent is Phase 1.2's local run. Note: staging's **pre**-deploy state can't serve as RED — the sovereign lineage already contains the fix) | WEBHOOK-2 red (200 on missing header) → fix not in the image → check deployed SHA |
| 3.3 | **GDPR erasure drill — THE ETHICAL-STOP proof:** on staging: (i) create an order with a delivery photo for a test customer (deliver-v2 courier flow, or fixture: `storage.put` a probe object + set `orders.delivery_photo_key`), (ii) add a rating with feedback text, (iii) insert/trigger a `gdpr_erasure_requests` row for that customer, (iv) let the worker run. **PRE-assertion (the drill's own RED-vacuity arm): GET the photo object → it EXISTS before erasure** — if this reads absent, the drill is void, fix the fixture | — | Post-worker, ALL of: (a) `orders` row → `delivery_photo_key` IS NULL, `delivery_lat/lng` IS NULL, `anonymized_at` NOT NULL; (b) `order_ratings.feedback` IS NULL; (c) `gdpr_erasure_requests` → `status='completed'`, `subject_phone` IS NULL, `metadata.storagePurged ≥ 1`; (d) **GET the same R2 key → gone** (404/null). DB access per memory `staging-db-access-2026-06-30` (proxy 15432) | (c) `storagePurged=0` **or** (d) object still readable → the F5 wiring is broken/missing in the image → NO-GO, back to Phase 1.4. (a)/(b) partial → worker gate must have written `failed`, not `completed` — if it wrote `completed` anyway, the REV-S9-3 gate itself is broken → NO-GO |
| 3.4 | Regression sweep: `flow-core-lifecycles.spec.ts` vs staging; staging `/health`; retention-worker log line shows sane `storagePurged` accounting | — | Lifecycle e2e green; no new worker errors over 30 min | Any lifecycle red that repros on the restore-point image too → pre-existing, note + proceed; red only on candidate → NO-GO |
| 3.5 | (Optional) restore staging to the 3.0 image to re-park the sovereign state | 🔴 OPERATOR | `flyctl releases` shows restore | — |

### Phase 4 — Prod ship (🔴 OPERATOR — red-line: GDPR/legal + prod deploy trigger)

| # | Action | Gate | VbM proof (GREEN) | RED case → response |
|---|---|---|---|---|
| 4.1 | Present Phase 1–3 evidence; obtain **explicit** operator approval to merge (per `never-bypass-human-gates`: blanket ≠ per-change) | 🔴 OPERATOR | Written approval on the PR | No approval → hold; the branch is inert |
| 4.2 | Merge PR → push to main. **This IS the prod deploy** (CI: migrate → no-op, zero new migrations → `flyctl deploy` → post-deploy e2e, prod-safe skips). If Phase 0.4 flagged it: operator re-issues `setWebhook` with `secret_token` immediately after machines are healthy | 🔴 OPERATOR | CI run green end-to-end; `flyctl status -a dowiz` 1/1 passing; **livez stays 200 throughout** (watch during rollout; boot-fix precedent: v410 deployed with zero outage) | Deploy health-check failure → `flyctl machine restart` per `docs/ops/PROD-UNBLOCK-RUNBOOK-2026-07-03.md`; livez flapping >5 min → Phase 5 rollback |
| 4.3 | **Prod webhook proof:** `curl -X POST https://dowiz.fly.dev/webhook/telegram/$SECRET -H 'Content-Type: application/json' -d '{}'` with **no** secret header → expect **401** (safe post-fix: rejected before any processing). Then a real bot interaction (owner sends `/start` or any command) → bot responds (proves Telegram's calls carry the header and pass) | 🔴 OPERATOR (holds `TELEGRAM_BOT_SECRET`) | 401 on the naked POST **and** a real update processed | 200 on naked POST → fix not live (image mismatch — check deployed SHA); real updates failing 401 → registration lacks `secret_token` → run the 0.4/4.2 `setWebhook` remedy **immediately** (notifications are down until then) |
| 4.4 | **Prod GDPR proof, tier 1 (mandatory, zero prod writes):** deployed image ties to the merge SHA (`gh run view` → `flyctl releases`); staging drill (3.3) + blob identity (1.3) transfer the proof. **Tier 2 (optional drill):** repeat the 3.3 drill on ONE shadow-demo tenant (the 12 Durrës demos are shadow/owner_id NULL) with a synthetic order + photo, then erase it | 🔴 OPERATOR (tier 2 writes to prod DB) | Tier 1: SHA chain verified. Tier 2: same (a)–(d) assertions as 3.3 on prod | Tier-2 failure → the prod R2 credentials/bucket differ from staging in a way `/health r2:ok` didn't catch → investigate `R2_*` secrets; photo-column half still holds (column nulled), object purge degraded — decide rollback vs forward-fix |
| 4.5 | Close the loop: mark ledger rows #61/74/75/76 as shipped-to-prod (date/release); write the memory entry; annotate/close GH issue #9 (stale at c8b2d5a0 — F7); update audit §4.3 status | — | Ledger + memory committed on a follow-up docs commit | — |

**Total effort estimate:** Phase 0: 20 min · Phase 1: 1–2 h · Phase 2: 30 min · Phase 3: 1–2 h ·
Phase 4: 1 h + monitoring. **One focused session (~4–6 h)** with two operator touchpoints (3.0, 4.1–4.4).

---

## 5. Risks & rollback

### Rollback plan (cheap by design)

- **Zero migrations, zero env/secret changes, zero flag flips** shipped → rollback is pure app-image.
- Primary: `git revert <commit(s)> && git push` → CI auto-redeploys (proven-green pipeline). The three
  commits are independently revertable (webhook / GDPR set / wiring).
- Emergency (CI too slow): `flyctl deploy` the previous image or `flyctl machine restart` per
  `docs/ops/PROD-UNBLOCK-RUNBOOK-2026-07-03.md`.
- Data reversibility note: erasures executed while the fix is live are **intentionally irreversible**
  (that is the feature). Rollback restores code, not erased data — no correctness issue, but state it.

### Risk register

| # | Risk | Likelihood | Mitigation |
|---|---|---|---|
| R1 | **Erasure requests start gating to `failed`** where the old code wrote false `completed` (stricter subject-graph gate surfaces real partial failures) | Medium — this is the fix working | Post-deploy: watch `ANONYMIZER_GDPR_FAILED` bus signals + `gdpr_erasure_requests.status='failed'` rows; each is a real under-erasure to remediate, not a regression |
| R2 | Worker boot regression from DI change | Low | `fresh-provision` CI boots the API; boot budget 3s + tolerated-and-reported purge semantics (never throws); staging soak 3.4 |
| R3 | Fan-out performance: a customer with many orders makes erasure slower (per-order transactions) | Low (tenant order counts are small) | Tolerated-and-reported per order; worker is async; no user-facing path blocks on it |
| R4 | **Fail-closed webhook 401s ALL Telegram updates** if prod's webhook was registered without `secret_token` while `TELEGRAM_BOT_SECRET` is set (the old fail-open branch existed precisely for this). No in-repo `setWebhook` call exists — registration is out-of-band, state unknown | **Medium — the sharpest operational risk in this ship** | Phase 0.4 preflight + 4.2 `setWebhook` remedy + 4.3 real-update proof. Blast radius if hit: owner notifications/actions pause until `setWebhook` re-issued (minutes); customer ordering unaffected |
| R5 | Staging rehearsal disturbs the parked sovereign/cutover staging state | Certain (by design) | Operator consent 3.0 + recorded restore point + optional 3.5 restore; that state is already 6 days stale (audit §9 rec 7 wants it re-baselined anyway) |
| R6 | Curated blobs drag the B6 fail-closed anonymizer semantics (throw on missing `locationId`) into main callers | Very low — **verified**: both worker call sites pass `locationId`; retention/test blobs identical across lineages | Phase 1.6 full unit suite; `fresh-provision` boot |
| R7 | Concurrent push to main between Phase 2 and 4 changes the base | Low | Phase 4.2 re-checks `origin/main` == PR base (or rebases + re-runs Phase 2); the deploy pipeline serializes on CI anyway |
| R8 | The future paleo/sovereign reconciliation conflicts with this commit | Very low | Content-identical blobs on both sides ⇒ the eventual merge sees **equal** content for these files (no conflict); the webhook file will 3-way cleanly (fix present on both sides) |

---

## 6. Operator decision points

| # | Decision | Recommendation | Where |
|---|---|---|---|
| D1 | Consent to overwrite staging (parked sovereign state) for the rehearsal, and whether to restore it after | Yes; restore optional (state already stale — consider folding into audit rec 7 re-baseline) | Phase 3.0/3.5 |
| D2 | **Approve prod merge** (= prod deploy; GDPR/legal red-line) | Yes after Phase 3 green — this is the only live legal-exposure item in the program | Phase 4.1 |
| D3 | Prod webhook `secret_token` verification / re-issue (out-of-band Telegram admin) | Verify in 0.4; if unknown, plan the `setWebhook` re-issue inside the 4.2 window | Phase 0.4 / 4.2 |
| D4 | Tier-2 prod erasure drill on a shadow-demo tenant (writes to prod DB) | Recommended once — it is the only direct prod observation of the R2 purge; use a synthetic order on a shadow demo | Phase 4.4 |
| D5 | Include gap #1 (`orders.metadata.client_ip_hash` strip) in this vehicle | **Yes** — one line + one test arm, same files, same rehearsal; closing "gap #1" while shipping "gap #2" avoids a second full prod cycle | Phase 1.5 |
| D6 | **G02 ordering:** ship G01 **before** the remote history scrub. Then regenerate the scrub from a **fresh clone of current origin** (local 07-05 twins are stale: local main = `2be9e692`, 06-25 — force-pushing it would roll prod source back 8 days and auto-deploy the regression; GitHub now has 35 branches incl. 5 post-scrub plane-maintainer branches). Scrub rewrites origin/main too (the 4 secret commits are its ancestors — verified); the rewritten tip is content-identical ⇒ the triggered CI deploy ships an identical image (harmless; still monitor). Prune `fix/audit-remediation` only **after** G01 lands (it holds the trio's provenance until then) | Adopt this ordering; treat the stale local twins as read-only archives, never push sources | G02 blueprint (separate) |

---

*Blueprint author: read-only research session 2026-07-11. Verification commands are reproduced inline;
every hash cited was resolved live against the local object store, the GitHub API, or prod HTTP.*
