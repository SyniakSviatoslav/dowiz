# P-I Wave-1 Audit — Cross-Repo Consolidation (2026-07-17)

> "P-I" here = CORE-ROADMAP **Layer I** (cross-repo consolidation), an altitude lens — NOT execution
> phase P-anything. Naming ruling ratified per this audit's own §4; see `BLUEPRINT-P-I-consolidation.md`.

> **Wave 1, Opus, read-only.** Ground-truth audit for CORE-ROADMAP layer **P-I** (cross-repo
> consolidation): prove that superseding the older master-roadmap docs with banners loses **nothing
> unique**. Every cite verified live this pass on branch `feat/p19-growth-engine` (HEAD `f01f9bb6b`),
> not inherited from any prior doc's claim. Companion Wave-1 audits already on disk in this dir:
> `P-D-audit-root-delegation-policy.md`, `P-G-audit-product-ui-post-decommission.md`,
> `P-H-audit-telemetry-regression-benchmarks.md`. This is the fourth (P-I).
>
> **Bottom line up front:** the consolidation is **safe to execute** — the canonical roadmap package
> already covers the overwhelming majority of every older doc. But this audit found **(1)** the
> inventory is undercounted: there are **6** master docs, not 5 — `MASTER-EXECUTION-PLAN-2026-07-13.md`
> was omitted from `CORE-ROADMAP-STANDARD` §0 and needs the same banner; **(2)** the phase-blueprint
> range is **P01–P19 as numbered files but P01–P30 as canonical phases** (P20–P30 live as standalone
> blueprints referenced by SOVEREIGN §8); **(3)** **six genuinely-would-be-lost items** (most minor,
> two worth folding); and **(4)** the P-A..P-I letters are an **orthogonal altitude axis**, not a
> renumbering — a concrete de-collision recommendation is in §4.

---

## 0. Verified current state (run fresh this pass, not trusted from the failed prior attempt)

| Fact | Verified value | Method |
|---|---|---|
| Branch | `feat/p19-growth-engine` | `git branch --show-current` |
| HEAD | `f01f9bb6b` (Merge P19 growth-engine) | `git log --oneline -1` |
| **Numbered phase blueprints** | **P01–P19** (19 files) | `ls sovereign-roadmap-2026-07-16/BLUEPRINT-P*.md` → exactly P01…P19 |
| **Canonical roadmap phase range** | **P01–P30** | SOVEREIGN §8.1–§8.12 add P20–P30, each pointing at a **standalone** blueprint file in `docs/design/` (verified all 13 referenced files EXIST on disk) |
| Master-roadmap docs on disk | **6** (not 5) | `find` for `MASTER-*.md` + `*MVP*.md` |
| Prior partial P-I output | **none** | no `P-I-*.md` in this dir; the failed run died before writing. Siblings P-D/P-G/P-H present |

**Correction to the prior attempt's claim.** The failed run reportedly concluded "canonical doc spans
P01–P30." That is **half-right and misleading**: the *numbered* `BLUEPRINT-P*.md` files span only
**P01–P19**; phases **P20–P30 exist only as standalone `BLUEPRINT-*-2026-07-17.md` files** indexed from
SOVEREIGN §8, never as `BLUEPRINT-P20..P30` files. Anyone auditing "the P0x blueprints" who stops at
the numbered directory misses a third of the roadmap. This distinction is load-bearing for §4.

**The 6 master docs (real inventory):**

| # | Doc | Lines | In CORE-STANDARD §0 inventory? | Carries a superseded-by banner today? |
|---|---|---|---|---|
| 1 | `MASTER-ROADMAP-MVP-2026-07-12.md` (repo root) | 147 | ✅ listed | ❌ no |
| 2 | `docs/design/MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md` | 178 | ✅ listed | ❌ no |
| 3 | `docs/design/MASTER-INTEGRATION-PLAN-2026-07-14.md` | 172 | ✅ listed | ❌ no |
| 4 | `docs/design/MASTER-ROADMAP-10-PHASES-2026-07-14.md` | 117 | ✅ listed | ❌ no |
| 5 | **`docs/design/MASTER-EXECUTION-PLAN-2026-07-13.md`** | 111 | **❌ OMITTED** | ❌ no |
| 6 | `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` (**canonical**) | 461 | ✅ (as canonical) | n/a (it is the target) |

Doc #5 is a real find. `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:11` explicitly names
`MASTER-EXECUTION-PLAN-2026-07-13.md` among the four it supersedes — yet the CORE-ROADMAP-STANDARD's
own ground-truth inventory (§0, lines 23–30) lists only the other four older docs and misses this one.
**The banner pass must cover all 5 older docs, not 4.** None of the 5 carries an incoming
superseded-by banner today; SOVEREIGN declares supersession one-directionally in its own header only.

---

## 1. Method

For each older doc, every concrete decision / finding / scoped item was extracted and grepped against
the **canonical roadmap package**: the SOVEREIGN doc + `sovereign-roadmap-2026-07-16/` (all R1/R2
reports + BLUEPRINT-P01..P19 + the follow-up research docs) + all 13 standalone P20–P30 blueprints +
the `bebop2-mesh-tensor-hermetic-2026-07-17/` masterwork corpus. An item is **COVERED** if it appears
in a phase blueprint or the SOVEREIGN doc; **WEAK** if only in an R1/R2 gap-analysis input; **LOST** if
absent everywhere. (Grep caveat noted for the record: extended-regex alternation must use bare `|`, not
`\|` — an early pass produced false zeros before this was corrected.)

---

## 2. Per-doc audit — what is covered, and the would-be-lost items

### 2.1 `MASTER-ROADMAP-MVP-2026-07-12.md` — mostly COVERED

Covered (cite = where it lands in canon): PQ substrate built-but-unconsumed (SOVEREIGN §1.1;
`BLUEPRINT-P03`); hybrid KEM X25519⊕ML-KEM + ML-DSA-65 (P03); confidential transit / `NoopPayloadEnc`
→ real payload enc, D4 (P03, `BLUEPRINT-P09` §confidential-wire; SOVEREIGN §1.1 item 1); at-rest
AES-256-GCM volume key (P09; R1-A); QRNG entropy seam OS-default+fallback (`BLUEPRINT-P02`, R1-A,
P09); BPv7 store-and-forward custody (`BLUEPRINT-P09:412` `bpv7.rs` custody + `MeshNode<T:Transport>`
generic, F12 island/custody tests); roles owner/courier/customer + NOSTR/ActivityPub/MCP adapter law
(D5; `BLUEPRINT-P02:113`, R1-E:118 — "bridges never core", MCP=E17); hard invariants D0
(decentralized/local-first/PQ/mesh) = the whole mesh-foundation thesis.

**Would-be-lost item L1 — node self-update code-signing (D4.4, `codesign.rs`).**
`MASTER-ROADMAP-MVP-2026-07-12.md:41,68` and the P3 stream (`:89`) specify: update-blob **ML-DSA
verify vs a pinned root before apply**, RED gate = unsigned/tampered blob refuses. Grep of the whole
canon package for `codesign|code-sign|update blob|pinned root` returns only **incidental** hits
(`BLUEPRINT-P19:102` lists "code-signing" as one property of "crypto-as-protocol"; an R2 mention) —
**no scoped phase item** for signing node update artifacts. The primitive was actually built
(`kernel/src/pq/codesign.rs`, 6 tests, on the `dowiz-pq` lineage).
→ **Recommendation:** fold into **Phase 10** (Hub Runtime: Policy-as-Data, Kill-Switch, **Boot**) as a
boot-integrity / update-verification unit — that phase already owns boot and the M9 kill-switch signing
path, so update-blob verification is its natural home. If the operator prefers, an E53-form deferral is
acceptable (trigger: first over-the-mesh node-update mechanism), but silent drop is not — this is a real
decentralized-node security need, not obsolete.

**Would-be-lost item L2 (minor) — transport bake-off rationale.**
`MASTER-ROADMAP-MVP-2026-07-12.md:76–78` scoped `docs/transport-research-2026-07-12.md`: DTN/dtn7-rs vs
QUIC/TCPCLv4 vs **Zenoh vs Reticulum vs SpaceWire/SpaceFibre**, with **libp2p rejected** and BIBE
custody verification. The **decision** is embodied in code (P09's `bpv7.rs` + `Transport` trait), but
the *comparison/rejection rationale* is orphaned — grep for `Zenoh|Reticulum|TCPCLv4|BIBE|transport-research`
across canon = the research doc is never cross-linked (only OPEN-SOURCE-CREDITS-LIST captures transport
*influences* generically). → **Recommendation:** one cross-link line from `BLUEPRINT-P09` to the
transport-research doc; low stakes (decision is code-realized, not at risk).

Everything else in the MVP doc (fractal fingerprint tag, S1–S4 QUIC/TLS/first-order-sim milestones) is
either superseded status-reporting or covered by P03/P09/P13.

### 2.2 `MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md` — largely obsolete (pre-decommission), 3 items

This doc is the most stale: it is almost entirely about the **now-fully-deleted** Node/TS stack
(`server.ts`, `apps/web`, `packages/*`, GDPR DI, `/claim` 404, OG cards) — deleted at HEAD
(SOVEREIGN §1.2; commits `79ef316f6`/`db766de47`/`5675c349b`). Its Tier-0..Tier-5 spine and TS specifics
are genuinely obsolete-by-architectural-decision (the mesh-foundation pivot + JS/TS decommission).

Covered: money crypto audit ladder Wycheproof→FIPS→ACVP→constant-time→audit (`:85`) → `BLUEPRINT-P06`,
`BLUEPRINT-P08`, `DECART-P06` (`Wycheproof|ACVP` = 11 canon hits); reliability gate **LD0–LD11 GO
before cutover** (`:81`) → `BLUEPRINT-P16:32,491–494` explicitly *reconciles* the "L0-L11" label
against `docs/audit/2026-06-18/reliability-gate-SKILL-reconciled.md` ("L0–L11 → L0–L9 mapped to real
code") and the `reliability-gate` skill; migration ladder corrected to **pgrust replica** not SQLite
(`:75–79`) → `BLUEPRINT-P12` durable-storage; **DRIFT R2 courier-scoring hard fork** (`:107`) →
NO-COURIER-SCORING is now law M12, covered densely (`BLUEPRINT-P02`, `BLUEPRINT-P14`, `BLUEPRINT-P01`;
31 canon hits).

**Would-be-lost item L3 — courier out-of-app notification send-path (`notify.rs` / NotifyHub / N1–N2).**
`:132–141` documents a *built*, dependency-free `server/src/notify.rs` (`NotifyHub` trait + `WebhookSink`
+ VAPID web-push seam) that **signals couriers on every legal order transition** — the "real order
reaches a courier" gap it explicitly closed. Grep for `NotifyHub|VAPID|web-push|out-of-app` across canon
= one **incidental** hit (`R1-C:94` lists VAPID as an env secret). No scoped item. Partially *dissolved*
by the mesh model (a courier device is a node that receives `MeshEvent`s directly), but an out-of-band
wake alert (sleeping device) remains a real need, and the live Telegram alerting pipeline
(`rust-spool`, flagged in `BLUEPRINT-FAULT-ISOLATION` A1) is the closest surviving analog.
→ **Recommendation:** fold into **Phase 13** (Delivery on Protocol — courier leg) as an out-of-band
notification unit, cross-referencing P08's alerting sink; or explicitly record "superseded — courier
node receives MeshEvents directly; out-of-band wake is a P13 sub-unit." Not a silent drop.

**Would-be-lost item L4 (minor) — anonymous `.onion` / Tor tier.**
`:88` (Tier-5 earn-it) scopes an "Anonymous .onion tier → gated on vendor node." Grep `onion` across the
whole canon package = **0 hits**. → **Recommendation:** an E53-form backlog entry (what: anonymity/Tor
tier; why-suspended: no vendor-node tier + no demonstrated anonymity demand; trigger: vendor-node tier
ships + a venue requires anonymity), not a phase. Cheapest correct handling of a genuinely-deferred
earn-it.

**Would-be-lost item L5 (historical, low stakes) — the "lost reports" honesty ledger.**
`:152–162` records the "13 research/design reports genuinely LOST" finding, marked RESOLVED-AS-LOST (the
decisions survive in `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3`). `MASTER-ROADMAP-10-PHASES:104` adds
"~20 missing 2026-07-11 reports, manifest filed honest." Grep `RESOLVED-AS-LOST|genuinely LOST|missing.*report`
= 0 canon hits. → **Recommendation:** a single line in the consolidation record noting these were
closed-as-lost with their decisions preserved elsewhere; no phase, no resurrection (re-creating them
would violate ground-truth discipline).

### 2.3 `MASTER-INTEGRATION-PLAN-2026-07-14.md` — COVERED (its findings became phases/blueprints)

This doc's concrete technical findings are the **best-absorbed** of all — nearly every one has a named
carrier: Kalman-first / `ema_next`=1D-Kalman / SpectralKalman predict→full / courier constant-velocity
Kalman (`:57–64,99–102`) → `BLUEPRINT-P04`, R1-C, R2 (`Kalman|ema_next` = 20 canon hits); minimal
reverse-mode autodiff for capture-fitting (`:98,B2`) → `BLUEPRINT-P17`; `eqc` → named-equation IR graph
(`:24,B3`) → `eqc` (18 canon hits, P01/P02/P04/P19); native Rust backup organ (Duplicati pattern,
content-addressed + remote-rebuildable index + **FastCDC** + two-phase compaction) (`:106–109,B4`) →
`BLUEPRINT-P12` durable-storage + `3-2-1-1-0` off-Hetzner (P12); **3-eigensolver dual-authority** parity
gate (`:70–76,A3`) → `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17` + `BLUEPRINT-P11`
(`eigensolver|Faddeev|dual-authority` = 20 hits); stranded **recall@5=1.0** living-knowledge engine at
`545f37df` (`:77–79,A2`) → R1-D, R2, `BLUEPRINT-P02` (8 hits); `Vec<Vec<f64>>`→flat matmul (`:83–84,A4`)
→ P04, EIGENVECTOR-REFACTOR, `BLUEPRINT-MEMORY-OPTIMIZATION` (6 hits); Noether "money as non-covariant
scalar" framing of `money_guard` (`:64,C4`) → `BLUEPRINT-P02`, `BLUEPRINT-P15` (7 hits); L0 trigram +
ART secondary index (`:110,B5`) → R1-C, R1-D, R2 (11 hits); circuit/queueing lens (Little's Law /
**Kingman VUT** / M/M/1) (`:32–43,C2`) → `BLUEPRINT-LATENCY-ELIMINATION` (present, P29); deep-research
verify-failure→retrieval-trigger (`:49–52,C1`) → harness research (`HARNESS-IMPROVEMENT-SYNTHESIS`).

**No confirmed would-be-lost items** in this doc. One weak spot: the **3-broken-backup-scripts BUG**
(`:66–70` — `scripts/backup-{verify,restore,drill}.ts` importing deleted `apps/api/src/workers/backup`)
is now **moot** (the TS scripts + `apps/api` are deleted; the replacement is P12's native backup organ)
— genuinely obsolete, superseded by the decommission + P12. Note it as obsolete, don't carry it.

### 2.4 `MASTER-ROADMAP-10-PHASES-2026-07-14.md` — COVERED except the self-development research queue

Explicitly superseded by SOVEREIGN's header. Its P1–P8/P10 map cleanly (P1 sovereign core → P04/P13;
P2 kernel math → P04; P5 PQ envelope → P03; P6 mesh → P09/P13; P7 decide-gateway red-line → P07/P13; P8
ops → P08/P12; P10 OSS AGPLv3+TM+DCO → `BLUEPRINT-P18`, and SOVEREIGN §1.6 records LICENSE already
flipped to AGPLv3 `ac1caba40`). Mesh trust-graph Fiedler λ₂/SLEM/τ (`:82–84`) → `BLUEPRINT-P02`,
`BLUEPRINT-P11`, R1-B/D/E (14 hits).

**Would-be-lost item L6 — the self-development / growth-substrate research queue.**
`MASTER-ROADMAP-10-PHASES:88–95` (Phase P9) scopes a concrete research queue: **causal inference
(do-operator / back-door)**, **integer/overflow laws**, **category theory of the kernel↔wasm↔UI
functorial mapping**, **info-geometry of the self-improvement gradient**, plus the already-done Bayesian
calibration (`bayes_calibration.rs`). Grep across canon: `category theory|functorial|info-geometry` =
**0 hits**; `Bayesian calibration|Beta-Binomial` = 0; `do-operator|do-calculus` = 1 (unrelated
`causal.rs` context in a mesh batch doc); `overflow law` = 0. **This queue is absent from the product
roadmap entirely.**
→ **Recommendation: do NOT force it into a numeric product phase.** This is a *different axis* — it is
the operator's standing SELF-DEVELOPMENT track, and it already has a home in
`MEMORY.md` → `physics-math-exploration.md` (the "research queue: spectral graph theory, Bayesian,
causal, category theory, info-geometry; trigger-based" line). The correct consolidation action is that
the forthcoming **`CORE-ROADMAP-INDEX.md` must carry an explicit cross-track pointer** to the MEMORY
self-development corpus as a non-product, always-running axis (parallel to, not inside, P01–P30). Losing
it from `10-PHASES` is only a loss if the index doesn't point at MEMORY. Flag it so the pointer exists.

### 2.5 `MASTER-EXECUTION-PLAN-2026-07-13.md` — the omitted 6th doc — STRUCTURALLY ABSORBED

This Ukrainian-language master **index** ties 9 thematic design-plan directories + 1 executed ops track
(~151 blueprint units) into a 4-phase altitude sequence **Земля → ядро → поверхня → платформа**
(ground → core → surface → platform). Its 9 sub-plans: `ops-reliability` (22 OPS), `docker-swap`
(10 DK), `mesh-real` (14 MESH), `integration-ports` (21 IP), `field-ui-engine` (17 FE),
`rust-engine-rewrite` (12 RW), `dowiz-interfaces` (12 DZ), `ecosystem-strategy` (20 EC),
`hydraulic-loop-v2` (23) (`:61–73`).

**Coverage:** each sub-plan is broadly absorbed by the anchor-based (M/V/D/S/E/F) decomposition —
mesh-real → P09/P13 (and directly corrected by the agentic-mesh arc §7.2); ops-reliability → P08/P12;
field-ui/rust-engine/dowiz-interfaces → `BLUEPRINT-P16` + `LIVING-INTERFACE-ROADMAP`; ecosystem-strategy
→ `BLUEPRINT-P19`; docker-swap (microVM+WASM zero-OCI, WASI-p2=Scope) → SandboxTier (agentic-mesh B1) +
P09/P10, and Docker itself was **decommissioned this session** (`5675c349b`); hydraulic-loop-v2 → P15 +
self-development. The 9 sub-plan **directories are individually indexed in `MEMORY.md`** ("Active arcs
(earlier)": docker-swap-arc, field-ui-engine-arc, dowiz-interfaces-design-arc, rust-engine-rewrite-arc,
integration-ports-reactive-arc, ecosystem-strategy-arc, ops-reliability-arc, mesh-real-arc,
hydraulic-loop-v2-arc). So the **navigation survives in MEMORY**, not in the roadmap.

**No new would-be-lost content**, but two structural notes:
1. Its **4-phase altitude spine (ground→core→surface→platform) is the direct ancestor of CORE-STANDARD's
   P-A..P-I** letter axis (§4). Worth stating in the banner so the lineage is explicit, not silently
   re-derived.
2. Its cross-cutting-threads table (`:12–21`) and the "кор незмінний / capability-порти / pgrust /
   WASM-WASI-p2 / PQ / RED-discipline / reuse-first" invariants (`:90–100`) are all preserved as canon
   laws (M-series + STRATEGIC-VECTORS) — nothing unique.
→ **Recommendation:** banner it like the other four **and** add it to the CORE-STANDARD §0 inventory
(the standard currently under-inventories itself). Its value going forward is purely as the historical
index to the 9 pre-pivot sub-plan dirs — cheap to preserve via one banner line pointing at MEMORY's
"Active arcs (earlier)" section.

---

## 3. Consolidated would-be-lost ledger

| ID | Item | Source cite | Status | Fold-in recommendation |
|---|---|---|---|---|
| **L1** | Node self-update **code-signing** (ML-DSA vs pinned root, `codesign.rs`) | MVP `:41,68,89` | **LOST** (only incidental mentions) | **Phase 10** (boot/update integrity) or explicit E53 deferral |
| **L2** | Transport bake-off **rationale** (Zenoh/Reticulum/TCPCLv4/libp2p-rejected/BIBE) | MVP `:76–78` | **WEAK** (decision in code, rationale orphaned) | Cross-link `docs/transport-research-2026-07-12.md` from `BLUEPRINT-P09` |
| **L3** | Courier **out-of-app notification** send-path (`notify.rs`/NotifyHub/N1–N2/VAPID) | BUILD-SEQ `:132–141` | **LOST** (1 incidental) | **Phase 13** courier-leg unit, xref P08 alerting; or "dissolved-by-mesh + wake sub-unit" |
| **L4** | Anonymous **`.onion`/Tor tier** | BUILD-SEQ `:88` | **LOST** (0 hits) | E53 backlog entry (trigger: vendor-node tier + anonymity demand) |
| **L5** | "**Lost reports**" honesty ledger (13 + ~20 reports) | BUILD-SEQ `:152–162`; 10-PH `:104` | **LOST** (0 hits) | One line in consolidation record: closed-as-lost, decisions survive in UDP-v3; no resurrection |
| **L6** | Self-development **research queue** (causal / category-theory functorial / info-geometry / integer-laws) | 10-PH `:88–95` | **LOST from roadmap** (0 hits for category/info-geometry) | **Do NOT force into P01–P30.** Point `CORE-ROADMAP-INDEX.md` at MEMORY `physics-math-exploration.md` as a separate always-running track |

Genuinely-obsolete (superseded by a real decision, correctly *not* carried): the entire Node/TS
Tier-0..Tier-5 backlog (deleted stack, `5675c349b`); the 3-broken-backup-`.ts`-scripts BUG (deleted
`apps/api` + P12 native organ); Docker/microVM launch specifics (Docker decommissioned).

**Six would-be-lost items total; two (L1, L3) worth folding into a phase, four minor (cross-link /
E53 / one-line note / index-pointer).** None is a hidden architectural decision that would break the
roadmap — the consolidation is safe once these six are dispositioned.

---

## 4. Numbering resolution — P-A..P-I vs the numeric phases (operator: "no confusing double-numbering")

**Finding: there are three numbering systems, and P-A..P-I is an orthogonal AXIS, not a renumbering.**

1. **Numeric execution phases `P01–P30`** — the canonical build units. `P01–P19` are numbered blueprint
   files in `sovereign-roadmap-2026-07-16/`; `P20–P30` are standalone `BLUEPRINT-*-2026-07-17.md` files
   indexed from SOVEREIGN §8.1–§8.12. Dependency-ordered, wave-scheduled (SOVEREIGN §2). **This is the
   single canonical execution numbering.**
2. **Letter layer-phases `P-A..P-I`** (`CORE-ROADMAP-STANDARD` §3) — a **thematic altitude axis**
   (core-kernel → product → consolidation). Each letter **absorbs a *cluster* of numeric phases +
   masterwork batches + standalone blueprints**, explicitly *not* 1:1. Proof from the standard's own
   "Absorbs" column: **P-A** (core kernel primitives) cuts across P04/P11/P28 + eqc + eigenvector +
   cache-tensor-arena; **P-D** (consensus/trust) cuts across P03/P06/P10 + masterwork Batch 4/6/7;
   **P-G** (product/UI) = P16 + LIVING-INTERFACE; **P-I** (this audit) = the consolidation meta-task,
   which maps to **no numeric phase at all**. A letter that maps to "many numeric phases" and one that
   maps to "zero numeric phases" cannot be a renumbering — it is a cross-cutting lens.
3. **Legacy tier spines** — MVP's T0–T5, EXECUTION-PLAN's 4-phase ground→core→surface→platform,
   10-PHASES' P1–P10. All retiring. Note that **P-A..P-I is a direct descendant of EXECUTION-PLAN's own
   ground→core→surface→platform altitude spine** — the same idea, re-expressed.

**The collision risk the operator flagged is real and purely lexical:** "**P-D**" reads dangerously like
"**P04**/PD"; a reader seeing both `P-D` and `P04` in one corpus cannot tell they are different axes.
The Wave-1 audit filenames already in this dir (`P-D-`, `P-G-`, `P-H-`, and this `P-I-`) bake the "P-"
prefix into artifacts.

**Recommendation (concrete, single choice): keep `P01–P30` as the sole execution numbering; RENAME the
letter axis off the "P-" prefix to `Layer A..I` (equivalently `Track A..I`).** Then:
- `CORE-ROADMAP-STANDARD` §3's table header becomes "Layer A (Core kernel primitives) — absorbs
  P04, P11, P28, eqc, …" — the useful altitude grouping survives, the collision dies.
- The four Wave-1 audit files keep their on-disk names for provenance but each gains a one-line header:
  *"'P-I' here = CORE-ROADMAP **Layer I** (cross-repo consolidation), an altitude lens — NOT execution
  phase P-anything."* (This file carries that clarification in its own title context.)
- Publish a **crosswalk table** in `CORE-ROADMAP-INDEX.md`: `Layer A..I → {numeric phases + standalone
  blueprints + masterwork batches it rolls up}`. That table is the single anti-double-numbering artifact
  — it makes explicit that the letters index the numbers, never replace them.
- **Do not renumber P01–P30.** They are referenced by dozens of downstream docs, memory files, and the
  agentic-mesh/spectral arcs; renumbering them would be the actual expensive mistake.

Rejected alternative: dropping the letter axis and mapping its content straight onto the numeric phases.
It loses the altitude grouping that CORE-STANDARD's Wave-2 fan-out is organized around (write one
blueprint per layer), and P-I/Layer-I ("consolidation") has no numeric home to fold into.

---

## 5. Worktree-arc indexing check — both arcs are referenced, not orphaned

**`dowiz-agentic-mesh` (`AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md`) — INDEXED.** Referenced from SOVEREIGN
§8.1 (P21 note: "shares sequencing with the agentic-mesh arc") and §8.3 (completeness audit:
"B1/B3/B4 … the agentic-mesh arc"); listed in `CORE-ROADMAP-STANDARD` §0 ("Consolidated arc summaries");
carried in `MEMORY.md` active-arcs. Its dependencies are reconciled with the numeric roadmap: the **P07
dedup-ordering hard precondition** it rediscovered live (`event_log.rs:348` before `:297–300`) is
covered by `BLUEPRINT-P07` + masterwork W1-L2; the **0x12/0x13 discriminant collision** it found is
docketed as masterwork **R-1**; its 8 proposed canon-diffs (CD-1..CD-8) remain **operator-merge pending**
(not applied — expected). Its own §5 Q2 self-audit flags 3 R-doc findings it had dropped
(Poly-Network anchor-mutation invariant → repaired inline as CD-8; R2 §5 replication-policy and R3 §5(2)
admission-time loop analysis → both carry E53-form triggers). **Not orphaned.**

**`dowiz-spectral-evolution` (`SPECTRAL-EVOLUTION-CONSOLIDATED.md`) — INDEXED.** Referenced from SOVEREIGN
§8.3 ("E1/E2 … the spectral-evolution arc"); listed in CORE-STANDARD §0; carried in MEMORY. Its E1
Laplacian-parity finding feeds `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN` (Layer A); **E3-Phase-B is
correctly gated on P06 `key_V`** (matches the standing 3-way blocker), and that block is faithfully
represented in the roadmap (SOVEREIGN §8.4). **Not orphaned.**

**One indexing gap to fix during consolidation:** neither arc appears in the SOVEREIGN doc's **§2 phase
table** — they are referenced only in prose (§8) and in CORE-STANDARD §0. Both arcs' own consolidated
docs explicitly *requested* a MEMORY active-arcs line (each §6 Ananke) — MEMORY **does** carry both, so
that request is satisfied. → **Recommendation:** the forthcoming `CORE-ROADMAP-INDEX.md` should list the
**three** cross-cutting arc consolidated docs — agentic-mesh, spectral-evolution, **and the in-repo
`living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md`** (also in CORE-STANDARD §0, feeds Layer-G/P16,
61 KB, easy to overlook) — under an explicit "cross-cutting arcs (own consolidated docs, not numeric
phases)" heading, so no reader has to already know the worktree/dir exists to find them.

---

## 6. Concurrent-work note (per task item 6)

Substantial concurrent activity on this repo since the initiative started, materially relevant to scope:
- The roadmap **grew from P19 to P30** (SOVEREIGN §8.1–§8.12, all dated 2026-07-17) after
  CORE-ROADMAP-STANDARD was written — the standard's §3 "reconcile P01-P19" language is already stale by
  11 phases. The banner/reconcile pass must target **P01–P30**, not P01–P19.
- The **Node/TS + Docker stack was decommissioned this session** (`5675c349b`) — this is what makes most
  of BUILD-SEQUENCE and EXECUTION-PLAN's docker-swap obsolete rather than pending.
- **P19 (`f01f9bb6b`), P07 money-reversal (`a9dd2faf0`), P01 CI-truth-floor (`4b05ee588`), P18
  public-flip (`aea7955a4`), eqc Rust port (`7c7763af7`)** all landed on this branch — several older-doc
  "open" items (OSS readiness, decide-gateway red-line, money law) are now **built, not planned**,
  consistent with SOVEREIGN §8.3's "assumed-unbuilt, actually built" staleness direction.
- The git status shows `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` **modified** (uncommitted) —
  another session may be editing the canonical doc concurrently; the banner pass should re-read it at
  execution time.

None of this changes the audit's conclusion (consolidation is safe) — but it **does** change the scope:
**6 docs to banner (not 4), P01–P30 to reconcile (not P01–P19), and 6 would-be-lost items to
disposition.**

---

*P-I Wave-1 audit written 2026-07-17 on `feat/p19-growth-engine` (HEAD `f01f9bb6b`), read-only. Sources
read in full: all 6 master docs, both worktree-arc consolidated docs, SOVEREIGN §0–§8.12; the 19
numbered blueprints + 13 standalone P20–P30 blueprints + masterwork corpus verified by targeted grep.
No code, canon, or doc edited by this audit.*
