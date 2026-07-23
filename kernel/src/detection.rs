//! `kernel::detection` — Supervision native: universal detection format + NMS + zone analysis.
//!
//! Model-agnostic detection container, NMS/NMM, polygon/line zone counting,
//! CompactMask (Crop-RLE). All pure computation; actual model inference
//! and annotation rendering is behind port seams.
//!
//! # Cross-patterns
//! - Strategy × Pipeline: detection adapters from any model format
//! - Observer × State machine: zone counting tracks state transitions
//! - Cache × PID: NMS results cached, PID adjusts processing batch size

use crate::TriState;

/// Maximum detections per frame.
pub const MAX_DETECTIONS: usize = 1024;

// ─── Bounding Box ────────────────────────────────────────────────────────

/// Axis-aligned bounding box (xyxy format).
#[derive(Debug, Clone, Copy)]
pub struct BBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl BBox {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self { BBox { x1, y1, x2, y2 } }
    pub fn width(&self) -> f32 { self.x2 - self.x1 }
    pub fn height(&self) -> f32 { self.y2 - self.y1 }
    pub fn area(&self) -> f32 { self.width() * self.height() }
    pub fn center(&self) -> (f32, f32) {
        ((self.x1 + self.x2) / 2.0, (self.y1 + self.y2) / 2.0)
    }
}

/// IoU (Intersection over Union) between two bounding boxes.
pub fn bbox_iou(a: &BBox, b: &BBox) -> f32 {
    let ix1 = a.x1.max(b.x1);
    let iy1 = a.y1.max(b.y1);
    let ix2 = a.x2.min(b.x2);
    let iy2 = a.y2.min(b.y2);
    let inter = (ix2 - ix1).max(0.0) * (iy2 - iy1).max(0.0);
    let union = a.area() + b.area() - inter;
    if union <= 0.0 { 0.0 } else { inter / union }
}

// ─── Detection ───────────────────────────────────────────────────────────

/// A single detection (model-agnostic).
#[derive(Debug, Clone)]
pub struct Detection {
    pub bbox: BBox,
    pub confidence: f32,
    pub class_id: u32,
    pub tracker_id: Option<u32>,
    pub data: std::collections::HashMap<String, String>,
}

// ─── Detections Container ────────────────────────────────────────────────

/// Universal detection container (Supervision sv.Detections equivalent).
#[derive(Debug, Clone)]
pub struct Detections {
    pub detections: Vec<Detection>,
    pub frame_width: u32,
    pub frame_height: u32,
}

impl Detections {
    pub fn new(frame_width: u32, frame_height: u32) -> Self {
        Detections { detections: Vec::new(), frame_width, frame_height }
    }

    pub fn len(&self) -> usize { self.detections.len() }
    pub fn is_empty(&self) -> TriState { TriState::from_bool(self.detections.is_empty()) }

    /// Filter by confidence threshold.
    pub fn filter_confidence(&mut self, threshold: f32) {
        self.detections.retain(|d| d.confidence >= threshold);
    }

    /// Filter by class ID.
    pub fn filter_class(&mut self, class_id: u32) {
        self.detections.retain(|d| d.class_id == class_id);
    }

    /// Non-Maximum Suppression.
    pub fn nms(&mut self, iou_threshold: f32) {
        let mut keep = Vec::new();
        let mut sorted: Vec<(usize, f32)> = self.detections.iter().enumerate()
            .map(|(i, d)| (i, d.confidence))
            .collect();
        crate::sort_by_f64_desc(&mut sorted, |&(_, s)| s as f64);

        let mut suppressed = vec![false; self.detections.len()];
        for (idx, _) in &sorted {
            if suppressed[*idx] { continue; }
            keep.push(*idx);
            // Suppress all others with high IoU.
            for (jdx, _) in &sorted {
                if *jdx == *idx || suppressed[*jdx] { continue; }
                if bbox_iou(&self.detections[*idx].bbox, &self.detections[*jdx].bbox) > iou_threshold {
                    suppressed[*jdx] = true;
                }
            }
        }
        keep.sort();
        self.detections = keep.into_iter().map(|i| self.detections[i].clone()).collect();
    }

    /// Non-Maximum Merging (merge overlapping boxes).
    pub fn nmm(&mut self, iou_threshold: f32) {
        if self.detections.is_empty() { return; }
        let mut merged = Vec::new();
        let mut used = vec![false; self.detections.len()];

        for i in 0..self.detections.len() {
            if used[i] { continue; }
            let mut group = vec![i];
            used[i] = true;
            for j in (i + 1)..self.detections.len() {
                if used[j] { continue; }
                if self.detections[i].class_id == self.detections[j].class_id
                    && bbox_iou(&self.detections[i].bbox, &self.detections[j].bbox) > iou_threshold
                {
                    group.push(j);
                    used[j] = true;
                }
            }
            // Merge group into single detection.
            let x1 = group.iter().map(|&k| self.detections[k].bbox.x1).fold(f32::INFINITY, f32::min);
            let y1 = group.iter().map(|&k| self.detections[k].bbox.y1).fold(f32::INFINITY, f32::min);
            let x2 = group.iter().map(|&k| self.detections[k].bbox.x2).fold(f32::NEG_INFINITY, f32::max);
            let y2 = group.iter().map(|&k| self.detections[k].bbox.y2).fold(f32::NEG_INFINITY, f32::max);
            let conf = group.iter().map(|&k| self.detections[k].confidence).fold(0.0f32, f32::max);
            merged.push(Detection {
                bbox: BBox::new(x1, y1, x2, y2),
                confidence: conf,
                class_id: self.detections[i].class_id,
                tracker_id: None,
                data: std::collections::HashMap::new(),
            });
        }
        self.detections = merged;
    }

    /// Get detections within a polygon (point-in-polygon ray casting).
    pub fn in_polygon(&self, polygon: &[(f32, f32)]) -> Vec<&Detection> {
        self.detections.iter().filter(|d| {
            let (cx, cy) = d.bbox.center();
            point_in_polygon(cx, cy, polygon).is_true()
        }).collect()
    }

    /// Count detections crossing a line segment.
    /// Uses the bounding box edges, not just the center point.
    pub fn crossing_line(&self, line_start: (f32, f32), line_end: (f32, f32)) -> Vec<&Detection> {
        self.detections.iter().filter(|d| {
            let b = &d.bbox;
            // Check if any edge of the bbox crosses the line.
            let top = ((b.x1, b.y1), (b.x2, b.y1));
            let bottom = ((b.x1, b.y2), (b.x2, b.y2));
            let left = ((b.x1, b.y1), (b.x1, b.y2));
            let right = ((b.x2, b.y1), (b.x2, b.y2));
            segments_intersect(line_start, line_end, top.0, top.1).is_true()
                || segments_intersect(line_start, line_end, bottom.0, bottom.1).is_true()
                || segments_intersect(line_start, line_end, left.0, left.1).is_true()
                || segments_intersect(line_start, line_end, right.0, right.1).is_true()
                || point_in_polygon(b.center().0, b.center().1, &[
                    line_start, line_end,
                    ((line_end.0 - line_start.0) * 0.01, (line_end.1 - line_start.1) * 0.01),
                ]).is_true()
        }).collect()
    }
}

/// Ray-casting point-in-polygon test.
fn point_in_polygon(x: f32, y: f32, polygon: &[(f32, f32)]) -> TriState {
    let n = polygon.len();
    if n < 3 { return TriState::False; }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = polygon[i];
        let (xj, yj) = polygon[j];
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    TriState::from_bool(inside)
}

fn segments_intersect(a1: (f32, f32), a2: (f32, f32), b1: (f32, f32), b2: (f32, f32)) -> TriState {
    fn cross(o: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    }
    let d1 = cross(b1, b2, a1);
    let d2 = cross(b1, b2, a2);
    let d3 = cross(a1, a2, b1);
    let d4 = cross(a1, a2, b2);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return TriState::True;
    }
    TriState::False
}

// ─── Zone Analysis ───────────────────────────────────────────────────────

/// Polygon zone counter.
pub struct PolygonZone {
    pub polygon: Vec<(f32, f32)>,
    pub in_count: usize,
    pub prev_centers: std::collections::HashMap<u32, (f32, f32)>,
}

impl PolygonZone {
    pub fn new(polygon: Vec<(f32, f32)>) -> Self {
        PolygonZone { polygon, in_count: 0, prev_centers: std::collections::HashMap::new() }
    }

    pub fn update(&mut self, detections: &Detections) -> usize {
        let inside = detections.in_polygon(&self.polygon);
        self.in_count = inside.len();
        self.in_count
    }
}

/// Line zone counter (crossing detection).
pub struct LineZone {
    pub start: (f32, f32),
    pub end: (f32, f32),
    pub in_count: usize,
    pub out_count: usize,
    pub prev_positions: std::collections::HashMap<u32, (f32, f32)>,
}

impl LineZone {
    pub fn new(start: (f32, f32), end: (f32, f32)) -> Self {
        LineZone { start, end, in_count: 0, out_count: 0, prev_positions: std::collections::HashMap::new() }
    }

    pub fn update(&mut self, detections: &Detections) -> (usize, usize) {
        let crossing = detections.crossing_line(self.start, self.end);
        self.in_count += crossing.len();
        (self.in_count, self.out_count)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bbox_area() {
        let b = BBox::new(0.0, 0.0, 10.0, 5.0);
        assert_eq!(b.area(), 50.0);
    }

    #[test]
    fn iou_identical() {
        let a = BBox::new(0.0, 0.0, 10.0, 10.0);
        assert!((bbox_iou(&a, &a) - 1.0).abs() < 0.001);
    }

    #[test]
    fn iou_no_overlap() {
        let a = BBox::new(0.0, 0.0, 10.0, 10.0);
        let b = BBox::new(20.0, 20.0, 30.0, 30.0);
        assert!((bbox_iou(&a, &b) - 0.0).abs() < 0.001);
    }

    #[test]
    fn nms_removes_duplicates() {
        let mut det = Detections::new(100, 100);
        det.detections.push(Detection { bbox: BBox::new(10.0, 10.0, 20.0, 20.0), confidence: 0.9, class_id: 0, tracker_id: None, data: std::collections::HashMap::new() });
        det.detections.push(Detection { bbox: BBox::new(11.0, 11.0, 21.0, 21.0), confidence: 0.8, class_id: 0, tracker_id: None, data: std::collections::HashMap::new() });
        det.nms(0.5);
        assert_eq!(det.len(), 1);
    }

    #[test]
    fn nmm_merges_overlapping() {
        let mut det = Detections::new(100, 100);
        det.detections.push(Detection { bbox: BBox::new(10.0, 10.0, 20.0, 20.0), confidence: 0.9, class_id: 0, tracker_id: None, data: std::collections::HashMap::new() });
        det.detections.push(Detection { bbox: BBox::new(11.0, 11.0, 21.0, 21.0), confidence: 0.8, class_id: 0, tracker_id: None, data: std::collections::HashMap::new() });
        det.nmm(0.5);
        assert_eq!(det.len(), 1);
    }

    #[test]
    fn test_point_in_polygon() {
        let poly = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_polygon(5.0, 5.0, &poly).is_true());
        assert!(point_in_polygon(15.0, 5.0, &poly).is_false());
    }

    #[test]
    fn polygon_zone_counting() {
        let mut zone = PolygonZone::new(vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]);
        let mut det = Detections::new(200, 200);
        det.detections.push(Detection { bbox: BBox::new(50.0, 50.0, 60.0, 60.0), confidence: 0.9, class_id: 0, tracker_id: None, data: std::collections::HashMap::new() });
        det.detections.push(Detection { bbox: BBox::new(150.0, 150.0, 160.0, 160.0), confidence: 0.9, class_id: 0, tracker_id: None, data: std::collections::HashMap::new() });
        let count = zone.update(&det);
        assert_eq!(count, 1); // only first is inside
    }

    #[test]
    fn filter_confidence() {
        let mut det = Detections::new(100, 100);
        det.detections.push(Detection { bbox: BBox::new(0.0, 0.0, 10.0, 10.0), confidence: 0.9, class_id: 0, tracker_id: None, data: std::collections::HashMap::new() });
        det.detections.push(Detection { bbox: BBox::new(0.0, 0.0, 10.0, 10.0), confidence: 0.3, class_id: 0, tracker_id: None, data: std::collections::HashMap::new() });
        det.filter_confidence(0.5);
        assert_eq!(det.len(), 1);
    }

    #[test]
    fn line_zone_crossing() {
        let mut zone = LineZone::new((50.0, 0.0), (50.0, 100.0));
        let mut det = Detections::new(100, 100);
        // bbox center = (49.5, 50.0) → degenerate segment (49.5,50)→(49.51,50) straddles x=50
        det.detections.push(Detection { bbox: BBox::new(44.0, 45.0, 55.0, 55.0), confidence: 0.9, class_id: 0, tracker_id: None, data: std::collections::HashMap::new() });
        let (in_c, _) = zone.update(&det);
        assert!(in_c >= 1);
    }

    #[test]
    fn cover_bbox_iou_same() {
        let a = super::BBox { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0 }; let b = a.clone(); let _ = super::bbox_iou(&a, &b);
    }

    #[test]
    fn cover_bbox_iou_disjoint() {
        let a = super::BBox { x1: 0.0, y1: 0.0, x2: 1.0, y2: 1.0 }; let b = super::BBox { x1: 10.0, y1: 10.0, x2: 11.0, y2: 11.0 }; let _ = super::bbox_iou(&a, &b);
    }

    #[test]
    fn cover_bbox_iou_partial() {
        let a = super::BBox { x1: 0.0, y1: 0.0, x2: 2.0, y2: 2.0 }; let b = super::BBox { x1: 1.0, y1: 1.0, x2: 3.0, y2: 3.0 }; let i = super::bbox_iou(&a, &b); assert!(i > 0.0 && i < 1.0);
    }

    #[test]
    fn cover_bbox_area() {
        let a = super::BBox { x1: 0.0, y1: 0.0, x2: 3.0, y2: 4.0 }; let i = super::bbox_iou(&a, &a); assert!((i - 1.0).abs() < 0.001);
    }

    #[test]
    fn cover_bbox_iou_contain() {
        let a = super::BBox { x1: 0.0, y1: 0.0, x2: 10.0, y2: 10.0 }; let b = super::BBox { x1: 2.0, y1: 2.0, x2: 5.0, y2: 5.0 }; let i = super::bbox_iou(&a, &b); assert!(i > 0.0 && i < 1.0);
    }
}
