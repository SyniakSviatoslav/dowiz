# Індекс звітів аналізу (Фаза 1/3)
- B-living-memory.md — bebop LivingMemory/agentic_git/knowledge/recall_graph/enrich/error_patterns (DONE)
- E-kernel-governance.md — dowiz kernel decide/fold + money + wasm + agent-governance/resonator.ts (DONE)
- A-l5-control.md — wiring/guard/stabilizer/governor/drift/coherence/active_inference (pending)
- C-field-physics.md — field/wavefield/optical/geometry + CSR WASM (pending)
- D-bebop2-math.md — resonator/lyapunov/kalman/fft/chebyshev/vsa/algebra/PQ (pending)
- F-orchestration.md — loops/eval-layer/metric-core/automation/audit-sentinel/hooks (pending)

# Ключові висновки що вже зафіксовані (для синтезу):
## B (living memory):
- tick() ВЖЕ non-destructive (attic move, commit cf917ba); attic UNBOUNDED (defect)
- salience зберігається але НЕ читається decay — "forgetting" = hash-lottery mod 7 (кожен вузол evicted ≤7 тіків) — НЕ importance-weighted
- FNV-1a-32 id → колізії ~77k концептів (50% birthday)
- spreading activation: w = decay^(dist+1), MAX-merge, hops=2, decay=0.5; eval gate graph@4=0.917 vs flat 0.500
- NOISE_FLOOR=0.35 cosine на 256-dim byte histogram (без embedder!)
- adaptive-k: H=-Σp·log2 p, norm=H/8, k=round(3+17·norm)∈[3,20]
- agentic_git: content-addressed commits (FNV32 sorted state), verify_integrity tamper-check; АЛЕ snapshot lossy (тільки concept→payload, без layer/salience/attic)
- audit.rs AuditLog = SHA256 hash-chain (~90% WORM) АЛЕ wiring.rs імпортує СЛАБКИЙ research_patterns::AuditLog (Vec без chain) — SECURITY DEFECT
- enrich.rs: Pareto frontier, GD, Adam, SEAL store (trigger→correction), confidence gate → human review
- error_patterns: 15 markers, count across scans, JSON persist ~/.bebop/error_patterns.json
- ledger.rs: SHA256 id, Σbalance=0 (i128), idempotent replay — готовий шаблон ентропійного/токен-бюджету
- active_inference.rs: EFE(a)=Σ p(s'|a)·(−G[s']), argmin
- stabilizer consensus: entropy=√variance пропозицій; reject if > threshold
- sealfb: tol_i = base/(1+k·(energy_i/max_energy)); stationary: max|cur−prev|<eps
- Повнота для контуру: WORM 90%, checkpoints 80%, persistence claims 55%, entropy ledger 50%, memory-as-ground drift 40%

## E (kernel+governance):
- kernel/src/pq НЕ існує на цій гілці; тестів 37 inline
- order_machine: повна таблиця allowed_next, 3-tier error taxonomy (SameStatus/ScaffoldDisabled/Illegal), stable codes; fold_transitions зупиняється на першому illegal + повертає позицію
- money: i64 minor units, SCALE=1e6 micro-rate, i128 intermediates, half-up в ОДНОМУ місці; дефекти: unchecked arithmetic, dead guards (NaN-fossil, round_half_up=identity), silent i128→i64 cast
- wasm: created_at_ms = seq counter (НЕ час!), Box::leak unbounded, HashMap funnel — недетермінований JSON порядок
- analytics ChannelLedger: idempotent ingest (dup order_id → false), фіксований 10-stage funnel; reduce_anomalies реюзає Law (measurement = source of truth); дефект: same at_ms тихо перезаписує
- resonator.ts: tick = generate→reflect→supervise→(lyapunov guard: nextErr>error+1e-9 ⇒ freeze)→commit/hold→drift+=|Δerr|; Converged/Fused/Stalled(quality<0.5); rollbackToBest
- resonator дефекти: quality=0 конфлація frozen vs zero-quality; stall unreachable якщо reflector завжди ≥0.5; drift рахує ОБИДВА напрямки; lateral cycling на equal-error shell; crash на empty checkpoints; sync-only actors (потрібен async port + per-tick timeout/budget); metric NaN не валідований; L2 truncates до min(len)
- governance: drift detector = substring lint (25%); error-patterns TS дзеркало Rust (70%); voodoo HARD BAN тримається лише якщо call sites перевіряють isHardBanned
- Повнота: decide-law 85%, event-fold 60% (linear, не дерево), integer accounting 75%, resonator control shape 90%, drift (a)70/(b)65/(c)25

## A (L5 control layer) — DONE:
- wire() тік: field veto (heat-kernel blast>0.10 на secrets-node ⇒ refuse, fail-closed) → stabilize_step → consensual_aggregate → permit_action (гейт САТУРОВАНОГО значення, не raw — C2 інваріант!) → TargetScope CIDR → proceed = кон'юнкція reasons.is_empty() → memory.remember → audit.record
- L5 output НЕ впливає на proceed — bounded actuation поруч із go/no-go
- Математика: V̇=(v_cur−v_prev)/dt, dt≤0⇒0 (D1 FAIL-OPEN!); adaptation_allowed: V̇≤freeze_threshold(=0); saturate=limit·tanh(δ/limit); V(θ)=½Σk_i(θ_i−b_i)², shape mismatch⇒∞; wall: +h·(1−(d/r)²) if d<r; consensual: σ=√(Σ(p−μ)²/N)>θ⇒None else saturate(μ) — "entropy"=std-dev, НЕ Shannon
- guard: io_guard refuse if !field_stable or |δ|>max_delta(1.0); KillSwitch ≥2/3 supermajority, self-vote ignored
- governor PID: e=q−0.9, I clamp[−1,1], u=1.4e+0.22I+1.5Δe, authority clamp[0,1]; u_min/u_max/dead_ic DEAD (D4: один reject u≈−2.6 slam до 0)
- SMC: s=c(x−x_ref), reaching s·ṡ<0, u=u_eq−K·sgn(s), boundary layer |s|<φ ⇒ −K·s/φ
- root locus: s²+2ζωn·s+Kωn²=0, stable=−b/2<0 (D3 WRONG для K<0); lead: φmax=asin((α−1)/(α+1))
- Kalman scalar: p+=q; k=p/(p+r); x+=k(z−x); p=(1−k)p — конвергенція+gain decay tested
- limit-cycle: flips≥min AND (peak−trough)≤band; len<4⇒false ("silence is not safety")
- sealfb: tol_i=base/(1+k·E_i/maxE); is_stationary: max|Δ|<eps
- active_inference: EFE=Σp(s'|a)·(−G[s']), argmin; fail-closed НЕ повний (D9 panic на b[a≥1])
- фрактальний dispatch: φ, fib fast-doubling (n>92⇒None), golden_branch_depth=⌈ln(leaves)/ln φ⌉
- 52 тести; стрес-тест 400 тіків: freeze fired + settles stationary + finite
- ВІДСУТНЄ: integral action у actuation path, gain scheduling, СПЕКТРАЛЬНИЙ аналіз (10%), freeze hysteresis (D16 flapping), checkpoint-diode (20%)
- Повнота: ground re-injection 90%, proposer/decider seam 85%, PID 75%, fuses 70% (немає token/timeout), persistence filter 70%, contraction k<1 60%, entropy drain 40%, diode 20%, spectral 10%
- Дефекти топ: D1 dt≤0 fail-open; D2 coherence інтегрує +L (АНТИ-дифузія, exp(+Lt)); D3 root-locus K<0; D6 wire() hardcode freeze=0, ensemble default entropy_threshold=0 (будь-який спред ⇒ reject); D7 memory overwrite повторного task (FNV колізії); D8 killswitch 2-node net не може suspend, немає un-suspend; D12 два різні V конфльовані (field energy vs параметри θ); D16 freeze flapping без dwell

## F (orchestration infra) — DONE:
- loops/: 17-полева YAML loop-card схема; фази SENSE→DIAGNOSE→ACT→VERIFY→REPEAT (10/12), design-convergence: FRAME→PROPOSE→ATTACK→RESOLVE→RE-ATTACK; exit = ALL-must-hold кон'юнкція; no-fake-green у всіх 12
- Рубрика M1-M11: M9 = anti-cheat dry-run (зламаний fixture МУСИТЬ дати RED —

## F (orchestration infra) — DONE:
- loop-card 17 полів; фази default [SENSE,DIAGNOSE,ACT,VERIFY,REPEAT]; error-fix [RUN,DIAGNOSE,FIX,RE-VERIFY,REPEAT]; design [FRAME,PROPOSE,ATTACK,RESOLVE,RE-ATTACK,REPEAT]
- exit = ALL-must-hold кон'юнкція; iron principle no-fake-green у ВСІХ 12
- Рубрика M1–M11: M3 verification real "not vibe", M4 hard exit ALL-hold, M5 no-fake-green, M9 ANTI-CHEAT DRY-RUN (зламаний fixture МУСИТЬ дати RED), M11 separate-agent cross-review; вердикт binary CERTIFIED|REJECTED
- loop-orchestrator (dispatch, ніколи не будує) vs loop-architect (будує+сертифікує, ніколи не dispatch) vs worker(skill) vs counsel(health-pass)
- metric-core run-checks.mjs: ДЕТЕРМІНОВАНИЙ done-gate (НЕ LLM), hard/soft split, passed=0 hard failures, score=fraction, exit 0 iff all hard pass; Langfuse OTLP spans; hard: tsc/money/rls/playwright-smoke/env; soft: lint/contracts(=echo OK ПЛЕЙСХОЛДЕР!)
- eval-layer: OpenRouter judge (gpt-4o, temp0), 3 GEval метрики @0.5 threshold (ADVISORY, не gating); --dry-run пише fake scores у той самий файл (false-green вектор)
- tier1 (read-only cron watch, haiku, max-turns 12, Telegram кожен run) → tier2 (nightly clone audit, $1 fuse, morning aggregate) → tier3 (mechanical sweep: executor→adversarial reviewer VERDICT:PASS|REJECT→revert-on-anything-else fail-closed, draft PR НІКОЛИ не merge)
- audit-sentinel: probes E1-E5 (TLS/HSTS/cookies=BLOCKER/health/rate-limit), baseline diff NEW/REGRESSED, prod+BLOCKER→exit1; escalate/telegram.ts формат готовий
- hooks (settings.json, детерміновані): protect-paths (hard block migrations/.github/.claude/etc), serious-gate (deny until .claude/state/serious-cleared), red-line-doubt-gate (deny migrations until redline-confirmed), pre-edit-lessons (advisory), post-edit-gates (post-hoc red-line grep), loop-detector (N=3 signature→escalation ladder), route-request (nudge), require-classification (Stop block без CHANGE-MANIFEST)
- 17 ESLint rules: no-insecure-random (crypto для token/otp/secret), require-auth-hook (owner/courier routes), no-permissive-status-assertion (тест anti-cheat), no-direct-websocket (єдиний shared), no-hardcoded-color/string тощо
- escalation ladder: self-divergence→specialist subagent→stronger model→council→human; budget K=2, loop N=3
- Дефекти: guard-bash.sh НЕ підключений (dead), tier2 auditor STUB (NO-GO 0 findings), sentinel Telegram unwired + cooldown in-memory, REGRESSED dead path, 2 checks configs divergent (root unreachable), check-contracts=echo OK, CERTIFIED loops без proof reports на диску, status flip ungated (serious-gate exempt loops/*)
- Повнота: intake compiler 60%, deterministic separator 85%, fuses/budgets 80%, human bypass valve 90%, loop certification 75% design/40% enforcement, escalation ladder 70%, instrument panel 65% (per-run є, per-iteration НЕМАЄ)
- Ліфтабельне: M1-M11+M9 template, run-checks.mjs, tier3 executor→reviewer→revert, loop-detector signature-fuse, .claude/state/* file-valve

## C (field physics) — DONE:
- Heat kernel u(t)=exp(−cLt)u0 ТРИ реалізації: (a) spectral Chebyshev (rust-core field_spectral) — ПРАВИЛЬНИЙ знак, mass-conserving, детермінований libm (fexp/fcos shims), 64 quadrature nodes, deg=20 caller — ВИКОРИСТОВУВАТИ ЦЮ; (b) field_active Euler +dt·cLu — АНТИ-дифузія D3; (c) coherence::propagate — D1 wrong sign + single step. Chebyshev: ã(L)=(2/b)L−I, b=λmax=2·maxdeg
- CSR kernel: raw C-ABI WASM (no wasm-bindgen), STATE Mutex single-instance, ACCUM Σ|Δu| sensitivity; field_build/matvec/spectral/active/rank/cost/sensitivity; f32 packing storage-only (compute f64); no memory cap; 200-cycle leak gate
- field.rs veto: keyword→node (secret|auth|money|migrat|rls→node4), plan_csr 6-node, field_rank(seed,t=1,coeff=0.5,deg=20), veto iff out[4]>TOLERANCE=0.10; secrets-blast≈0.66 vs docs≈0.06; Override+Unhealthy⇒"override" fail-closed
- wire() conjunction: proceed = field==Permit AND forbidden-cleared AND in-scope (field veto UNCONDITIONAL over L5)
- field_physics.rs: damped wave m·ü=c²(L̂_solid+L̂_graph)u−γu̇+s; V-dim tensor per node (platonic 4/6/8/12/20 vertices=mass); WAVE_C2=1, WAVE_DAMP=0.08, FLUID_ADV=0.05; change_impact (blast radius, dt=0.02 pinned, 0.05 divergent); wave_energy=KE+intra-solid PE (D4: graph term dropped!); hop_distances BFS
- wavefield.rs: graph_laplacian_eigs = cyclic Jacobi (АЛЕ тільки eigenVALUES, НЕ vectors — D: не можна проєктувати drift на моди, лише λ₂); spectral_notch λ₂<frac·λmax⇒brittle; field_divergence=Σout−Σin (runaway hub); floyd_cycle; layout FR spring; LinkKind weights Action1/Method0.7/Relation0.5/Data0.3
- geometry_field: platonic exact (F,E,V) Euler=2, spherical harmonics Y_l^m real, Nyquist winding; φ=(1+√5)/2
- mathx: divergence_2d central-diff, first_order step/settling, lagrange_interp, classify_trajectory (FixedPoint/LimitCycle/Divergent/Undetermined mid-third vs last-third amplitude)
- optical.rs = perceptual aHash (НЕ optics!) 8×8 64-bit Hamming
- 60+ тестів; rust-core 19 (mass preservation, deadlock-free concurrent, no-accumulation 200 cycles)
- Дефекти: D1/D3 sign (coherence+field_active анти-дифузія), D4 energy incomplete (graph term dropped), D5 asymmetric inter-solid coupling non-conservative, D8 keyword veto bypassable (s3cret), D9 Floyd guard fail-OPEN, Jacobi no eigenvectors, D12 BEBOP_WAVE_GATE=0 still enables, D14 O(N²)/O(N³) unenforced
- Повнота для контуру: blast-radius predict 90%, diffusion-entropy 90% (тільки Chebyshev!), potential-field ground 80%, damping 75% (немає critical-damping selector γ_crit=2c√λ), spectral drift modes 55% (eigenvalues only), loop resonance watchdog 80%

## D (bebop2 math core) — DONE:
- ⚠️ КРИТИЧНО: resonator.rs ПОВНИЙ closed-loop controller АЛЕ НЕ зареєстрований у lib.rs (немає pub mod resonator) — МЕРТВИЙ КОД, 6 тестів не бігають. Живий тільки TS-порт. Найдешевший unlock: додати #[cfg(feature="host")] pub mod resonator;
- resonator Rust vs TS розбіжності (4): (1) Rust L2Metric mismatch⇒INFINITY (TS min(len) → тихо конвергує!); (2) Rust seed checkpoint[0]=initial (TS порожній масив → crash max_iter=0); (3) Rust має is_chaotic termination (rising_streak≥n && total>1e-9); TS НЕМАЄ; (4) Rust return best checkpoint (TS повертає last committed, rollback opt-in)
- resonator: defaults max_iter=64, ε=1e-6, stall_patience=8, lyapunov_guard=true; tick generate→reflect→supervise→watchdog(next_err>error+1e-9⇒freeze, 1-D proxy НЕ викликає lyapunov.rs)→commit/hold→drift.step→best-checkpoint; termination Converged/Stalled(quality<0.5 OR chaotic)/Fused; ЗАВЖДИ повертає best checkpoint
- lyapunov.rs: НЕ time-series LLE — це лінійна спектральна стійкість; eigenvals через Jacobi (MAX_SWEEP=100, TOL=1e-14); stability_margin=max Re(λ); spectral_radius=max|λ| (=контракційна константа k! stable iff ρ<1)
- kalman.rs: НЕ повний фільтр — тільки covariance time-update P_k=A·P·Aᵀ+Q (немає H/R/gain/innovation); SpectralKalman eigenbasis pointwise O(n²); Gauss-Jordan invert
- fft.rs: radix-2 Cooley-Tukey, власний Complex, dft_oracle, circulant_eigenvalues=FFT(row); callers: vsa::bind/unbind (єдиний)
- chebyshev.rs: matrix-free exp(−coeff·L·t)u0, qp=64 quadrature, T_{k+1}=2X·T_k−T_{k−1}, X=(2/b)L−I, b=2·maxdeg; ПРАВИЛЬНИЙ знак (на відміну від crates/bebop coherence/field_active)
- field.rs (bebop2): LaplacianSpectrum З eigenVECTORS (Jacobi jacobi_eigen degenerate-fix phi==0→t=1)! propagate_spectral=Σ e^{−λt}⟨u0,φ⟩φ; active_diffuse dt_STABLE=0.02 (B11, 0.05 divergent); rank/cost/sensitivity; НА ВІДМІНУ від crates/bebop wavefield (тільки eigenvalues) — ТУТ Є МОДИ для проєкції drift
- algebra.rs: cosine (zero-guard 1e-12, clamp), project/reconstruct (signal=спектральні коефіцієнти); vsa.rs: bind=IFFT(FFT⊙FFT) circular conv, unbind=Fourier deconv (|A|²>1e-30), bundle=mean; padded_dim=next_pow2; sparse-spectrum key НЕ інвертується
- active.rs: F=½bᵀLb−H[b], spectral F=½Σλ_k⟨b,φ_k⟩²−H; entropy H=−Σb ln b; belief_diffuse b'=normalize(b−dt·β·L·b); ТІЛЬКИ perception half (немає policy selection/EFE)
- rng.rs: ChaCha20 CSPRNG (RFC 8439), same seed⇒same stream; from_seed gated #[cfg(test/dangerous_deterministic)] — prod НЕ може створити predictable; fail-closed entropy (compile_error на невідомий target); XChaCha subkey
- lib.rs: no_std gated, resonator ВІДСУТНІЙ; crypto завжди on, analytic host-gated; C8 fexp fix (symmetric range reduction для x<0)
- crypto (95% готовий tamper-evident checkpoints): SHA-512/SHA3 (hash.rs), Ed25519 (sign.rs GF(2²⁵⁵−19) 4×u64), ML-KEM-768 q=3329 (pq_kem), ML-DSA-65 q=8380417 vendored NIST ACVP byte-exact 64 tests (pq_dsa), XChaCha20-Poly1305 (aead), Argon2id+BLAKE2b (kdf); всі мають tamper-RED
- proto-cap: Capability (Ed25519+ML-DSA-65 hybrid), TLV canonical signing (DOMAIN_TAG16‖tag‖ver‖count‖fields, sha3-256), SignedFrame, HybridGate (replay reject, MAX_SEEN_NONCES=2²⁰, expiry via monotonic tick НЕ wall-clock), AnchorRoster (genesis-frozen, verify_chain narrow-only attenuation)
- proto-wire: Transport trait, Envelope{version,trace16,payload}, framing [u32 len][JSON] MAX 8MiB, channel_binding_hash=sha3_256(transcript) (replay defense), WssTransport real
- 68 тестів analytic + crypto (aead5/hash8/kdf5/sign5/pq_kem5/pq_dsa 64 ACVP-expanded)
- Повнота: signed checkpoints 95%, VSA similarity 80%, contraction/k measurement 70%, spectral drift modes 70%, deterministic rng 100%, Kalman estimation 40% (немає measurement update)
