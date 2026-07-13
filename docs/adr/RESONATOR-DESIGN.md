# Resonator — детермінований feedback-loop controller (DONE + verified)

Дата: 2026-07-13. Джерело ідеї: електротехнічна аналогія genAI (Voltage/Current/Resistance/
Transformer/Fuse) + кібернетичний closed-loop із довгого дослідницького дампу Gortai.
Рішення оператора: "все що було для gortai — використовуй як аналогію для bebop, bebop2, dowiz".

## Ground truth (що ВЖЕ було в репо — НЕ будували вдруге)
- **bebop/crates/bebop/src/stabilizer.rs** — Lyapunov descent, SMC, root-locus setpoint,
  saturation clamp (fuse + transformer), ground_state. САМА електротехнічна аналогія в Rust.
- **bebop/crates/bebop/src/wavefield.rs** — граф-хвилі, spectral notch, "capacitor" енергії.
- **bebop2/core/src/{lyapunov,kalman,active,field,chebyshev,fft,vsa}.rs** — математичне ядро:
  `stability_margin`, spectral covariance (Q=resistance/budget), active inference (step-down
  transformer), spectral propagators, FFT-власні значення, VSA-пам'ять.
- **dowiz/agent-governance/index.ts** — governance-порт bebop: осі, settings, `detectDrift`,
  error-patterns. RED+GREEN тести через `node:test` + `tsx --test`.
- **dowiz/packages/ui/src/theme + ThemeProvider** — design tokens / brand presets (voltage
  stabilizer / brand ground).
- TS `strict: true` увімкнено.

## Gap (чого НЕМАЄ) — І заповнено
Високорівневого **closed-loop orchestrator**, що зшиває math core в ОДНУ керовану петлю із
fuse/rollback. Це не нова математика — це проводка існуючого ядра.

## Реалізовано (VERIFIED)

### 1. bebop2/core/src/resonator.rs  (host-gated, zero-dep, no_std-compatible core)
Immutable `Reference` (ground) + три актори `Generator`/`Reflector`/`Supervisor` + `metric`:
- Δ-threshold збіжність (`error < ε` → `Converged`)
- max-iteration fuse (`max_iterations` → `Fused`)
- stall patience (слабкий reflector, низька якість → `Stalled`)
- drift accumulator (Σ |Δerror| — "струм не туди")
- Lyapunov chaos watchdog (`lyapunov_guard`): якщо крок збільшує error (дивергує) — freeze
- `rollback_to_best` — реверсія до найкращого checkpoint

Verify: `cargo test -p bebop2-core --features host resonator` → **6/6 green**;
повний crate **166/166 green**.

### 2. dowiz/agent-governance/resonator.ts  (zero-dep, deterministic, strict-friendly)
TS-дзеркало #1. Перевикористовує існуючий governance-стиль (uk|en, pure functions).
Verify: `npx tsx --test agent-governance/resonator.test.ts` → **6/6 green**;
весь governance suite **16/16**; `tsc --noEmit` чисто (0 помилок у resonator/agent-governance).

### 3. Документація аналогії
**bebop-repo/docs/GORTAI-ANALOGY.md** — mapping table (electrical ↔ cybernetic ↔ bebop/bebop2/
dowiz primitive) + що вже існувало + що додано + чим стає Gortai-продукт (тонкий шар поверх
`resonator`, не окремий репо).

## Межі (innovate:)
- `resonator` — контролер, НЕ LLM-оркестратор. Актори ін'єктяться; модель не викликається.
- Lyapunov watchdog у bebop2 використовує 1-D proxy (знак d(error)/d(step)) замість повної
  eigen-decomp щотику — щоб лишатися allocation-free. Upgrade trigger: підключити
  `crate::lyapunov::spectral_radius` напряму, коли стан `S` несе квадратну eigen-систему.
- Gortai (підлітковий адаптивний контент) — це product layer поверх `resonator`
  (Sensor→Comparator→Controller→Actuator→Feedback). Окремий репо не потрібен.

## Verify gate (пройдено)
- bebop2: `cargo test -p bebop2-core --features host resonator` → 6/6 ✅ ; crate 166/166 ✅
- dowiz:  `npx tsx --test agent-governance/resonator.test.ts` → 6/6 ✅ ; suite 16/16 ✅ ; tsc ✅
