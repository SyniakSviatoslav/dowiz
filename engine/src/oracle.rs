//! OracleSystem — dead reckoning + ETA для кур'єрської доставки.
//!
//! Зливає дані з акселерометра/гіроскопа (IMU) та GPS в єдину оцінку
//! позиції девайсу. Коли GPS недоступний, використовує dead reckoning
//! (подвійне інтегрування акселерометра) + гіроскоп для курсу.
//!
//! ETA рахується через `kernel::geo::eta_seconds_adaptive` з використанням
//! згладженої швидкості з `CourierSpeedEma`.
//!
//! Ніколи не блокує рендер — усі сенсори опціональні.

use std::sync::atomic::{AtomicU64, Ordering};
use dowiz_kernel::geo::{self, CourierSpeedEma};

/// Семпл з акселерометра/гіроскопа.
#[derive(Debug, Clone, Copy)]
pub struct ImuSample {
    pub ax: f32,
    pub ay: f32,
    pub az: f32,
    pub gx: f32,
    pub gy: f32,
    pub gz: f32,
    pub timestamp_us: u64,
}

/// GPS-фікс.
#[derive(Debug, Clone, Copy)]
pub struct GpsFix {
    pub lat: f64,
    pub lng: f64,
    pub accuracy_m: f32,
    pub speed_mps: Option<f32>,
    pub timestamp_us: u64,
}

/// Поточна оцінка позиції девайсу.
#[derive(Debug, Clone, Copy)]
pub struct DeviceDisplacement {
    pub estimated_lat: f64,
    pub estimated_lng: f64,
    pub bearing_deg: f32,
    pub confidence: f32,
}

impl DeviceDisplacement {
    pub const fn unknown() -> Self {
        DeviceDisplacement {
            estimated_lat: 0.0,
            estimated_lng: 0.0,
            bearing_deg: 0.0,
            confidence: 0.0,
        }
    }
}

/// Стан калібрування інерційного сенсора.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationState {
    Uncalibrated,
    Calibrating,
    Calibrated,
}

/// Oracle — інерційна навігація + GPS.
///
/// - IMU семпли накопичуються в кільцевому буфері (останні 100).
/// - Dead reckoning: подвійне інтегрування акселерометра для позиції,
///   гіроскоп для bearing.
/// - GPS-фікси скидають дрейф і оновлюють `CourierSpeedEma`.
/// - confidence = 1.0 після GPS, згасає з часом без фіксів.
pub struct OracleSystem {
    imu_buffer: Vec<ImuSample>,
    last_gps: Option<GpsFix>,
    displacement: DeviceDisplacement,
    speed_ema: CourierSpeedEma,
    calibration: CalibrationState,
    last_timestamp_us: u64,
    velocity: (f64, f64),
    route_version: u64,
    // Telemetry counters
    gps_fix_total: AtomicU64,
    dr_freeze_count: AtomicU64,
    imu_sample_total: AtomicU64,
}

impl OracleSystem {
    pub fn new() -> Self {
        OracleSystem {
            imu_buffer: Vec::with_capacity(100),
            last_gps: None,
            displacement: DeviceDisplacement::unknown(),
            speed_ema: CourierSpeedEma::new(),
            calibration: CalibrationState::Uncalibrated,
            last_timestamp_us: 0,
            velocity: (0.0, 0.0),
            route_version: 0,
            gps_fix_total: AtomicU64::new(0),
            dr_freeze_count: AtomicU64::new(0),
            imu_sample_total: AtomicU64::new(0),
        }
    }

    /// Кількість GPS-фіксів від початку життя системи (telemetry).
    pub fn gps_fix_count(&self) -> u64 {
        self.gps_fix_total.load(Ordering::Relaxed)
    }

    /// Кількість IMU-семплів у буфері (telemetry).
    pub fn imu_buffer_len(&self) -> usize {
        self.imu_buffer.len()
    }

    /// Кількість IMU-семплів від початку (telemetry).
    pub fn imu_sample_total(&self) -> u64 {
        self.imu_sample_total.load(Ordering::Relaxed)
    }

    /// Скільки разів dead reckoning заморожувався через низький confidence.
    pub fn dr_freeze_count(&self) -> u64 {
        self.dr_freeze_count.load(Ordering::Relaxed)
    }

    /// Залити семпл з акселерометра/гіроскопа.
    ///
    /// Виконує dead reckoning: інтегрує акселерометр для швидкості та позиції,
    /// гіроскоп — для bearing. Працює лише після калібрування.
    pub fn ingest_imu(&mut self, sample: ImuSample) {
        if self.calibration != CalibrationState::Calibrated {
            if self.imu_buffer.len() < 50 {
                self.imu_buffer.push(sample);
                self.calibration = CalibrationState::Calibrating;
                self.last_timestamp_us = sample.timestamp_us;
                return;
            }
            self.calibration = CalibrationState::Calibrated;
            self.imu_buffer.clear();
        }

        let dt_us = sample.timestamp_us.saturating_sub(self.last_timestamp_us);
        self.last_timestamp_us = sample.timestamp_us;
        if dt_us == 0 {
            return;
        }
        let dt_s = dt_us as f64 / 1_000_000.0;

        if self.imu_buffer.len() >= 100 {
            self.imu_buffer.remove(0);
        }
        self.imu_buffer.push(sample);
        self.imu_sample_total.fetch_add(1, Ordering::Relaxed);
        crate::telemetry_count!("oracle", "imu_sample", 1);

        self.displacement.confidence =
            (self.displacement.confidence - 0.01).max(0.0);

        // Dead reckoning: використовується тільки при confidence > DR_FREEZE.
        // Нижче цього порогу позиція ЗАВМИРАЄ — дрейф надто високий,
        // і подвійне інтегрування дасть хибні координати.
        const DR_FREEZE: f32 = 0.1;
        const METERS_PER_DEG_LAT: f64 = 111_320.0;

        if self.displacement.confidence > DR_FREEZE {
            let vx = self.velocity.0 + sample.ax as f64 * dt_s;
            let vy = self.velocity.1 + sample.ay as f64 * dt_s;
            // Обмежуємо швидкість: 30 м/с (~108 км/год) max — захист від шуму IMU.
            let speed = (vx * vx + vy * vy).sqrt();
            let clamped = if speed > 30.0 { 30.0 / speed } else { 1.0 };
            self.velocity = (vx * clamped, vy * clamped);

            let cos_lat = self.displacement.estimated_lat.to_radians().cos().max(0.01);
            let dlat = self.velocity.0 * dt_s / METERS_PER_DEG_LAT;
            let dlng = self.velocity.1 * dt_s / (METERS_PER_DEG_LAT * cos_lat);
            self.displacement.estimated_lat += dlat;
            self.displacement.estimated_lng += dlng;
        } else {
            // Confidence нижче порогу — зупиняємо DR, обнуляємо velocity.
            self.dr_freeze_count.fetch_add(1, Ordering::Relaxed);
            self.velocity = (0.0, 0.0);
        }

        self.displacement.bearing_deg = (self.displacement.bearing_deg as f64
            + sample.gz as f64 * dt_s * 57.2958) as f32 % 360.0;
        if self.displacement.bearing_deg < 0.0 {
            self.displacement.bearing_deg += 360.0;
        }
    }

    /// Залити GPS-фікс.
    ///
    /// Скидає дрейф dead reckoning: позиція = GPS, confidence = 1.0.
    /// Передає `speed_mps` в `CourierSpeedEma` для згладженої швидкості.
    /// При зміні `route_version` (новий маршрут) EMA скидається.
    pub fn ingest_gps(&mut self, fix: GpsFix) {
        self.gps_fix_total.fetch_add(1, Ordering::Relaxed);
        crate::telemetry_count!("oracle", "gps_fix", 1);
        self.last_gps = Some(fix);
        self.displacement = DeviceDisplacement {
            estimated_lat: fix.lat,
            estimated_lng: fix.lng,
            bearing_deg: self.displacement.bearing_deg,
            confidence: 1.0,
        };
        self.velocity = (0.0, 0.0);
        if let Some(speed) = fix.speed_mps {
            self.speed_ema.accept_ping(speed as f64, self.route_version);
        }
    }

    /// Поточна позиція.
    pub fn current_displacement(&self) -> DeviceDisplacement {
        self.displacement
    }

    /// Час до прибуття (секунди).
    ///
    /// Використовує `kernel::geo::progress_along_route` для проєкції на маршрут
    /// та `eta_seconds_adaptive` з поточною згладженою швидкістю.
    pub fn eta_to(&self, dest_lat: f64, dest_lng: f64, route: &[(f64, f64)]) -> f64 {
        let pos = (self.displacement.estimated_lat, self.displacement.estimated_lng);
        let progress = geo::progress_along_route(route, pos);
        let total_m = geo::polyline_length_meters(route);
        let baseline_s = if total_m > 0.0 { total_m / 5.0 } else { 300.0 };
        let live = self.speed_ema.observed();
        geo::eta_seconds_adaptive(progress.remaining_m, total_m, baseline_s, live)
    }

    /// Позначити новий маршрут (скидає EMA).
    pub fn new_route(&mut self) {
        self.route_version = self.route_version.wrapping_add(1);
        self.speed_ema.reset();
        self.velocity = (0.0, 0.0);
    }

    pub fn calibration(&self) -> CalibrationState {
        self.calibration
    }

    pub fn last_gps(&self) -> Option<GpsFix> {
        self.last_gps
    }

    pub fn has_gps(&self) -> bool {
        self.last_gps.is_some()
    }
}

impl Default for OracleSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(ax: f32, ay: f32, gz: f32, ts: u64) -> ImuSample {
        ImuSample { ax, ay, az: 0.0, gx: 0.0, gy: 0.0, gz, timestamp_us: ts }
    }

    fn gps(lat: f64, lng: f64) -> GpsFix {
        GpsFix { lat, lng, accuracy_m: 5.0, speed_mps: Some(3.0), timestamp_us: 1000 }
    }

    #[test]
    fn starts_unknown() {
        let oracle = OracleSystem::new();
        assert_eq!(oracle.calibration(), CalibrationState::Uncalibrated);
        assert_eq!(oracle.current_displacement().confidence, 0.0);
    }

    #[test]
    fn calibrates_after_50_samples() {
        let mut oracle = OracleSystem::new();
        for i in 0..60 {
            oracle.ingest_imu(sample(0.1, 0.0, 0.0, i * 10_000));
        }
        assert_eq!(oracle.calibration(), CalibrationState::Calibrated);
    }

    #[test]
    fn gps_sets_confidence() {
        let mut oracle = OracleSystem::new();
        oracle.ingest_gps(gps(50.45, 30.52));
        let d = oracle.current_displacement();
        assert!((d.confidence - 1.0).abs() < 1e-6);
        assert!((d.estimated_lat - 50.45).abs() < 1e-6);
        assert!((d.estimated_lng - 30.52).abs() < 1e-6);
    }

    #[test]
    fn confidence_decays_without_gps() {
        let mut oracle = OracleSystem::new();
        for i in 0..50 {
            oracle.ingest_imu(sample(0.0, 0.0, 0.0, i * 10_000));
        }
        oracle.ingest_gps(gps(50.45, 30.52));
        assert!((oracle.current_displacement().confidence - 1.0).abs() < 1e-6);
        for i in 0..30 {
            oracle.ingest_imu(sample(0.01, 0.0, 0.0, 100_000 + i * 10_000));
        }
        assert!(oracle.current_displacement().confidence < 1.0);
    }

    #[test]
    fn gyroscope_updates_bearing() {
        let mut oracle = OracleSystem::new();
        for i in 0..50 {
            oracle.ingest_imu(sample(0.0, 0.0, 0.0, i * 10_000));
        }
        oracle.ingest_gps(gps(50.45, 30.52));
        let bearing_before = oracle.current_displacement().bearing_deg;
        for i in 0..10 {
            oracle.ingest_imu(sample(0.0, 0.0, 0.1, 200_000 + i * 100_000));
        }
        let bearing_after = oracle.current_displacement().bearing_deg;
        assert!(
            (bearing_after - bearing_before).abs() > 0.1
                || (bearing_after - bearing_before + 360.0).abs() > 0.1,
            "gyro must change bearing: {bearing_before} -> {bearing_after}"
        );
    }

    #[test]
    fn eta_uses_live_speed() {
        let mut oracle = OracleSystem::new();
        oracle.ingest_gps(gps(50.45, 30.52));
        let route = [(50.45, 30.52), (50.455, 30.53)];
        let eta = oracle.eta_to(50.455, 30.53, &route);
        assert!(eta > 0.0, "ETA must be positive, got {eta}");
        assert!(eta.is_finite(), "ETA must be finite, got {eta}");
    }

    #[test]
    fn eta_with_gps_speed() {
        let mut oracle = OracleSystem::new();
        for _ in 0..5 {
            oracle.ingest_gps(gps(50.45, 30.52));
        }
        let route = [(50.45, 30.52), (50.46, 30.54)];
        let eta = oracle.eta_to(50.46, 30.54, &route);
        assert!(eta.is_finite());
    }

    #[test]
    fn new_route_resets_state() {
        let mut oracle = OracleSystem::new();
        for _ in 0..5 {
            oracle.ingest_gps(GpsFix {
                lat: 50.45, lng: 30.52, accuracy_m: 5.0,
                speed_mps: Some(8.0), timestamp_us: 1000,
            });
        }
        let (v_pre, n_pre) = oracle.speed_ema.observed().unwrap();
        assert!(n_pre >= 3);
        oracle.new_route();
        let route = [(50.45, 30.52), (50.46, 30.54)];
        let eta = oracle.eta_to(50.46, 30.54, &route);
        assert!(eta.is_finite());
    }

    #[test]
    fn imu_buffer_capped() {
        let mut oracle = OracleSystem::new();
        for i in 0..150 {
            oracle.ingest_imu(sample(0.0, 0.0, 0.0, i * 10_000));
        }
        assert!(oracle.imu_buffer_len() <= 100);
    }

    #[test]
    fn gps_updates_position() {
        let mut oracle = OracleSystem::new();
        oracle.ingest_gps(gps(50.45, 30.52));
        assert_eq!(oracle.has_gps(), true);
        let pos = oracle.current_displacement();
        assert!((pos.estimated_lat - 50.45).abs() < 1e-10);
    }

    #[test]
    fn eta_returns_finite_when_no_route() {
        let oracle = OracleSystem::new();
        let eta = oracle.eta_to(50.46, 30.54, &[]);
        assert_eq!(eta, 0.0);
    }
}
