# S2-AUTH Cutover Amendment — BREAKER FINDINGS (re-attack)

> **Seat:** system-breaker (S2-auth), the seat that authored **gate-iv**. **Charge:** re-run the
> breaker role against `amendment-cutover-reratify.md` — prove where the amendment breaks, `file:line`,
> no fixes. The amendment's own sharpest question is routed to me ("does family-sticky close the split,
> is bare atomic weaker") because it turns on gate-iv.
>
> **Verdict up front:** the amendment's **core claim is REFUTED** — a family-STICKY canary
> (`consistent-hash(family_id) → stack`, Option C1) is **not implementable as specified**, because the
> routing key `family_id` is **not present anywhere the front-door can read it** (owner refresh token is
> opaque random bytes; courier refresh key rotates per-token). Every implementable proxy key is
> per-*token*, not per-*family*, so the concurrent-refresh split C1 claims to eliminate **survives across
> rotation**. Separately, C1's own widen operation rides the **same `UPDATE cutover_flags + NOTIFY`
> pooler window** the amendment condemns bare-atomic for. What actually carries safety on this
> irreversible surface is **gate-iv (shared-DB atomic UPDATE + SQL `now()`)** — which both postures
> require and neither posture strengthens. **1 CRIT · 4 HIGH · 3 MED · 1 LOW.**
>
> Kept honest: the amendment is **correct** that bare-atomic-without-quiesce is *not strictly safer* than
> the signed canary (its §3a trip-wire-is-post-commit point is CONFIRMED, MED-2), and Option **C2
> (atomic + refresh-write quiesce) genuinely closes the window** where C1 does not.

Legend: **[BREAK]** the exact failure · **[EVID]** file:line · **[SCEN]** repro/number · **[INV]** invariant broken.

---

## CRITICAL

### AC1 · B-CONSIST / gate-iv / AQ1-C1 · `consistent-hash(family_id)` is UNCOMPUTABLE at the gate — the sticky key does not exist in any request the front-door sees; C1 is not implementable, and every implementable proxy key is per-token → the split C1 claims to close persists across rotation

- **[BREAK]** C1 (`amendment:62`, `amendment:122-126`) routes the canary by `consistent-hash(family_id) → stack` so "every refresh of one family pins to one arm." But **`family_id` is never in a token or a request.** It is a column of `auth_refresh_tokens` / `courier_sessions`, populated at issue time by `crypto.randomUUID()` — it is **not** a JWT claim and **not** in the refresh token:
  - **JWT claims carry no family.** `AuthBase = { sub, iat, exp, kid }`; owner adds `userId, activeLocationId`; courier adds `activeLocationId, jti`; customer adds `orderId, locationId` (`packages/shared-types/src/legacy.ts:162-172`). `refreshedOwnerClaims` returns `{role, userId, activeLocationId}` (`apps/api/src/routes/auth.ts:17-26`). **No `family_id` anywhere.** (grep of `legacy.ts`+`jwt.ts` for `family` → 0 hits.)
  - **Owner refresh request is opaque.** `POST /auth/refresh` body = `{ refresh_token: string, active_location_id? }` (`auth.ts:239`), **no bearer preHandler** (`auth.ts:234-238`, config = rateLimit + schema only). `refresh_token` is a raw `crypto.randomBytes(32).toString('hex')`; the backend does `token_hash = sha256(refresh_token)` then `SELECT * FROM auth_refresh_tokens WHERE token_hash=$1` (`auth.ts:243-247`). **The only way to learn `family_id` is to run that exact DB read.**
  - **Courier refresh key rotates per-token.** `refresh_token = ${sessionId}.${tokenPlain}` (`courier/auth.ts:142`); the front-door can parse `sessionId` (`courier/auth.ts:391-398`) but `family_id` still needs `SELECT * FROM courier_sessions WHERE id=$1` (`courier/auth.ts:401`), and **every rotation mints a NEW `sessionId`** (`courier/auth.ts:453`) while keeping `family_id` (`courier/auth.ts:449-451`).
- **[SCEN]** The front-door has exactly two implementable routing keys, and **both defeat family-stickiness**:
  - **Route owner by `sha256(refresh_token)`** (computable without a DB read): two refresh tokens of ONE family are **independent random values** → independent hashes → **different stacks.** Concurrent case that trips reuse-detection: tab A rotates `tok1→tok2` on stack X (`family` kept); tab B still holds `tok1`. `tok1`↦X but `tok2`↦(possibly) Y. Any subsequent refresh of the family lands wherever *its current random token* hashes — **not pinned.** The cross-stack same-family split C1 exists to remove is **reopened on every rotation.**
  - **Route courier by `sessionId`:** `sessionId` rotates each refresh (`courier/auth.ts:453`) → refresh N on stack X mints `sessionId_{N+1}` which hashes to stack Y → refresh N+1 lands on Y. **Not family-sticky.**
- **[BREAK, second head]** To route by *anything* family-stable, the front-door must replicate `sha256(refresh_token)` + the `auth_refresh_tokens` lookup — i.e. **re-implement gate-(ii) hash-format parity inside the router** (`resolution.md:51-52`, "sha256 of hex/base64url string, not raw bytes"). A router↔backend hash-format disagreement mis-routes **every** refresh. C1 therefore **relocates the gate-ii hash-parity hazard into a new surface (the router)** rather than removing a hazard.
- **[INV]** Breaks the amendment's own load-bearing premise (`amendment:60`, "eliminate the split, because the split is the only path to the irreversible revoke") and gate-iv's routing assumption. **The signed decision's letter ("canary gated on family-revocation-rate", `resolution.md:39-40`) is satisfiable; C1's specific mechanism is not.**
- **[MAPS]** AQ1 / Option C1 (`amendment:62,98-101,122-126`). **This is the finding the S2-breaker was asked to rule on — it refutes the core claim.**

---

## HIGH

### AH1 · B-FAIL / AQ1-C1 · C1's canary-WIDEN is `UPDATE cutover_flags + NOTIFY` — it rides the SAME 1–5s pooler split-brain the amendment condemns bare-atomic for; sticky canary does not escape the pooler window, it amortizes it over MORE flips

- **[BREAK]** The amendment's case against bare-atomic (`amendment:46-47`, §3a) is: the flip is `UPDATE cutover_flags + NOTIFY`, LISTEN/NOTIFY is blocked on the transaction pooler, so convergence degrades to a **1–5s split-brain per flip.** This is real: `server.ts:220-221` and `server.ts:240-241` document "Operational pool may use transaction pooler which doesn't support LISTEN/NOTIFY" / "Transaction pooler (port 6543) blocks LISTEN/NOTIFY." **But a canary WIDEN (1% → 5% → 50%) is the identical mechanism** — it changes the same `cutover_flags` row and propagates by the same NOTIFY. C1's central operation ("widen only when it matches baseline", `resolution.md:39-40`) is therefore **not atomic either.**
- **[SCEN]** At each widen step, families in the newly-included hash band move Node→Rust. During the 1–5s convergence, instance A (old width) routes family F's tab-B refresh to Node while instance B (new width) routes family F's tab-A refresh to Rust → **cross-stack same-family split — the exact §3a hazard, reintroduced at every widen step.** A canary that widens through, say, 6 steps (1→2→5→10→25→50%) crosses the pooler window **6 times** vs bare-atomic's **1**. C1 does not eliminate the pooler split; it **multiplies the number of pooler-window crossings** while shrinking the population per crossing.
- **[INV]** Contradicts the amendment's split-geometry table (`amendment:51-54`), which credits the canary with a "Bounded/Continuous" *time* profile but silently assumes widening is free of the very non-atomicity it charged against atomic. The `cutover_flags` mechanism is **not yet built** (grep: no `cutover_flags` table in `apps/`/`packages/`) — so this is catchable at design time.
- **[MAPS]** AQ1; harness HIGH-1 / REV-C3 (`docs/design/rebuild-cutover-harness/resolution.md:26`).

### AH2 · B-SEC / AQ3 / §4 · The S2↔S7 "cross-surface invariant" is documentary, not mechanical — `courier/auth.ts` flips on an INDEPENDENT per-surface flag, so a courier family splits across stacks during S7's own pooler window regardless of S2's posture; and C1 is equally unimplementable for courier

- **[BREAK]** §4 (`amendment:71-78`) correctly finds `courier/auth.ts` mints via the **identical** `signAuthToken` (`courier/auth.ts:7,131,330,460`), runs the same family model (`courier_sessions` keyed on `family_id`, `courier/auth.ts:124,324,449`), and reuse-revokes the family (`UPDATE courier_sessions SET revoked_at=now() WHERE family_id=$1`, `courier/auth.ts:420`). It proposes to "elevate C1 to a cross-surface auth invariant binding S2 **and** S7 … not by re-homing routes" (`amendment:76,108-111`). **But the enforcement surface is the per-surface cutover flag, and S2 and S7 hold *separate* flags.** Nothing in the flag machinery couples them; the amendment itself rejects re-homing the routes ("would fight the template matcher", `amendment:76`). So the invariant is a **line in two markdown records** (`amendment:78`), not an interlock.
- **[SCEN]** S2 adopts posture P; S7 flips `courier/auth.ts` on its own go/no-go (a different date, a different operator action). During S7's flip, the S7 pooler window (`server.ts:240-241`) gives instance A `courier/auth`→Node, instance B `courier/auth`→Rust for **1–5s**. A courier mid-shift with two tabs: tab A rotates `sessionId_1→sessionId_2` on Rust-S7; tab B replays `sessionId_1` on Node-S7. If the two stacks' refresh SQL is not byte-identical (gate-iv on the S7 path — which the S2 record **never scoped**, `amendment:71`), tab B trips `UPDATE courier_sessions SET revoked_at=now() WHERE family_id` → **the working courier is family-revoked mid-shift** (soft-revoke, `courier/auth.ts:420` — recoverable only by re-login; harm to the least-powerful actor, `amendment:74`). S2's posture is irrelevant to this: it is an S7-flag event.
- **[BREAK, second head]** C1 is **unimplementable for courier** for the same AC1 reason — the courier front-door sees only `sessionId`, which rotates per-refresh (`courier/auth.ts:453`), not `family_id`. So even if S7 "inherits C1 by reference," the inherited mechanism cannot be built on the courier path.
- **[INV]** The C1 body-kid parity invariant is surface-independent (`amendment:73`), but the *cutover-posture* invariant is not enforceable across two independent flags. A future reader flipping S7 auth "thinking S2's gate did not reach it" (`amendment:78`) is prevented only by documentation, which the amendment admits is the weakest control.
- **[MAPS]** AQ3 (`amendment:108-111`); route-surface-map (`docs/design/rebuild-cutover-harness/route-surface-map.generated.md:199-204`).

### AH3 · B-COMPAT / AQ2 / §5 · Verification-parity has a genuine ONE-directional round-trip (body-kid AND leeway), so a token minted pre-flip fails post-flip — but the amendment leaves the gate un-mechanized (documentary ordering, no flag interlock)

- **[BREAK]** §5 (`amendment:84-90`) requires a both-directions body-`kid` RS256 round-trip as a "hard ordering gate," but the underlying asymmetry is real and one-directional in **two** independent ways:
  1. **body-kid (structural).** Node writes `kid` into the body AND header (`jwt.ts:50-56`), selects the verify key by **header** kid (`jwt.ts:87,94`), then `AuthToken.parse` **requires body kid** (`jwt.ts:114` + `legacy.ts:162`). An idiomatic Rust mint (`kid` in `Header.kid` only) → **verifies on Rust, 401 on Node** (missing required body claim). One-directional: Node→Rust may pass while Rust→Node fails.
  2. **leeway (temporal).** jose `jwtVerify` passes `algorithms:['RS256']` and **no `clockTolerance`** → leeway 0 (`jwt.ts:105-107`); idiomatic `jsonwebtoken::Validation::default()` = **leeway 60s**. A token in `[exp, exp+60s)` **verifies on Rust, 401 on Node.**
- **[SCEN]** Surface S3 flips to Rust; a customer/owner token minted **pre-flip** on Node sits at `exp+30s`. Rust accepts it (leeway 60) → request succeeds → user believes they are authed. A sibling request routes to a still-Node surface (S4/S5) or rolls back → Node rejects (past exp, leeway 0) → **401 mid-session** on the same credential. Symmetric structural version: a Rust-minted (header-only-kid) token 401s the instant any request touches a Node surface. **A token that round-trips in one direction but not the other lets a surface flip while a pre-flip token becomes unverifiable post-flip** — precisely the class §5 exists to gate.
- **[BREAK, gate is soft]** §5 asserts this is a "hard gate on the whole authed-surface sequence" (`amendment:88`) owned by "architect + S2 lead + operator" (`amendment:90`), but names **no mechanical interlock** that blocks a `cutover_flags` write when the round-trip vector is red. It is an ordering *discipline*, not an enforced gate — the same documentary-only weakness as AH2.
- **[INV]** Q10(a) rollback premise ("Node-minted rotates on Rust, Rust-minted verifies on Node, rollback leaves every live session valid", `resolution.md:104-105`); breaker C1 (`breaker-findings.md:21-48`) and M1 (`breaker-findings.md:204-213`).
- **[MAPS]** AQ2 (`amendment:103-106`); gate-(i) (`resolution.md:50-52`).

### AH4 · B-CONSIST / gate-iv / §3b · The safety on the irreversible surface is carried by gate-iv (shared-DB atomic UPDATE + SQL `now()`), NOT by stickiness — so "family-sticky closes the split" is false at the mechanism level: the DB, not the router, serializes concurrent refreshes

- **[BREAK]** gate-iv's own text (`resolution.md:54-56`) states cross-stack concurrent refresh is "**safe via the shared-DB atomic UPDATE iff both stacks use identical refresh SQL.**" The serialization point is the DB row, not the routing arm: `UPDATE auth_refresh_tokens SET used=true WHERE id=$1 AND used=false RETURNING id` (`auth.ts:266`) is atomic in the **shared** DB regardless of which stack issues it; the benign-window check `created_at > now() - interval '5 seconds'` uses **SQL `now()`** (`auth.ts:277`), one DB clock for both stacks. **Two concurrent refreshes of one family hitting *different* stacks still serialize on the shared DB** — one wins `rowCount 1`, the other gets `rowCount 0` → 409 benign (`auth.ts` 409 branch) or, only if the stale token is replayed **>5s** after rotation, the family DELETE (`auth.ts:284`).
- **[SCEN]** Therefore stickiness is **neither necessary nor sufficient** for the split's safety:
  - *When gate-iv holds:* the cross-stack split is **benign** — the DB serializes and the 5s SQL window (`auth.ts:277`) *absorbs* the 1–5s pooler window (AH1). Stickiness adds nothing the shared DB doesn't already provide.
  - *When gate-iv is open* (e.g. H5 "right value, wrong clock" — Rust computes the window in `Utc::now()` instead of SQL `now()`, `breaker-findings.md:160-166`): stickiness pins a family to the buggy stack, so the bug **still deletes families** — it just does so without a *cross-stack* signature. The amendment concedes "no cutover posture is safe if gate-iv is open" (`amendment:65`).
- **[INV]** Refutes §3b's tie-breaker logic (`amendment:58-63`, "eliminate the split, because the split is the only path to the irreversible revoke"): the path to the irreversible revoke is a **mis-encoded reuse-detection within one stack**, gated by gate-iv, **not** the cross-stack routing geometry. Removing the split (C1/C2) is a defense-in-depth nicety on top of gate-iv, not the load-bearing control the amendment frames it as. Bare-atomic + gate-iv and C1 + gate-iv are far closer in real risk than the split-geometry table (`amendment:51-54`) implies.
- **[MAPS]** AQ1 §3b; H5 (`breaker-findings.md:152-171`).

---

## MED

### AM1 · B-CONSIST / §3b / gate-iv · Family-stickiness MASKS the wrong-clock variant of gate-iv (H5) — the app-clock-vs-SQL-`now()` skew only manifests when a family crosses stacks, which under C1 happens only at widen boundaries → the canary is LESS able to detect exactly the bug gate-iv exists to catch

- **[BREAK]** H5 (`breaker-findings.md:160-166`) warns the 5s window must be computed in SQL `now()`, never Rust app-clock. Under a *per-request* canary, families cross Node↔Rust continuously, so any Node(DB-clock)↔Rust(app-clock) disagreement on the 5s boundary surfaces immediately and often. Under **C1 family-stickiness**, a family stays on ONE stack (ONE clock) in steady state; the two clocks are only ever compared for the **same** family at a **widen boundary** (AH1) — a rare event. So the wrong-clock bug can sit undetected on the Rust arm through the entire canary and only misfire during a widen, when a family straddles Node(SQL `now()`) and Rust(app `now()`) and the 5s window is evaluated against two clocks.
- **[SCEN]** Rust ships with `Utc::now() - Duration::seconds(5)`. Canary runs 1%→50% over a week, healthy (each pinned family uses only Rust's clock, internally self-consistent → no false family-delete). At the 50%→100% widen, families straddle → 200-500ms round-trip + 1-3s NTP drift flips a benign 2s two-tab refresh to "replay" → **family DELETE** (`auth.ts:284`) at the widest, most-populous step — the worst possible moment. Stickiness converted a continuously-detectable bug into a widen-only latent one.
- **[INV]** Undercuts the canary's stated virtue ("detect-before-commit", `amendment:53`) for the specific failure gate-iv is authored against.
- **[MAPS]** §3b; H5.

### AM2 · B-CONSIST / §3a-table · The split-geometry table's "Detection vs commit: Before (front-loaded)" cell is misleading — a family-DELETE on a canary family IS an irreversible commit; BOTH postures commit irreversible deletes before the rate-detector fires

- **[BREAK]** The table (`amendment:51-54`) credits the per-request canary with "**Before** commit (front-loaded)" detection vs atomic's "**After** commit (trip-wire reactive)." But the canary's detector is *also* the family-revocation-**rate** (`amendment:53`, `resolution.md:39`) — a rate crosses threshold only *after* N families are already DELETE-d (`auth.ts:284`, irreversible). The canary limits the **population** of pre-detection deletes to the canary %, but it does **not** detect *before* the first irreversible commit — the canary families are the sacrificial detection cost.
- **[SCEN]** 1% canary of, say, 5,000 active vendor families = 50 families on the canary arm. A parity bug deleting families at even 2%/hr trips the rate metric only after ~1 family is irreversibly deleted; "front-loaded" detection still costs real, un-un-deletable evictions. The honest table cell is "detection bounds the population, not the pre-commit boundary" — which weakens (does not eliminate) the canary's edge over atomic.
- **[INV]** `resolution.md:38-39` (family-DELETE not rollback-recoverable) applies to canary families too.
- **[MAPS]** §3a.

### AM3 · B-FAIL / §3a · The amendment OVERSTATES bare-atomic's pooler hazard within the 5s window — under gate-iv, the 1–5s convergence window is largely absorbed by the SQL 5s benign window, so most pooler-window concurrent refreshes 409 (benign), not DELETE

- **[BREAK]** §3a asserts the atomic flip's 1–5s pooler window "manufactures … the same cross-stack same-family split" that deletes families (`amendment:46-47`). But the family-DELETE fires **only** when a stale token is replayed **outside** the `interval '5 seconds'` window (`auth.ts:277,284`). The pooler window is **1–5s** (`server.ts:240-241`); the benign window is **5s**, on the shared DB clock. So a concurrent refresh landing on the "wrong" stack during the pooler window, with typical client-retry latency <5s, hits `rowCount 0` → recent-rotation TRUE → **409 benign**, not DELETE. The DELETE is reachable only where pooler-window + retry latency + skew **exceeds 5s** — a boundary case, not the general case §3a implies.
- **[SCEN]** Two-tab concurrent refresh during a flip's 3s pooler window: tab A rotates on Rust, tab B replays the old token on Node 1.2s later → `created_at > now()-5s` TRUE → 409 → client retries with the stored fresh token → success. No delete. The atomic hazard is real but **thin** — it is the ≤~2s residual past the 5s window, not the whole window.
- **[INV]** This *strengthens* the amendment's ultimate recommendation (require gate-iv; prefer C2-quiesce) but corrects its risk *magnitude* for atomic — the split-geometry table overstates atomic's blast on the irreversible axis.
- **[MAPS]** §3a; harness HIGH-1.

---

## LOW

### AL1 · B-CONSIST / AQ1-C1 · Login/re-login mints a NEW `family_id` (`randomUUID`) that hashes independently → a user's devices/families scatter across arms, and a reuse→revoke→re-login mid-canary lands the fresh family on either arm

- **[BREAK]** Rotation keeps `family_id` (`auth.ts` INSERT reuses `tokenRecord.family_id`; `courier/auth.ts:449-451`), but **login and invite-redeem mint a NEW** `family_id = crypto.randomUUID()` (`courier/auth.ts:119,319`; owner code-exchange `auth.ts` uses `crypto.randomUUID()` for family). A fresh family hashes independently of the user's other families. So "family-sticky" does **not** give a *user* a stable stack — a 2-device user holds 2 families that can pin to different arms, and a family revoked by reuse-detection then re-created by re-login can flip arms mid-canary.
- **[SCEN]** Not a same-family split (each family is internally consistent), so LOW — but it undermines the intuition (`amendment:60`) that stickiness gives coherent per-user behavior, and it means a support-mediated re-auth after a wrongful revoke (`amendment:115`) may re-home the user onto the very arm that revoked them.
- **[INV]** None broken; a scoping caveat on C1's "every refresh of one family pins to one arm" (`amendment:62`) — true per family, false per user/device.
- **[MAPS]** AQ1.

---

## Vectors probed and NOT broken (kept honest — the amendment is right about these)

- **"Bare-atomic-without-quiesce is not strictly safer than the signed canary" (§3a).** CONFIRMED. The trip-wire is a *rate* detector on an irreversible DELETE (`auth.ts:284`); it fires post-commit. The amendment's core criticism of bare-atomic stands (modulo the magnitude correction in AM3).
- **Option C2 (atomic + refresh-write quiesce) genuinely closes the window (§3b).** CONFIRMED sound. Quiescing the refresh write path for the ~1–5s convergence (mirroring harness REV-C3, `docs/design/rebuild-cutover-harness/resolution.md:26`) converts the bounded-window split to zero split — this is the one posture that is *mechanically*, not rhetorically, safer. Unlike C1, C2 needs no family key at the gate.
- **gate-iv as a hard prerequisite in all postures (`amendment:65`).** CONFIRMED and correctly the load-bearing control (AH4 makes it *more* load-bearing than the amendment frames it, not less).
- **§4 courier-JWT-outside-S2 is a genuine scope gap.** CONFIRMED — `courier/auth.ts` really does mint S2-shaped tokens via the identical signer (`courier/auth.ts:7`) with the same family-revoke (`courier/auth.ts:420`). The *finding* is real; only the *remedy's enforceability* breaks (AH2).
- **body-kid + leeff parity must be a gate (§5).** CONFIRMED as a real hazard (AH3); the break is that it is left un-mechanized, not that it is wrong.

---

## SHARPEST — the ruling the S2-breaker owes the record

**AC1 refutes C1's mechanism.** `consistent-hash(family_id) → stack` cannot be built: the front-door never sees `family_id` (owner refresh token is opaque random bytes, `auth.ts:243-247`; courier key is a per-rotation `sessionId`, `courier/auth.ts:453`; no JWT carries a family claim, `legacy.ts:162-172`). The only implementable proxy keys are per-token, so the concurrent-refresh split C1 promises to close **survives across rotation** (AC1), and C1's widen rides the same pooler split-brain the amendment condemns in atomic (AH1). What actually makes the split safe — or fatal — is **gate-iv** (shared-DB atomic UPDATE + SQL `now()`, `auth.ts:266,277`), which serializes concurrent refreshes on the DB regardless of routing arm, and which **both** postures require (AH4). Stickiness is not the load-bearing control the amendment makes it.

**Verdict on the core claim (`family-sticky-is-safer-than-bare-atomic`): REFUTED as stated.** C1 is not implementable as specified (AC1) and does not escape the pooler window (AH1); the amendment's *broader instinct* — never ship bare-atomic-without-quiesce, always require gate-iv — is right, but it is **Option C2 (atomic + quiesce), not C1 (family-sticky), that carries it.** If the seats want to keep a canary, the correct amendment is to route it by a **gate-computable, family-stable** key the front-door can actually read (not `family_id`) — a question the amendment did not resolve and must, before C1 can be ratified.
