# BLUEPRINT P46 — Multi-Product Platform: "dowiz Local" + Marketplace (2026-07-18)

> **Priority banner:** P46 is the **FURTHEST-FUTURE** phase — the terminal node of the entire
> roadmap (§10.5.5: "Depends on literally everything above… Blocks nothing"). This blueprint's
> content is therefore almost entirely a **gate definition** and a **reuse-measurement
> methodology**. It deliberately does NOT design the second product. A blueprint that argues
> for its own low priority is more honest than one that inflates its own importance.

Phase: **P46** (ECOSYSTEM/OPS) · Status: **PLANNED (0%) — and correctly 0%**
Absorbs: EC-17 + the multi-product/marketplace remainder of the ecosystem-strategy arc
Source arc: `/root/.claude/projects/-root-dowiz/memory/ecosystem-strategy-arc-2026-07-13.md`
Master entry: `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.5 (P46)

---

## 1. Ground truth — every cite re-verified live this pass (standard §2 item 1)

1. **"dowiz Local" has zero code hits.** Live grep across `*.rs`/`*.ts`/`*.toml` in this repo:
   zero matches for `dowiz Local`/`dowiz-Local`. It exists only in the ecosystem-strategy arc
   memory and roadmap prose. §10.5.5's "never shipped, zero grep hits" claim: **confirmed**.
2. **The first product is not live.** Per §10.2 (re-checked): P37 (order HTTP surface) is
   PLANNED 0% — "no dynamic HTTP server exists in-repo"; P38 (render engine) and P39
   (app-shell: installability + capability-auth + offer math,
   `MASTER-ROADMAP…-2026-07-16.md:973`) are PLANNED. There are no real tenants because there
   is no reachable product. Every clause of this phase's gate is therefore currently red.
3. **The reuse substrate P46 would prove already exists and is the point.** The kernel is one
   crate consumed by the product, not forked per-product; the arc's shared `KernelFacade`
   concept is already built in PROTOCOL (§10.5.5 absorption ledger — cross-referenced there,
   not re-claimed here). P46 adds no kernel machinery; it *measures* what CORE/PROTOCOL built.
4. **Ledger note on one dependency cite:** the coordinating session names P39 and a "P52"
   among this phase's upstream dependencies. P39 is verified on disk (cite above). **"P52"
   has zero grep hits anywhere in `docs/` at write time** — it is presumably a phase being
   drafted in a parallel lane this session. Recorded honestly: a future reader must re-grep
   before treating P52 as a binding gate clause; the on-disk gate (§3) is authoritative until
   the master roadmap itself names P52.

## 2. Scope & anti-scope — why this file is deliberately light

Same reasoning as sibling `BLUEPRINT-P44-cache-layers-scaleout.md` §2, stated once more
because it is this file's load-bearing wall: the operator's standing instruction — "нічого не
добавляти що не критично" — plus this session's own "planning outpaces building" diagnosis
mean that investing design effort in the roadmap's terminal node, while P34/P37's wiring sits
unbuilt, would be the exact failure mode already flagged. This blueprint exists so a future
swarm agent finds a real gate and a real measurement method instead of a blank — nothing more.

**In scope (all of it far-future):**
- The single hard gate (§3) and the reuse-measurement methodology (§4) — this file's actual
  content.
- A short, non-binding sketch of what the second product plausibly is (§5) — cited from the
  arc, feature-design explicitly excluded.
- Marketplace (EC-17): named as the step *after* reuse is proven, per the arc's own sequencing
  (its wave plan defers marketplace to W8, after ship-dowiz-Local at W7). Nothing else about
  it belongs in writing yet.

**Anti-scope (binding):**
- **Do NOT design "dowiz Local" features** — no vocab lists, no screens, no domain model for
  laundry/repair/grocery. That work starts only after the §3 gate is green, against a then-real
  first product, and gets its own blueprint at that time.
- **Do NOT build marketplace scaffolding, plugin registry, or partner API** before §3's gate
  AND §4's reuse proof are both green (§10.5.5: a hard gate, not a suggestion — honoring the
  original EC plan's own warning against building marketplace infra before proving reuse
  empirically).
- **Do NOT patch the kernel for any product.** Any product-specific patch to CORE falsifies
  the phase's entire premise (§10.5.5 DoD-1). The kernel stays one shared crate; products are
  consumers.
- **Do NOT let this file grow.** Additions belong here only when the gate flips; speculative
  appendices are the failure mode, not diligence.

**Depends on / blocks:** depends on everything — see §3. Blocks nothing; terminal node. An
agent landing here while the gate is red should leave and build the gate's red clauses instead.

## 3. THE GATE — the single hard precondition (this blueprint's centerpiece, part 1)

P46 has exactly one gate, already named in §10.5.5 and formalized here. **P46 is startable
only when a real first product is live with real tenants**, which decomposes into the
conjunction (every clause independently checkable, all currently red):

| # | Clause | Checked by |
|---|--------|-----------|
| G1 | DELIVERY real: P37 order HTTP surface + P38 rendered UI + P39 app-shell live | a real order placed end-to-end on the deployed product |
| G2 | Real tenants: paying/operating venues that are not the operator's own fixtures | tenant count ≥ 2 in production data (the "second-tenant proof" — one tenant proves a demo, two prove a product) |
| G3 | ECOSYSTEM P45 ops floor green **including off-Hetzner immutable backup** (P45 DoD-4) | restore-from-off-site drill passes with Hetzner unreachable |
| G4 | P43: at least one working external port | the port's own DoD (e.g. a messenger send that actually transmits) |

Falsifier discipline: the gate is a conjunction — a swarm agent may not argue "3 of 4 is
close enough." Any red clause = P46 not startable = go build that clause. (Per §1.4: if a
P52 phase lands in canon as a delivery dependency, it joins G1 by reference; re-grep first.)

## 4. "Reuse proven" operationally — the measurement methodology (centerpiece, part 2)

What P46 actually delivers is a **number**, not a platform. Formalizing §10.5.5 DoD-2:

1. **Zero-fork check (mechanical):** the second product consumes the *same* kernel crate at
   the same version as the first — same `Cargo.toml` dependency, no vendored copy, no
   product-specific `#[cfg]` in CORE. Falsified by any kernel diff attributable to a product.
   This is CI-checkable (one job: assert both products' lockfiles pin the identical kernel).
2. **Reuse ratio (published, not asserted):**
   `R = LOC_nonkernel(product2) / LOC_nonkernel(product1)`, measured by the same tool
   (`tokei`-class) over both product trees, kernel and shared crates excluded from both
   numerators. The arc's own claim to beat: adjacent verticals need "~100% reuse, only
   vocab + skin" — i.e. R should be *small*. The number is published in this file when
   measured; no target is pinned today because pinning a threshold against an unbuilt
   product would be fake precision. What is pinned: **the ratio must be published with the
   measurement command, or reuse is unproven** — DoD-2's falsifier.
3. **Third-party reproducibility:** the measurement is one command an agent with zero context
   can run; if it takes a narrative to explain what counted as "non-kernel," the measurement
   is wrong, not the narrative.

## 5. What the second product plausibly is — cited, not designed

The ecosystem-strategy arc's own adjacency ladder (arc memory, verified this pass):
**Delivery → Local (~100% reuse, only vocab + skin) → Fleet (+matching port) → Ledger**.
"dowiz Local" = local services (laundry/repair/grocery-class) on the unmodified delivery
kernel: same order/state machine, same money law, same courier/agent mesh — different
vocabulary and skin. That one sentence is the entire product thesis, and per §2 anti-scope,
this document deliberately stops there. Choosing the actual vertical is an operator decision
at gate-green time, informed by whatever the first product's real tenants taught — not by a
2026 guess.

## 6. Build items — spec → RED test → code (standard items 3, 5)

**Honest statement: there is nothing to build.** P46 is 0% because it must be. The only
artifacts this phase will ever add before its gate flips are (a) the CI zero-fork check
(§4.1) and (b) the reuse-ratio measurement script (§4.2) — and even those are only worth
writing when a second product tree exists to point them at. Inventing RED-test names today
for a product that must not be designed yet would be fake rigor; this section stays empty on
purpose. Adversarial case (item 5), recorded for the future pass: the zero-fork CI check must
go RED on a deliberately vendored kernel copy before it is trusted GREEN.

## 7. DoD — falsifiable (standard item 2; formalizing §10.5.5's two items)

1. A second product runs on the **unmodified** kernel with zero kernel forks — falsified by
   any product-specific patch to CORE (mechanically: §4.1's CI check red).
2. Reuse is **measured, not asserted** — the second product's non-kernel LOC is published
   against the first product's, per §4.2, reproducible by one command.
3. (Marketplace/EC-17 has NO DoD here — it does not get one until 1 and 2 are green.)

## 8. Links to docs & memory (standard item 7)

- Master entry: `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.5 P46 block
  (+ P45 block for gate clause G3, P43 block for G4).
- Source arc: `/root/.claude/projects/-root-dowiz/memory/ecosystem-strategy-arc-2026-07-13.md`
  (adjacency ladder, wave plan, marketplace deferral — design provenance; cite, don't
  re-derive).
- Sibling far-future blueprint (shared scope-honesty rationale):
  `BLUEPRINT-P44-cache-layers-scaleout.md` §2.
- Gate-clause blueprints: `BLUEPRINT-P37-order-http-surface.md`,
  `BLUEPRINT-P38-webgpu-render-engine.md`, `BLUEPRINT-P45-ops-security-monitoring.md`
  (same directory).

## 9. Standard-compliance map (all 20 points — honest N/A where true)

| Item | Status | Item | Status |
|---|---|---|---|
| 1 ground truth | §1, live cites | 11 isolation | N/A until a second product exists |
| 2 DoD | §7, falsifiable | 12 mesh | N/A at this depth |
| 3 TDD plan | §6 (empty on purpose, one future adversarial case) | 13 rollback-as-math | N/A at this depth |
| 4 types first | none needed — this phase defines a gate + a ratio, not code | 14 error-propagation gate | §4.1 CI zero-fork check |
| 5 adversarial case | §6 vendored-kernel RED check | 15 living memory | N/A |
| 6 hazard-as-math | zero-fork invariant: product-specific kernel state made unrepresentable by single-shared-crate structure (§4.1) | 16 tensor/spectral | N/A |
| 7 links | §8 | 17 regression | ratio published → becomes the pinned number |
| 8 scaling axis | G2's tenant count IS the axis; marketplace = the schema-change point | 18 agent instructions | §10 |
| 9 Linux discipline | nothing new to verdict | 19 reuse-first | the phase IS the reuse proof |
| 10 benchmarks | §4.2 ratio = the measured number | 20 Hermetic | correspondence: second product mirrors first through the same kernel Law; no new authority surfaces |

## 10. Clear instructions for other agentic workers (standard item 18)

You have zero session context. Read this before touching P46:

1. **Run the gate (§3) first.** All four clauses green? Almost certainly not. Any red clause
   means P46 is not startable — go build that clause (P37/P38/P39, P45 DoD-4, or P43). That
   redirect is this blueprint's primary function.
2. If (and only if) the gate is green: bring the operator §5's adjacency ladder plus the
   first product's real tenant learnings, and get a vertical decision. Do not pick one
   yourself.
3. Then write the second product's own blueprint (new file, against the 20-point standard),
   build §4's two measurement artifacts alongside it, and publish the ratio here.
4. Marketplace work remains forbidden until §7 items 1-2 are green. No exceptions, no
   scaffolding "to save time later."
