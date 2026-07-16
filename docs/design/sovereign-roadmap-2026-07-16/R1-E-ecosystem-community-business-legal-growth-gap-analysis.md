# R1-E — Ecosystem Ops, Community, Business, Legal & Growth — Gap Analysis

> Cluster: **D3, D5, D8, V4, V6, E2-E8, E11, E12, E46-E50, E51-E62** (2026-07-16).
> Method: every claim below is grounded in a live file read, a git command, or an honest
> "NOT BUILT / UNVERIFIABLE". Canon = `docs/design/ARCHITECTURE.md` + `STRATEGIC-VECTORS-LOCKED-2026-07-16.md`.
> SCOPE RULE (ARCHITECTURE §0) applies to every gate proposed here: canonical-repo DEV-TIME
> fence only; hubs are sovereign at runtime (M5/M9/M11).

---

## 0. Two findings that change the canon itself

### 0.1 The canon's own D3 statement is STALE — LICENSE is already AGPLv3

ARCHITECTURE.md §8 and S3 (lines 41, 128) still assert *"ADR-020: LICENSE Apache-2.0 vs
AGPLv3"*. **This is no longer true.** Verified live:

- `/root/dowiz/LICENSE` = full GNU AGPLv3 text (660 lines). `origin/main:LICENSE` = AGPLv3 too.
- Flipped at commit `ac1caba40` (2026-07-14 18:26) — *"feat(oss): AGPLv3 + DCO + NOTICE + TM
  policy"* — ancestor of HEAD. This landed AFTER the P10 audit (same day, morning), which is
  why `docs/design/P10-OSS-READINESS-AUDIT.md` §1/§3/§4 (Apache-2.0, no DCO, no NOTICE) are
  now stale on gates 1-3.
- In-tree today: `NOTICE` (AGPLv3 + TM notice), `DCO` (DCO 1.1), `TRADEMARK.md` (usage
  policy + EUTM = operator asset), `CONTRIBUTING.md` (DCO sign-off), `MANIFESTO.md` (C1-C13,
  C6 = "AGPLv3 + trademark + DCO, gated on secrets scrub").

**Relicensing mechanics (why the flip is legally sound):** the repo is ~97.8% single-authorship
(BRAIN-TOPOLOGY residue stat); a sole copyright holder may relicense outright. Apache-2.0 →
AGPLv3 is additionally one-way compatible (Apache-2.0 code may be incorporated into
(A)GPLv3 works). The only third-party commit is a dependabot version bump (non-creative).
No consent problem exists.

### 0.2 The scrub blocker is substantively RESOLVED (newest evidence wins, DECISIONS.md D8)

Canon §8 still says *"force-push scrub BLOCKED (red-line)"*. The evidence chain says otherwise:

1. **2026-07-03** incident: prod Supabase creds in 12 tree files + 4 history commits
   (memory `secrets-exposure-incident-2026-07-03`). Repo private → not externally exposed.
2. **2026-07-05**: operator ROTATED both passwords (creds dead); local
   `git-filter-repo --replace-text` scrub over all 1014 commits verified (0 hits).
3. **2026-07-13**: `docs/red-team/2026-07-13/H8-SECRET-SCRUB-RUNBOOK.md` — status
   **"CLOSED locally + GitHub GC VERIFIED NOT NEEDED"**. Full scan of all 35,818 reachable
   blobs = ZERO real secrets; the only 2 real RSA-key blobs were dangling-only (never on a
   pushable branch); GitHub-side existence probe = HTTP 404 on both. A full SHA-rewrite
   force-push was assessed and **declined as theater**.
4. **2026-07-16** P10 FORCE-PUSH DECISION (memory): scrub-commit `f9ab28ff1` already ancestor;
   history `.env` = templates only; `origin/main` already points at scrubbed tip `6c7212b5`
   via safe FF → GitHub history already clean. Force-push = redundant + irreversible.
   Revisit only if operator explicitly wants SHA rewrite.

**Residual (cheap, non-blocking):** the 2026-07-05 note mentioned 26 stale remote branches with
pre-scrub history; the 2026-07-16 decision found only templates. One re-verify is warranted:
`gitleaks` over ALL `origin/*` refs, logged as an artifact (Phase E-4). The P10 audit §2
("credential-shaped strings remain recoverable") is superseded by the newer, deeper H8 +
07-16 evidence — per DECISIONS.md D8 precedence, newest wins.

**What actually remains operator-gated for D3/ADR-020:** (a) EUTM filing, (b) the explicit
public-flip "go" (one-way door), (c) optional SHA rewrite (declined). NOT the scrub itself.

---

## 1. Anchor-by-anchor

### D3 — Legal / license (ADR-020: AGPLv3 + TM + DCO)

**Current state:**
- AGPLv3 + NOTICE + DCO + TRADEMARK.md + CONTRIBUTING.md + MANIFESTO.md all in tree (§0.1).
- bebop repo is AHEAD: `GOVERNANCE.md` (AGPL-3.0-or-later, DCO no-exceptions, roles, owner
  command reference for repo settings/branch protection/topics), `CONTRIBUTING.md`, `DCO.md`,
  `CITATION.cff` (license: AGPL-3.0-or-later), `advisories/` (vendored advisory-db), `deny.toml`.
- rsa RUSTSEC-2023-0071 triaged with `innovate:` marker at `kernel/Cargo.toml:31` (verified) —
  the falsifiable-trigger template V3 mandates for every future suspension (E53).

**Remaining gaps (file-verified):**
1. **False CI claim:** `CONTRIBUTING.md:17` says "CI rejects commits without a valid
   Signed-off-by" — `.github/workflows/ci.yml` contains only a telemetry self-test and eqc
   math proofs. **No DCO check, no gitleaks, no license job exists.** By VERIFIED-BY-MATH this
   is an unshipped RED: the doc claims a gate that does not exist.
2. **License metadata inconsistency:** `kernel/Cargo.toml` has NO `license` field;
   `tools/async-spool/Cargo.toml` and `tools/native-spa-server/Cargo.toml` say `license = "MIT"`
   (conflicts with AGPLv3-only policy — either an intentional per-tool permissive carve-out
   that must be documented in NOTICE, or a bug to fix). No SPDX headers anywhere.
3. **Missing files ADR-020 memory calls for:** `SECURITY.md` (+ private disclosure channel —
   CONTRIBUTING.md says "contact the owner privately" with no channel named) and
   `CODE_OF_CONDUCT` — both absent (verified `ls`).
4. **ADR-020 itself is not written as an ADR.** It is referenced in 5+ docs and memory but no
   `docs/adr/ADR-020*.md` exists (verified find). The canonical legal decision lives only in
   memory and roadmap prose.
5. **EUTM pending** — operator action (see E59 for verified costs/process).
6. Canon §8/S3 stale lines (§0.1, §0.2) must be corrected — the canon may not carry
   known-false statements.

**Target:** ADR-020 written in-tree; all license metadata consistent; DCO+gitleaks CI real;
SECURITY.md + CoC present; canon corrected; public flip executed after EUTM + operator go.

### D5, D8 — HONEST CANON-GAP (deliverable, not a failure)

**(a) Re-grep confirmed (2026-07-16, this session).** In the current canon:
- Defined D-anchors: **D1** (ARCHITECTURE:32, DB), **D2** (:33, network), **D6** (:48,
  patterns), **D7** (:45, observability). That is FOUR.
- **D3 and D4 were defined in the previous revision** (`git show 4aa71b725:docs/design/ARCHITECTURE.md`):
  D3 = AGPLv3+TM legal status (line 25: "D3 partial: non-destructive files ready, force-push
  scrub BLOCKED red-line, EUTM pending operator"; line 46: "AGPLv3+TM (D3)"), D4 = "dowiz UI =
  deterministic physics/math wasm (D4)" (line 44). The `0d1935d96` mesh-pivot rewrite dropped
  the explicit D3/D4 labels; their content survives unlabeled in §1-Legal (line 36), S3
  (line 41), and E41/F47. **Recommend restoring the D3/D4 labels in canon** during the fix.
- **D5 and D8: ZERO occurrences in ANY revision of either canon doc** — checked `8180b03eb`,
  `4aa71b725`, `0d1935d96`, and HEAD `574f05604` of both ARCHITECTURE.md and
  STRATEGIC-VECTORS-LOCKED. The count "8 (D)" first appears at `4aa71b725`
  (STRATEGIC-VECTORS line 105: "6 (V) + 8 (D) + 9 (S) + 12 + 50 = 91") with only 6 D-anchors
  ever defined; the compact "D1-8" form first appears at `0d1935d96`. **The count was
  overstated by 2 at inception and has been carried forward unverified ever since** — a live
  instance of the BRAIN-TOPOLOGY self-certification pattern (claim replaces check).

**(b) Trace search (all docs/design/*.md, MEMORY.md + topic files, git log -S):** every other
"D5"/"D8" hit belongs to a DIFFERENT, colliding numbering scheme:
- Root `DECISIONS.md` (2026-07-12, AUTHORITATIVE, operator-confirmed) defines **D0-D9**:
  D5 = "Roles + adapters" (3 node roles; NOSTR/ActivityPub/MCP as bridges, never core
  transport, ML-DSA/ML-KEM envelope first); D8 = "Plan precedence — newest outranks older".
  Note this scheme CONFLICTS with the canon D-series (DECISIONS D3 = DTN transport vs
  canon D3 = legal).
- `MASTER-ROADMAP-10-PHASES-2026-07-14.md:5` "Precedence: D8 … D0–D7 (DECISIONS)" refers to
  the DECISIONS.md scheme.
- ops-reliability arc "D5-F2/D5-F8" = red-team finding IDs; deliver-v2 "D5" and
  sovereign-core "D5" = yet other local series. None are sovereign anchors.

**(c) Verdict: OPEN CANON-GAP requiring an operator decision before roadmapping.** Options:
1. **Define D5/D8** — lowest-risk candidates are transcriptions of the already
   operator-confirmed DECISIONS.md items whose themes are otherwise missing from the canon
   D-series: **D5 := roles+adapters bridge law** (DECISIONS D5) and **D8 := doc-precedence /
   merge-not-append law** (DECISIONS D8, which V2 already restates). This is a hypothesis
   about intent, NOT canon — only the operator can ratify it.
2. **Renumber** — declare the D-series as the 6 real anchors (D1,D2,D3,D4,D6,D7) and correct
   the total to 145.
Until decided, every "147 anchors" claim is off by two undefined anchors, and this document
is the falsifiable record of that. **No content was invented for D5/D8.**

### V4 — Work organization (split-track + closure-criterion)

- **Current:** PRACTICED, NOT CODIFIED. `origin/main` = frozen anchor, canonical stack on
  feature branches (ROADMAP-GROUND-TRUTH-2026-07-14; main-merge = operator gate). Arcs carry
  ad-hoc DONE/NEXT lines; the FSM arc ("DONE 2026-07-14 (99c7698f)") is the exemplar of a
  closed arc, but there is no standard closure-criterion template and no repo file defining
  stable-vs-experimental track membership or the promotion rule. Self-development = PRIMARY
  (operator directive 2026-07-13) lives only in memory, not canon.
- **Target:** split-track written into canon (stable = product/ADR-020-prep, gated by V3;
  experimental = kernel-growth, never merges to main without explicit promotion); every arc
  carries closure-criterion = done-when + falsifiable evidence + strand/archive condition.
- **Gap:** one canon section + a closure-criterion template + retrofit of active arcs. Pure
  docs work, zero code.

### V6 — Future/ecosystem (dual-track + metaphor discipline)

- **Current:** dual-track is de-facto real: experimental track thrives (kernel 152 tests,
  eqc, loop-signals); stable track's G11 (first real order) NOT achieved. bebop PQ headliner
  is substantively built (ML-DSA-65/ML-KEM-768, 144 PQ tests per roadmap baseline). Metaphor
  discipline: SLEM exemplar exists (tools/loop-signals Markov detector, advisory/fail-open),
  but canon itself uses "living-organism" (M11) adjacent to a kill-switch, not a computed
  criterion, and several docs use "emergent/swarm" bare.
- **Gap:** (a) G11 = the stable-track closure criterion (mechanics owned by the product
  cluster; this cluster owns recording it as THE criterion); (b) a one-pass metaphor audit:
  every "emergent/swarm/organism" occurrence in canon must sit adjacent to a named computed
  criterion (SLEM/escape-mass/ρ/etc.) or be reworded to "designed coordination".

### E2 — GH-Actions + gitleaks: **NOT BUILT (CI side)**

`gitleaks` binary IS installed (`/usr/bin/gitleaks`), `cargo-audit` too — but
`.github/workflows/` contains ci.yml (telemetry self-test + eqc proofs), heartbeat-monitor,
safety-floor, skill-security, visual — **zero** gitleaks / DCO / i18n / IDOR / OTP jobs
(grep verified). V3's "restore load-bearing gates as BLOCKING CI" is entirely unstarted.
Every gate ships with a falsifiable reinstatement/waiver trigger in the rsa-triage form (E53).

### E3 — Vendored + cache + audit: **PARTIAL**

No `vendor/` dir, no in-repo `.cargo/config.toml`; local `~/.cargo/registry` kept for
offline-safety (memory, disk-cleanup 2026-07-16); cargo-audit run manually (rsa triage).
bebop already vendors `advisories/` (advisory-db) + has `deny.toml`; dowiz has neither.
Gap: `cargo vendor` (or a documented registry-cache policy), `deny.toml`, and audit-in-CI
with a waiver file.

### E4 — wasm-demo → video-after-GPU: **BUILT (demo half)**

`web/` boots the kernel wasm and renders ρ/drift/FSM from kernel math only
(`web/src/app.mjs:1-8`, W17 GREEN, node EXIT=0). Video correctly deferred: W21 documents the
wgpu offline-ceiling with trigger = network cargo-add. No gap beyond keeping the trigger
recorded in canon (§8 already does).

### E5 — GitHub-Discussions + AGPL + social: **NOT BUILT / UNVERIFIABLE pre-flip**

`gh repo view SyniakSviatoslav/dowiz` cannot resolve the repo from this box (fine-grained
PAT scope) — honest status: unverifiable here; all evidence says the repo is private and the
public flip has not happened, so Discussions cannot serve a community yet. bebop
`GOVERNANCE.md` lines 39-56 contain the exact owner-command block (repo edit, topics, branch
protection, DCO) to replicate for dowiz at flip time. Social (X/Telegram-lite) — nothing shipped.

### E6 — "Sovereign PQ delivery infra" marketing (delivery = by-product): **DOCS-PARTIAL**

MANIFESTO §0 one-sentence is the honest kernel of the pitch. No positioning/landing doc for
the mesh-pivot era; memory's `docs/design/dowiz-brand/BRAND-BIBLE.md` **does not exist on
disk** (verified — stale memory pointer). Gap: one positioning doc + landing copy where
bebop PQ protocol is the headliner and dowiz delivery is proof-of-concept-on-top.

### E7 / E56 — B2B + grants: **NOT BUILT (artifacts)** — web-verified facts below

No grant dossier, no in-tree pricing doc surviving the pivot (ADR-020 pricing-v2 —
hosted-cloud as only sold path, Free≤50/$0 … Business∞/$59 — lives only in memory).
**NLnet NGI Zero Commons Fund (verified 2026-07-16):** first-time proposals ≤ €50,000
(subsequent ≤ €150k, lifetime cap €500k), 1-12-month projects, outputs MUST be recognised
FOSS licenses (AGPLv3 qualifies), R&D focus + clear European dimension (EU/UA delivery
protocol fits E60), concise English application via nlnet.nl/propose. Call 2026-06Z had
€6.1M; its deadline (2026-06-01 12:00 Brussels) has PASSED — calls recur; target the next
opening. Sources: [NLnet Commons Fund guide](https://nlnet.nl/commonsfund/guideforapplicants/),
[NLnet apply](https://nlnet.nl/propose/), [call listing](https://www.subsdy.com/grants/ngi-zero-commons-fund-2026-06z).

### E8 / E51 — graph-wiki, **PER-HUB REPLICATED** (corrected form): **SEED BUILT, replication NOT BUILT**

Using the HYDRA-sweep correction (canon §8: "C4 single-graph wiki → PER-HUB REPLICATED, no
central SPOF"; F48), NOT the original E8-A single-wiki lock. Current: `kernel/src/living_knowledge.rs`
exists and is PRIMARY recall (recall@5 = 1.0, W18) — that is the single-hub graph substrate.
Missing: per-hub replication + opportunistic sync over the protocol (depends on the mesh
cluster's sync layer). Done-test: two hubs exchange wiki deltas with no central authority.

### E11 — native + pgrust-backup: **BUILT**

`kernel/Cargo.toml`: `pgrust = ["dep:sqlx", "dep:tokio"]` optional feature; SQL adapter in
`kernel/src/event_log.rs` compiled only under the feature; native vectorless = default (D1).
`deploy/pgrust.env|.service|.toml` exist. Residual gap: a restore-from-pgrust drill has never
been evidenced — fold into the E50 drill.

### E12 — i18n UA+EN+AL: **NOT BUILT — full rebuild required**

The legacy TS i18n (`packages/ui/src/lib/i18n.ts`) was DELETED at `db766de47` ("remove
legacy JS/TS thin-layer, kernel is now sole source of truth"); only untracked dist/
node_modules remain under apps/packages. The canonical `web/` UI has `lang="en"` and zero
i18n framework, zero locale files, zero CI gate (grep verified across web/kernel/engine).
Target: Rust-native locale table (std-only preferred; any new dep ⇒ DECART per E62),
UA+EN+AL with EN-main, all-locales-via-OSS contribution path, and a **canonical-repo
blocking CI completeness gate ONLY** (SCOPE RULE — hubs ship any locale set).

### E46-E50 — Ops cluster

- **E46 tracing+OTel: PARTIAL.** `tracing 0.1` + `tracing-subscriber` (env-filter) in kernel
  (dev/CLI only, never wasm) = local tracing BUILT. OTel absent everywhere — compliant with
  M8 (opt-in only), but the opt-in local-sink option itself is NOT BUILT.
- **E47 claim-latency alerts: NOT BUILT.** Zero hits for claim-latency in tools/scripts/
  kernel/engine. V5-B (time diff-landing→GREEN-claim per commit, anomaly flagging — the 52s
  self-green detector) is unimplemented.
- **E48 H8 runbook: BUILT.** `docs/red-team/2026-07-13/H8-SECRET-SCRUB-RUNBOOK.md` (status
  CLOSED, evidence-rich) + `docs/ops/P1-PAUSE-SECRET-PUSH-RUNBOOK.md`.
- **E49 OpenTofu: NOT BUILT — confirmed.** No `opentofu/` dir, no tofu/terraform binary;
  `deploy/` = systemd/env files + check-no-docker.sh only.
- **E50 COLD-restore: HALF-BUILT.** Archives exist (`/root/.backups/cold/`: state-db,
  claude-projects, buckets-c, 2026-07-16 + preprune db), creation was verify-gated. **No
  restore DRILL has ever run** (no script, no runbook, no drill log). And a live ops defect:
  `tools/deep-clean` is built + both Hermes cronjobs registered (deep-clean-daily 843e5b0ee3ba,
  weekly-audit 8e652764b103) **but `hermes cron list` warns "Gateway is not running — jobs
  won't fire automatically"** (verified this session). The hygiene loop is currently dead.

### E52-E55, E57-E62 — Community / business / growth remainder

- **E52 DCO+AGPL: files BUILT, CI enforcement MISSING** (see D3 gap 1). bebop side complete
  and ahead (GOVERNANCE/CONTRIBUTING/DCO.md/CITATION.cff, CI runs boot+test+typecheck).
- **E53 rsa-triage: DONE** — `kernel/Cargo.toml:31` `innovate:` marker verified. This is the
  canonical template every future gate-suspension must replicate (named owner + checkable
  revisit condition), per V3.
- **E54 AGPLv3+TM: see D3.** **E55 Manifesto: BUILT** (`/root/dowiz/MANIFESTO.md`, C1-C13).
- **E57 usage-PQ-api: NOT BUILT** — no billing/usage surface exists in the Rust stack (legacy
  TS billing deleted with the thin-layer). Keep as spec-only until a falsifiable demand
  trigger (first paying B2B inquiry) — building it now violates C8/ponytail.
- **E58 NO-COURIER-SCORING: BUILT in kernel** (references in `kernel/src/domain.rs`,
  `wasm.rs`, `event_log.rs`, native-spa-server) + bebop dormant pre-commit guards
  (fc1805f). Missing: the dowiz-side CI wire (fold into E2 phase).
- **E59 TM/EUTM (operator): verified facts** — EUIPO e-filing €850 one class (+€50 second,
  +€150 each from third), payable within 1 month; Fast Track free with harmonized-database
  terms (exam ~10 business days); 3-month opposition window; 4-6 months total unopposed,
  12-18 if opposed. ADR-020 §6.6 flag stands: "DeliveryOS" is a weak descriptive mark —
  brand decision ("dowiz" is the stronger mark) needed BEFORE filing. EUIPO SME Fund 2026
  vouchers may reimburse part of the fee. Sources:
  [EUIPO fees](https://www.euipo.europa.eu/en/trade-marks/before-applying/fees-payments),
  [EUIPO after applying](https://www.euipo.europa.eu/en/trade-marks/after-applying),
  [SME Fund 2026](https://www.euipo.europa.eu/en/sme-corner/sme-fund/2026/vouchers/trademarks-and-designs).
- **E60 EU-UA: positioning only** — no artifact needed beyond the grant dossier's European-
  dimension section (E7).
- **E61 kernel math-first: substantively BUILT** (kernel 152 tests, eqc compiler, spectral/FSM
  authority) — it is the growth-track exemplar; no gap in this cluster's scope.
- **E62 DECART-gate deps: PRACTICED, NOT ENFORCED.** DECART reports exist
  (KERNEL-OBSERVABILITY-DECART-2026-07-15.md et al.); no CI detector. Gap: a Cargo.lock/
  package-diff job that goes RED on a new dependency without a DECART report reference
  (dev-time fence, SCOPE RULE).

### Prior art reused (not duplicated)

`docs/design/ecosystem-strategy/` (EC-01..EC-20; ★caching = only real infra gap — owned by
the storage cluster, noted here only as the cross-reference), `P10-OSS-READINESS-AUDIT.md`
(GO/NO-GO table structure reused in Phase E-4's done-test), H8 runbook, bebop GOVERNANCE.md
(settings template), memory arcs `open-source-goal-adr020-2026-07-03` and
`secrets-exposure-incident-2026-07-03`.

---

## 2. BUILD PHASES (ordered, zero exceptions, every anchor lands somewhere)

### Phase E-1 — Canon truth & D-series repair (docs only, do FIRST)
**Anchors:** D3(doc-side), **D5/D8 (operator decision)**, V4, V6(metaphor+G11-criterion), E55(link), E53(template codified).
**Dependencies:** none — pure docs; everything later cites canon, so canon must stop carrying
known-false statements first.
**Scope:** (1) Correct ARCHITECTURE.md §8+S3: LICENSE = AGPLv3 since ac1caba40; scrub =
resolved per H8 + P10-decision 2026-07-16; remaining gates = EUTM + public-flip go. Restore
the dropped D3/D4 labels. (2) **Present the operator the D5/D8 memo: define (candidates =
DECISIONS.md D5 roles+adapters, D8 newest-wins precedence) OR renumber the total to 145.
This phase CANNOT close without that decision — an acceptable outcome is "operator must
define these before R2 merge", recorded in canon §8.** (3) Write ADR-020 as a real
`docs/adr/` file. (4) Codify V4 split-track + closure-criterion template; retrofit active
arcs. (5) V6 metaphor audit pass over canon.
**Falsifiable done-test:** `grep -n "Apache-2.0" docs/design/ARCHITECTURE.md` → 0 hits;
D-series enumerable D1..D8 complete OR total corrected everywhere (grep "147" → consistent);
every active arc header has the 3 closure fields; every emergent/swarm/organism hit in canon
sits within 3 lines of a named computed criterion.

### Phase E-2 — Governance gates become real CI (dev-time fences, SCOPE RULE)
**Anchors:** E2, E52(CI half), E58(dowiz wire), E62, E3, D3(gap 1+2).
**Dependencies:** E-1 (V3 trigger-form + canon truth); independent of product code.
**Scope:** gitleaks job (blocking); DCO Signed-off-by check (makes CONTRIBUTING.md:17 true);
cargo-audit + deny.toml with an rsa-triage-form waiver file; NO-COURIER-SCORING grep guard;
new-dep DECART detector (Cargo.lock diff ⇒ require DECART doc reference); vendoring decision
(`cargo vendor` or documented cache policy); fix license metadata (kernel Cargo.toml
`license = "AGPL-3.0-or-later"`, decide+document the two MIT tool crates).
**Falsifiable done-test:** on a scratch branch, a planted fake secret, an unsigned commit,
and an un-DECARTed new dep each turn CI RED; a clean signed commit turns it GREEN; the RED
runs are linked as artifacts.

### Phase E-3 — i18n rebuild + ops floor
**Anchors:** E12, E46, E47, E49, E50, E11(drill).
**Dependencies:** E-2 (CI exists to host the i18n gate); deep-clean/COLD work is independent
and can run in parallel with E-2.
**Scope:** (1) Rust-native i18n: UA+EN+AL locale table (std-only preferred, else DECART),
wired into web/ wasm UI; blocking completeness gate in CI (canonical repo ONLY). (2) Start
the Hermes cron gateway (`hermes gateway install --system`) so deep-clean jobs actually fire.
(3) COLD-restore drill: script + runbook; restore state-db archive to scratch,
`integrity_check=ok`, log artifact; include a pgrust-restore leg (E11). (4) claim-latency
logger (V5-B): per-commit time(diff→GREEN-claim) ledger + anomaly flag. (5) OTel opt-in
local-sink behind a feature flag (M8-compliant). (6) E49: minimal OpenTofu module for the
single host (systemd units + env files) — or an honest DECART declining it; silence is not
an option.
**Falsifiable done-test:** i18n gate RED on a missing key / GREEN on full set; `hermes cron
status` shows gateway running + next-run times; restore-drill log with integrity_check=ok
exists; claim-latency ledger has ≥1 real commit entry; tofu plan applies clean OR a DECART
doc exists.

### Phase E-4 — Public-flip readiness (operator-gated; prep is agent work)
**Anchors:** D3(close), E5, E54, E59, E52(bebop parity).
**Dependencies:** E-2 hard (gitleaks CI green is a flip precondition); E-1 (canon truth).
The flip itself is a ONE-WAY DOOR — never autonomous (standing rule).
**Scope — agent-preparable now:** SECURITY.md + named private disclosure channel;
CODE_OF_CONDUCT; README polish; repo-settings script cloned from bebop GOVERNANCE.md
lines 39-56 (topics, branch protection, Discussions enable); stale-remote-branch gitleaks
sweep over all origin refs (closes the last §0.2 residual); CITATION.cff for dowiz.
**Scope — operator-only:** brand decision (dowiz vs weak "DeliveryOS") → EUTM e-filing
(€850, Fast Track, SME-Fund voucher check) → explicit public-flip go.
**Falsifiable done-test:** re-run the P10 GO/NO-GO table — all 6 gates ✅ with evidence
(gate 4 = sweep artifact, gate 5 = EUTM application number, + written operator go);
post-flip `gh repo view` returns AGPL-3.0 license + Discussions enabled.

### Phase E-5 — Ecosystem growth engine
**Anchors:** E6, E7/E56, E57(spec-only), E8/E51, E60, V6(G11 recorded).
**Dependencies:** E-4 (public repo is the surface grants/community point at); E8 replication
additionally depends on the mesh cluster's protocol sync layer (cross-cluster).
**Scope:** positioning doc + landing copy ("sovereign PQ delivery infra", bebop protocol =
headliner, delivery = by-product); NLnet NGI Zero dossier (≤€50k first-time, European
dimension = EU/UA, FOSS mandate already satisfied) targeted at the next call deadline;
pricing/B2B doc refreshed against the mesh pivot (hosted-cloud sold path per ADR-020);
usage-PQ-api written as spec ONLY with a falsifiable build-trigger (first paying inquiry);
living_knowledge per-hub replication once the mesh sync layer lands (two-hub delta-exchange
test).
**Falsifiable done-test:** grant application submission receipt OR a dated, reasoned
decision not to apply; positioning doc merged to canon-adjacent docs; two-hub wiki
delta-exchange test green; G11 criterion recorded in canon with its owner cluster.

---

## 3. Blockers & cross-cluster notes (for the R2 merge pass)

1. **D5/D8 operator decision** blocks the "147 anchors" arithmetic used by ALL clusters —
   R2 should not publish a total until Phase E-1 item 2 resolves.
2. **D3 is NO LONGER blocked on a scrub** — the real remaining gates are EUTM + operator
   public-flip go (+ one cheap remote-branch sweep). R2 should re-word any phase that
   inherits the stale "force-push BLOCKED" line.
3. **Hermes cron gateway down** — trivial fix, but until then all "automated hygiene"
   claims are false; flag to the ops-owning cluster too.
4. **E8 corrected form (PER-HUB REPLICATED)** used throughout — any other cluster still
   citing single-graph-wiki E8-A must adopt the HYDRA-sweep correction.
5. **caching gap** (ecosystem-strategy ★EC-05) is acknowledged prior art owned by the
   storage cluster (E26-30) — deliberately not double-planned here.
