//! MapEngine — гео-позиції, мапи, tile cache, WGSL-рендер.
//!
//! Відповідає за:
//! - Гео-обчислення (haversine, web mercator, tile координати) — ЧИСТО CPU
//! - Кеш тайлів мапи (кільцевий буфер)
//! - Підготовку даних для WGSL-шейдера (map.wgsl)
//!
//! GPU-рендер мапи виконується через WGSL compute shader, який отримує
//! дані тайлів з CPU у вигляді flat storage buffer. Сам engine не залежить
//! від wgpu (див. `bridge.rs` — це gpu-адаптер).
//!
//! FE-06 (bridge.rs::geo) вже надає базову geo-bridge. Цей модуль — надбудова
//! для мап: tile management, кешування, підготовка шейдерних буферів.

/// Гео-координата.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoCoord {
    pub lat: f64,
    pub lng: f64,
    pub alt: f64,
}

impl GeoCoord {
    pub fn new(lat: f64, lng: f64) -> Self {
        GeoCoord { lat, lng, alt: 0.0 }
    }

    pub fn with_alt(lat: f64, lng: f64, alt: f64) -> Self {
        GeoCoord { lat, lng, alt }
    }
}

/// Viewport мапи.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapViewport {
    pub center: GeoCoord,
    pub zoom: f32,
    pub bearing: f32,
    pub pitch: f32,
}

impl MapViewport {
    pub fn new(lat: f64, lng: f64, zoom: f32) -> Self {
        MapViewport {
            center: GeoCoord::new(lat, lng),
            zoom,
            bearing: 0.0,
            pitch: 0.0,
        }
    }
}

/// Ключ тайла мапи (z/x/y — Slippy Map standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapTileKey {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

impl MapTileKey {
    pub fn new(z: u32, x: u32, y: u32) -> Self {
        MapTileKey { z, x, y }
    }
}

/// Тип фічі на мапі.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureType {
    Road,
    Building,
    Park,
    Water,
}

/// Заголовок фічі тайла.
#[derive(Debug, Clone)]
pub struct TileFeature {
    pub feature_type: FeatureType,
    pub count: u32,
    pub coords: Vec<f64>,
}

/// Завантажений тайл.
#[derive(Debug, Clone)]
pub struct LoadedTile {
    pub key: MapTileKey,
    pub features: Vec<TileFeature>,
    pub byte_length: usize,
}

// ── Гео-обчислення ─────────────────────────────────────────────────────────
// Усі гео-примітиви (haversine, bearing, ETA) живуть в `kernel::geo`.
// Цей модуль містить ТІЛЬКИ tile-специфічні обчислення (mercator, tile keys),
// яких немає в kernel.

/// Радіус Землі (метри) — делегує kernel.
pub use dowiz_kernel::geo::haversine_meters as haversine_meters_raw;

/// Haversine відстань (метри) для `GeoCoord`. Делегує `kernel::geo::haversine_meters`.
pub fn haversine_distance_m(a: GeoCoord, b: GeoCoord) -> f64 {
    dowiz_kernel::geo::haversine_meters(a.lat, a.lng, b.lat, b.lng)
}

/// Розмір тайла в пікселях.
pub const TILE_SIZE: u32 = 256;

/// Web Mercator X для довготи.
pub fn web_mercator_x(lng: f64, zoom: f32) -> f64 {
    ((lng + 180.0) / 360.0) * (2.0_f64).powi(zoom as i32)
}

/// Web Mercator Y для широти.
pub fn web_mercator_y(lat: f64, zoom: f32) -> f64 {
    let lat_rad = lat.to_radians();
    let t = lat_rad.tan() + (1.0 / lat_rad.cos());
    (1.0 - t.ln() / std::f64::consts::PI) / 2.0 * (2.0_f64).powi(zoom as i32)
}

/// Перетворити (lng, lat) на tile key (z, x, y).
pub fn lon_lat_to_tile(lng: f64, lat: f64, zoom: u32) -> MapTileKey {
    let n = 2u64.pow(zoom);
    if n == 0 {
        return MapTileKey { z: zoom, x: 0, y: 0 };
    }
    let x = (web_mercator_x(lng, zoom as f32) as u64).min(n - 1);
    let y = (web_mercator_y(lat, zoom as f32) as u64).min(n - 1);
    MapTileKey { z: zoom, x: x as u32, y: y as u32 }
}

/// Межі тайла в градусах.
pub fn tile_bounds(key: MapTileKey) -> (f64, f64, f64, f64) {
    let n = 2u64.pow(key.z);
    let west = key.x as f64 / n as f64 * 360.0 - 180.0;
    let east = (key.x + 1) as f64 / n as f64 * 360.0 - 180.0;
    let north_rad = (std::f64::consts::PI * (1.0 - 2.0 * key.y as f64 / n as f64)).sinh().atan();
    let south_rad = (std::f64::consts::PI * (1.0 - 2.0 * (key.y + 1) as f64 / n as f64)).sinh().atan();
    (north_rad.to_degrees(), south_rad.to_degrees(), east, west)
}

/// Покриття тайлів навколо центру на заданому zoom.
/// Використовує `kernel::geo::haversine_meters` (канонічне джерело).
pub fn tile_coverage(center: GeoCoord, zoom: u32, radius_km: f64) -> Vec<MapTileKey> {
    let rad_m = 6_371_000.0; // Earth radius (для tile_coverage — локально, не дублює kernel)
    let d_lat = radius_km * 1000.0 / rad_m * (180.0 / std::f64::consts::PI);
    let d_lng = radius_km * 1000.0 / rad_m * (180.0 / std::f64::consts::PI)
        / center.lat.to_radians().cos();

    let north = (center.lat + d_lat).min(85.0511);
    let south = (center.lat - d_lat).max(-85.0511);
    let east = center.lng + d_lng;
    let west = center.lng - d_lng;

    let n = 2u64.pow(zoom);
    let x_min = (((west + 180.0) / 360.0) * n as f64).floor() as u64;
    let x_max = (((east + 180.0) / 360.0) * n as f64).ceil() as u64;
    let y_min = (web_mercator_y(north, zoom as f32)).floor() as u64;
    let y_max = (web_mercator_y(south, zoom as f32)).ceil() as u64;

    let mut tiles = Vec::new();
    for x in x_min..=x_max.min(n - 1) {
        for y in y_min..=y_max.min(n - 1) {
            tiles.push(MapTileKey::new(zoom, x as u32, y as u32));
        }
    }
    tiles
}

/// Завантажений тайл в плоскому форматі для GPU.
/// Кожен тайл — це header + координати: [type|count, lng1, lat1, lng2, lat2, ...]
/// type = 1(road), 2(building), 3(park), 4(water)
/// count = кількість точок
pub fn flatten_tile_for_gpu(tile: &LoadedTile) -> Vec<u32> {
    let mut data = Vec::new();
    for feature in &tile.features {
        let feature_type = match feature.feature_type {
            FeatureType::Road => 1u32,
            FeatureType::Building => 2u32,
            FeatureType::Park => 3u32,
            FeatureType::Water => 4u32,
        };
        // Header: type | (count << 16)
        data.push(feature_type | (feature.count << 16));
        for i in 0..feature.count as usize {
            // Координати як u32: lng*1e7, lat*1e7
            let lng_raw = (feature.coords[i * 2] * 10_000_000.0) as i32 as u32;
            let lat_raw = (feature.coords[i * 2 + 1] * 10_000_000.0) as i32 as u32;
            data.push(lng_raw);
            data.push(lat_raw);
        }
    }
    data
}

/// Tile cache — кільцевий буфер завантажених тайлів.
#[derive(Debug, Clone)]
pub struct TileCache {
    tiles: Vec<LoadedTile>,
    capacity: usize,
}

impl TileCache {
    pub fn new(capacity: usize) -> Self {
        TileCache {
            tiles: Vec::with_capacity(capacity),
            capacity: capacity.max(1),
        }
    }

    /// Додати тайл. При переповненні витісняється найстаріший.
    pub fn insert(&mut self, tile: LoadedTile) {
        if self.tiles.len() >= self.capacity {
            self.tiles.remove(0);
        }
        self.tiles.push(tile);
    }

    /// Знайти тайл за ключем.
    pub fn get(&self, key: &MapTileKey) -> Option<&LoadedTile> {
        self.tiles.iter().find(|t| t.key == *key)
    }

    /// Кількість завантажених тайлів.
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Очистити кеш.
    pub fn clear(&mut self) {
        self.tiles.clear();
    }

    /// Повернути всі ключі.
    pub fn keys(&self) -> Vec<MapTileKey> {
        self.tiles.iter().map(|t| t.key).collect()
    }

    /// Повернути всі тайли.
    pub fn all(&self) -> &[LoadedTile] {
        &self.tiles
    }
}

/// MapEngine — об'єднує гео-обчислення та кеш тайлів.
#[derive(Debug, Clone)]
pub struct MapEngine {
    pub viewport: MapViewport,
    pub cache: TileCache,
}

impl MapEngine {
    pub fn new(viewport: MapViewport, cache_capacity: usize) -> Self {
        MapEngine {
            viewport,
            cache: TileCache::new(cache_capacity),
        }
    }

    /// Отримати видимі тайли навколо центру viewport.
    pub fn visible_tiles(&self, range: u32) -> Vec<MapTileKey> {
        let zoom = self.viewport.zoom.round() as u32;
        let center_tile = lon_lat_to_tile(self.viewport.center.lng, self.viewport.center.lat, zoom);
        let n = 2u64.pow(zoom);
        let mut tiles = Vec::new();
        for dx in -(range as i64)..=range as i64 {
            for dy in -(range as i64)..=range as i64 {
                let x = ((center_tile.x as i64 + dx).rem_euclid(n as i64)) as u32;
                let y = ((center_tile.y as i64 + dy).rem_euclid(n as i64)) as u32;
                tiles.push(MapTileKey::new(zoom, x, y));
            }
        }
        tiles
    }

    /// Підготувати дані для WGSL шейдера мапи.
    ///
    /// Повертає: (viewport_uniforms_f32, tile_data_u32)
    /// viewport: [center_lat, center_lng, zoom, bearing, pitch, screen_w, screen_h, tile_size]
    pub fn prepare_gpu_data(&self, screen_w: f32, screen_h: f32) -> (Vec<f32>, Vec<u32>) {
        let uniforms = vec![
            self.viewport.center.lat as f32,
            self.viewport.center.lng as f32,
            self.viewport.zoom,
            self.viewport.bearing,
            self.viewport.pitch,
            screen_w,
            screen_h,
            TILE_SIZE as f32,
        ];

        let mut tile_data = Vec::new();
        for tile in self.cache.all() {
            let flat = flatten_tile_for_gpu(tile);
            let flat_len = flat.len();
            // Вирівнювання до 8192 байт (2048 u32) на тайл
            tile_data.extend(flat);
            let padding = (2048 - flat_len % 2048) % 2048;
            tile_data.extend(std::iter::repeat(0u32).take(padding));
        }

        (uniforms, tile_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kyiv() -> GeoCoord { GeoCoord::new(50.45, 30.52) }

    #[test]
    fn web_mercator_basic() {
        // Equator, prime meridian → 0.5, 0.5 at zoom 1
        // At zoom 1: 2^1 = 2 tiles. lng=0 → ((0+180)/360)*2 = 1.0
        let x = web_mercator_x(0.0, 1.0);
        let y = web_mercator_y(0.0, 1.0);
        assert!((x - 1.0).abs() < 1e-6, "x={}", x);
        assert!((y - 1.0).abs() < 1e-4, "y={}", y);
    }

    #[test]
    fn lon_lat_to_tile_kyiv() {
        let tile = lon_lat_to_tile(30.52, 50.45, 10);
        assert_eq!(tile.z, 10);
        assert!(tile.x > 0);
        assert!(tile.y > 0);
    }

    #[test]
    fn haversine_kyiv_to_lviv() {
        let kyiv = GeoCoord::new(50.45, 30.52);
        let lviv = GeoCoord::new(49.84, 24.03);
        let d = haversine_distance_m(kyiv, lviv);
        // ~470 km
        assert!((d - 470_000.0).abs() < 20_000.0, "distance={}", d);
    }

    #[test]
    fn haversine_zero_distance() {
        let p = GeoCoord::new(50.0, 30.0);
        let d = haversine_distance_m(p, p);
        assert!(d.abs() < 1e-6);
    }

    #[test]
    fn tile_coverage_returns_tiles() {
        let tiles = tile_coverage(kyiv(), 10, 5.0);
        assert!(!tiles.is_empty());
        assert!(tiles.len() >= 4, "coverage at zoom 10 should cover multiple tiles");
    }

    #[test]
    fn tile_cache_insert_and_retrieve() {
        let mut cache = TileCache::new(3);
        let key = MapTileKey::new(10, 500, 300);
        let tile = LoadedTile {
            key,
            features: vec![],
            byte_length: 0,
        };
        cache.insert(tile);
        assert_eq!(cache.len(), 1);
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn tile_cache_evicts_oldest() {
        let mut cache = TileCache::new(2);
        cache.insert(LoadedTile { key: MapTileKey::new(10, 1, 1), features: vec![], byte_length: 0 });
        cache.insert(LoadedTile { key: MapTileKey::new(10, 2, 2), features: vec![], byte_length: 0 });
        cache.insert(LoadedTile { key: MapTileKey::new(10, 3, 3), features: vec![], byte_length: 0 });
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&MapTileKey::new(10, 1, 1)).is_none());
        assert!(cache.get(&MapTileKey::new(10, 3, 3)).is_some());
    }

    #[test]
    fn visible_tiles_at_zoom() {
        let engine = MapEngine::new(MapViewport::new(50.45, 30.52, 13.0), 64);
        let tiles = engine.visible_tiles(2);
        // 5x5 grid = 25 tiles
        assert_eq!(tiles.len(), 25);
        assert!(tiles.iter().all(|t| t.z == 13));
    }

    #[test]
    fn flatten_tile_gpu_format() {
        let tile = LoadedTile {
            key: MapTileKey::new(10, 1, 1),
            features: vec![
                TileFeature {
                    feature_type: FeatureType::Road,
                    count: 2,
                    coords: vec![30.52, 50.45, 30.53, 50.46],
                },
            ],
            byte_length: 0,
        };
        let flat = flatten_tile_for_gpu(&tile);
        // Header: type(1) | (count << 16) = 1 | (2 << 16) = 131073
        assert_eq!(flat[0], 1 | (2 << 16));
        // 4 coordinates → 2 u32 coords = 2 u32 values
        assert_eq!(flat.len(), 5); // 1 header + 2*2 coords
    }

    #[test]
    fn gpu_data_preparation() {
        let viewport = MapViewport::new(50.45, 30.52, 13.0);
        let mut engine = MapEngine::new(viewport, 4);
        engine.cache.insert(LoadedTile {
            key: MapTileKey::new(13, 4400, 2800),
            features: vec![
                TileFeature { feature_type: FeatureType::Building, count: 1, coords: vec![30.52, 50.45] },
            ],
            byte_length: 0,
        });
        let (uniforms, tile_data) = engine.prepare_gpu_data(1920.0, 1080.0);
        assert_eq!(uniforms.len(), 8);
        assert!(!tile_data.is_empty());
    }

    #[test]
    fn tile_bounds_are_plausible() {
        let (n, s, e, w) = tile_bounds(MapTileKey::new(10, 500, 300));
        assert!(n > s, "north should be above south");
        assert!(e > w, "east should be east of west");
        assert!(n <= 90.0 && n >= -90.0);
        assert!(s <= 90.0 && s >= -90.0);
    }
}
