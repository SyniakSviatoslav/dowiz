//! The door before the body, read from each handler's own source.

/// `(file, source, handler)`: every route W-FIX O9 reordered, and the menu import.
const ROUTES: &[(&str, &str, &str)] = &[
    ("operations/mod.rs", include_str!("../operations/mod.rs"), "restore"),
    ("ordering/promotions.rs", include_str!("../ordering/promotions.rs"), "set_promotion"),
    ("operations/stock.rs", include_str!("../operations/stock.rs"), "stock_move"),
    ("operations/supplies.rs", include_str!("../operations/supplies.rs"), "retire_supply"),
    ("engagement/verdict.rs", include_str!("../engagement/verdict.rs"), "approve_post"),
    ("engagement/voice.rs", include_str!("../engagement/voice.rs"), "voice"),
    ("venue/place.rs", include_str!("../venue/place.rs"), "set_place"),
    ("identity/staff_admin.rs", include_str!("../identity/staff_admin.rs"), "invite_staff"),
    ("identity/staff_admin.rs", include_str!("../identity/staff_admin.rs"), "set_staff"),
    ("catalogue/import.rs", include_str!("../catalogue/import.rs"), "import_menu"),
];

/// The doors a handler asks before it may read anything.
const DOORS: &[&str] =
    &["owner_and_venue(", "authenticate(", "staff_venue(", "signer_for(", "owner_at(", "staff_at("];

/// The body of `async fn name(` in `src`, brace-matched.
fn body_of<'a>(src: &'a str, name: &str) -> Option<&'a str> {
    let at = src.find(&format!("async fn {name}("))?;
    let open = at + src[at..].find('{')?;
    let mut depth = 0usize;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[open..open + i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Where the door is asked and where the body is first read, in `body`.
fn door_then_read(body: &str) -> (Option<usize>, Option<usize>) {
    let first = |pats: &[&str]| pats.iter().filter_map(|p| body.find(p)).min();
    (first(DOORS), first(&["req.json()", "req.text()", "req.bytes()", "body::parse(", "body::strict("]))
}

#[test]
fn every_listed_route_asks_the_door_before_it_reads_the_body() {
    for (file, src, name) in ROUTES {
        let body = body_of(src, name).unwrap_or_else(|| panic!("{file}: no `async fn {name}`"));
        let (door, read) = door_then_read(body);
        let (door, read) = (door.expect("a door"), read.expect("a body read"));
        assert!(door < read, "{file}::{name} reads its body before it asks who is calling");
    }
}

/// The twin: the check tells the two orders apart.
#[test]
fn the_check_sees_a_body_read_before_the_door() {
    let wrong = "async fn f(mut req: Request) { let b = req.json().await; let l = owner_and_venue(&req); }";
    let right = "async fn f(mut req: Request) { let l = owner_and_venue(&req); let b = req.json().await; }";
    let (d, r) = door_then_read(body_of(wrong, "f").unwrap());
    assert!(d.unwrap() > r.unwrap());
    let (d, r) = door_then_read(body_of(right, "f").unwrap());
    assert!(d.unwrap() < r.unwrap());
    assert!(body_of(right, "g").is_none());
    assert!(body_of("async fn f( {", "f").is_none(), "an unclosed body");
}

/// W-FIX O9, the other half: the menu import is bounded like its siblings, and
/// the bound is asked before the CSV is parsed.
#[test]
fn the_menu_import_is_bounded_before_it_is_parsed() {
    let (_, src, name) = ROUTES.iter().find(|r| r.2 == "import_menu").unwrap();
    let body = body_of(src, name).unwrap();
    let bound = body.find("bulk::too_big(text.len())").expect("import_menu asks the bound");
    let parse = body.find("from_csv(").expect("import_menu parses the CSV");
    assert!(bound < parse, "the bound is asked before the parse");
}
