# D5 — Reliability / Ops / Supply-Chain Red-Team

**Target:** `/root/dowiz` · branch `feat/decentralized-pq-protocol` · date 2026-07-13
**Method:** read-only. gitleaks (v-installed, `/usr/bin/gitleaks`), `git fsck`/`rev-list`, `pnpm audit --prod`, config/script review. CONFIRMED = observed here; PLAUSIBLE = inferred, not executed.

---

## 1. Bottom line — biggest operational / supply-chain risk

Two heads, both about **prod blast radius on `main`**:

1. **Auto-deploy pipeline is a single unguarded lever.** A push to `main` runs `pnpm migrate:up` **directly against the prod DB** and then `flyctl deploy` — with **no approval gate, no automated rollback, no concurrency guard**, and the deploy step imports an **unpinned third-party action pinned to a moving branch** (`superfly/flyctl-actions/setup-flyctl@master`) that holds `FLY_API_TOKEN`. One bad migration or one compromised upstream action = prod outage / prod takeover at 3am.
2. **Secrets hygiene: reachable history is clean and live keys were rotated, but the incident is not closed.** 10 unreachable/orphaned git blobs still carry (rotated) `JWT_PRIVATE_KEY` / `COURIER_PII_ENCRYPTION_KEY` / RSA private keys. The memory-tracked remote scrub/force-push is an **OPEN gate**, so those objects almost certainly still live on GitHub, retrievable by SHA even though `git log` cannot see them. The secrets gate (`gitleaks detect`) scans reachable refs only — it structurally cannot see them and reports "clean."

---

## 2. Secrets-in-git verdict (DEFINITIVE)

**No LIVE, currently-valid secret is present in the reachable git history.** But the historical creds incident is REAL and not fully remediated (rotated, not scrubbed).

Evidence:

- **`.env`, `.env.test`, `.secrets.local` were NEVER committed on any reachable ref.**
  `git log --all --oneline -- .env / .env.test / .secrets.local` → empty. `git ls-files` → only `.env.example` tracked. `.env` is gitignored (`.gitignore:6-7`). verify-secrets check #4 (`git log --all --diff-filter=A -- "*.env" "*.pem" "*.key"`) passes.
- **The one credential-shaped string in reachable history is a UI-mockup placeholder.**
  `src/screens/14-admin-branding.html:310` (initial commit `76856da9` + polish commits `84b95d66`, `991a486d`) holds `value="sk_live_dS8f…"` inside a **disabled `<input>`**. Length = **18 chars total** (~10 after `sk_live_`); a real Stripe secret key is ~107 chars. **Definitively fabricated placeholder**, not a functional credential. It is allowlisted in `.gitleaks.toml` (`src/screens/`) and `.gitleaksignore:3`.
- **The gitleaks run reports 49 "leaks" — all noise, none committed.** All 49 are in **untracked, gitignored** local files: `.venv-paddle/**` (Python lib test fixtures — PyCrypto/PIL test keys), `.env.test`, `.secrets.local`. gitleaks scans the working tree; none are in history.
- **The incident residue: 10 orphaned/unreachable blobs still carry rotated key material.**
  `git fsck --no-reflogs --unreachable` → 1875 unreachable blobs; **10** contain `BEGIN RSA PRIVATE KEY` / `COURIER_PII_ENCRYPTION_KEY=` / `JWT_PRIVATE_KEY=` (e.g. blob `4505d018…`, `git rev-list --all --objects | grep` count = **0** → unreachable from every ref, from a scrubbed `export …` secrets file).
  **Are they still live?** Hash-compared the orphaned `COURIER_PII_ENCRYPTION_KEY`, `TELEGRAM_BOT_TOKEN`, `GOOGLE_CLIENT_SECRET` against the current live `.env`: **0 / 0 / 0 matches → rotated.** So the exposed material is STALE.

**Residual risk (why this is not "closed"):**
(a) Only 3 secret classes were sampled — a **full rotation audit of every secret in the 10 blobs** (DB passwords, JWT_SIGNING_SECRET, OpenRouter key, VAPID) is required to prove none is still valid.
(b) The remote scrub/force-push is an OPEN gate → the same unreachable objects are almost certainly still fetchable via the GitHub API by SHA (GitHub retains unreachable commits ~indefinitely without a support-driven GC).
(c) `open-source per ADR-020` is a stated goal — publishing a repo whose object store contains recoverable private keys is a hard blocker.

**Verdict:** History is clean of *live* secrets; the incident's *rotated* key material persists as unreachable objects locally and (PLAUSIBLE) on the remote. **Not a live-credential leak, but an unresolved hygiene/scrub liability.**

---

## 3. Findings

### F1 — Orphaned git objects retain rotated private keys; remote scrub is an OPEN gate
- **Severity:** HIGH
- **Location:** local `.git` object store (blob `4505d018…` + 9 others); GitHub remote (unverified from here)
- **Evidence:** `git fsck --unreachable` → 10 blobs carrying `JWT_PRIVATE_KEY=` / `COURIER_PII_ENCRYPTION_KEY=` / `-----BEGIN RSA PRIVATE KEY-----`; reachable-from-refs = 0; live-`.env` hash match = 0/0/0 (rotated).
- **Failure/exploit:** Rival clones the repo (or hits the GitHub commit/blob API by SHA) and harvests key material + secret **names/formats**; if any un-sampled secret was not actually rotated, it is directly usable. Invisible to `git log` and to the refs-scanning gate → false sense of "clean."
- **Fix:** Complete BFG/`filter-repo` scrub + force-push all refs; request GitHub support GC of unreachable objects; run a full rotation audit across all 10 blobs; add a dangling-object scan to the secrets gate.

### F2 — Prod auto-deploy + prod DB migration on push-to-`main`, no approval / rollback / concurrency
- **Severity:** HIGH
- **Location:** `.github/workflows/ci.yml:127-153` (`deploy` job, `if: github.ref == 'refs/heads/main'` → `pnpm migrate:up` with `secrets.DATABASE_URL_MIGRATIONS`, then `flyctl deploy --remote-only`). Fly also re-runs migrations via `attic/fly.toml` `release_command = "dist/migrate/index.cjs"`.
- **Evidence:** cited lines; no GitHub `environment:`/protection rule, no `concurrency:` group, no backup-before-migrate step.
- **Failure/exploit:** A forward-only node-pg-migrate migration is applied to prod **before** the app deploys; a bad/destructive migration corrupts prod schema with no gated approval and no automated `down`. Two rapid merges → overlapping migration/deploy race. 3am pager with manual recovery only.
- **Fix:** GitHub Environments approval gate on `deploy`; expand-contract migrations + dry-run; `concurrency` group; automated pre-migrate backup snapshot.

### F3 — Unpinned third-party deploy action holds the prod Fly token
- **Severity:** HIGH
- **Location:** `.github/workflows/ci.yml:150` `uses: superfly/flyctl-actions/setup-flyctl@master` (job env `FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN }}`)
- **Evidence:** `@master` = mutable branch, not a SHA. (All other actions are at least major-tag-pinned: `actions/*@v4/v5`, `pnpm/action-setup@v3`, `github/codeql-action@v3`.)
- **Failure/exploit:** Upstream account takeover or a malicious commit to `master` runs in the prod-deploy job and exfiltrates `FLY_API_TOKEN` → full Fly-org control (deploy arbitrary image to `dowiz.fly.dev`).
- **Fix:** Pin to a full commit SHA; scope the Fly deploy token to the single app; consider OIDC over a long-lived token.

### F4 — gitleaks allowlist over-permissive + gate comment is factually wrong
- **Severity:** MEDIUM
- **Location:** `.gitleaks.toml:32-55`; `scripts/verify-secrets.ts:26-31`
- **Evidence:** Allowlist blanket-excludes whole dirs: `scripts/`, `tools/`, `docs/`, `.github/workflows/`, `e2e/`, `apps/api/tests/`, `loops/runs/`, `rebuild/`, `src/screens/`. verify-secrets comment claims "git mode (default: respects `.gitignore`, so local `.env`/`.venv`/`dist` are NOT scanned)" — but `gitleaks detect -c .gitleaks.toml` returns **49 leaks / exit 1** over gitignored `.venv-paddle/**`, `.env.test`, `.secrets.local`. The claim is false, and `.venv-paddle/` is not even covered by the allowlist (regex is `\.venv/`, not `.venv-paddle/`).
- **Failure/exploit:** (a) A real secret committed into any allowlisted dir (e.g. a live token in a `scripts/deploy-*.ts`) is invisible to the gate. (b) Locally the gate is red on noise → developers run `git commit --no-verify` (memory records prior `--no-verify` commits) → the gate is effectively bypassed in day-to-day work.
- **Fix:** Allowlist specific files, not whole trees; add `.venv-paddle`/scan only tracked files (`--no-git`) or scan history explicitly; correct the misleading comment.

### F5 — Secrets gate's default-secret scan no longer covers the server code
- **Severity:** MEDIUM
- **Location:** `scripts/verify-secrets.ts:67` `findFiles(path.join(ROOT, 'apps'), …)`
- **Evidence:** On `feat/decentralized-pq-protocol` the API moved to `attic/apps-api/` (`apps/` now contains only `web`). Check #3 (hardcoded `JWT_PRIVATE_KEY`/`TELEGRAM_BOT_TOKEN`/`VAPID_PRIVATE_KEY` defaults) scans only `apps/` → the server source is unscanned.
- **Failure/exploit:** A `process.env.JWT_PRIVATE_KEY || '<hardcoded default>'` in the server ships without the gate noticing.
- **Fix:** Point the scan at the actual server source root / the tree the deploy artifact is built from.

### F6 — Container runs as root; unpinned base; runtime-stage unpinned `npm install` with install scripts
- **Severity:** MEDIUM
- **Location:** `Dockerfile:30-45`
- **Evidence:** No `USER` directive → runs as **root**. `FROM node:22-slim` (mutable tag, no digest) in both stages. Line 42: `RUN npm install argon2 sharp @aws-sdk/client-s3 @aws-sdk/lib-storage` in the **runtime** stage — no lockfile, no version pin, no `--ignore-scripts` → resolves "latest" at build time and executes their install scripts as root.
- **Failure/exploit:** A compromised/yanked `argon2`/`sharp`/`@aws-sdk` release (or a transient registry/typosquat window) runs a postinstall as root in the prod image; any later RCE/container-escape also runs as root.
- **Fix:** Pin base by digest; move these deps into a pinned `package.json` + lockfile; `--ignore-scripts` where feasible; add `USER node`.

### F7 — Dockerfile drifted from the branch tree → un-buildable / deploy config attic'd
- **Severity:** MEDIUM
- **Location:** `Dockerfile:38` `COPY --from=builder /app/apps/api/public …`; `apps/api` absent on this branch
- **Evidence:** `ls apps/api` → "No such file or directory" (moved to `attic/apps-api`). `fly.toml`, the boot-guard, and the reliability/`healthz` ratchet are all in `attic/` on this branch (`find` returns no live `reliability.rs`/boot-guard/server crate; only `kernel/Cargo.toml`).
- **Failure/exploit:** A build/deploy from this branch fails at the `COPY`. More importantly the branch has **no deployable server + no live boot-guard/health/storm-latch** — a latent release-integrity trap if it is merged/deployed without reconciliation. (Prod ships from `main`, which still has these — so this is branch-scoped drift, not a live prod outage.)
- **Fix:** Reconcile Dockerfile/fly with the branch's real layout before it can ship; keep the boot-guard/health code out of `attic` for the deployable artifact.

### F8 — Live local secret files are world-readable / world-writable
- **Severity:** MEDIUM
- **Location:** `/root/dowiz/.env` mode `-rw-rw-rw-` (0666); `.env.test`, `.secrets.local` 0644
- **Evidence:** `ls -la`. `.env` holds live `JWT_PRIVATE_KEY`, `TELEGRAM_BOT_TOKEN`, `GOOGLE_CLIENT_SECRET`, `COURIER_PII_ENCRYPTION_KEY`, `OPENROUTER_API_KEY`, `IP_HASH_SALT`.
- **Failure/exploit:** Any other local user or a compromised non-root process reads the live secrets; `.env` is world-**writable** → an attacker can also rewrite `APP_BASE_URL`/`JWT_*` to hijack signing/redirects.
- **Fix:** `chmod 600` on all secret files; move to a secrets manager / Fly secrets only.

### F9 — Service-worker push-resubscribe endpoint mismatch → silent courier push loss
- **Severity:** LOW
- **Location:** `public/sw.js:103` POSTs `/api/push/resubscribe`; server route is `/api/courier/push/resubscribe` (per memory/routes); three divergent SW copies exist (`public/`, `web/public/`, `apps/web/public/`).
- **Evidence:** path divergence across copies.
- **Failure/exploit:** When a browser rotates the push subscription, `pushsubscriptionchange` POSTs to a 404 → the courier silently stops receiving dispatch pushes; no log/alert surfaces it.
- **Fix:** Unify the endpoint across all SW copies; alert on resubscribe 4xx.

### F10 — SW `notificationclick` navigates to unvalidated server-supplied `data.url`
- **Severity:** LOW
- **Location:** `public/sw.js:74,88` (`targetUrl = d.url` → `c.navigate(targetUrl)`)
- **Evidence:** no origin/allowlist check on `data.url`.
- **Failure/exploit:** Whoever can send a web-push (i.e. holds the VAPID private key) can redirect a courier's focused client to an arbitrary URL. Gated by VAPID-key secrecy → low.
- **Fix:** Restrict navigation to same-origin/relative paths.

---

## 4. Positive controls (acknowledged)

- All three workflows trigger on `pull_request` (**not** `pull_request_target`) → deploy secrets are not exposed to forked PRs; `deploy` is gated `if: github.ref == 'refs/heads/main'`.
- `fresh-provision` CI job: brand-new Postgres → migrate → seed → boot → `/health` 200 smoke (real from-scratch boot proof).
- `pnpm install --frozen-lockfile` everywhere; `pnpm audit --prod --audit-level=high` → **0 known vulnerabilities**; **no first-party pre/postinstall scripts**.
- Service worker uses **no Cache API / no `fetch` handler** → the classic SW cache-poisoning / stale-authenticated-content vector is **absent**.
- `.dockerignore` excludes `**/.env`; multi-stage build keeps dev deps out of the runtime image.
- Backup tooling present: `scripts/backup-drill.ts`, `backup-restore.ts`, `backup-verify.ts`, `backup-list.ts`.

---

## 5. Infra / CI hardening list (prioritised)

1. **Close the secrets incident:** BFG/`filter-repo` scrub + force-push + GitHub GC request; full rotation audit of every secret in the 10 orphaned blobs; block the open-source publish until proven clean. *(F1)*
2. **Gate the prod deploy:** GitHub Environment with required reviewers on `deploy`; expand-contract + dry-run migrations; pre-migrate backup; `concurrency` group. *(F2)*
3. **Pin every third-party action to a full SHA** (start with `superfly/flyctl-actions@master`); scope `FLY_API_TOKEN` to one app / move to OIDC. *(F3)*
4. **Fix the secrets gate:** narrow the allowlist to files not dirs; scan tracked files + history (not the noisy working tree); add a dangling-object scan; correct the false "respects `.gitignore`" comment; point the default-secret scan at the real server source. *(F4, F5)*
5. **Harden the image:** `USER node`; digest-pin the base; pin + lockfile the runtime `npm install`, `--ignore-scripts`; reconcile the Dockerfile/fly with the branch layout. *(F6, F7)*
6. **`chmod 600` all local secret files**; prefer Fly secrets / a manager over on-disk `.env`. *(F8)*
7. **Unify + monitor push:** single resubscribe endpoint, alert on 4xx; same-origin allowlist for SW navigation. *(F9, F10)*
</content>
</invoke>
