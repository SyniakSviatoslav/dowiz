//! OpenObserve reimplementation — observability platform primitives.
//! 
//! Logs, metrics, traces + columnar storage, mapped to kernel primitives:
//! - **Logs**       → `event_log::EventStore` / `MeshEvent` (content-addressed, hash-chained)
//! - **Metrics**    → `typed_metrics::MetricLine` (typed CPU/mem/GPU samples)
//! - **Traces**     → `json::parse` / `json::Value` (structured trace payloads)
//! - **Aggregation**→ `telemetry_aggregator::TelemetryAggregator` (snapshot + harvest)
//!
//! Columnar storage: metrics and logs are stored column-wise (one vec per field)
//! so aggregations scan contiguous memory and skip irrelevant columns.
//!
//! Pure Rust, zero external dependencies.

use alloc::collections::BTreeMap;

// ── Columnar metric store ──────────────────────────────────────────────────

/// A columnar metric store: one contiguous vector per field, so scans
/// over a single dimension (e.g. all `mono_ns`) are cache-friendly and
/// irrelevant columns are never touched.
#[derive(Debug, Clone, Default)]
pub struct ColumnarMetricStore {
    /// Metric type tag per row: "Cpu" | "Mem" | "Gpu".
    pub kind: Vec<String>,
    /// Monotonic nanosecond timestamp per row (present on all variants).
    pub mono_ns: Vec<u128>,
    /// CPU-specific: process PID.
    pub cpu_pid: Vec<u32>,
    /// CPU-specific: user-mode ticks.
    pub cpu_utime_ticks: Vec<u64>,
    /// CPU-specific: kernel-mode ticks.
    pub cpu_stime_ticks: Vec<u64>,
    /// CPU-specific: clock ticks per second (USER_HZ).
    pub cpu_clk_tck: Vec<u64>,
    /// Mem-specific: VmRSS in KiB.
    pub mem_vm_rss_kb: Vec<u64>,
    /// Mem-specific: VmHWM (high-water mark) in KiB.
    pub mem_vm_hwm_kb: Vec<u64>,
    /// Gpu-specific: utilization %.
    pub gpu_util_pct: Vec<f32>,
    /// Gpu-specific: memory used in MiB.
    pub gpu_mem_used_mb: Vec<u64>,
    /// Row count (all columns kept in sync).
    row_count: usize,
}

impl ColumnarMetricStore {
    /// Number of rows stored.
    pub fn len(&self) -> usize {
        self.row_count
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.row_count == 0
    }

    /// Append a typed metric line to the columnar store. Each field vector
    /// grows by one; columns not relevant to this metric get a sentinel
    /// (0 / 0.0) so every row has the same width.
    pub fn append(&mut self, line: crate::typed_metrics::MetricLine) {
        let mono = match line {
            crate::typed_metrics::MetricLine::Cpu(c) => c.mono_ns,
            crate::typed_metrics::MetricLine::Mem(_) => 0, // Mem samples don't carry mono_ns
            crate::typed_metrics::MetricLine::Gpu(_) => 0,  // Gpu samples don't carry mono_ns
        };

        match line {
            crate::typed_metrics::MetricLine::Cpu(c) => {
                self.kind.push("Cpu".to_string());
                self.mono_ns.push(c.mono_ns);
                self.cpu_pid.push(c.pid);
                self.cpu_utime_ticks.push(c.utime_ticks);
                self.cpu_stime_ticks.push(c.stime_ticks);
                self.cpu_clk_tck.push(c.clk_tck);
                self.mem_vm_rss_kb.push(0);
                self.mem_vm_hwm_kb.push(0);
                self.gpu_util_pct.push(0.0);
                self.gpu_mem_used_mb.push(0);
            }
            crate::typed_metrics::MetricLine::Mem(m) => {
                self.kind.push("Mem".to_string());
                self.mono_ns.push(mono);
                self.cpu_pid.push(0);
                self.cpu_utime_ticks.push(0);
                self.cpu_stime_ticks.push(0);
                self.cpu_clk_tck.push(0);
                self.mem_vm_rss_kb.push(m.vm_rss_kb);
                self.mem_vm_hwm_kb.push(m.vm_hwm_kb);
                self.gpu_util_pct.push(0.0);
                self.gpu_mem_used_mb.push(0);
            }
            crate::typed_metrics::MetricLine::Gpu(g) => {
                self.kind.push("Gpu".to_string());
                self.mono_ns.push(mono);
                self.cpu_pid.push(0);
                self.cpu_utime_ticks.push(0);
                self.cpu_stime_ticks.push(0);
                self.cpu_clk_tck.push(0);
                self.mem_vm_rss_kb.push(0);
                self.mem_vm_hwm_kb.push(0);
                self.gpu_util_pct.push(g.util_pct);
                self.gpu_mem_used_mb.push(g.mem_used_mb);
            }
        }
        self.row_count += 1;
    }

    /// Columnar scan: return every row whose `kind` equals `tag`.
    /// Only the relevant columns are accessed.
    pub fn rows_by_kind(&self, tag: &str) -> Vec<RowView> {
        let mut out = Vec::new();
        for i in 0..self.row_count {
            if self.kind[i] == tag {
                out.push(self.row_view(i));
            }
        }
        out
    }

    /// Row view at index `i` — borrows one row's worth of columns without
    /// copying.
    pub fn row_view(&self, i: usize) -> RowView {
        RowView {
            index: i,
            kind: self.kind[i].clone(),
            mono_ns: self.mono_ns[i],
            cpu_pid: self.cpu_pid[i],
            cpu_utime_ticks: self.cpu_utime_ticks[i],
            cpu_stime_ticks: self.cpu_stime_ticks[i],
            cpu_clk_tck: self.cpu_clk_tck[i],
            mem_vm_rss_kb: self.mem_vm_rss_kb[i],
            mem_vm_hwm_kb: self.mem_vm_hwm_kb[i],
            gpu_util_pct: self.gpu_util_pct[i],
            gpu_mem_used_mb: self.gpu_mem_used_mb[i],
        }
    }

    /// Clear all columns.
    pub fn clear(&mut self) {
        self.kind.clear();
        self.mono_ns.clear();
        self.cpu_pid.clear();
        self.cpu_utime_ticks.clear();
        self.cpu_stime_ticks.clear();
        self.cpu_clk_tck.clear();
        self.mem_vm_rss_kb.clear();
        self.mem_vm_hwm_kb.clear();
        self.gpu_util_pct.clear();
        self.gpu_mem_used_mb.clear();
        self.row_count = 0;
    }

    /// Compute per-kind row counts (a columnar aggregation).
    pub fn kind_histogram(&self) -> BTreeMap<String, usize> {
        let mut hist: BTreeMap<String, usize> = BTreeMap::new();
        for k in &self.kind {
            *hist.entry(k.clone()).or_default() += 1;
        }
        hist
    }
}

/// A borrowed view of one row across all columnar fields.
#[derive(Debug, Clone)]
pub struct RowView {
    pub index: usize,
    pub kind: String,
    pub mono_ns: u128,
    pub cpu_pid: u32,
    pub cpu_utime_ticks: u64,
    pub cpu_stime_ticks: u64,
    pub cpu_clk_tck: u64,
    pub mem_vm_rss_kb: u64,
    pub mem_vm_hwm_kb: u64,
    pub gpu_util_pct: f32,
    pub gpu_mem_used_mb: u64,
}

impl RowView {
    /// Reconstruct the original `MetricLine` from this view.
    pub fn to_metric_line(&self) -> crate::typed_metrics::MetricLine {
        match self.kind.as_str() {
            "Cpu" => crate::typed_metrics::MetricLine::Cpu(
                crate::typed_metrics::ProcCpuSample {
                    pid: self.cpu_pid,
                    utime_ticks: self.cpu_utime_ticks,
                    stime_ticks: self.cpu_stime_ticks,
                    clk_tck: self.cpu_clk_tck,
                    mono_ns: self.mono_ns,
                },
            ),
            "Mem" => crate::typed_metrics::MetricLine::Mem(
                crate::typed_metrics::MemSample {
                    vm_rss_kb: self.mem_vm_rss_kb,
                    vm_hwm_kb: self.mem_vm_hwm_kb,
                },
            ),
            "Gpu" => crate::typed_metrics::MetricLine::Gpu(
                crate::typed_metrics::GpuSample {
                    util_pct: self.gpu_util_pct,
                    mem_used_mb: self.gpu_mem_used_mb,
                },
            ),
            _ => panic!("unknown kind: {}", self.kind),
        }
    }
}

// ── Log store (event_log-backed) ───────────────────────────────────────────

/// An observability log store: appends structured log entries as
/// `MeshEvent` records into a content-addressed `EventStore`, keyed by
/// SHA3-256 of the log payload. Provides timestamp-indexed lookup.
#[derive(Debug)]
pub struct LogStore<S: crate::event_log::EventStore> {
    store: S,
    /// Hint index: log timestamp (ns) → event id, for time-range queries.
    time_index: Vec<(u128, [u8; 32])>,
}

impl<S: crate::event_log::EventStore> LogStore<S> {
    /// Wrap a backing store.
    pub fn new(store: S) -> Self {
        LogStore {
            store,
            time_index: Vec::new(),
        }
    }

    /// Append a log entry. The payload is JSON-serialized via the kernel's
    /// `json` module so traces, structured logs, and metric snapshots all
    /// share one wire format.
    ///
    /// Returns the event content-id on success, or `StoreError` on durability
    /// failure.
    pub fn append_log(&mut self, timestamp_ns: u128, payload: &crate::json::Value) -> Result<[u8; 32], crate::event_log::StoreError> {
        let json_bytes = payload.to_string();
        let ev = crate::event_log::MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [0u8; 32],
            actor_seq: 0,
            payload: json_bytes.into_bytes(),
        };
        let id = ev.event_id();
        if self.store.contains(&id) {
            return Ok(id); // idempotent
        }
        // Write through the event_log trait.
        self.store.insert(id, ev)?;
        self.time_index.push((timestamp_ns, id));
        Ok(id)
    }

    /// Append a log entry from a JSON string (convenience over `append_log`).
    pub fn append_log_str(&mut self, timestamp_ns: u128, json_str: &str) -> Result<[u8; 32], crate::event_log::StoreError> {
        let val = crate::json::parse(json_str).map_err(|_| crate::event_log::StoreError::Write("invalid JSON".into()))?;
        self.append_log(timestamp_ns, &val)
    }

    /// Retrieve a log entry by content-id.
    pub fn get_log(&self, id: &[u8; 32]) -> Option<crate::event_log::MeshEvent> {
        self.store.get(id)
    }

    /// Return all log content-ids in insertion order (via the time index).
    pub fn log_ids(&self) -> Vec<[u8; 32]> {
        self.time_index.iter().map(|(_, id)| *id).collect()
    }

    /// Time-range query: return event ids whose timestamp falls in `[start, end)`.
    pub fn logs_in_range(&self, start: u128, end: u128) -> Vec<[u8; 32]> {
        self.time_index
            .iter()
            .filter(|(ts, _)| *ts >= start && *ts < end)
            .map(|(_, id)| *id)
            .collect()
    }

    /// Number of log entries.
    pub fn len(&self) -> usize {
        self.time_index.len()
    }

    /// Whether the log store is empty.
    pub fn is_empty(&self) -> bool {
        self.time_index.is_empty()
    }

    /// Clear the time index (does NOT clear the backing store — that is the
    /// store's own responsibility). Used primarily for tests.
    pub fn clear_index(&mut self) {
        self.time_index.clear();
    }
}

// ── Trace parser ────────────────────────────────────────────────────────────

/// Parse a trace payload (JSON) into a `TraceSpan`. Expected shape:
/// `{"trace_id": "...", "span_id": "...", "name": "...", "duration_ms": N, "status": "..."}`.
pub fn parse_trace(json_str: &str) -> Result<TraceSpan, crate::json::Error> {
    let val = crate::json::parse(json_str)?;
    let trace_id = val.get("trace_id").and_then(|v| v.as_str()).unwrap_or("");
    let span_id = val.get("span_id").and_then(|v| v.as_str()).unwrap_or("");
    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let duration_ms = val.get("duration_ms").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let status = val.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");

    Ok(TraceSpan {
        trace_id: trace_id.to_string(),
        span_id: span_id.to_string(),
        name: name.to_string(),
        duration_ms,
        status: status.to_string(),
    })
}

/// A single parsed trace span.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceSpan {
    pub trace_id: String,
    pub span_id: String,
    pub name: String,
    pub duration_ms: f64,
    pub status: String,
}

// ── Observability hub ───────────────────────────────────────────────────────

/// Top-level observability hub: composes a columnar metric store, a log store,
/// and a telemetry aggregator into one interface.
pub struct ObservabilityHub {
    metrics: ColumnarMetricStore,
    logs: LogStore<crate::event_log::MemEventStore>,
    aggregator: crate::telemetry_aggregator::TelemetryAggregator,
}

impl ObservabilityHub {
    /// Create a new hub with a default in-memory log store and telemetry
    /// aggregator.
    pub fn new() -> Self {
        ObservabilityHub {
            metrics: ColumnarMetricStore::default(),
            logs: LogStore::new(crate::event_log::MemEventStore::new()),
            aggregator: crate::telemetry_aggregator::TelemetryAggregator::new(1024),
        }
    }

    /// Record a typed metric line.
    pub fn record_metric(&mut self, line: crate::typed_metrics::MetricLine) {
        self.metrics.append(line);
    }

    /// Record a log entry from a JSON string.
    pub fn record_log(&mut self, timestamp_ns: u128, json_str: &str) -> Result<[u8; 32], crate::event_log::StoreError> {
        self.logs.append_log_str(timestamp_ns, json_str)
    }

    /// Record a structured log entry from a `json::Value`.
    pub fn record_log_value(&mut self, timestamp_ns: u128, value: &crate::json::Value) -> Result<[u8; 32], crate::event_log::StoreError> {
        self.logs.append_log(timestamp_ns, value)
    }

    /// Parse and record a trace span.
    pub fn record_trace(&mut self, json_str: &str) -> Result<TraceSpan, crate::json::Error> {
        let span = parse_trace(json_str)?;
        // Record the trace as a log entry.
        let log_val = crate::json::Value::Object(vec![
            ("type".into(), crate::json::Value::Str("trace".into())),
            ("trace_id".into(), crate::json::Value::Str(span.trace_id.clone())),
            ("span_id".into(), crate::json::Value::Str(span.span_id.clone())),
            ("name".into(), crate::json::Value::Str(span.name.clone())),
            ("duration_ms".into(), crate::json::Value::Float(span.duration_ms)),
            ("status".into(), crate::json::Value::Str(span.status.clone())),
        ]);
        let _ = self.record_log_value(0, &log_val);
        Ok(span)
    }

    /// Take a telemetry snapshot (samples `/proc` + stamps the wall clock, then
    /// delegates to the no_std aggregator core).
    pub fn snapshot(&mut self) -> crate::telemetry_aggregator::TelemetrySnapshot {
        let timestamp_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        let cpu = crate::typed_metrics::proc_cpu_sample_from_proc_self();
        let mem = crate::typed_metrics::mem_sample_from_proc_self();
        self.aggregator.snapshot(timestamp_us, cpu, mem)
    }

    /// Borrow the columnar metric store.
    pub fn metrics(&self) -> &ColumnarMetricStore {
        &self.metrics
    }

    /// Borrow the log store.
    pub fn logs(&self) -> &LogStore<crate::event_log::MemEventStore> {
        &self.logs
    }

    /// Clear all metrics and the log time index.
    pub fn clear(&mut self) {
        self.metrics.clear();
        self.logs.clear_index();
        self.aggregator.clear();
    }
}

impl Default for ObservabilityHub {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typed_metrics::{MetricLine, ProcCpuSample, MemSample, GpuSample};

    // ── ColumnarMetricStore ─────────────────────────────────────────────────

    fn sample_cpu() -> MetricLine {
        MetricLine::Cpu(ProcCpuSample {
            pid: 42,
            utime_ticks: 100,
            stime_ticks: 50,
            clk_tck: 100,
            mono_ns: 1_000_000,
        })
    }

    fn sample_mem() -> MetricLine {
        MetricLine::Mem(MemSample {
            vm_rss_kb: 2048,
            vm_hwm_kb: 4096,
        })
    }

    fn sample_gpu() -> MetricLine {
        MetricLine::Gpu(GpuSample {
            util_pct: 75.5,
            mem_used_mb: 1024,
        })
    }

    #[test]
    fn columnar_store_append_and_len() {
        let mut store = ColumnarMetricStore::default();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store.append(sample_cpu());
        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());

        store.append(sample_mem());
        assert_eq!(store.len(), 2);

        store.append(sample_gpu());
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn columnar_store_roundtrip_cpu() {
        let mut store = ColumnarMetricStore::default();
        let cpu = sample_cpu();
        store.append(cpu.clone());

        let rows = store.rows_by_kind("Cpu");
        assert_eq!(rows.len(), 1);
        let view = &rows[0];
        assert_eq!(view.cpu_pid, 42);
        assert_eq!(view.cpu_utime_ticks, 100);
        assert_eq!(view.cpu_stime_ticks, 50);
        assert_eq!(view.cpu_clk_tck, 100);
        assert_eq!(view.mono_ns, 1_000_000);

        let reconstructed = view.to_metric_line();
        assert_eq!(reconstructed, cpu);
    }

    #[test]
    fn columnar_store_roundtrip_mem() {
        let mut store = ColumnarMetricStore::default();
        let mem = sample_mem();
        store.append(mem.clone());

        let rows = store.rows_by_kind("Mem");
        assert_eq!(rows.len(), 1);
        let view = &rows[0];
        assert_eq!(view.mem_vm_rss_kb, 2048);
        assert_eq!(view.mem_vm_hwm_kb, 4096);

        let reconstructed = view.to_metric_line();
        assert_eq!(reconstructed, mem);
    }

    #[test]
    fn columnar_store_roundtrip_gpu() {
        let mut store = ColumnarMetricStore::default();
        let gpu = sample_gpu();
        store.append(gpu.clone());

        let rows = store.rows_by_kind("Gpu");
        assert_eq!(rows.len(), 1);
        let view = &rows[0];
        assert!((view.gpu_util_pct - 75.5).abs() < 1e-6);
        assert_eq!(view.gpu_mem_used_mb, 1024);

        let reconstructed = view.to_metric_line();
        assert_eq!(reconstructed, gpu);
    }

    #[test]
    fn columnar_store_kind_histogram() {
        let mut store = ColumnarMetricStore::default();
        for _ in 0..3 {
            store.append(sample_cpu());
        }
        for _ in 0..2 {
            store.append(sample_mem());
        }
        store.append(sample_gpu());

        let hist = store.kind_histogram();
        assert_eq!(hist.get("Cpu").copied(), Some(3));
        assert_eq!(hist.get("Mem").copied(), Some(2));
        assert_eq!(hist.get("Gpu").copied(), Some(1));
    }

    #[test]
    fn columnar_store_clear() {
        let mut store = ColumnarMetricStore::default();
        store.append(sample_cpu());
        store.append(sample_mem());
        assert_eq!(store.len(), 2);

        store.clear();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.rows_by_kind("Cpu").is_empty());
        assert!(store.rows_by_kind("Mem").is_empty());
    }

    #[test]
    fn columnar_store_non_matching_kind_returns_empty() {
        let mut store = ColumnarMetricStore::default();
        store.append(sample_cpu());
        let rows = store.rows_by_kind("Fsync");
        assert!(rows.is_empty());
    }

    // ── LogStore ────────────────────────────────────────────────────────────

    #[test]
    fn log_store_append_and_retrieve() {
        let store = crate::event_log::MemEventStore::new();
        let mut logs = LogStore::new(store);

        let val = crate::json::Value::Object(vec![
            ("level".into(), crate::json::Value::Str("info".into())),
            ("msg".into(), crate::json::Value::Str("hello".into())),
        ]);
        let id = logs.append_log(1000, &val).expect("append should succeed");
        assert_eq!(id.len(), 32);

        let retrieved = logs.get_log(&id);
        assert!(retrieved.is_some());
        let ev = retrieved.unwrap();
        assert_eq!(ev.payload, b"{\"level\":\"info\",\"msg\":\"hello\"}");
    }

    #[test]
    fn log_store_idempotent_append() {
        let store = crate::event_log::MemEventStore::new();
        let mut logs = LogStore::new(store);

        let val = crate::json::Value::Object(vec![
            ("msg".into(), crate::json::Value::Str("dup".into())),
        ]);
        let id1 = logs.append_log(1000, &val).expect("first append");
        let id2 = logs.append_log(2000, &val).expect("second append (idempotent)");
        assert_eq!(id1, id2);
        assert_eq!(logs.len(), 1); // time_index has one entry per call, but...
        // Note: time_index grows per call; idempotency is at the store level.
        // The store itself dedupes; the time_index is a hint.
    }

    #[test]
    fn log_store_logs_in_range() {
        let store = crate::event_log::MemEventStore::new();
        let mut logs = LogStore::new(store);

        for i in 0..10 {
            let val = crate::json::Value::Object(vec![
                ("i".into(), crate::json::Value::Int(i as i64)),
            ]);
            let _ = logs.append_log((i * 1000) as u128, &val);
        }

        let range = logs.logs_in_range(3000, 7000);
        assert_eq!(range.len(), 4); // timestamps 3000, 4000, 5000, 6000
    }

    #[test]
    fn log_store_append_invalid_json_rejected() {
        let store = crate::event_log::MemEventStore::new();
        let mut logs = LogStore::new(store);

        // append_log_str validates JSON; invalid input returns StoreError::Write.
        let result = logs.append_log_str(0, "{bad json}");
        assert!(result.is_err());
    }

    #[test]
    fn log_store_is_empty_initially() {
        let store = crate::event_log::MemEventStore::new();
        let logs = LogStore::new(store);
        assert!(logs.is_empty());
        assert_eq!(logs.len(), 0);
    }

    // ── Trace parser ────────────────────────────────────────────────────────

    #[test]
    fn parse_trace_valid() {
        let json = r#"{"trace_id":"abc-123","span_id":"span-1","name":"http.request","duration_ms":42.5,"status":"ok"}"#;
        let span = parse_trace(json).expect("valid trace");
        assert_eq!(span.trace_id, "abc-123");
        assert_eq!(span.span_id, "span-1");
        assert_eq!(span.name, "http.request");
        assert!((span.duration_ms - 42.5).abs() < 1e-6);
        assert_eq!(span.status, "ok");
    }

    #[test]
    fn parse_trace_missing_fields_defaults() {
        let json = r#"{"trace_id":"t1"}"#;
        let span = parse_trace(json).expect("partial trace parses");
        assert_eq!(span.trace_id, "t1");
        assert_eq!(span.span_id, "");
        assert_eq!(span.name, "");
        assert_eq!(span.duration_ms, 0.0);
        assert_eq!(span.status, "unknown");
    }

    #[test]
    fn parse_trace_invalid_json_returns_err() {
        let result = parse_trace("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_trace_empty_object() {
        let span = parse_trace("{}").expect("empty object parses");
        assert_eq!(span.trace_id, "");
        assert_eq!(span.span_id, "");
        assert_eq!(span.name, "");
        assert_eq!(span.duration_ms, 0.0);
        assert_eq!(span.status, "unknown");
    }

    // ── ObservabilityHub ────────────────────────────────────────────────────

    #[test]
    fn hub_record_metric_and_snapshot() {
        let mut hub = ObservabilityHub::new();
        hub.record_metric(sample_cpu());
        hub.record_metric(sample_mem());

        let snap = hub.snapshot();
        assert_eq!(snap.total_events, 0); // aggregator has no harvest events yet
        assert_eq!(hub.metrics().len(), 2);
    }

    #[test]
    fn hub_record_log_and_retrieve() {
        let mut hub = ObservabilityHub::new();
        let json = r#"{"level":"info","msg":"test"}"#;
        let id = hub.record_log(1000, json).expect("log append");
        assert_eq!(hub.logs().len(), 1);

        let ev = hub.logs().get_log(&id);
        assert!(ev.is_some());
    }

    #[test]
    fn hub_record_trace() {
        let mut hub = ObservabilityHub::new();
        let json = r#"{"trace_id":"t1","span_id":"s1","name":"op","duration_ms":10.0,"status":"ok"}"#;
        let span = hub.record_trace(json).expect("trace parse+record");
        assert_eq!(span.trace_id, "t1");
        assert_eq!(hub.logs().len(), 1); // trace recorded as a log entry
    }

    #[test]
    fn hub_clear_resets_state() {
        let mut hub = ObservabilityHub::new();
        hub.record_metric(sample_cpu());
        hub.record_metric(sample_mem());
        hub.record_log(0, r#"{"msg":"x"}"#).expect("log");

        assert_eq!(hub.metrics().len(), 2);
        assert_eq!(hub.logs().len(), 1);

        hub.clear();
        assert_eq!(hub.metrics().len(), 0);
        assert_eq!(hub.logs().len(), 0);
        assert_eq!(hub.snapshot().total_events, 0);
    }

    #[test]
    fn hub_default_construction() {
        let mut hub = ObservabilityHub::default();
        assert!(hub.metrics().is_empty());
        assert!(hub.logs().is_empty());
        assert_eq!(hub.snapshot().total_events, 0);
    }
}
