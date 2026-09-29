//! W-CRUD: a deleted dish's or category's translations go with it -- through
//! the real table image, the way `forget_translations` walks it.
use super::*;
use dowiz_hub::table::Table;

fn table() -> Table {
    let mut t = Table::create(crate::hubstore::I18N_BYTES).unwrap();
    for (l, e, id, f, v) in [
        ("uk", "product", "sake", "name", "Саке"),
        ("ru", "product", "sake", "description", "два кусочка"),
        ("uk", "product", "sake-2", "name", "Саке 2"),
        ("uk", "product", "ebi", "name", "Ебі"),
        ("uk", "category", "sake", "name", "Категорія Саке"),
        ("uk", "category", "rolls", "name", "Роли"),
    ] {
        t.put(crate::hubstore::I18N_KIND, &crate::hubstore::i18n_key(l, e, id, f), v, &[], &[]).unwrap();
    }
    t
}

#[test]
fn only_the_named_entitys_rows_are_stale_in_every_language_and_field() {
    let t = table();
    let all = t.all(crate::hubstore::I18N_KIND);
    let mut stale = stale_keys(&all, "product", &["sake".into(), "ghost".into()]);
    stale.sort();
    assert_eq!(stale, ["ru/product/sake/description", "uk/product/sake/name"]);
    assert_eq!(stale_keys(&all, "category", &["sake".into()]), ["uk/category/sake/name"], "a category with a dish's id is its own entity");
    assert!(stale_keys(&all, "product", &[]).is_empty());
    assert!(stale_keys(&[("garbage".into(), "x".into())], "product", &["garbage".into()]).is_empty(), "a key that is not four parts is nobody's");
}

#[test]
fn sweeping_removes_them_and_leaves_the_neighbours() {
    let mut t = table();
    let ids = vec!["sake".to_string(), "rolls".to_string()];
    let stale = stale_keys(&t.all(crate::hubstore::I18N_KIND), "product", &ids);
    for key in &stale {
        assert!(t.remove(crate::hubstore::I18N_KIND, key));
    }
    let mut left: Vec<String> = t.all(crate::hubstore::I18N_KIND).into_iter().map(|(k, _)| k).collect();
    left.sort();
    assert_eq!(left, ["uk/category/rolls/name", "uk/category/sake/name", "uk/product/ebi/name", "uk/product/sake-2/name"],
        "sake-2 is not sake, ebi was not named, and the categories are another entity");
}
