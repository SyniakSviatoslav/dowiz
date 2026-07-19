# Batch/Coalescing Scan — Function-Call/RPC Coalescing & Event-Processing Batching

**Date:** 2026-07-19
**Scope:** dowiz/DeliveryOS kernel + bebop-repo (bebop2). Research-only; no code written, no branches touched.
**Sibling scope (settled, NOT revisited here):** single-item crypto/money batch-verify (B4 Ed25519 walk-back; ML-DSA batch).
**Method:** live `Read`/`Grep` against the working tree. The repo has been rewritten to Rust since the 2026-06-14 Repowise index — the old Node.js `apps/api` TS DB layer no longer exists on disk; there is no TS `server.ts`/`orders.ts`/`packages/db` anymore. Findings below are against the *current* Rust source.

---

## TL;DR — both angles are HONEST-NEGATIVE

Neither request-coalescing (DataLoader/N+1) nor event-tick batching has a real, measurable target in this codebase. Two independent reasons, both load-bearing:

1. **The architecture is already batch-oriented exactly where batching matters.** The WASM analytics entrypoints ingest a whole event *array* per call; bebop's mesh sync ships all missing events as ONE batched, Merkle-diffed `SignedFrame` (anti-entropy), not frame-by-frame. The naive "you should batch this" recommendation is already implemented.
2. **Realistic volume is tiny for a logistics platform.** Documented capacity is ~0.5 orders/sec *system-wide* at the optimistic pilot ceiling, ~33 events/sec at a hypothetical 100-location ceiling, and the design docs explicitly say a per-event commit is "unmeasurable... still fine at 1000× that." Batching is a throughput optimization; there is no throughput problem to solve. Adding a batch tick would *add* latency (queue-and-flush delay) for zero measurable win, and would fight the single-writer / durable-insert-then-set_tip determinism.

---

## Documented volume (the deciding evidence)

| Source | Number |
|---|---|
| `docs/design/DELIVERY-FLOWS-BACKEND-AUDIT-AND-MULTITENANT-TESTING-2026-07-17.md:46` | one hub peak **~2–4 orders/min ≈ 300–600 orders/day ≈ ~5k lifecycle events/day** |
| `docs/design/DELIVERY-EDGE-CASES-AND-DETERMINISTIC-INVENTORY-2026-07-17.md:48` | commit is ~µs; "at 4 orders/min it is **unmeasurable; it would still be fine at 1000× that**" |
| `docs/design/golive-remediation/proposal.md:53` | pilot = 1 tenant, ~3–10 locations, **~0.5 orders/sec system-wide** at the optimistic ceiling; "the pool wedge is NOT a throughput problem" |
| `docs/design/WEB3-SYNTHESIS-INVISIBLE-AGENTIC-LOCAL-INFRA-2026-07-17.md:255` | hypothetical 100 locations × 2 orders/min × ~10 events/order **≈ 33 events/s peak, ~1 KB each** |
| `docs/design/BLUEPRINT-EVENT-DRIVEN-ORCHESTRATOR-2026-07-17.md:296` | rejects Temporal/Restate as "heavyweight for **≤200 events/day**" |
| `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P34-mesh-kernel-wiring.md:602` | ML-DSA-65 verify at ~10 frames/order, 1000 orders/day **≈ 0.12 verifications/sec** |

A DataLoader/coalescer earns its keep at hundreds–thousands of calls per request or thousands of QPS. Nothing here is within three orders of magnitude of that.

---

## Angle 1 — Function-call / RPC coalescing (N+1 / DataLoader)

**Verdict: no real target.** There is no hot path issuing repeated single DB/external/verify calls in a loop that a batch API would help.

- **No N+1 DB layer exists to batch.** The kernel is dependency-light and does *zero* DB round-trips inline: persistence sits behind the `EventStore` trait (`kernel/src/event_log.rs:182`) and `MemoryStore` trait (`kernel/src/retrieval/memory_store.rs:26`), with the real Postgres/pgrust adapter documented as living in the `deploy/` service tree (`event_log.rs:9-20`). `deploy/` contains no Rust query loops. The one sqlx-backed store (`memory_store.rs:142-220`, feature-gated) already does its multi-row read as a **single** `SELECT ... ORDER BY key` and folds in memory (`:209-220`) — that is the batched form, not an N+1.
- **Every in-memory loop I inspected is CPU over a small owned collection, not a call-per-iteration.** e.g. order-total fold `json_api.rs:166` (`for it in &items`), analytics folds `analytics.rs:142/150`, settlement sums `ports/payment.rs:283/297/706`, catalog checks `catalog.rs:791/832`, retrieval index/PPR/diffusion `retrieval/*.rs`. None issue IO or a batchable external call inside the loop.
- **Payment provider (the task's flagged external surface) is single-item by contract and out of scope.** `PaymentProvider` (`kernel/src/ports/payment_provider.rs:207`) exposes `create_with_key`/`query_status_by_key`/`capture_leg`/`void_leg`/`refund` — all per-item. The only loop over provider calls is `run_nleg_saga` (`kernel/src/ports/payment.rs:416`, `for (leg, res) in ...`), which walks the *legs of a single split-payment order* (a handful), sequentially and deliberately for saga atomicity. This is the money-exactness path explicitly excluded from today's scope, and real PSPs offer no cross-order batch-authorize anyway; orders arrive at 0.5/sec regardless.
- **Repeated crypto verify in a loop exists (`mesh.rs:227` `verify_chain`, `event_log.rs:475` `verify_chain`, bebop `core/event_log.rs:141`) but is the settled batch-verify topic** — deliberately single-verify per entry after the B4 walk-back proved batching gives no correctness-preserving throughput gain. Not revisited.

**Sketch if a target ever appeared (it hasn't):** a DataLoader-style coalescer only makes sense once a real fan-out request (e.g. an owner dashboard fetching N orders' statuses in one HTTP handler) lands on a durable pgrust store. At 0.5 orders/sec and low-thousands of rows for months (`golive-remediation/resolution.md:329`), a per-item query is sub-millisecond and below the free-tier DB noise floor. Trigger to re-evaluate: 10× pilot volume *and* a real fan-out handler — neither present.

---

## Angle 2 — Event-processing batching (tick vs one-at-a-time)

**Verdict: no real target — and the batchable surfaces are already batched.**

**dowiz kernel event_log** (`kernel/src/event_log.rs`): single-writer, append-only, durable-insert-then-`set_tip` (`:302-321`, `:366-391`). `append`/`commit_after_decide` operate on **one event** because that is the correctness contract: each commit runs `decide` before persist, dedups on a stable content-id, and the durability barrier (`insert?`) gates the tip so in-memory state never claims a rejected write. Batching multiple events per fsync would blur the per-event Law-reject vs store-fault poles (`CommitError`, `:269-275`) and the per-event idempotency the P07 §2 fix depends on. At ~5k events/day/hub with a ~µs commit, there is nothing to gain and determinism to lose. **Correctly single-event.**

**dowiz WASM analytics — already batch-in.** `channel_ledger` and `reduce_anomalies_logic` (`kernel/src/wasm.rs:261`, `:300`) take a whole `Vec<EventIn>` per call and fold the entire stream in one pass, capped at `MAX_CHANNEL_EVENTS = 100_000` (`wasm.rs:33/249`). This is the batch API a naive audit would ask for — it exists.

**bebop mesh gossip/relay — already batch, by Merkle anti-entropy (the decisive finding).** Sync does NOT process frames one at a time:
- `bebop2/core/src/anti_entropy.rs`: `digest` (`:35`) → `diff` (`:75`) → `apply_pull(log, missing: &[(u64, &[u8])])` (`:121`) folds a **slice of missing events in one call**.
- `bebop2/proto-wire/src/mesh_sync_integration.rs:29-32`: "ship ONE batch (all missing `SyncFrame`s length-prefixed into a single `SignedFrame`)"; `wrap_batch`/`encode_batch`/`decode_batch` (`:107/137/148`), `fold_batch` (`:200`).
- `bebop2/proto-wire/src/sync_pull.rs:506/593`: "Outcome of folding a **batch** of pulled frames"; "Fold a **batch** of pulled frames."

The Transport's per-call `recv()` (`bebop2/proto-wire/src/lib.rs:94`, `mesh-node/src/node.rs:83`) returns one `SignedFrame`, but that frame already *carries the whole delta batch*. The only genuinely one-at-a-time path is the live event carry (`MeshNode::admit_inbound`, `node.rs:69`) for real-time delivery events at ~0.5–33/sec — batching there would inject queue-and-flush latency into a user-facing event for zero throughput benefit. The outbound sink already coalesces via `MeshEventSink::drain()` (`node.rs:115`, returns `Vec<Event>`).

**bebop core event_log** (`bebop2/core/src/event_log.rs`) mirrors dowiz: single `append` (`:83`) with a batch `rebuild_from_payloads(&[Vec<u8>])` (`:123`) already available for the at-rest bulk path.

---

## Bottom line

Both batch techniques are real and well-established, but neither has a genuine target here. Where batching pays off (bulk analytics ingest; mesh catch-up/anti-entropy) the code already batches. Where processing is one-at-a-time (per-event commit, live event admit, per-leg saga) it is deliberately single-item for determinism/atomicity, and the platform's documented volume (~0.5 orders/sec system-wide; "unmeasurable, fine at 1000×") means a coalescer or batch tick would add latency and complexity for a win below the measurement floor. Consistent with today's research discipline: **do not manufacture a batch target that the volume does not justify.** Re-evaluate only on a 10×+ volume shift combined with a real fan-out request handler landing on a durable store — neither exists today.
