# ADR-0008 — Per-node local SQLite + PQ at-rest envelope (no central DB)

- Status: PROPOSED (design gate — ports red-team D1-F3 / legacy H2 / H4 class into the NEW architecture)
- Date: 2026-07-13
- Red-line: DATA / CRYPTO. Forward-only.
- Supersedes/relates: DECISIONS D1 (drop central server/DB), DECISIONS D4.3 (per-node local SQLite + PQ-wrapped at-rest envelope), MANIFESTO §2 (local-first), red-team `D1-appsec-authz.md` F3 (cross-tenant PII erasure) + Positive-controls note on RLS.

## Context
The legacy stack held **all tenants' PII in one central Postgres**, with Row-Level Security as the *sole* cross-tenant barrier. A null-context worker (D1-F3) received broad cross-tenant grants and a missing `location_id` filter permitted **irreversible cross-tenant customer-PII erasure**. Root cause was structural: a single shared central DB means **one context bug = multi-tenant breach**.

DECISIONS D1 drops the centralized server; DECISIONS D4.3 mandates **per-node local SQLite** with a **PQ-wrapped at-rest envelope**. The envelope rides *inside* the store (the transport is the channel — DTN/BPv7 + QUIC/TCPCLv4 + BIBE per DECISIONS D3); keys are ML-KEM-768 / ML-DSA-65 (DECISIONS D2/D9, zero-dep, KAT-gated).

## Decision
- **Each node owns a local SQLite store.** There is no shared central DB, so "cross-tenant" does not exist — a node holds only its own operator's data.
- **At-rest data is wrapped in a PQ envelope** keyed to `node_id` (ADR-0007). Queries never span nodes; federation is explicit, signed, per-peer, and opt-in.
- Within a single operator's node, multi-venue data stays row-scoped by `location_id` as **defense-in-depth** — but it is no longer the *primary* barrier (the primary barrier is that the data never aggregates).

This closes the D1-F3 / H2 / H4 class **at the architecture level**: no central DB ⇒ no cross-tenant query is expressible; RLS is not relied upon as the sole barrier; PII never aggregates into one attacker-reachable surface.

## Alternatives considered
- **A — central Postgres + stricter RLS (legacy model):** REJECTED. The D1-F3 incident proved RLS-alone is one null-context bug from a cross-tenant breach.
- **B — per-tenant databases on one server:** REJECTED. still a single operator/blast-radius; doesn't remove the aggregation surface.
- **C — per-node local SQLite + PQ at-rest (chosen):** data residency by construction, PQ at rest, zero cross-tenant blast radius.

## Consequences
- **+** Data residency by construction; PQ at-rest; no cross-tenant blast radius.
- **+** Red-team D1-F3 root cause removed — cross-tenant erasure is not expressible.
- **−** No global SQL/analytics across nodes; cross-node queries must federate explicitly (signed, per-peer).
- **−** Node storage is single-operator; multi-venue-within-one-operator still row-scoped by `location_id` (kept as defense-in-depth).

## Open items / human decisions
- **HUMAN — envelope key-rotation policy** (frequency, out-of-band key backup, rotation on node_id change). Owner: operator.
- **Proof (Mandatory Proof Rule):** a falsifiable test that a query cannot address another node's store, and that an at-rest file is unreadable without the node's PQ key.
