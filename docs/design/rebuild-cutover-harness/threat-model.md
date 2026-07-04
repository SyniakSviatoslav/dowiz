# Rebuild Cutover Harness — Threat Model

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Adversarial input to the cutover-harness council. The failure modes a
> reversible per-surface switch introduces that no single-stack surface faced — where the *novel* risk is
> **two stacks writing to one DB** and **routing that must be instantly reversible under load**. Read
> alongside `proposal.md`. Docs only; no code.

- **Method:** STRIDE-lite over the switch mechanism + fold-in of the per-surface postures (S2 irreversible
  deletes, S3 two-writer, S5 cross-stack double-order). The DB is the single reconciliation point, so the
  novel class is **write-byte-divergence** and **routing-reversibility**, not distributed consistency.
- **Prime insight (state it once):** **routing is reversible; committed side-effects are not.** The flag makes
  *traffic* revert in seconds. It does **not** un-write a row, un-charge a card, or un-revoke a token family.
  So the safety of every flip is entirely a function of whether the two stacks write **the same bytes** to the
  **same shared DB** under the **same constraints** — before the flip is ever allowed.

---

## 1. Assets

| ID | Asset | Where | Why it matters |
|---|---|---|---|
| A1 | The **routing decision** (`cutover_flags.target` per surface) | Postgres `cutover_flags` (operator-only, FORCE RLS) | The production kill-switch; a wrong/stale value routes a surface to the wrong stack |
| A2 | The **single public ingress** (Node front-door) | `dowiz.fly.dev` | Every request enters here; its availability is total-system availability (already true today) |
| A3 | The **shared DB** (86 tables) | one Supabase Postgres | The reconciliation point — both stacks read/write it; the *only* thing keeping them consistent |
| A4 | **Write-byte-identity** (money math, `request_hash`, state folds, refresh SQL) | in both codebases | The invariant that makes a mid-flip write safe; a drift is a divergent row on shared data |
| A5 | **Session/token continuity** (RS256 keys, `auth_refresh_tokens`, body-`kid`) | shared keys + shared table | A token minted on one stack must be honored on the other across the flip instant |
| A6 | The **shared uniqueness/idempotency guards** (`idempotency_keys` unique, order UUIDs, atomic refresh UPDATE) | Postgres constraints | The last line against a cross-stack duplicate write |
| A7 | **Irreversible money/auth effects** (crypto charge, refund_due, family DELETE) | orders/payment_events/auth tables | A routing rollback cannot undo these — they must be prevented, not reverted |

## 2. Trust boundaries

- **TB-1 client → front-door.** Unchanged from today; auth + rate-limit + real-IP applied here BEFORE any
  routing decision.
- **TB-2 front-door → Rust upstream (the novel hop).** Node forwards a matched surface's request to an
  internal Rust app over Fly 6PN. Trust is one-directional (Node initiates); the upstream host is hardcoded.
- **TB-3 stack ↔ stack via the shared DB.** The trust each stack places in the other's *writes* is mediated
  **only** by the shared DB invariants (same tables, same constraints, byte-identical writes). A divergent
  impl on either stack breaks it — no code path enforces it; only the DoD gates + shared constraints do.
- **TB-4 operator → flag.** The flip is a privileged write to `cutover_flags`, gated on `readiness_ok` +
  upstream health + a sign-off token; audited via `updated_by`.

---

## 3. Threats, triggers, mitigations (red→green)

| # | Threat | Trigger | Mitigation to prove red→green |
|---|---|---|---|
| **T1** | **Split-brain — both stacks serving one surface at once** | (a) flag-map skew across Node instances mid-flip; (b) an **overlapping/ambiguous** `(method,path)` rule matching two surfaces | A surface is wholly `node` OR wholly `rust` — **never split within a surface** (the flip is per-surface-atomic). `NOTIFY` + short TTL bounds cross-instance skew to seconds. The path map is a **provable partition** (Phase-0 CI gate: every one of 236 routes → exactly one surface; no double-match). For write surfaces a brief skew is still safe because writes are byte-identical + shared-constraint-guarded (T5). Proof: a router-partition unit test (0 overlaps) + a mid-flip skew probe (two instances, one flipped, verify no request hits both) |
| **T2** | **Data divergence on a mid-flight flip** | The two stacks write **different bytes** for the same logical write (money drift, request-hash drift, state-fold drift) | Impossible at the row level (one DB); the only vector is byte-divergence, blocked by the **flip preconditions**: money byte-parity vectors, request-hash golden-vectors (both directions), state-fold parity, identical refresh SQL. Shared unique constraints catch a duplicate. Proof: the per-surface golden-vector suite green before `readiness_ok=true` |
| **T3** | **Rollback under load** | Flip OFF while N requests are mid-execution on Rust | A routing change affects **only NEW requests**; in-flight requests finish on whichever stack they started (no request is torn mid-flight — Rust is not shut down, only de-routed). The only residual: a client *retry* of an in-flight write landing on the other stack → the idempotency guard (T5) absorbs it. Proof: the G6 rollback drill under synthetic load — flip ON→OFF at k req/s, assert zero 5xx and zero duplicate writes |
| **T4** | **Money-surface irreversibility (S5)** | A charge/refund/duplicate order committed during the overlap; a routing rollback cannot undo it | **Crypto stays dark through the entire overlap** (no live charge at create; cash charges at delivery, not create) → the only irreversible money effect is feature-flagged OFF. `refund_due` (086) is a shared, non-throwing floor landed before the flip. Money byte-parity + request-hash gates prevent a wrong/duplicate charge from being *written* in the first place. Proof: cross-stack idempotency probe (create on X, retry on Y → one order) + "no `metadata.channel`/no client money field feeds `total`" guardrails |
| **T5** | **Cross-stack duplicate/divergent write** | A retry hits the other stack; or a fold/hash differs across stacks | Shared `idempotency_keys (key, location_id)` unique + byte-identical `request_hash` (the guard is effective **iff** the hash matches); order UUIDs can't collide; the refresh atomic UPDATE picks one winner (409 the other) **iff** both stacks run identical SQL incl. the `interval '5 seconds'` window. Proof: replay→one order, reused-key+mutated-cart→422, cross-stack race→one 409 |
| **T6** | **Flag-store failure / stale flag** | The `cutover_flags` read fails, or a `NOTIFY` is missed | Fail-safe to the last-known-good cached map; no cache + DB unreachable → default **all surfaces to Node** (the incumbent); TTL backstop bounds staleness; **`CUTOVER_FORCE_ALL_NODE` break-glass** forces all-Node when the flag path itself is the failure. A stale flag can only route to a stack that already passed its DoD, so the worst case is "rollback delayed by ≤ TTL", covered by the break-glass. Proof: kill the flag-read path → assert all-Node + alert |
| **T7** | **Front-door as SPOF / SSRF-adjacent forward** | The forward path is new code that connects to an internal host | Node is **already** the sole ingress — the shim adds no new SPOF, only forward code. The upstream host is **hardcoded** (`dowiz-rust.flycast`), never client-derived; only mapped surface paths forward; auth + rate-limit apply at the front-door BEFORE forwarding (a Rust surface can't bypass the incumbent throttles). A slow upstream is bounded by a front-door timeout + streaming (not buffering) + a bounded undici pool → no event-loop exhaustion. Proof: an SSRF attempt (arbitrary upstream via header) is inert; a slow-upstream chaos test does not wedge Node |
| **T8** | **Header / correlation / real-IP loss across the hop** | The forward drops `Authorization`, correlation-id, or `X-Forwarded-For` | Explicit header-forwarding contract: carry `Authorization` (zero cookies), propagate the **server-authoritative** correlation-id (never trust an inbound one), set `X-Forwarded-For` = the real `clientIp` (the #9 fix) so S5 velocity throttles key on the customer not the Fly socket; apply rate-limit ONCE (at the front-door, not re-applied on Rust). Proof: a request traced end-to-end shows one correlation-id across both stacks; a velocity-throttle E2E keys on the real IP through the proxy |
| **T9** | **Two migrators racing one DB** | Node's `node-pg-migrate` (`pgmigrations`) + Rust's `sqlx::migrate` (`_sqlx_migrations`) both run `release_command` on deploy | **Schema is frozen during cutover** (REBUILD-MAP rule); the only migrations in the program are operator-gated money drafts (Node-side), each landed BEFORE its surface's flip (086 before S5). Rust authors nothing during the overlap. The two migration tables coexist by design (REBUILD-MAP §2) and are never both introducing a change in the same window. Proof: a deploy-order assertion that no Rust migration exists during cutover + the 086-before-S5-flip ordering gate |
| **T10** | **Session discontinuity across a flip (S2/S6)** | A token minted on Node is rejected by Rust at the flip instant; or a WS connection is orphaned | RS256 verify is stateless + shared keys; the body-`kid` round-trip gate proven **both directions** before ANY authenticated surface flips (R-2); refresh rows are shared. WS (S6): a flip drops live sockets; clients **auto-reconnect** and the target stack re-authenticates the same token — continuity via reconnect, not connection-migration (an already-upgraded socket stays until it reconnects). Proof: mint-on-Node → verify-on-Rust E2E (and reverse); WS flip → reconnect → same authz E2E |
| **T11** | **Unauthorized / premature flip** | Someone flips a surface that hasn't passed its DoD, or without sign-off | The flip is refused unless `readiness_ok=true` (DoD recorded green) + upstream `/healthz` green + operator sign-off token (the scaling-gate, proposal §4/§9). `cutover_flags` is operator-only (FORCE RLS + platform-admin policy); every flip is audited. Proof: a flip attempt with `readiness_ok=false` is rejected; the audit row records `updated_by` |

---

## 4. What the harness does NOT change (scope guard)

- **The B3 NOBYPASSRLS flip** and the Node→Rust flip are **orthogonal, independently reversible** events
  (S5 threat-model §4). The harness never couples them; a surface's tenancy correctness must hold under either
  pool role. RLS `ENABLE + FORCE` on every tenant table is untouched.
- **Tenant isolation** — each surface family seats its own tenancy GUC on **both** stacks (owner
  `with_user(app.user_id)`; order-write `with_tenant(app.current_tenant=locationId)`). The proxy hop does not
  weaken it; the GUC is seated in-transaction on whichever stack executes the write.
- **Zero cookies / zero PII to AI / zero secrets in git** — the harness moves no data to any AI path, adds no
  secret, and forwards `Authorization` headers, never Set-Cookie.

---

## 5. Residual risks (summary for the human)

- **Cross-stack duplicate paid order (T4/T5, S5)** — the money-irreversible failure the atomic-flip posture
  exists to prevent; bounded by the shared unique constraint **iff** the request-hash is byte-identical.
  **The most likely breaker escalation** — the council should have the breaker attack the request-hash
  canonicalization + a route-by-route flip. Owner: architect + operator + breaker.
- **S2 posture revision (R-1)** — atomic-flip + trip-wire replaces the council's stated per-request canary.
  Argued safer, but a human ratification, not a silent override. **The most likely counsel flag.** Owner:
  architect + S2 lead + operator.
- **The front-door is the sole ingress (T7)** — no new SPOF vs today, but the new forward code is a real
  surface; mitigated by health-gate degrade + break-glass + SSRF guard. Owner: architect.
- **The 085 settlement watermark (2026-07-10)** — a timing landmine independent of the switch; surfaced so the
  cutover schedule cannot silently trip a double-pay. Owner: operator.

**None of A1–A7's failure modes is *introduced* by the surfaces themselves** — the harness's *new* risk is
entirely the **cutover concurrency** (two stacks writing one DB) and the **routing reversibility under load**,
neither of which any single-stack surface faced. Both are contained by the same discipline: **prove
write-byte-identity + shared-constraint guards before the flip, keep the only irreversible money effect
(crypto) dark through the overlap, and make rollback a one-statement, health-gated, break-glass-backed act.**
