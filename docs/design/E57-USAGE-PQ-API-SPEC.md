# E57 — Usage-based PQ-api: SPEC-ONLY design + build-trigger

> BLUEPRINT-P19 §5. **STATUS: SPECIFICATION ONLY. Zero implementation is scheduled in
> this phase.** This document defines the shape so that *when* demand is real the build
> is a small, known step — not so that it is built now.

## Concept

A hosted endpoint exposing the repo's zero-dep ML-DSA-65 / ML-KEM-768 primitives to
callers who want post-quantum crypto without self-hosting a node — the paid convenience
layer over the free protocol.

## Endpoint shape (design sketch, not to be implemented here)

- `POST /v1/pq/sign` — body → ML-DSA-65 signature (caller-supplied key handle).
- `POST /v1/pq/verify` — {msg, sig, pubkey} → bool.
- `POST /v1/pq/kem/encaps` / `POST /v1/pq/kem/decaps` — ML-KEM-768 encapsulate / decapsulate.
- `GET  /v1/pq/usage` — the caller's metered counters (self-service, no surveillance).

## Pricing-model shape

Usage-based: a free monthly quota, then pay-as-you-go per 1k operations, with a
per-account budget ceiling (reuses the TokenBucket / Budget posture from E19·F6). No
seat licenses, no minimums — metered convenience over a commons primitive.

## Metering approach

Count operations locally as **typed metrics** consistent with M8 (per-process, typed,
local sink; never exfiltrated as surveillance); billing reads the local counter. No
request bodies retained; no PII; deterministic per-op accounting.

> **BUILD-TRIGGER (E57):** the first qualified **paying B2B inquiry** — a real prospect
> who has asked to pay for hosted PQ operations. Until that fires, E57 stays spec-only.
> Building billing before a paying inquiry exists violates MANIFESTO C8 / ponytail (YAGNI).
> When it fires, E57 converts to an implementation phase (endpoints + metering + budget
> rails), not before.

## Verification (this phase does NOT build it)

- `grep -rn "BUILD-TRIGGER" docs/` hits this line (named, grep-able trigger).
- `grep` finds **no** billing / usage / PQ-api endpoint code in the Rust stack.
- Acceptance: spec-only; implementation is a separate, trigger-gated phase.
