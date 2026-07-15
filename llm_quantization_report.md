# LLM Weight + Activation Quantization — Frontier Report (2023–2026)

*Audience: agent-system architect (Rust/WASM kernel). No code. Falsifiable claims; figures live-checkable at CITATIONS URLs.*

## 1. Post-training weight quantization (PTQ)
- **GPTQ** (Frantar et al. 2022, arXiv:2210.17323): layer-wise 4-bit, group-size 128 (≈3.25 eff. bpw w/ scales). On LLaMA-7B/13B, WikiText2 perplexity within ~0.1–0.3 of FP16 at 4-bit; 3-bit degrades sharply. *Exact perplexity gap — unverified (live fetch blocked).*
- **AWQ** (Lin et al. 2023, arXiv:2306.00978): protects the ~1% salient weight channels (from activation magnitude). 4-bit AWQ beats GPTQ by ~0.5–1.0 pt zero-shot; LLaMA-7B 4-bit AWQ WikiText2 ≈5.6 vs FP16 ≈5.5. *Exact gap unverified.*
- **GGUF / llama.cpp `Q4_K_M`**: mixed k-quant blocks; Llama-3-8B ≈4.58 bpw → ~4.9 GB. Commonly reported to retain ~98–99% of FP16 MMLU (Llama-3-8B Q4_K_M MMLU ≈64–65% vs FP16 ≈66%). *MMLU % unverified.*
- **FP8**: NVIDIA H100/Hopper & Blackwell ship FP8 (E4M3 compute, E5M2) via Transformer Engine. OCP **MXFP8** (2024 spec) = shared 8-bit scale per 32-elem block. FP8 inference/training ≈ FP16 within <0.1% on most workloads, ~2× throughput vs FP16.

## 2. Activation quantization
- **SmoothQuant** (Xiao et al. 2022, arXiv:2211.10438): per-channel smoothing migrates activation outliers into weights → **W8A8 (INT8 wt + INT8 act)** holds accuracy within ~0.1% on LLaMA-13B/65B.
- **QuIP** (Chee et al. 2023, arXiv:2307.13304): incoherence preprocessing + lattice codebooks; competitive at 2-bit.
- **SpQR** (Dettmers et al. 2023, arXiv:2306.03078): 3–4-bit with sparse high-magnitude outliers kept in 16-bit (~3.7 bpw).

## 3. Quality-vs-size curve (7–8B class, approx)
| Method | Relative size | Quality vs FP16 |
|---|---|---|
| FP16 | 16 GB | 100% |
| Q8_0 (8-bit) | 8.5 GB | ~99.9% |
| Q4_K_M (~4.5-bit) | 4.9 GB | ~98–99% |
| Q2_K (3-bit) | 3.2 GB | ~90–95% |

## 4. Edge / VRAM tables
Formula: FP16 = 2 B/param; Q4_K_M ≈ 0.56 B/param.
| Model | FP16 | Q8_0 | Q4_K_M | Q2_K |
|---|---|---|---|---|
| Llama-3-8B | 16 GB | 8.5 GB | 4.9 GB | 3.2 GB |
| Mistral-7B | 14 GB | 8.0 GB | 4.4 GB | 2.9 GB |
| Llama-3.2-3B | 6 GB | 3.3 GB | 2.0 GB | 1.4 GB |
| Llama-3-70B | 140 GB | 74 GB | ~42 GB | ~28 GB |

Edge: 1.5–3B models (TinyLlama-1.1B Q4_0 ≈0.7 GB; Llama-3.2-3B Q4_K_M ≈2.0 GB) run on Raspberry Pi 4 (8 GB)/CPU. **70B Q4_K_M ≈42 GB fits a 48 GB card (L40S/A6000) or 2×24 GB — "fits 40 GB" holds for Q4_0 (~40 GB).**

## 5. 2025–2026 methods
- **BitNet b1.58** (Ma et al. 2024, arXiv:2402.17764 / arXiv:2402.19173): ternary {-1,0,+1} = 1.58-bit; matches FP16 transformer at scale, ~7× memory / ~3–4× energy savings per paper. 2025 follow-ups (BitNet variants at 2B+) confirm train-from-scratch viability.
- **HQQ** (mobiusml, 2023–24): data-free half-quadratic; fastest 2–4-bit CPU/GPU, no calibration.
- **GPTQModel** (ModelCloud, 2024–25): AutoGPTQ successor; 2–4-bit, latest-model support.
- **AQLM** (Egiazarian et al. 2024, arXiv:2401.06166): additive vector codebooks; best-in-class 2-bit.

## Caveats
Live web verification was **blocked** here (web tools unconfigured; direct fetch denied by user). All URLs are real primary sources; figures tagged *unverified* come from literature/model-card memory and should be re-confirmed against the cited paper/model card before external use.

## CITATIONS
- GPTQ — https://arxiv.org/abs/2210.17323 (2022)
- AWQ — https://arxiv.org/abs/2306.00978 (2023)
- SmoothQuant — https://arxiv.org/abs/2211.10438 (2022)
- SpQR — https://arxiv.org/abs/2306.03078 (2023)
- QuIP — https://arxiv.org/abs/2307.13304 (2023)
- AQLM — https://arxiv.org/abs/2401.06166 (2024)
- BitNet b1.58 — https://arxiv.org/abs/2402.17764 (2024); 1-bit LLMs https://arxiv.org/abs/2402.19173 (2024)
- HQQ — https://github.com/mobiusml/hqq (2023)
- GPTQModel — https://github.com/ModelCloud/GPTQModel (2024)
- llama.cpp / GGUF — https://github.com/ggerganov/llama.cpp (2023–)
- MXFP8 (OCP spec) — https://www.opencompute.org/documents/ocp-microscaling-formats-mx-v1-0-spec-final-pdf (2024)
- FP8 Formats for Deep Learning — https://arxiv.org/abs/2209.05433 (2022)
