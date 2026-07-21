//! `kernel::power_forecast` — weather + grid load prediction for server time stability.
//!
//! Server location: Falkenstein, Germany (50.26°N, 12.36°E, elevation 565m)
//! Grid: ENTSO-E Continental Europe (50 Hz), DE region
//!
//! Server clock stability is affected by:
//! - CPU temperature (thermal throttling → clock drift)
//! - Power grid load (voltage droop → frequency variation)
//! - Ambient temperature (cooling efficiency → sustained load)
//!
//! This module forecasts these factors from recent observations and
//! external APIs. The forecast feeds into `TimeStabilizer`'s drift model
//! so ticks are predicted with awareness of external thermal/grid stress.
//!
//! # Falkenstein, Germany — constants
//! - Latitude:  50.26°N
//! - Longitude: 12.36°E
//! - Elevation: 565 m
//! - Grid:      ENTSO-E (50 Hz), DE (Germany) bidding zone
//! - Timezone:  Europe/Berlin (CET/CEST, UTC+1/+2)
//! - Climate:   Central European (cool temperate, no extreme heat)
//!
//! # Data sources (external APIs, behind port seam at runtime)
//! - Open-Meteo (free, no key) for ambient temperature, humidity, wind
//! - ENTSO-E Transparency Platform for grid load forecast (DE region)
//! - CPU-internal thermal sensors (via /sys/class/thermal/)
//!
//! # Kernel module (pure computation)
//! The forecast engine is pure computation (EMA + linear trends).
//! The actual API calls happen outside the kernel in an adapter.
//!
//! innovate: ceiling — only EMA + linear trend forecast.
//! upgrade: when >10^4 observations, enable seasonal ARIMA.

use crate::TriState;

/// Minimum observations before forecast is reliable.
pub const MIN_FORECAST_SAMPLES: usize = 12;

/// Falkenstein, Germany geographic constants.
pub const FALKENSTEIN_LAT: f64 = 50.26;
pub const FALKENSTEIN_LON: f64 = 12.36;
pub const FALKENSTEIN_ELEVATION_M: f64 = 565.0;

/// Grid constants (ENTSO-E Continental Europe).
pub const GRID_NOMINAL_HZ: f64 = 50.0;
pub const GRID_REGION_DE: &str = "DE";
/// Typical German grid load at 2026-07-21 15:00 CEST on a summer weekday
/// (~45-55 GW). Used as initial forecast baseline.
pub const TYPICAL_GRID_LOAD_MW: f64 = 50_000.0;

/// Summer ambient baseline for Falkenstein data center (July mean ~16°C).
pub const AMBIENT_BASELINE_MDEG: u64 = 16_000; // 16°C in millidegrees

// ─── Temperature Sample ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TempSample {
    /// CPU package temperature (millidegrees Celsius).
    pub cpu_mdeg: u64,
    /// Ambient temperature (millidegrees Celsius), 0 = unknown.
    pub ambient_mdeg: u64,
    /// Timestamp (ns).
    pub timestamp_ns: u64,
    /// Sample validity.
    pub valid: TriState,
}

// ─── Grid Load Sample ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GridSample {
    /// Current grid load (MW).
    pub load_mw: f64,
    /// Grid frequency (Hz). Nominal = 50.0 or 60.0.
    pub frequency_hz: f64,
    /// Timestamp.
    pub timestamp_ns: u64,
    /// Validity.
    pub valid: TriState,
}

// ─── Weather Sample ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WeatherSample {
    /// Ambient temperature (Celsius * 100 = centidegrees).
    pub temp_centi: i32,
    /// Relative humidity (percent).
    pub humidity_pct: f64,
    /// Barometric pressure (hPa).
    pub pressure_hpa: f64,
    /// Wind speed (m/s).
    pub wind_ms: f64,
    /// Timestamp.
    pub timestamp_ns: u64,
    /// Validity.
    pub valid: TriState,
}

// ─── Forecast ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PowerForecast {
    /// Predicted CPU temperature (millidegrees) in 1 minute.
    pub cpu_mdeg_1m: u64,
    /// Predicted ambient temperature (millidegrees) in 1 minute.
    pub ambient_mdeg_1m: u64,
    /// Predicted grid load (MW) in 1 minute.
    pub grid_mw_1m: f64,
    /// Expected clock drift contribution from thermal (ns/s).
    pub thermal_drift_ns_per_s: f64,
    /// Expected clock drift contribution from grid (ns/s).
    pub grid_drift_ns_per_s: f64,
    /// Combined expected drift (ns/s).
    pub total_drift_ns_per_s: f64,
    /// Forecast confidence.
    pub confidence: f64,
}

// ─── Thermal Observer ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ThermalObserver {
    temps: Vec<TempSample>,
    max_samples: usize,
}

impl ThermalObserver {
    pub fn new(max_samples: usize) -> Self {
        ThermalObserver { temps: Vec::with_capacity(max_samples), max_samples }
    }

    pub fn observe(&mut self, sample: TempSample) {
        if self.temps.len() >= self.max_samples {
            self.temps.remove(0);
        }
        self.temps.push(sample);
    }

    pub fn forecast_cpu_temp(&self) -> (f64, f64) {
        let n = self.temps.len();
        if n < 2 {
            let last = self.temps.last().map(|t| t.cpu_mdeg as f64).unwrap_or(40000.0);
            return (last, 0.0); // 40C default (typical Falkenstein DC ambient + IT load), no trend
        }
        let mean: f64 = self.temps.iter().map(|t| t.cpu_mdeg as f64).sum::<f64>() / n as f64;
        let slope = if n >= 3 {
            (self.temps[n-1].cpu_mdeg as f64 - self.temps[0].cpu_mdeg as f64) / n as f64
        } else { 0.0 };
        (mean, slope)
    }
}

// ─── Grid Observer ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GridObserver {
    samples: Vec<GridSample>,
    max_samples: usize,
    /// Nominal grid frequency (50.0 or 60.0).
    pub nominal_hz: f64,
}

impl GridObserver {
    pub fn new(max_samples: usize, nominal_hz: f64) -> Self {
        GridObserver { samples: Vec::with_capacity(max_samples), max_samples, nominal_hz }
    }

    pub fn observe(&mut self, sample: GridSample) {
        if self.samples.len() >= self.max_samples {
            self.samples.remove(0);
        }
        self.samples.push(sample);
    }

    pub fn forecast(&self) -> (f64, f64) {
        let n = self.samples.len();
        if n < 2 {
            return (self.samples.last().map(|s| s.load_mw).unwrap_or(0.0), 0.0);
        }
        let mean: f64 = self.samples.iter().map(|s| s.load_mw).sum::<f64>() / n as f64;
        let mean_freq: f64 = self.samples.iter().map(|s| s.frequency_hz).sum::<f64>() / n as f64;
        let freq_dev = mean_freq - self.nominal_hz;
        (mean, freq_dev)
    }

    /// Drift contribution from grid frequency deviation (ppm).
    pub fn drift_ppm(&self) -> f64 {
        let (_, freq_dev) = self.forecast();
        if freq_dev.abs() < 0.01 { return 0.0; }
        // Every 0.01 Hz deviation ≈ 200 ppm clock drift on PLL-synchronized clocks.
        (freq_dev / 0.01) * 200.0
    }
}

// ─── Power Forecast Engine ────────────────────────────────────────────────

/// Combined power + thermal forecast engine.
pub struct PowerForecastEngine {
    pub thermal: ThermalObserver,
    pub grid: GridObserver,
    pub weather: Vec<WeatherSample>,
    max_weather: usize,
}

impl PowerForecastEngine {
    pub fn new() -> Self {
        PowerForecastEngine {
            thermal: ThermalObserver::new(1000),
            grid: GridObserver::new(1000, GRID_NOMINAL_HZ),
            weather: Vec::with_capacity(100),
            max_weather: 100,
        }
    }

    pub fn observe_temp(&mut self, cpu_mdeg: u64, ambient_mdeg: u64, now_ns: u64) {
        self.thermal.observe(TempSample {
            cpu_mdeg, ambient_mdeg, timestamp_ns: now_ns, valid: TriState::True,
        });
    }

    pub fn observe_grid(&mut self, load_mw: f64, frequency_hz: f64, now_ns: u64) {
        self.grid.observe(GridSample {
            load_mw, frequency_hz, timestamp_ns: now_ns, valid: TriState::True,
        });
    }

    pub fn observe_weather(&mut self, sample: WeatherSample) {
        if self.weather.len() >= self.max_weather {
            self.weather.remove(0);
        }
        self.weather.push(sample);
    }

    /// Compute the current power forecast.
    pub fn forecast(&self) -> PowerForecast {
        let (cpu_mean, cpu_slope) = self.thermal.forecast_cpu_temp();
        let cpu_1m = (cpu_mean + cpu_slope * 60.0) as u64;

        let ambient_mean = self.thermal.temps.last()
            .map(|t| t.ambient_mdeg)
            .unwrap_or(AMBIENT_BASELINE_MDEG);
        let ambient_trend = if self.thermal.temps.len() >= 3 {
            let first = self.thermal.temps.first().map(|t| t.ambient_mdeg as f64).unwrap_or(0.0);
            let last = self.thermal.temps.last().map(|t| t.ambient_mdeg as f64).unwrap_or(0.0);
            ((last - first) / self.thermal.temps.len() as f64) * 60.0
        } else { 0.0 };
        let ambient_1m = (ambient_mean as f64 + ambient_trend) as u64;

        let grid_drift_ppm = self.grid.drift_ppm();

        // Thermal drift: each 10°C above 40°C baseline ≈ 1 ppm clock drift.
        let cpu_delta = (cpu_mean - 40000.0).max(0.0) / 10_000.0; // 10°C units
        let thermal_drift_ppm = cpu_delta * 1.0;

        let (grid_load, _) = self.grid.forecast();
        let total_drift_ppm = thermal_drift_ppm + grid_drift_ppm;
        let drift_ns_per_s = total_drift_ppm / 1_000_000.0 * 1_000_000_000.0;

        let confidence = if self.thermal.temps.len() >= MIN_FORECAST_SAMPLES
            && self.grid.samples.len() >= MIN_FORECAST_SAMPLES
        { 0.85 } else { 0.3 };

        PowerForecast {
            cpu_mdeg_1m: cpu_1m,
            ambient_mdeg_1m: ambient_1m,
            grid_mw_1m: grid_load,
            thermal_drift_ns_per_s: thermal_drift_ppm * 1000.0,
            grid_drift_ns_per_s: grid_drift_ppm * 1000.0,
            total_drift_ns_per_s: drift_ns_per_s,
            confidence,
        }
    }

    pub fn dashboard(&self) -> String {
        let fc = self.forecast();
        let mut out = String::with_capacity(256);
        out.push_str("Power Forecast\n");
        out.push_str(&format!("  CPU temp:    {} mdeg (1m: {})\n",
            self.thermal.temps.last().map(|t| t.cpu_mdeg).unwrap_or(0), fc.cpu_mdeg_1m));
        out.push_str(&format!("  Ambient:     {} mdeg (1m: {})\n",
            self.thermal.temps.last().map(|t| t.ambient_mdeg).unwrap_or(0), fc.ambient_mdeg_1m));
        out.push_str(&format!("  Grid load:   {:.0} MW\n", fc.grid_mw_1m));
        out.push_str(&format!("  Thermal drf: {:.3} ns/s\n", fc.thermal_drift_ns_per_s));
        out.push_str(&format!("  Grid drf:    {:.3} ns/s\n", fc.grid_drift_ns_per_s));
        out.push_str(&format!("  Total drf:   {:.3} ns/s\n", fc.total_drift_ns_per_s));
        out.push_str(&format!("  Confidence:  {:.0}%\n", fc.confidence * 100.0));
        out
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_observer_trend() {
        let mut obs = ThermalObserver::new(10);
        for i in 0..5 {
            obs.observe(TempSample {
                cpu_mdeg: 40000 + i * 100, ambient_mdeg: 20000,
                timestamp_ns: i as u64 * 1_000_000, valid: TriState::True,
            });
        }
        let (mean, slope) = obs.forecast_cpu_temp();
        assert!(mean > 0.0);
        assert!(slope >= 0.0);
    }

    #[test]
    fn grid_observer_drift() {
        let mut obs = GridObserver::new(10, 50.0);
        obs.observe(GridSample {
            load_mw: 5000.0, frequency_hz: 49.99,
            timestamp_ns: 1000, valid: TriState::True,
        });
        obs.observe(GridSample {
            load_mw: 5100.0, frequency_hz: 49.98,
            timestamp_ns: 2000, valid: TriState::True,
        });
        assert!(obs.drift_ppm() != 0.0);
    }

    #[test]
    fn power_forecast_defaults() {
        let pfe = PowerForecastEngine::new();
        let fc = pfe.forecast();
        assert!(fc.confidence < 0.5);
        assert_eq!(fc.cpu_mdeg_1m, 40000); // 40°C default (DC ambient + IT load)
        assert_eq!(fc.ambient_mdeg_1m, AMBIENT_BASELINE_MDEG);
        assert_eq!(pfe.grid.nominal_hz, GRID_NOMINAL_HZ);
    }

    #[test]
    fn power_forecast_with_data() {
        let mut pfe = PowerForecastEngine::new();
        for i in 0..15 {
            pfe.observe_temp(45000 + i * 50, 22000 + i * 10, i as u64 * 1_000_000);
            pfe.observe_grid(5000.0 + i as f64 * 10.0, 50.0, i as u64 * 1_000_000);
        }
        let fc = pfe.forecast();
        assert!(fc.confidence > 0.5);
        assert!(fc.cpu_mdeg_1m > 40000);
    }

    #[test]
    fn dashboard_contains_power() {
        let pfe = PowerForecastEngine::new();
        let d = pfe.dashboard();
        assert!(d.contains("Power Forecast"));
    }
}
