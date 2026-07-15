# Tech Synthesis 2026-07-15 — Research Conspect

> Full sourcing for PLAN.md's rankings. Research run 2026-07-15 via parallel background agents +
> direct verification (GitHub API, live repo fetch, kernel source reads). Metrics cited, not
> invented — where no verifiable data exists, that's stated explicitly rather than estimated.

## Cluster 1 — Rendering / physics-GPU

**Gaussian Splatting.** Canonical reference: [graphdeco-inria/gaussian-splatting](https://github.com/graphdeco-inria/gaussian-splatting) — **non-commercial research-only license**, hard blocker for product use. 22.6k★, ≥30fps@1080p, 24GB VRAM to train, CUDA+Python only, no WASM path. Faster reference: [nerfstudio-project/gsplat](https://github.com/nerfstudio-project/gsplat), Apache-2.0, 4x memory reduction / 15% training-time reduction vs. graphdeco — still CUDA-only.

Rust/wgpu/WASM ports (the technically relevant set):
| Repo | License | wgpu/WASM | Metric |
|---|---|---|---|
| [ArthurBrussee/brush](https://github.com/ArthurBrussee/brush) | Apache-2.0 | wgpu via Burn+CubeCL, WASM (Chrome 134+ only) | No published fps |
| [KeKsBoTer/web-splat](https://github.com/KeKsBoTer/web-splat) | Apache-2.0 | wgpu→WebGPU | **&gt;200fps RTX3090, ~130fps 8-yr-old AMD R9 380** @1200×799 |
| [mosure/bevy_gaussian_splatting](https://github.com/mosure/bevy_gaussian_splatting) | Apache-2.0 | wgl2+webgpu, WASM | No published fps |

**Octorender: does not exist.** Confirmed via GitHub repo/code search, crates.io API, npm registry API — zero matches. Likely misremembered as OctaneRender (closed-source, unrelated) or a conflation with Octree-GS ([city-super/Octree-GS](https://github.com/city-super/Octree-GS), CUDA/Python-only, non-commercial license inherited from graphdeco-inria, 64.87% memory reduction vs Mip-NeRF-360 baseline). Dropped from the plan.

## Cluster 2 — Scientific computing / math kernel

Grounded against actual kernel source: `kernel/src/spectral.rs`, `kernel/src/householder.rs`,
`kernel/src/causal.rs`, `kernel/Cargo.toml` (read directly, not inferred).

**CuPy** ([cupy/cupy](https://github.com/cupy/cupy), MIT, 12,141★): 10-50x elementwise speedup, 50-200x dense matmul speedup vs NumPy on GPU — but **up to 10x SLOWER than NumPy for arrays under ~1MB** (transfer overhead dominates). `householder.rs`'s eigensolver is explicitly stack-only, N≤32 (≤8KB) — 2-3 orders of magnitude below that threshold. No Rust bindings exist; Rust-native equivalents reaching the same CUDA libs are `candle`+`cudarc` (mature) or the portable `wgpu` compute path.

**Gamma function:** hook = Beta-distribution `Γ(α+β)/(Γ(α)Γ(β))·x^(α-1)(1-x)^(β-1)` as the conjugate prior for turning `causal.rs::empirical_identify`'s current point-estimate into a credible interval (Gelman et al., *Bayesian Data Analysis*; arXiv:1308.5595).

**Wronskian:** Abel's identity/Liouville's formula (`dW/dt = tr(A)·W`) links Wronskian growth to `tr(A) = Σλᵢ` — real, citable, but redundant: the kernel already checks solution independence via direct rank/determinant. Discrete analogue = Casoratian (arXiv:2403.09658), more relevant to the kernel's actual discrete-time recurrences (`markov.rs`, `kalman.rs`) but still no unmet need.

**Harmonic progression:** harmonic centrality (Marchiori & Latora; NetworkX/igraph/Neo4j-GDS all ship `harmonic_centrality`) — reciprocal of the harmonic mean of shortest-path distances, handles disconnected graphs cleanly (`∞⁻¹=0`). Information centrality = harmonic mean of resistance distances, computable directly from `spectral.rs`'s existing Laplacian eigenmodes via Moore-Penrose pseudo-inverse. Strongest fit of the four math primitives — see BLUEPRINTS.md TS-01.

**Born rule:** quantum-causal-model literature exists (arXiv:2506.10045, arXiv:1107.5849) but `causal.rs` is entirely classical Pearl-style SCM — no complex amplitudes, no non-commuting operators anywhere in the codebase. Explicitly a stretch; not adopted.

## Cluster 3 — LLM inference/agent infra

| Tool | Repo | Lang/License | Serving mode | Metric |
|---|---|---|---|---|
| AirLLM | [lyogavin/airllm](https://github.com/lyogavin/airllm) | Python/Apache-2.0 | None (in-process only) | 70B→~4GB VRAM; 3rd-party: 10-40min/200tok unquantized |
| Mesh-LLM | [Mesh-LLM/mesh-llm](https://github.com/Mesh-LLM/mesh-llm) | Rust (71.5%)/Apache-2.0 | OpenAI-compatible `/v1` :9337 | None published; self-labeled "experimental" |
| Omni-route | [diegosouzapw/OmniRoute](https://github.com/diegosouzapw/OmniRoute) | TS/MIT | OpenAI-compatible `/v1` :20128, Docker | 15-95% token compression claimed (self-reported, unverified) |
| OpenInterpreter (current) | [openinterpreter/openinterpreter](https://github.com/openinterpreter/openinterpreter) | Rust/Apache-2.0 | ACP agent mode | None published |
| OpenInterpreter (classic) | [endolith/open-interpreter](https://github.com/endolith/open-interpreter) | Python/AGPL-3.0 | CLI/REPL | N/A — unmaintained fork |

Note: the `openinterpreter/openinterpreter` GitHub org/repo now serves a completely different, Rust-based Codex-fork agent (confirmed via GitHub API — same repo ID `666299222`, different codebase) than the historically-known Python tool, which now lives as an unofficial fork at `endolith/open-interpreter`.

## Cluster 4 — Niche/unclear identification

| Item | Status | Real match |
|---|---|---|
| OpenAlice | IDENTIFIED | [TraderAlice/OpenAlice](https://github.com/TraderAlice/OpenAlice), AGPL-3.0, trading-agent workspace, ~6k★ |
| VeRa | AMBIGUOUS | Best guess: VeRA PEFT method (arXiv:2310.11454), implemented in `huggingface/peft` |
| Webscope | IDENTIFIED | [Aditya060806/WebScope](https://github.com/Aditya060806/WebScope), MIT — page→text-grid bridge for agents, 80-100ms render, 50-150 tokens vs ~2000 for a screenshot |
| Netclode | IDENTIFIED | [angristan/netclode](https://github.com/angristan/netclode) — Kata Containers+Cloud Hypervisor microVM agent sandboxing, ~199★, license unconfirmed |
| Afaan/mc | UNIDENTIFIED | No match found after real search effort |
| IaC/wip | Not a named tool | Generic label; dowiz already chose OpenTofu (OPS-18) |
| Skylos | IDENTIFIED | [duriantaco/skylos](https://github.com/duriantaco/skylos), Apache-2.0 — TS/JS/Python/Go/etc. dead-code+secret scanner, own benchmark: 29/29 vs Vulture 24/29 recall |

## Cluster 5 — Authorized security testing

**Hydra** ([vanhauser-thc/thc-hydra](https://github.com/vanhauser-thc/thc-hydra), AGPL-3.0, 12,056★, C, pushed 2026-07-11). 50+ protocols incl. `HTTP-FORM-POST` (web login forms), SSH, RDP, SMB, MySQL, PostgreSQL, MongoDB. Documented benchmark: **POP3 cracking, 92min single-threaded → 50sec at 128 parallel tasks (~110x)**. README: "THIS TOOL IS FOR LEGAL PURPOSES ONLY." Scoped here strictly to authorized testing of dowiz's own staging login endpoint — see BLUEPRINTS.md TS-03.

**Excluded (not researched further, per explicit scope decision):** njRat, AsyncRAT, SpyNote, 888 RAT, GhostShell Framework, hexsec-rat, KittySploit — RATs/breach-tooling with no coherent "integrate into a production delivery platform" story under any framing (see PLAN.md header for the full reasoning).

## Cluster 6 — AI/architecture concepts

See CONCEPTS-APPLIED.md for the full breakdown (already-implemented vs. net-new-with-rationale vs.
conflicts-with-a-standing-decision). Headline findings: the pasted "bidirectional real-time mesh"
architecture sketch validates the already-scoped bebop2-mesh/Living-Memory-as-pgrust direction rather
than proposing an alternative; Terraform and the AWS service list both conflict with decisions
already recorded in `docs/design/ops-reliability/BLUEPRINTS-OPS-RELIABILITY.md` (OpenTofu chosen
over Terraform at OPS-18; Hetzner-consolidation away from managed cloud).
