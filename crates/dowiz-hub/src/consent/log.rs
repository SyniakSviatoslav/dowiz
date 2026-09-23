//! WHAT IS WRITTEN TO THE VENUE'S `consent` IMAGE (§3.2), and nothing else.
//!
//! Two kinds in one append-only `LogImage`: `c`, one consent act, filed under
//! `cust:<key>` so one person's history is one prefix read; and `w`, the exact
//! sentence an act names, filed ONCE under its own id so the proof of what a
//! person was told can always be shown back as words.
//!
//! THE WRITER REFUSES WHAT THE FOLD WOULD IGNORE, with the same `check`, and
//! refuses BEFORE a byte lands: the image is append-only, so a record written
//! in error is a record kept for ever.

use super::{check, subject_of, wording_json, Act, State, KIND_ACT, KIND_WORDING, LANGS};
use crate::logimage::LogImage;

/// The language whose sentence has this id, or `None` -- an id this build
/// cannot show as words is not proof of anything.
pub fn lang_of_wording(id: &str) -> Option<&'static str> {
    LANGS.iter().copied().find(|l| super::wording_id(l) == id)
}

/// Append one act, and its wording the first time that wording is used.
///
/// A GRANT MUST NAME A SENTENCE THIS BUILD KNOWS. A withdrawal names none --
/// Art. 7(3), as easy as giving.
pub fn write(log: &mut LogImage, act: &Act) -> Result<(), String> {
    check(act)?;
    if act.state == State::Given {
        let Some(lang) = lang_of_wording(&act.wording_id) else {
            return Err(format!("unknown wording {:?}: no sentence to show for it", act.wording_id));
        };
        if log.about(KIND_WORDING, Some(&act.wording_id), 1).is_empty() {
            log.append(KIND_WORDING, &act.wording_id, &wording_json(lang))
                .map_err(|e| format!("consent image refused the wording: {e:?}"))?;
        }
    }
    log.append(KIND_ACT, &subject_of(&act.key), &act.to_json())
        .map_err(|e| format!("consent image refused the act: {e:?}"))
}
