# Offline Resilience — Synthesis and Build Plan (dowiz / bebop2)

**Status: RESEARCH SYNTHESIS / PLAN — no code written, nothing built yet.**
**Date:** 2026-07-20
**Precedes:** one completed code-audit + external-patterns research pass (Opus), verified against the live tree at `/root/dowiz`. This document turns that audit into one buildable plan. It does not re-litigate the audit.
**Operator request (verbatim):** "research how the current architecture can still maintain it's funcionality even without an access to any network - my idea is to make it resilient, so even during offline mode/network issues most of the actions, interface is still working."

---

## 0. Verdict in one paragraph

dowiz's offline posture is a strength, not a deficit. The kernel commits real order-state transitions with zero network (proven by test), the reconciliation algorithm for divergent nodes is a proven CvRDT (proven by test), payment is cash-only and structurally bulkheaded from the order lifecycle, and the browser runs the full kernel in WASM so a warm session needs no server at all. The audit falsified two hypothesized gaps and found three real ones, all at the edges rather than the core: (1) the proven mesh reconciliation algorithm has no transport — nothing calls it over a wire; (2) the web shell has no offline cold-start path (no Service Worker); (3) the browser has no durable event store (a reload loses state). None of these touch kernel math. The plan below is: a small shell+persistence change in `web/` (Phase A), the first real mesh transport with partition-tolerance as a construction invariant (Phase B), and one recorded design constraint for future payment rails (no build work). One operator ruling is required before Phase A ships (§6).

---

## 1. Two different problems — CAP/PACELC framing (read this before the plan)

"Offline resilience" here is two problems with two different solutions. Conflating them produces the wrong design. Framing per Brewer (2000), Gilbert & Lynch (2002), Abadi's PACELC (2012).

**Problem A — mesh partition: one node offline while the rest of the mesh continues.**
This is the classical CAP scenario. dowiz's answer is an AP design: each node keeps accepting writes through the local Law during the partition, and nodes converge on reconnect via state-based CRDT merge. This is *already solved at the algorithm layer* — `kernel/src/mesh_replication.rs` implements a content-addressed G-Set CvRDT whose union is commutative, associative, and idempotent (the convergence laws of Shapiro/Preguiça/Baquero/Zawirski, SSS 2011, implemented by name), and the test `two_nodes_diverge_offline_reconnect_pull_identical_folded_state` demonstrates two nodes that diverged offline converging to an identical folded event set. What is missing is only the wire (§4, Phase B).

**Problem B — local availability: the owner or customer using the interface while their own hub/device has no WAN.**
This is not a consensus problem and CAP barely applies, because the authority — the kernel — is *locally present*: it compiles to WASM and executes in the browser. Availability during a WAN outage is limited only by (a) whether the shell can cold-load without network and (b) whether local state survives a reload. Both are UI-layer engineering (Service Worker + IndexedDB), not distributed-systems engineering (§4, Phase A).

**PACELC position, stated once:** dowiz on the order path is **PA/EL** — under Partition choose Availability; Else choose Latency. There is no synchronous cross-node coordination on the decide/fold path, ever; consistency is eventual and convergent by construction. This is a deliberate consequence of DECISIONS.md D0 (local-first, mesh, reliability-over-latency), not an accident to be "fixed" later with consensus.

A future reader who proposes a quorum, a lock service, or a synchronous replication step on the order path is proposing to change Problem A's answer from AP to CP, which contradicts D0. Record objections against D0, not against this document.

---

## 2. What is already solid — settled audit facts

These were verified against real code by the completed research pass. They are cited here as settled; do not re-audit before building on them.

**2.1 Offline writes through the real Law.**
`kernel/src/event_log.rs`: a node commits order-state transitions through the actual kernel `decide` fully offline. Proven by the test `write_succeeds_offline_with_kernel_decide`, whose own comment states: "No network call was ever made … This IS the offline-write property." Writes are idempotent by content-id; durability faults are typed errors, never silently swallowed.

**2.2 The transition function is pure and offline-computable.**
`kernel/src/order_machine.rs`: `decide → Event`, `state = fold(events)`. Verified: no clock, no RNG, no network, no float in the transition-decision code (MANIFESTO C2). Every node replays identically offline. Forbidden transitions are errors, not silent no-ops.

**2.3 Reconciliation after divergence is a proven algorithm.**
`kernel/src/mesh_replication.rs` (+ `hydra::FileEventStore` for host-side durability): Merkle-digest pull anti-entropy over a content-addressed event set — a real G-Set CvRDT. Proven by the test named in §1. The *algorithm* needs no further validation; only a transport (§4B).

**2.4 Payment is cash-only and bulkheaded — a hypothesized gap that is actually a strength.**
`kernel/src/ports/payment.rs`: `RailKind` has exactly one variant (cash); card/PSP rails are deliberately absent and a compile-firewall test enforces it. Cash settlement is a courier's signed attestation verified locally — zero network. Critically, settlement is bulkheaded from lifecycle progress: a rejected settle appends *nothing*; the order stays complete-with-settlement-pending. An order progresses accepted→prepared→dispatched→delivered independent of settlement completion. This is exactly the structural precondition an eventual online rail needs (§4, Gap 4).

**2.5 The browser runs the kernel, not a proxy to a server.**
`web/src/lib/kernel/kernel_client.mjs` does `WebAssembly.instantiate`; every action (place_order, apply_event, estimate_order_total, spectral/geo calls) executes synchronously client-side. `web/serve.mjs` is a pure static file server, not an application server. A **warm** session (page already loaded) keeps working with zero network today, before any of this plan is built.

**2.6 Failure-class isolation at the agent lane.**
`llm-adapters`' Ollama calls are localhost HTTP: they fail on process-down (ECONNREFUSED) — a *different failure class* from WAN loss — and are correctly isolated at the agent-lane edge, never in the order path. Preserve this taxonomy (§7): "the internet is down," "a local sidecar process is down," and "the mesh peer is unreachable" are three distinct failure classes with three distinct blast radii, and the current architecture keeps them separated. No code change needed; this is a constraint on future wiring.

---

## 3. The gap ledger

| # | Gap | Layer | Severity framing |
|---|-----|-------|------------------|
| 1 | No live mesh transport exists anywhere in the repo. `mesh-adapter/src/lib.rs:17` declares "no transport, no storage" as explicit anti-scope. Offline and online mesh behavior are currently *identical* — there is no live sync to lose. | Problem A | Not a resilience defect — a missing first build. Reframed in §4B. |
| 2 | `web/` has no offline shell: no Service Worker, no Cache API. A cold load (first visit, hard refresh) needs network to fetch HTML+WASM, even though a warm session does not. | Problem B | Real, small, user-visible. |
| 3 | `web/` has no client-side persistence: state lives in ephemeral in-memory JS objects; a reload loses everything. The durable `FileEventStore` is host-Rust only, not compiled into the browser WASM path. | Problem B | Real, small, user-visible. |
| 4 | Online payment rails do not exist yet. Not a current gap (payment is cash-only) — a *future integration point* that must not violate the existing bulkhead. | Cross-cutting | Design constraint to record, zero build work now. |

Honesty note: gaps 2 and 3 do not mean "the web app is broken offline." A warm session already works fully offline (§2.5). They mean the two specific scenarios *cold load offline* and *reload during offline* fail today. That is the precise, falsifiable scope of Phase A.

---

## 4. Build plan

Phasing rationale: Phase A is small, self-contained, immediately user-visible, and blocked only on one operator ruling. Phase B is a larger arc with an explicit dependency on the in-progress concurrency-architecture synthesis (§8) — but its first milestone (B1) is deliberately runtime-agnostic so it need not wait.

### Phase A — Browser offline shell + durable event store (Gaps 2+3)

**What it is NOT:** a re-implementation of any kernel math in JS. The kernel already runs in the browser as WASM (§2.5). Phase A is a *shell + persistence* change only. Any diff in this phase that computes money, order state, or graph math in JS is out of scope by definition and should be rejected in review.

**A1 — Service Worker, cache-first shell (~60–100 lines of vanilla JS, zero dependencies).**
- One `sw.js` in `web/`, registered from `index.html`. No Workbox, no framework — the research explicitly assessed the heavy PWA toolkit as violating the "drop js" doctrine, while the two load-bearing primitives (a small vanilla SW + IndexedDB) are near-zero-JS.
- Strategy: a versioned cache name; on `install`, precache the fixed asset manifest (index.html, the WASM binary, the wasm-bindgen glue `.mjs` files, `kernel_client.mjs` and its imports); on `activate`, delete stale caches. Fetch handler: cache-first for content-hashed/immutable assets (the WASM bundle), stale-while-revalidate for `index.html` so shell updates propagate on the next online visit without ever blocking an offline load.
- **What this buys, falsifiably:** after one online visit, a cold load or hard refresh with zero network serves the full app from cache, and the WASM kernel initializes and accepts actions. Acceptance test: Playwright, load once online → emulate offline (`context.setOffline(true)`) → hard reload → assert the storefront renders and `place_order` succeeds through the WASM kernel.

**A2 — IndexedDB event-store binding (~100–150 lines, zero dependencies).**
- Role: the browser-side mirror of `FileEventStore`'s role — a durability adapter, nothing more. One object store `events`, keyed by the kernel-computed content-id, value = the canonical serialized event bytes as produced by the kernel. Puts are idempotent by key, matching the event log's content-id idempotency (§2.1).
- Boot path: on page load, read all persisted events and feed them through the existing `apply_event` surface so folded state is rebuilt by the kernel's own `fold` — the JS layer never interprets event contents.
- Write path: every kernel-accepted event is appended to IndexedDB *before* the UI treats it as committed-local (§5.3). An IndexedDB write failure is surfaced as an explicit typed failure in the UI, mirroring `event_log.rs`'s typed-durability-fault discipline — never silently dropped.
- Forward compatibility, stated but not built: this store is the same content-addressed set that `mesh_replication.rs` reconciles. When Phase B's transport later reaches the browser, the IndexedDB store is the natural local replica; nothing in A2's schema should preclude that (keying by content-id already guarantees it).
- **What this buys, falsifiably:** a reload — online or offline — restores every previously committed order to the exact folded state. Acceptance test: place order offline → reload → assert the order is present with identical state.

**A3 — Operator precedent flag (required before merge — see §6).**
Phase A introduces JS (a Service Worker and an IndexedDB adapter) into `web/` after the 2026-07-15 "drop js" ruling. The sibling AR/voice blueprint's O3 ruling already established one precedent: vendoring `<model-viewer>` as the first JS component since the drop. **Recommendation:** the SW + IDB adapter is a closer fit to the doctrine's spirit than `<model-viewer>` was — it is infrastructure/shell code with zero external dependencies and zero application logic (the kernel remains the only math authority; `web/` continues to "only render" plus now *cache and persist*). I recommend treating it as inside the same exception class. **But this is a recommendation, not a decision:** it needs explicit operator ratification before A1/A2 land, because "drop js" is doctrine, not preference, and exceptions to doctrine must be enumerated, not inferred.

### Phase B — First mesh transport (Gap 1)

**Reframing — read this first.** This is not "make the mesh resilient offline." There is no live sync today, so offline and online mesh behavior are identical; resilience-under-partition is currently a property of an unbuilt system and is not yet measurable. The correct framing is: **build the first transport, with partition-tolerance as a construction invariant from day one** — cheaper and safer than any retrofit, and the AP/CvRDT layer underneath makes it nearly free (B3).

**B1 — Minimum wire: two processes, one real socket, the existing algorithm (no new algorithm work).**
The smallest thing that makes `mesh_replication.rs`'s proven logic reachable over a wire:
- Two OS processes (not threads — two processes prove there is no shared-memory shortcut), each with its own `FileEventStore`.
- One real socket: TCP on localhost or a Unix domain socket, via `std::net` / `std::os::unix::net`. **Pure std, blocking I/O, no async runtime** — a two-node integration test needs none, and choosing blocking-std here deliberately defers the runtime question to the concurrency synthesis (§8) without blocking this proof.
- A minimal framing protocol: length-prefixed frames, exactly three message types — (1) digest exchange, (2) missing-content-id request, (3) event bytes. The transport moves bytes; `reconcile` does everything else.
- **The first integration test is the existing in-process proof promoted to the wire:** diverge the two stores offline (disjoint writes through the real Law), connect the socket, run pull anti-entropy for real, assert identical folded state on both sides. Working name: `two_processes_diverge_offline_reconnect_over_socket_identical_folded_state`. RED→GREEN per repo culture: the test exists and fails (no transport) before the transport lands.
- One additional test to pin semantics, flagged by the AP framing itself: **concurrent same-order divergence.** Both partitions act on the *same* order while separated (e.g., conflicting transitions); after merge, fold over the union is a deterministic total function, so all nodes agree on the outcome — the test's job is to pin *what that outcome is* and document it, so conflict semantics are a stated invariant rather than an emergent surprise. This is a test-to-write, not a discovered defect: the audit proved set-convergence and folded-state identity on the disjoint-divergence scenario; this pins the overlapping-divergence scenario.

**B2 — Where it lives.**
`mesh-adapter`'s "no transport, no storage" anti-scope (`mesh-adapter/src/lib.rs:17`) is a deliberate, documented boundary. Do not silently violate it. Two clean options: (a) a new standalone crate (e.g., `mesh-transport/`, path-depending on `kernel`, per the repo's no-workspace crate pattern), leaving `mesh-adapter`'s anti-scope intact; or (b) deliberately revise the anti-scope with a documented rationale. Recommend (a) — it preserves the existing boundary as written and matches the repo's crate-per-seam structure. Per feature discipline (CLAUDE.md), anything beyond pure-std goes behind an off-by-default feature; B1 as specified needs nothing beyond std.

**B3 — Resilience properties designed in from day one (why the transport gets to be simple).**
Because the state layer is a CvRDT, the transport can be *dumb* and still be partition-tolerant. Bake these in as invariants, each testable:
1. **Restartable at any point:** an interrupted reconciliation leaves both stores valid — union is monotone and idempotent by content-id, so partial transfer is safe *by construction*. Test: kill the connection mid-transfer, reconnect, converge.
2. **No session state that must survive disconnect:** every reconnect starts from a fresh digest exchange.
3. **Order-independence:** frames may arrive/apply in any order (commutativity of union). No sequence numbers on the sync path.
4. **Typed transport failures:** connection loss/refusal surfaces as typed errors to the caller, matching `event_log.rs`'s durability-fault discipline — never a silent retry loop, never a swallowed error.
5. **Failure-class labeling:** transport errors carry which class they are (peer-unreachable vs local-socket failure), preserving the §2.6 taxonomy.

**B4 — Explicit dependency: the concurrency-architecture synthesis.**
A separate synthesis from this same session (`docs/design/CONCURRENCY-ARCHITECTURE-SYNTHESIS-2026-07-20.md`) — covering native-concurrency-vs-tokio, exokernel-inspired minimalism, and the end-to-end principle — landed the same day as this document. **A real mesh transport is the one surface in dowiz where genuinely high peer-connection-count concurrency could actually apply** (unlike the single-hub-process LLM/voice surfaces, which that sibling synthesis found do not need tokio's scale) — and that synthesis's §5.4 already recommends exactly this crate be the tokio entry point, gated on a measured concurrent-peer-socket threshold. This document deliberately does not re-resolve the runtime question. Sequencing: **B1 is runtime-agnostic (blocking std, 2 peers) and may proceed now; the N-peer daemon (call it B5, not specified here) follows the concurrency synthesis's §5.4 gate**, so the two documents stay consistent. That doc governs the runtime choice; this doc governs the reconciliation/resilience invariants (B3), which hold under any runtime.

### Gap 4 — Future online payment rails: a recorded design constraint, not work

Nothing to build now. Payment is cash-only by deliberate design (§2.4). But when an online rail (card/PSP) is eventually added, the integration must satisfy the **transactional-outbox + idempotency-key discipline** (Pat Helland, "Life Beyond Distributed Transactions," CIDR 2007, rev. 2016) — and dowiz already has every structural precondition, so this is "keep doing what you're already doing," recorded so a future payment-adapter task cannot accidentally violate it:

1. **The decide/fold path never makes a network call** (MANIFESTO C2 — already true; PSP calls are adapter-side, fed *from* the event log as an outbox, never inline in the Law).
2. **Every outbound PSP call carries an idempotency key** derived from kernel-owned identity (dowiz already has the right primitives: settlement idempotent by `order_id`, events idempotent by content-id). This is the safeguard against double-charge races when a request outcome is unknown (timeout, partition) and must be retried.
3. **The bulkhead is preserved:** rail failure never blocks lifecycle transitions — an order completes with settlement-pending exactly as the cash path does today ("a rejected settle appends NOTHING"). Any payment-adapter design in which delivery waits on a PSP response violates this document and §2.4's verified structure.
4. **PSP outcomes re-enter as events through `decide`**, never as direct state mutation — the outcome of an external call is a fact to be folded, subject to the same idempotency and replay guarantees as every other event.

---

## 5. Degraded-mode UX — concrete, falsifiable design

Grounding: Nielsen's Visibility of System Status heuristic and NN/g offline-UX guidance (be explicit that changes are saved locally and will sync — never silently pretend nothing changed), implemented via the Google Docs/Figma pattern: a persistent low-alarm status chip plus per-item pending markers, not modals, not toast storms.

**5.1 Connectivity state machine — lives in the JS shell, never in the kernel.**
The kernel has no clock and no network by design (C2); connectivity is a shell concern. Three states: `ONLINE`, `OFFLINE`, `SYNCING`. Transition triggers, specified falsifiably:
- `navigator.onLine === false` and its events are a *hint only* (known to false-positive on captive portals and interface-up-but-no-route conditions). The **authoritative** signal is the outcome of real requests: any failed sync/asset/probe fetch ⇒ `OFFLINE`; a subsequent successful probe ⇒ `SYNCING` (if unsynced events exist) or `ONLINE`.
- While `OFFLINE`: probe with capped backoff (retry at 15 s, doubling to a 60 s cap). While `SYNCING`: transition to `ONLINE` when the unsynced-event count reaches zero.
- (Until Phase B's transport reaches the browser, "sync" for the storefront means asset revalidation only; the state machine and chip are still built in Phase A so the contract exists before the transport does.)

**5.2 The status chip — exact contents per surface.**
A single persistent chip, fixed position, no animation beyond state change:

| State | Customer storefront | Owner hub console |
|---|---|---|
| `ONLINE` | No chip (absence is the signal; matches Docs/Figma low-alarm default) | Neutral dot, no text |
| `OFFLINE` | "Offline — your actions are saved on this device" | "Offline — N events pending sync" (N = live count of unsynced events in IndexedDB) |
| `SYNCING` | "Reconnecting…" | "Syncing… N remaining" (N counts down) |

**5.3 Per-action semantics — three outcomes, not "optimistic UI."**
Because the kernel is local, offline actions are not optimistic guesses awaiting server confirmation — they are **locally authoritative**: `decide` runs the real Law, the event is durably appended, folded state updates immediately. The UI distinguishes exactly three outcomes:

1. **Committed-local:** `decide` accepted, event durably in IndexedDB, not yet replicated to any peer. Render fully and normally, with one subtle pending-sync marker on the affected order card (a small glyph, not a spinner, not dimming — the state is real and final locally). Marker clears when the event is known-replicated.
2. **Rejected-by-Law:** a forbidden transition. Immediate error, **byte-identical to the online error**, because the Law is local and pure. *Falsifiable invariant: the error shown for a forbidden transition must be identical online and offline. A divergence is a bug.*
3. **Failed-durability:** the IndexedDB append failed. Explicit failed-and-needs-retry UI on that action (error state + retry affordance), never silent — mirroring `event_log.rs`'s typed durability faults. The action is *not* shown as committed.

**5.4 Honesty about the true offline boundary.**
Enumerate, in the UI's own copy, what works offline and what genuinely cannot: placing orders, all state transitions, price estimates — local (WASM kernel), fully functional. Cross-node visibility — a courier on *another node* seeing the order — genuinely requires the mesh. The owner console therefore shows, on offline-created orders: "will reach couriers when connection returns," and never fakes dispatched-to-network status. Pretending otherwise would violate both the NN/g guidance and the repo's verified-not-claimed culture.

**5.5 Acceptance tests (Playwright, offline emulation via `context.setOffline(true)`).**
- **T1 (warm offline action):** load online → go offline → place order → order appears committed-local with pending marker; chip shows the offline text; forbidden transition shows the identical error as online.
- **T2 (cold load offline):** visit once online → close → go offline → open URL → app loads from SW cache and accepts a `place_order` (Phase A1's proof).
- **T3 (reload persistence):** place order offline → hard reload while offline → order present with identical folded state (Phase A2's proof).
- **T4 (reconnect):** restore network → chip transitions `OFFLINE → SYNCING → ONLINE`; pending markers clear; N counts to zero.

---

## 6. Operator decision points (blocking / non-blocking)

> **RULED 2026-07-20 — see `DECISIONS.md` D14.** Item 1 (Service Worker + IndexedDB doctrine
> exception) is ratified; Phase A is unblocked. Items 2–3 proceed per this section's own stated
> recommendations (not escalated).

1. **[Blocking Phase A] Service Worker + IndexedDB vs the "drop js" doctrine.** Recommendation in §A3: treat as infrastructure/shell code inside the exception class the AR/voice blueprint's O3 ruling opened with `<model-viewer>` (this candidate is *more* conservative: zero external deps, zero application logic, kernel remains sole math authority). **Needs explicit ratification; do not infer it.**
2. **[Non-blocking, decide before B lands] Transport crate placement.** Recommendation in §B2: new standalone `mesh-transport/` crate, preserving `mesh-adapter`'s documented "no transport, no storage" anti-scope as written.
3. **[Non-blocking] Sequencing.** Recommendation: Phase A first (small, user-visible, gated only on decision 1), B1 in parallel or after (runtime-agnostic, gated on nothing), B's N-peer daemon after the concurrency-architecture synthesis's §5.4 gate.

---

## 7. Failure-class taxonomy (preserve; no work)

Three classes, currently well-separated; future wiring must not merge them:
- **WAN/internet loss** — the subject of this document. Blast radius after Phase A: cross-node visibility only.
- **Local sidecar process down** — e.g., `llm-adapters`' Ollama at localhost (ECONNREFUSED). Correctly isolated at the agent-lane edge behind the P40 compile firewall; never in the order path. A WAN outage must not be reported as an LLM failure or vice versa.
- **Mesh peer unreachable** — Phase B's domain; a per-peer condition, not a global one, and (per B3) never an error state for local operation — only for that peer's convergence lag.

---

## 8. Cross-references (consistency contracts with sibling threads — not restated here)

- **`docs/design/CONCURRENCY-ARCHITECTURE-SYNTHESIS-2026-07-20.md`:** governs the async-runtime question for Phase B's eventual N-peer daemon (§B4). This document deliberately keeps B1 runtime-agnostic so nothing here pre-empts that synthesis. That doc owns the runtime choice; this document owns the reconciliation/resilience invariants (§B3), which are runtime-independent.
- **`docs/design/BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md`, O3 ruling:** source of the `<model-viewer>` "first JS since the drop" precedent invoked (not extended unilaterally) in §A3/§6.
- **Space-grade kernel roadmap and `kernel/src/mesh_replication.rs`:** §2.3's algorithm was built earlier this same session; this plan consumes it as-is and adds no algorithm work.

---

## 9. Summary of deliverables and their proofs

| Item | Deliverable | RED→GREEN proof |
|---|---|---|
| A1 | Vanilla Service Worker, cache-first shell in `web/` | T2: cold load offline succeeds after one online visit |
| A2 | IndexedDB event-store adapter mirroring `FileEventStore`'s role | T3: offline reload preserves folded state; typed failure on IDB write error |
| A (UX) | Connectivity state machine + status chip + three-outcome action semantics | T1, T4; forbidden-transition error identical online/offline |
| B1 | `mesh-transport` two-process socket harness (pure std) | `two_processes_diverge_offline_reconnect_over_socket_identical_folded_state`; interrupted-transfer restart test; concurrent same-order divergence semantics pinned |
| Gap 4 | This section recorded as a standing design constraint (§4, Gap 4) | Enforced at review time on any future payment-adapter diff; no code now |

Nothing in this plan modifies `kernel/src/order_machine.rs`, `kernel/src/money.rs`, or any red-line path. Phase A touches only `web/`; Phase B adds a new crate. The kernel's offline authority — the thing that makes this whole plan cheap — is already built, already tested, and is the reason "most of the actions, interface still working offline" is an engineering completion task rather than an architecture change.
