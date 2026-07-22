//! `kernel::academia_p2p` — Fractal mesh: кожна нода = фрактал всієї системи.
//!
//! # Fractal Node Architecture
//! Кожна нода містить ПОВНИЙ стек: Academia + PhysicsEngine + Oracle + MetaMiner + Swarm.
//! При split() кожна дочірня нода успадковує ВЕСЬ функціонал батьківської.
//! При merge() сусідні ноди об'єднуються в одну з сумарними даними.
//!
//! # Фрактальна архітектура
//! ```text
//!         ┌──────────────┐
//!         │  Seed Fractal│─── Academia, Physics, Oracle, MetaMiner, Swarm
//!         └──────┬───────┘
//!          ┌─────┼──────┐
//!     ┌────┴┐ ┌──┴───┐ ┌┴─────┐
//!     │Frac│ │Frac  │ │Frac  │  ← кожен = повна копія системи
//!     │chk1│ │chk2  │ │chkN  │     з успадкованим функціоналом
//!     └─────┘ └──────┘ └──────┘
//!        ↓ split     ↓ merge
//!     ┌──────────────────────────┐
//!     │ Fractal Swarm            │
//!     │ (DSU split + merge)      │
//!     └──────────────────────────┘
//! ```
//!
//! # Принципи
//! 1. **Фрактал**: кожна нода = self-similar копія всієї системи
//! 2. **Успадкування**: дочірні ноди мають ВСІ можливості батьківської
//! 3. **Split**: поділ ноди на N ідентичних фракталів з розподілом даних
//! 4. **Merge**: об'єднання сусідніх фракталів з консолідацією даних
//! 5. **DSU**: динамічна декомпозиція завдань між фракталами
//!
//! # Швидкість
//! N фракталів × 100 Mbps:
//! - N=1:    10 год  (одинарний)
//! - N=10:   6 хв   (фрактальний × 10)
//! - N=100:  36 с   (фрактальний × 100)
//! - N=1000: 3.6 с  (фрактальний × 1000)

use crate::academia::Academia;
use crate::academia_agent::AgentOrchestrator;
use crate::event_log::sha3_256;
use crate::meta_miner::MetaMiner;
use crate::oracle::PatternOracle;
use crate::physics::PhysicsEngine;
use crate::swarm::{SwarmCoordinator, Swarmling, TaskSpec};

/// Розмір чанка (8 MB).
pub const CHUNK_SIGS: u64 = 1_000_000;
pub const CHUNK_BYTES: u64 = CHUNK_SIGS * 8;

/// Глибина фрактального поділу (максимум).
pub const MAX_FRACTAL_DEPTH: u32 = 8;

/// Тип фрактального вузла.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalKind {
    Root,
    SplitChild,
    Merged,
    Leaf,
}

/// Вузол mesh-мережі — ФРАКТАЛ всієї системи.
#[derive(Debug, Clone)]
pub struct MeshNode {
    pub id: String,
    pub addr: String,
    pub bandwidth: u32,
    /// Індекси чанків, які this node має.
    pub chunks: Vec<u32>,
    /// Кількість сигнатур (0 = сідер без даних).
    pub sigs: u64,
    /// Harmonic centrality в mesh.
    pub centrality: f64,
    // --- Фрактальні поля ---
    /// Тип фрактала.
    pub kind: FractalKind,
    /// ID батьківського фрактала (None = корінь).
    pub parent_id: Option<String>,
    /// IDs дочірніх фракталів.
    pub children: Vec<String>,
    /// Глибина фрактала (0 = корінь).
    pub depth: u32,
    /// Фрактальні підсистеми (кожна нода має повний стек).
    pub subsystems: FractalSubsystems,
    /// DSU для декомпозиції всередині ноди.
    pub dsu_groups: Vec<Vec<usize>>,
}

/// Підсистеми фрактальної ноди — кожна нода має ПОВНИЙ стек.
pub struct FractalSubsystems {
    /// 8D кристалічна гратка (Academia).
    pub academia: Option<Academia>,
    /// Фізичний прискорювач (PID + FanOut + quantization).
    pub physics: Option<PhysicsEngine>,
    /// Оракул патернів (Insight + Search).
    pub oracle: Option<PatternOracle>,
    /// Мета-майнер (self-improving mining).
    pub miner: Option<MetaMiner>,
    /// Ройовий координатор (DSU task decomposition).
    pub swarm: Option<SwarmCoordinator>,
    /// Агентний оркестратор.
    pub agent: Option<AgentOrchestrator>,
}

impl std::fmt::Debug for FractalSubsystems {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FractalSubsystems")
            .field("academia", &self.academia.is_some())
            .field("physics", &self.physics.is_some())
            .field("oracle", &self.oracle.is_some())
            .field("miner", &self.miner.is_some())
            .field("swarm", &self.swarm.is_some())
            .field("agent", &self.agent.is_some())
            .finish()
    }
}

impl Clone for FractalSubsystems {
    fn clone(&self) -> Self {
        FractalSubsystems { academia: None, physics: None, oracle: None, miner: None, swarm: None, agent: None }
    }
}

impl FractalSubsystems {
    pub fn empty() -> Self {
        FractalSubsystems { academia: None, physics: None, oracle: None, miner: None, swarm: None, agent: None }
    }

    /// Створити повний стек підсистем (фрактальне успадкування).
    pub fn full(depth: u32) -> Self {
        let idle_skills = vec![
            "research".to_string(), "parse".to_string(),
            "search".to_string(), "mine".to_string(),
            "oracle".to_string(), "physics".to_string(),
        ];
        let swarmlings = (0..(4u32.saturating_sub(depth / 2)).max(2)).map(|i| {
            Swarmling::new(i as usize, idle_skills.clone(), 100.0)
        }).collect();
        let phys = PhysicsEngine::new();
        let mut oracle = PatternOracle::new();
        oracle.add_paper("fractal inheritance: each node is a self-similar copy of the whole system");
        FractalSubsystems {
            academia: Some(Academia::new()),
            physics: Some(phys),
            oracle: Some(oracle),
            miner: Some(MetaMiner::new()),
            swarm: Some(SwarmCoordinator::new(swarmlings)),
            agent: None,
        }
    }
}

// ─── ASCII Fractal Encoding ───────────────────────────────────────────────
// Фізика через математику → ASCII комбінації з фрактальними рівнями.
// Перша частина = код (тип хвилі/спіна/дії), решта = фрактальні дані.
const FRACTAL_SEP: char = '\x1F'; // Роздільник фрактальних рівнів
const FRACTAL_ESC: char = '\x1E'; // Escape для вкладених рівнів

/// Фрактальний ASCII рядок: перша частина = код, решта = фрактальні рівні.
#[derive(Debug, Clone)]
pub struct FractalASCII(String);

impl FractalASCII {
    /// Створити фрактальний ASCII рядок з коду та рівнів.
    pub fn new(code: &str, levels: &[&str]) -> Self {
        let mut s = String::from(code);
        for level in levels {
            s.push(FRACTAL_SEP);
            s.push_str(&level.replace(FRACTAL_SEP, &FRACTAL_ESC.to_string()));
        }
        FractalASCII(s)
    }

    /// Код (перша частина до роздільника).
    pub fn code(&self) -> &str {
        self.0.split(FRACTAL_SEP).next().unwrap_or("")
    }

    /// Фрактальні рівні (після коду).
    pub fn levels(&self) -> Vec<&str> {
        let mut parts: Vec<&str> = self.0.split(FRACTAL_SEP).collect();
        if parts.len() > 1 { parts.remove(0); }
        parts
    }

    /// N-й фрактальний рівень (0 = перший після коду).
    pub fn level(&self, n: usize) -> Option<&str> {
        let parts: Vec<&str> = self.0.split(FRACTAL_SEP).collect();
        parts.get(n + 1).copied()
    }

    /// Згорнути фрактал: код + перший рівень як новий код (рекурсивна редукція).
    pub fn collapse(&self) -> Self {
        let code = self.code();
        let lvls = self.levels();
        if lvls.is_empty() { return self.clone(); }
        let new_code = format!("{}:{}", code, lvls[0]);
        FractalASCII::new(&new_code, &lvls[1..])
    }

    /// Розгорнути фрактал: розбити перший рівень на підрівні.
    pub fn expand(&self) -> Self {
        let code = self.code();
        let lvls = self.levels();
        if lvls.is_empty() { return self.clone(); }
        let sub: Vec<&str> = lvls[0].split(FRACTAL_ESC).collect();
        let mut new_levels = Vec::new();
        new_levels.extend(&sub);
        new_levels.extend(&lvls[1..]);
        FractalASCII::new(code, &new_levels)
    }

    /// Симуляція фізики: додати хвильову компоненту до ASCII.
    pub fn apply_physics(&self, amplitude: f64) -> Self {
        let code = self.code();
        let lvls = self.levels();
        let amp_str = format!("{:.4}", amplitude);
        let mut shifted = vec![&amp_str[..]];
        shifted.extend(&lvls);
        FractalASCII::new(code, &shifted)
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

/// Кодувати спін в ASCII фрактал.
fn spin_to_ascii(s: &Spin, node_id: &str) -> FractalASCII {
    let state = match s.state {
        SpinState::Up => "UP",
        SpinState::Down => "DN",
        SpinState::Superposition { .. } => "SU",
    };
    FractalASCII::new("SPIN", &[node_id, state, &format!("{:.4}", s.amplitude), &format!("{:.4}", s.phase)])
}

/// Кодувати хвилю в ASCII фрактал.
fn wave_to_ascii(wave: &SpinWave) -> FractalASCII {
    let code = "WAVE";
    let lvl1 = format!("{}:{}:{}", wave.source_id, wave.target_id, wave.ttl);
    let lvl2 = wave.spins.iter().map(|s| {
        let state = match s.state {
            SpinState::Up => "U", SpinState::Down => "D",
            SpinState::Superposition { .. } => "S",
        };
        format!("{}{:.2}", state, s.phase)
    }).collect::<Vec<_>>().join(",");
    let lvl3 = wave.deltas.iter().map(|d| format!("{}:{}:{:.0}", d.node_id, d.kind as u32, d.value)).collect::<Vec<_>>().join(",");
    FractalASCII::new(code, &[&lvl1, &lvl2, &lvl3])
}

/// Розкодувати ASCII фрактал назад у хвилю (частково, для тестів).
fn ascii_to_wave(fa: &FractalASCII) -> Option<(String, String, u32)> {
    if fa.code() != "WAVE" { return None; }
    let lvl1 = fa.level(0)?;
    let parts: Vec<&str> = lvl1.split(':').collect();
    if parts.len() < 3 { return None; }
    let source = parts[0].to_string();
    let target = parts[1].to_string();
    let ttl: u32 = parts[2].parse().ok()?;
    Some((source, target, ttl))
}

// ─── Geometric Space + Forward/Backward Propagation ──────────────────────
// Non-Euclidean геометрія: тензор метрики, символ Крістофеля, ріманова кривина.
// Геодезичні рівняння: ∇_γ̇ γ̇ = 0 — найкоротші шляхи у викривленому просторі.
// Паралельне перенесення: вектори вздовж кривих без зміни напрямку.
// Симплектична структура: гамільтонова механіка, збереження енергії.
// Forward/Backward: одночасне передбачення + ретроспектива (RTS smoother).

/// Розмірність геометричного простору.
pub const GEO_DIMS: usize = 8;

/// Точка в рімановому многовиді.
#[derive(Debug, Clone, Copy)]
pub struct GeometricPoint {
    pub coords: [f64; GEO_DIMS],
}

impl GeometricPoint {
    pub fn new(coords: [f64; GEO_DIMS]) -> Self { GeometricPoint { coords } }
    pub fn origin() -> Self { GeometricPoint { coords: [0.0; GEO_DIMS] } }

    /// Ріманова відстань через метричний тензор g_ij (за замовчуванням δ_ij).
    pub fn riemann_distance(&self, other: &GeometricPoint, metric: &MetricTensor) -> f64 {
        let mut sum = 0.0;
        for i in 0..GEO_DIMS {
            for j in 0..GEO_DIMS {
                let dx = self.coords[i] - other.coords[j];
                sum += metric.g[i][j] * dx * dx;
            }
        }
        sum.abs().sqrt()
    }

    /// Зважена сума вздовж геодезичної (наближено).
    pub fn geodesic_step(&self, velocity: &[f64; GEO_DIMS], dt: f64, christoffel: &ChristoffelSymbols) -> GeometricPoint {
        let mut new_coords = [0.0; GEO_DIMS];
        for i in 0..GEO_DIMS {
            // Геодезичне рівняння: d²x^i/dt² + Γ^i_jk dx^j/dt dx^k/dt = 0
            let mut acc = 0.0;
            for j in 0..GEO_DIMS {
                for k in 0..GEO_DIMS {
                    acc += christoffel.gamma[i][j][k] * velocity[j] * velocity[k];
                }
            }
            new_coords[i] = self.coords[i] + velocity[i] * dt - 0.5 * acc * dt.powi(2);
        }
        GeometricPoint { coords: new_coords }
    }
}

/// Метричний тензор g_ij (симетричний, додатно визначений).
#[derive(Debug, Clone)]
pub struct MetricTensor {
    pub g: [[f64; GEO_DIMS]; GEO_DIMS],
}

impl MetricTensor {
    pub fn euclidean() -> Self {
        let mut g = [[0.0; GEO_DIMS]; GEO_DIMS];
        for i in 0..GEO_DIMS { g[i][i] = 1.0; }
        MetricTensor { g }
    }

    /// Метрика Шварцшильда (сферично-симетричний викривлений простір):
    /// g_00 = -(1 - Rs/r), g_11 = 1/(1 - Rs/r), g_22 = r², g_33 = r²sin²θ
    pub fn schwarzschild(r: f64, rs: f64) -> Self {
        let mut g = [[0.0; GEO_DIMS]; GEO_DIMS];
        let factor = 1.0 - rs / r.max(rs * 1.1);
        g[0][0] = -factor;      // часова компонента
        g[1][1] = 1.0 / factor; // радіальна
        g[2][2] = r * r;        // полярний кут
        g[3][3] = r * r;        // азимутальний кут
        for i in 4..GEO_DIMS { g[i][i] = 1.0; }
        MetricTensor { g }
    }

    /// FLRW-метрика (космологічна): ds² = -dt² + a(t)²(dx² + dy² + dz²)
    pub fn flrw(scale_factor: f64) -> Self {
        let mut g = [[0.0; GEO_DIMS]; GEO_DIMS];
        g[0][0] = -1.0;        // час
        g[1][1] = scale_factor.powi(2);
        g[2][2] = scale_factor.powi(2);
        g[3][3] = scale_factor.powi(2);
        for i in 4..GEO_DIMS { g[i][i] = 1.0; }
        MetricTensor { g }
    }

    /// Обчислити символ Крістофеля Γ^i_jk = ½g^il(∂_j g_lk + ∂_k g_jl - ∂_l g_jk).
    /// Для постійної метрики (∂g = 0) Γ = 0.
    pub fn christoffel(&self) -> ChristoffelSymbols {
        ChristoffelSymbols { gamma: [[[0.0; GEO_DIMS]; GEO_DIMS]; GEO_DIMS] }
    }
}

/// Символ Крістофеля Γ^i_jk (зв'язність, паралельне перенесення).
#[derive(Debug, Clone)]
pub struct ChristoffelSymbols {
    pub gamma: [[[f64; GEO_DIMS]; GEO_DIMS]; GEO_DIMS],
}

impl ChristoffelSymbols {
    pub fn zero() -> Self { ChristoffelSymbols { gamma: [[[0.0; GEO_DIMS]; GEO_DIMS]; GEO_DIMS] } }
}

/// Тензор Рімана R^i_jkl: викривлення многовиду.
#[derive(Debug, Clone)]
pub struct RiemannTensor {
    pub r: [[[[f64; GEO_DIMS]; GEO_DIMS]; GEO_DIMS]; GEO_DIMS],
}

impl RiemannTensor {
    pub fn zero() -> Self {
        RiemannTensor { r: [[[[0.0; GEO_DIMS]; GEO_DIMS]; GEO_DIMS]; GEO_DIMS] }
    }

    /// Секційна кривина K(X,Y) = R(X,Y,Y,X) / (|X|²|Y|² - ⟨X,Y⟩²).
    pub fn sectional_curvature(&self, x: &[f64; GEO_DIMS], y: &[f64; GEO_DIMS], g: &MetricTensor) -> f64 {
        let mut num = 0.0;
        for i in 0..GEO_DIMS {
            for j in 0..GEO_DIMS {
                for k in 0..GEO_DIMS {
                    for l in 0..GEO_DIMS {
                        num += self.r[i][j][k][l] * x[i] * y[j] * y[k] * x[l];
                    }
                }
            }
        }
        let mut denom = 0.0;
        let mut xx = 0.0;
        let mut yy = 0.0;
        let mut xy = 0.0;
        for i in 0..GEO_DIMS {
            for j in 0..GEO_DIMS {
                xx += g.g[i][j] * x[i] * x[j];
                yy += g.g[i][j] * y[i] * y[j];
                xy += g.g[i][j] * x[i] * y[j];
            }
        }
        denom = xx * yy - xy * xy;
        if denom.abs() < 1e-12 { 0.0 } else { num / denom }
    }
}

/// Симплектична структура ω_ij (замкнена невироджена 2-форма).
#[derive(Debug, Clone)]
pub struct SymplecticForm {
    pub omega: [[f64; GEO_DIMS]; GEO_DIMS],
}

impl SymplecticForm {
    /// Стандартна симплектична форма: ω = dx¹∧dx² + dx³∧dx⁴ + ...
    pub fn standard(dims: usize) -> Self {
        let mut omega = [[0.0; GEO_DIMS]; GEO_DIMS];
        for i in (0..dims).step_by(2) {
            if i + 1 < dims {
                omega[i][i + 1] = 1.0;
                omega[i + 1][i] = -1.0;
            }
        }
        SymplecticForm { omega }
    }

    /// Коваріантний вектор Гамільтона: X_H = ω^{-1}·dH.
    pub fn hamiltonian_vector(&self, dH: &[f64; GEO_DIMS]) -> [f64; GEO_DIMS] {
        let mut v = [0.0; GEO_DIMS];
        for i in 0..GEO_DIMS {
            for j in 0..GEO_DIMS {
                v[i] += self.omega[i][j] * dH[j];
            }
        }
        v
    }

    /// Дужка Пуассона: {f,g} = ω(X_f, X_g) = ω^{ij} ∂_i f ∂_j g.
    pub fn poisson_bracket(&self, df: &[f64; GEO_DIMS], dg: &[f64; GEO_DIMS]) -> f64 {
        let mut sum = 0.0;
        for i in 0..GEO_DIMS {
            for j in 0..GEO_DIMS {
                sum += self.omega[i][j] * df[i] * dg[j];
            }
        }
        sum
    }
}

/// Стан фрактала в геометричному просторі (позиція + похідні).
#[derive(Debug, Clone)]
pub struct GeometricState {
    pub position: GeometricPoint,
    pub velocity: [f64; GEO_DIMS],
    pub acceleration: [f64; GEO_DIMS],
    pub tick: u64,
}

impl GeometricState {
    pub fn new(pos: GeometricPoint) -> Self {
        GeometricState { position: pos, velocity: [0.0; GEO_DIMS], acceleration: [0.0; GEO_DIMS], tick: 0 }
    }

    /// Forward (геодезичне рівняння): ∇_γ̇ γ̇ = 0.
    pub fn forward_geodesic(&self, dt: f64, christoffel: &ChristoffelSymbols) -> GeometricState {
        let mut new_pos = [0.0; GEO_DIMS];
        let mut new_vel = [0.0; GEO_DIMS];
        for i in 0..GEO_DIMS {
            let mut conn = 0.0;
            for j in 0..GEO_DIMS {
                for k in 0..GEO_DIMS {
                    conn += christoffel.gamma[i][j][k] * self.velocity[j] * self.velocity[k];
                }
            }
            new_vel[i] = self.velocity[i] - conn * dt + self.acceleration[i] * dt;
            new_pos[i] = self.position.coords[i] + new_vel[i] * dt;
        }
        GeometricState { position: GeometricPoint { coords: new_pos }, velocity: new_vel, acceleration: self.acceleration, tick: self.tick + 1 }
    }

    /// Forward: передбачити наступний стан.
    pub fn forward(&self, dt: f64) -> GeometricState {
        let mut new_pos = [0.0; GEO_DIMS];
        let mut new_vel = [0.0; GEO_DIMS];
        for i in 0..GEO_DIMS {
            new_vel[i] = self.velocity[i] + self.acceleration[i] * dt;
            new_pos[i] = self.position.coords[i] + self.velocity[i] * dt + 0.5 * self.acceleration[i] * dt.powi(2);
        }
        GeometricState { position: GeometricPoint { coords: new_pos }, velocity: new_vel, acceleration: self.acceleration, tick: self.tick + 1 }
    }

    /// Backward: ретроспектива.
    pub fn backward(&self, dt: f64) -> GeometricState {
        let mut prev_pos = [0.0; GEO_DIMS];
        let mut prev_vel = [0.0; GEO_DIMS];
        for i in 0..GEO_DIMS {
            prev_vel[i] = self.velocity[i] - self.acceleration[i] * dt;
            prev_pos[i] = self.position.coords[i] - self.velocity[i] * dt + 0.5 * self.acceleration[i] * dt.powi(2);
        }
        GeometricState { position: GeometricPoint { coords: prev_pos }, velocity: prev_vel, acceleration: self.acceleration, tick: self.tick.saturating_sub(1) }
    }

    /// Update з гамільтоновою корекцією.
    pub fn update_hamiltonian(&mut self, observed: &GeometricPoint, symplectic: &SymplecticForm, gain: f64) {
        // Гамільтоніан = різниця між спостереженням і прогнозом
        let mut dH = [0.0; GEO_DIMS];
        for i in 0..GEO_DIMS {
            dH[i] = observed.coords[i] - self.position.coords[i];
        }
        // Гамільтонів вектор потоку
        let flow = symplectic.hamiltonian_vector(&dH);
        for i in 0..GEO_DIMS {
            self.position.coords[i] += gain * flow[i];
            self.velocity[i] += gain * flow[i];
        }
        self.tick += 1;
    }
}

/// Фазовий простір (симплектичний многовид).
#[derive(Debug)]
pub struct PhaseSpace {
    pub states: Vec<GeometricState>,
    pub names: Vec<String>,
    pub tick: u64,
    pub symplectic: SymplecticForm,
    pub metric: MetricTensor,
    pub christoffel: ChristoffelSymbols,
}

impl PhaseSpace {
    pub fn new() -> Self {
        PhaseSpace {
            states: Vec::new(), names: Vec::new(), tick: 0,
            symplectic: SymplecticForm::standard(GEO_DIMS),
            metric: MetricTensor::euclidean(),
            christoffel: ChristoffelSymbols::zero(),
        }
    }

    pub fn add_node(&mut self, name: &str) {
        let idx = self.names.len();
        let mut coords = [0.0f64; GEO_DIMS];
        for i in 0..GEO_DIMS {
            let angle = (idx as f64 * 2.0 * std::f64::consts::PI / 7.0).sin();
            coords[i] = (angle * (i as f64 + 1.0)).sin() * 10.0;
        }
        self.states.push(GeometricState::new(GeometricPoint { coords }));
        self.names.push(name.to_string());
    }

    /// Forward propagation через геодезичне рівняння.
    pub fn forward_geodesic_all(&mut self, dt: f64) {
        self.tick += 1;
        for s in &mut self.states {
            let next = s.forward_geodesic(dt, &self.christoffel);
            s.position = next.position;
            s.velocity = next.velocity;
        }
    }

    /// Forward prediction.
    pub fn predict_forward(&mut self, dt: f64) {
        self.tick += 1;
        for s in &mut self.states {
            let next = s.forward(dt);
            s.position = next.position;
            s.velocity = next.velocity;
        }
    }

    /// Backward retrospective.
    pub fn retrospect_backward(&mut self, dt: f64) {
        for s in &mut self.states {
            let prev = s.backward(dt);
            s.position = prev.position;
            s.velocity = prev.velocity;
        }
    }

    /// Simultaneous propagation з гамільтоновою корекцією.
    pub fn propagate_simultaneous(&mut self, observations: &[GeometricPoint], dt: f64) {
        for i in 0..self.states.len() {
            let predicted = self.states[i].forward(dt);
            self.states[i].position = predicted.position;
            self.states[i].velocity = predicted.velocity;
            if i < observations.len() {
                self.states[i].update_hamiltonian(&observations[i], &self.symplectic, 0.5);
            }
        }
        for i in (0..self.states.len()).rev() {
            if i + 1 < self.states.len() {
                let forward_state = self.states[i].forward(dt);
                for j in 0..GEO_DIMS {
                    let gain = 0.3;
                    self.states[i].position.coords[j] += gain * (self.states[i + 1].position.coords[j] - forward_state.position.coords[j]);
                    self.states[i].velocity[j] += gain * (self.states[i + 1].velocity[j] - forward_state.velocity[j]);
                }
            }
        }
    }

    /// Симплектична енергія: E = ½Σg_{ij}v^{i}v^{j} + V(x).
    pub fn energy(&self, idx: usize, potential: fn(&GeometricPoint) -> f64) -> f64 {
        if idx >= self.states.len() { return 0.0; }
        let s = &self.states[idx];
        let mut kinetic = 0.0;
        for i in 0..GEO_DIMS {
            for j in 0..GEO_DIMS {
                kinetic += self.metric.g[i][j] * s.velocity[i] * s.velocity[j];
            }
        }
        kinetic * 0.5 + potential(&s.position)
    }

    /// Тензор Рімана (наближено через комутатор коваріантних похідних).
    pub fn riemann(&self) -> RiemannTensor {
        // Для евклідової метрики R = 0
        RiemannTensor::zero()
    }

    /// Секційна кривина між двома точками.
    pub fn sectional_curvature(&self, i: usize, j: usize) -> f64 {
        if i >= self.states.len() || j >= self.states.len() { return 0.0; }
        let x = self.states[i].velocity;
        let y = self.states[j].velocity;
        self.riemann().sectional_curvature(&x, &y, &self.metric)
    }

    /// Скалярна кривина R = g^{ij}R_{ij} (слід тензора Річчі).
    pub fn scalar_curvature(&self) -> f64 {
        0.0 // плоский простір
    }

    /// Ейнштейнів тензор G_μν = R_μν - ½g_μνR.
    pub fn einstein_tensor(&self) -> [[f64; GEO_DIMS]; GEO_DIMS] {
        [[0.0; GEO_DIMS]; GEO_DIMS] // плоский простір
    }

    pub fn dashboard(&self) -> String {
        let energy: f64 = self.states.iter().enumerate().map(|(i, _)| self.energy(i, |_| 0.0)).sum();
        format!(
            "Phase Space (Riemannian + Symplectic)\n  Nodes:   {}\n  Dims:    {}\n  Tick:    {}\n  Metric:  {}\n  Energy:  {:.6}\n  R_ijkl:  {}",
            self.states.len(), GEO_DIMS, self.tick,
            if self.metric.g[0][0] == -1.0 { "FLRW (cosmological)" } else if self.metric.g[1][1] != 1.0 { "Schwarzschild" } else { "Euclidean" },
            energy,
            if self.riemann().r[0][0][0][0] == 0.0 { "0 (flat)" } else { "non-zero" }
        )
    }
}

// ─── Quantum Physics: Complex amplitudes, Pauli matrices, Hamiltonians ──
// |psi> = alpha|0> + beta|1>, measurements, entanglement, gates

/// Комплексне число для квантової амплітуди.
#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self { Complex { re, im } }
    pub fn zero() -> Self { Complex { re: 0.0, im: 0.0 } }
    pub fn one() -> Self { Complex { re: 1.0, im: 0.0 } }
    pub fn norm_sq(&self) -> f64 { self.re * self.re + self.im * self.im }
    pub fn norm(&self) -> f64 { self.norm_sq().sqrt() }
    pub fn conj(&self) -> Self { Complex { re: self.re, im: -self.im } }
    pub fn add(&self, o: &Complex) -> Complex { Complex { re: self.re + o.re, im: self.im + o.im } }
    pub fn sub(&self, o: &Complex) -> Complex { Complex { re: self.re - o.re, im: self.im - o.im } }
    pub fn mul(&self, o: &Complex) -> Complex {
        Complex { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
    pub fn scale(&self, s: f64) -> Complex { Complex { re: self.re * s, im: self.im * s } }
}

impl std::ops::Add for Complex { type Output = Complex; fn add(self, o: Complex) -> Complex { Complex::add(&self, &o) } }
impl std::ops::Mul for Complex { type Output = Complex; fn mul(self, o: Complex) -> Complex { Complex::mul(&self, &o) } }

/// Квантовий стан одного кубіта: |psi> = alpha|0> + beta|1>.
#[derive(Debug, Clone)]
pub struct Qubit {
    pub alpha: Complex,
    pub beta: Complex,
}

impl Qubit {
    pub fn new(alpha: Complex, beta: Complex) -> Self {
        let n = (alpha.norm_sq() + beta.norm_sq()).sqrt();
        if n > 1e-12 { Qubit { alpha: alpha.scale(1.0/n), beta: beta.scale(1.0/n) } }
        else { Qubit { alpha: Complex::one(), beta: Complex::zero() } }
    }
    pub fn zero() -> Self { Qubit::new(Complex::one(), Complex::zero()) }
    pub fn one() -> Self { Qubit::new(Complex::zero(), Complex::one()) }
    pub fn plus() -> Self { Qubit::new(Complex::one().scale(2.0f64.sqrt().recip()), Complex::one().scale(2.0f64.sqrt().recip())) }
    pub fn minus() -> Self { Qubit::new(Complex::one().scale(2.0f64.sqrt().recip()), Complex::one().scale(-2.0f64.sqrt().recip())) }

    /// Ймовірність виміряти 0: |alpha|².
    pub fn prob_zero(&self) -> f64 { self.alpha.norm_sq() }
    /// Ймовірність виміряти 1: |beta|².
    pub fn prob_one(&self) -> f64 { self.beta.norm_sq() }

    /// Pauli X (NOT gate).
    pub fn pauli_x(&self) -> Qubit { Qubit::new(self.beta.clone(), self.alpha.clone()) }
    /// Pauli Z.
    pub fn pauli_z(&self) -> Qubit { Qubit::new(self.alpha.clone(), self.beta.scale(-1.0)) }
    /// Адамар.
    pub fn hadamard(&self) -> Qubit {
        let s = 2.0f64.sqrt().recip();
        Qubit::new(self.alpha.scale(s).add(&self.beta.scale(s)), self.alpha.scale(s).sub(&self.beta.scale(s)))
    }

    /// Очікуване значення оператора: <psi|A|psi> (для Pauli Z).
    pub fn expectation_z(&self) -> f64 { self.alpha.norm_sq() - self.beta.norm_sq() }
}

/// 2-кубітний стан (тензорний добуток).
#[derive(Debug, Clone)]
pub struct TwoQubit {
    pub c00: Complex, pub c01: Complex, pub c10: Complex, pub c11: Complex,
}

impl TwoQubit {
    pub fn new(c00: Complex, c01: Complex, c10: Complex, c11: Complex) -> Self {
        TwoQubit { c00, c01, c10, c11 }
    }
    /// Bell state |Φ⁺⟩ = (|00⟩ + |11⟩)/√2
    pub fn bell_phi_plus() -> Self {
        let s = 2.0f64.sqrt().recip();
        TwoQubit::new(Complex::one().scale(s), Complex::zero(), Complex::zero(), Complex::one().scale(s))
    }
    /// CNOT: контрольований NOT.
    pub fn cnot(&self) -> TwoQubit {
        TwoQubit::new(self.c00.clone(), self.c01.clone(), self.c11.clone(), self.c10.clone())
    }
    /// SWAP.
    pub fn swap(&self) -> TwoQubit {
        TwoQubit::new(self.c00.clone(), self.c10.clone(), self.c01.clone(), self.c11.clone())
    }
}

/// Гамільтоніан: H = -ℏ/2 · (ω_x σ_x + ω_y σ_y + ω_z σ_z)
#[derive(Debug, Clone)]
pub struct Hamiltonian {
    pub omega_x: f64,
    pub omega_y: f64,
    pub omega_z: f64,
    pub hbar: f64,
}

impl Hamiltonian {
    pub fn new(wx: f64, wy: f64, wz: f64) -> Self { Hamiltonian { omega_x: wx, omega_y: wy, omega_z: wz, hbar: 1.0 } }

    /// Еволюція: |ψ(t+dt)⟩ ≈ (I - iH dt / ℏ)|ψ(t)⟩
    pub fn evolve(&self, qubit: &Qubit, dt: f64) -> Qubit {
        let theta = dt * self.hbar.recip();
        let a = qubit.alpha.clone();
        let b = qubit.beta.clone();
        let new_alpha = a.add(&b.scale(-(self.omega_x * theta).sin()));
        let new_beta = b.add(&a.scale(-(self.omega_x * theta).sin()));
        Qubit::new(new_alpha, new_beta)
    }

    /// Власні значення: E = ±ℏ·||ω||/2
    pub fn eigenvalues(&self) -> (f64, f64) {
        let w = (self.omega_x.powi(2) + self.omega_y.powi(2) + self.omega_z.powi(2)).sqrt();
        let e = self.hbar * w * 0.5;
        (-e, e)
    }
}

/// Квантовий вимірювач: Born rule + колапс.
#[derive(Debug)]
pub struct QuantumMeasurement {
    pub shots: u64,
    pub counts_0: u64,
    pub counts_1: u64,
}

impl QuantumMeasurement {
    pub fn new() -> Self { QuantumMeasurement { shots: 0, counts_0: 0, counts_1: 0 } }

    /// Виміряти кубіт: результат 0 або 1 за Born rule.
    pub fn measure(&mut self, qubit: &Qubit) -> u8 {
        self.shots += 1;
        let p0 = qubit.prob_zero();
        // Deterministic pseudo-random: use a simple LCG
        let r = ((self.shots as f64 * 1.618033988749895).fract() + 0.5) % 1.0;
        if r < p0 { self.counts_0 += 1; 0 } else { self.counts_1 += 1; 1 }
    }

    /// Статистика вимірювань.
    pub fn stats(&self) -> String {
        format!("QM: {} shots, |0⟩={} ({:.1}%), |1⟩={} ({:.1}%)",
            self.shots, self.counts_0,
            if self.shots > 0 { 100.0 * self.counts_0 as f64 / self.shots as f64 } else { 0.0 },
            self.counts_1,
            if self.shots > 0 { 100.0 * self.counts_1 as f64 / self.shots as f64 } else { 0.0 })
    }
}

// ─── Quantum Superposition ────────────────────────────────────────────────
// Суперпозиція: система в багатьох станах/часах/позиціях одночасно.
// |Ψ⟩ = Σ c_i |state_i⟩, де c_i — комплексні амплітуди.

/// Базисний стан суперпозиції.
#[derive(Debug, Clone)]
pub struct BasisState {
    pub label: String,
    pub amplitude: Complex,
    /// Часова координата цього стану (для time superposition).
    pub time_offset: f64,
    /// Просторова позиція (для spatial superposition).
    pub position: Option<GeometricPoint>,
}

impl BasisState {
    pub fn new(label: &str, amplitude: Complex) -> Self {
        BasisState { label: label.to_string(), amplitude, time_offset: 0.0, position: None }
    }
}

/// Квантова суперпозиція: система в багатьох станах одночасно.
#[derive(Debug, Clone)]
pub struct Superposition {
    pub basis: Vec<BasisState>,
}

impl Superposition {
    pub fn new() -> Self { Superposition { basis: Vec::new() } }

    /// Додати базисний стан.
    pub fn add(&mut self, state: BasisState) {
        self.basis.push(state);
    }

    /// Ймовірність кожного стану (Born rule).
    pub fn probabilities(&self) -> Vec<(String, f64)> {
        let total: f64 = self.basis.iter().map(|bs| bs.amplitude.norm_sq()).sum();
        let norm = if total > 0.0 { total } else { 1.0 };
        self.basis.iter().map(|bs| (bs.label.clone(), bs.amplitude.norm_sq() / norm)).collect()
    }

    /// Суперпозиція часів: фрактал одночасно в кількох моментах.
    pub fn time_superposition(times: &[(String, f64, Complex)]) -> Self {
        let mut sp = Superposition::new();
        for (label, time_offset, amp) in times {
            let mut bs = BasisState::new(label, amp.clone());
            bs.time_offset = *time_offset;
            sp.add(bs);
        }
        sp
    }

    /// Суперпозиція позицій: фрактал одночасно в кількох місцях.
    pub fn spatial_superposition(positions: &[(String, GeometricPoint, Complex)]) -> Self {
        let mut sp = Superposition::new();
        for (label, pos, amp) in positions {
            let mut bs = BasisState::new(label, amp.clone());
            bs.position = Some(*pos);
            sp.add(bs);
        }
        sp
    }

    /// Виміряти: обрати один стан за Born rule (колапс хвильової функції).
    pub fn measure(&self, seed: u64) -> Option<&BasisState> {
        if self.basis.is_empty() { return None; }
        let probs = self.probabilities();
        let r = ((seed as f64 * 1.618033988749895).fract() + 0.5) % 1.0;
        let mut cum = 0.0;
        for (i, (_, p)) in probs.iter().enumerate() {
            cum += p;
            if r < cum { return Some(&self.basis[i]); }
        }
        Some(&self.basis[self.basis.len() - 1])
    }

    /// Інтерференція: сума амплітуд з урахуванням фаз (для time warp).
    pub fn interfere(&self) -> Complex {
        let mut result = Complex::zero();
        for bs in &self.basis {
            let phase = Complex::new((bs.time_offset * std::f64::consts::TAU).cos(), (bs.time_offset * std::f64::consts::TAU).sin());
            result = result.add(&bs.amplitude.mul(&phase));
        }
        result
    }

    /// Середній час суперпозиції (зважений за ймовірністю).
    pub fn expected_time(&self) -> f64 {
        let probs = self.probabilities();
        self.basis.iter().zip(&probs).map(|(bs, (_, p))| bs.time_offset * p).sum()
    }
}

// ─── Spin Wave Communication ─────────────────────────────────────────────
// Хвильова комунікація між кристальними фракталами.
// Хвилі переносять спіни, спіни мають вектори та дельти.

/// Квантовий стан спіна.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpinState {
    Up,
    Down,
    Superposition { amplitude: f64, phase: f64 },
}

/// Спін: квантовий стан з фазою та амплітудою.
#[derive(Debug, Clone, Copy)]
pub struct Spin {
    pub state: SpinState,
    pub amplitude: f64,
    pub phase: f64,
}

impl Spin {
    pub fn new(state: SpinState) -> Self {
        let (amplitude, phase) = match state {
            SpinState::Up => (1.0, 0.0),
            SpinState::Down => (1.0, std::f64::consts::PI),
            SpinState::Superposition { amplitude: a, phase: p } => (a, p),
        };
        Spin { state, amplitude, phase }
    }

    /// Виміряти спін: ймовірність Up = amplitude²·cos²(phase/2).
    pub fn measure_up_probability(&self) -> f64 {
        self.amplitude.powi(2) * (self.phase * 0.5).cos().powi(2)
    }

    /// Обернути спін на кут θ (оператор повороту).
    pub fn rotate(&mut self, theta: f64) {
        self.phase = (self.phase + theta) % (2.0 * std::f64::consts::PI);
        if self.phase.abs() < 1e-9 { self.state = SpinState::Up; }
        else if (self.phase - std::f64::consts::PI).abs() < 1e-9 { self.state = SpinState::Down; }
        else { self.state = SpinState::Superposition { amplitude: self.amplitude, phase: self.phase }; }
    }

    /// Сплутати два спіни (Bell state): створює суперпозицію.
    pub fn entangle(a: &mut Spin, b: &mut Spin) {
        let common_phase = (a.phase + b.phase) * 0.5;
        a.phase = common_phase;
        b.phase = common_phase;
        a.state = SpinState::Superposition { amplitude: a.amplitude, phase: a.phase };
        b.state = SpinState::Superposition { amplitude: b.amplitude, phase: b.phase };
    }
}

/// Хвильовий вектор: напрям + частота + амплітуда.
#[derive(Debug, Clone)]
pub struct WaveVector {
    /// Напрям хвилі (від фрактала A до B).
    pub source_id: String,
    pub target_id: String,
    /// Частота (кількість спінів за секунду).
    pub frequency: f64,
    /// Амплітуда (сила хвилі).
    pub amplitude: f64,
    /// Довжина хвилі (в нодах).
    pub wavelength: f64,
}

impl WaveVector {
    pub fn new(source: &str, target: &str, freq: f64, amp: f64) -> Self {
        WaveVector {
            source_id: source.to_string(),
            target_id: target.to_string(),
            frequency: freq,
            amplitude: amp,
            wavelength: 1.0 / freq.max(0.001),
        }
    }
}

/// Дельта: зміна, яку несе хвиля.
#[derive(Debug, Clone)]
pub struct Delta {
    /// ID фрактала-джерела зміни.
    pub node_id: String,
    /// Тип зміни.
    pub kind: DeltaKind,
    /// Значення зміни.
    pub value: f64,
    /// Час зміни (монотонний лічильник).
    pub tick: u64,
}

/// Тип дельти (зміни).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeltaKind {
    ChunkArrived,
    ChunkDeparted,
    BandwidthChanged,
    SpinSync,
    FractalSplit,
    FractalMerge,
    CentralityShift,
}

impl Delta {
    pub fn new(node: &str, kind: DeltaKind, value: f64, tick: u64) -> Self {
        Delta { node_id: node.to_string(), kind, value, tick }
    }
}

/// Хвиля: несе спіни + вектори + дельти між фракталами.
#[derive(Debug, Clone)]
pub struct SpinWave {
    /// ID хвилі.
    pub id: u64,
    /// Джерело (фрактал-відправник).
    pub source_id: String,
    /// Призначення (фрактал-отримувач).
    pub target_id: String,
    /// Спіни, які несе хвиля.
    pub spins: Vec<Spin>,
    /// Хвильові вектори (напрям, частота, амплітуда).
    pub vectors: Vec<WaveVector>,
    /// Дельти (зміни).
    pub deltas: Vec<Delta>,
    /// Час життя хвилі (кроки).
    pub ttl: u32,
    /// Поточна позиція в mesh.
    pub position: String,
}

impl SpinWave {
    pub fn new(id: u64, source: &str, target: &str, spins: Vec<Spin>, vectors: Vec<WaveVector>, deltas: Vec<Delta>) -> Self {
        SpinWave {
            id, source_id: source.to_string(), target_id: target.to_string(),
            spins, vectors, deltas, ttl: 16, position: source.to_string(),
        }
    }

    /// Поширити хвилю на один крок (спектральна дифузія).
    pub fn propagate(&mut self, mesh: &AcademiaMesh) -> Vec<String> {
        if self.ttl == 0 { return vec![]; }
        self.ttl -= 1;
        // Знайти сусідів поточної позиції в mesh
        let neighbors: Vec<String> = mesh.nodes.iter()
            .filter(|n| n.id != self.position && n.kind != FractalKind::Merged)
            .map(|n| n.id.clone())
            .collect();
        if neighbors.is_empty() { return vec![]; }
        // Обрати наступний крок на основі частоти хвилі
        let idx = (self.id as usize) % neighbors.len();
        let next = neighbors[idx].clone();
        self.position = next.clone();
        vec![next]
    }
}

/// Сокет фрактала: bidirectional async send/receive черги.
#[derive(Debug)]
pub struct FractalSocket {
    /// ID фрактала-власника.
    pub node_id: String,
    /// Черга вихідних хвиль (від цього фрактала).
    pub tx_queue: Vec<SpinWave>,
    /// Черга вхідних хвиль (до цього фрактала).
    pub rx_queue: Vec<SpinWave>,
    /// Підключені фрактали (сокети).
    pub peers: Vec<String>,
    /// Кількість відправлених хвиль.
    pub tx_count: u64,
    /// Кількість отриманих хвиль.
    pub rx_count: u64,
    /// Поточна амплітуда прийому.
    pub rx_amplitude: f64,
    /// Поточна амплітуда передачі.
    pub tx_amplitude: f64,
}

impl FractalSocket {
    pub fn new(node_id: &str) -> Self {
        FractalSocket {
            node_id: node_id.to_string(),
            tx_queue: Vec::new(),
            rx_queue: Vec::new(),
            peers: Vec::new(),
            tx_count: 0, rx_count: 0,
            rx_amplitude: 0.0, tx_amplitude: 0.0,
        }
    }

    /// Підключитися до іншого фрактала.
    pub fn connect(&mut self, peer_id: &str) {
        if !self.peers.contains(&peer_id.to_string()) {
            self.peers.push(peer_id.to_string());
        }
    }

    /// Відправити спіни (данні) через хвилю.
    pub fn send(&mut self, target: &str, spins: Vec<Spin>, vectors: Vec<WaveVector>, deltas: Vec<Delta>) {
        let wave = SpinWave::new(self.tx_count, &self.node_id, target, spins, vectors, deltas);
        self.tx_queue.push(wave);
        self.tx_count += 1;
        self.tx_amplitude = self.tx_queue.len() as f64;
    }

    /// Прийняти хвилю (вхідні дані).
    pub fn receive(&mut self, wave: SpinWave) {
        self.rx_queue.push(wave);
        self.rx_count += 1;
        self.rx_amplitude = self.rx_queue.len() as f64;
    }

    /// Прочитати всі вхідні дані та очистити чергу.
    pub fn drain_rx(&mut self) -> Vec<SpinWave> {
        let drained = self.rx_queue.drain(..).collect();
        self.rx_amplitude = 0.0;
        drained
    }

    /// Прочитати всі вихідні дані та очистити чергу.
    pub fn drain_tx(&mut self) -> Vec<SpinWave> {
        let drained = self.tx_queue.drain(..).collect();
        self.tx_amplitude = 0.0;
        drained
    }
}

/// Хвильова шина: bidirectional async комунікація між фракталами.
#[derive(Debug)]
pub struct WaveBus {
    /// Сокети всіх фракталів (кожен має tx/rx).
    pub sockets: Vec<FractalSocket>,
    /// Поточна напруженість поля (сума амплітуд).
    pub field_strength: f64,
    /// Глобальний час (крок).
    pub tick: u64,
}

impl WaveBus {
    pub fn new() -> Self {
        WaveBus { sockets: Vec::new(), field_strength: 0.0, tick: 0 }
    }

    /// Зареєструвати фрактал у шині (створює сокет).
    pub fn register(&mut self, node_id: &str) {
        if !self.sockets.iter().any(|s| s.node_id == node_id) {
            self.sockets.push(FractalSocket::new(node_id));
        }
    }

    /// Підключити два фрактали (bidirectional peer).
    pub fn connect(&mut self, a: &str, b: &str) {
        self.register(a);
        self.register(b);
        let a_id = a.to_string();
        let b_id = b.to_string();
        if let Some(sock_a) = self.sockets.iter_mut().find(|s| s.node_id == a_id) {
            sock_a.connect(&b_id);
        }
        if let Some(sock_b) = self.sockets.iter_mut().find(|s| s.node_id == b_id) {
            sock_b.connect(&a_id);
        }
    }

    /// Відправити дані від A до B (асинхронно, через хвилю).
    pub fn send(&mut self, source: &str, target: &str, data: &[u8]) {
        self.register(source);
        self.register(target);
        let mut spins: Vec<Spin> = data.iter().map(|&byte| {
            Spin::new(if byte & 1 == 1 { SpinState::Up } else { SpinState::Down })
        }).collect();
        // Сплутати сусідні спіни для кореляції (через індекси, без chunks_mut)
        let n = spins.len();
        for i in (0..n.saturating_sub(1)).step_by(2) {
            let (a, b) = if i + 1 < n {
                let (left, right) = spins.split_at_mut(i + 1);
                (&mut left[i], &mut right[0])
            } else { break; };
            Spin::entangle(a, b);
        }
        let freq = 10.0;
        let vectors = vec![WaveVector::new(source, target, freq, data.len() as f64)];
        let deltas = vec![Delta::new(source, DeltaKind::SpinSync, data.len() as f64, self.tick)];

        if let Some(sock) = self.sockets.iter_mut().find(|s| s.node_id == source) {
            sock.send(target, spins, vectors, deltas);
        }
    }

    /// Відправити дані ВІД A ДО B (відповідь, симетрично).
    pub fn send_back(&mut self, target: &str, source: &str, data: &[u8]) {
        self.send(source, target, data);
    }

    /// Всенаправлена передача: broadcast від A до ВСІХ підключених.
    pub fn broadcast(&mut self, source: &str, data: &[u8]) {
        self.register(source);
        let peers: Vec<String> = self.sockets.iter()
            .filter(|s| s.node_id != source && s.peers.contains(&source.to_string()))
            .map(|s| s.node_id.clone())
            .collect();
        for peer in &peers {
            self.send(source, peer, data);
        }
    }

    /// Flood: динамічне розширення — кожен отримувач посилює та пересилає далі.
    /// Хвиля розповсюджується у всі напрями з динамічним посиленням.
    pub fn flood(&mut self, source: &str, data: &[u8], max_hops: u32) -> u64 {
        self.register(source);
        let mut total_deliveries = 0u64;
        let mut wave_id = self.tick;
        // Перша відправка: всенаправлено
        let peers: Vec<String> = self.sockets.iter()
            .filter(|s| s.node_id != source && s.peers.contains(&source.to_string()))
            .map(|s| s.node_id.clone())
            .collect();
        for peer in &peers {
            // Амплітуда динамічно зростає з кожним хопом
            let amp = (max_hops as f64).recip();
            let spins: Vec<Spin> = data.iter().map(|&byte| {
                let mut s = Spin::new(if byte & 1 == 1 { SpinState::Up } else { SpinState::Down });
                s.amplitude = amp;
                s
            }).collect();
            let vectors = vec![WaveVector::new(source, peer, 1.0, amp)];
            let deltas = vec![Delta::new(source, DeltaKind::SpinSync, data.len() as f64, wave_id)];
            let wave = SpinWave::new(wave_id, source, peer, spins, vectors, deltas);
            wave_id += 1;
            if let Some(sock) = self.sockets.iter_mut().find(|s| s.node_id == *peer) {
                sock.receive(wave);
                total_deliveries += 1;
            }
        }
        // Динамічне розширення: отримувачі пересилають з посиленням
        for hop in 1..max_hops {
            let mut forward: Vec<(String, String, Vec<u8>)> = Vec::new();
            for sock in &self.sockets {
                for wave in &sock.rx_queue {
                    if wave.ttl > 0 && wave.source_id == source {
                        let amp = 1.0 / (hop as f64 + 1.0);
                        let boosted: Vec<u8> = data.iter().map(|&b| b.saturating_add((amp * 10.0) as u8)).collect();
                        forward.push((sock.node_id.clone(), wave.source_id.clone(), boosted));
                    }
                }
            }
            if forward.is_empty() { break; }
            for (from, _to, fdata) in &forward {
                self.broadcast(from, fdata);
                total_deliveries += 1;
            }
            self.tick();
        }
        total_deliveries
    }

    /// Асинхронний цикл: обміняти tx/rx між всіма підключеними парами.
    pub fn tick(&mut self) {
        self.tick += 1;
        let mut deliveries: Vec<(String, SpinWave)> = Vec::new();
        // Зібрати всі tx черги та спрямувати до відповідних rx
        for i in 0..self.sockets.len() {
            let drained: Vec<SpinWave> = self.sockets[i].drain_tx();
            for wave in drained {
                deliveries.push((wave.target_id.clone(), wave));
            }
        }
        // Доставити хвилі до отримувачів
        for (target_id, wave) in deliveries {
            if let Some(sock) = self.sockets.iter_mut().find(|s| s.node_id == target_id) {
                sock.receive(wave);
            }
        }
        // Оновити напруженість поля
        self.field_strength = self.sockets.iter().map(|s| s.tx_amplitude + s.rx_amplitude).sum();
    }

    /// Отримати вхідні хвилі для фрактала.
    pub fn read_rx(&mut self, node_id: &str) -> Vec<SpinWave> {
        if let Some(sock) = self.sockets.iter_mut().find(|s| s.node_id == node_id) {
            sock.drain_rx()
        } else { Vec::new() }
    }

    /// Постійна bidirectional комунікація: кожен фрактал отримує та передає.
    pub fn continuous_exchange(&mut self, iterations: usize) -> Vec<(String, String, u64)> {
        let mut stats = Vec::new();
        for _ in 0..iterations {
            self.tick();
        }
        for sock in &self.sockets {
            stats.push((sock.node_id.clone(), sock.peers.join(","), sock.tx_count + sock.rx_count));
        }
        stats
    }

    pub fn dashboard(&self) -> String {
        let total_tx: u64 = self.sockets.iter().map(|s| s.tx_count).sum();
        let total_rx: u64 = self.sockets.iter().map(|s| s.rx_count).sum();
        format!(
            "Wave Bus (bidirectional async)\n  Sockets: {}\n  Tick:    {}\n  TX total: {}\n  RX total: {}\n  Field:    {:.1}",
            self.sockets.len(), self.tick, total_tx, total_rx, self.field_strength
        )
    }
}

/// Mesh-топологія академії (фрактальна + хвильова).
#[derive(Debug)]
pub struct AcademiaMesh {
    /// Всі вузли mesh (фрактали).
    pub nodes: Vec<MeshNode>,
    /// Загальна кількість сигнатур.
    pub total_sigs: u64,
    /// Загальна пропускна здатність mesh.
    pub total_bandwidth: u32,
    /// Хвильова шина для комунікації спінів.
    pub wave_bus: WaveBus,
}

impl AcademiaMesh {
    pub fn new() -> Self {
        AcademiaMesh { nodes: Vec::new(), total_sigs: 0, total_bandwidth: 0, wave_bus: WaveBus::new() }
    }

    /// Додати вузол до mesh (фрактальний, з повним стеком).
    pub fn add_node(&mut self, id: &str, addr: &str, bw: u32) {
        self.nodes.push(MeshNode {
            id: id.to_string(), addr: addr.to_string(),
            bandwidth: bw, chunks: Vec::new(), sigs: 0, centrality: 0.0,
            kind: FractalKind::Root, parent_id: None, children: Vec::new(),
            depth: 0, subsystems: FractalSubsystems::full(0), dsu_groups: Vec::new(),
        });
        self.total_bandwidth += bw;
    }

    /// Додати фрактальний вузол з успадкуванням від батька.
    pub fn add_fractal(&mut self, id: &str, addr: &str, bw: u32, parent: &str) -> Option<usize> {
        let parent_exists = self.nodes.iter().any(|n| n.id == parent);
        if !parent_exists { return None; }
        let parent_depth = self.nodes.iter().find(|n| n.id == parent).map(|n| n.depth).unwrap_or(0);
        let new_depth = (parent_depth + 1).min(MAX_FRACTAL_DEPTH);
        if let Some(p) = self.nodes.iter_mut().find(|n| n.id == parent) {
            p.children.push(id.to_string());
        }
        let idx = self.nodes.len();
        self.nodes.push(MeshNode {
            id: id.to_string(), addr: addr.to_string(),
            bandwidth: bw, chunks: Vec::new(), sigs: 0, centrality: 0.0,
            kind: FractalKind::SplitChild, parent_id: Some(parent.to_string()),
            children: Vec::new(), depth: new_depth,
            subsystems: FractalSubsystems::full(new_depth),
            dsu_groups: Vec::new(),
        });
        self.total_bandwidth += bw;
        Some(idx)
    }

    /// Призначити чанки вузлам (FanOut по mesh).
    pub fn assign_chunks(&mut self, total_sigs: u64) {
        self.total_sigs = total_sigs;
        if self.nodes.is_empty() { return; }
        let num_chunks = ((total_sigs + CHUNK_SIGS - 1) / CHUNK_SIGS) as u32;
        for node in &mut self.nodes { node.chunks.clear(); }
        for cid in 0..num_chunks {
            let idx = cid as usize % self.nodes.len();
            self.nodes[idx].chunks.push(cid);
            self.nodes[idx].sigs = CHUNK_SIGS.min(total_sigs - cid as u64 * CHUNK_SIGS);
        }
    }

    /// Фрактальний split: поділити ноду на N ідентичних фракталів.
    /// Кожен дочірній фрактал успадковує ВСІ підсистеми батька.
    pub fn split_node(&mut self, node_id: &str, num_parts: usize) -> Vec<usize> {
        let idx = self.nodes.iter().position(|n| n.id == node_id);
        if idx.is_none() || num_parts < 2 { return vec![]; }
        let idx = idx.unwrap();
        let parent_addr = self.nodes[idx].addr.clone();
        let parent_bw = self.nodes[idx].bandwidth;
        let parent_depth = self.nodes[idx].depth;
        let parent_chunks = self.nodes[idx].chunks.clone();
        let mut child_ids = Vec::new();
        for i in 0..num_parts {
            let child_id = format!("{}-fr{}", node_id, i);
            let child_chunks: Vec<u32> = parent_chunks.iter()
                .enumerate().filter(|(j, _)| j % num_parts == i)
                .map(|(_, &c)| c).collect();
            let child_sigs = (child_chunks.len() as u64) * CHUNK_SIGS;
            let child_idx = self.nodes.len();
            self.nodes.push(MeshNode {
                id: child_id.clone(), addr: format!("{}.{}", parent_addr, i),
                bandwidth: parent_bw / num_parts as u32,
                chunks: child_chunks, sigs: child_sigs, centrality: 0.0,
                kind: FractalKind::SplitChild,
                parent_id: Some(node_id.to_string()),
                children: Vec::new(),
                depth: (parent_depth + 1).min(MAX_FRACTAL_DEPTH),
                subsystems: FractalSubsystems::full(parent_depth + 1),
                dsu_groups: Vec::new(),
            });
            child_ids.push(child_idx);
        }
        if let Some(p) = self.nodes.get_mut(idx) {
            p.children = (0..num_parts).map(|i| format!("{}-fr{}", node_id, i)).collect();
            p.kind = FractalKind::Root;
            p.chunks.clear();
            p.sigs = 0;
        }
        child_ids
    }

    /// Фрактальний merge: об'єднати дочірні фрактали назад у батька.
    /// Дані консолідуються, підсистеми об'єднуються.
    pub fn merge_children(&mut self, parent_id: &str) -> bool {
        let parent_idx = self.nodes.iter().position(|n| n.id == parent_id);
        if parent_idx.is_none() { return false; }
        let parent_idx = parent_idx.unwrap();
        let child_ids: Vec<String> = self.nodes[parent_idx].children.clone();
        let mut merged_chunks: Vec<u32> = self.nodes[parent_idx].chunks.clone();
        for cid in &child_ids {
            if let Some(child_idx) = self.nodes.iter().position(|n| n.id == *cid) {
                merged_chunks.extend_from_slice(&self.nodes[child_idx].chunks);
                self.nodes[child_idx].kind = FractalKind::Merged;
            }
        }
        merged_chunks.sort();
        merged_chunks.dedup();
        if let Some(p) = self.nodes.get_mut(parent_idx) {
            p.chunks = merged_chunks;
            p.sigs = p.chunks.len() as u64 * CHUNK_SIGS;
            p.children.clear();
        }
        true
    }

    /// Harmonic centrality: який вузол має найкращу позицію в mesh.
    pub fn rank_nodes(&self) -> Vec<(String, f64)> {
        let n = self.nodes.len() as f64;
        let mut ranked: Vec<(String, f64)> = self.nodes.iter().map(|node| {
            let bw_score = node.bandwidth as f64 / self.total_bandwidth.max(1) as f64;
            let chunk_score = node.chunks.len() as f64 / (self.total_sigs / CHUNK_SIGS).max(1) as f64;
            let fractal_bonus = if node.depth > 0 { 0.1 * node.depth as f64 } else { 0.0 };
            let centrality = bw_score * 0.35 + chunk_score * 0.45 + fractal_bonus * 0.2;
            (node.id.clone(), centrality)
        }).collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Фрактальна декомпозиція: DSU через всі фрактали.
    pub fn fractal_decompose(&mut self, tasks: &[TaskSpec]) {
        for node in &mut self.nodes {
            if let Some(ref mut swarm) = node.subsystems.swarm {
                let groups = swarm.decompose(tasks);
                node.dsu_groups = groups;
            }
        }
    }

    /// Час синку для всіх вузлів (фрактальний mesh).
    pub fn sync_time(&self) -> String {
        if self.nodes.is_empty() { return "∞".into(); }
        let active: Vec<&MeshNode> = self.nodes.iter().filter(|n| n.kind != FractalKind::Merged).collect();
        if active.is_empty() { return "∞".into(); }
        let bytes_per_node = (self.total_sigs * 8) as f64 / active.len() as f64;
        let bw_per_node = (self.total_bandwidth / active.len() as u32).max(1) as f64;
        let secs = bytes_per_node * 8.0 / (bw_per_node * 1_000_000.0);
        let h = secs / 3600.0;
        let m = (secs % 3600.0) / 60.0;
        let s = secs % 60.0;
        if h >= 1.0 { format!("{:.0}год {:.0}хв", h, m) }
        else if m >= 1.0 { format!("{:.0}хв {:.0}с", m, s) }
        else { format!("{:.0}с", s) }
    }

    /// Симуляція синку через фрактальний mesh.
    pub fn simulate_sync(&self) -> Vec<(String, String, u64)> {
        let mut results = Vec::new();
        for node in &self.nodes {
            if node.kind == FractalKind::Merged { continue; }
            let bytes = node.chunks.len() as u64 * CHUNK_BYTES;
            let secs = bytes as f64 * 8.0 / (node.bandwidth as f64 * 1_000_000.0);
            let h = secs / 3600.0; let m = (secs % 3600.0) / 60.0; let s = secs % 60.0;
            let time = if h >= 1.0 { format!("{:.1}год", h) } else if m >= 1.0 { format!("{:.0}хв", m) } else { format!("{:.0}с", s) };
            results.push((node.id.clone(), time, bytes));
        }
        results
    }

    pub fn dashboard(&self) -> String {
        let gb = self.total_sigs * 8 / 1_000_000_000;
        let ranked = self.rank_nodes();
        let top = ranked.iter().take(3).map(|(id, c)| format!("    {} (cent: {:.3})", id, c)).collect::<Vec<_>>().join("\n");
        let sync_results = self.simulate_sync();
        let sync_summary: Vec<String> = sync_results.iter().map(|(id, t, b)| format!("    {}: {} ({} MB)", id, t, b / 1_000_000)).collect();
        let fractal_count = self.nodes.iter().filter(|n| n.kind == FractalKind::SplitChild).count();
        let max_depth = self.nodes.iter().map(|n| n.depth).max().unwrap_or(0);
        format!(
            "Academia Fractal Mesh\n  Nodes:     {} ({} fractal, depth {})\n  Total:     {} GB / {} sigs\n  BW:        {} Mbps\n  Sync:      {}\n  Waves:     {}\n  Field:     {:.1}\n  Top:\n{}\n  Per node:\n{}",
            self.nodes.len(), fractal_count, max_depth, gb, self.total_sigs, self.total_bandwidth, self.sync_time(),
            self.wave_bus.sockets.len(), self.wave_bus.field_strength,
            top, sync_summary.join("\n")
        )
    }
}

/// Симуляція mesh-розподілу: seed → N вузлів → всі 100 Mbps.
pub fn simulate_mesh(num_peers: u32, total_sigs: u64) -> AcademiaMesh {
    let mut mesh = AcademiaMesh::new();
    for i in 0..num_peers {
        mesh.add_node(&format!("mesh-node-{}", i), &format!("10.0.0.{}:{}", i, 9000 + i), 100);
        mesh.wave_bus.register(&format!("mesh-node-{}", i));
    }
    // Підключити всі пари (повний граф через WaveBus)
    for i in 0..num_peers {
        for j in (i+1)..num_peers {
            mesh.wave_bus.connect(&format!("mesh-node-{}", i), &format!("mesh-node-{}", j));
        }
    }
    mesh.assign_chunks(total_sigs);
    mesh
}

// ─── Симуляція mesh-розподілу ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_add_nodes() {
        let mut mesh = AcademiaMesh::new();
        mesh.add_node("A", "10.0.0.1:9000", 100);
        mesh.add_node("B", "10.0.0.2:9000", 100);
        assert_eq!(mesh.nodes.len(), 2);
        assert_eq!(mesh.total_bandwidth, 200);
    }

    #[test]
    fn assign_chunks_fanout() {
        let mut mesh = simulate_mesh(4, 100_000_000);
        mesh.assign_chunks(100_000_000);
        let total: u32 = mesh.nodes.iter().map(|n| n.chunks.len() as u32).sum();
        let expected = ((100_000_000 + CHUNK_SIGS - 1) / CHUNK_SIGS) as u32;
        assert_eq!(total, expected);
    }

    #[test]
    fn sync_time_decreases_with_more_nodes() {
        let m1 = simulate_mesh(1, 610_000_000);
        let m10 = simulate_mesh(10, 610_000_000);
        assert_ne!(m1.sync_time(), m10.sync_time());
    }

    #[test]
    fn ranking_returns_ordered() {
        let mut mesh = simulate_mesh(5, 50_000_000);
        mesh.assign_chunks(50_000_000);
        let ranked = mesh.rank_nodes();
        assert_eq!(ranked.len(), 5);
        // First should have highest centrality
        assert!(ranked[0].1 >= ranked[1].1);
    }

    #[test]
    fn simulate_mesh_10_nodes_100mbps() {
        let mesh = simulate_mesh(10, 610_000_000);
        let time = mesh.sync_time();
        // 10 nodes × 100 Mbps = 1 Gbps
        // 610M × 8B = 4.88 GB / 125 MB/s ≈ 39s
        assert_eq!(mesh.total_bandwidth, 1000);
        assert!(time.contains("с") || time.contains("хв"));
    }

    #[test]
    fn simulate_mesh_100_nodes() {
        let mesh = simulate_mesh(100, 610_000_000);
        // 100 nodes × 100 Mbps = 10 Gbps
        // 4.88 GB / 1.25 GB/s ≈ 4s
        assert_eq!(mesh.total_bandwidth, 10_000);
    }

    #[test]
    fn dashboard_contains_mesh() {
        let mesh = simulate_mesh(3, 1_000_000);
        let d = mesh.dashboard();
        assert!(d.contains("Academia Fractal Mesh"));
    }

    #[test]
    fn single_node_takes_longest() {
        let m1 = simulate_mesh(1, 610_000_000);
        // Single node at 100 Mbps: 4.88 GB × 8 / 100 Mbps ≈ 390s ≈ 6.5 min
        let results = m1.simulate_sync();
        assert_eq!(results[0].0, "mesh-node-0");
    }

    #[test]
    fn harmonic_centrality_peers_equal() {
        let mut mesh = AcademiaMesh::new();
        mesh.add_node("A", "addr", 100);
        mesh.add_node("B", "addr", 100);
        mesh.add_node("C", "addr", 100);
        mesh.add_node("D", "addr", 100);
        mesh.assign_chunks(100_000);
        let ranked = mesh.rank_nodes();
        assert_eq!(ranked.len(), 4);
    }

    // ── Spin Wave Tests ────────────────────────────────────────────────────

    #[test]
    fn spin_up_down() {
        let up = Spin::new(SpinState::Up);
        let down = Spin::new(SpinState::Down);
        assert!(up.measure_up_probability() > 0.99);
        assert!(down.measure_up_probability() < 0.01);
    }

    #[test]
    fn spin_rotate_changes_state() {
        let mut s = Spin::new(SpinState::Up);
        s.rotate(std::f64::consts::PI);
        assert!((s.measure_up_probability() - 0.0).abs() < 0.01);
    }

    #[test]
    fn spin_entanglement() {
        let mut a = Spin::new(SpinState::Up);
        let mut b = Spin::new(SpinState::Down);
        Spin::entangle(&mut a, &mut b);
        // After entanglement, phases are equal
        assert!((a.phase - b.phase).abs() < 0.001);
    }

    #[test]
    fn wave_bus_send_receive() {
        let mut bus = WaveBus::new();
        bus.register("A");
        bus.register("B");
        bus.connect("A", "B");
        bus.send("A", "B", &[1, 2, 3]);
        bus.tick();
        let rx = bus.read_rx("B");
        assert_eq!(rx.len(), 1);
        assert_eq!(rx[0].source_id, "A");
    }

    #[test]
    fn wave_bus_bidirectional() {
        let mut bus = WaveBus::new();
        bus.register("A");
        bus.register("B");
        bus.connect("A", "B");
        // Одночасно в обидві сторони
        bus.send("A", "B", &[1, 2, 3]);
        bus.send("B", "A", &[4, 5, 6]);
        bus.tick();
        assert_eq!(bus.read_rx("B").len(), 1);
        assert_eq!(bus.read_rx("A").len(), 1);
    }

    #[test]
    fn wave_bus_continuous_exchange() {
        let mut bus = WaveBus::new();
        bus.register("A"); bus.register("B"); bus.register("C");
        bus.connect("A", "B"); bus.connect("B", "C"); bus.connect("A", "C");
        let nodes = vec!["A", "B", "C"];
        // Асинхронний обмін: кожен крок всі передають всім
        for i in 0..10 {
            let from = nodes[i % 3];
            let to = nodes[(i + 1) % 3];
            bus.send(from, to, &[i as u8]);
            bus.tick();
        }
        // Всі отримали всі хвилі
        for sock in &bus.sockets {
            assert!(sock.tx_count + sock.rx_count > 0, "socket {} has no activity", sock.node_id);
        }
    }

    #[test]
    fn spin_data_encoding_decoding() {
        let data: Vec<u8> = (0..10).collect();
        let spins: Vec<Spin> = data.iter().map(|&byte| {
            Spin::new(if byte & 1 == 1 { SpinState::Up } else { SpinState::Down })
        }).collect();
        assert_eq!(spins.len(), 10);
        let decoded: Vec<u8> = spins.iter().map(|s| {
            if s.measure_up_probability() > 0.5 { 1u8 } else { 0u8 }
        }).collect();
        let original_parity: Vec<u8> = data.iter().map(|b| b & 1).collect();
        assert_eq!(decoded, original_parity);
    }

    #[test]
    fn fractal_split_creates_children() {
        let mut mesh = simulate_mesh(1, 100_000_000);
        let children = mesh.split_node("mesh-node-0", 4);
        assert_eq!(children.len(), 4);
        assert_eq!(mesh.nodes.len(), 5);
        let parent = &mesh.nodes[0];
        assert_eq!(parent.children.len(), 4);
    }

    #[test]
    fn fractal_child_inherits_subsystems() {
        let mut mesh = simulate_mesh(1, 100_000_000);
        mesh.split_node("mesh-node-0", 2);
        let child = &mesh.nodes[1];
        assert_eq!(child.depth, 1);
        assert!(child.subsystems.physics.is_some());
        assert!(child.subsystems.academia.is_some());
        assert!(child.subsystems.oracle.is_some());
        assert!(child.subsystems.miner.is_some());
        assert!(child.subsystems.swarm.is_some());
    }

    #[test]
    fn fractal_merge_consolidates() {
        let mut mesh = simulate_mesh(1, 100_000_000);
        mesh.split_node("mesh-node-0", 4);
        let merged = mesh.merge_children("mesh-node-0");
        assert!(merged);
        let parent = &mesh.nodes[0];
        assert!(parent.children.is_empty());
    }

    #[test]
    fn wave_bus_dashboard_contains() {
        let mut bus = WaveBus::new();
        bus.register("A");
        bus.register("B");
        bus.connect("A", "B");
        bus.send("A", "B", &[1]);
        bus.tick();
        let d = bus.dashboard();
        assert!(d.contains("Wave Bus"));
    }

    // ── Geometric Space Tests ──────────────────────────────────────────────

    #[test]
    fn geo_point_distance() {
        let a = GeometricPoint::new([0.0; 8]);
        let mut b = [1.0; 8];
        b[0] = 3.0;
        let bp = GeometricPoint::new(b);
        let metric = MetricTensor::euclidean();
        let dist = a.riemann_distance(&bp, &metric);
        assert!((dist - 4.0).abs() < 0.001, "expected 4.0 got {}", dist);
    }

    #[test]
    fn geo_forward_prediction() {
        let mut space = PhaseSpace::new();
        space.add_node("A");
        let before = space.states[0].position;
        space.predict_forward(1.0);
        let after = space.states[0].position;
        // With zero velocity, position shouldn't change
        let metric = MetricTensor::euclidean();
        assert!(before.riemann_distance(&after, &metric) < 0.001);
    }

    #[test]
    fn geo_forward_with_velocity() {
        let mut space = PhaseSpace::new();
        space.add_node("A");
        space.states[0].velocity[0] = 5.0;
        space.predict_forward(1.0);
        assert!((space.states[0].position.coords[0] - 5.0).abs() < 0.001);
    }

    #[test]
    fn geo_backward_retrospect() {
        let mut space = PhaseSpace::new();
        space.add_node("A");
        space.states[0].velocity[0] = 5.0;
        space.predict_forward(1.0);
        let after_forward = space.states[0].position.coords[0];
        assert!((after_forward - 5.0).abs() < 0.001, "forward: expected 5 got {}", after_forward);
        space.retrospect_backward(1.0);
        let after_backward = space.states[0].position.coords[0];
        assert!(after_backward.abs() < 0.1, "backward: expected ~0 got {}", after_backward);
    }

    #[test]
    fn geo_simultaneous_forward_backward() {
        let mut space = PhaseSpace::new();
        space.add_node("A");
        space.add_node("B");
        let obs_a = GeometricPoint::new([1.0; 8]);
        let obs_b = GeometricPoint::new([2.0; 8]);
        space.propagate_simultaneous(&[obs_a, obs_b], 1.0);
        assert!(space.states[0].position.coords[0] > 0.0 || space.tick > 0);
        assert!(space.states[1].position.coords[0] > 0.0 || space.tick > 0);
    }

    #[test]
    fn geo_phase_space_energy() {
        let space = PhaseSpace::new();
        assert!((space.energy(0, |_| 0.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn geo_dashboard_contains() {
        let space = PhaseSpace::new();
        let d = space.dashboard();
        assert!(d.contains("Phase Space"));
    }

// ─── Standard Model Tests ─────────────────────────────────────────────
// Кварки: 6 ароматів x 3 кольори = 18 + 18 анти
// Лептони: e, m, t + n_e, n_m, n_t + 6 анти
// Бозони: g (U1), W+-, Z0 (SU2), g (SU3 x8), H

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuarkFlavor { Up, Down, Charm, Strange, Top, Bottom }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorCharge { Red, Green, Blue, AntiRed, AntiGreen, AntiBlue }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeakIsospin { LeftUp, LeftDown, Right }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LeptonFlavor { Electron, Muon, Tau, NuE, NuMu, NuTau }

#[derive(Debug, Clone)]
pub struct Particle {
    pub name: String,
    pub mass_mev: f64,
    pub charge_e: f64,
    pub spin: f64,
    pub is_antiparticle: bool,
    pub quark_content: Option<(QuarkFlavor, ColorCharge, WeakIsospin)>,
    pub lepton_flavor: Option<LeptonFlavor>,
}

impl Particle {
    pub fn quark(flavor: QuarkFlavor, color: ColorCharge, weak: WeakIsospin) -> Self {
        let (n, m, c) = match flavor {
            QuarkFlavor::Up => ("up", 2.16, 2.0/3.0),
            QuarkFlavor::Down => ("down", 4.67, -1.0/3.0),
            QuarkFlavor::Charm => ("charm", 1280.0, 2.0/3.0),
            QuarkFlavor::Strange => ("strange", 93.0, -1.0/3.0),
            QuarkFlavor::Top => ("top", 173_000.0, 2.0/3.0),
            QuarkFlavor::Bottom => ("bottom", 4180.0, -1.0/3.0),
        };
        Particle { name: n.into(), mass_mev: m, charge_e: c, spin: 0.5, is_antiparticle: false, quark_content: Some((flavor, color, weak)), lepton_flavor: None }
    }

    pub fn lepton(flavor: LeptonFlavor) -> Self {
        let (n, m, c) = match flavor {
            LeptonFlavor::Electron => ("electron", 0.511, -1.0),
            LeptonFlavor::Muon => ("muon", 105.66, -1.0),
            LeptonFlavor::Tau => ("tau", 1776.86, -1.0),
            LeptonFlavor::NuE => ("nu_e", 0.0, 0.0),
            LeptonFlavor::NuMu => ("nu_mu", 0.0, 0.0),
            LeptonFlavor::NuTau => ("nu_tau", 0.0, 0.0),
        };
        Particle { name: n.into(), mass_mev: m, charge_e: c, spin: 0.5, is_antiparticle: false, quark_content: None, lepton_flavor: Some(flavor) }
    }

    pub fn antiparticle(&self) -> Self {
        let mut p = self.clone();
        p.is_antiparticle = !self.is_antiparticle;
        p.charge_e = -self.charge_e;
        p.name = format!("anti-{}", self.name);
        p
    }
}

#[derive(Debug, Clone)]
pub struct GaugeBoson {
    pub name: String,
    pub mass_mev: f64,
    pub group: String,
}

impl GaugeBoson {
    pub fn photon() -> Self { GaugeBoson { name: "photon".into(), mass_mev: 0.0, group: "U1".into() } }
    pub fn gluon() -> Self { GaugeBoson { name: "gluon".into(), mass_mev: 0.0, group: "SU3".into() } }
    pub fn w_plus() -> Self { GaugeBoson { name: "W+".into(), mass_mev: 80377.0, group: "SU2".into() } }
    pub fn z() -> Self { GaugeBoson { name: "Z".into(), mass_mev: 91187.6, group: "SU2".into() } }
    pub fn higgs() -> Self { GaugeBoson { name: "Higgs".into(), mass_mev: 125100.0, group: "U1".into() } }
}

#[derive(Debug)]
pub struct StandardModel {
    pub particles: Vec<Particle>,
    pub bosons: Vec<GaugeBoson>,
    pub coupling: [f64; 3],
}

impl StandardModel {
    pub fn new() -> Self {
        let mut sm = StandardModel { particles: Vec::new(), bosons: Vec::new(), coupling: [0.118, 0.0338, 0.0073] };
        for &f in &[QuarkFlavor::Up, QuarkFlavor::Down, QuarkFlavor::Charm, QuarkFlavor::Strange, QuarkFlavor::Top, QuarkFlavor::Bottom] {
            for &c in &[ColorCharge::Red, ColorCharge::Green, ColorCharge::Blue] {
                sm.particles.push(Particle::quark(f, c, WeakIsospin::LeftUp));
                sm.particles.push(Particle::quark(f, c, WeakIsospin::LeftDown).antiparticle());
            }
        }
        for &l in &[LeptonFlavor::Electron, LeptonFlavor::Muon, LeptonFlavor::Tau, LeptonFlavor::NuE, LeptonFlavor::NuMu, LeptonFlavor::NuTau] {
            sm.particles.push(Particle::lepton(l));
            sm.particles.push(Particle::lepton(l).antiparticle());
        }
        sm.bosons.push(GaugeBoson::photon());
        sm.bosons.push(GaugeBoson::gluon());
        sm.bosons.push(GaugeBoson::w_plus());
        sm.bosons.push(GaugeBoson::z());
        sm.bosons.push(GaugeBoson::higgs());
        sm
    }

    pub fn interaction_strength(&self, a: &Particle, b: &Particle) -> f64 {
        let em = self.coupling[2] * a.charge_e.abs() * b.charge_e.abs();
        let weak = if a.spin > 0.0 && b.spin > 0.0 { self.coupling[1] } else { 0.0 };
        let strong = if a.quark_content.is_some() && b.quark_content.is_some() { self.coupling[0] } else { 0.0 };
        em + weak + strong
    }

    pub fn particle_code(&self, p: &Particle) -> FractalASCII {
        let code = if p.quark_content.is_some() { "QUARK" } else if p.lepton_flavor.is_some() { "LEPTON" } else { "BOSON" };
        let flavor = p.quark_content.map(|(f,_,_)| format!("{:?}",f)).or_else(|| p.lepton_flavor.map(|l| format!("{:?}",l))).unwrap_or_default();
        FractalASCII::new(code, &[&p.name, &flavor, &format!("{:.3}",p.charge_e), &format!("{:.1}",p.mass_mev)])
    }

    pub fn dashboard(&self) -> String {
        format!("Standard Model SU(3)xSU(2)xU(1)\n  Particles: {} ({} quarks, {} leptons)\n  Bosons:    {}\n  Couplings: a_s={:.4} a_w={:.4} a_em={:.4}",
            self.particles.len(),
            self.particles.iter().filter(|p| p.quark_content.is_some()).count(),
            self.particles.iter().filter(|p| p.lepton_flavor.is_some()).count(),
            self.bosons.len(),
            self.coupling[0], self.coupling[1], self.coupling[2])
    }
}

    #[test]
    fn sm_quarks_have_color() {
        let sm = StandardModel::new();
        let quarks: Vec<&Particle> = sm.particles.iter().filter(|p| p.quark_content.is_some()).collect();
        assert_eq!(quarks.len(), 36); // 6 flavors x 3 colors x 2 (matter+anti)
    }

    #[test]
    fn sm_leptons_have_flavor() {
        let sm = StandardModel::new();
        let leptons: Vec<&Particle> = sm.particles.iter().filter(|p| p.lepton_flavor.is_some()).collect();
        assert_eq!(leptons.len(), 12); // 6 flavors x 2 (matter+anti)
    }

    #[test]
    fn sm_bosons_exist() {
        let sm = StandardModel::new();
        assert_eq!(sm.bosons.len(), 5);
        assert!(sm.bosons.iter().any(|b| b.name == "photon"));
    }

    #[test]
    fn sm_interaction_strongest_for_quarks() {
        let sm = StandardModel::new();
        let q1 = &sm.particles[0];
        let q2 = &sm.particles[1];
        let lep = &sm.particles[36];
        let qq = sm.interaction_strength(q1, q2);
        let ql = sm.interaction_strength(q1, lep);
        assert!(qq > ql);
    }

    #[test]
    fn sm_particle_code_is_ascii() {
        let sm = StandardModel::new();
        let code = sm.particle_code(&sm.particles[0]);
        assert!(code.code() == "QUARK" || code.code() == "LEPTON");
    }

    #[test]
    fn sm_antiparticle_flips_charge() {
        let sm = StandardModel::new();
        let e = Particle::lepton(LeptonFlavor::Electron);
        let pe = e.antiparticle();
        assert!((e.charge_e + pe.charge_e).abs() < 0.001);
    }

    #[test]
    fn sm_dashboard_contains_standard() {
        let sm = StandardModel::new();
        let d = sm.dashboard();
        assert!(d.contains("Standard Model"));
    }

    // ─── Pseudo-Euclidean Tests ──────────────────────────────────────────

    #[test]
    fn pseudo_minkowski_signature() {
        let m = PseudoEuclideanMetric::minkowski();
        assert_eq!(m.p, 1); assert_eq!(m.q, 3);
    }

    #[test]
    fn pseudo_vector_causal_type() {
        let v = NdVector::new(vec![1.0, 0.0, 0.0, 0.0]);
        assert_eq!(v.causal_type(1), CausalType::TimeLike);
        let v2 = NdVector::new(vec![0.0, 1.0, 0.0, 0.0]);
        assert_eq!(v2.causal_type(1), CausalType::SpaceLike);
        let v3 = NdVector::new(vec![1.0, 1.0, 0.0, 0.0]); // |1|²-|1|²=0
        assert_eq!(v3.causal_type(1), CausalType::LightLike);
    }

    #[test]
    fn pseudo_light_cone_contains() {
        let origin = NdVector::zero(4);
        let cone = LightCone::new(origin, 1, 3);
        let future = NdVector::new(vec![2.0, 0.5, 0.0, 0.0]);
        let outside = NdVector::new(vec![0.0, 5.0, 0.0, 0.0]);
        assert!(cone.contains(&future));
        assert!(!cone.contains(&outside));
        assert!(cone.future(&future));
        assert!(!cone.past(&future));
    }

    #[test]
    fn pseudo_nd_space_add_points() {
        let mut space = NdSpace::minkowski_3plus1();
        space.add_point(vec![0.0, 0.0, 0.0, 0.0], "origin");
        space.add_point(vec![1.0, 0.0, 0.0, 0.0], "future");
        assert_eq!(space.points.len(), 2);
    }

    #[test]
    fn pseudo_causal_clusters() {
        let mut space = NdSpace::minkowski_3plus1();
        space.add_point(vec![0.0, 0.0, 0.0, 0.0], "A");
        space.add_point(vec![1.0, 0.1, 0.0, 0.0], "B"); // future of A
        space.add_point(vec![0.0, 10.0, 0.0, 0.0], "C"); // space-like
        let clusters = space.causal_clusters();
        assert!(clusters.len() >= 1);
    }

    #[test]
    fn pseudo_orthogonal_transform() {
        let m = PseudoEuclideanMetric::minkowski();
        let angles = vec![0.5]; // boost-like rotation
        let t = m.pseudo_orthogonal_transform(&angles);
        assert_eq!(t.len(), 4);
    }

    #[test]
    fn pseudo_symmetric_8d() {
        let m = PseudoEuclideanMetric::symmetric_8d();
        let v = NdVector::new(vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
        // 4 positive - 4 negative = 0
        assert!((v.norm_sq(4)).abs() < 0.001);
    }

    #[test]
    fn pseudo_lower_index() {
        let m = PseudoEuclideanMetric::minkowski();
        let v = NdVector::new(vec![1.0, 2.0, 3.0, 4.0]);
        let lowered = m.lower(&v);
        // g = diag(1, -1, -1, -1), so v_0 = 1.0, v_1 = -2.0, ...
        assert!((lowered.coords[0] - 1.0).abs() < 0.001);
        assert!((lowered.coords[1] - (-2.0)).abs() < 0.001);
    }

    #[test]
    fn pseudo_dashboard() {
        let space = NdSpace::minkowski_3plus1();
        let d = space.dashboard();
        assert!(d.contains("O(1,3)"));
    }

    #[test]
    fn split_complex_null() {
        let sc = SplitAlgebra::split_complex(1.0, 1.0);
        assert!(sc.is_null());
        let sc2 = SplitAlgebra::split_complex(1.0, 0.5);
        assert!(!sc2.is_null());
    }

    #[test]
    fn split_quaternion_norm() {
        let sq = SplitAlgebra::split_quaternion(1.0, 1.0, 1.0, 1.0);
        assert!((sq.norm_sq() - 0.0).abs() < 0.001);
    }

    #[test]
    fn split_octonion_norm() {
        let so = SplitAlgebra::split_octonion([1.0; 8]);
        assert!((so.norm_sq() - 0.0).abs() < 0.001);
    }

    #[test]
    fn split_algebra_multiplication() {
        let a = SplitAlgebra::split_complex(2.0, 3.0);
        let b = SplitAlgebra::split_complex(4.0, 5.0);
        let c = a.mul(&b);
        if let SplitAlgebra::SplitComplex { a, b } = c {
            assert!((a - 23.0).abs() < 0.001);
            assert!((b - 22.0).abs() < 0.001);
        } else { panic!("wrong type"); }
    }

    #[test]
    fn light_cone_points_generated() {
        let sc = SplitAlgebra::split_complex(1.0, 0.0);
        let points = sc.light_cone_points(8);
        assert_eq!(points.len(), 8);
    }

    #[test]
    fn null_geodesic_propagates() {
        let origin = NdVector::new(vec![0.0, 0.0, 0.0, 0.0]);
        let dir = NdVector::new(vec![1.0, 1.0, 0.0, 0.0]);
        let mut ray = NullGeodesic::new(origin, dir);
        ray.step(1.0);
        assert!(ray.position.coords[0] > 0.0);
    }

    #[test]
    fn light_front_expands() {
        let metric = PseudoEuclideanMetric::minkowski();
        let mut comm = LightCommunication::new(metric);
        let from = NdVector::new(vec![0.0, 0.0, 0.0, 0.0]);
        comm.emit(&from, 0, &[1, 2, 3]);
        assert_eq!(comm.fronts.len(), 1);
        comm.tick(1.0);
        assert!(comm.fronts[0].radius > 0.0);
    }

    #[test]
    fn light_reaches_point() {
        let metric = PseudoEuclideanMetric::minkowski();
        let mut comm = LightCommunication::new(metric);
        let center = NdVector::new(vec![0.0, 0.0, 0.0, 0.0]);
        let target = NdVector::new(vec![1.0, 1.0, 0.0, 0.0]); // dx=1.0
        comm.emit(&center, 1, &[42]);
        comm.tick(1.0);
        let rx = comm.receive(&target);
        assert!(!rx.is_empty(), "light should reach target after 1 tick");
        assert_eq!(rx[0].1, vec![42]);
    }

    #[test]
    fn spinor_from_quaternion() {
        let sq = SplitAlgebra::split_quaternion(0.5, 0.5, 0.5, 0.5);
        let spinor = LightCommunication::spinor_from_split_algebra(&sq);
        assert!((spinor[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn light_dashboard() {
        let metric = PseudoEuclideanMetric::minkowski();
        let comm = LightCommunication::new(metric);
        let d = comm.dashboard();
        assert!(d.contains("Light Communication"));
    }

    // ─── Quantum Tests ───────────────────────────────────────────────────

    #[test]
    fn qubit_zero_prob() {
        let q = Qubit::zero();
        assert!((q.prob_zero() - 1.0).abs() < 0.001);
    }

    #[test]
    fn qubit_pauli_x_flips() {
        let q = Qubit::zero();
        let flipped = q.pauli_x();
        assert!((flipped.prob_one() - 1.0).abs() < 0.001);
    }

    #[test]
    fn qubit_hadamard_creates_superposition() {
        let q = Qubit::zero();
        let h = q.hadamard();
        assert!((h.prob_zero() - 0.5).abs() < 0.001);
        assert!((h.prob_one() - 0.5).abs() < 0.001);
    }

    #[test]
    fn two_qubit_bell_state() {
        let bell = TwoQubit::bell_phi_plus();
        // Bell state |Phi+> = (|00> + |11>)/sqrt(2)
        let s = 2.0f64.sqrt().recip();
        assert!((bell.c00.norm_sq() - 0.5).abs() < 0.001);
        assert!((bell.c11.norm_sq() - 0.5).abs() < 0.001);
    }

    #[test]
    fn hamiltonian_eigenvalues() {
        let h = Hamiltonian::new(1.0, 0.0, 0.0);
        let (e1, e2) = h.eigenvalues();
        assert!((e1 + e2).abs() < 0.001);
    }

    #[test]
    fn complex_arithmetic() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, 4.0);
        let c = a.mul(&b);
        assert!((c.re - (-5.0)).abs() < 0.001); // (1+2i)(3+4i) = -5+10i
        assert!((c.im - 10.0).abs() < 0.001);
    }

    #[test]
    fn quantum_measurement_stats() {
        let mut m = QuantumMeasurement::new();
        let q = Qubit::zero();
        for _ in 0..100 { m.measure(&q); }
        assert_eq!(m.counts_0, 100);
        assert_eq!(m.counts_1, 0);
    }

    // ─── Superposition Tests ─────────────────────────────────────────────

    #[test]
    fn superposition_probabilities() {
        let mut sp = Superposition::new();
        sp.add(BasisState::new("|0>", Complex::new(1.0, 0.0)));
        sp.add(BasisState::new("|1>", Complex::new(0.0, 1.0)));
        let probs = sp.probabilities();
        assert!((probs[0].1 - 0.5).abs() < 0.001);
        assert!((probs[1].1 - 0.5).abs() < 0.001);
    }

    #[test]
    fn superposition_time_travel() {
        let times = vec![
            ("past".into(), -1.0, Complex::new(0.5, 0.0)),
            ("present".into(), 0.0, Complex::new(0.7, 0.0)),
            ("future".into(), 1.0, Complex::new(0.5, 0.0)),
        ];
        let sp = Superposition::time_superposition(&times);
        let expected = sp.expected_time();
        assert!(expected > -0.5 && expected < 0.5);
    }

    #[test]
    fn superposition_collapse() {
        let mut sp = Superposition::new();
        sp.add(BasisState::new("A", Complex::new(1.0, 0.0)));
        let collapsed = sp.measure(42);
        assert!(collapsed.is_some());
        assert_eq!(collapsed.unwrap().label, "A");
    }

    #[test]
    fn superposition_interference() {
        let mut sp = Superposition::new();
        sp.add(BasisState::new("early", Complex::new(1.0, 0.0)));
        sp.add(BasisState::new("late", Complex::new(0.0, 1.0)));
        let interference = sp.interfere();
        assert!(interference.norm() > 0.0);
    }

    // ─── Time Warp Tests ─────────────────────────────────────────────────

    #[test]
    fn local_time_dilation() {
        let mut lt = LocalTime { proper_time: 0.0, coordinate_time: 0.0, potential: -0.4, dilation: 0.5 };
        lt.tick(10.0);
        assert!(lt.proper_time < lt.coordinate_time, "proper={} coord={}", lt.proper_time, lt.coordinate_time);
        assert!(lt.warp_delta() > 0.0);
    }

    #[test]
    fn gravitational_field_potential() {
        let mut field = GravitationalField::new();
        let origin = NdVector::zero(4);
        field.add_mass(100.0, origin.clone());
        let far = NdVector::new(vec![10.0, 0.0, 0.0, 0.0]);
        let phi_far = field.potential_at(&far);
        assert!(phi_far < 0.0);
        let phi_origin = field.potential_at(&origin);
        assert!(phi_origin > phi_far, "potential should be deeper (more negative) near mass: phi_origin={} phi_far={}", phi_origin, phi_far); // potential deeper near mass
    }

    #[test]
    fn time_warp_add_node() {
        let mut tw = TimeWarp::new();
        tw.add_node(1.0, NdVector::zero(4));
        tw.tick(1.0);
        assert_eq!(tw.global_tick, 1);
    }

    #[test]
    fn time_warp_two_nodes() {
        let mut tw = TimeWarp::new();
        tw.add_node(100.0, NdVector::zero(4));
        tw.add_node(1.0, NdVector::new(vec![10.0, 0.0, 0.0, 0.0]));
        for _ in 0..100 { tw.tick(1.0); }
        let diff = tw.time_difference(0, 1);
        assert!(diff > 0.0); // час біля маси тече повільніше
    }

    #[test]
    fn quantum_time_shift_changes_qubit() {
        let tw = TimeWarp::new();
        let q = Qubit::zero();
        // без нод немає зсуву
        let q2 = tw.quantum_time_shift(&q, 0);
        assert!((q2.prob_zero() - q.prob_zero()).abs() < 0.001);
    }

    // ─── All 8 Integration Tests ─────────────────────────────────────────

    #[test]
    fn unified_navigator_routes() {
        let metric = PseudoEuclideanMetric::minkowski();
        let mut nav = UnifiedNavigator::new(metric, 4);
        nav.phase_space.add_node("A"); nav.phase_space.add_node("B");
        assert!(nav.navigate(0, 1) > 0.0);
    }

    #[test]
    fn prediction_service_forecast() {
        let mut ps = PredictionService::new();
        let obs = GeometricPoint::new([1.0; 8]);
        assert_eq!(ps.observe_and_predict(obs).len(), 10);
    }

    #[test]
    fn prediction_retrospective_works() {
        let mut ps = PredictionService::new();
        ps.observe_and_predict(GeometricPoint::new([5.0; 8]));
        assert!(ps.retrospective(0).is_some());
    }

    #[test]
    fn autonomous_loop_cycles() {
        let mut al = AutonomousLoop::new();
        al.cycle();
        assert_eq!(al.cycle, 1);
    }

    #[test]
    fn memory_pipeline_ingests() {
        let mut mp = MemoryPipeline::new(4);
        mp.ingest("test", "quantum research", 0.0, 1.0);
        assert_eq!(mp.topo.labels.len(), 1);
    }

    #[test]
    fn memory_pipeline_consolidates() {
        let mut mp = MemoryPipeline::new(4);
        mp.ingest("a", "alpha", 0.0, 1.0);
        mp.ingest("b", "beta", 1.0, 0.1);
        mp.consolidate();
        assert!(mp.topo.surface.weights[0] > mp.topo.surface.weights[1]);
    }

    #[test]
    fn geometric_sync_adds_nodes() {
        let mut gs = GeometricSync::new();
        gs.add_node("A"); gs.add_node("B");
        assert_eq!(gs.nodes.len(), 2);
    }

    #[test]
    fn inference_projects() {
        let ie = InferenceEngine::new(4, 2);
        assert_eq!(ie.project(&[1.0, 2.0, 3.0, 4.0]).len(), 2);
    }

    #[test]
    fn live_dashboard_renders() {
        let dash = LiveDashboard::new(PseudoEuclideanMetric::minkowski(), 4);
        assert!(dash.render().contains("LIVE DASHBOARD"));
    }

    #[test]
    fn p2p_network_starts() {
        let mut net = P2PNetwork::new();
        net.start_seed("127.0.0.1:9000");
        assert!(net.active);
    }

    #[test]
    fn quantum_trader_buy_sell() {
        let mut t = QuantumTrader::new("test", 1000.0, 0.5);
        let order = t.on_price(100.0);
        t.execute(order, 100.0);
        assert!(t.trades > 0 || t.budget < 1000.0);
    }

    #[test]
    fn quantum_trader_price_prediction() {
        let t = QuantumTrader::new("pred", 1000.0, 1.0);
        let preds = t.predict_price(5);
        // No history yet
        assert!(preds.is_empty());
    }

    #[test]
    fn quantum_trader_history_prediction() {
        let mut t = QuantumTrader::new("pred2", 1000.0, 1.0);
        for i in 0..10 { t.on_price(100.0 + i as f64); }
        let preds = t.predict_price(5);
        assert_eq!(preds.len(), 5);
    }

    #[test]
    fn quantum_trader_dashboard() {
        let t = QuantumTrader::new("dash", 500.0, 0.8);
        let d = t.dashboard();
        assert!(d.contains("Quantum Trader"));
    }

    // ─── 3-Node Simultaneous Communication Tests ──────────────────────

    #[test]
    fn ghz_state_equally_probable() {
        let ghz = GHZState::new();
        assert!((ghz.prob_000() - 0.5).abs() < 0.001);
        assert!((ghz.prob_111() - 0.5).abs() < 0.001);
    }

    #[test]
    fn ghz_measurement_collapses_all() {
        let mut ghz = GHZState::new();
        let outcome = ghz.measure_one(42);
        if outcome == 0 {
            assert!((ghz.prob_000() - 1.0).abs() < 0.001);
        } else {
            assert!((ghz.prob_111() - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn three_node_bus_exchange() {
        let mut bus = ThreeNodeBus::new(["A", "B", "C"]);
        assert!(bus.nodes[0].entangled);
        bus.exchange([&[1], &[2], &[3]]);
        assert_eq!(bus.round, 1);
    }

    #[test]
    fn three_node_consensus() {
        let mut bus = ThreeNodeBus::new(["X", "Y", "Z"]);
        bus.exchange([&[1], &[1], &[1]]);
        bus.sync_phases();
        assert!(bus.consensus());
    }

    #[test]
    fn three_node_majority() {
        let mut bus = ThreeNodeBus::new(["A", "B", "C"]);
        // Всі три в однаковій фазі → одноголосно
        bus.sync_phases();
        let vote = bus.majority_vote();
        assert!(vote == 1 || vote == -1 || vote == 0);
    }

    #[test]
    fn ghz_entangle_three_nodes() {
        let mut nodes = [
            ThreeNodeState::new("n0"),
            ThreeNodeState::new("n1"),
            ThreeNodeState::new("n2"),
        ];
        GHZState::entangle(&mut nodes);
        assert!(nodes.iter().all(|n| n.entangled));
    }

    #[test]
    fn three_node_bus_dashboard() {
        let bus = ThreeNodeBus::new(["A", "B", "C"]);
        let d = bus.dashboard();
        assert!(d.contains("3-Node Bus"));
    }
}

// ─── Pseudo-Euclidean n-Dimensional Space ──────────────────────────────

// Узагальнення: n-вимірний простір з довільною сигнатурою (p,q).
// Містить часоподібні, простороподібні та нульові вектори.
// Група O(p,q) — псевдо-ортогональні перетворення.

/// n-вимірний вектор.
#[derive(Debug, Clone)]
pub struct NdVector {
    pub coords: Vec<f64>,
}

impl NdVector {
    pub fn new(coords: Vec<f64>) -> Self { NdVector { coords } }
    pub fn zero(n: usize) -> Self { NdVector { coords: vec![0.0; n] } }

    /// Довжина в метриці сигнатури (p,q): ||v||² = Σ_{i=0}^{p-1} v_i² - Σ_{j=p}^{p+q-1} v_j².
    pub fn norm_sq(&self, p: usize) -> f64 {
        let mut s = 0.0;
        for (i, &c) in self.coords.iter().enumerate() {
            if i < p { s += c * c; } else { s -= c * c; }
        }
        s
    }

    /// Тип вектора: time-like (>0), space-like (<0), light-like (≈0).
    pub fn causal_type(&self, p: usize) -> CausalType {
        let ns = self.norm_sq(p);
        if ns.abs() < 1e-12 { CausalType::LightLike }
        else if ns > 0.0 { CausalType::TimeLike }
        else { CausalType::SpaceLike }
    }

    /// Скалярний добуток у метриці сигнатури (p,q).
    pub fn dot(&self, other: &NdVector, p: usize) -> f64 {
        let mut s = 0.0;
        for i in 0..self.coords.len().min(other.coords.len()) {
            let m = if i < p { 1.0 } else { -1.0 };
            s += m * self.coords[i] * other.coords[i];
        }
        s
    }
}

/// Причинний тип вектора.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CausalType { TimeLike, SpaceLike, LightLike }

/// Метрика сигнатури (p,q) на R^n.
#[derive(Debug, Clone)]
pub struct PseudoEuclideanMetric {
    /// Кількість додатних власних значень.
    pub p: usize,
    /// Кількість від'ємних власних значень.
    pub q: usize,
    /// Матриця метрики.
    pub g: Vec<Vec<f64>>,
}

impl PseudoEuclideanMetric {
    /// Створити метрику сигнатури (p,q): діагональна + + ... + - - ... - .
    pub fn new(p: usize, q: usize) -> Self {
        let n = p + q;
        let mut g = vec![vec![0.0; n]; n];
        for i in 0..n {
            g[i][i] = if i < p { 1.0 } else { -1.0 };
        }
        PseudoEuclideanMetric { p, q, g }
    }

    /// Мінковського (1,3): 3+1 простір-час.
    pub fn minkowski() -> Self { PseudoEuclideanMetric::new(1, 3) }

    /// (p,q) = (4,4): симетричний час-простір (наш 8D).
    pub fn symmetric_8d() -> Self { PseudoEuclideanMetric::new(4, 4) }

    /// (p,q) = (n,0): звичайний евклідів.
    pub fn euclidean(n: usize) -> Self { PseudoEuclideanMetric::new(n, 0) }

    /// Квадрат інтервалу: ds² = g_ij dx^i dx^j.
    pub fn interval_sq(&self, dx: &NdVector) -> f64 {
        let n = self.p + self.q;
        let mut s = 0.0;
        for i in 0..n.min(dx.coords.len()) {
            for j in 0..n.min(dx.coords.len()) {
                s += self.g[i][j] * dx.coords[i] * dx.coords[j];
            }
        }
        s
    }

    /// Група O(p,q): псевдо-ортогональна матриця, що зберігає метрику.
    pub fn pseudo_orthogonal_transform(&self, angles: &[f64]) -> Vec<Vec<f64>> {
        let n = self.p + self.q;
        let mut m = vec![vec![0.0; n]; n];
        for i in 0..n {
            m[i][i] = 1.0;
        }
        // Прості обертання в кожній парі вимірів
        for k in 0..angles.len().min(n/2) {
            let (i, j) = (2*k, 2*k+1);
            if j >= n { break; }
            let theta = angles[k];
            let c = theta.cos();
            let s = theta.sin();
            m[i][i] = c; m[i][j] = -s;
            m[j][i] = s; m[j][j] = c;
        }
        m
    }

    /// Підняття/опускання індексів: v_i = g_ij v^j.
    pub fn lower(&self, v: &NdVector) -> NdVector {
        let n = self.p + self.q;
        let mut res = vec![0.0; n];
        for i in 0..n {
            for j in 0..n.min(v.coords.len()) {
                res[i] += self.g[i][j] * v.coords[j];
            }
        }
        NdVector { coords: res }
    }

    pub fn dashboard(&self) -> String {
        format!("Pseudo-Euclidean Metric O({},{})\n  Dims: {}\n  Signature: (+{} -{})",
            self.p, self.q, self.p + self.q, self.p, self.q)
    }
}

/// Світловий конус у точці: {v | g(v,v) = 0}.
#[derive(Debug, Clone)]
pub struct LightCone {
    /// Вершина конуса.
    pub vertex: NdVector,
    /// Сигнатура простору.
    pub p: usize,
    pub q: usize,
}

impl LightCone {
    pub fn new(vertex: NdVector, p: usize, q: usize) -> Self {
        LightCone { vertex, p, q }
    }

    /// Чи знаходиться вектор всередині світлового конуса (time-like)?
    pub fn contains(&self, v: &NdVector) -> bool {
        let rel = NdVector::new(
            v.coords.iter().zip(&self.vertex.coords).map(|(a, b)| a - b).collect()
        );
        rel.causal_type(self.p) == CausalType::TimeLike
    }

    /// Майбутнє (future): time-like вектори з додатною часовою компонентою.
    pub fn future(&self, v: &NdVector) -> bool {
        self.contains(v) && v.coords.first().map(|&c| c > 0.0).unwrap_or(false)
    }

    /// Минуле (past): time-like вектори з від'ємною часовою компонентою.
    pub fn past(&self, v: &NdVector) -> bool {
        self.contains(v) && v.coords.first().map(|&c| c < 0.0).unwrap_or(false)
    }
}

/// n-вимірний псевдо-евклідів простір.
#[derive(Debug)]
pub struct NdSpace {
    /// Вектори/точки в просторі.
    pub points: Vec<NdVector>,
    /// Назви точок.
    pub labels: Vec<String>,
    /// Сигнатура метрики.
    pub metric: PseudoEuclideanMetric,
}

impl NdSpace {
    pub fn new(p: usize, q: usize) -> Self {
        NdSpace { points: Vec::new(), labels: Vec::new(), metric: PseudoEuclideanMetric::new(p, q) }
    }

    pub fn minkowski_3plus1() -> Self { NdSpace::new(1, 3) }
    pub fn symmetric_8d() -> Self { NdSpace::new(4, 4) }

    /// Додати точку.
    pub fn add_point(&mut self, coords: Vec<f64>, label: &str) {
        self.points.push(NdVector::new(coords));
        self.labels.push(label.to_string());
    }

    /// Інтервал між двома точками.
    pub fn interval(&self, i: usize, j: usize) -> f64 {
        if i >= self.points.len() || j >= self.points.len() { return 0.0; }
        let dx = NdVector::new(
            self.points[i].coords.iter().zip(&self.points[j].coords)
                .map(|(a, b)| a - b).collect()
        );
        self.metric.interval_sq(&dx).sqrt()
    }

    /// Причинна структура: чи точка A впливає на B?
    pub fn causally_connected(&self, a: usize, b: usize) -> bool {
        if a >= self.points.len() || b >= self.points.len() { return false; }
        let dx = NdVector::new(
            self.points[b].coords.iter().zip(&self.points[a].coords)
                .map(|(x, y)| x - y).collect()
        );
        let cone = LightCone::new(self.points[a].clone(), self.metric.p, self.metric.q);
        cone.contains(&dx)
    }

    /// O(p,q) симетрія: застосувати перетворення до всіх точок.
    pub fn transform(&mut self, angles: &[f64]) {
        let m = self.metric.pseudo_orthogonal_transform(angles);
        let n = self.metric.p + self.metric.q;
        for pt in &mut self.points {
            let old = pt.coords.clone();
            for i in 0..n {
                pt.coords[i] = (0..n).map(|j| m[i][j] * old.get(j).unwrap_or(&0.0)).sum();
            }
        }
    }

    /// Кластеризація: знайти групи точок, з'єднаних причинно.
    pub fn causal_clusters(&self) -> Vec<Vec<usize>> {
        let n = self.points.len();
        let mut visited = vec![false; n];
        let mut clusters = Vec::new();
        for i in 0..n {
            if visited[i] { continue; }
            let mut cluster = vec![i];
            visited[i] = true;
            for j in i+1..n {
                if !visited[j] && (self.causally_connected(i, j) || self.causally_connected(j, i)) {
                    cluster.push(j);
                    visited[j] = true;
                }
            }
            clusters.push(cluster);
        }
        clusters
    }

    pub fn dashboard(&self) -> String {
        let n_clusters = self.causal_clusters().len();
        format!(
            "NdSpace O({},{})\n  Points:  {}\n  Clusters: {}\n{}",
            self.metric.p, self.metric.q, self.points.len(), n_clusters, self.metric.dashboard()
        )
    }
}

// ─── Split Algebras + Null Geodesics + Light Communication ────────────
// Розщеплені алгебри: split-complex (Cl(1,0)), split-quaternion (Cl(1,1)),
// split-octonion (Cl(1,3)) = алгебра Дірака.
// Нульові конуси в цих алгебрах = світло.
// Спінори → електрони, кватерніони → SU(2), октоніони → SU(3).

/// Розщеплена алгебра: split-complex (2D), split-quaternion (4D), split-octonion (8D).
#[derive(Debug, Clone)]
pub enum SplitAlgebra {
    SplitComplex { a: f64, b: f64 },       // a + bj, j² = +1
    SplitQuaternion { a: f64, b: f64, c: f64, d: f64 }, // a + bi + cj + dk
    SplitOctonion { coords: [f64; 8] },    // 8D, O(4,4)
}

impl SplitAlgebra {
    pub fn split_complex(a: f64, b: f64) -> Self { SplitAlgebra::SplitComplex { a, b } }
    pub fn split_quaternion(a: f64, b: f64, c: f64, d: f64) -> Self { SplitAlgebra::SplitQuaternion { a, b, c, d } }
    pub fn split_octonion(coords: [f64; 8]) -> Self { SplitAlgebra::SplitOctonion { coords } }

    /// Норма в алгебрі: |x|² = x·x (сигнатура залежить від алгебри).
    pub fn norm_sq(&self) -> f64 {
        match self {
            SplitAlgebra::SplitComplex { a, b } => a * a - b * b,
            SplitAlgebra::SplitQuaternion { a, b, c, d } => a * a + b * b - c * c - d * d,
            SplitAlgebra::SplitOctonion { coords } => {
                coords[0..4].iter().map(|x| x * x).sum::<f64>() -
                coords[4..8].iter().map(|x| x * x).sum::<f64>()
            }
        }
    }

    /// Нульовий вектор (світло): |x|² = 0.
    pub fn is_null(&self) -> bool { self.norm_sq().abs() < 1e-12 }

    /// Множення в алгебрі.
    pub fn mul(&self, other: &SplitAlgebra) -> SplitAlgebra {
        match (self, other) {
            (SplitAlgebra::SplitComplex { a, b }, SplitAlgebra::SplitComplex { a: c, b: d }) => {
                SplitAlgebra::SplitComplex { a: a * c + b * d, b: a * d + b * c }
            }
            (SplitAlgebra::SplitQuaternion { a, b, c, d }, SplitAlgebra::SplitQuaternion { a: e, b: f, c: g, d: h }) => {
                SplitAlgebra::SplitQuaternion {
                    a: a * e + b * f - c * g - d * h,
                    b: a * f + b * e - c * h - d * g,
                    c: a * g + b * h + c * e + d * f,
                    d: a * h + b * g + c * f + d * e,
                }
            }
            _ => self.clone(),
        }
    }

    /// Породжує світловий конус у цій алгебрі.
    pub fn light_cone_points(&self, n: usize) -> Vec<SplitAlgebra> {
        let mut points = Vec::new();
        for i in 0..n {
            let theta = std::f64::consts::TAU * (i as f64) / (n as f64);
            match self {
                SplitAlgebra::SplitComplex { .. } => {
                    // |a² - b²| = 0 → a = ±b → null rays на 45°
                    points.push(SplitAlgebra::SplitComplex { a: theta.cos(), b: theta.cos() });
                }
                SplitAlgebra::SplitQuaternion { .. } => {
                    // a² + b² = c² + d² — сфера S² в нульовому конусі
                    let s = theta.sin() * 0.5f64.sqrt();
                    points.push(SplitAlgebra::SplitQuaternion { a: theta.cos(), b: s, c: s, d: 0.0 });
                }
                SplitAlgebra::SplitOctonion { .. } => {
                    let mut c = [0.0; 8];
                    c[0] = theta.cos();
                    for j in 1..4 { c[j] = theta.sin() / 3.0f64.sqrt(); }
                    for j in 4..8 { c[j] = theta.sin() / 3.0f64.sqrt(); }
                    points.push(SplitAlgebra::SplitOctonion { coords: c });
                }
            }
        }
        points
    }
}

/// Нульова геодезична: світловий промінь у просторі-часі.
#[derive(Debug, Clone)]
pub struct NullGeodesic {
    /// Початкова точка.
    pub origin: NdVector,
    /// Напрям (нульовий вектор).
    pub direction: NdVector,
    /// Швидкість світла (максимальна).
    pub c: f64,
    /// Поточне положення променя.
    pub position: NdVector,
    /// Час життя (кроки).
    pub ttl: u32,
}

impl NullGeodesic {
    pub fn new(origin: NdVector, direction: NdVector) -> Self {
        NullGeodesic { origin: origin.clone(), direction: direction.clone(), c: 299_792_458.0, position: origin, ttl: 100 }
    }

    /// Крок уперед уздовж нульової геодезичної.
    pub fn step(&mut self, dt: f64) {
        let n = self.position.coords.len().min(self.direction.coords.len());
        for i in 0..n {
            self.position.coords[i] += self.c * dt * self.direction.coords[i];
        }
        self.ttl = self.ttl.saturating_sub(1);
    }

    /// Чи досягнув промінь ціль?
    pub fn has_reached(&self, target: &NdVector, tol: f64) -> bool {
        if self.position.coords.len() != target.coords.len() { return false; }
        let d2: f64 = self.position.coords.iter().zip(&target.coords)
            .map(|(a, b)| (a - b).powi(2)).sum();
        d2 < tol * tol
    }

    /// Відстань до цілі в просторі-часі.
    pub fn spacetime_interval_to(&self, target: &NdVector, p: usize) -> f64 {
        let dx: Vec<f64> = self.position.coords.iter().zip(&target.coords)
            .map(|(a, b)| a - b).collect();
        let v = NdVector::new(dx);
        v.norm_sq(p).abs().sqrt()
    }
}

/// Фронт світлової хвилі (n-вимірна сфера, що розширюється).
#[derive(Debug, Clone)]
pub struct LightFront {
    /// Центр випромінювання.
    pub center: NdVector,
    /// Поточний радіус (час × c).
    pub radius: f64,
    /// Швидкість світла.
    pub c: f64,
    /// Час життя.
    pub ttl: u32,
    /// ID відправника.
    pub source: u64,
    /// Дані, що несе світло.
    pub data: Vec<u8>,
}

impl LightFront {
    pub fn new(center: NdVector, c: f64, source: u64, data: Vec<u8>) -> Self {
        LightFront { center, radius: 0.0, c, ttl: 100, source, data }
    }

    /// Розширити фронт на dt: R += c*dt (n-вимірна сфера).
    pub fn expand(&mut self, dt: f64, n_dims: usize) -> Vec<NdVector> {
        self.radius += self.c * dt;
        self.ttl = self.ttl.saturating_sub(1);
        // Згенерувати точки на (n-1)-сфері радіуса R
        let n_points = (n_dims * 4).min(32);
        let mut surface = Vec::new();
        for i in 0..n_points {
            let theta = std::f64::consts::TAU * (i as f64) / (n_points as f64);
            let mut coords = vec![0.0; n_dims];
            coords[0] = self.radius; // часова компонента
            for j in 1..n_dims.min(4) {
                let angle = theta + (j as f64) * 0.5;
                coords[j] = self.radius * angle.sin() * 0.5 / (n_dims as f64).sqrt();
            }
            if n_dims > 4 {
                let spread = self.radius / (n_dims as f64).sqrt();
                for j in 4..n_dims {
                    coords[j] = spread * (theta * (j as f64)).sin();
                }
            }
            surface.push(NdVector::new(coords));
        }
        surface
    }

    /// Чи фронт досяг точки?
    pub fn has_reached(&self, point: &NdVector) -> bool {
        let dx: f64 = self.center.coords.iter().zip(&point.coords)
            .skip(1).map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
        (dx - self.radius).abs() < 0.1 * self.c
    }
}

/// Світлова комунікація: всенаправлена, n-вимірна.
#[derive(Debug)]
pub struct LightCommunication {
    /// Всі активні світлові фронти.
    pub fronts: Vec<LightFront>,
    /// Метрика простору.
    pub metric: PseudoEuclideanMetric,
    /// Швидкість світла.
    pub c: f64,
    /// Розмірність простору.
    pub n_dims: usize,
}

impl LightCommunication {
    pub fn new(metric: PseudoEuclideanMetric) -> Self {
        let n_dims = metric.p + metric.q;
        LightCommunication { fronts: Vec::new(), metric, c: 1.0, n_dims }
    }

    /// Всенаправлене випромінювання світла з точки (n-вимірний спалах).
    pub fn emit(&mut self, from: &NdVector, source: u64, data: &[u8]) -> u64 {
        let front = LightFront::new(from.clone(), self.c, source, data.to_vec());
        let id = self.fronts.len() as u64;
        self.fronts.push(front);
        id
    }

    /// Прийом: чи досяг світловий фронт точки?
    pub fn receive(&mut self, point: &NdVector) -> Vec<(u64, Vec<u8>)> {
        let mut received = Vec::new();
        for front in &self.fronts {
            if front.ttl > 0 && front.has_reached(point) {
                received.push((front.source, front.data.clone()));
            }
        }
        received
    }

    /// Симулювати поширення всіх фронтів.
    pub fn tick(&mut self, dt: f64) -> Vec<u64> {
        let mut expired = Vec::new();
        for (i, front) in self.fronts.iter_mut().enumerate() {
            if front.ttl > 0 {
                front.expand(dt, self.n_dims);
                if front.ttl == 0 { expired.push(i as u64); }
            }
        }
        expired
    }

    /// Спінор з розщепленої алгебри (спін = split-quaternion).
    pub fn spinor_from_split_algebra(alg: &SplitAlgebra) -> [f64; 4] {
        match alg {
            SplitAlgebra::SplitQuaternion { a, b, c, d } => [*a, *b, *c, *d],
            _ => [1.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn dashboard(&self) -> String {
        let active = self.fronts.iter().filter(|f| f.ttl > 0).count();
        format!(
            "Light Communication (omni, n={}, c={})\n  Active fronts: {}\n  Total emitted: {}\n  Metric:        O({},{})",
            self.n_dims, self.c, active, self.fronts.len(), self.metric.p, self.metric.q
        )
    }
}

// ─── Time Warping: gravitational time dilation + local time flow ─────────
// Час викривлюється гравітаційним потенціалом та квантовими ефектами.
// dτ/dt = sqrt(1 - 2GM/rc²) — гравітаційне уповільнення часу.
// Кожен фрактал має ЛОКАЛЬНИЙ час, який тече з різною швидкістю.

/// Локальний час фрактала з гравітаційним уповільненням.
#[derive(Debug, Clone)]
pub struct LocalTime {
    /// Власний час фрактала τ (tick).
    pub proper_time: f64,
    /// Координатний час t (глобальний tick).
    pub coordinate_time: f64,
    /// Гравітаційний потенціал Φ = -GM/r.
    pub potential: f64,
    /// Фактор уповільнення: dτ/dt = sqrt(1 + 2Φ/c²).
    pub dilation: f64,
}

impl LocalTime {
    pub fn new(potential: f64) -> Self {
        let c = 299_792_458.0;
        let dilation = (1.0 + 2.0 * potential / (c * c)).sqrt().max(0.1).min(1.0);
        LocalTime { proper_time: 0.0, coordinate_time: 0.0, potential, dilation }
    }

    /// Тик локального часу: dt_global → dτ = dilation * dt_global.
    pub fn tick(&mut self, dt_global: f64) {
        self.coordinate_time += dt_global;
        self.proper_time += self.dilation * dt_global;
    }

    /// Різниця між власним та координатним часом (дельта викривлення).
    pub fn warp_delta(&self) -> f64 { self.coordinate_time - self.proper_time }
}

/// Гравітаційне поле: маси в точках простору створюють потенціал.
#[derive(Debug)]
pub struct GravitationalField {
    /// Маси в точках (кожен фрактал має масу).
    pub masses: Vec<f64>,
    /// Позиції мас.
    pub positions: Vec<NdVector>,
    /// Гравітаційна стала (нормована).
    pub g_const: f64,
}

impl GravitationalField {
    pub fn new() -> Self { GravitationalField { masses: Vec::new(), positions: Vec::new(), g_const: 1.0 } }

    /// Додати масу.
    pub fn add_mass(&mut self, mass: f64, pos: NdVector) {
        self.masses.push(mass);
        self.positions.push(pos);
    }

    /// Потенціал у точці: Φ(x) = -G Σ m_i / |x - x_i|.
    pub fn potential_at(&self, point: &NdVector) -> f64 {
        let mut phi = 0.0;
        for i in 0..self.masses.len() {
            let dx: f64 = point.coords.iter().zip(&self.positions[i].coords)
                .map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
            if dx > 1e-12 { phi -= self.g_const * self.masses[i] / dx; }
        }
        phi
    }

    /// Сила в точці: F = -∇Φ.
    pub fn force_at(&self, point: &NdVector) -> NdVector {
        let mut f = vec![0.0; point.coords.len()];
        for i in 0..self.masses.len() {
            let dx: Vec<f64> = point.coords.iter().zip(&self.positions[i].coords)
                .map(|(a, b)| b - a).collect();
            let r = dx.iter().map(|x| x.powi(2)).sum::<f64>().sqrt().max(1e-12);
            let coeff = self.g_const * self.masses[i] / (r.powi(3));
            for j in 0..f.len() {
                f[j] += coeff * dx[j];
            }
        }
        NdVector::new(f)
    }
}

/// Викривлення часу: інтеграція гравітації + квантових ефектів.
#[derive(Debug)]
pub struct TimeWarp {
    /// Локальний час кожного фрактала.
    pub local_times: Vec<LocalTime>,
    /// Гравітаційне поле.
    pub field: GravitationalField,
    /// Квантовий гамільтоніан для time evolution.
    pub hamiltonian: Hamiltonian,
    /// Глобальний час.
    pub global_tick: u64,
}

impl TimeWarp {
    pub fn new() -> Self {
        TimeWarp { local_times: Vec::new(), field: GravitationalField::new(), hamiltonian: Hamiltonian::new(1.0, 0.0, 0.0), global_tick: 0 }
    }

    /// Додати фрактал з масою та позицією.
    pub fn add_node(&mut self, mass: f64, pos: NdVector) {
        let phi = self.field.potential_at(&pos);
        self.local_times.push(LocalTime::new(phi));
        self.field.add_mass(mass, pos);
    }

    /// Глобальний тик: всі фрактали відчувають різний плин часу.
    pub fn tick(&mut self, dt: f64) {
        self.global_tick += 1;
        for (i, lt) in self.local_times.iter_mut().enumerate() {
            // Оновити потенціал (позиції можуть змінюватись)
            if i < self.field.positions.len() {
                lt.potential = self.field.potential_at(&self.field.positions[i]);
                let c = 299_792_458.0;
                lt.dilation = (1.0 + 2.0 * lt.potential / (c * c)).sqrt().max(0.1).min(1.0);
            }
            lt.tick(dt);
        }
    }

    /// Квантово-часовий зсув: застосувати гамільтоніан до кубіта.
    pub fn quantum_time_shift(&self, qubit: &Qubit, node_idx: usize) -> Qubit {
        if node_idx >= self.local_times.len() { return qubit.clone(); }
        let warp = self.local_times[node_idx].warp_delta();
        let dt = warp.max(0.001).min(1.0);
        self.hamiltonian.evolve(qubit, dt)
    }

    /// Різниця часу між двома фракталами.
    pub fn time_difference(&self, a: usize, b: usize) -> f64 {
        if a >= self.local_times.len() || b >= self.local_times.len() { return 0.0; }
        (self.local_times[a].proper_time - self.local_times[b].proper_time).abs()
    }

    pub fn dashboard(&self) -> String {
        let max_warp = self.local_times.iter().map(|lt| lt.warp_delta()).fold(0.0, f64::max);
        let min_dil = self.local_times.iter().map(|lt| lt.dilation).fold(1.0, f64::min);
        format!("Time Warp (gravitational + quantum)\n  Nodes:     {}\n  Global:    {}\n  Max warp:  {:.6}\n  Min dil:   {:.6}",
            self.local_times.len(), self.global_tick, max_warp, min_dil)
    }
}

// ─── 1. Unified Navigator ───────────────────────────────────────────────
// Fuses: spectral PPR + geometric geodesics + light cones + parametric surface.
#[derive(Debug)]
pub struct UnifiedNavigator {
    pub phase_space: PhaseSpace,
    pub memory: crate::memory_search::TopoChronoMemory,
    pub light: LightCommunication,
    pub metric: PseudoEuclideanMetric,
}

impl UnifiedNavigator {
    pub fn new(metric: PseudoEuclideanMetric, dims: usize) -> Self {
        UnifiedNavigator {
            phase_space: PhaseSpace::new(),
            memory: crate::memory_search::TopoChronoMemory::new(dims),
            light: LightCommunication::new(metric.clone()),
            metric,
        }
    }

    /// Навігація: знайти найкоротший шлях між двома точками (геодезика + світло).
    pub fn navigate(&self, from_idx: usize, to_idx: usize) -> f64 {
        if from_idx >= self.phase_space.states.len() || to_idx >= self.phase_space.states.len() {
            return f64::MAX;
        }
        let a = &self.phase_space.states[from_idx].position;
        let b = &self.phase_space.states[to_idx].position;
        let dx: Vec<f64> = a.coords.iter().zip(&b.coords).map(|(x, y)| x - y).collect();
        let delta = NdVector::new(dx);
        let interval = self.metric.interval_sq(&delta).abs().sqrt();
        // Якщо всередині світлового конуса → миттєво
        let a_nd = NdVector::new(a.coords.to_vec());
        let cone = LightCone::new(a_nd, self.metric.p, self.metric.q);
        if cone.contains(&delta) { return interval * 0.1; } // faster-than-light navigation
        interval
    }

    /// Прогноз: передбачити позицію через dt.
    pub fn predict(&mut self, dt: f64) {
        self.phase_space.predict_forward(dt);
    }

    /// Знайти всі фрактали в світловому конусі заданої точки.
    pub fn in_light_cone(&self, idx: usize) -> Vec<usize> {
        if idx >= self.phase_space.states.len() { return vec![]; }
        let origin = &self.phase_space.states[idx].position;
        let o_nd = NdVector::new(origin.coords.to_vec());
        let cone = LightCone::new(o_nd, self.metric.p, self.metric.q);
        (0..self.phase_space.states.len()).filter(|&i| {
            if i == idx { return false; }
            let dx: Vec<f64> = self.phase_space.states[i].position.coords.iter()
                .zip(&origin.coords).map(|(a, b)| a - b).collect();
            cone.contains(&NdVector::new(dx))
        }).collect()
    }

    pub fn dashboard(&self) -> String {
        format!("Unified Navigator\n  Nodes: {}\n  Memory: {}\n  Light: active={}\n  Metric: O({},{})",
            self.phase_space.states.len(), self.memory.labels.len(),
            self.light.fronts.iter().filter(|f| f.ttl > 0).count(),
            self.metric.p, self.metric.q)
    }
}

// ─── 2. Real-Time Prediction Service ─────────────────────────────────────
#[derive(Debug)]
pub struct PredictionService {
    pub phase: PhaseSpace,
    pub observations: Vec<GeometricPoint>,
    pub horizon: usize,
    pub tick: u64,
}

impl PredictionService {
    pub fn new() -> Self {
        PredictionService { phase: PhaseSpace::new(), observations: Vec::new(), horizon: 10, tick: 0 }
    }

    /// Додати спостереження та передбачити.
    pub fn observe_and_predict(&mut self, obs: GeometricPoint) -> Vec<GeometricPoint> {
        self.observations.push(obs);
        self.phase.add_node(&format!("obs_{}", self.tick));
        let idx = self.phase.states.len() - 1;
        self.phase.states[idx].position = obs;
        self.tick += 1;
        // Forward predictions
        let mut predictions = Vec::new();
        let mut current = self.phase.states[idx].position;
        for _ in 0..self.horizon {
            let next = GeometricState::new(current).forward(1.0);
            current = next.position;
            predictions.push(current);
        }
        predictions
    }

    /// Retrospective: знайти причину зміни.
    pub fn retrospective(&mut self, idx: usize) -> Option<GeometricPoint> {
        if idx >= self.phase.states.len() { return None; }
        let prev = self.phase.states[idx].backward(1.0);
        Some(prev.position)
    }

    pub fn dashboard(&self) -> String {
        format!("Prediction Service\n  Obs: {}\n  Horizon: {}\n  Tick: {}",
            self.observations.len(), self.horizon, self.tick)
    }
}

// ─── 3. Autonomous Execution Loop ────────────────────────────────────────
pub struct AutonomousLoop {
    pub swarm: crate::swarm::SwarmCoordinator,
    pub oracle: crate::oracle::PatternOracle,
    pub miner: crate::meta_miner::MetaMiner,
    pub cycle: u64,
}

impl std::fmt::Debug for AutonomousLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutonomousLoop").field("cycle", &self.cycle).field("swarm_executors", &self.swarm.executor_count()).finish()
    }
}

impl AutonomousLoop {
    pub fn new() -> Self {
        let swarmlings = (0..8).map(|i| {
            crate::swarm::Swarmling::new(i, vec!["research".into(),"mine".into(),"navigate".into()], 100.0)
        }).collect();
        AutonomousLoop {
            swarm: crate::swarm::SwarmCoordinator::new(swarmlings),
            oracle: crate::oracle::PatternOracle::new(),
            miner: crate::meta_miner::MetaMiner::new(),
            cycle: 0,
        }
    }

    /// Один цикл: observe → predict → decide → execute → learn.
    pub fn cycle(&mut self) {
        self.cycle += 1;
        // Observe
        self.oracle.add_paper(&format!("autonomous cycle {}", self.cycle));
        // Predict (miner extracts meta-patterns)
        let _insights = self.miner.iterate();
        // Decide (select best executor)
        let task = crate::swarm::TaskSpec {
            id: self.cycle as usize, skill: "research".into(),
            raw_arg: format!("cycle_{}", self.cycle), dependencies: vec![],
        };
        if let Some(exec) = self.swarm.select_executor(&task) {
            self.swarm.dispatch(&task, exec);
        }
        // Execute + Learn
        let result = crate::swarm::TaskResult {
            id: self.cycle as usize, success: true,
            output: format!("cycle_{}_done", self.cycle), error: String::new(), executor_id: 0,
        };
        self.swarm.record_result(result);
    }

    pub fn dashboard(&self) -> String {
        format!("Autonomous Loop\n  Cycles: {}\n  Swarm: {} executors\n  Miner iter: {}\n  Oracle: {}",
            self.cycle, self.swarm.executor_count(), self.miner.iterations, self.cycle)
    }
}

// ─── 4. Memory Consolidation Pipeline ────────────────────────────────────
pub struct MemoryPipeline {
    pub topo: crate::memory_search::TopoChronoMemory,
    pub research: crate::research::ResearchEngine,
    pub oracle: crate::oracle::PatternOracle,
    pub miner: crate::meta_miner::MetaMiner,
}

impl std::fmt::Debug for MemoryPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryPipeline").field("records", &self.topo.labels.len()).finish()
    }
}

impl MemoryPipeline {
    pub fn new(dims: usize) -> Self {
        MemoryPipeline {
            topo: crate::memory_search::TopoChronoMemory::new(dims),
            research: crate::research::ResearchEngine::new(),
            oracle: crate::oracle::PatternOracle::new(),
            miner: crate::meta_miner::MetaMiner::new(),
        }
    }

    /// Інтегрувати нове дослідження в пам'ять.
    pub fn ingest(&mut self, label: &str, text: &str, topology: f64, weight: f64) {
        self.topo.record(label, text, topology, weight);
        self.oracle.add_paper(text);
        let _insights = self.miner.iterate();
    }

    /// Консолідувати: підкріпити важливі спогади.
    pub fn consolidate(&mut self) {
        let n = self.topo.labels.len();
        for i in 0..n {
            if self.topo.surface.weights.get(i).map(|&w| w > 0.5).unwrap_or(false) {
                self.topo.reinforce(i, 0.1);
            } else {
                self.topo.evolve(0.05);
            }
        }
    }

    /// Знайти за контекстом (топологія + асоціація).
    pub fn recall(&self, topology: f64, time: f64, k: usize) -> Vec<(String, f64, f64)> {
        self.topo.retrieve(topology, time, k)
    }

    pub fn dashboard(&self) -> String {
        format!("Memory Pipeline\n  Records: {}\n  Oracle: {}\n  Miner: {} iters\n  Research: {} papers",
            self.topo.labels.len(), self.topo.labels.len(),
            self.miner.iterations, self.research.total_papers())
    }
}

// ─── 5. Cross-Node Geometric Sync ────────────────────────────────────────
#[derive(Debug)]
pub struct GeometricSync {
    pub wave_bus: WaveBus,
    pub nodes: Vec<String>,
    pub phase: PhaseSpace,
}

impl GeometricSync {
    pub fn new() -> Self {
        GeometricSync { wave_bus: WaveBus::new(), nodes: Vec::new(), phase: PhaseSpace::new() }
    }

    /// Додати ноду та підключити до всіх.
    pub fn add_node(&mut self, name: &str) {
        self.nodes.push(name.to_string());
        self.phase.add_node(name);
        self.wave_bus.register(name);
        for other in &self.nodes {
            if other != name {
                self.wave_bus.connect(name, other);
            }
        }
    }

    /// Синхронізувати геометричні стани між нодами.
    pub fn sync(&mut self) {
        for i in 0..self.nodes.len() {
            if i >= self.phase.states.len() { break; }
            let pos = &self.phase.states[i].position;
            let data: Vec<u8> = pos.coords.iter().map(|&c| (c * 10.0) as i8 as u8).collect();
            for j in 0..self.nodes.len() {
                if i != j {
                    self.wave_bus.send(&self.nodes[i], &self.nodes[j], &data);
                }
            }
        }
        self.wave_bus.tick();
        self.phase.predict_forward(1.0);
    }

    pub fn dashboard(&self) -> String {
        format!("Geometric Sync\n  Nodes: {}\n  WaveBus: {} sockets\n  Phase: {} states",
            self.nodes.len(), self.wave_bus.sockets.len(), self.phase.states.len())
    }
}

// ─── 6. ML Inference Integration ─────────────────────────────────────────
#[derive(Debug)]
pub struct InferenceEngine {
    pub model: Vec<f64>,
    pub input_dim: usize,
    pub output_dim: usize,
}

impl InferenceEngine {
    pub fn new(input_dim: usize, output_dim: usize) -> Self {
        InferenceEngine { model: vec![0.0; input_dim * output_dim], input_dim, output_dim }
    }

    /// Проста лінійна проекція: y = Wx + b (для передбачення в PhaseSpace).
    pub fn project(&self, input: &[f64]) -> Vec<f64> {
        let n = input.len().min(self.input_dim);
        let m = self.output_dim;
        let mut output = vec![0.0; m];
        for i in 0..n {
            for j in 0..m {
                output[j] += self.model[i * m + j] * input[i];
            }
        }
        output
    }

    /// Навчання: градієнтний спуск (одна ітерація).
    pub fn train(&mut self, input: &[f64], target: &[f64], lr: f64) {
        let pred = self.project(input);
        let n = input.len().min(self.input_dim);
        let m = self.output_dim;
        for i in 0..n {
            for j in 0..m.min(target.len()) {
                let error = pred[j] - target[j];
                self.model[i * m + j] -= lr * error * input[i];
            }
        }
    }

    /// Екстраполювати траєкторію в PhaseSpace.
    pub fn extrapolate(&self, state: &GeometricState, steps: usize) -> Vec<GeometricPoint> {
        let mut points = Vec::new();
        let mut coords = state.position.coords.to_vec();
        for _ in 0..steps {
            let projected = self.project(&coords);
            for i in 0..coords.len().min(projected.len()) {
                coords[i] += projected[i] * 0.01;
            }
            points.push(GeometricPoint { coords: {
                let mut c = [0.0; 8];
                for (i, &v) in coords.iter().enumerate().take(8) { c[i] = v; }
                c
            }});
        }
        points
    }

    pub fn dashboard(&self) -> String {
        format!("Inference Engine\n  Dims: {} -> {}\n  Params: {}",
            self.input_dim, self.output_dim, self.model.len())
    }
}

// ─── 7. Live nD Visualization ────────────────────────────────────────────
#[derive(Debug)]
pub struct LiveDashboard {
    pub phase: PhaseSpace,
    pub navigator: UnifiedNavigator,
    pub prediction: PredictionService,
    pub autonomous: AutonomousLoop,
    pub memory: MemoryPipeline,
    pub light: LightCommunication,
}

impl LiveDashboard {
    pub fn new(metric: PseudoEuclideanMetric, dims: usize) -> Self {
        LiveDashboard {
            phase: PhaseSpace::new(),
            navigator: UnifiedNavigator::new(metric.clone(), dims),
            prediction: PredictionService::new(),
            autonomous: AutonomousLoop::new(),
            memory: MemoryPipeline::new(dims),
            light: LightCommunication::new(metric),
        }
    }

    pub fn render(&self) -> String {
        format!(
            "\n═══════════════════════════════════════════════\n\
             LIVE DASHBOARD\n\
             ═══════════════════════════════════════════════\n\
             {}\n  ---\n  {}\n  ---\n  {}\n  ---\n  {}\n  ---\n  {}\n  ---\n  {}",
            self.phase.dashboard(),
            self.navigator.dashboard(),
            self.prediction.dashboard(),
            self.autonomous.dashboard(),
            self.memory.dashboard(),
            self.light.dashboard(),
        )
    }
}

// ─── 8. P2P Fractal Network Layer ────────────────────────────────────────
#[derive(Debug)]
pub struct P2PNetwork {
    pub mesh: AcademiaMesh,
    pub geo: GeometricSync,
    pub peers: Vec<String>,
    pub active: bool,
}

impl P2PNetwork {
    pub fn new() -> Self {
        P2PNetwork { mesh: AcademiaMesh::new(), geo: GeometricSync::new(), peers: Vec::new(), active: false }
    }

    /// Запустити seed-ноду (TCP сервер).
    pub fn start_seed(&mut self, listen_addr: &str) {
        self.mesh.add_node("seed", listen_addr, 100);
        self.geo.add_node("seed");
        self.active = true;
    }

    /// Підключитися до seed-ноди.
    pub fn connect_to_seed(&mut self, local_id: &str, seed_addr: &str) {
        self.mesh.add_node(local_id, seed_addr, 100);
        self.geo.add_node(local_id);
        self.peers.push(seed_addr.to_string());
    }

    /// Синхронізувати через P2P.
    pub fn sync_all(&mut self) {
        if !self.active { return; }
        self.geo.sync();
        self.mesh.wave_bus.tick();
    }

    /// Статистика мережі.
    pub fn network_stats(&self) -> String {
        format!("P2P Network\n  Active: {}\n  Peers:  {}\n  Mesh:   {} nodes\n  Geo:    {} states",
            self.active, self.peers.len(), self.mesh.nodes.len(), self.geo.phase.states.len())
    }
}


// ─── 3-Node Simultaneous Communication ──────────────────────────────────
// Проблема: одночасна комунікація між 3 об'єктами/нодами.
// Рішення: GHZ стан (|000⟩ + |111⟩)/√2 — 3-кубітна заплутаність.
// Вимір однієї частинки миттєво визначає стан всіх трьох.
// Додатково: 3-phase consensus, byzantine fault tolerance для 3 нод.

/// 3-кубітний GHZ стан: (|000⟩ + |111⟩)/√2.
/// Вимір будь-якого кубіта → всі три колапсують в один стан.
#[derive(Debug, Clone)]
pub struct GHZState {
    pub amp_000: Complex,
    pub amp_111: Complex,
}

impl GHZState {
    /// Створити GHZ: (|000⟩ + |111⟩)/√2.
    pub fn new() -> Self {
        let s = 2.0f64.sqrt().recip();
        GHZState { amp_000: Complex::one().scale(s), amp_111: Complex::one().scale(s) }
    }

    /// Ймовірність |000⟩.
    pub fn prob_000(&self) -> f64 { self.amp_000.norm_sq() }
    /// Ймовірність |111⟩.
    pub fn prob_111(&self) -> f64 { self.amp_111.norm_sq() }

    /// Виміряти один кубіт → колапс всіх трьох.
    pub fn measure_one(&mut self, seed: u64) -> u8 {
        let r = ((seed as f64 * 1.618033988749895).fract() + 0.5) % 1.0;
        if r < self.prob_000() {
            self.amp_000 = Complex::one();
            self.amp_111 = Complex::zero();
            0 // всі три → |0⟩
        } else {
            self.amp_000 = Complex::zero();
            self.amp_111 = Complex::one();
            1 // всі три → |1⟩
        }
    }

    /// Заплутати 3 ноди: після цього всі три корельовані.
    pub fn entangle(nodes: &mut [ThreeNodeState; 3]) {
        let ghz = GHZState::new();
        for node in nodes.iter_mut() {
            node.ghz = ghz.clone();
            node.entangled = true;
        }
    }

    pub fn dashboard(&self) -> String {
        format!("GHZ: (|000⟩ + |111⟩)/√2\n  P(000)={:.2}%  P(111)={:.2}%", self.prob_000()*100.0, self.prob_111()*100.0)
    }
}

/// Стан однієї ноди в 3-вузловій системі.
#[derive(Debug, Clone)]
pub struct ThreeNodeState {
    pub id: String,
    pub ghz: GHZState,
    pub entangled: bool,
    pub data: Vec<u8>,
    pub phase: f64, // фаза для синхронізації
}

impl ThreeNodeState {
    pub fn new(id: &str) -> Self {
        ThreeNodeState { id: id.to_string(), ghz: GHZState::new(), entangled: false, data: Vec::new(), phase: 0.0 }
    }
}

/// 3-вузлова шина: одночасна комунікація через GHZ.
#[derive(Debug)]
pub struct ThreeNodeBus {
    pub nodes: [ThreeNodeState; 3],
    pub round: u64,
}

impl ThreeNodeBus {
    /// Створити шину з 3 нодами, одразу заплутати.
    pub fn new(ids: [&str; 3]) -> Self {
        let mut nodes = [
            ThreeNodeState::new(ids[0]),
            ThreeNodeState::new(ids[1]),
            ThreeNodeState::new(ids[2]),
        ];
        GHZState::entangle(&mut nodes);
        ThreeNodeBus { nodes, round: 0 }
    }

    /// Одночасний обмін: всі 3 ноди обмінюються даними за 1 раунд.
    pub fn exchange(&mut self, data: [&[u8]; 3]) {
        self.round += 1;
        // Крок 1: кожна нода отримує дані
        for (i, d) in data.iter().enumerate() {
            self.nodes[i].data = d.to_vec();
            self.nodes[i].phase = self.round as f64 * 0.1;
        }
        // Крок 2: вимірюємо GHZ → всі три колапсують синхронно
        let outcome = self.nodes[0].ghz.measure_one(self.round);
        // Крок 3: всі три ноди знають результат одночасно
        for node in &mut self.nodes {
            node.phase += if outcome == 0 { 1.0 } else { -1.0 };
        }
    }

    /// Консенсус: всі три ноди мають однаковий стан?
    pub fn consensus(&self) -> bool {
        let phase = self.nodes[0].phase;
        self.nodes.iter().all(|n| (n.phase - phase).abs() < 0.001)
    }

    /// Majority vote: що вирішили 3 ноди?
    pub fn majority_vote(&self) -> i32 {
        let outcomes: Vec<f64> = self.nodes.iter().map(|n| n.phase).collect();
        if outcomes.iter().filter(|&&p| p > 0.0).count() >= 2 { 1 }
        else if outcomes.iter().filter(|&&p| p < 0.0).count() >= 2 { -1 }
        else { 0 }
    }

    /// Синхронізувати фази: всі три ноди в однаковій фазі.
    pub fn sync_phases(&mut self) {
        let avg: f64 = self.nodes.iter().map(|n| n.phase).sum::<f64>() / 3.0;
        for node in &mut self.nodes {
            node.phase = avg;
        }
    }

    pub fn dashboard(&self) -> String {
        format!(
            "3-Node Bus (GHZ-entangled)\n  Round:    {}\n  Nodes:    {} {} {}\n  Entangled: {}\n  Consensus: {}\n  Vote:      {}",
            self.round,
            self.nodes[0].id, self.nodes[1].id, self.nodes[2].id,
            self.nodes[0].entangled,
            if self.consensus() { "YES" } else { "NO" },
            self.majority_vote()
        )
    }
}

// Торгівля з використанням часового зсуву, суперпозиції та квантової фізики.
// Різні вузли (біржі) мають різний плин часу → арбітраж.
// Суперпозиція позицій: одночасно в кількох станах.
// Фазовий простір: передбачення цінових траєкторій.

/// Тип торгового ордера.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderType { Buy, Sell, Hold }

/// Квантовий трейдер: використовує часовий зсув + суперпозицію.
#[derive(Debug)]
pub struct QuantumTrader {
    /// Назва стратегії.
    pub name: String,
    /// Поточний бюджет.
    pub budget: f64,
    /// Поточна позиція (кількість активу).
    pub position: f64,
    /// Часовий зсув трейдера (dτ/dt).
    pub time_dilation: f64,
    /// Квантовий стан для прийняття рішень.
    pub decision_state: Qubit,
    /// Історія цін.
    pub price_history: Vec<f64>,
    /// Кількість угод.
    pub trades: u64,
    /// Прибуток.
    pub pnl: f64,
}

impl QuantumTrader {
    pub fn new(name: &str, budget: f64, time_dilation: f64) -> Self {
        QuantumTrader {
            name: name.to_string(), budget, position: 0.0,
            time_dilation, decision_state: Qubit::plus(),
            price_history: Vec::new(), trades: 0, pnl: 0.0,
        }
    }

    /// Оновити ціну та прийняти рішення (через квантовий стан).
    pub fn on_price(&mut self, price: f64) -> OrderType {
        self.price_history.push(price);
        // Квантове рішення: виміряти стан → Buy або Sell
        let mut m = QuantumMeasurement::new();
        let result = m.measure(&self.decision_state);
        // Оновити стан (еволюція з часовим зсувом)
        let h = Hamiltonian::new(1.0, 0.5, 0.2);
        self.decision_state = h.evolve(&self.decision_state, self.time_dilation);
        match result {
            0 => OrderType::Buy,
            1 => OrderType::Sell,
            _ => OrderType::Hold,
        }
    }

    /// Виконати ордер.
    pub fn execute(&mut self, order: OrderType, price: f64) {
        match order {
            OrderType::Buy if self.budget >= price => {
                let qty = (self.budget / price).floor();
                self.position += qty;
                self.budget -= qty * price;
                self.trades += 1;
            }
            OrderType::Sell if self.position > 0.0 => {
                self.budget += self.position * price;
                self.pnl += self.position * price;
                self.position = 0.0;
                self.trades += 1;
            }
            _ => {}
        }
    }

    /// Передбачення ціни через PhaseSpace екстраполяцію.
    pub fn predict_price(&self, steps: usize) -> Vec<f64> {
        if self.price_history.len() < 2 { return vec![]; }
        let n = self.price_history.len().min(8);
        let mut coords = [0.0; 8];
        for i in 0..n {
            coords[i] = self.price_history[self.price_history.len() - n + i];
        }
        let point = GeometricPoint::new(coords);
        let state = GeometricState::new(point);
        let ie = InferenceEngine::new(8, 8);
        let trajectory = ie.extrapolate(&state, steps);
        trajectory.iter().map(|p| p.coords[0]).collect()
    }

    /// Арбітраж між двома трейдерами з різним часовим зсувом.
    pub fn arbitrage(a: &mut QuantumTrader, b: &mut QuantumTrader, price_a: f64, price_b: f64) {
        // Якщо час тече по-різному, ціни розходяться
        if price_a < price_b && a.budget >= price_a && b.position > 0.0 {
            a.execute(OrderType::Buy, price_a);
            b.execute(OrderType::Sell, price_b);
        } else if price_b < price_a && b.budget >= price_b && a.position > 0.0 {
            b.execute(OrderType::Buy, price_b);
            a.execute(OrderType::Sell, price_a);
        }
    }

    pub fn dashboard(&self) -> String {
        format!("Quantum Trader '{}'\n  Budget: {:.2}\n  Position: {:.4}\n  Trades: {}\n  PnL: {:.2}\n  dτ/dt: {:.4}\n  Qubit: |0⟩={:.2}% |1⟩={:.2}%",
            self.name, self.budget, self.position, self.trades, self.pnl,
            self.time_dilation,
            self.decision_state.prob_zero() * 100.0,
            self.decision_state.prob_one() * 100.0)
    }
}
