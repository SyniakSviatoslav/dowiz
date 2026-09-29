//! RE-DERIVE THE DISHES AN ITEM REACHES (SPEC-SEMI-FINISHED §d (2)): a raw
//! supply's price or nutrition changed, or a ПФ's card did, and every dish
//! whose tree reaches it stores `cost`, `nutrition` and `weightG` from its
//! recipe. Re-running `set_bom` on each with its own stored lines, in the
//! SAME catalogue write as the change, keeps the stored numbers (the public
//! menu prints two of them) equal to what a read would derive -- and closes
//! the old gap where a raw's kcal edit left every dish stale until it was
//! saved again by hand. What the owner typed (`…Derived: false`) is kept, as
//! `Typed::from_record` says.

use dowiz_hub::catalog::Catalog;
use serde_json::Value;

use crate::recipe::apply::{set_bom, Typed};
use crate::recipe::BomLineIn;

/// The stored lines of a dish, as `set_bom` takes them.
pub fn lines_of(p: &Value) -> Vec<BomLineIn> {
    p.get("bom")
        .and_then(Value::as_array)
        .map(|b| {
            b.iter()
                .filter_map(|l| {
                    let weighed = |k: &str| l.get(k).and_then(Value::as_i64);
                    Some(BomLineIn { supply: l.get("supply")?.as_str()?.to_string(), qty: l.get("qty")?.as_i64()?, net: weighed("net"), out: weighed("out") })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Re-derive every dish whose tree reaches `item`; answers their ids. A
/// dish whose lines no longer all resolve is left as stored (its read still
/// derives what it can).
pub fn dishes_using(cat: &mut Catalog, item: &str) -> Vec<String> {
    let uses = dowiz_hub::prep::uses_of(item, &cat.supplies(), &cat.products());
    let mut done = Vec::new();
    for (pid, _) in uses.dishes {
        let Some(mut p) = cat.product(&pid).and_then(|j| serde_json::from_str::<Value>(&j).ok()) else { continue };
        let lines = lines_of(&p);
        let typed = Typed::from_record(&p);
        let before = p.to_string();
        if set_bom(&mut p, &lines, |s| cat.supply(s), typed).is_ok() && p.to_string() != before {
            cat.set_product(&pid, &p.to_string());
            done.push(pid);
        }
    }
    done
}
