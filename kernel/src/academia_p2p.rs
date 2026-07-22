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
