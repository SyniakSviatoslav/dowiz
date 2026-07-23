//! Satellite — інтеграція відкритих супутникових даних (Sentinel-2, OSM).
//!
//! Завантажує та кешує супутникові тайли для мапи. Підтримує:
//! - Sentinel-2 (ESA, 10m resolution)
//! - OpenStreetMap tiles (як fallback)
//! - Локальний кеш для офлайн-режиму
//!
//! DECART note: реальний HTTP-запит до ESA/OSM API потребує `ureq` або
//! wasm-шар (не кешовано, zero-dep mandate). До того — заглушка з чесним
//! `NotAvailable` + offline-degrade тести.
//! innovate: ceiling — реальний HTTP-клієнт після Decart-оцінки.

use crate::geo_map::MapTileKey;

/// Джерело супутникових даних.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SatelliteSource {
    Sentinel2,
    OSM,
    LocalCache,
}

impl SatelliteSource {
    pub fn name(&self) -> &'static str {
        match self {
            SatelliteSource::Sentinel2 => "sentinel-2",
            SatelliteSource::OSM => "osm",
            SatelliteSource::LocalCache => "local",
        }
    }
}

/// Результат запиту супутникового тайла.
#[derive(Debug, Clone)]
pub enum SatelliteResult {
    Available(SatelliteTile),
    NotAvailable,
    Loading,
}

/// Супутниковий тайл.
#[derive(Debug, Clone)]
pub struct SatelliteTile {
    pub key: MapTileKey,
    pub source: SatelliteSource,
    pub raster_data: Vec<u8>,
    pub captured_at: Option<u64>,
    pub cloud_cover_pct: Option<f32>,
}

impl SatelliteTile {
    pub fn new(key: MapTileKey, data: Vec<u8>) -> Self {
        SatelliteTile {
            key,
            source: SatelliteSource::LocalCache,
            raster_data: data,
            captured_at: None,
            cloud_cover_pct: None,
        }
    }
}

/// Статус з'єднання (віддзеркалює `environment::ConnectivityState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connectivity {
    Online,
    Offline,
}

/// Завантажувач супутникових тайлів.
///
/// - При `Online` — намагається завантажити з джерела (заглушка → `NotAvailable`).
/// - При `Offline` — тільки кеш.
/// - Кеш — кільцевий буфер (як `TileCache`).
#[derive(Debug, Clone)]
pub struct SatelliteLoader {
    source: SatelliteSource,
    tiles: Vec<SatelliteTile>,
    capacity: usize,
    connectivity: Connectivity,
}

impl SatelliteLoader {
    pub fn new(source: SatelliteSource, capacity: usize) -> Self {
        SatelliteLoader {
            source,
            tiles: Vec::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
            connectivity: Connectivity::Online,
        }
    }

    /// Встановити статус з'єднання.
    pub fn set_connectivity(&mut self, conn: Connectivity) {
        self.connectivity = conn;
    }

    /// Завантажити тайл.
    ///
    /// Повертає:
    /// - `Available(tile)` — з кешу або після успішного завантаження
    /// - `Loading` — запит в обробці (заглушка, не блокуюча)
    /// - `NotAvailable` — немає даних (offline або недоступне джерело)
    pub fn load(&mut self, key: &MapTileKey) -> SatelliteResult {
        if let Some(tile) = self.tiles.iter().find(|t| t.key == *key) {
            return SatelliteResult::Available(tile.clone());
        }
        match self.connectivity {
            Connectivity::Offline => SatelliteResult::NotAvailable,
            Connectivity::Online => {
                // Заглушка: реальний HTTP-запит буде додано після Decart-оцінки.
                // Наразі повертаємо Loading (симулюємо асинхронний запит).
                SatelliteResult::Loading
            }
        }
    }

    /// Зберегти завантажений тайл у кеш.
    pub fn cache_tile(&mut self, tile: SatelliteTile) {
        if self.tiles.len() >= self.capacity {
            self.tiles.remove(0);
        }
        self.tiles.push(tile);
    }

    /// Отримати всі кешовані тайли.
    pub fn cached_tiles(&self) -> &[SatelliteTile] {
        &self.tiles
    }

    /// Кількість кешованих тайлів.
    pub fn cache_len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_cache_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    /// Джерело.
    pub fn source(&self) -> SatelliteSource {
        self.source
    }

    /// Змінити джерело.
    pub fn set_source(&mut self, source: SatelliteSource) {
        self.source = source;
    }

    /// Сконвертувати супутниковий тайл у векторні фічі для мапи.
    ///
    /// У реальному застосуванні — растровий аналіз (NDVI, класифікація).
    /// Наразі — заглушка, яка повертає пустий список.
    pub fn raster_to_features(&self, _tile: &SatelliteTile) -> Vec<crate::geo_map::TileFeature> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satellite_tile_key_matches_map() {
        let key = MapTileKey::new(14, 8800, 5600);
        let tile = SatelliteTile::new(key, vec![0u8; 64]);
        assert_eq!(tile.key.z, 14);
        assert_eq!(tile.key.x, 8800);
        assert_eq!(tile.key.y, 5600);
    }

    #[test]
    fn offline_returns_not_available() {
        let mut loader = SatelliteLoader::new(SatelliteSource::Sentinel2, 10);
        loader.set_connectivity(Connectivity::Offline);
        let key = MapTileKey::new(14, 4400, 2800);
        match loader.load(&key) {
            SatelliteResult::NotAvailable => {},
            other => panic!("offline must return NotAvailable, got {:?}", other),
        }
    }

    #[test]
    fn online_returns_loading_stub() {
        let mut loader = SatelliteLoader::new(SatelliteSource::Sentinel2, 10);
        loader.set_connectivity(Connectivity::Online);
        let key = MapTileKey::new(14, 4400, 2800);
        match loader.load(&key) {
            SatelliteResult::Loading => {},
            other => panic!("online without cache must return Loading, got {:?}", other),
        }
    }

    #[test]
    fn cache_roundtrip() {
        let mut loader = SatelliteLoader::new(SatelliteSource::OSM, 10);
        let key = MapTileKey::new(14, 100, 200);
        let tile = SatelliteTile {
            key,
            source: SatelliteSource::OSM,
            raster_data: vec![1u8, 2, 3, 4],
            captured_at: Some(1000),
            cloud_cover_pct: Some(12.5),
        };
        loader.cache_tile(tile);

        match loader.load(&key) {
            SatelliteResult::Available(t) => {
                assert_eq!(t.raster_data, vec![1, 2, 3, 4]);
                assert_eq!(t.captured_at, Some(1000));
                assert_eq!(t.cloud_cover_pct, Some(12.5));
            },
            other => panic!("cached tile must be Available, got {:?}", other),
        }
    }

    #[test]
    fn cache_eviction() {
        let mut loader = SatelliteLoader::new(SatelliteSource::Sentinel2, 2);
        loader.set_connectivity(Connectivity::Offline);
        loader.cache_tile(SatelliteTile::new(MapTileKey::new(14, 1, 1), vec![1]));
        loader.cache_tile(SatelliteTile::new(MapTileKey::new(14, 2, 2), vec![2]));
        loader.cache_tile(SatelliteTile::new(MapTileKey::new(14, 3, 3), vec![3]));
        assert_eq!(loader.cache_len(), 2);
        assert!(matches!(loader.load(&MapTileKey::new(14, 1, 1)), SatelliteResult::NotAvailable));
    }

    #[test]
    fn source_names_are_displayable() {
        assert_eq!(SatelliteSource::Sentinel2.name(), "sentinel-2");
        assert_eq!(SatelliteSource::OSM.name(), "osm");
        assert_eq!(SatelliteSource::LocalCache.name(), "local");
    }

    #[test]
    fn raster_to_features_empty_stub() {
        let loader = SatelliteLoader::new(SatelliteSource::Sentinel2, 10);
        let tile = SatelliteTile::new(MapTileKey::new(14, 1, 1), vec![]);
        let features = loader.raster_to_features(&tile);
        assert!(features.is_empty(), "stub must return empty features");
    }

    #[test]
    fn cache_persists_after_source_change() {
        let mut loader = SatelliteLoader::new(SatelliteSource::OSM, 10);
        loader.cache_tile(SatelliteTile::new(MapTileKey::new(14, 5, 5), vec![42]));
        loader.set_source(SatelliteSource::Sentinel2);
        assert_eq!(loader.source(), SatelliteSource::Sentinel2);
        // Cached tiles survive source change
        let key = MapTileKey::new(14, 5, 5);
        match loader.load(&key) {
            SatelliteResult::Available(t) => assert_eq!(t.raster_data[0], 42),
            other => panic!("cache must survive source change, got {:?}", other),
        }
    }

    #[test]
    fn offline_then_online_recovers() {
        let mut loader = SatelliteLoader::new(SatelliteSource::Sentinel2, 10);
        loader.set_connectivity(Connectivity::Offline);
        let key = MapTileKey::new(14, 7, 7);
        assert!(matches!(loader.load(&key), SatelliteResult::NotAvailable));
        loader.set_connectivity(Connectivity::Online);
        assert!(matches!(loader.load(&key), SatelliteResult::Loading));
    }
}
