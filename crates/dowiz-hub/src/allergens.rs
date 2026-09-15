//! What is in the dish, for the people it could hurt.
//!
//! THE WHOLE MODULE TURNS ON ONE DISTINCTION: an ABSENT allergen list and an
//! EMPTY one are not the same claim. Absent means nobody has said; empty means
//! somebody has said "none of the fourteen". A customer with a shellfish
//! allergy reading a sushi menu needs to know which of those they are looking
//! at, and a system that renders both as no warning has quietly told them the
//! dish is safe. That is the failure this exists to prevent.
//!
//! So the state is a three-way `Declaration`, an undeclared dish cannot be put
//! on sale, and "none" has to be entered deliberately.
//!
//! THE LIST IS THE EU FOURTEEN, which is what Albanian and EU food labelling
//! actually names, rather than a set invented here. An unknown code is REFUSED
//! rather than stored: a dish tagged `shelfish` would match no filter, so the
//! customer who filtered for shellfish would be shown it as safe.

/// The fourteen, in the order the regulation lists them. The codes are the
/// storage form and are never shown; each surface translates them.
pub const EU14: [&str; 14] = [
    "gluten",
    "crustaceans",
    "eggs",
    "fish",
    "peanuts",
    "soy",
    "milk",
    "nuts",
    "celery",
    "mustard",
    "sesame",
    "sulphites",
    "lupin",
    "molluscs",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Declaration {
    /// Nobody has said. NOT a claim that the dish is free of anything.
    Undeclared,
    /// Somebody has said: none of the fourteen.
    None,
    /// These, and by implication not the others.
    Contains(Vec<String>),
}

impl Declaration {
    /// Has a human answered the question at all?
    pub fn is_declared(&self) -> bool {
        !matches!(self, Declaration::Undeclared)
    }

    pub fn codes(&self) -> &[String] {
        match self {
            Declaration::Contains(v) => v,
            _ => &[],
        }
    }
}

pub fn is_known(code: &str) -> bool {
    EU14.contains(&code)
}

/// Clean a submitted list, or say which code is not one of the fourteen.
///
/// Lower-cased, de-duplicated and put back into regulation order, so two dishes
/// carrying the same allergens carry them identically and a diff of the menu
/// does not show a reordering as a change.
pub fn validate(raw: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for r in raw {
        let c = r.trim().to_ascii_lowercase();
        if c.is_empty() {
            continue;
        }
        if !is_known(&c) {
            return Err(format!("\"{c}\" is not one of the fourteen declarable allergens"));
        }
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out.sort_by_key(|c| EU14.iter().position(|e| e == c).unwrap_or(usize::MAX));
    Ok(out)
}

/// Read a product's declaration out of its stored JSON.
///
/// `serde_json` is not used here: this crate reads its own records with
/// `minijson`, and an allergen list is a flat array of short strings. Anything
/// that is not a readable array is `Undeclared` -- a record the reader cannot
/// understand must not present itself as a safety claim.
pub fn read(product_json: &str) -> Declaration {
    let Some(list) = array_field(product_json, "allergens") else {
        return Declaration::Undeclared;
    };
    if list.is_empty() {
        return Declaration::None;
    }
    Declaration::Contains(list)
}

/// The strings inside `"key":[ ... ]`, or `None` when the key is absent.
///
/// Deliberately small: it reads one flat array of quoted strings and gives up
/// on anything else, which for this field is the whole grammar.
fn array_field(json: &str, key: &str) -> Option<Vec<String>> {
    let pat = format!("\"{key}\":");
    let at = json.find(&pat)?;
    let rest = json[at + pat.len()..].trim_start();
    let inner = rest.strip_prefix('[')?;
    let end = inner.find(']')?;
    let body = &inner[..end];
    let mut out = Vec::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut s = String::new();
        for c in chars.by_ref() {
            if c == '"' {
                break;
            }
            s.push(c);
        }
        if !s.is_empty() {
            out.push(s);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The distinction the module exists for.
    #[test]
    fn absent_and_empty_are_different_claims() {
        assert_eq!(read(r#"{"name":"Sake Futomaki"}"#), Declaration::Undeclared);
        assert_eq!(read(r#"{"allergens":[]}"#), Declaration::None);
        assert_eq!(
            read(r#"{"allergens":["fish","soy"]}"#),
            Declaration::Contains(vec!["fish".into(), "soy".into()])
        );
        assert!(!read(r#"{"name":"x"}"#).is_declared());
        assert!(read(r#"{"allergens":[]}"#).is_declared());
    }

    /// A record the reader cannot understand must not read as a safety claim.
    #[test]
    fn an_unreadable_field_is_undeclared_not_safe() {
        for bad in [
            r#"{"allergens":null}"#,
            r#"{"allergens":"fish"}"#,
            r#"{"allergens":{"a":1}}"#,
            r#"{"allergens":[  "#,
        ] {
            assert_eq!(read(bad), Declaration::Undeclared, "read as a claim: {bad}");
        }
    }

    #[test]
    fn a_misspelled_allergen_is_refused_rather_than_stored() {
        // "shelfish" would match no filter, so the customer who filtered for
        // shellfish would be shown this dish as safe.
        let e = validate(&["shelfish".into()]).unwrap_err();
        assert!(e.contains("shelfish"), "{e}");
        assert!(validate(&["FISH".into()]).is_ok(), "case is not a misspelling");
    }

    #[test]
    fn a_list_is_normalised_so_two_dishes_carry_it_identically() {
        let a = validate(&["Soy".into(), "fish".into(), "soy".into()]).unwrap();
        let b = validate(&["fish".into(), "SOY".into()]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a, vec!["fish".to_string(), "soy".to_string()], "regulation order");
    }

    #[test]
    fn the_fourteen_are_fourteen_and_distinct() {
        let mut v = EU14.to_vec();
        v.sort_unstable();
        v.dedup();
        assert_eq!(v.len(), 14);
        // Codes are matched literally against stored records, so a stray
        // capital or space would make a dish undeclarable.
        assert!(EU14.iter().all(|c| c.chars().all(|ch| ch.is_ascii_lowercase())));
    }

    #[test]
    fn an_empty_submission_declares_none() {
        assert_eq!(validate(&[]).unwrap(), Vec::<String>::new());
        assert_eq!(validate(&["".into(), "  ".into()]).unwrap(), Vec::<String>::new());
    }
}
