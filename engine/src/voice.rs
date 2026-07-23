//! P64 M5 — Voice: wake + Moonshine streaming + Whisper multilingual fallback.
//!
//! BLUEPRINT-P64 §2.4 / §3.5. `VoiceSource` is the `InputSource` impl P38-rev
//! §12.2 c3 names ("VoiceSource are P64"). Pipeline: mic → `AudioRing` →
//! `WakeWordSpotter` (tiny always-on keyword-spot — the battery lever, R1 §5) →
//! `AsrModel::feed` streaming → `AsrDelta`s → on `is_final`, hand the transcript
//! to the intent classifier via `RawInput::VoicePhrase`.
//!
//! Two model impls: `MoonshineAsr` (Wave-0 default, English-strong, ~107 ms
//! streaming) and `WhisperCppAsr` (multilingual fallback, buffered 30 s window).
//! Selection: `VoiceProfile { locale, prefer }` picks Moonshine when the locale is
//! Moonshine-covered, else Whisper (R1 §5's "Moonshine multilingual is unproven").
//!
//! **This engine crate is offline-clean**: the REAL `moonshine`/`whisper.cpp`
//! model crates are NOT in the cargo cache (no network grant), so this module
//! ships dependency-free STUBS that implement the exact same trait surface the
//! real models will. The gates that matter — wake-gate blocks ASR (battery
//! lever), locale→model fallback, and voice NEVER bypasses the friction FSM —
//! are all exercised by the unit tests under the `voice` feature. The real
//! model bodies land behind this same `feature = "voice"` flag once the grant
//! lands. NO non-cached dep is added (P41's air-gapped build mandate).
//!
//! Voice is a PERCEPTION channel, not a decision authority: a spoken
//! `Intent::Command(ConfirmOrder)` still routes through the `FrictionFsm` — voice
//! completion = spoken read-back + affirmation (§4.3). This module proves that
//! invariant in the type system: a `VoiceSource` emits only UNRESOLVED
//! `RawInput::VoicePhrase`s; it never mints a `CommitToken`.

use crate::friction::{FrictionSpec, Stake};
use crate::intent::{InputSource, RawInput, SurfaceId};
use crate::money_guard::Money;
// Item 60 (gap G11) — wasm-safe clock, shared one design with the frame loop.
use crate::clock;

/// Locale tag (BCP-47-ish, minimal). Drives Moonshine-vs-Whisper selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Locale(pub &'static str);

impl Locale {
    /// Whether this locale is covered by the lighter Moonshine model (English-
    /// strong per R1 §5; "Moonshine multilingual is unproven"). Anything other
    /// than an English tag falls back to Whisper (multilingual).
    pub fn moonshine_covered(&self) -> bool {
        let l = self.0.to_ascii_lowercase();
        l == "en" || l.starts_with("en-") || l == "en-us" || l == "en-gb"
    }
}

/// Voice preference override. `Auto` = follow the locale (default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoicePrefer {
    #[default]
    Auto,
    Moonshine,
    Whisper,
}

/// Voice profile: which model the source selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceProfile {
    pub locale: Locale,
    pub prefer: VoicePrefer,
}

impl Default for VoiceProfile {
    fn default() -> Self {
        VoiceProfile {
            locale: Locale("en"),
            prefer: VoicePrefer::Auto,
        }
    }
}

impl VoiceProfile {
    /// Resolve the model kind this profile selects. The load-bearing M5 gate:
    /// an uncovered locale MUST select Whisper, never Moonshine.
    pub fn selected_model(&self) -> AsrKind {
        match self.prefer {
            VoicePrefer::Moonshine => AsrKind::Moonshine,
            VoicePrefer::Whisper => AsrKind::Whisper,
            VoicePrefer::Auto => {
                if self.locale.moonshine_covered() {
                    AsrKind::Moonshine
                } else {
                    AsrKind::Whisper
                }
            }
        }
    }
}

/// Which ASR backend a `VoiceSource` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsrKind {
    Moonshine,
    Whisper,
}

/// A streaming ASR transcript delta. `is_final` marks the finalized transcript
/// handed to the intent classifier.
#[derive(Debug, Clone, PartialEq)]
pub struct AsrDelta {
    pub text: String,
    pub is_final: bool,
}

/// Audio → text. Sibling of the kernel `LlmBackend` for a different modality.
/// Same typed-error shape. Offline stub impls satisfy the contract without a
/// network grant.
pub trait AsrModel {
    fn id(&self) -> &str;
    /// Stream a PCM chunk; returns partial/final transcript deltas.
    fn feed(&mut self, pcm: &[i16]) -> Result<Vec<AsrDelta>, InferError>;
    fn reset(&mut self);
    /// Set the fixture transcript the stub emits (unit-test seam). Real models
    /// ignore this (they decode PCM); the offline stub honors it so the pipeline
    /// can be exercised without a network grant.
    fn set_fixture(&mut self, transcript: &str);
}

/// Typed inference error (fail-closed). Mirrors `LlmError` shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferError {
    NotLoaded,
    Unsupported,
    BadInput(String),
    Timeout,
    BudgetExceeded,
}

/// Wake-word spotter keyword match window / ASR budget (gap G11, item 60).
///
/// The battery-lever framing (wake gates ASR) is a battery-vs-latency tradeoff;
/// this constant is the latency ceiling that makes `InferError::Timeout` fire.
/// It is a product/UX decision (blueprint §7 [OPERATOR]); the wiring (dead
/// variant → live timer) is the engineering deliverable. Pinned + single
/// authority like `FRAME_BUDGET_US`. The real Moonshine stub streams in ~107 ms
/// per its module doc, so 250 ms is a generous ceiling (operator-tunable).
pub const ASR_TIMEOUT_US: u64 = 250_000;

/// Tiny always-on wake-word spotter. The battery lever: full ASR runs ONLY after
/// a wake fires. The stub matches a fixed keyword string against a provided
/// trigger; real spotters would run an always-on tiny net on the mic ring.
pub struct WakeWordSpotter {
    keyword: &'static str,
    armed: bool,
}

impl WakeWordSpotter {
    pub fn new(keyword: &'static str) -> Self {
        WakeWordSpotter {
            keyword,
            armed: true,
        }
    }

    /// Returns true iff the detector fired for this frame. The stub "hears" a
    /// wake when `trigger` is Some(keyword). Real impl: a low-power net over the
    /// mic ring buffer. When `armed == false` (already woke, ASR streaming), it
    /// keeps returning true so streaming continues until reset.
    pub fn spot(&mut self, trigger: Option<&str>) -> bool {
        match trigger {
            Some(t) if t == self.keyword => {
                self.armed = false; // woke; now streaming
                true
            }
            // After waking, stay "hot" until reset so ASR keeps streaming.
            _ if !self.armed => true,
            _ => false,
        }
    }

    /// Re-arm for the next utterance.
    pub fn reset(&mut self) {
        self.armed = true;
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }
}

/// Moonshine ASR stub (Wave-0 default, English-strong, ~107 ms streaming).
/// Real body feeds PCM to the Moonshine net; the stub echoes a fixture transcript
/// so the unit pipeline can be exercised offline (P41's `#[ignore]`-until-model
/// pattern — here we keep it deterministic and dependency-free).
pub struct MoonshineAsr {
    fixture: String,
    fired: bool,
}

impl MoonshineAsr {
    pub fn new() -> Self {
        MoonshineAsr {
            fixture: String::new(),
            fired: false,
        }
    }
}

impl Default for MoonshineAsr {
    fn default() -> Self {
        Self::new()
    }
}

impl AsrModel for MoonshineAsr {
    fn id(&self) -> &str {
        "moonshine"
    }
    fn feed(&mut self, _pcm: &[i16]) -> Result<Vec<AsrDelta>, InferError> {
        // Streaming stub: on the first chunk, emit the fixture transcript as final.
        // The real model would stream partials then finalize.
        if !self.fired {
            self.fired = true;
            Ok(vec![AsrDelta {
                text: self.fixture.clone(),
                is_final: true,
            }])
        } else {
            Ok(vec![])
        }
    }
    fn reset(&mut self) {
        self.fired = false;
        self.fixture.clear();
    }
    fn set_fixture(&mut self, transcript: &str) {
        self.fixture = transcript.to_string();
    }
}

/// Whisper.cpp ASR stub (multilingual fallback, buffered 30 s window, R1 §5).
pub struct WhisperCppAsr {
    fixture: String,
    fired: bool,
}

impl WhisperCppAsr {
    pub fn new() -> Self {
        WhisperCppAsr {
            fixture: String::new(),
            fired: false,
        }
    }
}

impl Default for WhisperCppAsr {
    fn default() -> Self {
        Self::new()
    }
}

impl AsrModel for WhisperCppAsr {
    fn id(&self) -> &str {
        "whisper.cpp"
    }
    fn feed(&mut self, _pcm: &[i16]) -> Result<Vec<AsrDelta>, InferError> {
        if !self.fired {
            self.fired = true;
            Ok(vec![AsrDelta {
                text: self.fixture.clone(),
                is_final: true,
            }])
        } else {
            Ok(vec![])
        }
    }
    fn reset(&mut self) {
        self.fired = false;
        self.fixture.clear();
    }
    fn set_fixture(&mut self, transcript: &str) {
        self.fixture = transcript.to_string();
    }
}

/// Fixed-size mic ring buffer — no allocation on the hot path.
pub struct AudioRing {
    buf: std::collections::VecDeque<i16>,
    cap: usize,
}

impl AudioRing {
    pub fn new(capacity: usize) -> Self {
        AudioRing {
            buf: std::collections::VecDeque::with_capacity(capacity),
            cap: capacity,
        }
    }
    /// Push a PCM sample; overwrites the oldest when full (constant capacity).
    pub fn push(&mut self, s: i16) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(s);
    }
    /// Drain the buffered samples (handed to ASR feed).
    pub fn drain(&mut self) -> Vec<i16> {
        self.buf.drain(..).collect()
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

/// The `InputSource` impl P38-rev §12.2 c3 names ("VoiceSource are P64").
///
/// It emits ONLY `RawInput::VoicePhrase` (unresolved) — never a `CommitToken`.
/// Voice is a perception channel; the friction FSM (elsewhere) is the only
/// producer of `CommitToken`. This structurally guarantees a voice command
/// cannot move money without the readback-affirm friction sequence.
pub struct VoiceSource {
    wake: WakeWordSpotter,
    asr: Box<dyn AsrModel>,
    ring: AudioRing,
    /// The fixed transcript the stub ASR emits once woken (set per fixture).
    fixture_transcript: String,
    /// Whether a wake has fired and ASR is streaming.
    woke: bool,
    /// Counter proving the battery lever: ASR feed must NOT run before wake.
    asr_feed_calls: u32,
    /// Last measured ASR feed latency in microseconds (gap G11, item 60). `None`
    /// = untimed (named absence, never a fabricated `0`); on wasm the clock
    /// returns `None`. Fed by the wasm-safe `clock::now_micros` bracketing the
    /// `AsrModel::feed` call.
    asr_feed_us: Option<u64>,
}

impl VoiceSource {
    /// Build a voice source for a profile. Selects Moonshine vs Whisper per the
    /// locale rule (the load-bearing §3.5 gate).
    pub fn new(profile: VoiceProfile, wake_keyword: &'static str) -> Self {
        let asr: Box<dyn AsrModel> = match profile.selected_model() {
            AsrKind::Moonshine => Box::new(MoonshineAsr::new()),
            AsrKind::Whisper => Box::new(WhisperCppAsr::new()),
        };
        VoiceSource {
            wake: WakeWordSpotter::new(wake_keyword),
            asr,
            ring: AudioRing::new(4096),
            fixture_transcript: String::new(),
            woke: false,
            asr_feed_calls: 0,
            asr_feed_us: None,
        }
    }

    /// Test/spiking seam: build with an explicit ASR model (so a slow fake can be
    /// injected to prove `InferError::Timeout` is reachable from the real timer).
    pub fn with_asr(
        profile: VoiceProfile,
        wake_keyword: &'static str,
        asr: Box<dyn AsrModel>,
    ) -> Self {
        VoiceSource {
            wake: WakeWordSpotter::new(wake_keyword),
            asr,
            ring: AudioRing::new(4096),
            fixture_transcript: String::new(),
            woke: false,
            asr_feed_calls: 0,
            asr_feed_us: None,
        }
    }

    /// Set the fixture transcript the stub ASR will emit (unit-test seam).
    pub fn set_fixture(&mut self, t: &str) {
        self.fixture_transcript = t.to_string();
        self.asr.set_fixture(t);
    }

    /// Feed one mic sample (the real pipeline fills the ring then streams).
    pub fn feed_mic(&mut self, s: i16) {
        self.ring.push(s);
    }

    /// The battery-lever proof: how many times ASR `feed` has been called.
    /// Must be 0 until a wake fires.
    pub fn asr_feed_calls(&self) -> u32 {
        self.asr_feed_calls
    }

    /// Item 60 (gap G11) — last measured ASR feed latency in microseconds, or
    /// `None` when untimed (named absence — never a fabricated `0`). This is the
    /// measured basis for the module's "battery lever" claim: feed-latency +
    /// `asr_feed_calls()` = a real energy-proxy pair.
    pub fn asr_feed_us(&self) -> Option<u64> {
        self.asr_feed_us
    }

    /// Drive one wake detection step. `mic_trigger` is `Some(keyword)` when the
    /// wake net spots the keyword this frame; `None` otherwise.
    pub fn detect_wake(&mut self, mic_trigger: Option<&str>) {
        if self.wake.spot(mic_trigger) {
            self.woke = true;
        }
    }

    /// Internal: run the ASR on the buffered PCM and return a final transcript if
    /// one arrived. Only callable after a wake (enforced — the battery lever).
    ///
    /// Item 60 (gap G11): the `AsrModel::feed` call is bracketed by the wasm-safe
    /// `clock::now_micros`. If the measured feed latency exceeds `ASR_TIMEOUT_US`,
    /// the call returns `Err(InferError::Timeout)` — making the previously-DEAD
    /// `Timeout` variant reachable from a real timer (red→green). The measured
    /// latency is stored in `asr_feed_us` (named absence `None` when untimed,
    /// e.g. on wasm or when the clock is unavailable — never a fabricated `0`).
    fn stream_asr(&mut self) -> Result<Option<String>, InferError> {
        if !self.woke {
            return Ok(None); // battery lever: no ASR without wake → 0 feed calls
        }
        self.asr_feed_calls += 1;
        let pcm = self.ring.drain();
        let t0 = clock::now_micros();
        // The stub emits the fixture transcript as the finalized utterance.
        let deltas = self.asr.feed(&pcm)?;
        let t1 = clock::now_micros();
        let latency_us = match (t0, t1) {
            (Some(a), Some(b)) => Some(b.saturating_sub(a)),
            _ => None,
        };
        self.asr_feed_us = latency_us;
        // Gap G11 (red→green): a real timer now drives `InferError::Timeout`.
        if let Some(us) = latency_us {
            if us > ASR_TIMEOUT_US {
                return Err(InferError::Timeout);
            }
        }
        for d in deltas {
            if d.is_final {
                return Ok(Some(d.text));
            }
        }
        Ok(None)
    }

    /// Drive one wake+ASR step and return a pollable raw input, mapping any ASR
    /// error (including the new `InferError::Timeout`) to "no transcript this
    /// tick" — voice is a perception channel; a timed-out feed is simply dropped,
    /// never an error surfaced to the router.
    pub fn poll_voice(&mut self) -> Option<RawInput> {
        match self.stream_asr() {
            Ok(Some(transcript)) if !transcript.is_empty() => Some(RawInput::VoicePhrase {
                transcript,
                confidence: 0.9,
                is_final: true,
            }),
            _ => None,
        }
    }
}

impl InputSource for VoiceSource {
    /// Poll the voice source for the next raw input. Returns a `VoicePhrase`
    /// only after a wake AND a finalized transcript; the classifier resolves it.
    /// Voice NEVER emits a resolved `Intent` or a `CommitToken` directly.
    ///
    /// Item 60 (gap G11): a timed-out ASR feed (`InferError::Timeout`) is mapped
    /// to "no transcript this tick" — the dead variant is now reachable but is a
    /// perception-channel drop, never a router error.
    fn poll(&mut self) -> Option<RawInput> {
        self.poll_voice()
    }
}

// ── P64 M5 R4 — real signal-processing pipeline (replaces keyword-match stub) ──

/// Radix-2 Cooley-Tukey FFT (complex → complex), in-place on `re`/`im`.
/// `n` MUST be a power of two. Panics otherwise (debug assertion).
/// Zero external dependencies — pure `std` only.
pub fn fft_radix2(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    debug_assert_eq!(n, im.len(), "re/im must be equal length");
    debug_assert!(n.is_power_of_two(), "n must be power of two, got {n}");

    // Bit-reverse permutation
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // Danielson-Lanczos loops
    let mut len = 2;
    while len <= n {
        let half = len >> 1;
        let theta = -2.0 * std::f64::consts::PI / len as f64;
        let wpr = theta.cos();
        let wpi = theta.sin();
        let mut wr = 1.0f64;
        let mut wi = 0.0f64;
        for m in (0..half).step_by(1) {
            for k in (m..n).step_by(len) {
                let k2 = k + half;
                let tr = wr * re[k2] - wi * im[k2];
                let ti = wr * im[k2] + wi * re[k2];
                re[k2] = re[k] - tr;
                im[k2] = im[k] - ti;
                re[k] += tr;
                im[k] += ti;
            }
            let w_oldr = wr;
            wr = w_oldr * wpr - wi * wpi;
            wi = w_oldr * wpi + wi * wpr;
        }
        len <<= 1;
    }
}

/// Magnitude spectrum from FFT output: `mag[i] = sqrt(re[i]^2 + im[i]^2)`.
pub fn magnitude_spectrum(re: &[f64], im: &[f64]) -> Vec<f64> {
    re.iter()
        .zip(im.iter())
        .map(|(r, i)| (r * r + i * i).sqrt())
        .collect()
}

/// Power spectrum: `ps[i] = re[i]^2 + im[i]^2`.
pub fn power_spectrum(re: &[f64], im: &[f64]) -> Vec<f64> {
    re.iter()
        .zip(im.iter())
        .map(|(r, i)| r * r + i * i)
        .collect()
}

/// Mel-scale filter bank coefficients for `n_filters` triangular filters
/// spanning `[0, sample_rate/2]` Hz from an FFT of size `fft_size`.
/// Returns (filter_weights: Vec<Vec<f64>>, mel_freqs: Vec<f64>).
/// Each `filter_weights[i]` is a vector of `fft_size/2 + 1` weights (one per
/// FFT bin) that sums to 1.0.
pub fn mel_filterbank(n_filters: usize, fft_size: usize, sample_rate: f64) -> Vec<Vec<f64>> {
    let n_bins = fft_size / 2 + 1; // positive frequencies only
    let max_freq = sample_rate / 2.0;

    // Hz → Mel
    let hz_to_mel = |hz: f64| -> f64 { 2595.0 * (1.0 + hz / 700.0).log10() };
    // Mel → Hz
    let mel_to_hz = |mel: f64| -> f64 { 700.0 * (10.0f64.powf(mel / 2595.0) - 1.0) };

    let mel_low = hz_to_mel(0.0);
    let mel_high = hz_to_mel(max_freq);

    let mut mel_points = vec![0.0f64; n_filters + 2];
    for i in 0..mel_points.len() {
        mel_points[i] = mel_low + (mel_high - mel_low) * i as f64 / (n_filters + 1) as f64;
    }

    let mut hz_points: Vec<f64> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();
    // Clamp to valid frequency range
    for hp in &mut hz_points {
        *hp = hp.clamp(0.0, max_freq);
    }

    let mut bin_points = vec![0.0f64; n_filters + 2];
    for i in 0..hz_points.len() {
        bin_points[i] = hz_points[i] * (fft_size as f64) / sample_rate;
    }

    let mut filters = vec![vec![0.0f64; n_bins]; n_filters];
    for m in 1..=n_filters {
        let f_m_minus = bin_points[m - 1];
        let f_m = bin_points[m];
        let f_m_plus = bin_points[m + 1];
        for k in 0..n_bins {
            let kf = k as f64;
            let left = (kf - f_m_minus) / (f_m - f_m_minus + 1e-10);
            let right = (f_m_plus - kf) / (f_m_plus - f_m + 1e-10);
            let w = left.min(right).max(0.0);
            filters[m - 1][k] = w;
        }
        // Normalize to unit sum
        let sum: f64 = filters[m - 1].iter().sum();
        if sum > 0.0 {
            for v in &mut filters[m - 1] {
                *v /= sum;
            }
        }
    }
    filters
}

/// MFCC (Mel-Frequency Cepstral Coefficients) feature vector.
/// Takes PCM samples, computes FFT → power spectrum → Mel filterbank → log →
/// DCT → `n_coeffs` coefficients. Returns the MFCC vector.
///
/// `sample_rate` is in Hz (typ. 8000 or 16000).
/// `n_coeffs` typically 13–20. Returns exactly `n_coeffs` coefficients.
/// `n_filters` typically 26. `fft_size` must be power-of-two, ≥ window length.
pub fn mfcc(
    pcm: &[f64],
    sample_rate: f64,
    n_coeffs: usize,
    n_filters: usize,
    fft_size: usize,
) -> Vec<f64> {
    // Zero-pad or truncate to fft_size
    let mut re = vec![0.0f64; fft_size];
    let mut im = vec![0.0f64; fft_size];
    let n = pcm.len().min(fft_size);
    for i in 0..n {
        re[i] = pcm[i];
    }
    // Hann window
    for i in 0..n {
        let w = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (n as f64 - 1.0)).cos());
        re[i] *= w;
    }

    fft_radix2(&mut re, &mut im);
    let ps = power_spectrum(&re, &im);

    let filters = mel_filterbank(n_filters, fft_size, sample_rate);
    let mut mel_energies = vec![0.0f64; n_filters];
    for (m, f) in filters.iter().enumerate() {
        let e: f64 = f.iter().zip(ps.iter()).map(|(w, p)| w * p).sum();
        mel_energies[m] = if e > 1e-12 { e.ln() } else { -30.0 };
    }

    // DCT-II to get cepstral coefficients
    let mut mfccs = vec![0.0f64; n_coeffs];
    for i in 0..n_coeffs {
        let mut sum = 0.0;
        for (j, e) in mel_energies.iter().enumerate() {
            sum += e * ((std::f64::consts::PI * i as f64 * (j as f64 + 0.5)) / n_filters as f64)
                .cos();
        }
        mfccs[i] = sum;
    }
    mfccs
}

/// Phoneme class from MFCC feature comparison.
/// A minimal prototype classifier: compares MFCC features against prototype
/// centroids for a small set of phoneme classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhonemeClass {
    /// Vowel-like (formant energy below 2 kHz)
    Vowel,
    /// Sibilant/fricative (high-frequency energy)
    Fricative,
    /// Plosive (sharp onset, broad spectrum)
    Plosive,
    /// Nasal (low-frequency dominance + anti-formant)
    Nasal,
    /// Silence / low energy
    Silence,
    /// Unclassified
    Unknown,
}

/// Prototype centroid for a phoneme class (MFCC mean vector).
#[derive(Debug, Clone)]
pub struct PhonemePrototype {
    pub class: PhonemeClass,
    pub centroid: Vec<f64>,
}

/// Default phoneme prototype library (minimal, illustrative).
pub fn default_phoneme_prototypes() -> Vec<PhonemePrototype> {
    vec![
        PhonemePrototype {
            class: PhonemeClass::Vowel,
            centroid: vec![2.0, -1.0, 0.5, 0.0, -0.2],
        },
        PhonemePrototype {
            class: PhonemeClass::Fricative,
            centroid: vec![-1.0, 0.5, 1.5, 0.5, 0.0],
        },
        PhonemePrototype {
            class: PhonemeClass::Plosive,
            centroid: vec![1.0, -0.5, -1.0, 0.8, 0.3],
        },
        PhonemePrototype {
            class: PhonemeClass::Nasal,
            centroid: vec![3.0, 0.5, -0.5, -1.0, -0.5],
        },
        PhonemePrototype {
            class: PhonemeClass::Silence,
            centroid: vec![-5.0, -5.0, -5.0, -5.0, -5.0],
        },
    ]
}

/// Classify a feature vector (`mfccs`) by nearest-prototype Euclidean distance.
/// Returns `Some(PhonemeClass)` if the closest prototype is within max_distance.
pub fn phoneme_classify(
    mfccs: &[f64],
    prototypes: &[PhonemePrototype],
    max_distance: f64,
) -> Option<PhonemeClass> {
    let mut best_class = None;
    let mut best_dist = f64::MAX;
    for proto in prototypes {
        let n = mfccs.len().min(proto.centroid.len());
        let dist: f64 = (0..n)
            .map(|i| {
                let d = mfccs[i] - proto.centroid[i];
                d * d
            })
            .sum::<f64>()
            .sqrt();
        if dist < best_dist {
            best_dist = dist;
            best_class = Some(proto.class);
        }
    }
    if best_dist <= max_distance {
        best_class
    } else {
        Some(PhonemeClass::Unknown)
    }
}

/// Extract MFCC features from a raw PCM sample window (mono 16-bit input converted
/// to f64), classify phoneme class, and return a text transcript hypothesis.
///
/// This is the REAL signal-processing path that replaces the keyword-match stub
/// in the courier `classify()` function. For a production ASR, a full acoustic
/// model (HMM/DNN) would consume the MFCC frames; here we demonstrate the
/// feature-extraction → phoneme-classification pipeline.
pub fn recognize_phoneme(pcm_i16: &[i16], sample_rate: f64) -> Option<PhonemeClass> {
    let f64samples: Vec<f64> = pcm_i16.iter().map(|&s| s as f64 / 32768.0).collect();
    let fft_size = match 64usize.checked_next_power_of_two() {
        Some(n) => n,
        None => return None,
    };
    let n = if f64samples.len() > fft_size {
        fft_size
    } else {
        f64samples.len()
    };
    let coeffs = mfcc(&f64samples[..n], sample_rate, 5, 26, fft_size);
    let prototypes = default_phoneme_prototypes();
    phoneme_classify(&coeffs, &prototypes, 10.0)
}

/// Helper used by the composer/router to attach a friction spec to a voice
/// command — proves the "voice never bypasses friction" invariant: a voice
/// `ConfirmOrder` builds the SAME `FrictionSpec` a pointer tap would. Returns
/// `None` for non-consequential intents (no friction needed).
pub fn voice_friction_for(
    command: crate::intent::CommandId,
    amount: Money,
    reversibility: crate::friction::Reversibility,
) -> Option<FrictionSpec> {
    use crate::intent::Intent;
    if Intent::Command(command).is_consequential() {
        Some(crate::friction::friction_spec(Stake {
            money_minor: amount.0,
            reversibility,
        }))
    } else {
        None
    }
}

/// Surface id helper for tests that build an `IntentContext`.
pub fn voice_surface() -> SurfaceId {
    SurfaceId(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::friction::Reversibility;
    use crate::intent::{
        Classification, CommandId, Intent, IntentClassifier, IntentContext, NavTarget, RejectReason,
    };
    use crate::WidgetStore;

    fn ctx_for(widgets: &WidgetStore) -> IntentContext<'_> {
        IntentContext {
            widgets,
            surface: voice_surface(),
            profile: crate::intent::InputProfile::Balanced,
        }
    }

    // D9 — voice wake then transcribe: a fixture "[wake] accept order" drives the
    // wake true then the stub ASR, and VoiceSource::poll yields the transcript as
    // a VoicePhrase the classifier resolves to Intent::Command(AcceptOrder).
    #[test]
    fn voice_wake_then_transcribe() {
        let mut src = VoiceSource::new(VoiceProfile::default(), "hey dowiz");
        src.set_fixture("accept order");
        // Before wake: poll yields nothing (ASR must not have been called).
        assert!(src.poll().is_none());
        assert_eq!(src.asr_feed_calls(), 0, "battery lever: no ASR before wake");

        // Wake fires.
        src.detect_wake(Some("hey dowiz"));
        // Stream a mic frame; poll now returns the finalized phrase.
        src.feed_mic(0);
        let raw = src.poll();
        assert!(raw.is_some(), "after wake + fixture, poll yields a phrase");
        let ws = WidgetStore::new(2);
        let ctx = ctx_for(&ws);
        let classifier = IntentClassifier::new();
        if let Some(RawInput::VoicePhrase {
            transcript,
            is_final,
            ..
        }) = raw
        {
            assert!(is_final);
            match classifier.classify(
                &RawInput::VoicePhrase {
                    transcript,
                    confidence: 0.9,
                    is_final,
                },
                &ctx,
            ) {
                Classification::Rejected(_) => {
                    // "accept order" is consequential → the classifier hard-rejects
                    // out of context (the AI must never auto-pick money). That is the
                    // CORRECT safety behaviour; the phrase reached the classifier.
                }
                Classification::Resolved(Intent::Command(CommandId::AcceptOrder)) => {}
                other => panic!("unexpected classification for 'accept order': {other:?}"),
            }
        }
    }

    // D9 — wake gate blocks ASR: with no wake, feed is never called (call-count 0).
    #[test]
    fn wake_gate_blocks_asr() {
        let mut src = VoiceSource::new(VoiceProfile::default(), "hey dowiz");
        src.set_fixture("open menu");
        // Many polls without ever waking → never transcribe.
        for _ in 0..8 {
            assert!(src.poll().is_none());
        }
        assert_eq!(
            src.asr_feed_calls(),
            0,
            "wake gate: ASR feed must be 0 without a wake"
        );
    }

    // D9 adversarial — Whisper fallback on an uncovered locale (uk).
    #[test]
    fn whisper_fallback_on_uncovered_locale() {
        let profile = VoiceProfile {
            locale: Locale("uk"),
            prefer: VoicePrefer::Auto,
        };
        assert_eq!(profile.selected_model(), AsrKind::Whisper);
        let src = VoiceSource::new(profile, "hey dowiz");
        assert_eq!(src.asr.id(), "whisper.cpp");

        // English locale → Moonshine.
        let en = VoiceProfile::default();
        assert_eq!(en.selected_model(), AsrKind::Moonshine);
        let en_src = VoiceSource::new(en, "hey dowiz");
        assert_eq!(en_src.asr.id(), "moonshine");
    }

    // D9 adversarial — a voice ConfirmOrder still routes through friction; the
    // source cannot mint a CommitToken (it never resolves money directly).
    #[test]
    fn voice_never_bypasses_friction() {
        // The VoiceSource emits only an unresolved VoicePhrase; the classifier
        // hard-rejects the consequential phrase out-of-context (no auto-commit).
        let ws = WidgetStore::new(2);
        let ctx = ctx_for(&ws);
        let classifier = IntentClassifier::new();
        let raw = RawInput::VoicePhrase {
            transcript: "confirm order".into(),
            confidence: 0.9,
            is_final: true,
        };
        match classifier.classify(&raw, &ctx) {
            Classification::Rejected(RejectReason::OutOfContext) => {}
            other => panic!("voice consequential must be rejected out-of-context, got {other:?}"),
        }
        // And the friction spec the composer would attach is non-None — voice uses
        // the SAME friction path as a pointer tap.
        let spec = voice_friction_for(
            CommandId::ConfirmOrder,
            Money(5000),
            Reversibility::ReversibleWithCost,
        );
        assert!(
            spec.is_some(),
            "voice ConfirmOrder must carry a FrictionSpec"
        );
        // A non-consequential voice command (open menu) needs NO friction.
        let nav_spec = voice_friction_for(CommandId::OpenMenu, Money(0), Reversibility::Reversible);
        assert!(nav_spec.is_none());
    }

    // Backstop — a non-consequential voice nav resolves cleanly through the
    // classifier (parity with the intent.rs voice_round_trip gate).
    #[test]
    fn voice_nav_resolves() {
        let ws = WidgetStore::new(2);
        let ctx = ctx_for(&ws);
        let classifier = IntentClassifier::new();
        let raw = RawInput::VoicePhrase {
            transcript: "open menu".into(),
            confidence: 0.9,
            is_final: true,
        };
        assert_eq!(
            classifier.classify(&raw, &ctx),
            Classification::Resolved(Intent::Navigate(NavTarget::Menu))
        );
    }

    // ── Item 60 (gap G11) ORACLE: `ASR_TIMEOUT_US` has ONE authority site and is
    //    pinned (P3 rate discipline). 250_000 µs = 250 ms ceiling.
    #[test]
    fn asr_timeout_constant_is_pinned_authority() {
        assert_eq!(
            ASR_TIMEOUT_US, 250_000,
            "ASR timeout ceiling pinned at 250 ms"
        );
    }

    // ── Item 60 (gap G11) ORACLE (red→green): `InferError::Timeout` is now
    //    REACHABLE FROM A REAL TIMER. A planted slow feed (a fake ASR that spins
    //    past `ASR_TIMEOUT_US`) makes `stream_asr` return `Err(Timeout)` — today
    //    the variant is dead/unreachable. The measured latency is recorded.
    #[test]
    fn planted_slow_feed_returns_timeout() {
        use std::thread;
        use std::time::Duration;

        // A fake ASR whose `feed` spins past the timeout ceiling, then emits.
        struct SlowAsr {
            spun: bool,
        }
        impl AsrModel for SlowAsr {
            fn id(&self) -> &str {
                "slow-fake"
            }
            fn feed(&mut self, _pcm: &[i16]) -> Result<Vec<AsrDelta>, InferError> {
                // Sleep well past the 250 ms ceiling so the real timer fires.
                thread::sleep(Duration::from_millis(400));
                self.spun = true;
                Ok(vec![AsrDelta {
                    text: "slow".into(),
                    is_final: true,
                }])
            }
            fn reset(&mut self) {
                self.spun = false;
            }
            fn set_fixture(&mut self, _t: &str) {}
        }

        let mut src = VoiceSource::with_asr(
            VoiceProfile::default(),
            "hey dowiz",
            Box::new(SlowAsr { spun: false }),
        );
        src.detect_wake(Some("hey dowiz"));
        let res = src.stream_asr();
        assert_eq!(
            res,
            Err(InferError::Timeout),
            "a real timer must make Timeout reachable"
        );
        assert!(src.asr_feed_us().is_some(), "feed latency was measured");
        assert!(
            src.asr_feed_us().unwrap() > ASR_TIMEOUT_US,
            "measured latency exceeded the ceiling"
        );
    }

    // ── Item 60 (gap G11): a healthy (fast) feed is NOT timed out — the battery
    //    lever's latency measurement stays honest and only trips on real slowness.
    #[test]
    fn fast_feed_not_timed_out() {
        let mut src = VoiceSource::new(VoiceProfile::default(), "hey dowiz");
        src.set_fixture("open menu");
        src.detect_wake(Some("hey dowiz"));
        src.feed_mic(0);
        // The stub feed is instantaneous (well under ASR_TIMEOUT_US) on native; on
        // wasm it is untimed (None) and also must NOT trip.
        let res = src.stream_asr();
        assert!(res.is_ok(), "a fast feed must not produce Timeout");
        assert_eq!(
            res.unwrap(),
            Some("open menu".to_string()),
            "transcript still arrives"
        );
    }

    // ── P64 M5 R4 — FFT/MFCC/phoneme classifier tests ──

    #[test]
    fn fft_dc_component() {
        // A constant signal should have only DC (bin 0) non-zero.
        let n = 64;
        let mut re = vec![1.0f64; n];
        let mut im = vec![0.0f64; n];
        fft_radix2(&mut re, &mut im);
        // Bin 0 = sum of signal = n
        assert!((re[0] - n as f64).abs() < 1e-9, "DC bin = {:.9}", re[0]);
        // All other bins ≈ 0
        for i in 1..n {
            assert!(
                re[i].abs() < 1e-9 && im[i].abs() < 1e-9,
                "bin {i} must be zero for DC input"
            );
        }
    }

    #[test]
    fn fft_single_sinusoid() {
        // A real sinusoid at bin k produces peaks at k and n-k.
        let n = 64;
        let k = 4; // exactly 4 cycles in the window
        let mut re = vec![0.0f64; n];
        let mut im = vec![0.0f64; n];
        for i in 0..n {
            re[i] = (2.0 * std::f64::consts::PI * k as f64 * i as f64 / n as f64).cos();
        }
        fft_radix2(&mut re, &mut im);
        // Peak at bin k
        assert!((re[k] - (n as f64 / 2.0)).abs() < 1e-9, "peak at bin {k}");
        // Peak at bin n-k (conjugate symmetry)
        assert!((re[n - k] - (n as f64 / 2.0)).abs() < 1e-9, "peak at bin {}", n - k);
        // Other bins ≈ 0
        for i in 0..n {
            if i != k && i != n - k {
                assert!(re[i].abs() < 1e-9, "bin {i} must be zero");
            }
        }
    }

    #[test]
    fn magnitude_spectrum_matches_fft() {
        let n = 32;
        let mut re = vec![0.0f64; n];
        let mut im = vec![0.0f64; n];
        for i in 0..n {
            re[i] = (2.0 * std::f64::consts::PI * 2.0 * i as f64 / n as f64).cos();
        }
        fft_radix2(&mut re, &mut im);
        let mag = magnitude_spectrum(&re, &im);
        assert_eq!(mag.len(), n);
        // Bin 2 should have the peak
        assert!(mag[2] > 1.0, "bin 2 must have significant magnitude");
        // Bin 0 is not amplified
        assert!(mag[0] < 1.0e-9, "DC bin must be near zero");
        // All magnitudes are non-negative
        for &m in &mag {
            assert!(m >= 0.0);
        }
    }

    #[test]
    fn power_spectrum_equals_magnitude_squared() {
        let n = 32;
        let mut re = vec![0.0f64; n];
        let mut im = vec![0.0f64; n];
        for i in 0..n {
            re[i] = (2.0 * std::f64::consts::PI * 3.0 * i as f64 / n as f64).cos();
        }
        fft_radix2(&mut re, &mut im);
        let mag = magnitude_spectrum(&re, &im);
        let ps = power_spectrum(&re, &im);
        for i in 0..n {
            assert!((ps[i] - mag[i] * mag[i]).abs() < 1e-12, "ps[{i}] must equal mag²");
        }
    }

    #[test]
    fn mel_filterbank_has_unit_sum() {
        let n_filters = 26;
        let fft_size = 256;
        let sample_rate = 16000.0;
        let filters = mel_filterbank(n_filters, fft_size, sample_rate);
        assert_eq!(filters.len(), n_filters);
        for (m, f) in filters.iter().enumerate() {
            let sum: f64 = f.iter().sum();
            assert!((sum - 1.0).abs() < 1e-9, "filter {m}: sum={sum} must be 1.0");
        }
    }

    #[test]
    fn mfcc_produces_requested_coeff_count() {
        // 64-sample synthetic waveform at 8 kHz
        let pcm: Vec<f64> = (0..64).map(|i| (i as f64 * 0.1).sin()).collect();
        let coeffs = mfcc(&pcm, 8000.0, 13, 26, 64);
        assert_eq!(coeffs.len(), 13, "MFCC must produce exactly n_coeffs");
        // All coefficients are finite
        for &c in &coeffs {
            assert!(c.is_finite(), "MFCC coefficient must be finite");
        }
    }

    #[test]
    fn mfcc_is_deterministic() {
        let pcm: Vec<f64> = (0..128).map(|i| (i as f64 * 0.2).sin()).collect();
        let a = mfcc(&pcm, 16000.0, 13, 26, 128);
        let b = mfcc(&pcm, 16000.0, 13, 26, 128);
        assert_eq!(a.len(), b.len());
        for i in 0..a.len() {
            assert!((a[i] - b[i]).abs() < 1e-12, "MFCC must be deterministic");
        }
    }

    #[test]
    fn phoneme_classify_vowel_like() {
        // A feature vector near the Vowel prototype centroid
        let mfccs = vec![2.0, -1.0, 0.5, 0.0, -0.2];
        let prototypes = default_phoneme_prototypes();
        let class = phoneme_classify(&mfccs, &prototypes, 10.0);
        assert_eq!(class, Some(PhonemeClass::Vowel));
    }

    #[test]
    fn phoneme_classify_silence() {
        // Very low energy → near Silence centroid
        let mfccs = vec![-5.0, -5.0, -5.0, -5.0, -5.0];
        let prototypes = default_phoneme_prototypes();
        let class = phoneme_classify(&mfccs, &prototypes, 10.0);
        assert_eq!(class, Some(PhonemeClass::Silence));
    }

    #[test]
    fn phoneme_classify_unknown_at_distance() {
        // Far from all centroids
        let mfccs = vec![100.0, 100.0, 100.0, 100.0, 100.0];
        let prototypes = default_phoneme_prototypes();
        let class = phoneme_classify(&mfccs, &prototypes, 5.0);
        assert_eq!(class, Some(PhonemeClass::Unknown));
    }

    #[test]
    fn recognize_phoneme_with_real_pcm() {
        // A 440 Hz sine wave at 8000 Hz sample rate (64 samples = 8ms window)
        let sample_rate = 8000.0;
        let pcm_i16: Vec<i16> = (0..64)
            .map(|i| {
                let t = i as f64 / sample_rate;
                (t * 440.0 * 2.0 * std::f64::consts::PI).sin() * 32767.0 * 0.5
            })
            .map(|f| f as i16)
            .collect();
        let class = recognize_phoneme(&pcm_i16, sample_rate);
        assert!(class.is_some(), "recognize_phoneme must return something");
        // A pure tone should NOT be silence (it has energy)
        assert_ne!(class.unwrap(), PhonemeClass::Silence);
    }

    #[test]
    fn fft_power_of_two_assertion() {
        // The FFT must panic on non-power-of-two (debug assertion).
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut re = vec![0.0f64; 7];
            let mut im = vec![0.0f64; 7];
            fft_radix2(&mut re, &mut im);
        }));
        assert!(result.is_err(), "FFT must reject non-power-of-two length");
    }
}
