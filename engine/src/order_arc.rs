//! OrderArcSystem — discovery arcs для кожного етапу замовлення.
//!
//! Кожен етап замовлення — невелика арка з унікальним профілем фізики
//! (wave amplitude, Turing evolution speed, microphysics stiffness),
//! тривалістю та discovery-діями (splat_reveal, map_pulse, content_unfold,
//! courier_approach).
//!
//! При переході між етапами: попередній отримує exit, новий — enter.
//! Етапи можна переглядати, додавати кастомні хуки на enter/exit.

use crate::mode::{MicroProfile, TuringProfile, WaveProfile};

/// Етап замовлення.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OrderStage {
    Discover,
    Browse,
    Cart,
    Order,
    Track,
    Receive,
    Review,
}

impl OrderStage {
    pub fn name(&self) -> &'static str {
        match self {
            OrderStage::Discover => "discover",
            OrderStage::Browse => "browse",
            OrderStage::Cart => "cart",
            OrderStage::Order => "order",
            OrderStage::Track => "track",
            OrderStage::Receive => "receive",
            OrderStage::Review => "review",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            OrderStage::Discover => 0,
            OrderStage::Browse => 1,
            OrderStage::Cart => 2,
            OrderStage::Order => 3,
            OrderStage::Track => 4,
            OrderStage::Receive => 5,
            OrderStage::Review => 6,
        }
    }

    /// Усі етапи по порядку.
    pub fn all() -> &'static [OrderStage] {
        &[
            OrderStage::Discover,
            OrderStage::Browse,
            OrderStage::Cart,
            OrderStage::Order,
            OrderStage::Track,
            OrderStage::Receive,
            OrderStage::Review,
        ]
    }

    /// Наступний етап після поточного.
    pub fn next(self) -> Option<OrderStage> {
        match self {
            OrderStage::Discover => Some(OrderStage::Browse),
            OrderStage::Browse => Some(OrderStage::Cart),
            OrderStage::Cart => Some(OrderStage::Order),
            OrderStage::Order => Some(OrderStage::Track),
            OrderStage::Track => Some(OrderStage::Receive),
            OrderStage::Receive => Some(OrderStage::Review),
            OrderStage::Review => None,
        }
    }
}

/// Discovery-дія для етапу.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryAction {
    SplatReveal,
    MapPulse,
    ContentUnfold,
    CourierApproach,
}

/// Ваги етапу: налаштування фізики та тривалість.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageWeight {
    pub wave_amplitude: f32,
    pub turing_evolution_speed: f32,
    pub microphysics_stiffness: f32,
    pub duration_seconds: f32,
}

/// Профіль фізики для конкретного етапу.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderArc {
    pub stage: OrderStage,
    pub wave: WaveProfile,
    pub turing: TuringProfile,
    pub micro: MicroProfile,
    pub expected_duration_seconds: f32,
    pub discovery_actions: &'static [DiscoveryAction],
}

/// Карта ваг для кожного етапу.
const STAGE_WEIGHTS: &[StageWeight] = &[
    StageWeight { wave_amplitude: 0.3, turing_evolution_speed: 0.002, microphysics_stiffness: 40.0, duration_seconds: 120.0 },
    StageWeight { wave_amplitude: 0.5, turing_evolution_speed: 0.005, microphysics_stiffness: 60.0, duration_seconds: 300.0 },
    StageWeight { wave_amplitude: 0.7, turing_evolution_speed: 0.008, microphysics_stiffness: 80.0, duration_seconds: 180.0 },
    StageWeight { wave_amplitude: 1.0, turing_evolution_speed: 0.012, microphysics_stiffness: 100.0, duration_seconds: 90.0 },
    StageWeight { wave_amplitude: 0.6, turing_evolution_speed: 0.006, microphysics_stiffness: 50.0, duration_seconds: 600.0 },
    StageWeight { wave_amplitude: 0.8, turing_evolution_speed: 0.010, microphysics_stiffness: 70.0, duration_seconds: 120.0 },
    StageWeight { wave_amplitude: 0.4, turing_evolution_speed: 0.003, microphysics_stiffness: 30.0, duration_seconds: 300.0 },
];

/// Discovery-дії для кожного етапу.
const DISCOVERY_ACTIONS: &[&[DiscoveryAction]] = &[
    &[DiscoveryAction::SplatReveal, DiscoveryAction::MapPulse],
    &[DiscoveryAction::ContentUnfold],
    &[DiscoveryAction::SplatReveal, DiscoveryAction::ContentUnfold],
    &[DiscoveryAction::SplatReveal, DiscoveryAction::CourierApproach],
    &[DiscoveryAction::MapPulse, DiscoveryAction::CourierApproach],
    &[DiscoveryAction::SplatReveal, DiscoveryAction::ContentUnfold],
    &[DiscoveryAction::SplatReveal],
];

fn weight_for_stage(stage: OrderStage) -> &'static StageWeight {
    &STAGE_WEIGHTS[stage.index()]
}

fn actions_for_stage(stage: OrderStage) -> &'static [DiscoveryAction] {
    DISCOVERY_ACTIONS[stage.index()]
}

/// Система арок замовлення.
///
/// Керує поточним етапом, сповіщає про вхід/вихід,
/// надає поточний профіль фізики для рендер-двигуна.
pub struct OrderArcSystem {
    current: OrderStage,
    elapsed_seconds: f32,
    on_enter: Vec<Box<dyn FnMut(OrderStage)>>,
    on_exit: Vec<Box<dyn FnMut(OrderStage)>>,
}

impl OrderArcSystem {
    /// Створити систему з початковим етапом Discover.
    pub fn new() -> Self {
        OrderArcSystem {
            current: OrderStage::Discover,
            elapsed_seconds: 0.0,
            on_enter: Vec::new(),
            on_exit: Vec::new(),
        }
    }

    /// Поточний етап.
    pub fn current(&self) -> OrderStage {
        self.current
    }

    /// Час у поточному етапі (секунди).
    pub fn elapsed(&self) -> f32 {
        self.elapsed_seconds
    }

    /// Встановити етап. Викликає exit для старого, enter для нового.
    pub fn set_stage(&mut self, stage: OrderStage) {
        if stage == self.current {
            return;
        }
        for cb in &mut self.on_exit {
            cb(self.current);
        }
        self.current = stage;
        self.elapsed_seconds = 0.0;
        for cb in &mut self.on_enter {
            cb(stage);
        }
    }

    /// Перейти до наступного етапу.
    pub fn advance(&mut self) -> Option<OrderStage> {
        if let Some(next) = self.current.next() {
            self.set_stage(next);
            Some(next)
        } else {
            None
        }
    }

    /// Отримати поточний профіль арки (фізика + discovery actions).
    pub fn current_arc(&self) -> OrderArc {
        let w = weight_for_stage(self.current);
        OrderArc {
            stage: self.current,
            wave: WaveProfile {
                enabled: true,
                amplitude: w.wave_amplitude,
                frequency: 0.5,
                phase: 0.0,
                decay: 0.1,
            },
            turing: TuringProfile {
                enabled: true,
                feed_rate: w.turing_evolution_speed * 100.0,
                kill_rate: 0.06,
                diffusion_rate_u: 0.16,
                diffusion_rate_v: 0.08,
                injection_strength: w.wave_amplitude * 0.5,
                injection_radius: 12,
                grid_size: (128, 128),
            },
            micro: MicroProfile {
                enabled: true,
                stiffness: w.microphysics_stiffness,
                damping: 12.0,
                max_displacement: 30.0,
                rest_position: 0.0,
            },
            expected_duration_seconds: w.duration_seconds,
            discovery_actions: actions_for_stage(self.current),
        }
    }

    /// Оновити час (викликається кожен кадр).
    pub fn update(&mut self, dt_seconds: f32) {
        self.elapsed_seconds += dt_seconds;
    }

    /// Прогрес у поточному етапі (0.0 .. 1.0).
    pub fn progress(&self) -> f32 {
        let w = weight_for_stage(self.current);
        if w.duration_seconds <= 0.0 {
            return 1.0;
        }
        (self.elapsed_seconds / w.duration_seconds).clamp(0.0, 1.0)
    }

    /// Підписатись на вхід в етап.
    pub fn on_stage_enter(&mut self, cb: Box<dyn FnMut(OrderStage)>) {
        self.on_enter.push(cb);
    }

    /// Підписатись на вихід з етапу.
    pub fn on_stage_exit(&mut self, cb: Box<dyn FnMut(OrderStage)>) {
        self.on_exit.push(cb);
    }
}

impl Default for OrderArcSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_discover() {
        let arc = OrderArcSystem::new();
        assert_eq!(arc.current(), OrderStage::Discover);
        assert!((arc.progress() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn advance_moves_to_next_stage() {
        let mut arc = OrderArcSystem::new();
        assert_eq!(arc.advance(), Some(OrderStage::Browse));
        assert_eq!(arc.current(), OrderStage::Browse);
        assert!((arc.progress() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn advance_through_all_stages() {
        let mut arc = OrderArcSystem::new();
        let stages = OrderStage::all();
        for (i, _stage) in stages.iter().enumerate().skip(1) {
            assert_eq!(arc.advance().unwrap(), *stages.iter().nth(i).unwrap());
        }
        // After Review, advance returns None
        assert_eq!(arc.advance(), None);
    }

    #[test]
    fn set_stage_fires_enter_exit() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let mut arc = OrderArcSystem::new();
        let entered = Rc::new(RefCell::new(Vec::new()));
        let exited = Rc::new(RefCell::new(Vec::new()));
        {
            let e = entered.clone();
            arc.on_stage_enter(Box::new(move |s| e.borrow_mut().push(s)));
        }
        {
            let e = exited.clone();
            arc.on_stage_exit(Box::new(move |s| e.borrow_mut().push(s)));
        }

        arc.set_stage(OrderStage::Cart);
        assert_eq!(*entered.borrow(), vec![OrderStage::Cart]);
        assert_eq!(*exited.borrow(), vec![OrderStage::Discover]);

        arc.set_stage(OrderStage::Track);
        assert_eq!(*entered.borrow(), vec![OrderStage::Cart, OrderStage::Track]);
        assert_eq!(*exited.borrow(), vec![OrderStage::Discover, OrderStage::Cart]);
    }

    #[test]
    fn same_stage_does_not_fire_callbacks() {
        use std::cell::Cell;
        use std::rc::Rc;
        let mut arc = OrderArcSystem::new();
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        arc.on_stage_enter(Box::new(move |_| c.set(c.get() + 1)));
        arc.set_stage(OrderStage::Discover);
        assert_eq!(count.get(), 0, "same stage must not fire on_enter");
    }

    #[test]
    fn weights_are_consistent() {
        // Verify every stage has a weight entry
        for stage in OrderStage::all() {
            let w = weight_for_stage(*stage);
            assert!(w.wave_amplitude > 0.0);
            assert!(w.duration_seconds > 0.0);
        }
    }

    #[test]
    fn discovery_actions_never_empty() {
        for stage in OrderStage::all() {
            let actions = actions_for_stage(*stage);
            assert!(!actions.is_empty(), "{:?} must have at least one discovery action", stage);
        }
    }

    #[test]
    fn progress_increases_with_time() {
        let mut arc = OrderArcSystem::new();
        let p0 = arc.progress();
        arc.update(30.0);
        let p1 = arc.progress();
        assert!(p1 > p0, "progress must increase with time");
    }

    #[test]
    fn progress_clamps_at_one() {
        let mut arc = OrderArcSystem::new();
        arc.update(1_000_000.0);
        assert!((arc.progress() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn arc_profile_has_valid_params() {
        let arc = OrderArcSystem::new();
        let profile = arc.current_arc();
        assert!(profile.wave.enabled);
        assert!(profile.turing.enabled);
        assert!(profile.micro.enabled);
        assert!(!profile.discovery_actions.is_empty());
    }
}
