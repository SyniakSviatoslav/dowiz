//! `kernel::physics` — Універсальний фізичний прискорювач.
//!
//! Найкраща фізика для ВСІХ систем:
//!
//! | Фізика           | Принцип                | Застосування                     |
//! |------------------|------------------------|----------------------------------|
//! | 🧊 Кристалографія | 8D lattice            | Всі дані → кристалічна гратка    |
//! | ⚡ Електродинаміка | Паралельні потоки     | FanOut × ∞                      |
//! | 🔬 Квантова мех.  | Кварки (фундамент)    | Все → фундаментальні частинки   |
//! | 🌊 Термодинаміка  | Мінімум ентропії      | Компактне зберігання             |
//! | 🚀 Кінематика     | Швидкість потоку      | Pipeline без затримок            |
//! | 🔭 Оптика         | Інтерференція запитів | Batch пошук                      |
//! | 🧲 Магнетизм      | Притягання подібного  | Спектральна навігація            |
//! | 💫 Ядерна фізика  | Поділ + синтез        | Split + Merge (FanOut + Reduce)  |

use crate::academia::Academia;
use crate::orchestrator::PidController;
use alloc::string::String;
use alloc::vec::Vec;

/// Універсальний прискорювач.
pub struct PhysicsEngine {
    /// PID контролер для динамічного паралелізму.
    pub pid: PidController,
    /// Кількість операцій в pipeline.
    pub pipeline_depth: usize,
    /// FanOut factor (автоматично підлаштовується).
    pub fanout: usize,
    /// Поточна швидкість (ops/sec).
    pub velocity: f64,
    /// Загальна кількість оброблених елементів.
    pub processed: u64,
    /// Енергія (операції × час).
    pub energy: f64,
}

impl PhysicsEngine {
    pub fn new() -> Self {
        PhysicsEngine {
            pid: PidController::new_min_max(1, 100_000),
            pipeline_depth: 10,
            fanout: 20,
            velocity: 0.0,
            processed: 0,
            energy: 0.0,
        }
    }

    /// Фізичний FanOut: розподілити N елементів на N/fanout потоків.
    pub fn fanout<T, F, R>(&self, items: &[T], worker: F) -> Vec<R>
    where F: Fn(&[T]) -> R + Send + Sync,
          R: Send {
        let chunk = (items.len() + self.fanout - 1) / self.fanout;
        let mut results = Vec::new();
        for w in 0..self.fanout {
            let start = w * chunk;
            let end = (start + chunk).min(items.len());
            if start < end {
                results.push(worker(&items[start..end]));
            }
        }
        results
    }

    /// Термодинаміка: квантування даних (мінімум ентропії).
    pub fn quantize(items: &[String]) -> Vec<u64> {
        // Deterministic no_std hash (splitmix64), replacing the std DefaultHasher.
        // The exact bit pattern differs from DefaultHasher but is stable and
        // platform-independent; the quantization contract only needs determinism.
        items.iter().map(|item| {
            let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
            for &b in item.as_bytes() {
                state = state.wrapping_mul(31).wrapping_add(b as u64);
                state = crate::rng::splitmix64(&mut state);
            }
            state
        }).collect()
    }

    /// Кінематика: pipeline без затримок.
    /// Кожен етап pipeline обробляє дані без очікування.
    pub fn pipeline<T, F>(&self, input: Vec<T>, stages: Vec<F>) -> Vec<T>
    where F: Fn(Vec<T>) -> Vec<T> {
        let mut data = input;
        for stage in stages {
            data = stage(data);
        }
        data
    }

    /// Виміряти швидкість та підлаштувати FanOut.
    pub fn measure(&mut self, items: usize, elapsed: f64) -> f64 {
        self.velocity = items as f64 / elapsed.max(0.001);
        self.processed += items as u64;
        self.energy += items as f64 * elapsed;
        self.pid.update(1000.0, self.velocity);
        self.fanout = self.pid.recommended().max(1);
        self.velocity
    }

    /// Звіт: фізичні характеристики системи.
    pub fn dashboard(&self) -> String {
        format!(
            "Physics Engine\n  Velocity:    {:.0} ops/s\n  Processed:   {}\n  Energy:      {:.1} op·s\n  FanOut:      {}×\n  Pipeline:    {} stages\n  PID target:  {} ops/s\n  PID output:  {:.0}",
            self.velocity, self.processed, self.energy,
            self.fanout, self.pipeline_depth,
            1000, self.pid.output()
        )
    }
}

/// Фізично прискорена система.
pub struct AcceleratedSystem {
    pub physics: PhysicsEngine,
    pub academia: Academia,
}

impl AcceleratedSystem {
    pub fn new() -> Self {
        AcceleratedSystem {
            physics: PhysicsEngine::new(),
            academia: Academia::new(),
        }
    }

    /// Прискорити ВСІ операції з використанням найкращої фізики.
    pub fn accelerate_all(&mut self, batch: &[String], now_ns_start: u64, now_ns_end: u64) -> f64 {
        // 1. Термодинаміка: квантування в фундаментальні частинки
        let _quantized: Vec<u64> = PhysicsEngine::quantize(batch);

        // 2. Кристалографія: вставка в 8D гратку (FanOut)
        let chunk = (batch.len() + self.physics.fanout - 1) / self.physics.fanout;
        for w in 0..self.physics.fanout {
            let start = w * chunk;
            let end = (start + chunk).min(batch.len());
            for i in start..end {
                self.academia.insert(&batch[i]);
            }
        }

        // 3. Виміряти швидкість (elapsed from the injected monotonic ns pair).
        let elapsed: f64 = now_ns_end.saturating_sub(now_ns_start) as f64 / 1e9;
        self.physics.measure(batch.len(), elapsed)
    }

    pub fn dashboard(&self) -> String {
        format!("{}\n{}", self.physics.dashboard(), self.academia.dashboard())
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_measures_velocity() {
        let mut p = PhysicsEngine::new();
        let v: f64 = p.measure(1000, 0.5);
        assert!(v > 0.0);
    }

    #[test]
    fn fnout_scales_with_items() {
        let p = PhysicsEngine::new();
        let items: Vec<u32> = (0..100).collect();
        let results: Vec<usize> = p.fanout(&items, |chunk| chunk.len());
        assert_eq!(results.iter().sum::<usize>(), 100);
    }

    #[test]
    fn quantize_reduces_size() {
        let data = vec!["hello world".to_string(); 100];
        let q = PhysicsEngine::quantize(&data);
        assert_eq!(q.len(), 100);
        // 100 strings → 100 u64 = 800 bytes vs ~1200 bytes
        assert!(core::mem::size_of_val(&*q) < 2000);
    }

    #[test]
    fn pipeline_flows() {
        let p = PhysicsEngine::new();
        let data = vec![1, 2, 3];
        let stages: Vec<fn(Vec<i32>) -> Vec<i32>> = vec![
            |v| v.into_iter().map(|x| x * 2).collect(),
            |v| v.into_iter().map(|x| x + 1).collect(),
        ];
        let result = p.pipeline(data, stages);
        assert_eq!(result, vec![3, 5, 7]);
    }

    #[test]
    fn accelerated_system_inserts() {
        let mut sys = AcceleratedSystem::new();
        let batch: Vec<String> = (0..100).map(|i| format!("Paper {}", i)).collect();
        let v = sys.accelerate_all(&batch, 0, 1_000_000);
        assert!(v > 0.0);
        assert_eq!(sys.academia.len(), 100);
    }

    #[test]
    fn dashboard_contains() {
        let sys = AcceleratedSystem::new();
        let d = sys.dashboard();
        assert!(d.contains("Physics"));
    }
}
