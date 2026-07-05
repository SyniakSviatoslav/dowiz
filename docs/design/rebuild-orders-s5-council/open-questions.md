# S5-ORDERS/MONEY Port — Council Packet · OPEN QUESTIONS

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Everything the live Triadic Council (architect / breaker /
> counsel / human) must decide before S5 orders/money is ported. Each question has options + a lane-R3
> recommendation — a *starting position for friction*, not a decision. This is the CROWN-JEWEL red-line
> surface (money + irreversible state + the scariest cutover); the 🔴 density is higher than any prior
> packet by design. Docs only.

Legend: **[MONEY]** money-correctness · **[STATE]** state-machine/lifecycle · **[SEC]**
security/tenancy · **[CONTRACT]** shape/parity · **[SCOPE]** surface placement · **[INFRA]**
cutover/topology · **[MIGRATION]** DB-draft interaction. 🔴 = red-line, operator sign-off required.

---

### Q1 🔴 [MONEY] The order-total composition contract — byte-parity + the `discountTotal` zero
The full formula (proposal §3): `subtotal` (in-tx MVCC snapshot) → `taxTotal` (BigInt half-up, inclusive
extract / exclusive add) → `chargedTax = includesTax ? 0 : taxTotal` (LC1) → `deliveryFee` (free-threshold
/ tier / flat) → `total = subtotal + deliveryFee + chargedTax − discountTotal`, with `discountTotal = 0`
hardcoded. The Rust port must reproduce `applyTax` **bit-for-bit** (`i128` intermediate, never `f64`) and
compose with `chargedTax` (never `taxTotal`). Server stays SoT (ADR-0005); the FE mirror is display-only.
- **(a) [DISCOUNT] CARRY the hardcoded `0` + keep the `− discountTotal` seam + accepted-risk FLAG.**
  Promo/discount redemption does not exist; wiring it is a *feature* with its own council (schema,
  redemption ledger, abuse model). Port the `0`; record the gap + owner. *(recommend)*
- **(b) Wire real redemption now** — rejected: couples a money-math port to a net-new feature +
  schema (DB is frozen) + a promo-abuse threat model, at the worst possible moment.
- **(c) Drop the `− discountTotal` term entirely** — rejected: deletes the composition seam, forcing a
  re-fork when redemption eventually lands (the exact duplication that certified LC1).

**R3 recommendation:** (a). Byte-parity vs the zero-import hand-derived money vectors + the property test
(`inclusive ⇒ total = subtotal + fee`, which needs no oracle). The `discountTotal=0` carry is
"schema-rich, runtime-minimal": the subtraction term is the seam, redemption stays unbuilt. **🔴** because
a one-minor-unit drift or a re-introduced double-add is a live overcharge. Owner: architect + operator.

### Q2 🔴 [STATE] State-machine wiring — every fold intact, and the actor-gate as a separate layer
The 10×10 matrix is ported + verified (`order_status.rs`). S5 wires axum handlers onto `assert_transition`
**plus** the rich `updateOrderStatus` mutator (status-guarded anti-race UPDATE; per-transition `*_at`
stamps; the R2-3 assignment-terminalize fold; the L-A `refund_due` fold with ESC-2; history + ETA
SAVEPOINTs; WS publish) **plus** the actor-gate (`assertOwnerTargetAllowed`) and the CC-1 strand guards.
- **(a) Port `updateOrderStatus` as ONE central mutator with every fold, called only inside a
  tenant-seated tx; wire the actor-gate + CC-1 as distinct authorization layers over the machine.**
  *(recommend)*
- **(b) Re-implement transitions per-route** — rejected: the folds (R2-3, L-A, history, stamps) would
  drift across callers; the whole point of the Node central mutator is single-writer coverage.

**R3 recommendation:** (a). The machine says *possible*; the actor-gate says *allowed* — encode both.
The L-A fold is inert until crypto flips but ports now (backstopped by the DB-level L-C trigger, mig 086).
**🔴** because a dropped fold silently strands a courier binding, loses a refund obligation, or lets an
owner drive a SYSTEM-only edge. Owner: architect + operator.

### Q3 🔴 [SEC] Order-write tenancy — seat `app.current_tenant=locationId`; wire the customer order-scope
Two coupled tenancy facts: (i) **`POST /orders` seats NO GUC today** (raw `db.connect()`+`BEGIN`,
BYPASSRLS-masked) → post-B3 the checkout INSERTs match 0 rows / raise; (ii) the customer order routes bind
`customer_id=sub` only, ignoring the token `orderId` (S2 REV-3/T-12 cross-order read/cancel).
- **(a) FIX-IN-PORT both:** route the create tx through **`with_tenant(app.current_tenant=locationId)`**
  (the service root — correct here, opposite of S3's owner `with_user`), validated against `locations`
  existence; and **wire `CustomerClaimsExt::require_order` (already built in S2) + keep the `customer_id`
  predicate** (belt-and-suspenders). NOBYPASSRLS probe + the token(A)→cancel(B)=403 E2E. *(recommend)*
- **(b) Carry the context-free create verbatim (rely on BYPASSRLS)** — rejected: the never-copy leak
  class; a silent total-checkout outage the instant B3 flips.
- **(c) Seat `app.user_id` on order writes** — rejected: wrong root (there is no owner membership for an
  anonymous/customer create); would match 0 rows under FORCE-RLS.

**R3 recommendation:** (a). Order writes are a **service-root** write (`app.current_tenant`=locationId),
exactly as the customer-cancel already seats (LC3). The S2 REV-3 seam already exists; S5 must not leave it
unwired. **🔴** — the create-side GUC fix is a B3-readiness correctness fix on the money hot path, and the
customer-scope wiring closes a live cross-order authz gap. Owner: S5 lead + operator + B3-council (for the
search_path pin dependency the order policies inherit).

### Q4 [CONTRACT] Idempotency — request-hash byte-fidelity + preflight non-idempotency
Order-create dedup keys on `idempotency_keys (key, location_id)` + a `request_hash` compare; preflight
(soft_confirm/hard_block) is deliberately non-idempotent and ROLLBACKs before the key check.
- **(a) Carry the exact ordering (preflight → idempotency → price → persist), the `.sub` customer-id read
  (#8 fix), and make `request_hash` byte-fidelity a NAMED cutover gate (golden-vector, both directions).**
  *(recommend)*
- **(b) Treat request-hash parity as a build detail** — rejected: it is the sole cross-stack
  double-order guard during the overlap (Q6); a one-byte drift is a silent duplicate-or-false-422.

**R3 recommendation:** (a). Not 🔴 on its own (settled by carry), but the request-hash golden-vector is
**escalated into the Q6 cutover gate set** because its blast radius is a duplicate paid order. Owner: S5 lead.

### Q5 [SCOPE/CONTRACT] Channel attribution + the MessengerKind 3-kind-422
Two channel-adjacent decisions. **(a)** `sales_channel` entity: REBUILD-MAP names it "first-class", but no
`sales_channel` table exists — the shipped path is the `x-channel` header → `orders.metadata.channel`
(write-only). **(b)** `CreateOrderInput.messenger_kind` is a stale 3-value enum
(`telegram/whatsapp/viber`) while the checkout selector offers 6 (`+phone/signal/simplex`) → a live 422.

- **Q5a [SCOPE]:** **(a) CARRY the metadata-jsonb attribution; DEFER the `sales_channel` first-class
  entity to a post-rebuild schema-evolution council** (DB is frozen; the attribution seam already lives in
  `metadata`). *(recommend)* — (b) introduce a `sales_channel` table now = a schema change the freeze
  forbids.
- **Q5b 🔴 [CONTRACT]:** **(a)** CARRY the 3-kind drift (parity-pure — re-ship a live checkout break,
  weakest); **(b)** **FIX-IN-PORT: unify to ONE canonical 6-kind `MessengerKind`** in the order-input
  contract, matching the checkout selector + the DB CHECK, with an E2E delta (a `signal` order 422→201) —
  **first confirming (Phase-0 `ci-schema-drift`) the DB CHECK admits all 6** (else (b) trades a 422 for a
  500). *(recommend (b))*

**R3 recommendation:** Q5a (a); Q5b (b). Q5b is 🔴 because it is a customer-facing order-*correctness* fix
the port naturally closes (the fix-vs-carry rule's "FIX-IN-PORT for 🔴 correctness with a documented E2E
delta") — but gated on the DB-CHECK confirmation. Owner: S5 lead + operator (Q5b).

### Q6 🔴 [INFRA] Cutover concurrency — atomic per-surface flip; both stacks accept orders
During the overlap both Node and Rust accept `POST /orders` (money + irreversible). UUID ids can't collide;
cash doesn't charge at create; crypto is dark. The real hazards: a **double-order** from a cross-stack
retry (guarded by the shared `idempotency_keys` unique **iff** `request_hash` is byte-identical), and
**divergent state folds** across stacks.
- **(a) Atomic per-surface flip (S3 REV-7): the WHOLE S5 route family flips together behind the proxy;
  land migration 086 (stack-agnostic `refund_due` floor) BEFORE the flip; time-box the overlap; rollback =
  proxy flag-flip. Gate on request-hash + money byte-parity + a cross-stack idempotency probe; keep crypto
  dark throughout.** *(recommend)*
- **(b) Strangle route-by-route within S5** (create on Rust, cancel on Node) — rejected: two divergent
  transition-service impls against one order; a Node-created order cancelled by a drifted Rust fold.
- **(c) Indefinite dual-accept** — rejected: doubles the API pool draw on one Postgres/Supavisor
  (connection-budget ceiling, §2); the flip is the *contract* to shed the Node pool.

**R3 recommendation:** (a). Atomic, time-boxed, 086-floored, parity-gated. **🔴** — this is the scariest
cutover in the whole rebuild (orders are money + irreversible); the request-hash byte-fidelity gate (Q4)
and the "never flip crypto during the overlap" rule are non-negotiable. Owner: architect + operator +
breaker (attack the cross-stack retry + the fold-parity).

### Q7 🔴 [MIGRATION] Dependency on money migrations 085/086/087 + the 2026-07-10 watermark
- **085 (settlements-catchup, watermark `2026-07-10 00:00:00+00` HARD gate):** an S7/settlement concern
  the S5 lifecycle never calls — **NOT an S5 build dependency**. BUT the watermark literal is a **timing
  landmine**: erring EARLY (literal before the real apply) DOUBLE-PAYS old rows; erring LATE is safe. If
  settlement apply slips past 2026-07-10, the operator must bump all three literal occurrences before apply.
- **086 (refund_due trigger, M-1):** stack-agnostic structural floor — **land BEFORE the S5 cutover** so
  both stacks share the `refund_due` floor during overlap. Non-throwing by design (do NOT "fix" to
  throwing); inert until crypto flips.
- **087 (reconciler, M-3):** worker-path; land with the S8 jobs slice, not an S5 dependency.
- **(a) S5 code builds independent of all three; land 086 before the flip; 085/087 on their own schedules;
  surface the 085 watermark as an operator timing gate.** *(recommend)*
- **(b) Block S5 on all three landing first** — rejected: over-couples S5 to settlement/jobs; 085/087 are
  not on the order hot path.

**R3 recommendation:** (a). **🔴** on the **085 watermark timing** (a double-pay hazard the operator owns)
and the **086-before-flip** sequencing (shared floor). S5 does not author/apply any migration — it consumes
086's floor and ports the app-side L-A fold. Owner: operator (watermark + 086 sequencing) + S5 lead.

---

## Decision-ordering note for the council
**Q1 (money composition)**, **Q2 (state-machine folds)**, and **Q3 (order-write tenancy + customer-scope)**
are **port-blocking** — no S5 write builds before all three are settled, because they define the three
load-bearing seams (money bytes, state folds, tenancy). Decide them first.

**Q6 (cutover concurrency)** and **Q7 (086-before-flip / 085 watermark)** are **cutover-blocking, not
build-blocking** — the Rust code can be built + dark-verified before they settle, but the **flip** cannot
happen until Q6's parity gates are green and Q7's 086 has landed. **Q4 (idempotency)** is settled by carry,
but its **request-hash byte-fidelity** escalates into Q6's cutover gate set (its blast radius is a duplicate
paid order). **Q5a (sales_channel)** is a scope defer; **Q5b (MessengerKind)** is a 🔴 correctness FIX
gated on a Phase-0 DB-CHECK confirmation — it blocks *that one contract field*, not the surface.

**The single most likely breaker escalation:** the **cross-stack request-hash drift** (Q4→Q6) — a duplicate
paid order is the money-irreversible failure the whole atomic-flip posture exists to prevent. **The single
most likely counsel flag:** the **`discountTotal=0` carry** (Q1a) — re-shipping a known money gap through a
deliberate rewrite is defensible as "unbuilt feature, not a defect" but must be an explicit accepted-risk
with an owner, not a silent omission.
