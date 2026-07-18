# GitHub Maintenance & Hygiene Audit — dowiz

> Read-only audit of GitHub state vs. real repo, for repo-hygiene research only. No code changes.
> Worktree: `/root/dowiz-verify-redteam` · branch `research/dowiz-verify-redteam-2026-07-17` · date 2026-07-17.
> Method: live `gh`/`git` read-only calls against `origin = git@github.com:SyniakSviatoslav/dowiz.git`,
> cross-checked against the working tree. `jq` is not installed on this host; counts use `git`/`grep`/`wc`.
> Every classification is a checkable finding, not an assumption.

---

## 0. Headline

The repo **exists and is reachable over SSH** (working `origin`), but the **GitHub REST/GraphQL API
cannot see it** through the authenticated `gh` token — `dowiz` is a **private repository the current
fine-grained PAT is not scoped to** (see §1). Consequently the discoverability metadata the task
asked for (description, topics, license-as-declared-on-GitHub, homepage, Releases) **cannot be read
via the API**; where it matters the on-disk truth is substituted and labelled as such.

Two gaps dominate everything else:
1. **Zero tags and zero releases on the remote.** The 5 local tags are all defensive backup markers
   and **none were ever pushed** — GitHub sees no version history at all.
2. **61 remote branches, ~40% of them scratch/bot/backup/snapshot** that were never meant to be
   permanent.

---

## 1. Real GitHub state (stated plainly, not guessed)

| Probe | Result |
|---|---|
| `gh repo view SyniakSviatoslav/dowiz --json …` | **FAILS** — `GraphQL: Could not resolve to a Repository with the name 'SyniakSviatoslav/dowiz'`. |
| `gh api repos/SyniakSviatoslav/dowiz` | **404 Not Found**. |
| `gh repo list SyniakSviatoslav --limit 200` | `dowiz` **absent**; the token lists **only public repos** (OpenBebop, bebop, and old portfolio projects). |
| `gh api user/repos?visibility=private` | `[]` — token surfaces **no** private repos. |
| `gh api -i user` → `X-OAuth-Scopes` | **empty** (classic scopes blank ⇒ a **fine-grained** `github_pat_…`, authenticated as `SyniakSviatoslav`). |
| `git ls-remote origin` (SSH) | **WORKS** — 112 refs, HEAD `4956faca…`. |
| Remote `HEAD` symref | `refs/heads/main` — **default branch = `main`** (confirmed). |
| Remote heads / tags | **61 branches / 0 tags** (`git ls-remote --heads/--tags`). |
| Releases | **Not directly queryable** (API 404); 0 tags ⇒ effectively **no GitHub Releases** (Releases attach to tags). |

**Interpretation.** SSH key access and API-token access are decoupled on this host: the machine's SSH
key can push/pull the private repo, but the `gh` PAT is scoped to a resource set that excludes it.
This is why the earlier "43 local branches" fact and the API are both partly blind — the **remote is
the larger, messier surface: 61 branches, not 43.** For any future GitHub-side metadata work
(topics, description, Releases API), the operator must either broaden the fine-grained PAT's repo
scope to include `dowiz`, or run those `gh` calls under a token that can see it. **Not fixable from
inside this audit** — flagged, not worked around.

**Discoverability metadata (from disk, since the API is blind):** the repo declares
`AGPL-3.0-or-later` (33 KB `LICENSE` present) with `NOTICE` + `TRADEMARK.md`; there is **no** way to
confirm GitHub's *repository topics* are set, and given 0 releases and a private scope, they are
almost certainly **unset**. Recommended topics once the API is reachable: see §6.

---

## 2. Versioning gap + recommended scheme

### The gap
- **Remote: 0 tags, 0 releases.** Local: 5 tags, all defensive (`H8-PRE-SCRUB`,
  `backup/main-pre-merge-20260714`, `recovery/*-pre-reconcile ×2`, `recovery/pre-decart-rebase-local`)
  — backup markers, never pushed, never semver. There is **no released-version history whatsoever**.
- No `CHANGELOG.md`, `HISTORY`, `NEWS`, `RELEASES`, or `VERSION` file exists at repo root.

### Recommendation: **CalVer for the repo, an independent protocol-version for the wire — not blanket SemVer**

Reasoning about *fit*, not cargo-culting SemVer:

- **SemVer's core signal is API-compatibility for downstream dependents.** dowiz has **no external
  consumers and no stable public API** (README self-describes as *"pre-1.0 / experimental … not yet a
  production GA"*). SemVer itself defines `0.y.z` as "initial development, anything may change" — so
  every release would sit in the zone where SemVer explicitly carries *no* compatibility promise. It
  would be technically valid but **low-information** at this stage.
- **The project already versions by date, everywhere.** Docs, roadmaps, and branches all carry
  `YYYY-MM-DD` (`CORE-ROADMAP-STANDARD-2026-07-17`, `plane-maintainer/2026-07-14-daily-run`, …). The
  work is a **continuous research/build stream shipped in waves**, not a library cutting
  compatibility-guaranteed releases. **CalVer communicates the axis that actually varies (when), and
  aligns 1:1 with the existing convention.**
- **But two artifacts DO need real compatibility semantics:** the **kernel's event-log / on-disk
  format** and the **mesh-protocol wire format** — independent nodes must interoperate, so a
  format-breaking change is a genuine breaking change even with zero "app" consumers.

**Concrete scheme (two tracks):**

| Track | Scheme | Bumps when | First value |
|---|---|---|---|
| **Repo / product tag** (DeliveryOS + docs corpus) | **CalVer `YYYY.MM.PATCH`** (e.g. `2026.07.0`) | a coherent wave lands (this session = the first baseline) | tag **`2026.07.0`** on `main` now |
| **Kernel + mesh wire/format** | **integer or SemVer `KERNEL_PROTO_VERSION` / `MESH_WIRE_VERSION`**, versioned *in code*, independent of the product tag | a determinism-affecting or wire-incompatible change ships | seed at `1` / `1.0.0`, documented in the kernel README |

Adopt **`0Y.0M`** (zero-padded) going forward for sortability, and seed a **`CHANGELOG.md`**
(Keep a Changelog format) from the roadmap-wave history so the first tag has real notes. Only revisit
a shift to product SemVer once there is an external consumer contract to protect — not before.

---

## 3. Branch hygiene

**61 remote branches** (18 more than the 43 local — the remote is the primary cleanup target).
Categorised by prefix (`git ls-remote --heads origin`):

| Category | Count | Nature | Recommendation |
|---|---:|---|---|
| `recover/*` (incl. `stash-0..6`, truncated auto-names like `recover/agentic_meta-layer_—`, `recover/harden_26_CRIT_HIGH_te`) | **13** | one-off crash/stash recovery, machine-truncated names | **archive then delete** — never permanent |
| `plane-maintainer/*` (`…-daily-run`, `…-digest`, `channel-liveness-guard`) | **6** | **bot-generated** daily/digest branches | **archive then delete**; fix the bot to not leave permanent branches |
| `docs/plane-status-*` (3 dated) | **3** | dated status snapshots | **archive then delete** (content lives in history) |
| `backup*` / `backup-wip*` | **2** | manual backup markers | **archive then delete** (redundant with tags) |
| **Subtotal: scratch / bot / backup / snapshot** | **24 (~39%)** | not meant to be permanent | **highest-volume cleanup** |
| `feat/*` | 13 | active/in-flight arcs (agentic-mesh, spectral, hermetic, sovereign-core, harness-llm-backend…) | **keep — active** |
| `fix/*` | 11 | fixes; several likely merged | **triage per-branch**: delete merged, keep open |
| `chore/*` | 7 | chores; several likely merged | **triage per-branch** |
| `governance/*`, `telemetry/plane`, `infra`, `integrate/merge-to-main`, `dependabot/*` | ~5 | mixed (one Dependabot esbuild bump — **review & merge or close**) | **triage per-branch** |

**Local corroboration:** of 43 local branches, **20 are already merged into `main`** (archivable —
notably the whole `feat/pa-T1…T8` tranche) and **23 are unmerged** (active or abandoned).

**Recommendation (categories, not specific branches — per-branch deletion needs a human given the
concurrent work):**
1. **Archive-then-delete** the 24 scratch/bot/backup/snapshot remote branches. "Archive" = a
   lightweight `archive/<name>` tag or a single `git bundle` before deletion, so nothing is lost —
   consistent with the repo's own move-not-delete memory rule.
2. **Fix the `plane-maintainer` bot** so it stops creating permanent branches (use ephemeral refs or
   auto-delete-on-merge; the repo's `deleteBranchOnMerge` setting should be turned **on**).
3. **Triage `fix/*` + `chore/*`** against `main`: delete the merged ones, keep the rest.
4. Leave all `feat/*` alone — that is live work.

---

## 4. README / docs discoverability

**The root `README.md` is accurate and current — the opposite of the bebop2/V6 drift case.** Every
concrete claim checks out against disk:

- Referenced governance files **all present**: `LICENSE`, `NOTICE`, `TRADEMARK.md`, `DCO`,
  `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CITATION.cff`.
- Referenced code roots **all present**: `kernel/`, `bebop2/`, `web/`.
- The decommissioned `apps/` and `packages/` (per this session's roadmap) are **genuinely gone** — the
  README correctly no longer mentions them.
- Honest maturity framing (*"pre-1.0 / experimental … No fabricated maturity claims"*) and the
  `{{BRAND}}` placeholder pending the O16 dowiz-vs-DeliveryOS decision are appropriate.

**The drift is not in the README — it is in two other surfaces:**
1. **`.claude/CLAUDE.md`'s Repowise index is STALE** (indexed 2026-06-14). It lists entry points
   `apps/api/src/server.ts`, `apps/web/src/index.ts`, `packages/*` and an "API/UI/Application/Config"
   layer map — **all referencing the now-deleted `apps/`+`packages/` tree**. Any agent trusting that
   index is navigating a repo that no longer exists. **Re-index or annotate as superseded.**
2. **`docs/design/` is fragmented**: 6+ overlapping roadmaps coexist —
   `CORE-ROADMAP-2026-07-17`, `CORE-ROADMAP-INDEX.md`, `CORE-ROADMAP-STANDARD-2026-07-17`,
   `MASTER-ROADMAP-10-PHASES-2026-07-14`, `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16`,
   `ROADMAP-GROUND-TRUTH-{2026-07-11,2026-07-14}`, plus `sovereign-roadmap-2026-07-16/`. The
   in-repo `CORE-ROADMAP-STANDARD-2026-07-17.md` **already prescribes the fix** (§0/§3/§4:
   SOVEREIGN doc becomes the single canonical entry, older MASTER-* docs get SUPERSEDED-BY banners,
   `CORE-ROADMAP-INDEX.md` is the crosswalk). That consolidation plan exists but is only partially
   executed — **finish it**.

**Also worth a README/discoverability note:** there is **no `docs/ARCHITECTURE.md`-style single map**
linked from the README; a newcomer gets the 3-bullet "what it is" but must dive into a 250-doc
`docs/design/` to go deeper. A short, stable `docs/ARCHITECTURE.md` (kernel → bebop2 → DeliveryOS,
one page) pointing at the canonical roadmap would close the gap.

---

## 5. Release / changelog practice

- **No `CHANGELOG.md` / release-notes practice of any kind** (no `CHANGELOG*`, `HISTORY*`,
  `RELEASES*`, `NEWS*`, `VERSION*` at root).
- **No GitHub Releases** (0 tags ⇒ nothing to attach a release to; API scope also blind).
- The de-facto "changelog" today is the commit stream plus the dated `docs/design/*` blueprints and
  the memory topic files — real audit trail, but **not consumable as version history** and not
  visible on the GitHub repo surface.

**Recommendation:** seed `CHANGELOG.md` (Keep a Changelog) from the roadmap-wave history, cut the
first annotated tag `2026.07.0` on `main`, and thereafter draft a GitHub Release per CalVer tag from
the changelog section. This is trivial once the tag exists and makes the wave cadence legible to any
future reader (human or agent).

---

## 6. Target "beautiful and correct" state (concrete)

1. **Versioning:** an annotated CalVer tag on every landed wave, starting `2026.07.0` on `main`;
   `0Y.0M.PATCH` going forward; an independent in-code `KERNEL_PROTO_VERSION` / `MESH_WIRE_VERSION`
   for the kernel/mesh wire format. Local defensive tags renamed under a clear `backup/` /
   `recovery/` namespace and **not** conflated with release tags.
2. **Changelog / releases:** `CHANGELOG.md` at root (Keep a Changelog); one GitHub Release per CalVer
   tag, body generated from the changelog.
3. **Branches:** the 24 scratch/bot/backup/snapshot remote branches archived (bundle/tag) then
   deleted; `deleteBranchOnMerge` enabled; `plane-maintainer` bot switched to ephemeral refs;
   `fix/*`+`chore/*` triaged; only `main` + active `feat/*` (+ open fixes) remain — target **≤ ~20
   remote branches**.
4. **Docs:** the `CORE-ROADMAP-STANDARD` consolidation finished — SOVEREIGN doc as sole canonical
   roadmap, SUPERSEDED-BY banners on the 5 older MASTER-* docs, `CORE-ROADMAP-INDEX.md` as crosswalk;
   a one-page `docs/ARCHITECTURE.md` linked from the README; the stale Repowise index in
   `.claude/CLAUDE.md` re-indexed or annotated.
5. **Repo tree:** committed generated/output dirs removed from tracking (see §7).
6. **GitHub metadata (once a PAT can see the repo):** set `description` to the README's one-liner;
   set **topics** e.g. `rust`, `wasm`, `webassembly`, `post-quantum`, `mesh-network`,
   `event-sourcing`, `deterministic`, `delivery`, `agpl`, `offline-first`; confirm the declared
   `AGPL-3.0-or-later` license shows on the repo card; decide the `{{BRAND}}` O16 flip separately.

---

## 7. Bonus finding — committed build/test output artifacts

Six generated/output directories are **tracked in git** at repo root:
`temp/`, `dogfood-output/`, `graphify-out/`, `qa-shots/`, `qa-onboarding-shots/`, `playwright-report/`.
Two of them (`playwright-report/`, `graphify-out/`) **are listed in `.gitignore`** — meaning they
were committed **before** being ignored, so the ignore rule is now inert for the already-tracked
files. **Recommendation:** `git rm -r --cached` these paths and let `.gitignore` take effect
(non-destructive to the working tree). This trims the tree and stops churn/noise in diffs. (Low risk,
low urgency — cosmetic vs. the §2–§4 items.)

---

## Appendix — commands run (all read-only)

```
gh repo view SyniakSviatoslav/dowiz --json …          # 404 / could-not-resolve
gh api repos/SyniakSviatoslav/dowiz                    # 404
gh repo list SyniakSviatoslav --limit 200              # dowiz absent; public-only
gh api user/repos?visibility=private                   # []
gh api -i user | grep X-OAuth-Scopes                   # empty ⇒ fine-grained PAT
git ls-remote --symref origin HEAD                     # default = main
git ls-remote --heads origin | wc -l                   # 61
git ls-remote --tags  origin | wc -l                   # 0
git branch --merged main | wc -l ; --no-merged main    # 20 merged / 23 not (local)
git ls-files temp dogfood-output graphify-out …        # tracked
```
