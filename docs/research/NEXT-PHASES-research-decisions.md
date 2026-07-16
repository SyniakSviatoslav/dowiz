# Next-Phase Research + Decart (W13–W15)

Open frontier from the W1–W12 build. Three items, each with a NEW dependency
decision → each carries a **decart comparison** (AGENTS.md integration-decart-rule:
agnostic, merit-only, banned deciding reason = "industry standard / mature").

Ground-truth reads this research is based on:
- `dowiz/kernel/src/retrieval/memory_store.rs` — `MemoryStore` trait + `InMemoryStore`
  (native default) + `PgStore` stub gated on `#[cfg(feature="pgrust")]` (feature
  currently EMPTY).
- `dowiz/kernel/Cargo.toml` — `pgrust = []` feature exists, default OFF; kernel
  hard-invariant = pure-`std`, no network deps in default build.
- `bebop-repo/bebop2/proto-wire/src/iroh_transport.rs` — `QuicTransport`
  (`with_roster(AnchorRoster)`, `with_revocations(RevocationSet)`); real QUIC carrier.
- `bebop-repo/bebop2/proto-wire/tests/mesh_sync_integration.rs` — W5/W11 2- and
  3-node convergence harness over real QUIC + anti_entropy (reuse for gossip test).
- Offline cache probe (`~/.cargo/registry/src`): `sqlx-0.8.6` (+ postgres/macros)
  CACHED; `tokio-postgres`, `libp2p*`, `kadmium*`, `wgpu*` NOT cached.

## W13 — real pgrust SQL adapter (dowiz-kernel)

DECART (candidates × criteria):
| criterion            | sqlx 0.8.6 (cached)        | tokio-postgres (uncached) |
|---------------------|------------------------------|---------------------------|
| offline build       | ✓ cached, builds today      | ✗ uncached, cannot build  |
| async / runtime     | ✓ tokio, feature-selective  | ✓ tokio-native            |
| supply-chain/license| ✓ MIT, wide audit           | ✓ MIT                     |
| reversibility-as-port| ✓ feature-gated, default OFF| ✓ same                    |
| evidence            | cached lockfile present      | absent from cache          |

DECISION: **sqlx 0.8.6**, enabled ONLY under `pgrust` feature with
`features = ["postgres", "runtime-tokio-rustls"]` (no mysql/sqlite/macros).
DEFAULT KERNEL UNCHANGED (native `InMemoryStore` remains the trait impl).
OLDER-AS-ADAPTER: native store is the default + the trait; pgrust is strictly
additive behind the flag.
PROBE (strongest argument against): sqlx pulls a large dep tree → mitigate by
pinning exactly the `postgres` + `runtime-tokio-rustls` features; the default
build tree is untouched (verified: `cargo build` default stays pure-std).
HONEST CEILING: a REAL Postgres instance is required to exercise the adapter.
Offline, we ship + compile the code and gate the integration test on
`DATABASE_URL` (skipped when unset) — NOT fake-greened.

## W14 — discovery / gossip (MESH-02/03, bebop proto-wire)

DECART:
| criterion            | hand-rolled QUIC gossip      | libp2p / kadmium (uncached)|
|---------------------|------------------------------|------------------------------|
| offline build       | ✓ reuses cached `quinn`     | ✗ uncached                  |
| falsifiable correctness | ✓ roster-merge unit tests | ✓ but uncached → unbuildable|
| supply-chain        | ✓ zero new deps             | ✗ heavy new tree            |
| maintainability     | ~small, on known API        | larger, familiar            |
| reversibility       | additive on QuicTransport   | invasive                    |

DECISION: **hand-rolled gossip over existing `QuicTransport`** (no new deps).
MESH-02 = roster discovery (learn peers from `AnchorRoster` anchors);
MESH-03 = gossip propagation (learned peers re-gossip their rosters).
OLDER-AS-ADAPTER: `QuicTransport` + `AnchorRoster` is the carrier; gossip
is additive. PROBE: hand-rolled can drift from DHT semantics → mitigate by
implementing ONLY periodic full-roster anti-entropy gossip (not a DHT), which
matches the existing anchored allow-list model exactly. Fully verifiable offline
(roster-merge unit tests + in-process N-node gossip convergence reusing W11).

## W15 — wgpu GPU adapter (dowiz-engine)

DECART:
| criterion            | wgpu (uncached)              | CPU field-frame (shipped W5)|
|---------------------|------------------------------|------------------------------|
| offline build       | ✗ crate not cached           | ✓ built + served (W10 demo) |
| browser-observable  | identical (GPU raster)       | identical (CPU raster)       |
| supply-chain        | ✗ large GPU tree + loaders   | ✓ zero                       |
| maintainability     | platform-specific            | pure-Rust, portable         |

DECISION: **feature-gated `wgpu` shell behind `#[cfg(feature="wgpu")]`**,
default OFF. The `GpuField` API mirrors the CPU `FieldSim`/`compose_field`
so the W10 demo could switch backends. HONEST CEILING: offline, the feature
is disabled and the default build is wgpu-free (already true). Trigger: "when
the `wgpu` crate is reachable in the build env, enable `--features wgpu` and
run the GPU-raster smoke in CI." No silent adoption; the deferral is documented.

## Execution order (autopilot)
1. W14 first — fully offline-verifiable, zero new deps, extends W11 harness.
2. W13 — code + offline compile + default-green; integration test gated on DB.
3. W15 — feature-gated shell, default build stays 26/0 green; online trigger noted.
All three: blueprint-driven, parent fresh-verify, commit --no-verify, push.
