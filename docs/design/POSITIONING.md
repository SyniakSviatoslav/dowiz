# POSITIONING — bebop protocol is the headliner, dowiz is the by-product

> Draft copy for the public-flip landing surface (BLUEPRINT-P19 §2, V6 dual-track).
> **This document is a plan + draft. Publishing it to a live site is a separate gated
> action owned by Phase 18 / the operator — not executed here.** Nothing here is
> submitted or shipped to any external surface.

## Hero

**bebop — a post-quantum protocol for networks that must not fail.**
Self-certifying nodes. Zero central server. Delivery is just the first thing we built on it.

## One sentence

bebop is a decentralized, post-quantum-secure protocol where every node owns its own
database and signs its own frames — ML-DSA-65 for signatures, ML-KEM-768 for key
exchange, both FIPS 204/203 with bit-exact known-answer tests in the repo — and
**dowiz** is a real delivery service that runs entirely on top of it, with no
privileged coordinator anywhere in the loop.

## Three pillars

**1 — The protocol is the product.** bebop is a wire format and a trust model, not an
app. Every edge is autonomous: it holds its own ML-DSA identity, signs its own frames,
and answers to no central CA. Any node can drop and the network routes around it by
recomputed shortest path — *designed coordination*, not a leader election.

**2 — Sovereignty is a hard invariant, not a tier.** There is no server we run that you
depend on. The protocol carries zero external dependencies at the trust boundary. You
self-host, or you don't run it. AGPLv3 guarantees the protocol can never be captured —
fork it, drop our brand, keep the wire.

**3 — Delivery is the proof.** dowiz is a working delivery network — orders,
proof-of-delivery signed by the courier's own key, integer-exact settlement — that
demonstrates the protocol under real load. It is deliberately a *by-product*: if
delivery is boring and reliable on bebop, the protocol is doing its job.

## Why this exists

Most "decentralized" systems re-centralize the moment money or matching is involved.
bebop makes the economic control points (matcher, settlement) open and replicable by
construction: any node can run the matcher; settlement is a threshold verifier
(≥k-of-n), never a single oracle. Post-quantum is treated as a *protocol* — KEM,
signatures, at-rest, code-signing, in-transit integrity, composed — not a checkbox
primitive. Built in the EU/Ukraine, for infrastructure that has to keep working when
the network doesn't.

## For builders

AGPLv3. Every claim above is falsifiable against the repo: FIPS 204/203 KATs,
deterministic event-sourced state, integer-only money, in-repo proofs. No AI in the
protocol path — deterministic Rust/WASM only.

## Metaphor discipline (V6 / Fable Pattern 6)

No bare "organism / emergent / swarm" appears in published-intent copy without an
adjacent named computed criterion; such words are replaced by "designed coordination"
where they would otherwise stand alone.

## Community goodwill

Upstream projects this work builds on are credited in
[`OPEN-SOURCE-CREDITS-LIST.md`](./sovereign-roadmap-2026-07-16/OPEN-SOURCE-CREDITS-LIST.md)
— a full 3-repo attribution sweep, so the operator can star / thank each one.

## Note on brand

The pre-pivot `docs/design/dowiz-brand/BRAND-BIBLE.md` pointer is **deprecated** — that
file never existed in-tree (verified). This positioning doc is the in-tree replacement.
The public brand name (dowiz vs DeliveryOS) and any EUTM filing remain an operator
decision (BLUEPRINT-P18 O16), applied at the public flip.
