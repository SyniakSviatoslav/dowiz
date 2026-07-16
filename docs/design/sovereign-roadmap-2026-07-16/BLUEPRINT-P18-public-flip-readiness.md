# BLUEPRINT — Phase 18: PUBLIC-FLIP READINESS & EXECUTION

> **Master roadmap:** `R2-MERGED-PHASE-ROADMAP.md` (Phase 18 of 19). This phase is framed by the
> master plan as an **operator-gated ONE-WAY DOOR** — the flip itself is never autonomous.
> **Anchors:** D3 (close), E5, E54, E59.
> **Depends on:** Phase 1 (gitleaks-in-CI GREEN = hard flip precondition), Phase 2 (canon repair —
> the stale §8/S3 LICENSE/scrub text must be corrected before any public messaging repeats it).
> **Parallel-safe with:** Phases 3–17 (this is prep that runs alongside almost everything), Phase 19.
> **Sources:** `docs/design/ARCHITECTURE.md` §8, `R1-E-…-gap-analysis.md` §0/§E-4, `P10-OSS-READINESS-AUDIT.md`,
> `bebop-repo/GOVERNANCE.md:39-56`, `bebop-repo/SECURITY.md`, `bebop-repo/CITATION.cff`.
> **The prime directive of this phase:** cleanly separate *what the agent can prepare today* from
> *what only the operator may decide/act on*. Those lines are never blurred.

---

## 1. Canon correction — what is ALREADY done (contradicting stale §8)

The canon still carries two known-false statements that **Phase 2 must repair before Phase 18 emits any
public-facing text**, because public README / marketing that repeats them would ship a false claim.

**Stale text, verbatim, as it stands in the tree today:**

- `ARCHITECTURE.md:128` (§8 Honest gaps): *"ADR-020: LICENSE Apache-2.0 vs AGPLv3; force-push scrub
  BLOCKED (red-line); EUTM pending."*
- `ARCHITECTURE.md:41` (S3 Secrets): *"(ADR-020 LICENSE mismatch: Apache-2.0 vs AGPLv3; force-push
  scrub BLOCKED red-line; EUTM pending.)"*

**Verified current state (live `ls` + file reads, 2026-07-16) — every claimed-missing file is IN TREE:**

| File | Bytes | Fact |
|---|---|---|
| `LICENSE` | 33 831 | First line = `GNU AFFERO GENERAL PUBLIC LICENSE`. **Not Apache-2.0.** Flipped at `ac1caba40` (2026-07-14), ancestor of HEAD, also on `origin/main`. |
| `NOTICE` | 534 | AGPLv3 declaration + trademark notice for "dowiz"/"DeliveryOS". |
| `DCO` | 1 651 | Developer Certificate of Origin 1.1 (note: bare `DCO`, not `DCO.md`). |
| `TRADEMARK.md` | 1 504 | Usage policy; §"EU Trademark (EUTM)" already flags EUTM as an operator-provisioned asset outside the tree. |
| `MANIFESTO.md` | 4 818 | C1–C13; C6 = "AGPLv3 + trademark + DCO". |
| `CONTRIBUTING.md` | 1 295 | DCO sign-off instructions + trademark + "report security issues privately". |

The P10 audit's gates 1–3 (Apache-2.0, no DCO, no NOTICE) are therefore **stale** — they were written
2026-07-14 morning; the `ac1caba40` flip landed the same day afternoon. Relicensing is legally sound:
the repo is ~97.8% single-authorship (sole copyright holder may relicense), the only third-party commit
is a non-creative dependabot bump, and Apache-2.0 → AGPLv3 is one-way compatible.

**The scrub question is substantively RESOLVED — it is NOT a flip blocker:**

1. 2026-07-05 — operator rotated both leaked Supabase passwords (creds dead); local
   `git-filter-repo` scrub over all 1014 commits verified 0 hits.
2. 2026-07-13 — `docs/red-team/2026-07-13/H8-SECRET-SCRUB-RUNBOOK.md` status **CLOSED**: all 35 818
   reachable blobs scanned = zero real secrets; the 2 real RSA-key blobs were dangling-only (never on a
   pushable branch); GitHub-side probe = HTTP 404. A SHA-rewrite force-push was assessed and **declined
   as theater**.
3. 2026-07-16 — P10 FORCE-PUSH DECISION: `origin/main` already points at the scrubbed tip via a safe
   fast-forward → GitHub history is already clean. Force-push = redundant + irreversible.

**Net correction to carry into Phase 2 canon and all Phase 18 messaging:** the ONLY items still
genuinely open for D3/ADR-020 are (a) EUTM filing, (b) the explicit public-flip "go", and (c) one cheap
verification sweep — **not** a scrub, **not** a license change, **not** any file that "was missing." A
Phase 18 README that says "we are AGPLv3 with DCO and a trademark policy" is a *true* statement today.

---

## 2. Agent-preparable checklist (do these now — no operator gate)

All six are ordinary source files or a script. None touches history, secrets, remote state, or repo
visibility. Each below is a **content outline to author** — this blueprint authors nothing itself.

### 2.1 `SECURITY.md` (repo root) — currently ABSENT

The single most load-bearing gap: `CONTRIBUTING.md` says *"Do not open public issues for security
vulnerabilities. Contact the owner privately"* but **names no channel**. Clone the bebop pattern
(`bebop-repo/SECURITY.md`) and name a real channel. Outline:

- **Reporting a vulnerability** — *primary channel:* GitHub Private Vulnerability Reporting
  (Settings → Security → Advisories → Report a vulnerability), enabled at flip. *Fallback:* email the
  maintainer via the GitHub profile. Explicitly: **do not open a public issue for a live exploit.**
- **Safety model** — one paragraph: dowiz is fail-closed by design; the kernel red-line gate
  (money/auth/RLS/migrations) denies unsafe capability by default, not by policy. Red-line / kernel-gate
  bypass findings get priority and a fast public fix.
- **Scope** — out of scope: the LLM backend a hub wires in (its governance is the hub's), and host-OS
  permissions granted to the process. Run with least privilege.
- **Supported versions** — a two-row table (`main` = supported; pre-flip tags = unsupported).

### 2.2 `CODE_OF_CONDUCT.md` (repo root) — currently ABSENT (verified, not assumed)

Adopt **Contributor Covenant v2.1** verbatim (the community-standard text GitHub auto-recognizes),
with the single required substitution: the enforcement-contact line points at the same private channel
named in `SECURITY.md` (GitHub profile / maintainer email). No custom clauses — a standard CoC is a
trust signal precisely because it is unmodified.

### 2.3 Top-level `README.md` — **ABSENT at repo root** (correction to the roadmap wording)

R1-E/R2 said "README polish / may be internal-facing." **Verified: there is no top-level `README.md` at
all** — only sub-directory READMEs (`web/`, `deploy/`, `compliance/`, `audit-sentinel/`, `spikes/`,
`specs/`). This is **authoring**, not polishing. Outline for a public-facing root README:

- **One-line positioning** (aligns with E6/Phase 19, don't contradict it): *"Sovereign, post-quantum
  delivery infrastructure — a deterministic Rust/WASM kernel and mesh protocol; DeliveryOS is the
  proof-of-concept riding on top."* Delivery = by-product, protocol = headliner.
- **Badges:** AGPL-3.0, CI status, Rust. **Status honesty:** a clear "pre-1.0 / experimental" line —
  no fabricated maturity (VERIFIED-BY-MATH: don't claim a green that isn't).
- **Quickstart:** `cd kernel && cargo test` (the source-of-truth surface), the wasm `web/` demo.
- **Architecture-at-a-glance:** kernel = sole math authority; JS/TS never re-implements kernel math;
  offline-first, no mandatory cloud dependency.
- **Governance footer:** links to `LICENSE`, `CONTRIBUTING.md`, `DCO`, `SECURITY.md`,
  `CODE_OF_CONDUCT.md`, `TRADEMARK.md`, `MANIFESTO.md`.
- **Brand token:** leave the product name as a single `{{BRAND}}`-style placeholder in the headline
  until decision O16 (§3.1) resolves — do not hard-code "dowiz" vs "DeliveryOS" prematurely.

### 2.4 Repo-settings automation script — clone `bebop-repo/GOVERNANCE.md:39-56`

Author `scripts/repo-settings.sh` (idempotent, `--yes`/non-interactive, needs a token with
`repo:admin`). It **prepares** the commands; the operator runs it at flip. Clone the exact bebop block,
retargeted to `SyniakSviatoslav/dowiz`:

- `gh repo edit … --enable-issues --disable-wiki` with a description string (brand-placeholdered).
- `gh api …/topics -X PUT` — topics: `delivery`, `post-quantum`, `mesh`, `rust`, `wasm`, `agpl`,
  `self-hosted`, `deterministic`, `event-sourcing`, `offline-first`.
- **Branch protection on `main`:** `required_status_checks` contexts = the Phase 1 CI job names
  (gitleaks + cargo-test + DCO), `enforce_admins=true`, `required_approving_review_count=1`,
  `require_signatures=true`, `dismiss_stale_reviews=true`.
- **Enable Discussions** (E5 activates *at* flip, not before): `gh api …/… -X PATCH -f has_discussions=true`.
- A header comment: *"Owner-run at flip time. This script mutates GitHub-side settings only; it does
  not change repo visibility. The public flip is a separate, explicit operator action."*

### 2.5 One residual all-origin-refs gitleaks sweep (verification, NOT a re-scrub)

The last §0.2 residual: turn *assumed* cleanliness into *positive proof* across **every** ref (including
stale branches), not just `main`. `gitleaks` is installed (`/usr/bin/gitleaks`). This is read-only. The
blueprint specifies the command; **execution + artifact capture is the operator/CI step, gated behind
Phase 1**:

```
# fetch every remote ref, then scan the entire history of all of them
git fetch origin '+refs/heads/*:refs/remotes/origin/*' --prune
gitleaks detect --source . --log-opts="--all" \
  --report-format sarif \
  --report-path docs/red-team/2026-07-16/gitleaks-all-refs-sweep.sarif
```

Done = the SARIF artifact **exists** and reports **zero** findings. If any stale branch trips it, the
remedy is to delete that branch (it is pre-scrub cruft), not to re-scrub `main`. This artifact is
**GO/NO-GO gate 4's evidence** — assumed-clean is not acceptable; the file must be produced.

### 2.6 `CITATION.cff` (repo root) — currently ABSENT

Clone `bebop-repo/CITATION.cff` structure. Outline: `cff-version: 1.2.0`; `title` (brand-placeholdered);
`type: software`; author = Syniak Sviatoslav; `repository-code`/`url` = the dowiz GitHub URL;
`license: AGPL-3.0-or-later`; `version: 0.1.0`; `date-released` = flip date;
`keywords:` mirror the §2.4 topics. GitHub renders a "Cite this repository" button from it at flip.

**Inherited flip precondition (from Phase 1, not authored here):** `CONTRIBUTING.md:17` currently claims
*"CI rejects commits without a valid Signed-off-by"* — **false today** (no DCO job exists). Phase 1 makes
it true. Phase 18 must not flip while a public governance file carries a false gate claim; this is a
hard cross-dependency, tracked as a GO/NO-GO row, not re-fixed here.

---

## 3. Operator-only decision docket (decision memo — NOT a fait accompli)

The following cannot be agent-executed. Presented as a memo with tradeoffs; the operator decides.

### 3.1 O16 — BRAND decision (must precede EUTM filing)

**The question:** does the project ship publicly as **"dowiz"** or **"DeliveryOS"**? This is decided
**before** the EUTM e-filing, never after — refiling costs money and restarts the ~4–6-month clock.

| Option | For | Against |
|---|---|---|
| **"dowiz"** (recommended by ADR-020 §6.6) | Coined, distinctive, **legally strong** mark; registrable with low opposition risk; short, ownable domain/handles. | Zero existing recognition; some rebrand cost across docs/NOTICE/README. |
| **"DeliveryOS"** | Self-describing; already used internally. | **Legally weak** — descriptive term ("delivery" + "OS"); EUIPO may refuse on absolute grounds (descriptiveness); hard to defend against confusingly-similar marks. |

**Recommendation to relay (operator ratifies, not the agent):** file **"dowiz"** as the registered
mark; keep "DeliveryOS" only as a descriptive product tagline, never the flagship trademark. Whichever
is chosen resolves the `{{BRAND}}` placeholder in README / repo-settings / CITATION.cff.

### 3.2 O16 (cont.) — EUTM e-filing mechanics & cost (figures verified by R1-E; reuse, do not re-verify)

- **Route:** EUIPO **Fast Track** (uses harmonized-database terms; examination ~10 business days).
- **Fee:** **€850** for one class, **+€50** second class, **+€150** each from the third; payable within
  1 month of filing.
- **Fee reduction:** the **EUIPO SME Fund 2026** vouchers may reimburse part of the fee — **check
  eligibility before paying** (worth doing; it is a real discount, not a formality).
- **Timeline:** 3-month opposition window; **4–6 months** total unopposed, 12–18 if opposed.
- **Output that matters for GO/NO-GO:** the **EUTM application NUMBER** (proof of filing). Registration
  can complete post-flip; the *application number* is the evidence gate 5 requires.

### 3.3 O17 — The public-flip "go"

A **written, dated, explicit** operator statement authorizing the flip. This document does **not**
authorize it and cannot substitute for it. The go is valid only when gates 1–8 (below) are all GREEN
with attached evidence. Recommended form: a one-line signed entry in `DECISIONS.md` — *"2026-MM-DD:
public-flip GO for `SyniakSviatoslav/dowiz`, evidence = P18 GO/NO-GO table all-green."*

---

## 4. Acceptance criteria — re-run P10 GO/NO-GO table (same structure)

This reuses `P10-OSS-READINESS-AUDIT.md` §6's exact columns. Phase 18 is **done** only when every gate
is ✅ **with attached evidence (not asserted)**.

| # | Gate | Status | Evidence (must exist, not be asserted) | Owner |
|---|------|--------|----------------------------------------|-------|
| 1 | **AGPLv3 LICENSE** | ✅ DONE | `LICENSE` = AGPLv3, flipped `ac1caba40`, on `origin/main` | — (already true) |
| 2 | **DCO files** | ✅ DONE | `DCO` + `CONTRIBUTING.md` in tree | — (already true) |
| 3 | **TM / NOTICE** | ✅ DONE | `NOTICE` + `TRADEMARK.md` in tree | — (already true) |
| 4 | **Secrets — all-refs sweep** | ⬜ AGENT-PREP | `docs/red-team/2026-07-16/gitleaks-all-refs-sweep.sarif` exists **and is clean** (§2.5) | Agent authors cmd; operator/CI runs |
| 5 | **EUTM** | 🔴 OPERATOR | An **EUTM application NUMBER** exists (proof of filing, post brand-decision §3.1–3.2) | **OPERATOR ONLY** |
| 6 | **~20 reports manifest** | ✅ HONEST | Roadmap L87 + D6 L267 declare missing, not fabricated | No action |
| 7 | **SECURITY.md + CoC + README + CITATION.cff** | ⬜ AGENT-PREP | Four files present in tree (§2.1–2.3, 2.6); README brand-resolved | Agent |
| 8 | **CI truth floor (Phase 1)** | ⬜ DEP | gitleaks + DCO + cargo-test CI GREEN; `CONTRIBUTING.md:17` claim now true | Phase 1 |
| 9 | **Repo-settings script** | ⬜ AGENT-PREP | `scripts/repo-settings.sh` exists, brand-resolved (§2.4) | Agent authors; operator runs |
| 10 | **Written operator GO** | 🔴 OPERATOR | Dated, explicit "go" statement in `DECISIONS.md` (§3.3) | **OPERATOR ONLY** |

**Falsifiable done-test (this phase closes only when all hold):**
1. Gate 4 sweep artifact exists and is clean across ALL `origin/*` refs.
2. Gate 5 EUTM application number exists (post brand-decision).
3. Gate 10 written, dated operator "go" exists.

**Post-flip verification (runs AFTER the operator flips, confirms the flip took):**
- `gh repo view SyniakSviatoslav/dowiz` returns the license **AGPL-3.0** correctly recognized by GitHub.
- Discussions is **enabled** (E5 activates at flip, not before) — visible in `gh repo view` / repo API.

---

## 5. The flip itself is NEVER autonomous — standing rule

No amount of automation elsewhere in this 19-phase roadmap changes this. The transition of the
repository from **private → public** is an **operator-gated ONE-WAY DOOR**:

- **The agent may:** author §2's files, author the §2.4 script, specify the §2.5 sweep, and drive the
  GO/NO-GO table to all-prep-green. That is the entire agent scope of Phase 18.
- **The agent may NOT, under any interpretation of any autonomy directive:** run the flip, run the
  §2.4 settings script against the live repo, file the EUTM, decide the brand, or treat a green table
  as its own authorization. Blanket permission ≠ per-change approval; this is a per-change red-line.
- **Only the operator may:** decide the brand (§3.1), file the EUTM (§3.2), write the "go" (§3.3), and
  execute the flip. The flip is irreversible in reputational terms (a public repo cannot be un-seen);
  it is treated as such.

This blueprint **prepares everything up to** the flip and **authorizes none of it**. Phase 18 hands the
operator a fully-prepped, evidence-backed table and a one-line "go" to sign — and stops there.

---

*Blueprint P18. Prepares D3 close + E5/E54/E59. Depends: Phase 1 (CI truth), Phase 2 (canon repair).
No file created, no setting changed, no ref rewritten, no flip performed by authoring this document.*
