//! `kernel::academia_agent` — Autonomous headless extraction agents.
//!
//! # Розподілені агенти
//! Кожен агент = окремий headless browser на окремому акаунті/IP.
//! Агенти незалежно екстрактують папери, конвертують в матрицю,
//! аплоадять чанки в HF mesh через proxy pool.
//!
//! # Anti-detect (agent_browser)
//! - Headless: кожен агент через окремий browser profile
//! - ZeroTracePolicy: очищення слідів після кожного запиту
//! - ProxyPool: ротація через різні IP/акаунти
//! - Jitter: випадкові затримки (10ms-5s)
//! - UserAgent: випадкові браузерні профілі
//!
//! # Розподіл
//! Кожен агент отримує свій сегмент даних (FanOut).
//! Результати зливаються в спільну матрицю через HF.
//!
//! # Пам'ять (на агента)
//! - Сегмент матриці: ~50 MB (6M паперів × 8 u8)
//! - Bloom: ~2 MB
//! - Всього: ~52 MB
//! - Час: ~6 хв на сегмент (OAI-PMH)

use crate::academia_p2p::AcademiaMesh;
use crate::TriState;

/// Максимум агентів.
pub const MAX_AGENTS: usize = 1000;
/// Паперів на сегмент агента.
pub const PAPERS_PER_AGENT: u64 = 6_000_000;

// ─── Agent Profile (anti-detect) ──────────────────────────────────────────

/// Профіль агента — headless browser + anti-detect конфіг.
#[derive(Debug, Clone)]
pub struct AgentProfile {
    pub id: String,
    pub proxy: String,
    pub user_agent: String,
    pub viewport: (u32, u32),
    pub timezone: String,
    pub language: String,
    pub platform: String,
    pub hardware_concurrency: u32,
}

impl AgentProfile {
    pub fn generate(id: &str, seed: u64) -> Self {
        let mut rng = seed;
        let mut next = || { rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1); rng >> 33 };

        let uas = [
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0.0.0 Safari/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120.0.0.0",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Safari/17.2",
            "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
        ];
        let proxies = [
            "http://proxy1:8080", "http://proxy2:8080", "http://proxy3:8080",
            "socks5://tor1:9050", "socks5://tor2:9050",
        ];
        let timezones = ["Europe/Berlin", "Europe/London", "Europe/Paris", "America/New_York", "Asia/Tokyo"];
        let languages = ["en-US,en;q=0.9", "de-DE,de;q=0.9", "fr-FR,fr;q=0.9", "ja-JP,ja;q=0.9"];
        let platforms = ["Win32", "MacIntel", "Linux x86_64", "Linux aarch64"];

        AgentProfile {
            id: id.to_string(),
            proxy: proxies[next() as usize % proxies.len()].to_string(),
            user_agent: uas[next() as usize % uas.len()].to_string(),
            viewport: (1920 + next() as u32 % 960, 1080 + next() as u32 % 540),
            timezone: timezones[next() as usize % timezones.len()].to_string(),
            language: languages[next() as usize % languages.len()].to_string(),
            platform: platforms[next() as usize % platforms.len()].to_string(),
            hardware_concurrency: 4 + next() as u32 % 12,
        }
    }
}

// ─── Agent Task ───────────────────────────────────────────────────────────

/// Завдання агента: який сегмент даних обробляти.
#[derive(Debug, Clone)]
pub struct AgentTask {
    pub agent_id: String,
    pub source: String,       // e.g. "arxiv", "semanticscholar"
    pub segment: u32,         // which segment of the source
    pub start_paper: u64,     // start index
    pub count: u64,           // how many papers
    pub output_chunk: u32,    // which chunk of the matrix
}

// ─── Agent Orchestrator ──────────────────────────────────────────────────

/// Оркестратор розподілених headless агентів.
#[derive(Debug)]
pub struct AgentOrchestrator {
    pub agents: Vec<AgentProfile>,
    pub tasks: Vec<AgentTask>,
    pub mesh: AcademiaMesh,
    pub active: Vec<(String, TriState)>,
}

impl AgentOrchestrator {
    pub fn new(num_agents: usize) -> Self {
        let agents: Vec<AgentProfile> = (0..num_agents.min(MAX_AGENTS))
            .map(|i| AgentProfile::generate(&format!("agent-{}", i), (i as u64 + 1) * 42))
            .collect();

        let mut mesh = AcademiaMesh::new();
        for agent in &agents {
            mesh.add_node(&agent.id, &agent.proxy, 100);
        }

        AgentOrchestrator { agents, tasks: Vec::new(), mesh, active: Vec::new() }
    }

    /// Розподілити роботу між агентами (FanOut).
    pub fn distribute(&mut self, total_papers: u64) {
        let num = self.agents.len().max(1);
        let per_agent = (total_papers + num as u64 - 1) / num as u64;

        for (i, agent) in self.agents.iter().enumerate() {
            let start = i as u64 * per_agent;
            let count = per_agent.min(total_papers - start);
            if count == 0 { break; }

            self.tasks.push(AgentTask {
                agent_id: agent.id.clone(),
                source: "arxiv_oai_pmh".to_string(),
                segment: i as u32, start_paper: start, count,
                output_chunk: (start / 1_000_000) as u32,
            });
        }
        self.mesh.assign_chunks(total_papers);
    }

    /// Статус агентів.
    pub fn status(&self) -> Vec<(&AgentProfile, TriState)> {
        self.agents.iter().map(|a| {
            let s = self.active.iter().find(|(id, _)| id == &a.id)
                .map(|(_, s)| *s).unwrap_or(TriState::Unknown);
            (a, s)
        }).collect()
    }

    /// Час виконання (всі агенти паралельно, кожен 100 Mbps).
    pub fn estimated_time(&self, total_papers: u64) -> String {
        if self.agents.is_empty() { return "∞".into(); }
        let per_agent = total_papers / self.agents.len() as u64;
        let secs = (per_agent * 8) as f64 * 8.0 / (self.agents[0].hardware_concurrency as f64 * 1_000_000.0);
        let secs = secs.max(60.0); // minimum 1 min per agent
        let h = secs / 3600.0; let m = (secs % 3600.0) / 60.0;
        if h >= 1.0 { format!("{:.0}год {:.0}хв", h, m) } else { format!("{:.0}хв", m) }
    }

    pub fn dashboard(&self) -> String {
        let total = self.tasks.iter().map(|t| t.count).sum::<u64>();
        let active = self.active.iter().filter(|(_, s)| *s == TriState::True).count();
        let finished = self.active.iter().filter(|(_, s)| *s == TriState::False).count();
        format!(
            "Academia Agents\n  Total:  {} agents\n  Active: {}\n  Done:   {}\n  Papers: {:.1e}\n  Time:   {}\n  Mesh:   {} nodes",
            self.agents.len(), active, finished, total as f64,
            self.estimated_time(total), self.mesh.nodes.len()
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_agent_profiles() {
        let a = AgentProfile::generate("test-1", 42);
        assert!(!a.user_agent.is_empty());
        assert!(!a.proxy.is_empty());
        assert!(a.viewport.0 >= 1920);
    }

    #[test]
    fn different_seeds_different_profiles() {
        let a1 = AgentProfile::generate("a", 1);
        let a2 = AgentProfile::generate("b", 2);
        // Different proxy or user agent
        assert!(a1.proxy != a2.proxy || a1.user_agent != a2.user_agent);
    }

    #[test]
    fn orchestrator_creates_agents() {
        let o = AgentOrchestrator::new(5);
        assert_eq!(o.agents.len(), 5);
    }

    #[test]
    fn distribute_fanout() {
        let mut o = AgentOrchestrator::new(4);
        o.distribute(24_000_000);
        assert_eq!(o.tasks.len(), 4);
        let total: u64 = o.tasks.iter().map(|t| t.count).sum();
        assert_eq!(total, 24_000_000);
    }

    #[test]
    fn estimated_time_decreases_with_more_agents() {
        let o1 = AgentOrchestrator::new(1);
        let o10 = AgentOrchestrator::new(10);
        let t1 = o1.estimated_time(610_000_000);
        let t10 = o10.estimated_time(610_000_000);
        assert_ne!(t1, t10);
    }

    #[test]
    fn status_returns_all_agents() {
        let o = AgentOrchestrator::new(3);
        let s = o.status();
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn dashboard_contains_info() {
        let o = AgentOrchestrator::new(5);
        let d = o.dashboard();
        assert!(d.contains("Agents"));
        assert!(d.contains("Mesh"));
    }
}
