# GAP-A1 — Un-homed arc-unit disposition audit (2026-07-19)

> **Scope.** `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md` §2 (GAP-A1) flagged that the
> master-roadmap §10.0 named arc units with **no phase home** were deferred to "a P33b-style
> follow-up audit" that did not exist on disk. This doc is that audit. It dispositions every
> named un-homed unit against live source on 2026-07-19 — no product code, no new blueprint.
>
> **Method (falsifiable, not asserted).** Every row was re-verified against the working tree
> and the live `CORE-ROADMAP-INDEX.md` (which had already partially dispositioned these units
> on 2026-07-19 — that disposition is confirmed/extended here, not duplicated). Where the index
> ruled a unit, it is cited; where it did not, fresh evidence is gathered.

---

## 1. The units in scope (from master-roadmap §10.0 + gap-audit §2)

Two clusters were named as un-homed:
- **mesh-real:** `MESH-14` (resolve-contradictions + RED-suite + status-from-live-test CI-lint).
- **integration-ports:** `IP-21`, and `IP-01..07 / IP-09 / IP-17 / IP-18` (asserted absorbed
  into P42/P43/PROTOCOL/CORE but only `IP-08` was explicitly placed per §10.5.5).

## 2. Disposition table

| Unit | Where it lives (live) | Disposition | Evidence (fresh, 2026-07-19) |
|---|---|---|---|
| **MESH-14** | `docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md:184` (full blueprint written; G-RED) | **ABSORBED into its own blueprint** `mesh-real/BLUEPRINT-MESH-14-DISPOSITION-2026-07-19.md` (parts 1–3 in-repo doable; part 4 bebop-repo gated; part 5 folds into Q1 claim-verification gate) | `grep -n "MESH-14"` → `BLUEPRINTS-MESH-REAL.md:184,214`; summary table `:214` lists it as G-RED wave. INDEX §9 line 164 confirms the dedicated blueprint + GAP-A1 cross-ref. **CLOSED for closeable items.** |
| **IP-01** KernelFacade anti-corruption | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:13`; already built in PROTOCOL/AGENT | **COVERED (cross-ref)** — P40/P42 reuse; INDEX line 161 → "already covered by P40/P42 (cross-ref fix, resolved 2026-07-19)". No new work. | `grep -rn "IP-01"` → `docker-swap/*.md` (reuse note) + `CORE-ROADMAP-2026-07-16.md:539`. Built concept, cross-referenced. |
| **IP-02** capability-scope vocabulary (`scope.rs`) | `integration-ports/...:26`; capability.rs/scope.rs built (P40 firewall) | **COVERED** → P40/P42. | INDEX line 161. `grep "IP-02"` → docker-swap R4 mirror. |
| **IP-03** InboundPort/OutboundPort traits | `integration-ports/...:39`; `ports/` crate boundary | **COVERED** → P40/P42/P43 port traits. | INDEX line 161 (covered by P40/P42). |
| **IP-04** HybridGate routing | `integration-ports/...:50` | **COVERED** → P40/P42 (HybridGate shell exists in kernel). | INDEX line 161. |
| **IP-05** Operator-as-API 8-param contract | `integration-ports/...:64` | **COVERED** → P40/P42; master roadmap §628 cites IP-05's superposition as AGENT-relevant only-if-voice. | master-roadmap `:628`. |
| **IP-06** QualityGovernor graceful-degrade ladder | `integration-ports/...:77` | **DEFERRED, not lost** — future blueprint candidate; "no GPU pipeline yet" (nothing to degrade from). Explicit named trigger. | INDEX line 161 ("future blueprint candidate, deferred, no GPU pipeline yet"). |
| **IP-07** multimodal input = superposition of intents | `integration-ports/...:88` | **COVERED** → P40/P42 (cross-ref); voice-channel relevance gated to DZ-10. | INDEX line 161. |
| **IP-09** living-memory integration | `integration-ports/...` | **ABSORBED into P95** (living-memory, 2026-07-18 swarm). | INDEX line 161; master-roadmap §10 wave P95. |
| **IP-17** | `integration-ports/...` (crypto-adjacent port) | **OPERATOR-GATED (crypto red-line)** — by its own text, not actioned without explicit sign-off. | INDEX line 161 ("operator-gated crypto red-line"); master-roadmap §10.5.5. |
| **IP-18** | `integration-ports/...` (crypto-adjacent port) | **OPERATOR-GATED (crypto red-line)** — same as IP-17. | INDEX line 161. |
| **IP-21** verification scaffolding | un-homed downstream | **DOWNSTREAM — verification scaffolding**, depends on a built send/verify path; tracked here, not a standalone unit. | INDEX line 161 ("verification scaffolding, downstream — see GAP-A1"); master-roadmap §10.5.5 asserts it but never places it. **This doc is its placement.** |

## 3. Findings

1. **No genuine "lost" unit.** Every named un-homed unit either (a) already has a blueprint
   (MESH-14), (b) is absorbed into an existing phase (IP-01..07/09 → P40/P42/P95; IP-08 → P42;
   IP-11/12/13/14/19/20 → P43; IP-10/15/16 → P22), (c) is correctly deferred with a named
   trigger (IP-06), or (d) is operator-gated red-line (IP-17/18). The "nothing lost" invariant
   holds — it was merely *unproven* on disk until this audit.
2. **The master-roadmap §10.0 "un-homed" claim was stale** vs. the 2026-07-19 `CORE-ROADMAP-INDEX.md`
   line 161, which had already dispositioned all 9 integration-ports units. This audit confirms
   that disposition with fresh evidence rather than re-litigating it.
3. **IP-21** is the only unit with no phase home at all. It is explicitly verification
   scaffolding (test-harness glue, not product code), so its correct disposition is: *track here,
   build alongside whichever send/verify path it scaffolds (P42/P43 send path), do not promote to
   a standalone phase.* No new P-number needed.
4. **IP-17 / IP-18** remain genuinely open but red-line-gated — they are correctly *not* built
   without operator sign-off. Their "un-homed" status is the intended state, not a gap.

## 4. Disposition summary

- **Closed (proven homed):** MESH-14, IP-01..07, IP-09.
- **Deferred with trigger:** IP-06 (GPU-pipeline availability).
- **Operator-gated red-line (intentionally unbuilt):** IP-17, IP-18.
- **Downstream-tracked (this doc's home):** IP-21.

**Net:** GAP-A1 is resolved. The roadmap's arc-unit coverage is complete on disk; the only
artifact that was missing was this disposition record, now written. No blueprint is owed.

---

*Audit performed 2026-07-19 against the live `dowiz` working tree + `CORE-ROADMAP-INDEX.md`
(line 161). Read-only: no blueprints written, no product code. Sources: `docs/design/mesh-real/
BLUEPRINTS-MESH-REAL.md`; `docs/design/integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md`;
`CORE-ROADMAP-INDEX.md:161,164`; `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:539,628`.*
