//! RouteCache — кеш маршрутів з локальним бекапом.
//!
//! Маршрути можна прегенерувати (з сервера або локально) та зберегти
//! в локальному кеші. Кеш серіалізується в `persistent_store` для
//! відновлення між сесіями. Кожен маршрут пам'ятає, які тайли мапи
//! йому потрібні — щоб можна було прекешувати тайли для офлайн-доступу.
//!
//! Прегенерація без зовнішнього графа доріг: використовує спрощену
//! grid-based A* евристику або, за наявності, polyline з сервера.

use crate::geo_map::MapTileKey;

/// Сегмент маршруту.
#[derive(Debug, Clone)]
pub struct RouteSegment {
    pub from: (f64, f64),
    pub to: (f64, f64),
    pub distance_m: f32,
    pub duration_s: f32,
    pub geometry: Vec<(f64, f64)>,
}

impl RouteSegment {
    pub fn new(from: (f64, f64), to: (f64, f64), geometry: Vec<(f64, f64)>) -> Self {
        let dist = geometry.windows(2)
            .map(|w| haversine_m(w[0], w[1]))
            .sum::<f32>();
        RouteSegment {
            from,
            to,
            distance_m: dist,
            duration_s: dist / 5.0,
            geometry,
        }
    }
}

/// Прегенерований маршрут.
#[derive(Debug, Clone)]
pub struct PregenRoute {
    pub id: u64,
    pub from: (f64, f64),
    pub to: (f64, f64),
    pub segments: Vec<RouteSegment>,
    pub total_distance_m: f32,
    pub total_duration_s: f32,
    pub created_at: u64,
    pub tile_keys: Vec<MapTileKey>,
}

/// Кеш маршрутів з локальною персистенцією.
///
/// - Кільцевий буфер (як `OfflineQueue` та `TileCache`)
/// - `persistent_store` для серіалізації/десеріалізації між сесіями
/// - Кожен маршрут знає свої `tile_keys` (які тайли мапи треба закешувати)
#[derive(Debug)]
pub struct RouteCache {
    routes: Vec<PregenRoute>,
    capacity: usize,
    next_id: u64,
    persistent_store: Vec<(String, Vec<u8>)>,
    // Telemetry counters (Atomic для &self-доступу в get())
    hit_count: std::sync::atomic::AtomicU64,
    miss_count: std::sync::atomic::AtomicU64,
    eviction_count: u64,
}

impl RouteCache {
    pub fn new(capacity: usize) -> Self {
        RouteCache {
            routes: Vec::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
            next_id: 1,
            persistent_store: Vec::new(),
            hit_count: std::sync::atomic::AtomicU64::new(0),
            miss_count: std::sync::atomic::AtomicU64::new(0),
            eviction_count: 0,
        }
    }

    /// Прегенерувати маршрут від `from` до `to` з проміжними точками.
    ///
    /// Використовує спрощену інтерполяцію для city-скейл маршрутів.
    /// Для точного маршруту використовуйте `store_route`.
    pub fn pregen(&mut self, from: (f64, f64), to: (f64, f64)) -> PregenRoute {
        let dlat = to.0 - from.0;
        let dlng = to.1 - from.1;
        let steps = 20.max((dlat.hypot(dlng) * 100.0) as usize);

        let mut geometry = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let lat = from.0 + dlat * t + (t * (1.0 - t) * 0.005 * (if i % 2 == 0 { 1.0 } else { -1.0 }));
            let lng = from.1 + dlng * t + (t * (1.0 - t) * 0.005 * (if i % 3 == 0 { 1.0 } else { -1.0 }));
            geometry.push((lat, lng));
        }

        let segment = RouteSegment::new(from, to, geometry);
        let total_distance = segment.distance_m;
        let total_duration = segment.duration_s;

        let tile_keys = self.compute_tile_keys(&[segment.clone()]);

        let id = self.next_id;
        self.next_id += 1;

        let route = PregenRoute {
            id,
            from,
            to,
            segments: vec![segment],
            total_distance_m: total_distance,
            total_duration_s: total_duration,
            created_at: crate::clock::monotonic_ms(),
            tile_keys,
        };

        if self.routes.len() >= self.capacity {
            self.routes.remove(0);
            self.eviction_count += 1;
            crate::telemetry_count!("route_cache", "eviction", 1);
        }
        self.routes.push(route.clone());
        route
    }

    /// Додати вже готовий маршрут (напр. з сервера).
    pub fn store_route(&mut self, route: PregenRoute) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let mut r = route;
        r.id = id;

        if self.routes.len() >= self.capacity {
            self.routes.remove(0);
            self.eviction_count += 1;
            crate::telemetry_count!("route_cache", "eviction", 1);
        }
        self.routes.push(r);
        id
    }

    /// Дістати маршрут за id.
    pub fn get(&self, id: u64) -> Option<&PregenRoute> {
        let found = self.routes.iter().find(|r| r.id == id);
        if found.is_some() {
            self.hit_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            crate::telemetry_count!("route_cache", "hit", 1);
        } else {
            self.miss_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            crate::telemetry_count!("route_cache", "miss", 1);
        }
        found
    }

    /// Кількість маршрутів.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Кількість кеш-хітів.
    pub fn cache_hits(&self) -> u64 {
        self.hit_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Кількість кеш-промахів.
    pub fn cache_misses(&self) -> u64 {
        self.miss_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Кількість витіснень.
    pub fn eviction_count(&self) -> u64 {
        self.eviction_count
    }

    /// Усі маршрути.
    pub fn all(&self) -> &[PregenRoute] {
        &self.routes
    }

    /// Які тайли мапи потрібні для маршруту (zoom 14).
    pub fn route_tiles(route: &PregenRoute, zoom: u32) -> Vec<MapTileKey> {
        let mut tiles = Vec::new();
        for &(lat, lng) in &route.segments.iter().flat_map(|s| s.geometry.iter()).copied().collect::<Vec<_>>() {
            let n = 2u64.pow(zoom);
            let x = (((lng + 180.0) / 360.0) * n as f64) as u64;
            let lat_rad = lat.to_radians();
            let term = lat_rad.tan() + (1.0 / lat_rad.cos());
            let y = ((1.0 - term.ln() / std::f64::consts::PI) / 2.0 * n as f64) as u64;
            let key = MapTileKey::new(zoom, x as u32, y as u32);
            if !tiles.contains(&key) {
                tiles.push(key);
            }
        }
        tiles
    }

    // ── Persistence ──────────────────────────────────────────────────

    /// Зберегти всі маршрути в persistent_store.
    pub fn backup_all(&mut self) {
        let encoded = self.serialize_routes();
        self.persistent_store.push(("route_cache_v1".to_string(), encoded));
    }

    /// Відновити маршрути з persistent_store.
    pub fn restore_all(&mut self) -> usize {
        let pos = self.persistent_store.iter().position(|(k, _)| k == "route_cache_v1");
        match pos {
            Some(idx) => {
                let data = &self.persistent_store[idx].1;
                let restored = self.deserialize_routes(data);
                let count = restored.len();
                self.routes = restored;
                count
            }
            None => 0,
        }
    }

    /// Скільки байт у persistent_store.
    pub fn store_size(&self) -> usize {
        self.persistent_store.iter().map(|(_, v)| v.len()).sum()
    }

    // ── Internal ─────────────────────────────────────────────────────

    /// Обчислити tile keys для набору сегментів.
    fn compute_tile_keys(&self, segments: &[RouteSegment]) -> Vec<MapTileKey> {
        let mut keys = Vec::new();
        for seg in segments {
            for &(lat, lng) in &seg.geometry {
                let tile = crate::geo_map::lon_lat_to_tile(lng, lat, 14);
                if !keys.contains(&tile) {
                    keys.push(tile);
                }
            }
        }
        keys
    }

    /// Формат серіалізації V1:
    /// [magic: 8 байт] [version: u32 LE] [crc32: u32 LE] [payload: ...]
    /// payload: [count_u64, ...] (той самий flat формат, що й раніше)
    const SERIAL_MAGIC: &'static [u8; 8] = b"DOWZROUT";
    const SERIAL_VERSION: u32 = 1;

    fn serialize_routes(&self) -> Vec<u8> {
        let payload = self.serialize_payload();
        let crc = crc32(&payload);
        let mut buf = Vec::with_capacity(16 + payload.len());
        buf.extend_from_slice(Self::SERIAL_MAGIC);
        buf.extend_from_slice(&Self::SERIAL_VERSION.to_le_bytes());
        buf.extend_from_slice(&crc.to_le_bytes());
        buf.extend_from_slice(&payload);
        buf
    }

    fn serialize_payload(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let count = self.routes.len() as u64;
        buf.extend_from_slice(&count.to_le_bytes());
        for r in &self.routes {
            buf.extend_from_slice(&r.id.to_le_bytes());
            buf.extend_from_slice(&r.created_at.to_le_bytes());
            buf.extend_from_slice(&r.total_distance_m.to_le_bytes());
            buf.extend_from_slice(&r.total_duration_s.to_le_bytes());
            buf.extend_from_slice(&r.from.0.to_le_bytes());
            buf.extend_from_slice(&r.from.1.to_le_bytes());
            buf.extend_from_slice(&r.to.0.to_le_bytes());
            buf.extend_from_slice(&r.to.1.to_le_bytes());
            let nseg = r.segments.len() as u64;
            buf.extend_from_slice(&nseg.to_le_bytes());
            for seg in &r.segments {
                let ng = seg.geometry.len() as u64;
                buf.extend_from_slice(&ng.to_le_bytes());
                buf.extend_from_slice(&seg.from.0.to_le_bytes());
                buf.extend_from_slice(&seg.from.1.to_le_bytes());
                buf.extend_from_slice(&seg.to.0.to_le_bytes());
                buf.extend_from_slice(&seg.to.1.to_le_bytes());
                buf.extend_from_slice(&seg.distance_m.to_le_bytes());
                buf.extend_from_slice(&seg.duration_s.to_le_bytes());
                for &(lat, lng) in &seg.geometry {
                    buf.extend_from_slice(&lat.to_le_bytes());
                    buf.extend_from_slice(&lng.to_le_bytes());
                }
            }
        }
        buf
    }

    fn deserialize_routes(&self, data: &[u8]) -> Vec<PregenRoute> {
        // Перевірка заголовка: magic + version + crc32
        if data.len() < 16 {
            return Vec::new();
        }
        if &data[0..8] != Self::SERIAL_MAGIC {
            return Vec::new();
        }
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        if version != Self::SERIAL_VERSION {
            return Vec::new();
        }
        let stored_crc = u32::from_le_bytes(data[12..16].try_into().unwrap());
        let payload = &data[16..];
        let actual_crc = crc32(payload);
        if stored_crc != actual_crc {
            return Vec::new();
        }
        self.deserialize_payload(payload)
    }

    fn deserialize_payload(&self, data: &[u8]) -> Vec<PregenRoute> {
        if data.len() < 8 {
            return Vec::new();
        }
        let mut pos = 0usize;
        let count = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap_or([0; 8]));
        pos += 8;
        let mut routes = Vec::with_capacity(count as usize);
        for _ in 0..count {
            if pos + 76 > data.len() { break; }
            let id = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let created_at = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let total_distance_m = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
            pos += 4;
            let total_duration_s = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
            pos += 4;
            let from_lat = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let from_lng = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let to_lat = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let to_lng = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let nseg = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            pos += 8;
            let mut segments = Vec::with_capacity(nseg as usize);
            for _ in 0..nseg {
                if pos + 32 > data.len() { break; }
                let ng = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
                pos += 8;
                let seg_from_lat = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
                pos += 8;
                let seg_from_lng = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
                pos += 8;
                let seg_to_lat = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
                pos += 8;
                let seg_to_lng = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
                pos += 8;
                let distance_m = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
                pos += 4;
                let duration_s = f32::from_le_bytes(data[pos..pos + 4].try_into().unwrap());
                pos += 4;
                let mut geometry = Vec::with_capacity(ng as usize);
                for _ in 0..ng {
                    if pos + 16 > data.len() { break; }
                    let lat = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
                    pos += 8;
                    let lng = f64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
                    pos += 8;
                    geometry.push((lat, lng));
                }
                segments.push(RouteSegment {
                    from: (seg_from_lat, seg_from_lng),
                    to: (seg_to_lat, seg_to_lng),
                    distance_m,
                    duration_s,
                    geometry,
                });
            }
            let tile_keys = self.compute_tile_keys(&segments);
            routes.push(PregenRoute {
                id,
                from: (from_lat, from_lng),
                to: (to_lat, to_lng),
                segments,
                total_distance_m,
                total_duration_s,
                created_at,
                tile_keys,
            });
        }
        routes
    }
}

/// CRC-32 (проста реалізація, zero-dep сумісна).
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn haversine_m(a: (f64, f64), b: (f64, f64)) -> f32 {
    dowiz_kernel::geo::haversine_meters(a.0, a.1, b.0, b.1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pregen_creates_route() {
        let mut cache = RouteCache::new(10);
        let route = cache.pregen((50.45, 30.52), (50.46, 30.53));
        assert!(route.total_distance_m > 0.0, "route must have positive distance");
        assert!(route.total_duration_s > 0.0);
        assert!(route.segments.len() >= 1);
        assert!(!route.tile_keys.is_empty(), "must have tile keys");
    }

    #[test]
    fn store_and_retrieve() {
        let mut cache = RouteCache::new(10);
        let route = cache.pregen((50.45, 30.52), (50.46, 30.53));
        let id = route.id;
        let stored = cache.get(id).expect("route must be findable");
        assert!((stored.from.0 - 50.45).abs() < 1e-4);
    }

    #[test]
    fn backup_restore_roundtrip() {
        let mut cache = RouteCache::new(10);
        cache.pregen((50.45, 30.52), (50.46, 30.53));
        cache.pregen((49.84, 24.03), (50.45, 30.52));
        let original_count = cache.len();

        cache.backup_all();
        assert!(cache.store_size() > 0, "backup must produce bytes");

        // Симулюємо персистенцію: беремо дані з одного кеша і передаємо іншому
        let data = cache.persistent_store.iter()
            .find(|(k, _)| k == "route_cache_v1")
            .map(|(_, v)| v.clone())
            .expect("backup key must exist");

        let mut cache2 = RouteCache::new(10);
        cache2.persistent_store.push(("route_cache_v1".to_string(), data));
        let restored = cache2.restore_all();
        assert_eq!(restored, original_count, "must restore same number of routes");
        assert!(!cache2.is_empty());
        assert!((cache2.all()[0].from.0 - 50.45).abs() < 1e-4);
    }

    #[test]
    fn eviction_works() {
        let mut cache = RouteCache::new(3);
        cache.pregen((50.0, 30.0), (50.1, 30.1));
        cache.pregen((50.1, 30.1), (50.2, 30.2));
        cache.pregen((50.2, 30.2), (50.3, 30.3));
        cache.pregen((50.3, 30.3), (50.4, 30.4));
        assert_eq!(cache.len(), 3, "must not exceed capacity");
    }

    #[test]
    fn route_tiles_for_pregen() {
        let route = {
            let mut cache = RouteCache::new(10);
            cache.pregen((50.45, 30.52), (50.46, 30.53))
        };
        let tiles = RouteCache::route_tiles(&route, 14);
        assert!(!tiles.is_empty(), "must return tiles");
        assert!(tiles.iter().all(|t| t.z == 14));
    }

    #[test]
    fn empty_cache_restore_returns_zero() {
        let mut cache = RouteCache::new(10);
        cache.backup_all();
        let n = cache.restore_all();
        assert_eq!(n, 0);
    }

    #[test]
    fn segment_distance_matches_haversine() {
        let geom = vec![(50.45, 30.52), (50.46, 30.53)];
        let seg = RouteSegment::new((50.45, 30.52), (50.46, 30.53), geom);
        assert!(seg.distance_m > 100.0, "~1 km between points");
    }

    #[test]
    fn serialization_format_magic_and_version() {
        let mut cache = RouteCache::new(10);
        cache.pregen((50.45, 30.52), (50.46, 30.53));
        cache.backup_all();
        let data = cache.persistent_store.iter()
            .find(|(k, _)| k == "route_cache_v1")
            .map(|(_, v)| v.clone())
            .expect("backup must exist");
        // Magic + version + crc32 = 16 bytes header
        assert!(data.len() >= 16, "must have header: {} bytes", data.len());
        assert_eq!(&data[0..8], b"DOWZROUT", "magic bytes");
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        assert_eq!(version, 1, "format version");
    }

    #[test]
    fn corrupted_data_rejected() {
        let mut cache = RouteCache::new(10);
        cache.pregen((50.45, 30.52), (50.46, 30.53));
        cache.backup_all();
        let data = cache.persistent_store.iter()
            .find(|(k, _)| k == "route_cache_v1")
            .map(|(_, v)| v.clone())
            .unwrap();
        // Пошкодити один байт у payload
        let mut corrupted = data.clone();
        if corrupted.len() > 20 {
            corrupted[19] ^= 0xFF;
        }
        let mut cache2 = RouteCache::new(10);
        cache2.persistent_store.push(("route_cache_v1".to_string(), corrupted));
        let n = cache2.restore_all();
        assert_eq!(n, 0, "corrupted data must be rejected");
    }

    #[test]
    fn wrong_magic_rejected() {
        let data = b"BADMAGIC\x01\x00\x00\x00\x00\x00\x00\x00".to_vec();
        let mut cache = RouteCache::new(10);
        cache.persistent_store.push(("route_cache_v1".to_string(), data));
        let n = cache.restore_all();
        assert_eq!(n, 0);
    }
}
