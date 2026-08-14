//! penecho.rs — PenEcho reimplementation: collaborative canvas with AI.
//!
//! Handwritten text, formulas, diagrams + spatial context.
//! Maps to kernel primitives: visual_index (PixelRAG tile management),
//! memory_search (section-based spatial search), parse (formula/text extraction).

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

/// Canvas tile — a region of the collaborative canvas.
#[derive(Debug, Clone)]
pub struct CanvasTile {
    /// Unique tile ID.
    pub id: u64,
    /// Tile position (x, y) in pixels.
    pub x: u64,
    pub y: u64,
    /// Tile size (width, height).
    pub width: u64,
    pub height: u64,
    /// Content type: text, formula, diagram, empty.
    pub content_type: TileContentType,
    /// Text content (for text tiles).
    pub text: Option<String>,
    /// Recognized formula (for formula tiles).
    pub formula: Option<String>,
    /// Confidence of recognition (0.0-1.0).
    pub confidence: f64,
    /// Spatial neighbors (adjacent tile IDs).
    pub neighbors: Vec<u64>,
}

/// Type of content on a canvas tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileContentType {
    Empty,
    Text,
    Formula,
    Diagram,
    Handwriting,
    Mixed,
}

/// Canvas region — a named spatial area on the canvas.
#[derive(Debug, Clone)]
pub struct CanvasRegion {
    /// Region name.
    pub name: String,
    /// Bounding box (x, y, width, height).
    pub x: u64,
    pub y: u64,
    pub width: u64,
    pub height: u64,
    /// Tags for the region.
    pub tags: Vec<String>,
}

/// The collaborative canvas.
pub struct PenEchoCanvas {
    /// Tiles indexed by tile ID.
    tiles: BTreeMap<u64, CanvasTile>,
    /// Regions on the canvas.
    regions: Vec<CanvasRegion>,
    /// Next tile ID.
    next_tile_id: u64,
    /// Canvas dimensions.
    canvas_width: u64,
    canvas_height: u64,
}

impl PenEchoCanvas {
    /// Create a new canvas with given dimensions.
    pub fn new(width: u64, height: u64) -> Self {
        PenEchoCanvas {
            tiles: BTreeMap::new(),
            regions: Vec::new(),
            next_tile_id: 0,
            canvas_width: width,
            canvas_height: height,
        }
    }

    /// Add a tile at the given position.
    pub fn add_tile(
        &mut self,
        x: u64,
        y: u64,
        width: u64,
        height: u64,
        content_type: TileContentType,
        text: Option<String>,
        formula: Option<String>,
        confidence: f64,
    ) -> u64 {
        let id = self.next_tile_id;
        self.next_tile_id += 1;

        let tile = CanvasTile {
            id,
            x,
            y,
            width,
            height,
            content_type,
            text,
            formula,
            confidence,
            neighbors: Vec::new(),
        };

        self.tiles.insert(id, tile);
        id
    }

    /// Add a region.
    pub fn add_region(&mut self, region: CanvasRegion) {
        self.regions.push(region);
    }

    /// Get a tile by ID.
    pub fn get_tile(&self, id: u64) -> Option<&CanvasTile> {
        self.tiles.get(&id)
    }

    /// Find tiles in a region.
    pub fn tiles_in_region(&self, region_name: &str) -> Vec<&CanvasTile> {
        let Some(region) = self.regions.iter().find(|r| r.name == region_name) else {
            return Vec::new();
        };
        self.tiles.values()
            .filter(|t| {
                t.x >= region.x && t.x < region.x + region.width
                    && t.y >= region.y && t.y < region.y + region.height
            })
            .collect()
    }

    /// Search tiles by text content.
    pub fn search_tiles_by_text(&self, query: &str) -> Vec<&CanvasTile> {
        let query_lower = query.to_lowercase();
        self.tiles.values()
            .filter(|t| {
                if let Some(ref text) = t.text {
                    text.to_lowercase().contains(&query_lower)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Search tiles by formula.
    pub fn search_tiles_by_formula(&self, query: &str) -> Vec<&CanvasTile> {
        let query_lower = query.to_lowercase();
        self.tiles.values()
            .filter(|t| {
                if let Some(ref formula) = t.formula {
                    formula.to_lowercase().contains(&query_lower)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Get all tiles of a specific content type.
    pub fn tiles_by_type(&self, content_type: TileContentType) -> Vec<&CanvasTile> {
        self.tiles.values()
            .filter(|t| t.content_type == content_type)
            .collect()
    }

    /// Get text tiles specifically.
    pub fn text_tiles(&self) -> Vec<&CanvasTile> {
        self.tiles_by_type(TileContentType::Text)
    }

    /// Get formula tiles specifically.
    pub fn formula_tiles(&self) -> Vec<&CanvasTile> {
        self.tiles_by_type(TileContentType::Formula)
    }

    /// Get handwriting tiles.
    pub fn handwriting_tiles(&self) -> Vec<&CanvasTile> {
        self.tiles_by_type(TileContentType::Handwriting)
    }

    /// Get the number of tiles.
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Get the number of regions.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Get all regions.
    pub fn regions(&self) -> &Vec<CanvasRegion> {
        &self.regions
    }

    /// Clear all tiles.
    pub fn clear(&mut self) {
        self.tiles.clear();
        self.next_tile_id = 0;
    }

    /// ASCII report.
    pub fn ascii_report(&self) -> String {
        let mut out = String::from("=== PenEcho Canvas Report ===\n");
        out.push_str(&format!(
            "Canvas: {}×{}, Tiles: {}, Regions: {}\n",
            self.canvas_width, self.canvas_height,
            self.tile_count(), self.region_count()
        ));

        out.push_str("\nRegions:\n");
        for region in &self.regions {
            out.push_str(&format!(
                "  {}: {}×{} at ({}, {}) — tags: {}\n",
                region.name, region.width, region.height,
                region.x, region.y, region.tags.join(", ")
            ));
        }

        out.push_str("\nContent summary:\n");
        out.push_str(&format!("  Text tiles: {}\n", self.text_tiles().len()));
        out.push_str(&format!("  Formula tiles: {}\n", self.formula_tiles().len()));
        out.push_str(&format!("  Handwriting tiles: {}\n", self.handwriting_tiles().len()));

        if !self.tiles.is_empty() {
            out.push_str("\nAll tiles:\n");
            for tile in self.tiles.values() {
                let content_type = match tile.content_type {
                    TileContentType::Empty => "empty",
                    TileContentType::Text => "text",
                    TileContentType::Formula => "formula",
                    TileContentType::Diagram => "diagram",
                    TileContentType::Handwriting => "handwriting",
                    TileContentType::Mixed => "mixed",
                };
                let extra = tile.text.as_ref()
                    .map(|t| format!(" \"{}\"", t.as_str()))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "  Tile #{} at ({}, {}): {} — conf={:.2}{}",
                    tile.id, tile.x, tile.y, content_type, tile.confidence, extra
                ));
                if let Some(ref f) = tile.formula {
                    out.push_str(&format!(" [formula: {}]", f));
                }
                out.push_str("\n");
            }
        }

        out.push_str("\n=== End Report ===\n");
        out
    }
}

impl Default for PenEchoCanvas {
    fn default() -> Self {
        Self::new(20000, 20000) // 20,000 × 20,000 — PenEcho's advertised canvas size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_canvas() -> PenEchoCanvas {
        PenEchoCanvas::new(1000, 1000)
    }

    #[test]
    fn new_canvas_has_no_tiles() {
        let c = make_canvas();
        assert_eq!(c.tile_count(), 0);
        assert_eq!(c.region_count(), 0);
    }

    #[test]
    fn add_tile_creates_tile() {
        let mut c = make_canvas();
        let id = c.add_tile(
            100, 200, 50, 50,
            TileContentType::Text,
            Some("Hello world".to_string()),
            None,
            0.95,
        );
        assert_eq!(id, 0);
        assert_eq!(c.tile_count(), 1);

        let tile = c.get_tile(id).unwrap();
        assert_eq!(tile.text, Some("Hello world".to_string()));
        assert_eq!(tile.confidence, 0.95);
    }

    #[test]
    fn add_region_and_query() {
        let mut c = make_canvas();
        c.add_region(CanvasRegion {
            name: "region1".to_string(),
            x: 0, y: 0,
            width: 500, height: 500,
            tags: vec!["notes".to_string()],
        });

        c.add_tile(100, 100, 20, 20, TileContentType::Text, Some("inside".to_string()), None, 1.0);
        c.add_tile(600, 600, 20, 20, TileContentType::Text, Some("outside".to_string()), None, 1.0);

        let tiles = c.tiles_in_region("region1");
        assert_eq!(tiles.len(), 1);
        assert_eq!(tiles[0].text, Some("inside".to_string()));
    }

    #[test]
    fn search_tiles_by_text() {
        let mut c = make_canvas();
        c.add_tile(0, 0, 10, 10, TileContentType::Text, Some("alpha beta".to_string()), None, 1.0);
        c.add_tile(0, 0, 10, 10, TileContentType::Text, Some("gamma delta".to_string()), None, 1.0);

        let results = c.search_tiles_by_text("alpha");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_tiles_by_formula() {
        let mut c = make_canvas();
        c.add_tile(0, 0, 10, 10, TileContentType::Formula, None, Some("E=mc^2".to_string()), 0.9);
        c.add_tile(0, 0, 10, 10, TileContentType::Formula, None, Some("a^2+b^2=c^2".to_string()), 0.95);

        let results = c.search_tiles_by_formula("mc");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn text_tiles_filtered() {
        let mut c = make_canvas();
        c.add_tile(0, 0, 10, 10, TileContentType::Text, Some("t1".to_string()), None, 1.0);
        c.add_tile(0, 0, 10, 10, TileContentType::Formula, None, Some("f1".to_string()), 1.0);
        c.add_tile(0, 0, 10, 10, TileContentType::Text, Some("t2".to_string()), None, 1.0);

        let texts = c.text_tiles();
        assert_eq!(texts.len(), 2);
    }

    #[test]
    fn formula_tiles_filtered() {
        let mut c = make_canvas();
        c.add_tile(0, 0, 10, 10, TileContentType::Formula, None, Some("x^2".to_string()), 1.0);
        c.add_tile(0, 0, 10, 10, TileContentType::Text, Some("text".to_string()), None, 1.0);
        c.add_tile(0, 0, 10, 10, TileContentType::Formula, None, Some("y^2".to_string()), 1.0);

        let formulas = c.formula_tiles();
        assert_eq!(formulas.len(), 2);
    }

    #[test]
    fn default_canvas_is_20000x20000() {
        let c = PenEchoCanvas::default();
        assert_eq!(c.canvas_width, 20000);
        assert_eq!(c.canvas_height, 20000);
    }

    #[test]
    fn ascii_report_format() {
        let c = make_canvas();
        let report = c.ascii_report();
        assert!(report.contains("PenEcho Canvas Report"));
        assert!(report.contains("Tiles: 0"));
    }

    #[test]
    fn clear_removes_all_tiles() {
        let mut c = make_canvas();
        c.add_tile(0, 0, 10, 10, TileContentType::Text, Some("x".to_string()), None, 1.0);
        c.add_tile(0, 0, 10, 10, TileContentType::Formula, None, Some("y".to_string()), 1.0);
        assert_eq!(c.tile_count(), 2);

        c.clear();
        assert_eq!(c.tile_count(), 0);
    }

    #[test]
    fn multiple_content_types() {
        let mut c = make_canvas();
        for &ct in &[TileContentType::Text, TileContentType::Formula,
                      TileContentType::Handwriting, TileContentType::Diagram,
                      TileContentType::Mixed] {
            c.add_tile(0, 0, 10, 10, ct, None, None, 1.0);
        }
        assert_eq!(c.tile_count(), 5);
    }
}
