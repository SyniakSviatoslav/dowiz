//! WHAT A DISH TASTES, FEELS AND SMELLS LIKE, as the kitchen declares it (W-SENSE, 2026-10-04).
//!
//! Operator, 2026-10-04: the dish cards must show every defined taste axis and the texture, and
//! AROMA is a third dimension beside them. This module is the one place the vocabulary and its
//! ranges are written down; the Worker validates the owner's edit with [`validate`], the device
//! and the server fold a guest's taste from [`vector`], and a model's draft is held to the
//! vocabulary by [`from_model`]. Its rules, each one a test in `sense/tests.rs`:
//!   * CLOSED VOCABULARY, STABLE IDS. Six taste axes 0..=5, nine textures and twelve aromas at
//!     1..=3. The ids never change; the words live in four languages on the surfaces.
//!   * ABSENT IS NOT ZERO. An axis nobody declared is absent and is never drawn. A taste axis at 0
//!     IS a claim ("not spicy at all"). A texture or aroma sent at 0 is removed: a tag is there at
//!     1..=3 or it is not.
//!   * INTEGERS ONLY. `2.5`, `"2"`, `true` are refused by name, never rounded.
//!   * NO ALLERGEN, NO HEALTH WORD in the vocabulary: a guest can be matched on what a dish tastes
//!     like, never on what could hurt them (allergens only ever filter or warn, `allergens.rs`).
//!   * THE OLD FIELD STILL READS. A product with no `sense` takes its taste from the old `taste`
//!     map (spicy/sweet/salty/sour at 1..=3, [`legacy_level`]); `richness` has no axis here.

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub mod lexicon;

pub const VERSION: i64 = 1;
/// The six taste axes, 0 (none) ..= 5 (very).
pub const TASTE: [&str; 6] = ["sweet", "sour", "salty", "bitter", "umami", "spicy"];
pub const TASTE_MAX: i64 = 5;
/// Texture and aroma tags, intensity 1 ..= 3.
pub const TEXTURE: [&str; 9] = ["crispy", "crunchy", "tender", "creamy", "chewy", "soft", "juicy", "silky", "flaky"];
pub const AROMA: [&str; 12] = [
    "smoky", "citrus", "herbal", "floral", "nutty", "toasty", "marine", "fermented", "fruity", "earthy", "buttery", "spice-warm",
];
pub const TAG_MAX: i64 = 3;
/// A vector's weights are per mille of the scale's top.
pub const SCALE: i64 = 1000;

/// The three dimensions, their wire name, key prefix, vocabulary and range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dim {
    Taste,
    Texture,
    Aroma,
}

impl Dim {
    pub const ALL: [Dim; 3] = [Dim::Taste, Dim::Texture, Dim::Aroma];
    pub fn wire(self) -> &'static str {
        match self {
            Dim::Taste => "taste",
            Dim::Texture => "texture",
            Dim::Aroma => "aroma",
        }
    }
    /// The prefix of a vector key: `t:spicy`, `x:crispy`, `a:smoky`.
    pub fn prefix(self) -> &'static str {
        match self {
            Dim::Taste => "t",
            Dim::Texture => "x",
            Dim::Aroma => "a",
        }
    }
    pub fn words(self) -> &'static [&'static str] {
        match self {
            Dim::Taste => &TASTE,
            Dim::Texture => &TEXTURE,
            Dim::Aroma => &AROMA,
        }
    }
    /// The lowest STORED value: a taste axis may be 0, a tag starts at 1.
    pub fn min(self) -> i64 {
        if self == Dim::Taste { 0 } else { 1 }
    }
    pub fn max(self) -> i64 {
        if self == Dim::Taste { TASTE_MAX } else { TAG_MAX }
    }
}

/// One dish's declared profile. Every map holds only vocabulary ids at stored values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sense {
    pub taste: BTreeMap<String, i64>,
    pub texture: BTreeMap<String, i64>,
    pub aroma: BTreeMap<String, i64>,
}

impl Sense {
    pub fn map(&self, d: Dim) -> &BTreeMap<String, i64> {
        match d {
            Dim::Taste => &self.taste,
            Dim::Texture => &self.texture,
            Dim::Aroma => &self.aroma,
        }
    }
    pub fn map_mut(&mut self, d: Dim) -> &mut BTreeMap<String, i64> {
        match d {
            Dim::Taste => &mut self.taste,
            Dim::Texture => &mut self.texture,
            Dim::Aroma => &mut self.aroma,
        }
    }
    pub fn is_empty(&self) -> bool {
        Dim::ALL.iter().all(|d| self.map(*d).is_empty())
    }
    /// The stored form, `{v, taste, texture, aroma}`, or `null` when nothing is declared.
    pub fn json(&self) -> Value {
        if self.is_empty() {
            return Value::Null;
        }
        json!({ "v": VERSION, "taste": self.taste, "texture": self.texture, "aroma": self.aroma })
    }
}

/// Is `key` a vector key of the vocabulary (`t:spicy`, `x:crispy`, `a:spice-warm`)?
pub fn key_ok(key: &str) -> bool {
    let Some((p, w)) = key.split_once(':') else { return false };
    Dim::ALL.iter().any(|d| d.prefix() == p && d.words().contains(&w))
}

/// Every key of the vocabulary, taste first.
pub fn all_keys() -> Vec<String> {
    Dim::ALL.iter().flat_map(|d| d.words().iter().map(move |w| format!("{}:{w}", d.prefix()))).collect()
}

/// The owner's edit, judged. `{taste?, texture?, aroma?, v?}`: anything else, an unknown id, a
/// value that is not an integer or is out of range is `Err(why)`. A tag at 0 is dropped.
pub fn validate(v: &Value) -> Result<Sense, String> {
    let obj = v.as_object().ok_or("sense is an object of taste, texture and aroma")?;
    let mut out = Sense::default();
    for (k, inner) in obj {
        if k == "v" {
            if inner.as_i64() != Some(VERSION) {
                return Err(format!("sense.v is {VERSION}"));
            }
            continue;
        }
        let Some(d) = Dim::ALL.iter().copied().find(|d| d.wire() == k) else {
            return Err(format!("unknown sense dimension {k:?}; taste, texture or aroma"));
        };
        let m: &Map<String, Value> = match inner {
            Value::Null => continue,
            Value::Object(m) => m,
            _ => return Err(format!("sense.{k} is an object of id: level")),
        };
        for (id, level) in m {
            if !d.words().contains(&id.as_str()) {
                return Err(format!("unknown {k} {id:?}"));
            }
            let n = match level {
                Value::Number(n) => n.as_i64().ok_or_else(|| format!("sense.{k}.{id} is a whole number"))?,
                Value::Null if d != Dim::Taste => 0,
                _ => return Err(format!("sense.{k}.{id} is a whole number")),
            };
            if d != Dim::Taste && n == 0 {
                continue; // a tag at 0 is a tag removed
            }
            if !(d.min()..=d.max()).contains(&n) {
                return Err(format!("sense.{k}.{id} is {} to {}", d.min(), d.max()));
            }
            out.map_mut(d).insert(id.clone(), n);
        }
    }
    Ok(out)
}

/// An old `taste` level (1..=3) on the new 0..=5 axis: x5/3, rounded half up (1->2, 2->3, 3->5).
pub fn legacy_level(l: i64) -> i64 {
    (l.clamp(0, 3) * 10 + 3) / 6
}

/// What the product declares, from `sense`, else from the old `taste` map; `None` when neither.
pub fn of_product(p: &Value) -> Option<Sense> {
    if let Some(s) = p.get("sense").filter(|s| s.is_object()).and_then(|s| validate(s).ok()) {
        return (!s.is_empty()).then_some(s);
    }
    let old = p.get("taste")?.as_object()?;
    let mut s = Sense::default();
    for (k, v) in old {
        if let (true, Some(l)) = (TASTE.contains(&k.as_str()), v.as_i64().filter(|l| (1..=3).contains(l))) {
            s.taste.insert(k.clone(), legacy_level(l));
        }
    }
    (!s.is_empty()).then_some(s)
}

/// The dish as a vector: `t:<axis>` = level/5, `x:`/`a:` = intensity/3, per mille. A taste at 0 is
/// not a weight (it says what the dish is NOT).
pub fn vector(s: &Sense) -> BTreeMap<String, i64> {
    let mut out = BTreeMap::new();
    for d in Dim::ALL {
        for (id, n) in s.map(d) {
            if *n > 0 {
                out.insert(format!("{}:{id}", d.prefix()), n * SCALE / d.max());
            }
        }
    }
    out
}

/// A model's answer, held to the vocabulary: the first JSON object in `text`, its known ids at
/// in-range integers kept, everything else dropped. Never an error: a useless answer is empty.
pub fn from_model(text: &str) -> Sense {
    let Some(start) = text.find('{') else { return Sense::default() };
    let Some(end) = text.rfind('}') else { return Sense::default() };
    let Ok(Value::Object(obj)) = serde_json::from_str::<Value>(text.get(start..=end).unwrap_or("")) else {
        return Sense::default();
    };
    let mut out = Sense::default();
    for d in Dim::ALL {
        for (id, v) in obj.get(d.wire()).and_then(Value::as_object).into_iter().flatten() {
            let ok = d.words().contains(&id.as_str());
            if let (true, Some(n)) = (ok, v.as_i64().filter(|n| (d.min()..=d.max()).contains(n))) {
                out.map_mut(d).insert(id.clone(), n);
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "sense/tests.rs"]
mod tests;
