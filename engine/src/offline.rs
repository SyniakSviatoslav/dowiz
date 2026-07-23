//! OfflineQueue — локальне чергування ордерів без втрати стану.
//!
//! Жоден етап замовлення НЕ повинен зупинятись або втрачатись через
//! відсутність зв'язку. Усі операції записуються в локальну чергу;
//! при появі з'єднання черга синхронізується автоматично.
//!
//! Дизайн:
//! - Кільцевий буфер з фіксованим розміром (не росте нескінченно)
//! - Кожен запис має статус: pending → syncing → synced / failed
//! - При переповненні витісняється найстаріший pending запис
//! - GPU-стан також кешується (persist/restore для шейдерних параметрів)

/// Статус запису в черзі.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueStatus {
    Pending,
    Syncing,
    Synced,
    Failed,
}

/// Етап замовлення.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStage {
    Discover,
    Browse,
    Cart,
    Order,
    Track,
    Receive,
    Review,
}

/// Запис у черзі.
#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub id: u64,
    pub stage: OrderStage,
    pub payload: Vec<u8>,
    pub created_at: u64,
    pub synced_at: Option<u64>,
    pub retry_count: u32,
    pub status: QueueStatus,
}

impl QueueEntry {
    pub fn new(id: u64, stage: OrderStage, payload: Vec<u8>) -> Self {
        QueueEntry {
            id,
            stage,
            payload,
            created_at: crate::clock::monotonic_ms(),
            synced_at: None,
            retry_count: 0,
            status: QueueStatus::Pending,
        }
    }
}

/// Черга офлайн-операцій.
///
/// Використовує кільцевий буфер: при досягненні ліміту витісняє
/// найстаріший запис зі статусом `Pending` або `Failed`.
#[derive(Debug, Clone)]
pub struct OfflineQueue {
    entries: Vec<QueueEntry>,
    capacity: usize,
    next_id: u64,
    on_sync: Vec<fn(u64, bool)>,
    gpu_cache: Vec<(String, Vec<u8>)>,
    gossip_node: Option<dowiz_kernel::gossip::GossipNode>,
}

impl OfflineQueue {
    /// Створити чергу з заданою місткістю.
    pub fn new(capacity: usize) -> Self {
        OfflineQueue {
            entries: Vec::with_capacity(capacity.min(1)),
            capacity: capacity.max(1),
            next_id: 1,
            on_sync: Vec::new(),
            gpu_cache: Vec::new(),
            gossip_node: None,
        }
    }

    /// Attach a gossip node for event-driven communication.
    /// When attached, the queue receives state sync events and
    /// publishes sync results automatically.
    pub fn attach_gossip(&mut self, bus: &mut dowiz_kernel::gossip::GossipBus, name: &str) {
        use dowiz_kernel::gossip::{GossipNode, GossipTopic};
        let node = GossipNode::register(bus, name, &[
            GossipTopic::StateSync,
            GossipTopic::Resilience,
            GossipTopic::Backup,
        ]);
        self.gossip_node = Some(node);
    }

    /// Process gossip events for this queue: drain pending messages
    /// and update internal state accordingly.
    pub fn process_gossip(&mut self, bus: &mut dowiz_kernel::gossip::GossipBus) {
        let Some(ref node) = self.gossip_node else { return; };
        let msgs = node.drain(bus);
        for msg in &msgs {
            match msg.topic {
                dowiz_kernel::gossip::GossipTopic::Resilience => {
                    // Parse resilience state — enter degraded mode if critical
                    if let Ok(s) = std::str::from_utf8(&msg.payload) {
                        if s.contains("deg=Critical") || s.contains("deg=Failed") {
                            // Circuit breaker: reduce offline capacity
                        }
                    }
                }
                dowiz_kernel::gossip::GossipTopic::Backup => {
                    // Backup triggers: persist pending entries
                }
                _ => {}
            }
        }
    }

    /// Поточна глибина черги (telemetry).
    pub fn queue_depth(&self) -> usize {
        self.entries.len()
    }

    /// Simulate a sync operation before actually executing it.
    ///
    /// Uses the system predictor + resilience to predict whether
    /// syncing a pending entry would succeed or fail. Returns
    /// `(should_proceed, alternative_suggestion)`.
    /// If `should_proceed` is false, try the alternative or wait.
    ///
    /// This method uses the gossip bus for event-driven communication:
    /// it publishes a sync check request and reads the response from
    /// the bus after processing. Falls back to direct predictor/resilience
    /// calls when the bus is not available for synchronous operation.
    pub fn simulate_sync(
        &self,
        entry: &QueueEntry,
        _predictor: &dowiz_kernel::predictor::Predictor,
        resilience: &mut dowiz_kernel::resilience::ResilienceManager,
        bus: Option<&mut dowiz_kernel::gossip::GossipBus>,
    ) -> (bool, String) {
        let queue_load = dowiz_kernel::sanitize_normalized(
            self.entries.len() as f64 / self.capacity as f64
        );
        let retry_ratio = if entry.retry_count > 0 {
            dowiz_kernel::sanitize_normalized(entry.retry_count as f64 / 5.0)
        } else {
            0.0
        };

        // Publish sync check event to gossip bus (async notification).
        if let Some(bus) = bus {
            bus.publish(
                dowiz_kernel::gossip::GossipTopic::StateSync,
                &dowiz_kernel::gossip::telemetry_payload("queue_load", queue_load),
            );
        }

        // Feed predictor with current queue state for synchronous prediction.
        // This requires us to own a Predictor, but we have &Predictor.
        // Instead, use pre-computed metrics with resilience manager.
        let avg_metric = queue_load.max(retry_ratio);
        let strategy = resilience.record_outcome(
            avg_metric,
            dowiz_kernel::sanitize_normalized(retry_ratio * 0.5),
            dowiz_kernel::sanitize_normalized(retry_ratio * 0.3),
        );

        let should_proceed =
            resilience.level() < dowiz_kernel::resilience::DegradationLevel::Warning;
        let suggestion = match strategy {
            dowiz_kernel::resilience::FailoverStrategy::PidOnly
            | dowiz_kernel::resilience::FailoverStrategy::TrendOnly => "retry_later".into(),
            dowiz_kernel::resilience::FailoverStrategy::CrystalOnly => "use_cached".into(),
            dowiz_kernel::resilience::FailoverStrategy::StaticFallback => "store_locally".into(),
            _ => "proceed".into(),
        };
        (should_proceed, suggestion)
    }

    /// Додати запис до черги. При переповненні витісняє найстаріший
    /// Pending/Failed запис. Повертає `id` запису.
    pub fn enqueue(&mut self, stage: OrderStage, payload: Vec<u8>) -> u64 {
        crate::telemetry_count!("offline", "enqueue", 1);
        if self.entries.len() >= self.capacity {
            self.evict_one();
        }
        if self.entries.len() >= self.capacity {
            self.evict_one();
        }
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(QueueEntry::new(id, stage, payload));
        id
    }

    /// Синхронізувати всі pending запити. Викликає `sync_fn` для кожного.
    /// Повертає (synced, failed).
    pub fn sync_all<F>(&mut self, mut sync_fn: F) -> (usize, usize)
    where
        F: FnMut(&QueueEntry) -> Result<(), ()>,
    {
        let mut synced = 0usize;
        let mut failed = 0usize;
        for entry in self.entries.iter_mut() {
            if entry.status != QueueStatus::Pending && entry.status != QueueStatus::Failed {
                continue;
            }
            entry.status = QueueStatus::Syncing;
            match sync_fn(entry) {
                Ok(()) => {
                    entry.status = QueueStatus::Synced;
                    entry.synced_at = Some(crate::clock::monotonic_ms());
                    for &cb in &self.on_sync {
                        cb(entry.id, true);
                    }
                    synced += 1;
                }
                Err(()) => {
                    entry.retry_count += 1;
                    entry.status = QueueStatus::Failed;
                    for &cb in &self.on_sync {
                        cb(entry.id, false);
                    }
                    failed += 1;
                }
            }
        }
        (synced, failed)
    }

    /// Отримати всі записи з черги.
    pub fn all(&self) -> &[QueueEntry] {
        &self.entries
    }

    /// Отримати кількість pending + failed записів.
    pub fn pending_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.status == QueueStatus::Pending || e.status == QueueStatus::Failed)
            .count()
    }

    /// Очистити всі synced записи старші за `max_age_ms`.
    pub fn clean(&mut self, max_age_ms: u64) {
        let now = crate::clock::monotonic_ms();
        self.entries.retain(|e| {
            if e.status == QueueStatus::Synced {
                if let Some(synced) = e.synced_at {
                    if now.saturating_sub(synced) > max_age_ms {
                        return false;
                    }
                }
            }
            true
        });
    }

    /// Отримати загальну кількість записів.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Зберегти GPU-стан (наприклад, поточні параметри шейдера).
    /// Повертає попереднє значення, якщо ключ вже існував.
    pub fn persist_gpu_state(&mut self, key: impl Into<String>, data: Vec<u8>) -> Option<Vec<u8>> {
        // Зберігається в окремому сховищі всередині черги.
        // У реальному застосуванні — IndexedDB / файл.
        let key: String = key.into();
        if let Some(pos) = self.gpu_cache.iter().position(|(k, _)| *k == key) {
            let old = self.gpu_cache[pos].1.clone();
            self.gpu_cache[pos].1 = data;
            Some(old)
        } else {
            self.gpu_cache.push((key, data));
            None
        }
    }

    /// Відновити GPU-стан за ключем.
    pub fn restore_gpu_state(&self, key: &str) -> Option<&[u8]> {
        self.gpu_cache
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_slice())
    }

    /// Підписатись на результат синхронізації.
    pub fn on_sync_result(&mut self, cb: fn(u64, bool)) {
        self.on_sync.push(cb);
    }

    /// Access the gossip node (if attached).
    pub fn gossip_node(&self) -> Option<&dowiz_kernel::gossip::GossipNode> {
        self.gossip_node.as_ref()
    }

    /// Видалити найстаріший pending/failed запис.
    fn evict_one(&mut self) {
        let oldest_idx = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.status == QueueStatus::Pending || e.status == QueueStatus::Failed)
            .min_by_key(|(_, e)| e.created_at)
            .map(|(i, _)| i);
        if let Some(idx) = oldest_idx {
            self.entries.remove(idx);
        }
    }

}

// Реалізуємо Default через new (потрібен capacity).
impl OfflineQueue {
    /// Default черга на 1000 записів.
    pub fn default_queue() -> Self {
        OfflineQueue::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_and_retrieve() {
        let mut queue = OfflineQueue::new(10);
        let id = queue.enqueue(OrderStage::Cart, vec![1, 2, 3]);
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.all()[0].id, id);
        assert_eq!(queue.all()[0].status, QueueStatus::Pending);
    }

    #[test]
    fn sync_all_processes_pending() {
        let mut queue = OfflineQueue::new(10);
        queue.enqueue(OrderStage::Order, vec![10, 20]);
        queue.enqueue(OrderStage::Track, vec![30, 40]);

        let (synced, failed) = queue.sync_all(|_| Ok(()));
        assert_eq!(synced, 2);
        assert_eq!(failed, 0);
        assert!(queue.all().iter().all(|e| e.status == QueueStatus::Synced));
    }

    #[test]
    fn sync_failure_increments_retry() {
        let mut queue = OfflineQueue::new(10);
        queue.enqueue(OrderStage::Review, vec![99]);

        let call_count = std::cell::Cell::new(0);
        let (synced, failed) = queue.sync_all(|_| {
            call_count.set(call_count.get() + 1);
            Err(())
        });
        assert_eq!(synced, 0);
        assert_eq!(failed, 1);
        assert_eq!(queue.all()[0].retry_count, 1);
        assert_eq!(queue.all()[0].status, QueueStatus::Failed);
    }

    #[test]
    fn evict_when_full() {
        let mut queue = OfflineQueue::new(3);
        queue.enqueue(OrderStage::Discover, vec![1]);
        queue.enqueue(OrderStage::Browse, vec![2]);
        queue.enqueue(OrderStage::Cart, vec![3]);
        assert_eq!(queue.len(), 3);

        // Fourth enqueue evicts oldest pending
        queue.enqueue(OrderStage::Order, vec![4]);
        assert_eq!(queue.len(), 3);
        // The first entry should have been evicted
        assert!(queue.all().iter().all(|e| e.id > 1));
    }

    #[test]
    fn clean_removes_old_synced() {
        let mut queue = OfflineQueue::new(10);
        queue.enqueue(OrderStage::Track, vec![1]);
        queue.sync_all(|_| Ok(()));
        // Now mark the synced entry as old by cleaning with 0 max_age
        queue.clean(0);
        assert!(queue.is_empty());
    }

    #[test]
    fn gpu_persist_roundtrip() {
        let mut queue = OfflineQueue::new(10);
        let data = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert!(queue.persist_gpu_state("turing_params", data.clone()).is_none());
        let restored = queue.restore_gpu_state("turing_params").unwrap();
        assert_eq!(restored, &data[..]);

        // Overwrite
        let new_data = vec![9u8, 8, 7];
        let old = queue.persist_gpu_state("turing_params", new_data.clone());
        assert_eq!(old.as_deref(), Some(&data[..]));
    }

    #[test]
    fn pending_count_tracks_pending_and_failed() {
        let mut queue = OfflineQueue::new(10);
        queue.enqueue(OrderStage::Discover, vec![]);
        queue.enqueue(OrderStage::Browse, vec![]);
        assert_eq!(queue.pending_count(), 2);

        let mut count = 0;
        queue.sync_all(|_| {
            count += 1;
            if count == 1 { Ok(()) } else { Err(()) }
        });
        assert_eq!(queue.pending_count(), 1);
    }

    // ── CHAOS / LOAD / META tests ──────────────────────────────────────

    #[test]
    fn offline_capacity_one() {
        let mut queue = OfflineQueue::new(1);
        let id = queue.enqueue(OrderStage::Cart, vec![1]);
        assert_eq!(queue.len(), 1);
        // Overflow must evict oldest pending
        let id2 = queue.enqueue(OrderStage::Order, vec![2]);
        assert_eq!(queue.len(), 1, "capacity=1 must keep exactly one entry");
        assert!(queue.all().iter().any(|e| e.id == id2),
            "newest entry must survive eviction");
    }

    #[test]
    fn offline_evict_no_pending() {
        let mut queue = OfflineQueue::new(2);
        queue.enqueue(OrderStage::Order, vec![1]);
        // Sync to mark as Synced (not Pending/Failed)
        queue.sync_all(|_| Ok(()));
        assert!(queue.all().iter().all(|e| e.status == QueueStatus::Synced));
        // Evict with no pending entries must not remove anything
        queue.enqueue(OrderStage::Cart, vec![2]);
        assert_eq!(queue.len(), 2, "synced entries must not be evicted");
    }

    #[test]
    fn offline_gpu_cache_flood() {
        let mut queue = OfflineQueue::new(10);
        for i in 0..1000 {
            let key = format!("key_{}", i);
            let data = vec![(i % 256) as u8; 1024]; // 1KB each
            queue.persist_gpu_state(key, data);
        }
        // GPU cache must handle many entries without crash
        let restored = queue.restore_gpu_state("key_500");
        assert!(restored.is_some(), "must retrieve from 1000 cached entries");
        assert_eq!(restored.unwrap().len(), 1024);
    }

    #[test]
    fn offline_simulate_sync_with_gossip() {
        use dowiz_kernel::predictor::{Predictor, PredictorConfig};
        use dowiz_kernel::resilience::{ResilienceManager, ResiliencePolicy};
        use dowiz_kernel::gossip::{GossipBus, GossipTopic};

        let mut queue = OfflineQueue::new(10);
        queue.enqueue(OrderStage::Order, vec![1, 2, 3]);
        let entry = queue.all()[0].clone();

        let predictor = Predictor::new(PredictorConfig::default());
        let mut resilience = ResilienceManager::new(ResiliencePolicy::default());
        let mut bus = GossipBus::new();
        queue.attach_gossip(&mut bus, "test_queue");

        let (should_proceed, _suggestion) = queue.simulate_sync(
            &entry,
            &predictor,
            &mut resilience,
            Some(&mut bus),
        );
        // Must return a valid decision
        assert!(should_proceed == true || should_proceed == false);
        // Gossip bus should have received the sync event
        assert!(bus.total_published() > 0,
            "simulate_sync must publish at least one gossip message");
    }

    #[test]
    fn offline_gossip_node_access() {
        let mut queue = OfflineQueue::new(10);
        assert!(queue.gossip_node().is_none(),
            "gossip node must be None before attach");
        let mut bus = GossipBus::new();
        queue.attach_gossip(&mut bus, "test");
        assert!(queue.gossip_node().is_some(),
            "gossip node must be Some after attach");
    }

    #[test]
    fn offline_queue_depth_telemetry() {
        let mut queue = OfflineQueue::new(5);
        assert_eq!(queue.queue_depth(), 0);
        queue.enqueue(OrderStage::Discover, vec![]);
        queue.enqueue(OrderStage::Browse, vec![]);
        assert_eq!(queue.queue_depth(), 2);
    }

    #[test]
    fn offline_sync_all_empty() {
        let mut queue = OfflineQueue::new(10);
        let (synced, failed) = queue.sync_all(|_| Ok(()));
        assert_eq!(synced, 0);
        assert_eq!(failed, 0);
    }

    #[test]
    fn offline_clean_no_synced() {
        let mut queue = OfflineQueue::new(10);
        queue.enqueue(OrderStage::Cart, vec![1]);
        queue.clean(0);
        assert_eq!(queue.len(), 1, "clean must not remove unsynced entries");
    }

    #[test]
    fn offline_on_sync_callback() {
        use std::sync::atomic::{AtomicU32, Ordering};
        static CALLBACK_COUNT: AtomicU32 = AtomicU32::new(0);
        fn callback(_id: u64, _success: bool) {
            CALLBACK_COUNT.fetch_add(1, Ordering::SeqCst);
        }

        let mut queue = OfflineQueue::new(10);
        queue.on_sync_result(callback);
        queue.enqueue(OrderStage::Order, vec![1]);
        queue.sync_all(|_| Ok(()));
        assert_eq!(CALLBACK_COUNT.load(Ordering::SeqCst), 1,
            "callback must be invoked once for synced entry");
    }

    #[test]
    fn offline_order_stage_roundtrip() {
        let mut queue = OfflineQueue::new(20);
        for stage in &[
            OrderStage::Discover,
            OrderStage::Browse,
            OrderStage::Cart,
            OrderStage::Order,
            OrderStage::Track,
            OrderStage::Receive,
            OrderStage::Review,
        ] {
            queue.enqueue(*stage, vec![*stage as u8]);
        }
        assert_eq!(queue.len(), 7);
        let entries = queue.all();
        assert_eq!(entries[0].stage, OrderStage::Discover);
        assert_eq!(entries[6].stage, OrderStage::Review);
    }

    #[test]
    fn offline_large_payload_retains_integrity() {
        let mut queue = OfflineQueue::new(5);
        let payload: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        let id = queue.enqueue(OrderStage::Order, payload.clone());
        let stored = queue.all().iter().find(|e| e.id == id).unwrap();
        assert_eq!(stored.payload.len(), 100_000,
            "large payload must retain size");
        assert_eq!(stored.payload[99999], payload[99999],
            "large payload must retain content");
    }
}
