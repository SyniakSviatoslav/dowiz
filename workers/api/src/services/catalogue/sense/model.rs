//! PURE. THE VENUE'S AI MODEL'S DRAFT OF A DISH'S SENSE, held to the vocabulary (W-TASTE row 3a;
//! operator default 2026-10-05: "wire the venue's AI-model draft (W-AI) through
//! dowiz_hub::sense::from_model").
//!
//! "Suggest" asks the venue's own AI chain (`engagement::ai::call`, the routes the owner chose in
//! the AI card) for a draft, when the venue turned AI on. Three rules, each a test:
//!   * NEVER TRUSTED RAW. The answer goes through `sense::from_model` (first JSON object, known ids
//!     at in-range integers, everything else dropped); its text is never stored, never echoed.
//!   * UNDER THE LEXICON. The lexicon's draft wins on every id it set (it carries the word that gave
//!     it); the model only fills ids the lexicon left empty, each marked `from: "model"`.
//!   * A FAILURE IS SILENCE. No model, a refusal, junk or an error: the lexicon draft alone, no error.
//!     And like the lexicon, it never writes: the owner saves with the ordinary Save.

use dowiz_hub::sense::{self, Dim, Sense};

use crate::services::engagement::ai::call::Prompt;

/// Enough for one small JSON object.
pub const MAX_TOKENS: i64 = 160;

/// The question: the dish's own words, and the closed vocabulary with its ranges.
pub fn prompt(name: &str, description: &str, ingredients: &[String]) -> Prompt {
    let ids = |d: Dim| d.words().join(", ");
    let system = format!(
        "You describe how a restaurant dish tastes, feels and smells. Answer with ONE JSON object and nothing else: \
         {{\"taste\": {{id: 0-5}}, \"texture\": {{id: 1-3}}, \"aroma\": {{id: 1-3}}}}. Use only these ids. \
         taste: {}. texture: {}. aroma: {}. Leave out what the words do not support. Whole numbers only.",
        ids(Dim::Taste),
        ids(Dim::Texture),
        ids(Dim::Aroma)
    );
    let clip = |s: &str, n: usize| s.chars().take(n).collect::<String>();
    let user = format!("Dish: {}\nDescription: {}\nIngredients: {}", clip(name, 120), clip(description, 600), clip(&ingredients.join(", "), 400));
    Prompt { system, user, max_tokens: MAX_TOKENS }
}

/// The model's text, held to the vocabulary (`sense::from_model`, after a reasoning block is cut).
pub fn read(text: &str) -> Sense {
    sense::from_model(&crate::services::engagement::ai::provider::without_thinking(text))
}

/// The lexicon's draft with the model's ids added UNDER it: `(merged, keys the model added)`.
pub fn merge(lexicon: &Sense, model: &Sense) -> (Sense, Vec<String>) {
    let mut out = lexicon.clone();
    let mut added = Vec::new();
    for d in Dim::ALL {
        for (id, n) in model.map(d) {
            if !out.map(d).contains_key(id) {
                out.map_mut(d).insert(id.clone(), *n);
                added.push(format!("{}:{id}", d.prefix()));
            }
        }
    }
    (out, added)
}
