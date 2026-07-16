# ARCHITECTURE — dowiz / DeliveryOS (CANONICAL, 2026-07-16)

> Single source of truth for stack + architecture law. Supersedes sprawling arc-notes where
> it contradicts them (see STRATEGIC-VECTORS-LOCKED-2026-07-16 V2). Revisit rejections ONLY
> via DECART-escape (honest falsifiable comparison-prototype, modern/rust-native default).

## Core law (locked)
- **Rust / WASM kernel** — bare-metal deterministic core; native Rust libs = default.
- **Trait-as-Port** — every external capability (cloud, GPU, transport, storage) behind a trait.
  Swapping a provider = adapter swap, never a rewrite. Older tech = adapter, NOT purged.
- **Content-addressing** — `sha3_256` is the universal cache/dedup/audit key (BlockStore/EventStore).
- **GPU = offline, behind-a-port, NEVER in-kernel, NEVER in request path.** Value transfers as
  design principles (fusion/batching/KV-cache/locality tiers) onto CPU, never as hardware.
  (Cross-corroborated: SYSTEMS-§11 `field_frame.rs`="wgpu OUT OF SCOPE" + GAUSSIAN-§2.3
  "GPU always offline/behind-a-port".)

## Permanent rejections (DECART-gated escape)
| Rejected | Why (code-grounded) | Escape |
|---|---|---|
| Managed cloud (AWS/RDS/EKS) as default | adapter-only behind traits | DECART comparison |
| Kubernetes | zero-OCI rule (`FROM scratch`, check-zero-oci.sh) | DECART comparison |
| GraphQL as mesh/inter-node protocol | trusted-resolver contradicts deny-by-default capability trust | DECART comparison |
| IAM/reputation/blacklist trust | capability-tokens only (`NO-COURIER-SCORING`) | DECART comparison |
| Literal GPU/CUDA in stack | kernel stays non-AI (`attention.rs`="reference scalar") | DECART comparison |
| "Digital MCU" as description of NOW | numeric layer allocation-heavy today; north-star only | close P0-A1/P1 core-pinning |

## Layers
- **kernel/** — Rust deterministic core (FSM, spectral, geo, money, order-machine, memory).
- **engine/** — Rust compute (field-frame, loop, VertexBridge CPU + gpu feature-gate).
- **web/** — Astro/Svelte + wasm bridges (math from wasm only, 0 JS re-impl).
- **bebop2/** (separate repo) — sovereign PQ delivery protocol (ML-DSA-65, mesh).
- **tools/** — eqc (math proofs), deep-clean (disk hygiene, rusqlite).

## Verification law (V5)
- cargo test / eqc = blocking CI. Red-line/money/orders require independent verifier re-execution.
- claim-latency metric logged; self-green anomalies sampled.
- VERIFIED-BY-MATH: works? provable? falsifiable? → ship RED otherwise.

## Consolidation note
Arc-notes (hk05/gaussian/systems/math-first/knowledge-spine/etc.) are research history, indexed
in MEMORY.md. They are NOT architecture law. This file is. New architecture claims → edit HERE
(merge), never append a new arc-file for stack decisions.
