//! oil_motion.rs — Oil Motion reimplementation: interactive animation pipeline.
//!
//! Keyframe → motion → web pipeline with scroll/drag/touch binding.
//! Maps to kernel primitives: parallel_patterns (Pipeline for stages),
//! agent_browser (anti-detect web rendering config), spectral (keyframe interpolation).

use alloc::collections::BTreeMap;

/// Animation keyframe — a snapshot of animation state at a point in time.
#[derive(Debug, Clone)]
pub struct Keyframe {
    /// Keyframe index (0-based position in sequence).
    pub index: usize,
    /// Time offset (milliseconds from start).
    pub time_ms: u64,
    /// Property values at this keyframe.
    pub properties: BTreeMap<String, f64>,
}

/// Animation action — what triggers the animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AnimationAction {
    Scroll,
    Drag,
    Touch,
    Orientation,
    MouseMove,
}

/// Animation pipeline stage — one phase of the keyframe→motion→web pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PipelineStage {
    /// Keyframe extraction and validation.
    KeyframeExtraction,
    /// Motion generation between keyframes.
    MotionGeneration,
    /// Web motion compilation (CSS/JS output).
    WebCompilation,
}

/// The animation pipeline — orchestrates the three stages.
pub struct OilMotionPipeline {
    /// Registered keyframes in order.
    keyframes: Vec<Keyframe>,
    /// Pipeline stage configurations.
    stages: BTreeMap<PipelineStage, StageConfig>,
    /// Action bindings — which actions trigger which animations.
    action_bindings: BTreeMap<AnimationAction, Vec<String>>,
}

/// Stage configuration.
#[derive(Debug, Clone)]
pub struct StageConfig {
    pub enabled: bool,
    pub params: BTreeMap<String, String>,
}

impl OilMotionPipeline {
    /// Create a new pipeline.
    pub fn new() -> Self {
        let mut stages = BTreeMap::new();
        stages.insert(PipelineStage::KeyframeExtraction, StageConfig {
            enabled: true,
            params: BTreeMap::new(),
        });
        stages.insert(PipelineStage::MotionGeneration, StageConfig {
            enabled: true,
            params: BTreeMap::new(),
        });
        stages.insert(PipelineStage::WebCompilation, StageConfig {
            enabled: true,
            params: BTreeMap::new(),
        });

        OilMotionPipeline {
            keyframes: Vec::new(),
            stages,
            action_bindings: BTreeMap::new(),
        }
    }

    /// Add a keyframe.
    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by_key(|k| k.index);
    }

    /// Bind an action to an animation name.
    pub fn bind_action(&mut self, action: AnimationAction, animation_name: &str) {
        self.action_bindings
            .entry(action)
            .or_default()
            .push(animation_name.to_string());
    }

    /// Get keyframes.
    pub fn keyframes(&self) -> &Vec<Keyframe> {
        &self.keyframes
    }

    /// Interpolate between two keyframes at a given time.
    pub fn interpolate(&self, t_ms: u64) -> Option<BTreeMap<String, f64>> {
        if self.keyframes.len() < 2 {
            return None;
        }

        let mut prev = &self.keyframes[0];
        let mut next = &self.keyframes[1];

        for i in 1..self.keyframes.len() {
            if self.keyframes[i].time_ms >= t_ms {
                next = &self.keyframes[i];
                if i > 0 {
                    prev = &self.keyframes[i - 1];
                }
                break;
            }
            prev = &self.keyframes[i];
        }

        if prev.time_ms == next.time_ms {
            return Some(prev.properties.clone());
        }

        let ratio = (t_ms - prev.time_ms) as f64 / (next.time_ms - prev.time_ms) as f64;
        let clamped_ratio = ratio.max(0.0).min(1.0);

        let mut result = BTreeMap::new();
        for (key, &prev_val) in &prev.properties {
            if let Some(&next_val) = next.properties.get(key) {
                result.insert(key.clone(), prev_val + (next_val - prev_val) * clamped_ratio);
            }
        }
        Some(result)
    }

    /// Generate CSS animation from keyframes.
    pub fn generate_css(&self, animation_name: &str) -> String {
        if self.keyframes.is_empty() {
            return String::new();
        }

        let mut css = format!("@keyframes {} {{\n", animation_name);
        for kf in &self.keyframes {
            css.push_str(&format!("  {}% {{\n", kf.time_ms as f64 / 10.0));
            for (prop, val) in &kf.properties {
                css.push_str(&format!("    {} : {};\n", prop, val));
            }
            css.push_str("  }\n");
        }
        css.push_str("}\n");
        css
    }

    /// Generate web motion config (JSON-like structure).
    pub fn generate_web_config(&self) -> String {
        let mut config = String::from("{\n");
        config.push_str(&format!("  \"keyframes\": {},\n", self.keyframes.len()));
        config.push_str(&format!("  \"actions\": {},\n", self.action_bindings.len()));

        let mut actions = String::from("    \"actions\": [\n");
        for (action, animations) in &self.action_bindings {
            actions.push_str(&format!(
                "      {{\"type\": \"{}\", \"animations\": [{}]}},\n",
                match action {
                    AnimationAction::Scroll => "scroll",
                    AnimationAction::Drag => "drag",
                    AnimationAction::Touch => "touch",
                    AnimationAction::Orientation => "orientation",
                    AnimationAction::MouseMove => "mousemove",
                },
                animations.iter().map(|a| format!("\"{}\"", a)).collect::<Vec<_>>().join(", ")
            ));
        }
        actions.push_str("    ]\n");
        config.push_str(&actions);
        config.push_str("}\n");
        config
    }

    /// Check if a stage is enabled.
    pub fn stage_enabled(&self, stage: PipelineStage) -> bool {
        self.stages.get(&stage).map(|s| s.enabled).unwrap_or(false)
    }

    /// Set stage enabled.
    pub fn set_stage(&mut self, stage: PipelineStage, enabled: bool) {
        if let Some(cfg) = self.stages.get_mut(&stage) {
            cfg.enabled = enabled;
        }
    }

    /// Get the number of keyframes.
    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }

    /// Clear all keyframes.
    pub fn clear(&mut self) {
        self.keyframes.clear();
        self.action_bindings.clear();
    }
}

impl Default for OilMotionPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pipeline() -> OilMotionPipeline {
        OilMotionPipeline::new()
    }

    #[test]
    fn new_pipeline_has_no_keyframes() {
        let p = make_pipeline();
        assert_eq!(p.keyframe_count(), 0);
    }

    #[test]
    fn add_keyframe_increments_count() {
        let mut p = make_pipeline();
        p.add_keyframe(Keyframe {
            index: 0,
            time_ms: 0,
            properties: BTreeMap::from([("opacity".to_string(), 0.0)]),
        });
        assert_eq!(p.keyframe_count(), 1);
    }

    #[test]
    fn keyframes_sorted_by_index() {
        let mut p = make_pipeline();
        p.add_keyframe(Keyframe { index: 2, time_ms: 200, properties: BTreeMap::new() });
        p.add_keyframe(Keyframe { index: 0, time_ms: 0, properties: BTreeMap::new() });
        p.add_keyframe(Keyframe { index: 1, time_ms: 100, properties: BTreeMap::new() });

        let kfs = p.keyframes();
        assert_eq!(kfs[0].index, 0);
        assert_eq!(kfs[1].index, 1);
        assert_eq!(kfs[2].index, 2);
    }

    #[test]
    fn interpolate_between_keyframes() {
        let mut p = make_pipeline();
        p.add_keyframe(Keyframe {
            index: 0,
            time_ms: 0,
            properties: BTreeMap::from([("x".to_string(), 0.0), ("y".to_string(), 0.0)]),
        });
        p.add_keyframe(Keyframe {
            index: 1,
            time_ms: 1000,
            properties: BTreeMap::from([("x".to_string(), 100.0), ("y".to_string(), 200.0)]),
        });

        let result = p.interpolate(500).unwrap();
        assert!((result["x"] - 50.0).abs() < 0.01);
        assert!((result["y"] - 100.0).abs() < 0.01);
    }

    #[test]
    fn interpolate_at_start_returns_first() {
        let mut p = make_pipeline();
        p.add_keyframe(Keyframe {
            index: 0,
            time_ms: 0,
            properties: BTreeMap::from([("x".to_string(), 10.0)]),
        });
        p.add_keyframe(Keyframe {
            index: 1,
            time_ms: 1000,
            properties: BTreeMap::from([("x".to_string(), 20.0)]),
        });

        let result = p.interpolate(0).unwrap();
        assert_eq!(result["x"], 10.0);
    }

    #[test]
    fn interpolate_at_end_returns_last() {
        let mut p = make_pipeline();
        p.add_keyframe(Keyframe {
            index: 0,
            time_ms: 0,
            properties: BTreeMap::from([("x".to_string(), 10.0)]),
        });
        p.add_keyframe(Keyframe {
            index: 1,
            time_ms: 1000,
            properties: BTreeMap::from([("x".to_string(), 20.0)]),
        });

        let result = p.interpolate(1000).unwrap();
        assert_eq!(result["x"], 20.0);
    }

    #[test]
    fn bind_action_registers_binding() {
        let mut p = make_pipeline();
        p.bind_action(AnimationAction::Scroll, "fade-in");
        assert_eq!(p.action_bindings.get(&AnimationAction::Scroll).unwrap().len(), 1);
    }

    #[test]
    fn generate_css_produces_valid_output() {
        let mut p = make_pipeline();
        p.add_keyframe(Keyframe {
            index: 0,
            time_ms: 0,
            properties: BTreeMap::from([("opacity".to_string(), 0.0)]),
        });
        p.add_keyframe(Keyframe {
            index: 1,
            time_ms: 1000,
            properties: BTreeMap::from([("opacity".to_string(), 1.0)]),
        });

        let css = p.generate_css("test-anim");
        assert!(css.contains("@keyframes test-anim"));
        assert!(css.contains("opacity"));
    }

    #[test]
    fn stage_enabled_defaults_to_true() {
        let p = make_pipeline();
        assert!(p.stage_enabled(PipelineStage::KeyframeExtraction));
        assert!(p.stage_enabled(PipelineStage::MotionGeneration));
        assert!(p.stage_enabled(PipelineStage::WebCompilation));
    }

    #[test]
    fn clear_removes_all_keyframes() {
        let mut p = make_pipeline();
        p.add_keyframe(Keyframe { index: 0, time_ms: 0, properties: BTreeMap::new() });
        p.add_keyframe(Keyframe { index: 1, time_ms: 100, properties: BTreeMap::new() });
        p.bind_action(AnimationAction::Scroll, "anim");

        assert_eq!(p.keyframe_count(), 2);

        p.clear();
        assert_eq!(p.keyframe_count(), 0);
        assert!(p.action_bindings.is_empty());
    }
}
