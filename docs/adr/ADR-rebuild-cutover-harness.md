# ADR-0022 — Reversible per-surface cutover harness (Node front-door reverse-proxy + Postgres runtime flag)

- Status: **DRAFT / PROPOSED** (design-time only; no production code in this change). ADR number `0022`
  provisional — confirm against the live sequence before ratification (last taken: ADR-0021 order-channel).
  Full design: `docs/design/rebuild-cutover-harness/{proposal.md,open-questions.md,threat-model.md}`.
- Date: 2026-07-04
- Seat: System Architect (DeliveryOS rebuild program)
- Relates: REBUILD-MAP §3 (Phase B strangler / cutover DoD; Phase D decommission); ADR-0001 (monolith-first);
  ADR-0005 (server-authoritative money); the S2/S3/S5 council resolutions (per-surface cutover posture);
  `06-complete-rebuild-stack.md` (the Playwright E2E net = the parity oracle).

## Context

Ten Rust surfaces (S1..S10) are being built dark. None has cut over, and the switch mechanism itself is
unbuilt and untested. The operator's binding directive: complete the rebuild only after isolated
switch/testing from S1 to S10, so **each phase is reversible and safely switched** to the new stack. Both
stacks share **one** Supabase DB — so the DB is the reconciliation point and there is no cross-stack
consistency problem, only a **write-byte-identity** problem. Three per-surface cutover postures are already
decided and must reconcile into one mechanism: S3 REV-7 (per-surface atomic flip, separate operator go/no-go),
S2 (canary gated on family-revoke-rate for auth), S5 Q6 (order double-charge guarded by shared
`idempotency_keys` iff `request_hash` is byte-identical; 086 before flip; crypto dark).

## Decision (proposed — pending council + operator sign-off)

1. **Node remains the single public ingress.** A front-door `onRequest` hook resolves each request's owning
   surface from a **provably-disjoint `(method, path)` ownership map**, reads that surface's target from an
   in-process cache of a Postgres `cutover_flags` table, and either falls through to the existing Node handler
   (`target=node`, default) or streams the request to an **internal-only** Rust Fly app (`dowiz-rust.flycast`)
   via the runtime's own HTTP client (`undici` — no new dependency class) (`target=rust`).
2. **Runtime flag in Postgres.** `cutover_flags(surface, target, readiness_ok, updated_at, updated_by)` — a
   flip is one `UPDATE` + `NOTIFY cutover_flags_changed`; every Node instance `LISTEN`s and refreshes; a short
   TTL (1–5s) is the backstop. **Rollback is the inverse statement — instant, no redeploy.** The table takes
   RLS `ENABLE + FORCE` with a platform-admin-only policy.
3. **Flip is machine-gated (scaling-gate).** A flip to `rust` is refused unless `readiness_ok=true` (the
   surface's parameterized cutover DoD is recorded green) **and** the Rust upstream `/healthz` is green **and**
   an operator sign-off token is present.
4. **Failure-first degradation.** The front-door health-gates the upstream: a `rust` surface whose upstream is
   unhealthy **degrades to Node automatically** and alerts (circuit breaker); per-surface trip-wires (S2
   family-revoke-rate; S5 duplicate-order-rate) auto-roll-back on divergence from the Node baseline. A
   `CUTOVER_FORCE_ALL_NODE` break-glass env forces all-Node when the flag store itself is impaired.
5. **Deploy topology.** Rust runs as a **second Fly app** (`dowiz-rust` / `dowiz-rust-staging`, `fra`, no
   public route) receiving zero external traffic until a flag flips. Small dark-phase pools ≤ the Node draw
   they later shed, so the two-stack connection budget never grows past the Supavisor ceiling; the overlap is
   time-boxed.
6. **Ordering (safe→risky), council-reconciled:** S1 (read-only — the mechanism's own proof) → S3 → S4 → S2
   (verification-parity-gated; atomic-flip + revoke-rate trip-wire, revising the council's per-request canary
   — operator-ratified) → S5 🔴 (request-hash byte-identity + 086 before flip + crypto dark) → S6 → S7 → S8 →
   S9 → S10.
7. **Migration/flip ordering rule:** no schema migration runs inside a surface's cutover window; the only
   migrations are the operator-gated money drafts (Node-side), each landed before its surface's flip (086
   before S5). Rust's `sqlx::migrate` authors nothing during cutover.

## Alternatives rejected

- **Edge/gateway router in front of both** — a new SPOF/deploy/health/attack-surface before S1, coarse
  routing, an extra hop for all traffic; over-engineered for the back-of-envelope load (~10 orders/min today).
- **DNS/hostname split** — DNS TTL makes rollback non-instant; not per-`(method,path)`; not runtime.
- **Build-time `VITE_*`/client flag** — not runtime; needs a redeploy to flip; cannot route server surfaces.
- **Env-var or Redis flag** — env is not runtime; Redis adds a dependency and the canon keeps state in Postgres.

## Consequences

- Node cannot be decommissioned until every surface flips and the front-door role migrates to Rust (Phase D);
  the shim is the strangler vine, built to be cut.
- The two-stack overlap doubles the steady-state pool footprint (bounded by small dark pools + the time-box +
  the Phase-A Supavisor decision).
- The forward path is a mild SSRF-adjacent surface — mitigated by a hardcoded internal upstream and by applying
  auth + rate-limit at the front-door before forwarding.
- **Routing is reversible; committed side-effects are not** — so every write surface must prove
  write-byte-identity + shared-constraint guards before its flip, and the only irreversible money effect
  (crypto charge) stays dark through the entire overlap.

## Open (require operator 🔴 sign-off — see open-questions.md)

Q1 mechanism · Q2 flag store · Q3 S2 posture revision (canary → atomic + trip-wire) · Q4 verification-parity
as an S3/S4/S5 flip gate · Q5 ordering · Q6 second-app topology + Astro sub-target · Q7 break-glass + flip
authority · Q8 S5 money gates bound into the harness · Q9 overlap time-box + connection budget · Q10
shadow-diff scope.

## Council

Seats to convene: architect (author), **breaker**, **counsel**, operator (human-gated 🔴). This ADR ratifies a
design direction; it does not authorize code. `BUILT` is a separate, human-gated act.
