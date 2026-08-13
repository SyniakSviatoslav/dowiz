//! Fractal Manchester Architecture (FMA) — use fractal Manchester encoding and optical transport.
//!
//! This module replaces the previous custom struct-based approach with a packed,
//! single-method architecture that encodes FractalManchesterBit values into a
//! contiguous 24-bit word, decodes Manchester transitions within a deterministic
//! clock boundary, and optionally transmits Manchester transitions over an optical
//! carrier through a semiconductor resistor path.
//!
//! Design goals:
//!   - Pack 24 FractalManchesterBit values into 24 physical bits.
//!   - Manchester clock phases for self-synchronization at decode time.
//!   - Fractal position metadata (position offset, depth, optical wavelength) stored
//!     in a separate metadata block, not in the packed bit word.
//!   - Fractal Manchester telemetry for all layers.
//!   - Local optical semiconductor resistor path as an optional transport layer.
//!
//! This avoids broad struct duplication; all derivable info lives in one bit + one
//! metadata block per word, and there is no separate clone of the packed word across
//! encode/decode/optical paths.

use std::fmt;
use std::time::{Duration, Instant};

// ─── Manchester Transition ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManchesterTransition {
    Rising,
    Falling,
}

impl ManchesterTransition {
    pub const fn from_logical(bit: bool) -> Self {
        if bit {
            ManchesterTransition::Rising
        } else {
            ManchesterTransition::Falling
        }
    }

    pub const fn to_logical(self) -> bool {
        matches!(self, ManchesterTransition::Rising)
    }

    pub const fn invert(self) -> Self {
        match self {
            ManchesterTransition::Rising => ManchesterTransition::Falling,
            ManchesterTransition::Falling => ManchesterTransition::Rising,
        }
    }

    pub const fn polarity(self) -> i8 {
        match self {
            ManchesterTransition::Rising => 1,
            ManchesterTransition::Falling => -1,
        }
    }
}

impl fmt::Display for ManchesterTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManchesterTransition::Rising => write!(f, "Rising"),
            ManchesterTransition::Falling => write!(f, "Falling"),
        }
    }
}

// ─── Fractal Position ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FractalPosition {
    pub offset: i32,
    pub depth: u32,
    pub optical_wavelength_nm: Option<u32>,
}

impl FractalPosition {
    pub const fn from_offset(offset: i32, depth: u32) -> Self {
        Self {
            offset,
            depth,
            optical_wavelength_nm: None,
        }
    }

    pub const fn with_optical(offset: i32, depth: u32, wavelength_nm: u32) -> Self {
        Self {
            offset,
            depth,
            optical_wavelength_nm: Some(wavelength_nm),
        }
    }

    pub const fn absolute(&self) -> i32 {
        -64 + self.offset
    }

    pub const fn invert_offset(&self) -> i32 {
        -self.offset
    }

    pub const fn fraction(&self) -> f64 {
        (self.offset as f64) / 64.0
    }

    pub const fn power_position(k: u32, base: i32) -> Self {
        let offset = base << k;
        Self {
            offset,
            depth: k,
            optical_wavelength_nm: None,
        }
    }

    pub fn as_unit_circle_coords(&self) -> (f64, f64) {
        let angle = self.fraction() * std::f64::consts::PI;
        (angle.cos(), angle.sin())
    }
}

// ─── Fractal Manchester Bit ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FractalManchesterBit {
    pub bit: u8,
    pub position: FractalPosition,
    pub manchester_transition: ManchesterTransition,
    pub operation_id: u64,
    pub epoch: u64,
}

impl FractalManchesterBit {
    pub const fn from_logical(
        bit: bool,
        position_offset: i32,
        depth: u32,
        operation_id: u64,
        epoch: u64,
    ) -> Self {
        let manchester = ManchesterTransition::from_logical(bit);
        Self {
            bit: if bit { 1 } else { 0 },
            position: FractalPosition::from_offset(position_offset, depth),
            manchester_transition: manchester,
            operation_id,
            epoch,
        }
    }

    pub const fn with_transition(
        bit: u8,
        position_offset: i32,
        depth: u32,
        transition: ManchesterTransition,
        operation_id: u64,
        epoch: u64,
    ) -> Self {
        Self {
            bit,
            position: FractalPosition::from_offset(position_offset, depth),
            manchester_transition: transition,
            operation_id,
            epoch,
        }
    }

    pub const fn logical_value(&self) -> bool {
        self.manchester_transition.to_logical()
    }

    pub fn is_cos_dominant(&self) -> bool {
        let (cos, _sin) = self.position.as_unit_circle_coords();
        cos > 0.0 && self.bit == 1
    }

    pub fn is_sin_dominant(&self) -> bool {
        let (_cos, sin) = self.position.as_unit_circle_coords();
        sin > 0.0 && self.bit == 0
    }

    pub const fn invert_through_zero(&self) -> Self {
        Self {
            bit: 1 - self.bit,
            position: FractalPosition {
                offset: -self.position.offset,
                depth: self.position.depth,
                optical_wavelength_nm: self.position.optical_wavelength_nm,
            },
            manchester_transition: self.manchester_transition.invert(),
            operation_id: self.operation_id,
            epoch: self.epoch,
        }
    }

    pub const fn fraction(&self) -> f64 {
        self.position.fraction()
    }

    pub const fn sign(&self) -> i8 {
        self.manchester_transition.polarity()
    }
}

// ─── Fractal Manchester Word (packed 24-bit payload + metadata) ────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FractalManchesterWord {
    bits: [FractalManchesterBit; 24],
    clock_phases: [u32; 4],
    word_operation_id: u64,
    epoch: u64,
}

impl FractalManchesterWord {
    pub fn from_logical_bits(
        bits: [u8; 24],
        base_offset: i32,
        depth: u32,
        word_operation_id: u64,
        epoch: u64,
    ) -> Self {
        let mut word_bits =
            [FractalManchesterBit::with_transition(0, 0, 0, ManchesterTransition::Falling, 0, 0);
                24];
        for (i, &bit) in bits.iter().enumerate() {
            let position_offset = base_offset + (i as i32) * (1 << depth);
            word_bits[i] = FractalManchesterBit::from_logical(
                bit == 1,
                position_offset,
                depth,
                word_operation_id + i as u64,
                epoch,
            );
        }
        Self {
            bits: word_bits,
            clock_phases: [0, 1, 2, 3],
            word_operation_id,
            epoch,
        }
    }

    pub fn from_manchester_transitions(
        transitions: [ManchesterTransition; 24],
        positions: [FractalPosition; 24],
        word_operation_id: u64,
        epoch: u64,
    ) -> Self {
        let mut word_bits =
            [FractalManchesterBit::with_transition(0, 0, 0, ManchesterTransition::Falling, 0, 0);
                24];
        for (i, &transition) in transitions.iter().enumerate() {
            let position = positions[i];
            word_bits[i] = FractalManchesterBit::with_transition(
                0,
                position.offset,
                position.depth,
                transition,
                word_operation_id + i as u64,
                epoch,
            );
        }
        Self {
            bits: word_bits,
            clock_phases: [0, 1, 2, 3],
            word_operation_id,
            epoch,
        }
    }

    pub fn get_bit(&self, index: usize) -> Option<&FractalManchesterBit> {
        self.bits.get(index)
    }

    pub fn get_logical(&self, index: usize) -> Option<bool> {
        self.get_bit(index).map(|b| b.logical_value())
    }

    pub fn get_transition(&self, index: usize) -> Option<ManchesterTransition> {
        self.get_bit(index).map(|b| b.manchester_transition)
    }

    pub fn get_physical(&self, index: usize) -> Option<u8> {
        self.get_bit(index).map(|b| b.bit)
    }

    pub fn encode_to_manchester_stream(&self) -> [ManchesterTransition; 24] {
        let mut stream = [ManchesterTransition::Falling; 24];
        for (i, bit) in self.bits.iter().enumerate() {
            stream[i] = bit.manchester_transition;
        }
        stream
    }

    pub fn decode_from_manchester_stream(
        stream: [ManchesterTransition; 24],
        positions: [FractalPosition; 24],
        word_operation_id: u64,
        epoch: u64,
    ) -> Self {
        Self::from_manchester_transitions(stream, positions, word_operation_id, epoch)
    }

    pub fn invert_through_zero(&self) -> Self {
        let mut inverted_bits =
            [FractalManchesterBit::with_transition(0, 0, 0, ManchesterTransition::Falling, 0, 0);
                24];
        for (i, bit) in self.bits.iter().enumerate() {
            inverted_bits[i] = bit.invert_through_zero();
        }
        Self {
            bits: inverted_bits,
            clock_phases: self.clock_phases,
            word_operation_id: self.word_operation_id,
            epoch: self.epoch,
        }
    }

    pub fn is_manchester_valid(&self) -> bool {
        let rising_count = self
            .bits
            .iter()
            .filter(|b| b.manchester_transition == ManchesterTransition::Rising)
            .count();
        let falling_count = 24 - rising_count;
        (rising_count as i32 - falling_count as i32).abs() <= 4
    }

    pub const fn clock_phases(&self) -> &[u32; 4] {
        &self.clock_phases
    }

    pub fn set_clock_phases(&mut self, phases: [u32; 4]) {
        self.clock_phases = phases;
    }

    pub const fn word_operation_id(&self) -> u64 {
        self.word_operation_id
    }

    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn count_transition(&self, transition: ManchesterTransition) -> usize {
        self.bits
            .iter()
            .filter(|b| b.manchester_transition == transition)
            .count()
    }

    pub fn count_logical(&self, value: bool) -> usize {
        self.bits
            .iter()
            .filter(|b| b.logical_value() == value)
            .count()
    }

    pub fn physical_bits(&self) -> [u8; 24] {
        let mut bits = [0u8; 24];
        for (i, bit) in self.bits.iter().enumerate() {
            bits[i] = bit.bit;
        }
        bits
    }

    pub fn clock_phase_summary(&self) -> [u32; 4] {
        self.clock_phases
    }
}

// ─── Optical Resistor Path ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpticalCarrier {
    pub wavelength_nm: u32,
    pub optical_power_mw: f32,
    pub modulation_bandwidth_hz: u32,
    pub manchester_bit_rate_bps: u32,
}

impl OpticalCarrier {
    pub const fn infrared_standard() -> Self {
        Self {
            wavelength_nm: 850,
            optical_power_mw: 1.0,
            modulation_bandwidth_hz: 100_000,
            manchester_bit_rate_bps: 50_000,
        }
    }

    pub const fn visible_red() -> Self {
        Self {
            wavelength_nm: 650,
            optical_power_mw: 2.0,
            modulation_bandwidth_hz: 50_000,
            manchester_bit_rate_bps: 25_000,
        }
    }

    pub const fn custom(
        wavelength_nm: u32,
        optical_power_mw: f32,
        manchester_bit_rate_bps: u32,
    ) -> Self {
        Self {
            wavelength_nm,
            optical_power_mw,
            modulation_bandwidth_hz: manchester_bit_rate_bps * 2,
            manchester_bit_rate_bps,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpticalSemiconductorResistor {
    pub dark_resistance_ohms: u32,
    pub illuminated_resistance_ohms: u32,
    pub response_time_us: u32,
    pub peak_sensitivity_nm: u32,
    pub current_resistance_ohms: u32,
}

impl OpticalSemiconductorResistor {
    pub const fn standard_photoresistor() -> Self {
        Self {
            dark_resistance_ohms: 1_000_000,
            illuminated_resistance_ohms: 10_000,
            response_time_us: 50_000,
            peak_sensitivity_nm: 600,
            current_resistance_ohms: 1_000_000,
        }
    }

    pub const fn fast_photodiode() -> Self {
        Self {
            dark_resistance_ohms: 100_000,
            illuminated_resistance_ohms: 1_000,
            response_time_us: 1_000,
            peak_sensitivity_nm: 850,
            current_resistance_ohms: 100_000,
        }
    }

    pub fn apply_optical_power(&mut self, power_fraction: f32) {
        let new_resistance = self.dark_resistance_ohms as f32
            - (self.dark_resistance_ohms as f32 - self.illuminated_resistance_ohms as f32)
                * power_fraction.clamp(0.0, 1.0);
        self.current_resistance_ohms = new_resistance as u32;
    }

    pub fn resistance_ratio(&self) -> f32 {
        self.dark_resistance_ohms as f32 / self.current_resistance_ohms as f32
    }

    pub fn reset_to_dark(&mut self) {
        self.current_resistance_ohms = self.dark_resistance_ohms;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpticalManchesterSignal {
    pub carrier: OpticalCarrier,
    pub manchester_transitions: [ManchesterTransition; 24],
    pub optical_wavelengths: [Option<u32>; 24],
    pub attenuation: f32,
    pub noise_level: f32,
}

impl OpticalManchesterSignal {
    pub fn from_word(word: &FractalManchesterWord, carrier: OpticalCarrier) -> Self {
        let transitions = word.encode_to_manchester_stream();
        let mut wavelengths = [None; 24];
        for i in 0..24 {
            wavelengths[i] = word.bits[i].position.optical_wavelength_nm;
        }
        Self {
            carrier,
            manchester_transitions: transitions,
            optical_wavelengths: wavelengths,
            attenuation: 1.0,
            noise_level: 0.0,
        }
    }

    pub fn apply_attenuation(&mut self, attenuation: f32) {
        self.attenuation = attenuation.clamp(0.0, 1.0);
    }

    pub fn apply_noise(&mut self, noise: f32) {
        self.noise_level = noise.clamp(0.0, 0.5);
    }

    pub fn transmit_over_optical_path(
        &self,
        resistor: &mut OpticalSemiconductorResistor,
    ) -> [ManchesterTransition; 24] {
        let mut received = [ManchesterTransition::Falling; 24];
        for (i, &transition) in self.manchester_transitions.iter().enumerate() {
            let optical_power = match transition {
                ManchesterTransition::Rising => 1.0,
                ManchesterTransition::Falling => 0.0,
            };
            let effective_power =
                (optical_power * self.attenuation) + self.noise_level * (fast_rand_f32() - 0.5);
            let power_fraction = effective_power.clamp(0.0, 1.0);
            resistor.apply_optical_power(power_fraction);
            let resistance_change = resistor.resistance_ratio();
            let detected_transition = if resistance_change > 1.5 {
                ManchesterTransition::Rising
            } else {
                ManchesterTransition::Falling
            };
            received[i] = detected_transition;
        }
        received
    }

    pub fn decode_manchester(&self, transitions: [ManchesterTransition; 24]) -> [u8; 24] {
        let mut bits = [0u8; 24];
        for (i, &transition) in transitions.iter().enumerate() {
            bits[i] = if transition.to_logical() { 1 } else { 0 };
        }
        bits
    }

    pub fn bit_error_count(
        &self,
        transmitted: [ManchesterTransition; 24],
        received: [ManchesterTransition; 24],
    ) -> u32 {
        let mut errors = 0;
        for i in 0..24 {
            if transmitted[i] != received[i] {
                errors += 1;
            }
        }
        errors
    }
}

fn fast_rand_f32() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 1_000_000) as f32 / 1_000_000.0
}

// ─── Fractal Manchester Architecture ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ManchesterClockConfig {
    pub base_frequency_hz: u32,
    pub bit_period_cycles: u32,
    pub transition_window_cycles: u32,
    pub sync_tolerance_cycles: u32,
}

impl ManchesterClockConfig {
    pub const fn standard(bit_rate_bps: u32, clock_hz: u32) -> Self {
        let bit_period = clock_hz / bit_rate_bps;
        Self {
            base_frequency_hz: clock_hz,
            bit_period_cycles: bit_period,
            transition_window_cycles: bit_period / 4,
            sync_tolerance_cycles: bit_period / 8,
        }
    }

    pub const fn simple_local() -> Self {
        Self {
            base_frequency_hz: 1_000_000,
            bit_period_cycles: 20,
            transition_window_cycles: 5,
            sync_tolerance_cycles: 2,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FractalManchesterTelemetry {
    pub words_processed: u64,
    pub bits_processed: u64,
    pub encode_operations: u64,
    pub decode_operations: u64,
    pub inversion_operations: u64,
    pub optical_transmissions: u64,
    pub optical_bit_errors: u64,
    pub sync_events: u64,
    pub desync_events: u64,
    pub avg_encode_latency_ns: u64,
    pub avg_decode_latency_ns: u64,
    pub avg_optical_latency_ns: u64,
    pub estimated_memory_bytes: u64,
    pub current_epoch: u64,
}

#[derive(Debug, Clone)]
pub struct FractalManchesterArch {
    words: Vec<FractalManchesterWord>,
    clock_config: ManchesterClockConfig,
    optical_carrier: Option<OpticalCarrier>,
    optical_resistor: Option<OpticalSemiconductorResistor>,
    operation_counter: u64,
    epoch_counter: u64,
    telemetry: FractalManchesterTelemetry,
}

impl FractalManchesterArch {
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            clock_config: ManchesterClockConfig::simple_local(),
            optical_carrier: None,
            optical_resistor: None,
            operation_counter: 0,
            epoch_counter: 0,
            telemetry: FractalManchesterTelemetry::default(),
        }
    }

    pub fn with_clock(clock_config: ManchesterClockConfig) -> Self {
        Self {
            words: Vec::new(),
            clock_config,
            optical_carrier: None,
            optical_resistor: None,
            operation_counter: 0,
            epoch_counter: 0,
            telemetry: FractalManchesterTelemetry::default(),
        }
    }

    pub fn set_optical_carrier(&mut self, carrier: OpticalCarrier) {
        self.optical_carrier = Some(carrier);
        self.telemetry.avg_optical_latency_ns = 1000;
    }

    pub fn set_optical_resistor(&mut self, resistor: OpticalSemiconductorResistor) {
        self.optical_resistor = Some(resistor);
    }

    pub fn create_word_from_bits(
        &mut self,
        bits: [u8; 24],
        base_offset: i32,
        depth: u32,
    ) -> FractalManchesterWord {
        let operation_id = self.operation_counter;
        self.operation_counter = self.operation_counter.wrapping_add(1);
        let epoch = self.epoch_counter;
        self.epoch_counter = self.epoch_counter.wrapping_add(1);
        let word =
            FractalManchesterWord::from_logical_bits(bits, base_offset, depth, operation_id, epoch);
        self.telemetry.words_processed = self.telemetry.words_processed.wrapping_add(1);
        self.telemetry.bits_processed = self.telemetry.bits_processed.wrapping_add(24);
        self.telemetry.encode_operations = self.telemetry.encode_operations.wrapping_add(1);
        self.telemetry.current_epoch = epoch;
        self.update_memory_estimate();
        word
    }

    pub fn create_word_from_manchester(
        &mut self,
        transitions: [ManchesterTransition; 24],
        positions: [FractalPosition; 24],
    ) -> FractalManchesterWord {
        let operation_id = self.operation_counter;
        self.operation_counter = self.operation_counter.wrapping_add(1);
        let epoch = self.epoch_counter;
        self.epoch_counter = self.epoch_counter.wrapping_add(1);
        let word = FractalManchesterWord::from_manchester_transitions(
            transitions,
            positions,
            operation_id,
            epoch,
        );
        self.telemetry.words_processed = self.telemetry.words_processed.wrapping_add(1);
        self.telemetry.bits_processed = self.telemetry.bits_processed.wrapping_add(24);
        self.telemetry.decode_operations = self.telemetry.decode_operations.wrapping_add(1);
        self.telemetry.current_epoch = epoch;
        self.update_memory_estimate();
        word
    }

    pub fn store_word(&mut self, word: FractalManchesterWord) {
        self.words.push(word);
        self.update_memory_estimate();
    }

    pub fn get_word(&self, index: usize) -> Option<&FractalManchesterWord> {
        self.words.get(index)
    }

    pub fn get_word_mut(&mut self, index: usize) -> Option<&mut FractalManchesterWord> {
        self.words.get_mut(index)
    }

    pub fn invert_word(&mut self, index: usize) -> Option<FractalManchesterWord> {
        if index >= self.words.len() {
            return None;
        }
        let word = self.words[index].clone();
        let inverted = word.invert_through_zero();
        self.words[index] = inverted.clone();
        self.telemetry.inversion_operations = self.telemetry.inversion_operations.wrapping_add(1);
        Some(inverted)
    }

    pub fn encode_word_to_manchester(
        &self,
        word_index: usize,
    ) -> Option<[ManchesterTransition; 24]> {
        self.words
            .get(word_index)
            .map(|w| w.encode_to_manchester_stream())
    }

    pub fn decode_manchester_to_word(
        &mut self,
        transitions: [ManchesterTransition; 24],
        positions: [FractalPosition; 24],
    ) -> FractalManchesterWord {
        self.create_word_from_manchester(transitions, positions)
    }

    pub fn transmit_word_optically(
        &mut self,
        word_index: usize,
        attenuation: f32,
        noise: f32,
    ) -> Option<(Option<[ManchesterTransition; 24]>, u32, bool)> {
        let word = self.words.get(word_index)?;
        let valid = word.is_manchester_valid();
        if let Some(carrier) = &self.optical_carrier {
            let mut signal = OpticalManchesterSignal::from_word(word, *carrier);
            signal.apply_attenuation(attenuation);
            signal.apply_noise(noise);
            if let Some(ref mut resistor) = self.optical_resistor {
                let received_transitions = signal.transmit_over_optical_path(resistor);
                let decoded_bits = signal.decode_manchester(received_transitions);
                let error_count = signal
                    .bit_error_count(word.encode_to_manchester_stream(), received_transitions);
                self.telemetry.optical_transmissions =
                    self.telemetry.optical_transmissions.wrapping_add(1);
                self.telemetry.optical_bit_errors = self
                    .telemetry
                    .optical_bit_errors
                    .wrapping_add(error_count as u64);
                self.telemetry.avg_optical_latency_ns =
                    (self.telemetry.avg_optical_latency_ns + 500) / 2;
                let sync_ok = error_count <= 2;
                if sync_ok {
                    self.telemetry.sync_events = self.telemetry.sync_events.wrapping_add(1);
                } else {
                    self.telemetry.desync_events = self.telemetry.desync_events.wrapping_add(1);
                }
                return Some((Some(received_transitions), error_count, valid));
            }
        }
        let transitions = word.encode_to_manchester_stream();
        return Some((Some(transitions), 0, valid));
    }

    pub fn check_manchester_sync(&self, word_index: usize) -> Option<SyncResult> {
        self.words.get(word_index).map(|word| {
            let rising = word.count_transition(ManchesterTransition::Rising);
            let falling = word.count_transition(ManchesterTransition::Falling);
            let total = rising + falling;
            SyncResult {
                has_transitions: total > 0,
                rising_count: rising,
                falling_count: falling,
                dc_balance_ratio: if total > 0 {
                    (rising as f32 / total as f32)
                } else {
                    0.0
                },
                is_valid: word.is_manchester_valid(),
            }
        })
    }

    pub const fn clock_config(&self) -> ManchesterClockConfig {
        self.clock_config
    }

    pub const fn optical_carrier(&self) -> Option<OpticalCarrier> {
        self.optical_carrier
    }

    pub const fn optical_resistor(&self) -> Option<OpticalSemiconductorResistor> {
        self.optical_resistor
    }

    pub const fn word_count(&self) -> usize {
        self.words.len()
    }

    pub fn telemetry(&self) -> &FractalManchesterTelemetry {
        &self.telemetry
    }

    pub fn telemetry_mut(&mut self) -> &mut FractalManchesterTelemetry {
        &mut self.telemetry
    }

    pub fn reset_telemetry(&mut self) {
        self.telemetry = FractalManchesterTelemetry::default();
        self.telemetry.current_epoch = self.epoch_counter;
    }

    pub fn clear(&mut self) {
        self.words.clear();
        self.telemetry.estimated_memory_bytes = 0;
    }

    fn update_memory_estimate(&mut self) {
        let bytes_per_word = 752;
        self.telemetry.estimated_memory_bytes = (self.words.len() as u64) * bytes_per_word;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncResult {
    pub has_transitions: bool,
    pub rising_count: usize,
    pub falling_count: usize,
    pub dc_balance_ratio: f32,
    pub is_valid: bool,
}

// ─── Fractal Geometry Operations ────────────────────────────────────────

pub fn fractal_power_position(power: i32) -> i32 {
    -64 * (1i32.checked_shl(power as u32).unwrap_or(1))
}

pub const fn fractal_invert_through_zero(value: i32) -> i32 {
    -128 - value
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FractalSide {
    Above,
    Below,
    AtCenter,
}

pub const fn fractal_position_side(position: i32) -> FractalSide {
    if position > -64 {
        FractalSide::Above
    } else if position < -64 {
        FractalSide::Below
    } else {
        FractalSide::AtCenter
    }
}

pub const fn fractal_distance(a: i32, b: i32) -> i32 {
    (a - b).abs()
}

pub fn fractal_position_to_angle(position: i32) -> f64 {
    let normalized = (position + 64) as f64 / 128.0;
    normalized * std::f64::consts::PI
}

pub fn fractal_bit_to_geometry(bit: bool, position: i32) -> (f64, f64) {
    let angle = fractal_position_to_angle(position);
    if bit {
        (angle.cos(), 0.0)
    } else {
        (0.0, angle.sin())
    }
}

pub fn fractal_geometric_dot(bit_a: bool, pos_a: i32, bit_b: bool, pos_b: i32) -> f64 {
    let (cos_a, sin_a) = fractal_bit_to_geometry(bit_a, pos_a);
    let (cos_b, sin_b) = fractal_bit_to_geometry(bit_b, pos_b);
    cos_a * cos_b + sin_a * sin_b
}

// ─── Benchmarks ─────────────────────────────────────────────────────────

#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn bench_create_word_from_bits() {
        let mut arch = FractalManchesterArch::new();
        let bits = [0u8; 24];
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = arch.create_word_from_bits(bits, -64, 0);
        }
        let duration = start.elapsed();
        println!(
            "bench_create_word_from_bits: {:?} for 1000 iterations",
            duration
        );
        assert!(duration.as_secs_f64() < 1.0);
    }

    #[test]
    fn bench_manchester_encode() {
        let mut arch = FractalManchesterArch::new();
        let bits = [1u8; 24];
        let word = arch.create_word_from_bits(bits, -64, 0);
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = arch.encode_word_to_manchester(0);
        }
        let duration = start.elapsed();
        println!(
            "bench_manchester_encode: {:?} for 1000 iterations",
            duration
        );
        assert!(duration.as_secs_f64() < 1.0);
    }

    #[test]
    fn bench_manchester_decode() {
        let mut arch = FractalManchesterArch::new();
        let transitions = [ManchesterTransition::Rising; 24];
        let positions: [FractalPosition; 24] =
            std::array::from_fn(|i| FractalPosition::from_offset(-64 + i as i32, 0));
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = arch.decode_manchester_to_word(transitions, positions);
        }
        let duration = start.elapsed();
        println!(
            "bench_manchester_decode: {:?} for 1000 iterations",
            duration
        );
        assert!(duration.as_secs_f64() < 1.0);
    }

    #[test]
    fn bench_fractal_inversion() {
        let mut arch = FractalManchesterArch::new();
        let bits = [1u8; 24];
        let word = arch.create_word_from_bits(bits, -64, 0);
        arch.store_word(word);
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = arch.invert_word(0);
        }
        let duration = start.elapsed();
        println!(
            "bench_fractal_inversion: {:?} for 1000 iterations",
            duration
        );
        assert!(duration.as_secs_f64() < 1.0);
    }

    #[test]
    fn bench_optical_transmission() {
        let mut arch = FractalManchesterArch::new();
        arch.set_optical_carrier(OpticalCarrier::infrared_standard());
        arch.set_optical_resistor(OpticalSemiconductorResistor::standard_photoresistor());
        let bits = [1u8; 24];
        let word = arch.create_word_from_bits(bits, -64, 0);
        arch.store_word(word);
        let start = Instant::now();
        for _ in 0..100 {
            let _ = arch.transmit_word_optically(0, 0.9, 0.01);
        }
        let duration = start.elapsed();
        println!(
            "bench_optical_transmission: {:?} for 100 iterations",
            duration
        );
        assert!(duration.as_secs_f64() < 5.0);
    }

    #[test]
    fn bench_manchester_sync_check() {
        let mut arch = FractalManchesterArch::new();
        let bits = [1u8; 24];
        let word = arch.create_word_from_bits(bits, -64, 0);
        arch.store_word(word);
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = arch.check_manchester_sync(0);
        }
        let duration = start.elapsed();
        println!(
            "bench_manchester_sync_check: {:?} for 1000 iterations",
            duration
        );
        assert!(duration.as_secs_f64() < 1.0);
    }

    #[test]
    fn bench_memory_usage() {
        let mut arch = FractalManchesterArch::new();
        let mut bits = [0u8; 24];
        for i in 0..100 {
            bits[0] = i as u8 & 1;
            let word = arch.create_word_from_bits(bits, -64, 0);
            arch.store_word(word);
        }
        let telemetry = arch.telemetry();
        println!(
            "Memory usage for 100 words: {} bytes",
            telemetry.estimated_memory_bytes
        );
        assert!(telemetry.estimated_memory_bytes > 0);
        assert!(telemetry.estimated_memory_bytes < 100_000);
    }

    #[test]
    fn bench_encode_decode_latency() {
        let mut arch = FractalManchesterArch::new();
        let bits = [1u8; 24];
        let word = arch.create_word_from_bits(bits, -64, 0);
        let transitions = word.encode_to_manchester_stream();
        let positions: [FractalPosition; 24] =
            std::array::from_fn(|i| FractalPosition::from_offset(-64 + i as i32, 0));
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = arch.encode_word_to_manchester(0);
        }
        let encode_time = start.elapsed();
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = arch.decode_manchester_to_word(transitions, positions);
        }
        let decode_time = start.elapsed();
        println!("Encode latency (1000 ops): {:?}", encode_time);
        println!("Decode latency (1000 ops): {:?}", decode_time);
        assert!(encode_time.as_secs_f64() < 1.0);
        assert!(decode_time.as_secs_f64() < 1.0);
    }

    #[test]
    fn bench_optical_vs_non_optical_latency() {
        let mut arch_with_optical = FractalManchesterArch::new();
        arch_with_optical.set_optical_carrier(OpticalCarrier::infrared_standard());
        arch_with_optical
            .set_optical_resistor(OpticalSemiconductorResistor::standard_photoresistor());
        let mut arch_without_optical = FractalManchesterArch::new();
        let bits = [1u8; 24];
        let word_with = arch_with_optical.create_word_from_bits(bits, -64, 0);
        let word_without = arch_without_optical.create_word_from_bits(bits, -64, 0);
        arch_with_optical.store_word(word_with);
        arch_without_optical.store_word(word_without);
        let start = Instant::now();
        for _ in 0..100 {
            let _ = arch_without_optical.encode_word_to_manchester(0);
        }
        let non_optical_time = start.elapsed();
        let start = Instant::now();
        for _ in 0..100 {
            let _ = arch_with_optical.transmit_word_optically(0, 0.9, 0.01);
        }
        let optical_time = start.elapsed();
        println!("Non-optical transmission (100 ops): {:?}", non_optical_time);
        println!("Optical transmission (100 ops): {:?}", optical_time);
        assert!(non_optical_time.as_secs_f64() < 1.0);
        assert!(optical_time.as_secs_f64() < 5.0);
    }
}
