# Adoption EV — bebop: max-EV moves to get people to TRY and USE it

> Research + strategy, 2026-07-11. **Read-only**: nothing in `/root/bebop-repo` or `/root/dowiz`
> was modified producing this doc; this file (in the dowiz research dir) is the only artifact
> created. Lens: BEBOP. Binding constraint: **zero-friction**. Honest sequencing constraint:
> G07 (`docs/design/gap-blueprints-2026-07-11/G07-program-spine-arbiter.md`) ranks bebop **PARKED
> after a capture session**, behind dowiz business validation (P0). So the ranking below privileges
> *cheap, passive, prerequisite* moves that survive parking over *operator-week campaigns* that
> contradict G07. Sources: bebop README/llms.txt/llm-manifest/GOVERNANCE, live repo probe
> (git + code surface), G07/G08/G09 blueprints, and three cited web-research passes (§7).

---

## 0. TL;DR

- **The binding gap is not marketing — it is that no stranger can install bebop today.** The three
  documented install paths are all broken or stale: `npm install -g bebop-agent` (the package
  **does not exist** on npm — 404, verified), the README quick-start `npm install && npm run build`
  claims "no Rust needed to run" but the live runtime is native Rust and `npm run build` =
  `cargo build --release -p bebop` (Rust *required*), and `bin/bebop` is a bash shim that runs
  `cargo run` from a cloned tree. Every fast-growing agent CLI (opencode, goose, aider) leads with a
  **single working command**; bebop currently has **zero**. Fix this first or nothing else matters.
- **The zero-friction "wow" already exists in code.** bebop's differentiators — guard-OS denying on
  red, living-memory `recall`, PQ `node` identity, `govern`, tamper-evident `ledger` — all run
  **keyless and deterministic** (native crate, no HTTP/API-key path). A canned no-key demo is not a
  build project; it is *packaging what already works* as the first thing a user sees. **Feasibility:
  strong YES** (§3).
- **AGPL is fine.** For a locally-run CLI the network-copyleft clause never triggers for an
  individual user; evidence says net effect on individual trial is neutral-to-slightly-positive.
  Keep it, market it as a sovereignty signal, **do not add a CLA** (§4).
- **The delivery-protocol thread has no zero-friction adoption story. None. Park it** (§5).
- **Sequencing:** the top moves are hours of work, passive, and prerequisite-shaped — they make the
  repo *trial-able the moment attention returns* without consuming the operator-weeks G07 assigns to
  dowiz. Campaigns (Show HN / PH / subreddits) are explicitly deferred (§6).

---

## 1. Who realistically tries bebop in 2026

The audience is developers already running local coding agents — a crowded field (Claude Code,
opencode ~180k★, goose, aider ~42k★, OpenHands ~70k★, cline, continue, crush). bebop's honest
differentiators vs that field: **guard-OS safety kernel** (deny auth/money/secrets/migrations by
default, `boot` proves it), **living VSA memory**, **PQ node identity/vault**, **AGPL sovereignty
posture**, **keyless-by-default**. What the research shows actually makes devs try a new agent CLI
(§7.1, all cited):

- **A single frictionless install** — universally the headline path (curl|sh one-liner for
  opencode/goose/aider, with pkg-manager fallbacks). bebop fails this today.
- **A benchmark or a sharp claim that makes people *look*** — OpenHands rode SWE-bench Verified;
  bebop has no benchmark surface (it is a control plane, not a model), so its "look at me" hook must
  be the *guard proof* ("math, not vibes — `boot` proves the gates go RED"), which is unusual and
  demonstrable, and the keyless demo.
- **Secondary star spikes come from releases that remove a cost/billing barrier** (opencode's
  jumps tracked to "route your existing Claude Max sub through it"). bebop's analogue is
  *free-by-default + keyless native* — a real, ownable wedge, currently buried.
- **Launch-day HN is high-variance** (2.3% of Show HN reach the front page; ~1.4★ per upvote).
  Not a reliable lever, and it is operator-week-expensive — deferred per G07.

Realistic near-term triers with the current zero-effort posture: **≈0** (0★/0 forks, 3 days old, no
install path, no promotion). This is not a pessimism note — it is the reason the EV of *cheap
prerequisite hygiene* dominates the EV of *campaigns* right now.

---

## 2. Ranked max-EV moves

Scoring: **Friction-to-try** = how close to zero-friction the move gets a new user (1 = still
painful, 5 = truly zero-friction). **Operator effort** = XS (<1h) · S (1–3h) · M (half-day) · L
(days) · XL (operator-weeks). EV reasoning is explicit. All Tier-A/B moves are compatible with
bebop being PARKED (they are hygiene/optionality, not a campaign); Tier-C is explicitly deferred.

### Tier A — prerequisites that unblock *any* trial (do these before anything else)

**A1. Ship one working install command + a prebuilt binary via GitHub Releases.**
*Friction-to-try: 5 · Operator effort: S–M (mostly CI).*
The single highest-EV move. Today there is **no** path a stranger can follow (npm pkg absent; npm
build needs Rust; shim needs a cloned tree + cargo). Cut a `v0.5.0` GitHub Release with prebuilt
`bebop` binaries (Linux/macOS, x86_64+arm64) and a `curl -fsSL …/install.sh | sh` that fetches the
right asset to `~/.local/bin` — the exact pattern opencode/goose/aider converged on (§7.1). A
`softprops/action-gh-release` dependabot branch already exists on the remote, so the release
scaffold is partway there. **EV:** converts the biggest single drop-off (install) from ∞ friction to
zero; nothing downstream (demo, stars, awesome-lists) has any value until this exists. **Prereq
gates:** the crypto/doc state must be commit-worthy enough to tag (G08 Phase 1 capture); a green
`cargo build --release` on the two target OSes. **Note:** publishing `bebop-agent` on npm is a
*weaker* alternative (it would still need the Rust toolchain unless you ship the binary in the
tarball) — prefer the static-binary+curl path; keep npm only as a redirecting stub if at all.

**A2. Doc truth-pass on the install/runtime story (the parts that block or mislead triers).**
*Friction-to-try: 4 · Operator effort: XS.*
A subset of G08 Phase 4 scoped to adoption: kill the false "no Rust needed to run" line
(README:51), remove/redirect `npm install -g bebop-agent` (getting-started.md), reconcile
`package.json` 0.5.0 vs crate 0.4.0, and make README quick-start = the A1 one-liner. A trier who
hits a contradictory or 404 install instruction bounces permanently. **EV:** protects the A1
investment; cheapest possible. **Prereq gate:** A1 defines the true command first. (bebop's
pre-commit check F ties test-count claims to live counts — batch these edits into the capture
commit.)

**A3. Push local `main` (+5) and the feature branch (+2) to origin.**
*Friction-to-try: 3 · Operator effort: XS.*
The public repo tip is *stale* vs local by 5 commits on `main` and 2 on `feat/wire-native-core`,
and the crown-jewel crypto is uncommitted (G08 §1c, G09). A visitor sees an older tree than exists;
worse, the differentiating work is invisible and unprotected (bus-factor-1). **EV:** trust/freshness
signal + eliminates the "the good stuff isn't even pushed" failure. **Prereq gate:** this *is* G08
Phase 1 (snapshot → fix C3a/C3b → 3-model gate → push) and G07 decision #4 (the capture session) —
i.e., it is already the mandated first bebop action; adoption just inherits it.

### Tier B — cheap, passive, durable adoption surface (safe to do while PARKED)

**B4. Package the keyless demo as the documented first-run "wow."**
*Friction-to-try: 5 · Operator effort: S.*
The differentiator value is *already* keyless (§3). Make `bebop boot` (guard proves gates go RED,
no key) the explicit first step, add a one-screen "what you just saw with zero setup and zero key"
block, and optionally a `bebop demo` alias that runs boot → recall → node → govern → ledger in
sequence. This is bebop's honest answer to "time-to-wow" and to "does requiring an API key kill
trial" — it *removes* the key from the first-value moment entirely, which no mainstream agent CLI
does (§7.3: pi-coding-agent's no-key onboarding is the closest analogue and it is a noted
differentiator). **EV:** highest wow-per-second of any move; turns the crowded-field disadvantage
(bebop isn't a better model) into an advantage (bebop shows *safety+memory+identity* value a model
CLI can't). **Prereq gate:** A1 (so the user can reach a binary to run it).

**B5. Awesome-list PRs + GitHub topics + MCP registry listing.**
*Friction-to-try: n/a (discovery) · Operator effort: XS each.*
Near-zero-cost durable long-tail: PR into `awesome-ai-agents` / `awesome-mcp-servers` / relevant
Rust lists; set the 10 repo topics GOVERNANCE.md already drafts; publish the MCP server to the
official `registry.modelcontextprotocol.io` + Smithery/PulseMCP (bebop ships an MCP stdio server and
`docs/mcp-tools.json`). **EV:** small but permanent backlinks; the MCP angle is a *second front door*
(people find MCP servers, then discover the guard-OS). Honest caveat (§7.4): directory-listing →
install causation is unproven and llms.txt is 97%-unused by AI crawlers today — so treat these as
cheap optionality, **not** as traffic drivers. **Prereq gate:** A1+A2 (a listing that leads to a
broken install is negative EV).

**B6. Keep/So-what the demo footage; verify it renders.**
*Friction-to-try: 4 (conversion) · Operator effort: XS.*
`docs/footage/` already holds real PTY captures — a 17.4s `bebop-session.gif` (1.8 MB, already
embedded above the fold) plus per-feature casts/gifs. **These are usable demo assets as-is** — no
re-recording needed for a first launch. GitHub READMEs can't embed the asciinema *player* (no JS),
so the committed GIF is the correct format (§7.5). One cheap upgrade: ensure the hero GIF shows the
*guard going RED* (the unique claim), not only the helm/radio ambiance. The "90-second" rule is
lore, not data; 17s is fine. **EV:** conversion lift is practitioner-lore, not measured — so this is
"don't regress," not a growth lever. **Prereq gate:** none.

### Tier C — campaigns (EXPLICITLY DEFERRED per G07; listed for completeness)

**C7. Show HN / Product Hunt / r/LocalLLaMA + r/rust + r/commandline post.**
*Friction-to-try: n/a · Operator effort: L–XL (prep + live comment defense + follow-through).*
These are the only channels with *measured* traffic (§7.3-README), but they are operator-week-heavy,
one-shot (you launch once), high-variance (2.3% front-page), and — decisively — they **amplify
demand for a product G07 says is parked behind dowiz validation**. Launching bebop now spends the
scarcest resource (operator bandwidth) on the repo G07 ranks below the live-revenue work, *and*
burns the one-time launch card while prerequisites (A1–A4) may still be unmet. **EV now: negative.**
**EV later: positive** — but only once A1–B6 are done AND dowiz P0 has freed bandwidth. Correct
disposition: **parked, re-decide at the G07 review date (2026-07-25).**

### One-glance table

| # | Move | Friction-to-try | Effort | Do while parked? | Gate |
|---|------|:---:|:---:|:---:|---|
| A1 | Working install one-liner + release binary | 5 | S–M | yes | G08 capture; green cross-OS build |
| A2 | Install/runtime doc truth-pass | 4 | XS | yes | A1 |
| A3 | Push main(+5) & branch(+2); capture crypto | 3 | XS | **this is the mandated capture** | G08 P1 / G07 #4 |
| B4 | Keyless demo as first-run wow (`bebop demo`) | 5 | S | yes | A1 |
| B5 | Awesome-lists + topics + MCP registry | — | XS | yes | A1+A2 |
| B6 | Verify/So-what the footage (show guard RED) | 4 | XS | yes | — |
| C7 | Show HN / PH / subreddits | — | L–XL | **NO — deferred** | A1–B6 + dowiz P0 |

---

## 3. Demo-without-key feasibility verdict: **STRONG YES — already ~80% true**

Can bebop show its guard-OS / memory / identity value on a canned session with **no model key**?
Yes — and it largely does today, which makes this the single best zero-friction wow.

Evidence from the code surface (native crate `crates/bebop/src/`, read-only):

- **No API-key or HTTP client on the value-demo path.** grep for `reqwest`/`api_key`/`API_KEY`
  across the host crate finds only test fixtures and a sandbox allow-list — the deterministic
  commands make **no network calls**.
- **The differentiator commands are keyless + deterministic:** `boot` (guard self-cert, denies on
  red — the headline claim, keyless), `recall` (in-process VSA/graph living memory), `node` (PQ
  self-certifying identity + encrypted vault, generated locally), `govern` (PID/ICIR telemetry over
  a supplied stream), `ledger` (tamper-evident audit chain over a real dispatch pipeline, keyless),
  `map`/`diagrams`/`outfit`/`status`/`init`. `manifest.free_llm.fallback` = "native (keyless
  deterministic stub) — never hard-blocks," and `bebop use native` is the documented default.
- The one thing that *does* need a key is **actually editing your code via a real model** — that
  routes to a backend (OpenRouter free tier, or Claude/Codex/etc.). That is fine: the *safety/
  memory/identity* story — bebop's whole reason to exist above a model CLI — is exactly the part
  that is keyless.

**So the "wow without a key" is not a feature to build; it is a flow to package** (move B4). The
gap is presentation, not capability: today a user must already have cloned + cargo-built to reach
it (blocked by A1), and the first-run docs don't foreground the keyless proof. Ship A1+B4 and bebop
has something the crowded field structurally lacks: **a first-value moment that needs no key, no
account, no cloud** — which is also its exact brand ("privacy by construction, no telemetry, no
account"). This is the strongest single adoption asset in the repo.

---

## 4. The AGPL question: evidence + verdict

**Verdict: keep AGPL. Net effect on *individual-developer trial* of a local CLI is neutral-to-
slightly-positive. The real AGPL cost lands later and elsewhere (enterprise embedding/OEM/hosting),
not on day-one trial.** Evidence (§7.2, all cited):

- **What AGPL actually restricts for a locally-run CLI: essentially nothing for the trier.** The
  copyleft trigger is *remote network interaction with a modified copy* — a dev who installs and
  runs bebop on their laptop, unmodified, triggers no obligation. The "AGPL is scary" corpus
  (Google/Meta bans, SCA-scanner flags, legal allergy) is about companies *embedding or hosting*
  code, not individuals *running* a tool. bebop's own posture ("not multi-user, not hosted, one
  local process") is precisely the shape AGPL barely touches.
- **Base rate:** the popular agent CLIs are permissive (aider Apache-2.0, opencode MIT, goose
  Apache-2.0, OpenHands MIT, cline/continue Apache-2.0; Claude Code proprietary). AGPL is *not* the
  default and will draw some reflexive HN/purist skepticism — **especially if paired with a CLA**
  (read as "open-core-washing"). bebop has DCO, not a CLA — good; keep it that way.
- **The one current in-niche precedent cuts bebop's way:** Warp open-sourced its agentic client
  under AGPL-3.0 (April 2026), framed explicitly as a *trust/sovereignty* signal ("inspect exactly
  what data flows to our servers") and hit ~56k★ in weeks with no visible license penalty.
- **Infra precedents show no trial suppression from the license text:** Grafana (Apache→AGPL 2021,
  now 25M+ users), MinIO (26k→56k★ through the change), Plausible/Cal.com/LibreTranslate all use
  AGPL *as* a marketed anti-lock-in / data-sovereignty feature and drew community + contributors.
  The real backlashes (MinIO, ONLYOFFICE) were about *feature-gating and enterprise upsell*, not the
  AGPL text — a trap bebop avoids by being fully open with no crippled community tier.

**Action:** none required beyond consistency — reconcile the LICENSE/Cargo license fields (dowiz-side
C-gate note; bebop is already AGPL throughout), and lean into the sovereignty framing bebop's README
already uses. Do **not** switch to MIT chasing the base rate; for *this* product (safety kernel you
own, no cloud, no telemetry) AGPL is on-message and the trial cost is ~nil.

---

## 5. The delivery-protocol thread: honest verdict

**There is no zero-friction adoption story for the delivery protocol today. There is no adoption
story at all. Park it.** Plainly:

- **It is design-only.** `delivery/` is empty (just a `telegram-pending/` stub). The primitives
  (`matcher.rs`, `pod.rs`, `reputation.rs`, `ledger.rs`) are tested *library* code inside the host
  crate; there is **no network layer, no nodes, no deployment, no users** (G07 scorecard D, G08
  Draft C, G09). A protocol with zero live nodes has nothing to "try."
- **Its own research says its path to value runs through dowiz.** DECOUPLED-MATCHER's cold-start
  play is "be the backend for direct orders — restaurant keeps margin," i.e., it needs a working
  dowiz with real venues to reach node-one value. And the web3-logistics postmortem bebop itself
  commissioned catalogs exactly how this class dies ("value at node one, or die at cold-start").
- **Therefore its EV as an *adoption* target now is ~0**, and pursuing it contradicts both G07
  (parked) and the protocol's own cold-start logic. The correct move is G08's: capture the research
  into memory, relocate the detritus, and let the thread re-enter *only* once dowiz has real order
  flow. No launch, no listing, no demo — nothing.

---

## 6. Sequencing note vs dowiz priority

G07's signed-draft ranking is **P0 dowiz validation > B (sovereign core) > A > D (bebop) > C**, with
bebop **PARKED after a one-session capture**. This adoption analysis **does not challenge that** —
it refines what "parked" should mean for bebop:

- **Parked ≠ frozen-without-state.** The Tier-A/B moves are cheap (XS–M), passive, and mostly
  *inherit* the mandated capture session (A3 = G08 Phase 1 = G07 decision #4). Doing A1–A2 and B4
  *inside or immediately after* that capture converts bebop from "parked and un-trialable" to
  "parked but trial-ready the instant attention returns" — for a few extra hours, not operator-weeks.
  That is pure optionality at near-zero marginal cost, and it is the honest reading of "make people
  able to try it" under a parking order.
- **The line that must not be crossed is Tier C.** A launch campaign is an operator-week spend that
  (a) contradicts G07's ranking, (b) burns the one-time launch card before prerequisites are met,
  and (c) amplifies demand for the parked product over the live-revenue one. **Defer to the
  2026-07-25 review**, and only after dowiz P0 (first real paid order) has freed bandwidth.
- **Suggested disposition for the operator:** fold A1 + A2 + B4 into the bebop capture session as a
  small "make-it-trialable" rider (adds hours, not days); do B5 + B6 opportunistically; hold C7
  behind the review date. If dowiz P0 succeeds first, the G07 machine block flips and bebop's launch
  becomes a deliberate, prerequisite-complete act rather than a distraction.

---

## 7. Sources

**7.1 Agent-CLI trial dynamics** (web pass 1): HN launches — aider https://news.ycombinator.com/item?id=39995725 ·
goose https://news.ycombinator.com/item?id=42879323 · OpenHands "Show HN vs Devin"
https://news.ycombinator.com/item?id=44051241 (SWE-bench Verified framing) · opencode
https://news.ycombinator.com/item?id=44482504 · crush/delight
https://tessl.io/blog/does-developer-delight-matter-in-a-cli-the-case-of-charm-s-crush/ · Claude Code
organic https://dev.to/max_quimby/claude-code-just-hit-1-on-hacker-news-... · opencode star spikes
tied to cost-barrier removal https://medium.com/@milesk_33/opencodes-january-surge-... · GitHub
trending = star-velocity https://ossinsight.io/blog/introducing-trending-page ,
https://github.com/orgs/community/discussions/163970 · install one-liners: opencode
https://opencode.ai/docs/ , goose https://github.com/block/goose/blob/main/download_cli.sh , aider
https://aider.chat/docs/install.html (uv) · curl|sh trust debate
https://news.ycombinator.com/item?id=36069024 . **Fact-checks (verified via raw API):** npm
`bebop-agent` → 404 (does not exist); `github.com/SyniakSviatoslav/bebop` → public, AGPL-3.0,
created 2026-07-08, **0★ / 0 forks / 6 open issues**, pushed 2026-07-11.

**7.2 AGPL trial-rate** (web pass 2): Google AGPL ban https://opensource.google/documentation/reference/using/agpl-policy
(triggers on *network-service* use, not local install) · AGPL misconceptions
https://danb.me/blog/common-agpl-misconceptions/ · Grafana relicense
https://grafana.com/blog/2021/04/20/qa-with-our-ceo-on-relicensing/ · MinIO growth + real
(non-license) backlash https://blog.min.io/five-years-in-the-making/ ,
https://www.blocksandfiles.com/ai-ml/2025/06/19/minio-users-complain-after-admin-ui-removed-... ·
MongoDB left AGPL *because too weak* → SSPL https://en.wikipedia.org/wiki/Server_Side_Public_License ·
Warp AGPL-as-trust-signal https://www.warp.dev/blog/warp-is-now-open-source · Plausible/Cal.com/
LibreTranslate sovereignty framing (plausible.io/blog/open-source-licenses, cal.com/blog/open-source) ·
"non-starter" vs "fine" split https://news.ycombinator.com/item?id=37903520 ,
https://news.ycombinator.com/item?id=41227172 (CLA = "open-source-washing" critique).

**7.3–7.5 README anatomy + launch mechanics + demos** (web pass 3): badge study (CMU STRUDEL, ICSE
2018) https://cmustrudel.github.io/papers/icse18badges.pdf (badges signal quality, don't cause
growth) · README-popularity correlation https://www.sciencedirect.com/science/article/abs/pii/S0164121223002017
· contrarian null result https://arxiv.org/html/2502.18440v1 · GitHub Octoverse 2021 docs=productivity
https://github.blog/2021-11-16-the-2021-state-of-the-octoverse/ · Show HN base rates (2.3%
front-page; ~1.4★/upvote) https://business.daily.dev/resources/hacker-news-marketing-developer-tools-show-hn-launch-day-... ,
10k-post analysis https://antontarasenko.github.io/show-hn/ · Product Hunt dev-tool stats
https://www.shno.co/marketing-statistics/product-hunt-launch-statistics · llms.txt 97%-unused
https://ppc.land/llms-txt-adoption-rises-8-8x-but-97-of-files-get-zero-ai-requests/ · MCP registries
https://registry.modelcontextprotocol.io , https://www.pulsemcp.com/statistics · no-key onboarding
(pi-coding-agent) https://pypi.org/project/pi-coding-agent/ · GitHub READMEs can't embed asciinema
JS → GIF/SVG only; `agg` https://docs.asciinema.org/manual/agg/ . *Flagged as practitioner-lore, not
data:* hero-GIF-above-fold lift, "under 90s" rule, directory-listing→install causation.

**7.6 bebop repo state** (live read-only probe, 2026-07-11): local `main` +5 / `feat/wire-native-core`
+2 vs origin; tags stop at v0.4.0 (TS-era); `package.json` 0.5.0 vs crate 0.4.0; `bin/bebop` = bash
shim → `cargo run`; `dist/` holds only `bebop_core.wasi.wasm` (no CLI binary); native crate has no
HTTP/API-key path on the deterministic commands; `docs/footage/bebop-session.gif` (17.4s, 1.8 MB)
already embedded. **7.7 Blueprints:** G07 (spine/parking), G08 (memory + WIP capture + doc drift),
G09 (crypto assurance) in `/root/dowiz/docs/design/gap-blueprints-2026-07-11/`.

---

*Authored 2026-07-11 by a read-only research session. Only this file was created; both repos left
exactly as found.*
