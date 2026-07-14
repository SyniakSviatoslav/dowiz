# P10 — Open-Source Readiness Gap Audit

**Doc type:** HONEST gap / flag audit (NOT a fix). ADR-020.
**Generated:** 2026-07-14 (P10 WAVE)
**Repo:** `/root/dowiz` (branch `feat/kernel-fsm-graph-analysis`)
**Author:** delegated audit subagent (read-only; operator-gated items untouched)

> This document FLAGS gaps. It changes no LICENSE, performs no force-push, scrubs no
> history, and touches no credential/secret file. Operator-gated HARD BLOCKERS are
> reported only — never acted on.

---

## 1. LICENSE status — GAP CONFIRMED

**Actual current LICENSE (verbatim, first 4 lines of `/root/dowiz/LICENSE`):**

```
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/
```

The file is the full 201-line Apache License 2.0 (confirmed: line 5 onward reads
"TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION"). **License in tree = Apache-2.0.**

**Mandated by roadmap P10 (verbatim, `docs/design/MASTER-ROADMAP-10-PHASES-2026-07-14.md` lines 84–87):**

```
### P10 — Open-source readiness (ADR-020, gated) — OPEN
AGPLv3 + TM + DCO. Gated on secrets scrub + EUTM. SECURITY INCIDENT (creds in git
history) rotated; REMOTE scrub force-push = open gate → HARD blocker for prod push.
~20 missing 2026-07-11 reports: UNVERIFIED manifest filed honest (not fabricated).
```

**Explicit gap:** The repo ships **Apache-2.0** but the roadmap mandates **AGPLv3**. Apache-2.0
is a permissive license; AGPLv3 is a strong copyleft (network-use clause). This is a REAL,
material mismatch and a release blocker.

**What an AGPLv3 + TM + DCO transition requires (flagged, NOT performed):**
1. Replace the LICENSE file with the **GPL Affero General Public License v3.0** text, and
   update any SPDX headers / `package.json`/`Cargo.toml` license fields accordingly.
2. Add a **DCO sign-off mechanism**: `Signed-off-by:` trailer required on every commit
   (see §3 — currently absent as a policy) **and** a `CONTRIBUTING.md` DCO clause
   (Developer Certificate of Origin 1.1 text + instruction to run `git commit -s`).
3. Add a **trademark / NOTICE policy file** (see §4 — currently absent).

**Action taken on LICENSE:** NONE. Flagged only. (HARD rule: do not modify LICENSE.)

---

## 2. Secrets in git history — CONFIRMED PRESENT, NOT SCRUBBED

**Evidence commands run (read-only):**

```bash
git log --all --oneline | head -40          # history inspected
git log -p --all -S 'password|api_key|secret|token' -- '.env*' 2>/dev/null | head
git log --all --oneline -- '.env*'          # commits touching any .env* path
git log --all --diff-filter=A --name-only --pretty=format: -- '.env*'  # files ever added
git log -p --all -- '.env*' | grep -E '^[+-]' | sed -E 's/(=|:).*/\1 <VALUE-REDACTED>/' \
  | grep -iE 'key|secret|token|password|...' | sort -u
```

**Findings (KEY NAMES only — all VALUES REDACTED, never printed):**

Credential-shaped keys are discoverable in history (committed in a `.env*`-class file):

```
***REDACTED***= <VALUE-REDACTED>
***REDACTED***= <VALUE-REDACTED>
***REDACTED***=    <VALUE-REDACTED>
***REDACTED***=        <VALUE-REDACTED>
***REDACTED***=    <VALUE-REDACTED>
***REDACTED***=         <VALUE-REDACTED>
***REDACTED***=          <VALUE-REDACTED>
***REDACTED***=      <VALUE-REDACTED>
***REDACTED***=     <VALUE-REDACTED>
***REDACTED***=      <VALUE-REDACTED>
***REDACTED***=     <VALUE-REDACTED>
***REDACTED***=       <VALUE-REDACTED>
```

The only `.env*` path ever added to the tree is `.env.example` (a template), but the keys
above (and their historical values) are recoverable from history. History also contains a
commit `a7d198db "chore(sovereign-core): clean-history snapshot of local tree (secrets
dropped)"` — that is a **local working-tree snapshot**, not a remote history scrub.

Commits that touched `.env*`-class paths (subjects + SHAs, no values):

```
1ac7339b wip(isolate): recover/harden_26_CRIT_HIGH_te (pre-agent WIP, NOT reviewed)
a7d198db chore(sovereign-core): clean-history snapshot of local tree (secrets dropped)
0fcfe23b WIP on feat/mvp-sensor-seams: ...
aa0c3fbd fix: finish invariant follow-ons + repo hygiene
dac2aa00 fix: finish invariant follow-ons + repo hygiene
a4760c68 feat(notifications): add WhatsApp channel via @whiskeysockets/baileys
349fa7df feat(notifications): add WhatsApp channel via @whiskeysockets/baileys
77dfa80e audit: full-spectrum 2026-06-14 — 31 fixes, 48 files
c9b54f87 audit: full-spectrum 2026-06-14 — 31 fixes, 48 files
76cb64ec feat: complete structural sweep — audit fixes, metric-core 1.0, ...
d27ce9ae feat: complete structural sweep — audit fixes, metric-core 1.0, ...
00ce2306 fix: health check 503 - make Telegram check non-fatal, remove @ts-nocheck
b0255ed5 fix: health check 503 - make Telegram check non-fatal, remove @ts-nocheck
84b95d66 UI/UX polish: ...
991a486d UI/UX polish: ...
```

**Conclusion:** Credential-shaped strings (key names + historically committed values) remain
recoverable in git history. This matches the documented past SECURITY INCIDENT (creds in git
history), which were ROTATED but whose history was NOT scrubbed.

> **History NOT scrubbed; scrub = force-push = operator-gated HARD blocker.
> DO NOT scrub without explicit operator authorization.**

**Action taken:** NONE. No history rewrite, no BFG/git-filter-repo, no force-push. (HARD rule.)

---

## 3. DCO / CI sign-off — ABSENT

**Evidence command:**

```bash
git log --all --grep='Signed-off-by' --format='%h %an %s'
```

**Result — exactly 1 commit carries a `Signed-off-by` trailer:**

```
6d871e96 dependabot[bot] chore(deps-dev): bump esbuild from 0.28.0 to 0.28.1
```

That single commit is a **dependabot[bot] auto-sign-off** on a dependency bump — not a
project-wide DCO practice.

**DCO artifacts:**
- Repo-root `CONTRIBUTING.md` — **ABSENT** (the only `CONTRIBUTING.md` in the tree is
  `tools/skillspector/CONTRIBUTING.md`, a subcomponent, not a project DCO).
- Repo-root `DCO` file — **ABSENT**.
- No CI enforcement of `Signed-off-by` is in place (no DCO GitHub Action / required-trailer
  check configured as project policy).

**Conclusion:** DCO is **not established** as a project policy. Only a bot's automated bump
is signed off.

**Action taken:** NONE. Flagged only. (Transition item, not operator-gated, but out of scope
of this audit-as-fix.)

---

## 4. Trademark (TM) / EUTM — ARTIFACTS MISSING

**Evidence command:**

```bash
find . -maxdepth 3 \( -iname 'NOTICE*' -o -iname 'TRADEMARK*' -o -iname 'CONTRIBUTING*' \
  -o -iname 'DCO*' -o -iname 'CODE_OF_CONDUCT*' \) -not -path '*/node_modules/*' \
  -not -path '*/.git/*'
```

**Result:** only `./tools/skillspector/CONTRIBUTING.md` (subcomponent). **No project NOTICE
file, no trademark policy, no CODE_OF_CONDUCT.**

**Missing trademark artifacts (flagged, NOT created):**
1. A top-level **`NOTICE`** file stating the project name, owner, and that the name/logo are
   trademarks (trademark notice for AGPLv3 + TM combination).
2. A **trademark policy** document (usage permissions, what is/isn't allowed, contact).
3. **EUTM (EU Trademark registration)** — operator-provisioned legal asset. NOT the
   agent's job; flagged as operator action only.

**Action taken:** NONE. (EUTM is explicitly operator-provisioned — not performed.)

---

## 5. "~20 missing 2026-07-11 reports" — HONEST UNVERIFIED MANIFEST

**Confirmed: an honest UNVERIFIED manifest entry exists; no report fabricated by this audit.**

**Manifest line (roadmap, `MASTER-ROADMAP-10-PHASES-2026-07-14.md` line 87):**
> `~20 missing 2026-07-11 reports: UNVERIFIED manifest filed honest (not fabricated).`

**Corroborating honest entry (red-team, `docs/red-team/2026-07-13/D6-business-value.md` line 267):**
> `| 8 | ~20 foundation research reports (2026-07-11 brief) | NEVER EXISTED ON DISK | ROADMAP-GROUND-TRUTH §0.1, "headline risk" |`

Both entries state the reports are **missing / never existed on disk / UNVERIFIED** — i.e. the
gap is self-declared honestly rather than back-filled with fabricated content. This audit did
**NOT** create, invent, or back-date any of the ~20 reports.

---

## 6. Consolidated GO / NO-GO table

| # | Gate | Status | Evidence | Single owner-action required |
|---|------|--------|----------|------------------------------|
| 1 | **AGPLv3 LICENSE** | ❌ BLOCKER (license mismatch) | Tree ships Apache-2.0; P10 mandates AGPLv3 | Operator + maintainer: replace LICENSE with AGPLv3 text + update SPDX headers |
| 2 | **DCO** | ❌ OPEN / ABSENT | Only 1 bot commit signed-off; no root CONTRIBUTING/DCO | Maintainer: add `CONTRIBUTING.md` DCO clause + require `git commit -s` + CI trailer check |
| 3 | **TM / NOTICE** | ❌ OPEN / ABSENT | No NOTICE file, no trademark policy in tree | Maintainer: add `NOTICE` + trademark policy doc |
| 4 | **Secrets-scrub** | 🔴 HARD BLOCKER (operator-gated) | Credential keys + values recoverable in history; not scrubbed | **OPERATOR ONLY:** authorize remote history scrub (force-push). Agent must NOT do this |
| 5 | **EUTM** | 🔴 HARD BLOCKER (operator-gated) | Legal trademark registration not provisioned | **OPERATOR ONLY:** provision EU trademark; not agent scope |
| 6 | **~20 reports manifest** | ✅ HONEST / UNVERIFIED | Roadmap L87 + D6 L267 declare missing, not fabricated | No action — gap honestly recorded; do not fabricate |

**Release verdict: NO-GO for OSS prod push** while gates 1–5 remain open. Gates 4 and 5 are
HARD operator-gated blockers; gates 1–3 are maintainer/operator transition work (not agent
self-service in this audit).

---

## VERIFY — working-tree integrity

- `git status --short` was inspected. **This audit created exactly ONE new untracked file:**
  `docs/design/P10-OSS-READINESS-AUDIT.md`.
- Pre-existing working-tree modifications (present BEFORE this audit began; NOT created by
  this audit) were observed on: `.gitignore`, `docs/operating-model/proposed-hooks/post-edit-gates.sh`,
  `kernel/pkg/dowiz_kernel.d.ts`, `kernel/pkg/dowiz_kernel.js`, `kernel/pkg/dowiz_kernel_bg.wasm`,
  `kernel/pkg/dowiz_kernel_bg.wasm.d.ts`. These are unrelated build/hook artifacts and were
  **not touched** by this audit.
- **LICENSE was NOT modified.** Git history was NOT rewritten. No force-push performed. No
  secret/credential file read for value extraction (key names only, values REDACTED).
- **No commit or push was performed.**

---

## OPERATOR-GATED HARD BLOCKERS NOT TOUCHED

✅ Confirmed: the following were **NOT** acted on (per HARD rules) — reported only:
- **LICENSE** — not changed (Apache-2.0 left in place; AGPLv3 transition flagged).
- **Git history scrub** — not performed; credential-shaped strings remain recoverable.
- **Force-push** — not performed; no remote mutation attempted.
- **EUTM** — not provisioned; flagged as operator legal action.
- **No credential/secret file** was opened for live-value extraction; all secret values are
  REDACTED in this report.
