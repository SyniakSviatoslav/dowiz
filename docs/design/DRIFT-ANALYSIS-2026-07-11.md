# DRIFT ANALYSIS — bebop (PROTOCOL) vs dowiz (PRODUCT) vs LONG-TERM VISION

> READ-ONLY / ZERO CODE CHANGES. Generated 2026-07-11 from:
> - `/root/bebop-repo/docs/design/CONSOLIDATED-AUDIT-EXTRACT-2026-07-11.md` (PART 1 + PART 2 + cross-cutting)
> - `/root/bebop-repo/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`
> - `/root/bebop-repo/docs/design/fable-review-bebop2-1783715896.md`
> - `/root/.claude/projects/-root-dowiz/memory/MEMORY.md`
> - Spot-reads of live source: dowiz `server.ts`, `rebuild/crates/api/src/routes/orders/pg.rs`, RLS config,
>   bebop2 `memory.rs`, `reputation.rs`.
>
> Every claim carries `file:line` or `doc:line` evidence. "cargo/dowiz" claims are agent-narrated unless
> a code line is cited; per the load-bearing lesson (CONSOLIDATED:380-385) trust code lines, not summaries.

---

## 1. PROTOCOL-vs-PRODUCT DIVERGENCE (bebop design intent vs dowiz current code)

### 1.1 BYPASSRLS in prod vs the operator's NOBYPASSRLS gate
- **Intent:** The operator's standing rule is `NOBYPASSRLS` — the red-line gate on sovereign core ownership
  and RLS enforcement. (MEMORY.md:11 "Phase 2.3 (customer ownership): NOBYPASSRLS gate"; MEMORY.md:28
  "wasm32 ... NOBYPASSRLS gate live/proven").
- **Product reality:** Prod still runs the `BYPASSRLS` role → ~103 RLS policies dormant. (CONSOLIDATED P8:212
  "prod runs BYPASSRLS → ~103 RLS policies dormant"). Code still assumes BYPASSRLS: `courier-cron.ts:37`
  ("Identical behavior under today's BYPASSRLS"); `reconciliation.ts:168` ("today's BYPASSRLS");
  `order-timeout-sweep.ts:68-70` ("Identical behavior under today's BYPASSRLS"). The remediation plan itself
  admits the operational pool was never switched: `proposed-migrations.sql:26-28` — role created `NOBYPASSRLS`
  "with a note to switch DATABASE_URL_OPERATIONAL to it. But the hot path WRITES through the operational pool".
- **Drift:** The gate that is the operator's named red-line is documented as *live/proven* (MEMORY.md) but is
  **not actually flipped in prod**. The two repos' "truth" disagree: memory says done, audit + code say not done.

### 1.2 Rust checkout bypasses `kernel::decide` (CREATE door) vs "all business logic through decide"
- **Intent:** "Immutable event-sourced state machine is the law: `Intent → decide → Event`." (BLUEPRINT C3:62).
- **Product reality — nuance confirmed by code:** Status *transitions* DO go through `decide`
  (`pg.rs:491` `decide(&order_state, cmd, &ctx)`, `pg.rs:567`), composing machine+actor-gate+cc1. BUT the
  **CREATE+PRICE door does not**: `create_order` (`pg.rs:91`) computes pricing inline (steps 6-7, money columns
  bound at `pg.rs:405-410`) and never constructs `Command::PlaceOrder` nor calls `decide` for the create. This
  confirms CONSOLIDATED P7:207 ("Command::PlaceOrder is never constructed in the api crate; same math, different
  door; no Priced event"). The CREATE pathway is a parallel door, not the single `decide` law.
- **Corroborating seam gap:** `pg.rs:863-864` hardcodes `let cause_hash = "placeholder";` so the signed-log
  causality seam (D2 dedup/ordering) is not wired into the persistent log. (CONSOLIDATED P5:199; G06 doc:116-119.)

### 1.3 "No central server" / local-first vs relay-assisted P2P honest floor
- **Intent:** "Local-first + no central server as a reachable-free invariant." (BLUEPRINT C4:63).
- **Reality (self-corrected in audit):** The honest floor is **relay-assisted P2P**, not serverless.
  CONSOLIDATED M2:247 "'no central server' = relay-assisted P2P (APNs/FCM, NAT-relay, ≥1 always-on replica)".
  The blueprint's clean "no central server" wording (BLUEPRINT C4) omits the relay caveat; only the audit
  states it. Dowiz prod today is unambiguously serverful (single Node hub LIVE — CONSOLIDATED P9:216).

### 1.4 Self-cert identity vs current Supabase auth
- **Intent:** L1 self-cert identity `id = H(pq_pub ‖ classical_pub)`, NO issuer, NO directory, NO phone-home.
  (BLUEPRINT L1:79-80; DANGER #4:95).
- **Product reality:** Auth is Supabase `verifyAuth` (e.g. `server.ts` imports, `claim.ts:7` "POST /api/claim/accept
  — verifyAuth ONLY", `routes.ts:101-102`). No PQ self-cert identity exists in the product. Per CONSOLIDATED M2:248
  bebop2 crypto is "ONLY as PQ half of hybrid ... never alone" — i.e. even the protocol half isn't wired into
  product identity yet. (MEMORY M5:282 "dowiz only relays" — single-owner, not PQ-identified.)

### 1.5 Threshold (k-of-n) settlement vs current single signatures
- **Intent:** L3 settlement = device-sig **THRESHOLD verifier** (≥k of n courier/owner sigs on PoD), NOT a single
  oracle. (BLUEPRINT L3:83; DANGER #3:94).
- **Product reality:** No payout contract / threshold verifier exists — G4 is an OPEN HIGH gap. CONSOLIDATED G4:154
  ("No payout contract | HIGH | DANGER #3 guard"). Current settlement is single-signature COD (MEMORY M5:282-283
  "cash pilot free", "venue fiscalizes on its own POS"). The threshold primitive is unbuilt on both sides.

---

## 2. PRODUCT-vs-VISION DRIFT (dowiz current vs long-term Sovereign→protocol path)

### 2.1 Two half-hubs ("one spine, two implementations")
- **Vision:** The owner hub is the thin *replaceable access layer* (L5) — one matcher among many, never the only
  one. (BLUEPRINT §5:138-142).
- **Reality:** A single-intake Node hub is LIVE; the designated Rust "kernel hub" is staging-dark and "not yet
  honest with itself" — literally described as "two half-hubs on one spine." (CONSOLIDATED P9:216, P7:209
  "Two half-hubs on one spine confirmed", P5:200 "two half-hubs"). The vision's clean "thin access layer" is not a
  realized seam — it's a split brain.

### 2.2 `hub_checkout` gates nothing
- **Vision:** Sovereign verification exit gate (0b-5, 1.2, 2.2, 2.3) must validate the event log.
- **Reality:** `hub_checkout` gates nothing; replay-parity is a placeholder; staging Playwright is vacuous (suites
  cannot fail). (CONSOLIDATED P5:198-200; extends hub-review #5). The exit gate — the red-line that proves the
  protocol seam holds — is decorative today.

### 2.3 "0% fee = moat" poetry
- **Vision (corrected):** Moat = earned local reputation graph + credible neutrality, NOT the fee. (BLUEPRINT §9:198
  "Not claiming '0% fee = moat' (poetry, F1)").
- **Drift:** The poetry survived into narrative/roadmap language and had to be explicitly retired. CONSOLIDATED C3:133
  "'0% fee = moat' — POETRY ... Economics must be 1–3% + value-added sinks". Product messaging still leans on the
  0% hook as differentiator (MEMORY brand canon, adoption lens M1:233 "InstaPorosi ... 0%"), contradicting the
  protocol economics it must eventually adopt (G5:1-3% — BLUEPRINT G5:154).

### 2.4 Field-wave over-claim
- **Vision:** Math core REAL (spectral/Kalman/Lyapunov/FFT). (CONSOLIDATED A2:33-36).
- **Drift:** "Field-sim wave replaces binary search" is a FALSE PREMISE — zero numeric root-finders in core; wave
  has no tuning surface to act on. (CONSOLATED C1:113-118). Worse, the iterative diffusion itself has a **SIGN BUG**
  (anti-diffusion, ‖u‖→4.7e31) MASKED by green tests — VbM violation. (CONSOLIDATED M6:289-291). The "physics" is
  both over-claimed AND partly wrong.

### 2.5 "Machine code" claim while wasm gate was broken
- **Vision:** wasm32 empty-import gate = the ONLY honest "machine code" proof. (CONSOLIDATED G9:148).
- **Drift:** While the gate failed (~94 errors, deep-research), "machine code" was claimed. The claim is now stale
  but was published. (CONSOLIDATED note:77-83; POETRY list:350-351 "'machine code' claim while wasm gate broken").
  See §4.1 for the blueprint/deep-research contradiction.

---

## 3. PROTOCOL-vs-VISION DRIFT (bebop current vs intent)

### 3.1 bebop2 used only as PQ-half of hybrid, not standalone
- **Vision:** bebop2 = the trustless PQ substrate the *whole* protocol rests on (L0-L4). (BLUEPRINT §4:99-118).
- **Reality:** bebop2 is "ONLY as PQ half of hybrid (KyberSlash-class timing), never alone." (CONSOLIDATED M2:248).
  By design it is a component, not the running protocol — the matcher/settlement/arbitration layers (L2-L4) are
  unbuilt (G4/G7/G8 gaps). The substrate exists; the protocol does not yet stand on it.

### 3.2 Destructive `memory.rs` tick contradicts "living memory" design
- **Intent:** Living memory = SOUND tiering, **move-not-delete** (dowiz ATTIC pattern). (MEMORY.md:55).
- **Reality:** `bebop2/crates/bebop/src/memory.rs:60-66` `tick()` does `nodes.retain(|_, n| hash%7 != clock%7)`
  = **PERMANENT delete**, no cold tier, no restore pointer (CONFIRMED by reading lines 60-66). CONSOLIDATED B2:97-100
  flags this as a DESTRUCTIVE design defect: "MUST refactor tick→move-to-ATTIC + restore pointer ... before
  LivingMemory holds real state." The protocol's memory design directly contradicts the operator's living-memory
  doctrine.

### 3.3 NTT exclusion correct but undocumented/applied inconsistently
- **Correct exclusion:** NTT is EXCLUDED; coefficient-domain schoolbook is bit-exact ground truth.
  (CONSOLIDATED A1:28 "NTT correctly EXCLUDED ... pq_kem.rs:306"). This aligns with the vision (bit-exact > fast).
- **Drift:** The *same rationale is not applied to ML-DSA-65* — which still uses butterflies/packing and is NOT
  bit-exact (G10). CONSOLIDATED A1:29 "ML-DSA-65 must follow SAME pattern (q=8380417, schoolbook, no butterflies)"
  — it does not yet. The exclusion rule is undocumented in code as a constraint; the inconsistency is the open gap.
  (fable-review H2:16 shows NTT was only removed this session — prior state was wrong; the *policy* "no NTT,
  bit-exact" is not stated in-code as a hard rule, so ML-DSA diverged.)

---

## 4. CROSS-CONTRADICTIONS (PART 1 vs PART 2, and docs vs code)

### 4.1 wasm32 "compiles clean" (blueprint) vs "~94 errors" (deep-research) vs "82 errors" (fable-review)
- BLUEPRINT:14 "G9: wasm32-unknown-unknown ... compiles CLEAN (0 errors, empty-import)."
- CONSOLIDATED note:77-83 → deep-research (same day, earlier) says wasm32 FAILS ~94 errors; fable-review H1:15
  "~82 errors (missing alloc::Vec imports, no #[global_allocator]/#[panic_handler]...)." Reality (cargo-verified,
  this session): wasm32 hardening MERGED (commit 388f90b) — so blueprint is *now* closer, but **overstated G10**
  and the two docs internally contradict each other on G9's status at audit time.
- **Net:** Trust cargo (merged, 0 errors now), not either narrative doc.

### 4.2 ML-DSA "roundtrip green" (blueprint) vs "NOT bit-exact" (audit)
- BLUEPRINT:12 "ML-DSA-65 roundtrip+tamper+**determinism-KAT drift-guard**"; BLUEPRINT:15 "⚠️ G10 (FIPS-204
  **bit-exact interop**): OPEN."
- CONSOLIDATED note:80-83 "ML-DSA packing sizes correct but NOT bit-exact (g10kat 9/5)." B1:90-91 "keygen pk
  diverges at byte 32 ... expand_mask γ1 buffer overrun pq_dsa.rs:299 (640B buf, 1024B read)."
- **Contradiction:** blueprint headlines "roundtrip green" while the same doc admits interop is OPEN; the audit
  shows it is concretely non-bit-exact (byte-32 divergence + buffer overrun). The "green" is a drift-guard KAT,
  not an interop proof — easy to misread as done.

### 4.3 `reputation.rs` courier-scoring vs operator red-line NO-COURIER-SCORING
- bebop2 `reputation.rs:69-81` `score()` computes a courier trust number; `reputation.rs:85-88` `risk_premium()`
  feeds a cost surface where "high-trust couriers are preferred, low/unknown trust costs more (risk premium),
  suspended = unreachable" (reputation.rs:11-12, :83-88). The doc-string even calls it the "poison/moat" (reputation.rs:14-16).
- Operator red-line: **NO-COURIER-SCORING** — legally sound (couriers = venue staff → dowiz avoids platform-law).
  CONSOLIDATED M2:251 "COLLISION for decision: bebop reputation.rs (courier scoring) vs operator red-line
  NO-COURIER-SCORING"; MEMORY M5:284 "No courier scoring decision is legally sound."
- **Direct contradiction:** the protocol ships courier scoring as a first-class primitive; the product/operator
  forbids it. This is a hard fork between bebop's trust model and dowiz's legal posture.

### 4.4 arch-hardening H4 (sqrt-Kalman): "NOW FIXED" vs "NOT fixed"
- BLUEPRINT:17-19 "arch-hardening H4 (sqrt-Kalman)... NOT fixed — deferred, host/analytic layer only."
- CONSOLIDATED B1:92-93 "arch-hardening H4 (sqrt-Kalman): earlier 'bebop2=0.30 vs numpy 4.66' — NOW FIXED this
  session (12/0, oracle 4.435489505337, reviewer APPROVE) — **blueprint §pre-amble line is STALE**."
- **Contradiction:** blueprint body says H4 deferred/unfixed; the audit says it was fixed and the blueprint line
  is stale. One of the two canonical docs is wrong about a crypto-math state.

### 4.5 `server.ts:858` /claim 404 — claim link death
- CONSOLIDATED P6:202-205 "Every claim link ever minted is dead (server.ts:858). 11/12 demos absent on prod. Prod
  worker machine STOPPED since 07-03."
- Code check: `server.ts:858` is the `SPA_ROUTES` array `['/admin','/courier','/dashboard','/s/','/login',
  '/branding-preview','/privacy']` — **`/claim` is absent**, so `GET /claim` falls through to the 404 handler
  (`server.ts:859-869`). The API routes (`/api/claim/accept` etc., `claim.ts:7-70`) exist, but the public claim
  *page* 404s. Confirms P6. (Note: P6 also asserts the prod worker machine stopped 07-03 — a runtime fact the
  code line alone cannot prove, but the SPA-route omission is independently verifiable.)

---

## 5. RECONCILATION (source-of-truth, what must change, convergence trigger)

### 5.1 Source-of-truth per axis
| Axis | Source-of-truth repo | Notes |
|------|----------------------|-------|
| Protocol / PQ crypto / matcher / settlement / identity | **bebop** (`bebop2` core) | BLUEPRINT §4:99-118; the protocol substrate is bebop's to define. |
| Product consuming the protocol (hub, courier, checkout, auth, RLS) | **dowiz** | dowiz consumes bebop; must not fork protocol semantics. |
| Long-term vision / red-lines / economics | **dowiz MEMORY + BLUEPRINT** (operator doctrine) | NO-COURIER-SCORING, NOBYPASSRLS, integer money, non-AI runtime are operator law. |
| Verifiable state | **cargo test / live code lines**, not agent summaries | CONSOLIDATED:380-385 load-bearing lesson. |

### 5.2 What MUST change where
- **bebop** must: (a) fix `memory.rs:60-66` destructive tick → ATTIC-move + restore (B2); (b) make ML-DSA-65
  follow the NTT-exclusion rule → schoolbook, bit-exact (G10, A1:29); (c) REMOVE/disable courier scoring in
  `reputation.rs` OR explicitly carve it out of dowiz's legal surface (§4.3, M2:251); (d) build L2-L4 (matcher,
  threshold settlement, arbitration) — currently only the crypto half exists (M2:248).
- **dowiz** must: (a) flip prod to the `NOBYPASSRLS` role (P8, §1.1) — memory says done, code says not; (b) route
  the CREATE+PRICE door through `decide` / `Command::PlaceOrder` (P7, §1.2); (c) wire `cause_hash` (pg.rs:863-864);
  (d) make `hub_checkout` a real gate (P5); (e) add `/claim` to `SPA_ROUTES` (server.ts:858) + restart prod worker
  (P6); (f) retire "0% fee = moat" poetry from narrative (C3, §2.3).
- **Docs (both)** must: reconcile G9/G10 status to cargo-truth (§4.1/4.2); fix H4 staleness (§4.4); quarantine
  field-wave + fabricated math claims (C1/C2, §2.4).

### 5.3 Single convergence trigger
**FIRST REAL ORDER.** Both PARTs and both blueprints converge here (CONSOLIDATED PART1:9 "Both converge on one
trigger: FIRST REAL ORDER"; PART2 G2:338 "BOTH programs converge on ONE trigger: FIRST REAL ORDER"; BLUEPRINT §8
build order starts at MVP hub). Until a real order flows through `decide` under `NOBYPASSRLS` with a wired
`cause_hash`, none of the protocol seams are proven in production. Every drift below is ranked by how much it
blocks or distorts that first real order.

### 5.4 Drift ranked by EV / risk
| Rank | Drift | Repo | EV/Risk | Why |
|------|-------|------|---------|-----|
| R1 | RLS `BYPASSRLS` in prod (§1.1) | dowiz | CRITICAL / active | ~103 policies dormant; operator red-line named but unmet; security exposure today. |
| R2 | `reputation.rs` courier-scoring vs NO-COURIER-SCORING (§4.3) | bebop↔dowiz | CRITICAL / legal | Hard fork: protocol builds what operator forbids; blocks any courier marketplace (G2). |
| R3 | CREATE door bypasses `decide` (§1.2) + `cause_hash` placeholder (§1.2) | dowiz | HIGH | Undermines the one law (C3); event-log seam decoration, not enforcement. |
| R4 | `hub_checkout` gates nothing (§2.2) | dowiz | HIGH | Exit gate is theater; 1.2/0b-5 integrity unproven. |
| R5 | `/claim` 404 + dead prod worker (§4.5) | dowiz | HIGH / growth | Highest growth-EV block, 1-line fix; every claim link dead. |
| R6 | Two half-hubs (§2.1) | dowiz | HIGH | Split brain; "thin access layer" unrealized. |
| R7 | `memory.rs` destructive tick (§3.2) | bebop | MED-HIGH | Corrupts living-memory doctrine; latent data loss before real use. |
| R8 | ML-DSA not bit-exact / NTT rule inconsistent (§3.3, §4.2) | bebop | HIGH / interop | Blocks protocol key minting + any cross-node trust. |
| R9 | "0% fee = moat" / field-wave / machine-code poetry (§2.3/2.4/2.5) | docs | MED | Misleads adopters + investors; erodes doc-truth discipline. |
| R10 | Doc self-contradictions G9/H4 (§4.1/4.4) | docs | MED | Cargo-truth vs narrative mismatch; trust erosion. |
| R11 | "No central server" vs relay caveat (§1.3) | docs/vision | LOW-MED | Honest once caveat added; not blocking. |
| R12 | bebop2 PQ-half-only, L2-L4 unbuilt (§3.1) | bebop | LONG / structural | Expected for MVP; gates Phase-2+, not first order. |

---

## Evidence index (cited lines)
- MEMORY.md:11, :28, :55, :282-284
- CONSOLIDATED-AUDIT: P1:181-184, P5:198-200, P6:202-205, P7:207-209, P8:211-213, P9:216-226,
  A1:28-29, A2:33-36, B1:90-93, B2:97-100, C1:113-118, C2:120-131, C3:133-135, M2:242-251, M5:282-287,
  M6:289-291, note:77-83, cross:344-353, queue:355-378, lesson:380-385
- BLUEPRINT-v3: C3:62, C4:63, L1:79-80, L3:83, DANGER:94-95, §4:99-118, §5:138-142, §8:181-191,
  §9:194-199, :14, :15, :17-19
- fable-review-bebop2: H1:15, H2:16, H4:18
- dowiz code: `server.ts:858-869`, `rebuild/crates/api/src/routes/orders/pg.rs:91, 405-410, 491, 567, 863-864`,
  `apps/api/src/routes/public/claim.ts:7-70`, `apps/api/src/workers/courier-cron.ts:37`,
  `apps/api/src/workers/reconciliation.ts:168`, `apps/api/src/workers/order-timeout-sweep.ts:68-70`,
  `docs/audit/2026-06-18/proposed-migrations.sql:26-28`
- bebop2 code: `crates/bebop/src/memory.rs:60-66`, `crates/bebop/src/reputation.rs:11-16, 69-88`
