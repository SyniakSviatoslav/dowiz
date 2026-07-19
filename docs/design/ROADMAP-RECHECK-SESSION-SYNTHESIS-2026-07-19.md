# Roadmap Recheck — Session Synthesis (2026-07-19, afternoon pass)

> **Planning document — writes ZERO product code, touches no branches, pushes nothing** (except
> the small factual doc-staleness corrections it and its source passes made along the way, per
> this corpus's own established discipline — same class of edit as prior session-synthesis docs).
> Consolidates the "recheck what's missing or not done" pass run this session across three
> parallel investigations (dowiz ground-truth/ledger, memory active-arcs vs git, bebop-repo
> status) plus the G2–G5 meta-gap-audit remediation and a P06-status correction found along the
> way. Follows the same pattern as `ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md` — a patch doc,
> not a new master roadmap.

**Standing caveat for this whole doc: the repo had confirmed ACTIVE CONCURRENT WRITERS during this
pass** (git author "Hermes Agent" landing real product code — `kernel/src/decision/import.rs`,
commit `1e1a7db09` — and absorbing separately-staged doc edits mid-session). Everything below was
re-verified against live `git log`/`git status` at check time, not assumed from memory or from
other docs. Treat this doc's own findings as a snapshot, same as every other status doc in this
corpus — re-verify before citing as current fact if much time has passed.

---

## 1. Meta-gap-audit G2–G5 — all four now closed

Source: `META-GAP-AUDIT-2026-07-19.md` (G1–G17), re-surfaced as still-open in
`ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md` §5–6. Cross-checked against live file state:

- **G1** (P95/P96 orphaned from ledger) — closed, pre-session (`e7cfd0f1b`).
- **G2** (P86/87/88 WebGL2/CPU-floor DoD line) — closed, pre-session (`e7cfd0f1b`); this session
  added the missing Q1-gate cross-reference to all three.
- **G3** (P75/P81/P82 bench-gate ownership hole) — closed, pre-session (`e7cfd0f1b`); this session
  added the P75→P81/P82 cross-reference and a Q1 cross-reference on the D3/D4 rows.
- **G4** (P91 ACVP vector prerequisite) — mostly closed pre-session (`e7cfd0f1b`, real
  `usnistgov/ACVP-Server` source cited); this session closed the remaining gap — the port-source
  commit pin, verified as a real SHA (`f38f2c5` in `/root/bebop-crypt`, confirmed via
  `git log --follow`, reachable from `openbebop/main`) — matching P85's existing pinning discipline.
- **G5** (P85/P91/P92 constant-time review line) — **the one genuinely new gap this wave** — closed
  this session in all three files, with concrete named attack sites (`basemul_kem`/`ntt_kem`/
  `ntt_inv_kem` for P85 and P91; `recv_fastpath`'s AEAD tag compare for P92), a new non-optional
  `D-CT` DoD row in P91, and cross-references to the existing bebop2 `dudect` gate as prior art
  rather than inventing new machinery.

**Root cause of the false "still open" read:** `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md` §5 was
written without re-verifying `e7cfd0f1b`'s prior remediation against current file state — it
re-surfaced the *original* META-GAP-AUDIT findings as still-actionable without checking whether
they'd already been patched. This is the same staleness class as the P06 finding below (§3) and
as GAP-A1 (§2): **"gap found" and "blocker present" language in this corpus does not
self-update when the underlying condition resolves — every status claim needs re-verification
against live state, not just its own prior text.**

## 2. GAP-A1 (un-homed arc-unit disposition) — already closed, predates this session

`ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md` §2 listed this as the one genuine category-(a)
coverage gap. Verified: `GAP-A1-DISPOSITION-AUDIT-2026-07-19.md` and
`mesh-real/BLUEPRINT-MESH-14-DISPOSITION-2026-07-19.md` both exist and are committed
(`06ac90c65`, `63a1cb364`), predating this session's swarm-safety-arc work. `CORE-ROADMAP-INDEX.md`
§7/§9 already cross-reference both correctly. No action needed here beyond noting the
`ROADMAP-BLUEPRINT-GAP-AUDIT` doc's own §0/§2 headline is stale in the same direction as §5 — it
was accurate at authoring time, not at read time.

## 3. P06 `key_V` — CLOSED, and its ripple effects need re-reading everywhere it's cited as a blocker

`P06 key_V` (`HybridSigner`) closed **`58987d79d`** (2026-07-18), survived the later merge wave
(`5a97e1f6f`, `dc8d3d234`), and now has real code consumers: `kernel/src/decision/import.rs`
(landed `1e1a7db09`, this session, mid-pass) and `tools/ci-truth/{main,v1}.rs`. This corpus cites
P06 as a "cross-cutting blocker" in several places that need the same correction pattern applied
here to `BLUEPRINT-P-F-local-ai-mesh.md` (§4.2 and its worker-step scope note, both corrected this
session — the "signed import-verdict... blocked until key_V lands" language was stale; corrected
to "no longer blocked, not yet built — a legitimate follow-up, not excluded scope"):

- **`CORE-ROADMAP-INDEX.md` §1 crosswalk** (the "Cross-cutting blocker: P06 key_V gates Layer C's
  independent-verification leg, Layer G's product-safety story, E3-Phase-B, P30's signed
  DecisionUnit import" note) — **stale, corrected in this pass** (see the index diff itself).
- **E3-Phase-B** (spectral-energy-flow arc) — blocker dissolved, but **the work itself remains
  completely unstarted** (verified: zero `HarnessConfig`/self-harness-loop code in-tree). Corrected
  in `SPECTRAL-EVOLUTION-CONSOLIDATED.md` (2 spots) by the Recheck-B pass this session — the
  "zero code hits outside docs" precondition-claim was stale (P06 now has real code hits), but E3
  itself is not accidentally-already-done — it's a real, cleanly-unblocked, not-yet-picked-up item.
- **Layer C / Layer G** — not deep-verified this pass (out of scope); flagged for a future pass,
  not asserted either way.

**The general lesson, stated once so it doesn't need re-deriving:** anywhere this corpus writes
"blocked on P06" / "blocked on key_V", that clause is now false. The *consumer* work each such
clause gates is not automatically done — only the blocker is gone. Each site needs its own
"unblocked, still not built" correction, not a blanket "P06 is done, ignore all P06 mentions."

## 4. Active-arcs recheck (memory NEXT-items vs. live git) — 4 arcs checked

| Arc | Memory claimed | Verified status |
|---|---|---|
| **agentic-mesh-protocol B2** | open (0x12→0x13 discriminant ruling + operator budget/money-leg decision needed) | **confirmed still open** — `scope.rs` header still marks `0x12` UNRATIFIED; no `WorkReceipt`/`Settlement` types exist |
| **agentic-mesh-protocol B3** | open (envelope + ledger halves) | **confirmed still open** — `TokenBucket` still only `new`/`try_acquire`/`available`; no `ExposureLedger` type exists anywhere |
| **agentic-mesh-protocol B1** (Wasmtime fuel wiring) | open | **partially stale** — `FuelTrancheRunner` IS wired into `AgentDispatcher::dispatch` (`955460008`, 2026-07-17, tests green); what remains is real-hardware fuel calibration — `FUEL_PER_UNIT` in `admission.rs` is still a literal `PLACEHOLDER — pending B4`. Memory's blanket "open" undersells the landed wiring |
| **agentic-mesh-protocol E3-Phase-A** | open | **confirmed still fully unbuilt** — zero self-harness-loop code in-tree, blueprint-only |
| **spectral-energy-flow E3-Phase-B** | "should be unblockable, re-verify" | **blocker confirmed dissolved (§3 above); work itself confirmed unstarted** — corrected in-doc |
| **math-first S0.5 eigensolve bench** | "NEXT" | **stale-now-resolved** — `kernel/examples/bench_hh.rs` landed with the Householder eigensolver (`bacea08fe`, 2026-07-16); the related eigen-surface consolidation also independently DONE (`03ac0fefe`, tracked under a different memory arc, never cross-linked back to the S-numbering — no doc edit made, not a false claim, just an unlinked one) |
| **hydraulic-loop-v2 resonator.rs unlock** | "1 line to unlock" | **stale-now-resolved** — the `#[cfg(feature="host")] pub mod resonator;` line is present on bebop-repo `main`; 544-line module, 6 tests, compiles clean. Corrected in `HYDRAULIC-LOOP-v2-PLAN.md` |

## 5. Dowiz not-done items GROUND-TRUTH's 4-item summary silently dropped

`GROUND-TRUTH-2026-07-19-FINAL.md`'s "only 4 not-done items" claim (P18, secrets-scrub, bebop C3/
P85, branch cleanup) is narrowly true for its own stated scope but incomplete as a *full*
not-done list. Confirmed genuinely open, no `feat/` branch exists for any:

- **M1** — an RFC-5705 exporter-binding fix; an **open red-team correctness bug** on the live mesh
  path (dowiz-side). Flagged here for visibility — this is a correctness/security item, not a
  documentation gap, and deserves operator attention independent of this doc corpus's own hygiene.
- **P95** — living-memory BM25 index persistence. No blocker of any kind (not bebop-gated, not
  GPU-gated) — simply not yet built. The cleanest "just go build it" item surfaced by this pass.
- **P84** — golden state-digest gate, correctly reserved pending operator ruling (money/FSM
  red-line surface) — not a gap, a deliberate hold.
- **P86/P87** — GPU SlotArena/2-bit work, correctly gated on the still-outstanding OD-11 decision
  (P38 §4.2) — not a gap, a deliberate hold.
- **P76, P78, P82, P92, P93, P94** — bebop/mesh-side, gated behind the bebop NTT-wiring freeze
  (§6 below) — arguably already covered by GROUND-TRUTH's bebop item, just not itemized.
- **OD-1 / OD-2 / OD-4** — outstanding operator rulings on P90 (GCRA adoption; branch-push policy).
- One test-count drift noted and dismissed as noise: kernel shows 893 passed / 0 failed / 3
  ignored now vs. GROUND-TRUTH's recorded 894 — within normal baseline movement given concurrent
  commits, not a regression worth chasing.

## 6. Bebop-repo status (separate repo, `/root/bebop-repo`, live remote `openbebop`)

- **C3** (constant-seed ML-DSA/ML-KEM keygen ungated in prod) — **closed** 2026-07-14; its CI
  guard script was re-hardened again today (`1b90803`).
- **C4b** (nonce leak, memory-flagged HIGH) — **closed**, not open. Fixed 2026-07-17
  (`7af7496`/`94f7184`, branch-free masked bignum), hardened 2026-07-18 with a cycle-accurate
  dudect Welch-t gate (`dc7ad51`/`6f56e58`/`d3d4d8c`; real |t|≈2.4–2.8 vs. mutant-leak |t|≈218).
  P06's real signer merge (`d9c4bff`) explicitly required this closed first. **The 2026-07-14
  memory note flagging this HIGH is stale — correct it if it resurfaces.**
- **C6b** — genuinely still open (low-priority test-fixture hygiene), no fix commit found.
- **The real "frozen lane"** (what GROUND-TRUTH's "bebop C3/P85" parenthetical actually points to,
  once disentangled — bebop has no P85 of its own; the label is a dowiz-side cross-reference) is
  the **NTT hook**: commit `986646a` (2026-07-18, pushed to `openbebop/main`) landed a correct,
  exhaustively-proven FIPS-203 NTT for ML-KEM but **explicitly left it unwired** — live keygen/
  encaps/decaps still run the old schoolbook `poly_mul`, "left for explicit sign-off." **This is
  genuinely still open**, matching dowiz's own P85 blueprint's premise.
- **⚠ Live signal at check time, unresolved as of this writing:** bebop-repo had an **uncommitted
  working-tree diff deleting the entire 220-line NTT block** from `pq_kem.rs`, in progress, on
  branch `perf/bus-contention-2026-07-18` (unrelated to crypto/NTT by name). Not touched by this
  pass (out of established trust/edit scope for bebop-repo this session) — flagged directly to the
  operator at discovery time. **Re-check bebop-repo state before treating the NTT-wiring item above
  as still-accurate** — it may have been resolved (wired or deliberately abandoned) or may still be
  mid-flight since this snapshot.
- **Dependabot** — GraphQL shows `vulnerabilityAlerts.totalCount: 0` on `OpenBebop` (REST endpoint
  403'd on token scope, so this is reasonably-but-not-100%-confirmed).

---

## 7. What changed on disk this pass (all staged, see `git diff --cached`)

- `BLUEPRINT-P86/87/88`, `BLUEPRINT-P75/81/82`, `BLUEPRINT-P91`, `BLUEPRINT-P85`, `BLUEPRINT-P92` —
  G2–G5 closures (§1).
- `BLUEPRINT-P-F-local-ai-mesh.md` — P06-status correction (§3).
- `MASTER-STATUS-LEDGER-2026-07-19.md` — status-update note on 10 superseded/merged rows (Recheck A).
- `SPECTRAL-EVOLUTION-CONSOLIDATED.md` — E3-Phase-B precondition correction (Recheck B, §3/§4).
- `HYDRAULIC-LOOP-v2-PLAN.md` — resonator.rs status correction (Recheck B, §4).
- `CORE-ROADMAP-INDEX.md` — P06 crosswalk correction, Meta-gap-audit row updated to closed, new row
  added for `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md`, new row added for this doc (see its own diff).

No product code was written or modified by this pass. No branches touched, nothing pushed.
