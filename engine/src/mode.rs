//! ModeController — Атмосфера / Справа (Atmosphere / Business).
//!
//! Два режими роботи інтерфейсу:
//! - **Atmosphere** (режисерський): повні анімації, хвилі, Turing-текстури,
//!   microphysics, наративні переходи, discovery-ефекти. Для першого знайомства
//!   та коли важливий досвід.
//! - **Business** (швидкий): усі анімації, переходи, декоративні ефекти вимкнені.
//!   Максимальна швидкість без очікування. Для досвідчених користувачів,
//!   низького заряду батареї, або коли важлива тільки швидкість.
//!
//! Перемикання між режимами НІКОЛИ не втрачає стан замовлення (P64 §3.1:
//! presentation-only — замовлення належить kernel FSM). Режим впливає тільки
//! на презентацію: які анімації, переходи, ефекти відображаються.

/// Тип режиму інтерфейсу.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeType {
    /// Повний досвід: анімації, хвилі, Turing-реакції, наративні переходи.
    #[default]
    Atmosphere,
    /// Швидкий: усе вимкнено, максимальна продуктивність.
    Business,
}

impl ModeType {
    pub fn name(&self) -> &'static str {
        match self {
            ModeType::Atmosphere => "atmosphere",
            ModeType::Business => "business",
        }
    }

    pub fn is_atmosphere(&self) -> bool {
        matches!(self, ModeType::Atmosphere)
    }

    pub fn is_business(&self) -> bool {
        matches!(self, ModeType::Business)
    }
}

/// Профіль анімаційних параметрів для режиму.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveProfile {
    pub enabled: bool,
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    pub decay: f32,
}

impl WaveProfile {
    pub const fn disabled() -> Self {
        WaveProfile { enabled: false, amplitude: 0.0, frequency: 0.0, phase: 0.0, decay: 0.0 }
    }

    pub const fn atmosphere() -> Self {
        WaveProfile { enabled: true, amplitude: 0.6, frequency: 0.5, phase: 0.0, decay: 0.1 }
    }
}

/// Профіль Turing-реакції-дифузії для режиму.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TuringProfile {
    pub enabled: bool,
    pub feed_rate: f32,
    pub kill_rate: f32,
    pub diffusion_rate_u: f32,
    pub diffusion_rate_v: f32,
    pub injection_strength: f32,
    pub injection_radius: u32,
    pub grid_size: (u32, u32),
}

impl TuringProfile {
    pub const fn disabled() -> Self {
        TuringProfile {
            enabled: false, feed_rate: 0.0, kill_rate: 0.0,
            diffusion_rate_u: 0.0, diffusion_rate_v: 0.0,
            injection_strength: 0.0, injection_radius: 0, grid_size: (0, 0),
        }
    }

    pub const fn atmosphere() -> Self {
        TuringProfile {
            enabled: true, feed_rate: 0.035, kill_rate: 0.06,
            diffusion_rate_u: 0.16, diffusion_rate_v: 0.08,
            injection_strength: 0.3, injection_radius: 12,
            grid_size: (128, 128),
        }
    }
}

/// Профіль microphysics (пружинки) для режиму.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MicroProfile {
    pub enabled: bool,
    pub stiffness: f32,
    pub damping: f32,
    pub max_displacement: f32,
    pub rest_position: f32,
}

impl MicroProfile {
    pub const fn disabled() -> Self {
        MicroProfile { enabled: false, stiffness: 0.0, damping: 0.0, max_displacement: 0.0, rest_position: 0.0 }
    }

    pub const fn atmosphere() -> Self {
        MicroProfile { enabled: true, stiffness: 60.0, damping: 12.0, max_displacement: 30.0, rest_position: 0.0 }
    }
}

/// Тип мапи для режиму.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapProfile {
    Explore,
    Minimal,
}

/// Набір активних профілів для поточного режиму.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActiveProfiles {
    pub wave: WaveProfile,
    pub turing: TuringProfile,
    pub micro: MicroProfile,
    pub map: MapProfile,
    pub transitions: bool,
    pub haptics: bool,
}

/// Контролер режимів інтерфейсу.
///
/// Зберігає поточний режим та сповіщає підписників про зміни.
/// Стан замовлення НІКОЛИ не зберігається тут — це presentation-only.
pub struct ModeController {
    mode: ModeType,
    on_change: Vec<Box<dyn FnMut(ModeType)>>,
}

impl ModeController {
    /// Створити новий контролер у режимі Atmosphere (за замовчуванням).
    pub fn new() -> Self {
        ModeController {
            mode: ModeType::Atmosphere,
            on_change: Vec::new(),
        }
    }

    pub fn get_mode(&self) -> ModeType {
        self.mode
    }

    /// Встановити режим. Сповіщає підписників тільки при реальній зміні.
    pub fn set_mode(&mut self, mode: ModeType) {
        if mode == self.mode {
            return;
        }
        self.mode = mode;
        for cb in &mut self.on_change {
            cb(mode);
        }
    }

    /// Перемкнути між Atmosphere ↔ Business.
    pub fn toggle(&mut self) {
        let next = match self.mode {
            ModeType::Atmosphere => ModeType::Business,
            ModeType::Business => ModeType::Atmosphere,
        };
        self.set_mode(next);
    }

    /// Автоматично вибрати режим на основі контексту.
    ///
    /// Повертає рекомендований режим, але НЕ змінює поточний —
    /// виклик [`set_mode`] окремий.
    pub fn auto_select(context: ModeContext) -> ModeType {
        if context.is_first_visit {
            return ModeType::Atmosphere;
        }
        if context.battery_level < 0.2 {
            return ModeType::Business;
        }
        if !context.is_online {
            return ModeType::Business;
        }
        if context.order_count >= 3 {
            return ModeType::Business;
        }
        ModeType::Atmosphere
    }

    /// Отримати активні профілі для поточного режиму.
    pub fn get_active_profiles(&self) -> ActiveProfiles {
        match self.mode {
            ModeType::Business => ActiveProfiles {
                wave: WaveProfile::disabled(),
                turing: TuringProfile::disabled(),
                micro: MicroProfile::disabled(),
                map: MapProfile::Minimal,
                transitions: false,
                haptics: false,
            },
            ModeType::Atmosphere => ActiveProfiles {
                wave: WaveProfile::atmosphere(),
                turing: TuringProfile::atmosphere(),
                micro: MicroProfile::atmosphere(),
                map: MapProfile::Explore,
                transitions: true,
                haptics: true,
            },
        }
    }

    /// Підписатись на зміну режиму.
    pub fn on_mode_change(&mut self, cb: Box<dyn FnMut(ModeType)>) {
        self.on_change.push(cb);
    }
}

impl Default for ModeController {
    fn default() -> Self {
        Self::new()
    }
}

/// Контекст для автоматичного вибору режиму.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModeContext {
    pub is_first_visit: bool,
    pub battery_level: f32,
    pub is_online: bool,
    pub order_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_atmosphere() {
        let mc = ModeController::new();
        assert_eq!(mc.get_mode(), ModeType::Atmosphere);
    }

    #[test]
    fn toggle_switches_modes() {
        let mut mc = ModeController::new();
        assert_eq!(mc.get_mode(), ModeType::Atmosphere);
        mc.toggle();
        assert_eq!(mc.get_mode(), ModeType::Business);
        mc.toggle();
        assert_eq!(mc.get_mode(), ModeType::Atmosphere);
    }

    #[test]
    fn set_mode_fires_callback() {
        use std::cell::Cell;
        use std::rc::Rc;
        let mut mc = ModeController::new();
        let last_mode = Rc::new(Cell::new(ModeType::Atmosphere));
        let lm = last_mode.clone();
        mc.on_mode_change(Box::new(move |m| { lm.set(m); }));

        mc.set_mode(ModeType::Business);
        assert_eq!(last_mode.get(), ModeType::Business);

        // Same mode — no callback
        mc.set_mode(ModeType::Business);
        assert_eq!(last_mode.get(), ModeType::Business);
    }

    #[test]
    fn auto_select_uses_context() {
        // First visit → Atmosphere
        assert_eq!(
            ModeController::auto_select(ModeContext {
                is_first_visit: true, battery_level: 1.0, is_online: true, order_count: 0,
            }),
            ModeType::Atmosphere,
        );
        // Low battery → Business
        assert_eq!(
            ModeController::auto_select(ModeContext {
                is_first_visit: false, battery_level: 0.15, is_online: true, order_count: 0,
            }),
            ModeType::Business,
        );
        // Offline → Business
        assert_eq!(
            ModeController::auto_select(ModeContext {
                is_first_visit: false, battery_level: 0.5, is_online: false, order_count: 0,
            }),
            ModeType::Business,
        );
        // 3+ orders → Business
        assert_eq!(
            ModeController::auto_select(ModeContext {
                is_first_visit: false, battery_level: 0.5, is_online: true, order_count: 5,
            }),
            ModeType::Business,
        );
        // Normal → Atmosphere
        assert_eq!(
            ModeController::auto_select(ModeContext {
                is_first_visit: false, battery_level: 0.8, is_online: true, order_count: 1,
            }),
            ModeType::Atmosphere,
        );
    }

    #[test]
    fn active_profiles_match_mode() {
        let mut mc = ModeController::new();

        let atm = {
            mc.set_mode(ModeType::Atmosphere);
            mc.get_active_profiles()
        };
        assert!(atm.wave.enabled);
        assert!(atm.turing.enabled);
        assert!(atm.micro.enabled);
        assert_eq!(atm.map, MapProfile::Explore);
        assert!(atm.transitions);
        assert!(atm.haptics);

        let bus = {
            mc.set_mode(ModeType::Business);
            mc.get_active_profiles()
        };
        assert!(!bus.wave.enabled);
        assert!(!bus.turing.enabled);
        assert!(!bus.micro.enabled);
        assert_eq!(bus.map, MapProfile::Minimal);
        assert!(!bus.transitions);
        assert!(!bus.haptics);
    }
}
