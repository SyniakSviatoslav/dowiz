# Pre-Open-Source Secrets Audit

**Date:** 2026-07-02
**Scope:** working tree (all tracked files) **and** full git history (`--all`).
**Method:** manual pattern audit (`gitleaks` not installed on this host —
`which gitleaks` → not found). Patterns scanned via `git grep` (tree) and
`git log -p --all -S` / `git log --all -p | grep` (history).
**Verdict:** 🔴 **NOT CLEAN — real production database credentials are present
in both the working tree and git history. Remediation is a hard gate before the
repository is made public.**

---

## 1. Finding — CRITICAL: live Supabase production DB credentials committed

Real credentials for the production Supabase project **`elxukhxvuycnftqwaghg`**
(host `aws-1-eu-central-1.pooler.supabase.com`) are hard-coded in tracked
throwaway scripts. Two distinct secrets are exposed:

| Secret | Value (redacted form) | Role |
| ------ | --------------------- | ---- |
| App user password | `REDACTED_DB_PASSWORD` | `deliveryos_api_user` |
| DB superuser password | `7V#KxApMx8Z5B5.` (URL-encoded `REDACTED_DB_PASSWORD`) | `postgres` (superuser) |

These are **not** placeholders — they reference a real project ref, a real
regional pooler host, and real role names, and appear in scripts that actually
open connections.

### 1a. Locations in the working tree (12 tracked files)

```
.agents/tmp/check-jobs.mjs
.agents/tmp/check-session.mjs
.agents/tmp/test-connections.cjs
apps/api/fix-db.js
packages/db/scripts/check-job-details.ts
packages/db/scripts/check-job-schema.ts
packages/db/scripts/check-notify-jobs2.ts
packages/db/scripts/check-schemas.ts
packages/db/scripts/test-connections.js
packages/platform/test-notify2.cjs
packages/platform/test-notify3.cjs
docs/design/telegram-notifications-actions/staging-probe-bypassrls.md   (doc — contains the ref)
```

All 11 code files are ad-hoc debug/throwaway scripts (`check-*`, `test-*`,
`fix-db.js`) that should never have been committed.

### 1b. Locations in git history (4 commits)

The credentials were introduced/carried by these commits (found via
`git log --all -S'elxukhxvuycnftqwaghg'` and `-S'REDACTED_DB_PASSWORD'`
/ `-S'7V%23KxApMx8Z5B5'`):

```
72fde8a6 feat(notifications): council-hardened Telegram categories + storefront action (dark)
bccfd324 fix: Telegram callback auth + CONFIRMED/REJECTED notifications
d5eef9cb fix(pg-boss): use operational pool instead of session pool for pg-boss connection
84b95d66 UI/UX polish: language switcher SQ/EN/UA ... (16/16 Playwright green)
```

**Deleting the files from the tree is NOT sufficient** — the secrets remain
retrievable from history.

### Remediation (required before public — do NOT rewrite history unilaterally)

1. **ROTATE FIRST (most important).** Treat both passwords as compromised:
   - Reset the `postgres` superuser password and the `deliveryos_api_user`
     password in the Supabase dashboard.
   - Rotate anything derived from them; update Fly secrets / env for prod and
     staging. Rotation makes the leaked values worthless even if history is
     never rewritten.
2. **Remove the files from the tree** (they are debug scripts with no product
   value): delete the 11 code files above and scrub the ref from the doc, or
   move them out of version control and add ignore rules.
3. **Rewrite history** to purge the secret from all 4 commits using
   **`git filter-repo`** (preferred) or **BFG Repo-Cleaner**, e.g.:
   ```bash
   # example — run only after backing up and coordinating with all clones
   git filter-repo --replace-text <(printf '%s\n' \
     'REDACTED_DB_PASSWORD==>REDACTED' \
     'REDACTED_DB_PASSWORD==>REDACTED' \
     'elxukhxvuycnftqwaghg==>REDACTED')
   ```
   This is destructive and rewrites commit hashes — it must be done by a human
   who coordinates with every clone/fork and force-pushes once. **This audit
   does not rewrite history; it reports only.**
4. **Add a pre-commit / CI secrets gate** (gitleaks or the repo's
   `verify:secrets`) so this cannot recur.

---

## 2. Non-findings (verified safe — do NOT rotate)

The following matched secret-shaped regexes but are **not** real secrets:

- **Placeholder connection strings** in `.env.example`
  (`postgresql://your-user:your-pass@your-host:...`) — templates only.
- **Local/CI ephemeral DBs** in `apps/api/tests/**`, `packages/**/tests/**`,
  `apps/api/tests/_env-stub.ts`, and `.github/workflows/{ci,visual}.yml`
  (`postgres://u:p@localhost`, `postgres:postgres@127.0.0.1`,
  `dowiz_app:app_pw@127.0.0.1`) — throwaway credentials for local/CI Postgres,
  no external reachability.
- **Test fixtures for the secret scanner itself** in
  `scripts/plane-telemetry.test.mjs` and the CCC secret-scan test — deliberately
  fake tokens used to prove the scanner works:
  - telegram `123456789:AAErtyuiodfghjkcvbnmqwertyuiopasdf1`
  - AWS `AKIAIOSFODNN7EXAMPLE` (AWS's own public documentation example key)
  - `ghp_abcdef…`, `sk-abcdef…`, `xoxb-1234567890-…`, `hunter2`
- **`.env` files** are correctly `.gitignore`d (`.env`, `.env.*`) and no real
  `.env` is tracked.

## 3. Telegram bot token check (explicitly requested)

Searched tree and history for the prod Telegram bot token pattern
`\b\d{6,}:[A-Za-z0-9_-]{30,}\b` (`git log -p --all -S':AA' -- ':!*.md'` and a
history-wide grep). **No real Telegram bot token found** — the only matches are
the fake `123456789:AAErty…` test fixture noted above. ✅

## 4. Commands run (reproducible)

```bash
which gitleaks                                   # -> not installed
# tree scans:
git grep -nIE '\b[0-9]{6,}:[A-Za-z0-9_-]{30,}\b' -- ':!*.md'
git grep -nIE '(sk-[A-Za-z0-9]{20,}|xox[baprs]-...|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----|ghp_[A-Za-z0-9]{30,}|postgres(ql)?://[^ ]*:[^@]*@)' \
  -- ':!*.md' ':!pnpm-lock.yaml'
git grep -lI 'elxukhxvuycnftqwaghg'
# history scans:
git log --all -p -S':AA' -- ':!*.md' | grep -nE '\b[0-9]{6,}:AA[A-Za-z0-9_-]{30,}\b'
git log --all --oneline -S'REDACTED_DB_PASSWORD'
git log --all --oneline -S'7V%23KxApMx8Z5B5'
git log --all --oneline -S'elxukhxvuycnftqwaghg'
git log --all -p | grep -aoE '(sk-...|xox...|ghp_...|AKIA...|SG\....)' | sort -u
```

> If `gitleaks` becomes available, also run
> `gitleaks detect --no-banner --redact` (tree) and
> `gitleaks detect --no-banner --redact --log-opts="--all"` (history) as a
> second, independent pass before flipping the repo public.

---

## Summary

- 🔴 **1 CRITICAL finding:** live Supabase prod credentials (2 passwords) in
  12 tracked files **and** 4 history commits.
- ✅ No Telegram bot token, API key, private key, or cloud key leaked (all such
  matches are documented test fixtures/placeholders).
- **Gate before public:** rotate the credentials → remove the files → rewrite
  history (filter-repo/BFG, by a human) → add a secrets CI gate.
