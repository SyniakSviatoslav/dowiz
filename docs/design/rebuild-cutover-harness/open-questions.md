# Rebuild Cutover Harness — Open Questions

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Everything the operator (and the breaker/counsel round) must decide
> before the cutover harness is built and before ANY surface flips. Each question has options + an architect
> recommendation — a *starting position for friction*, not a decision. 🔴 = red-line (routes prod traffic or
> touches money/auth/RLS); operator sign-off required. Docs only; no code.

Legend: **[MECH]** switch mechanism · **[STATE]** flag/reversibility · **[SEC]** auth/tenancy · **[ORDER]**
cutover sequencing · **[INFRA]** deploy/topology · **[MONEY]** money-irreversibility.

---

### Q1 🔴 [MECH] The switch mechanism — Node front-door reverse-proxy vs edge gateway
Proposal §3/§4 chooses **(A)** the Node front-door reverse-proxy (single ingress, `undici` forward to an
internal Rust app, runtime flag).
- **(a) Node front-door reverse-proxy** — reuses the sole existing ingress + rate-limit + correlation-id +
  real-IP; adds no new SPOF; flip = one SQL. *(recommend)*
- **(b) Edge/gateway router** — a new third component (Caddy/HAProxy/Fly-native) in front of both; cleaner
  long-term separation but a new SPOF/deploy/health/attack-surface before S1, and coarse routing. *(reject —
  over-engineered for the back-of-envelope load)*

**Recommendation:** (a). 🔴 because it defines the production traffic path for the whole rebuild. Owner:
architect + operator.

### Q2 🔴 [STATE] The flag store — Postgres `cutover_flags` + LISTEN/NOTIFY
- **(a) Postgres table + short-TTL cache + `NOTIFY`** — durable, auditable (`updated_by`), shared across Node
  instances, flip/rollback = one statement; state lives where the data lives (the reconciliation point).
  *(recommend)*
- **(b) Env var** — rejected: not runtime (needs redeploy to flip; violates the directive).
- **(c) Redis** — rejected: adds a dependency; the canon keeps idempotency/state in Postgres, not Redis; a
  Redis outage would either fail-open or block the flip.

**Recommendation:** (a). 🔴 because the flag row *is* the production kill-switch. Owner: architect + operator.

### Q3 🔴 [SEC] S2 cutover posture — reconcile "canary" with the atomic-flip mechanism (risk R-1)
The S2 council resolution says: *"the cutover must be a canary flip gated on the family-revocation-rate
matching the Node baseline, not a hard switch."* The harness mechanism is an **atomic per-surface flip** (S3
REV-7, S5 Q6). Reconciliation options:
- **(a) Atomic flip + a live family-revoke-rate auto-rollback trip-wire** (observation window; auto-revert if
  the rate exceeds Node baseline + ε). *Argued safer than a per-request canary:* a per-request split would
  route concurrent refreshes of the **same** family to **different** stacks — the cross-stack
  concurrent-refresh hazard — whereas an atomic flip keeps a family wholly on one stack and the trip-wire
  still catches a revoke-storm early and auto-reverts. *(recommend)*
- **(b) Literal per-request canary** — honors the council's words but re-introduces the concurrent-refresh
  hazard the atomic flip removes, and the mechanism does not do per-request splits (it would be a second
  mechanism just for S2).
- **(c) Keep S2 on Node until Phase D** — flip every OTHER authenticated surface using Node-minted-token
  verification parity (Q4) and never cut the mint/refresh writes over. Safest for the irreversible deletes;
  costs the auth port its cutover.

**Recommendation:** (a) — but this **revises a council decision**, so it is explicitly NOT adopted silently;
the operator must ratify the revision (or pick (b)/(c)). 🔴. Owner: architect + S2 lead + operator.

### Q4 🔴 [SEC/ORDER] Is cross-stack token-verification parity a gate on flipping S3/S4/S5 ahead of S2? (risk R-2)
The ordering (§7) flips S3 (owner CRUD) before S2 (auth mint). That means an owner authenticates on Node
(Node-minted RS256 token) then hits a Rust owner surface that must **verify** that token — even while minting
stays on Node.
- **(a) YES — the S2 body-`kid` round-trip + hash-format parity gates (verification, both directions) are a
  hard precondition for flipping ANY authenticated surface (S3/S4/S5) while mint stays on Node.** *(recommend)*
- **(b) NO — flip S2 (mint) first, before S3/S4/S5**, so there is never a Node-minted token hitting a Rust
  surface. Removes the cross-verification prerequisite but front-loads the riskiest-to-revert surface (S2
  deletes are irreversible) ahead of the safe ones — contradicts safe→risky.

**Recommendation:** (a). The verification path is stateless (shared public key) and far lower-risk than the
mint/delete path; proving it first lets the safe surfaces flip first. 🔴. Owner: architect + S2 lead + operator.

### Q5 🔴 [ORDER] Ratify the cutover ordering
Proposal §7: **S1 → S3 → S4 → S2 → S5 → S6 → S7 → S8 → S9 → S10** (safe→risky), reconciled with the councils.
- **(a)** as above — read-only first (S1 = the mechanism's own proof), then single-writer CRUD/media, then
  auth (verification-parity-gated), then money, then WS/dispatch, then jobs/GDPR/platform-admin. *(recommend)*
- **(b)** S2 before S3/S4 (mint-first; pairs with Q4(b)).
- **(c)** operator-specified alternative.

**Recommendation:** (a). 🔴 because the order determines which irreversible-effect surface is exposed when.
Owner: architect + operator.

### Q6 🔴 [INFRA] Deploy topology + cost + the Astro render sub-target (risk R-8)
- **(a) Second Fly app `dowiz-rust` / `dowiz-rust-staging`, internal-only (no public route), `fra`, small
  pools during dark.** *(recommend)* — independent deploy/rollback/scale/health; ~1 small machine's cost.
- **(b) Co-process in the Node machine** — rejected: couples lifecycles/memory/deploy; no independent rollback.
- **Sub-question (Astro SSR):** is `GET /s/{slug}` SSR served by the same `dowiz-rust` app, or a second
  internal Astro app? *(recommend: decide at the S1 build; the path map is agnostic — one logical upstream.)*

**Recommendation:** (a); Astro sub-target deferred to the S1 build. 🔴 on the cost + the second-app decision.
Owner: architect + operator.

### Q7 🔴 [STATE] Break-glass + who holds flip authority
- **(a) `CUTOVER_FORCE_ALL_NODE=1` front-door env override** (forces every surface to Node regardless of the
  flag table — the defense for a flag-store-read failure) **+ a named operator seat holds the flip token**
  (the `readiness_ok` + sign-off gate). *(recommend)*
- **(b) No break-glass** — rejected: leaves no recovery if the flag read is itself the failure (threat T6).

**Recommendation:** (a). 🔴 — the break-glass is a production kill-switch; its holder is a security decision.
Owner: operator.

### Q8 🔴 [MONEY] Bind the S5 council gates into the harness (risks R-3, R-4, R-7)
Re-affirm, now bound to the switch mechanism: **crypto stays dark through the entire S5 overlap** · **086
lands before the S5 flip** · **request-hash byte-identity golden-vector (both directions) + cross-stack
idempotency probe are hard flip preconditions** · the `discountTotal=0` carry is an explicit owned
accepted-risk · the 085 watermark (2026-07-10) is an operator-owned timing gate on the schedule.
- **(a) Adopt all of the above as machine-checked `readiness_ok` preconditions for the S5 flag.** *(recommend)*
- **(b) Treat any as advisory** — rejected: each is a money-irreversible blast radius.

**Recommendation:** (a). 🔴. Owner: operator + S5 lead + breaker.

### Q9 [INFRA] Overlap time-box + connection-budget ceiling
The two-stack overlap doubles the steady-state pool footprint (proposal §2). The ceiling depends on the
Phase-A Supavisor decision (cache-off vs all-:5432).
- **(a) Set a per-surface overlap time-box** (operator value, e.g. 3–7 days) **and cap Rust dark-phase client
  pools ≤ the Node draw they will shed**, so the sum never grows past the pooler ceiling. *(recommend)*
- **(b) Open-ended overlap** — rejected (S5 Q6c: doubles the pool draw indefinitely on one Supavisor).

**Recommendation:** (a), gated on the Phase-A Supavisor answer. Owner: operator + architect.

### Q10 [SEC] Shadow-diff of live traffic — scope + privacy
G8 mirrors a sample of live GETs to both stacks and diffs bytes (read-parity proof before a flip).
- **(a) Shadow-mirror ONLY read-only, unauthenticated S1 GETs** (menu/info/theme/sitemap) — no writes, no PII
  mutation, no auth replay. Authenticated/write surfaces are proven by the E2E slice + golden vectors, NOT by
  mirroring live authenticated traffic. *(recommend)*
- **(b) Shadow-mirror authenticated traffic too** — rejected: replaying a live bearer token to a second stack
  is an auth-surface + PII-egress risk with no gain the E2E slice doesn't already give.

**Recommendation:** (a). Owner: architect + counsel (PII).

---

## Decision-ordering note
**Q1, Q2** (mechanism + flag store) are **foundation** — nothing builds until they settle. **Q3, Q4, Q5**
(S2 posture, verification-parity gate, ordering) are **coupled** — decide them together; they determine the
whole sequence and whether S2's revision is accepted. **Q8** is **flip-blocking for S5 only** (the code can be
built + dark-verified first). **Q6, Q7, Q9, Q10** are operability/topology and can settle in parallel with the
S1 build.

**The single most likely breaker escalation:** the **cross-stack request-hash drift → duplicate paid order**
(Q8/R-3) — the one money-irreversible failure the atomic-flip posture exists to prevent. **The single most
likely counsel flag:** the **S2 posture revision** (Q3/R-1) — replacing a council-decided canary with an
atomic-flip trip-wire is defensible (and arguably safer) but must be an explicit human ratification, never a
silent override.
