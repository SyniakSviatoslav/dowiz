# Screenshot batch: verified reverse-engineering map

Date: 2026-08-13

## Method and scope

This work is a clean-room, kernel-native reimplementation of the transferable system patterns, not a claim that model weights, proprietary training data, Apple Metal kernels, hosted services, or complete web/desktop products were reproduced. External repositories and primary project pages were used to verify public behavior and licenses. Implementations in this crate are original Rust abstractions integrated with existing kernel organs.

## Verified source matrix

| Item | Verified primary source | License / availability | Kernel-native scope |
|---|---|---|---|
| MemHarness | arXiv:2607.28272; github.com/KnowledgeXLab/MemHarness | Apache-2.0 repository | `reconstruction_memory`: retrieve → critique → reconstruct → act; deterministic policy substitute, not GRPO training or checkpoints |
| Numbat | github.com/perplexityai/numbat | Apache-2.0 | `numbat`: normalized agent actions, rules, pre-action policy, tamper-evident forensic reconstruction |
| Breathing Form | Lindsay Kokoska social post | Artwork; no software/source license | No source-code port. A motion/meditation behavior can be represented through `oil_motion`, but the artwork itself is not copied |
| Bakous Passage | BUFT social post | Artwork; no software/source license | No source-code port. The displayed equation is dimensionally ambiguous; no unsupported physical claim is encoded |
| TurboFieldfare | github.com/drumih/turbo-fieldfare | Apache-2.0 | `turbofieldfare`: memory budgeting, sparse expert activation, adaptive routing. No Swift/Metal or Gemma weights |
| AI Agents in LangGraph | deeplearning.ai/courses/ai-agents-in-langgraph | Course, not a software artifact | Existing kernel orchestration plus planned explicit graph executor; no course material copied |
| OpenObserve | github.com/openobserve/openobserve | AGPL-3.0 | `openobserve`: columnar metrics, logs, traces and query surface using original Rust code; not an AGPL source port |
| Oil Motion | github.com/oil-oil/oil-motion | MIT | `oil_motion`: keyframes, motion stages, interaction bindings, fallback and reduced-motion behavior |
| PenEcho | github.com/penecho/penecho | AGPL-3.0 / commercial dual model | `penecho`: tile-indexed spatial canvas and region queries, written independently |
| DFlash | arXiv:2602.06036; github.com/z-lab/dflash | MIT | `dflash`: draft-block proposal/verification protocol and acceptance accounting. No diffusion model weights or GPU backend |
| Needle 2 | github.com/cactus-compute/needle; huggingface.co/Cactus-Compute/needle2 | Current GitHub page reports MIT; model distribution is separate | `needle2`: bounded tool-calling state machine for constrained devices. No copied weights; no claim of reproducing 45M-parameter quality |
| Open Science Desktop | github.com/ai4s-research/open-science | MIT | `open_science`: local-first notebook/report/artifact/audit workflow; no Tauri UI |
| Evolution Go | github.com/evolution-foundation/evolution-go | Apache-2.0 | `evolution_go`: transport-neutral sessions, messages, events, stores and licensing boundary. It deliberately does not impersonate WhatsApp Web |
| Entanglement monogamy/polygamy | Review literature including Frontiers in Physics 2022 and CKW-family results | Scientific literature | Planned finite-state inequality checker/educational model; not a quantum simulator |
| SL2T | deepmind.google/blog/putting-sign-language-ai-into-users-hands | Public product description; no public weights/code found | Planned privacy-preserving pose-landmark pipeline and translator interface only; no claim of reproducing Google’s model |

## Corrections to screenshot enrichment

1. MemHarness has a public repository: `https://github.com/KnowledgeXLab/MemHarness`, and the repository page identifies Apache-2.0.
2. DFlash is `https://github.com/z-lab/dflash`, paper `https://arxiv.org/abs/2602.06036`. The paper reports over 6× across tested settings; marketing maxima and per-configuration numbers must not be collapsed into one universal speedup.
3. Needle 2 is `https://github.com/cactus-compute/needle` and `https://huggingface.co/Cactus-Compute/needle2`. The current GitHub page reports MIT, not Apache-2.0; weights can have separate terms and must be checked at download time.
4. Open Science Desktop’s current citation metadata reports DOI `10.5281/zenodo.21805331` and version 0.3.3; the screenshot’s earlier DOI may refer to an older deposit/version.
5. ResearchClawBench rankings are time-dependent. A current public snapshot no longer supports the timeless claim that Open Science is #1; only dated snapshots should be quoted.
6. SL2T’s public description says the on-device MediaPipe Holistic stage extracts pose landmarks, then coordinates are sent to a server for translation. Therefore “fully on-device translation” would be inaccurate.
7. Entanglement “monogamy” and “polygamy” are families of inequalities under specific measures and assisted quantities; the distinction is not simply symmetric versus asymmetric correlation measures.
8. The Bakous equation is preserved only as an unverified artwork annotation. It is not used as a scientific formula.

## Architecture integration

- Memory path: `reconstruction_memory` should feed context-adapted guidance into agent/orchestrator decisions and record accept/reject evidence.
- Security path: `numbat` should normalize actions before `self_harness`/capability policy and append forensic records after decisions.
- Inference path: `turbofieldfare` selects a bounded active expert set; `dflash` performs lossless draft verification; `needle2` supplies bounded tool-call lifecycle semantics.
- Observability path: all new modules emit through `telemetry_aggregator`; `openobserve` supplies columnar/query views.
- Research path: `open_science` connects notebooks, artifacts, reports, skills and audit events.
- UX path: `penecho` stores spatial context; `oil_motion` maps deterministic progress to interaction-driven animation with reduced-motion fallback.
- Adapter path: `evolution_go` remains protocol-neutral and requires an explicit provider adapter for any external messaging network.

## Explicit ceilings

- No proprietary model or dataset is reconstructed from screenshots.
- No benchmark number is claimed without running the corresponding model/hardware workload.
- No AGPL implementation is copied into the kernel; only independently described behavior is reimplemented.
- No unofficial WhatsApp protocol client is enabled by default.
- No art asset is copied without an explicit reusable license.
- SL2T quality cannot be reproduced without weights/training data; the local scope is the privacy boundary, data model and pluggable inference contract.
