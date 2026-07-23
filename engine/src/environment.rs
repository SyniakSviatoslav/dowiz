//! EnvironmentSensor — погода, батарея, connectivity → параметри рендеру.
//!
//! Зчитує стан оточення (погоду, рівень батареї, тип з'єднання, час)
//! та перетворює його на дельти для фізики рендеру (wave energy,
//! Turing feed rate, microphysics stiffness, palette, bloom, map style).
//!
//! Жоден сенсор НЕ блокує рендер — якщо API недоступне, використовується
//! кешоване або default-значення. Усі зміни відбуваються через `RenderDeltas`,
//! які застосовуються до поточних параметрів рендеру.

/// Стан погоди.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherCondition {
    Clear,
    Rain,
    Cloudy,
    Snow,
}

/// Тип з'єднання.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectivityType {
    Wifi,
    Cellular,
    None,
}

/// Стан оточення.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvironmentState {
    pub weather: WeatherState,
    pub battery: BatteryState,
    pub connectivity: ConnectivityState,
    pub time: TimeState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherState {
    pub condition: WeatherCondition,
    pub temp_c: f32,
    pub humidity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryState {
    pub level: f32,
    pub charging: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectivityState {
    pub online: bool,
    pub conn_type: ConnectivityType,
    pub latency_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeState {
    pub hour: u32,
    pub day_of_week: u32,
    pub season: Season,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

/// Тип стилю мапи.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapStyle {
    Day,
    Night,
    Rain,
    Snow,
}

/// Дельта-зміни для параметрів рендеру на основі стану оточення.
///
/// Кожне поле додається до поточного значення параметра.
/// Додатне число = збільшити ефект, від'ємне = зменшити.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderDeltas {
    pub wave_energy_delta: f32,
    pub turing_feed_delta: f32,
    pub micro_stiffness_delta: f32,
    pub palette_warmth_delta: f32,
    pub bloom_strength_delta: f32,
    pub map_style: MapStyle,
}

impl RenderDeltas {
    pub const fn neutral() -> Self {
        RenderDeltas {
            wave_energy_delta: 0.0,
            turing_feed_delta: 0.0,
            micro_stiffness_delta: 0.0,
            palette_warmth_delta: 0.0,
            bloom_strength_delta: 0.0,
            map_style: MapStyle::Day,
        }
    }
}

/// Сенсор оточення.
///
/// Чисто CPU-side: не потребує API браузера чи зовнішніх запитів.
/// Стан оновлюється викликами `update_*` методів.
pub struct EnvironmentSensor {
    state: EnvironmentState,
    on_change: Vec<Box<dyn FnMut(&EnvironmentState)>>,
}

impl EnvironmentSensor {
    /// Початковий стан — нейтральний (clear, battery full, wifi, день).
    pub fn new() -> Self {
        EnvironmentSensor {
            state: EnvironmentState {
                weather: WeatherState {
                    condition: WeatherCondition::Clear,
                    temp_c: 20.0,
                    humidity: 50.0,
                },
                battery: BatteryState {
                    level: 1.0,
                    charging: true,
                },
                connectivity: ConnectivityState {
                    online: true,
                    conn_type: ConnectivityType::Wifi,
                    latency_ms: 10,
                },
                time: TimeState {
                    hour: 12,
                    day_of_week: 1,
                    season: Season::Summer,
                },
            },
            on_change: Vec::new(),
        }
    }

    pub fn get_state(&self) -> &EnvironmentState {
        &self.state
    }

    /// Обчислити дельти для рендеру на основі поточного стану.
    ///
    /// Використовується для мікро-реакцій: зміна параметрів wave/turing/
    /// microphysics/палітри/bloom залежно від погоди, батареї, часу.
    pub fn get_render_deltas(&self) -> RenderDeltas {
        let mut deltas = RenderDeltas::neutral();

        let s = &self.state;

        // Погода → wave energy, palette, map style
        match s.weather.condition {
            WeatherCondition::Clear => {
                deltas.wave_energy_delta += 0.2;
                deltas.palette_warmth_delta += 0.15;
            }
            WeatherCondition::Rain => {
                deltas.wave_energy_delta -= 0.2;
                deltas.palette_warmth_delta -= 0.15;
                deltas.map_style = MapStyle::Rain;
            }
            WeatherCondition::Cloudy => {
                deltas.wave_energy_delta -= 0.1;
                deltas.palette_warmth_delta -= 0.05;
            }
            WeatherCondition::Snow => {
                deltas.wave_energy_delta -= 0.15;
                deltas.palette_warmth_delta += 0.1;
                deltas.map_style = MapStyle::Snow;
            }
        }

        // Температура → Turing feed
        if s.weather.temp_c > 30.0 {
            deltas.turing_feed_delta += 0.01;
        } else if s.weather.temp_c < 5.0 {
            deltas.turing_feed_delta -= 0.01;
        }

        // Батарея → wave energy, microphysics stiffness
        if s.battery.level < 0.2 && !s.battery.charging {
            deltas.wave_energy_delta -= 0.3;
            deltas.micro_stiffness_delta -= 20.0;
        } else if s.battery.level < 0.5 && !s.battery.charging {
            deltas.wave_energy_delta -= 0.1;
            deltas.micro_stiffness_delta -= 5.0;
        }

        // Час → bloom, palette, map style
        if s.time.hour < 6 || s.time.hour > 20 {
            deltas.bloom_strength_delta += 0.3;
            deltas.palette_warmth_delta -= 0.2;
            deltas.map_style = if deltas.map_style == MapStyle::Day {
                MapStyle::Night
            } else {
                deltas.map_style
            };
        } else if s.time.hour < 8 || s.time.hour > 17 {
            deltas.bloom_strength_delta += 0.1;
            deltas.palette_warmth_delta -= 0.1;
        } else {
            deltas.bloom_strength_delta -= 0.1;
        }

        // Вологість → bloom
        if s.weather.humidity > 80.0 {
            deltas.bloom_strength_delta += 0.1;
        }

        deltas
    }

    pub fn update_weather(&mut self, weather: WeatherState) {
        self.state.weather = weather;
        self.emit();
    }

    pub fn update_battery(&mut self, battery: BatteryState) {
        self.state.battery = battery;
        self.emit();
    }

    pub fn update_connectivity(&mut self, connectivity: ConnectivityState) {
        self.state.connectivity = connectivity;
        self.emit();
    }

    pub fn update_time(&mut self, time: TimeState) {
        self.state.time = time;
        self.emit();
    }

    /// Підписатись на зміну стану оточення.
    pub fn on_change(&mut self, cb: Box<dyn FnMut(&EnvironmentState)>) {
        self.on_change.push(cb);
    }

    fn emit(&mut self) {
        for cb in &mut self.on_change {
            cb(&self.state);
        }
    }
}

impl Default for EnvironmentSensor {
    fn default() -> Self {
        Self::new()
    }
}

/// Допоміжна функція: визначити сезон за місяцем (1-12).
pub fn season_from_month(month: u32) -> Season {
    match month {
        3 | 4 | 5 => Season::Spring,
        6 | 7 | 8 => Season::Summer,
        9 | 10 | 11 => Season::Autumn,
        _ => Season::Winter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day_state() -> EnvironmentState {
        EnvironmentState {
            weather: WeatherState { condition: WeatherCondition::Clear, temp_c: 20.0, humidity: 50.0 },
            battery: BatteryState { level: 1.0, charging: true },
            connectivity: ConnectivityState { online: true, conn_type: ConnectivityType::Wifi, latency_ms: 10 },
            time: TimeState { hour: 12, day_of_week: 1, season: Season::Summer },
        }
    }

    #[test]
    fn neutral_state_yields_mostly_neutral_deltas() {
        let sensor = EnvironmentSensor::new();
        let deltas = sensor.get_render_deltas();
        // Clear + day + full battery → small positive wave_energy
        assert!(deltas.wave_energy_delta >= 0.0);
        assert_eq!(deltas.map_style, MapStyle::Day);
    }

    #[test]
    fn rain_reduces_energy() {
        let mut sensor = EnvironmentSensor::new();
        sensor.update_weather(WeatherState { condition: WeatherCondition::Rain, temp_c: 10.0, humidity: 90.0 });
        let deltas = sensor.get_render_deltas();
        assert!(deltas.wave_energy_delta < 0.0, "rain must reduce wave energy");
        assert_eq!(deltas.map_style, MapStyle::Rain);
        // High humidity → more bloom (neutralized by daytime -0.1 offset)
        assert!(deltas.bloom_strength_delta >= -0.1, "high humidity offsets daytime bloom reduction");
    }

    #[test]
    fn low_battery_reduces_effects() {
        let mut sensor = EnvironmentSensor::new();
        sensor.update_battery(BatteryState { level: 0.15, charging: false });
        let deltas = sensor.get_render_deltas();
        assert!(deltas.wave_energy_delta < 0.0, "low battery must reduce wave energy, got {}", deltas.wave_energy_delta);
        assert!(deltas.micro_stiffness_delta < 0.0, "low battery must reduce micro stiffness, got {}", deltas.micro_stiffness_delta);
    }

    #[test]
    fn night_time_adds_bloom_darkens_palette() {
        let mut sensor = EnvironmentSensor::new();
        sensor.update_time(TimeState { hour: 2, day_of_week: 3, season: Season::Winter });
        let deltas = sensor.get_render_deltas();
        assert!(deltas.bloom_strength_delta > 0.0, "night adds bloom");
        assert!(deltas.palette_warmth_delta < 0.0, "night cools palette");
        assert_eq!(deltas.map_style, MapStyle::Night);
    }

    #[test]
    fn snow_affects_style_and_energy() {
        let mut sensor = EnvironmentSensor::new();
        sensor.update_weather(WeatherState { condition: WeatherCondition::Snow, temp_c: -2.0, humidity: 70.0 });
        let deltas = sensor.get_render_deltas();
        assert!(deltas.wave_energy_delta < 0.0);
        assert_eq!(deltas.map_style, MapStyle::Snow);
    }

    #[test]
    fn hot_weather_increases_turing_feed() {
        let mut sensor = EnvironmentSensor::new();
        sensor.update_weather(WeatherState { condition: WeatherCondition::Clear, temp_c: 35.0, humidity: 30.0 });
        let deltas = sensor.get_render_deltas();
        assert!(deltas.turing_feed_delta > 0.0, "hot weather increases turing feed");
    }

    #[test]
    fn cold_weather_decreases_turing_feed() {
        let mut sensor = EnvironmentSensor::new();
        sensor.update_weather(WeatherState { condition: WeatherCondition::Clear, temp_c: 0.0, humidity: 30.0 });
        let deltas = sensor.get_render_deltas();
        assert!(deltas.turing_feed_delta < 0.0, "cold weather decreases turing feed");
    }

    #[test]
    fn modify_battery_triggers_callback() {
        use std::cell::Cell;
        use std::rc::Rc;
        let mut sensor = EnvironmentSensor::new();
        let last_level = Rc::new(Cell::new(1.0));
        let ll = last_level.clone();
        sensor.on_change(Box::new(move |s| { ll.set(s.battery.level); }));
        sensor.update_battery(BatteryState { level: 0.5, charging: true });
        assert!((last_level.get() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn season_from_month_works() {
        assert_eq!(season_from_month(1), Season::Winter);
        assert_eq!(season_from_month(4), Season::Spring);
        assert_eq!(season_from_month(7), Season::Summer);
        assert_eq!(season_from_month(10), Season::Autumn);
    }
}
