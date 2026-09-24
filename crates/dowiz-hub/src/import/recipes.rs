//! Turning an incumbent's tech cards into supplies and recipes.
//!
//! `BLUEPRINT-LAST-MILE-2026-09-22.md` §3.3.2, built as decided. TWO FILES,
//! because every source exports two things: an INGREDIENTS file (`name, unit,
//! cost, [currency, category, kind, kcal, protein, fat, carbs]`) that becomes
//! supplies, and a RECIPES file (`dish, ingredient, qty, [unit, gross, net,
//! yield, prepack, batch, from, to]`) that becomes each dish's `bom`.
//!
//! THE SAME DOORWAY AS THE MENU (`super::from_csv`): pure, no I/O; everything
//! it cannot read with certainty is a warning naming the file and the row;
//! ids are deterministic so a second import is an update. What this adds is
//! the part the menu never had to face: a recipe in every incumbent carries
//! MORE than dowiz's `{supply, qty}` — gross vs net vs yield, semi-finished
//! products, dated versions — and each of those is either kept, flattened or
//! declared lost here, by rule, never silently.
//!
//! A DISH WITH ANY LINE THIS CANNOT READ GETS NO RECIPE FROM THIS FILE, not a
//! partial one. A dish with no `bom` sells and reserves nothing
//! (`stock::bom_of`); a dish with half a `bom` reserves the wrong amounts
//! under a ledger that looks correct, which is worse than no ledger.

mod against;
mod cards;
mod flatten;
mod num;
mod supplies;
#[cfg(test)]
mod tests;

pub use against::recipes_against;
pub use num::CostScale;

use crate::minijson::esc;

/// A recipe line's quantity is bounded: a kitchen does not put a tonne in a
/// roll. The Worker's `recipe::QTY_MAX` is the same number and refuses the
/// same lines when an owner types them.
pub const QTY_MAX: i64 = 100_000;

/// What the caller knows that the file does not.
pub struct Opts<'a> {
    /// The venue's currency. A file in another one is refused whole.
    pub currency: &'a str,
    /// How the cost column writes money; `None` means the owner has not said,
    /// and then no cost is imported (one warning says why).
    pub cost_scale: Option<CostScale>,
    /// The unit a recipe quantity is in when neither its cell nor the unit
    /// column says. `None` refuses such a line.
    pub unit_hint: Option<&'a str>,
    /// The venue's LOCAL wall-clock now, in ms: which dated card is in force
    /// today. Read once by the Worker and passed down.
    pub local_now_ms: i64,
    /// `(id, name)` of every dish on the menu now.
    pub products: &'a [(String, String)],
    /// `(id, unit)` of every supply in the catalogue now. Those not in the
    /// file are RETIRED (`active:false`), never deleted — stock history names
    /// them; one whose unit the file changes is refused, because the ledger's
    /// counts would silently change meaning.
    pub existing_supplies: &'a [(String, String)],
}

#[derive(Debug, Clone, PartialEq)]
pub struct DraftSupply {
    pub id: String,
    pub name: String,
    /// `g`, `ml` or `unit`.
    pub unit: &'static str,
    /// `None` when the file does not say: the catalogue keeps what it had.
    pub kind: Option<String>,
    pub category: String,
    /// Venue minor units per 100 g/ml or per piece; `None` if not imported.
    pub cost_per_basis: Option<i64>,
    /// Per 100 g/ml or per piece. Floats because the catalogue's are
    /// (`kcalPer100`); never used in a reservation.
    pub kcal: Option<f64>,
    pub protein: Option<f64>,
    pub fat: Option<f64>,
    pub carbs: Option<f64>,
    /// The reorder threshold, in the base unit; `None` keeps what it was.
    pub low_at: Option<i64>,
    /// Who the kitchen buys it from, as the file writes it.
    pub supplier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftLine {
    pub supply: String,
    /// GROSS, in the supply's base unit: what the incumbents write off, so the
    /// ledger's counts agree with the ones the owner trusts on day one.
    pub qty: i64,
    /// Net and yield, kept on the line under keys the hub ignores.
    pub net: Option<i64>,
    pub yield_: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftRecipe {
    pub product_id: String,
    pub dish: String,
    pub lines: Vec<DraftLine>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RecipeDraft {
    pub supplies: Vec<DraftSupply>,
    pub recipes: Vec<DraftRecipe>,
    /// Dishes on the menu whose card could not be read whole: they get no
    /// recipe from this file, and each has a warning saying which row.
    pub without_recipe: Vec<String>,
    /// Supplies in the catalogue and not in the file: to be `active:false`.
    pub retired: Vec<String>,
    /// "dish: 'sauce' expanded into 4 lines", one per semi-finished use.
    pub flattened: Vec<String>,
    pub warnings: Vec<String>,
}

/// Parse the two files. Either may be empty; recipes need the supplies file,
/// because a line naming a supply that is not in it is refused.
pub fn from_csv(ingredients: &str, recipes: &str, opts: &Opts) -> RecipeDraft {
    let mut draft = RecipeDraft::default();
    if !supplies::read(ingredients, opts, &mut draft) {
        // Refused whole (a foreign currency): nothing from either file.
        return draft;
    }
    if !recipes.trim().is_empty() {
        cards::read(recipes, opts, &mut draft);
    }
    // Retiring needs a file that produced supplies: an unreadable one must not
    // take every ingredient off the list.
    if !draft.supplies.is_empty() {
        for (id, _) in opts.existing_supplies {
            if !draft.supplies.iter().any(|s| &s.id == id) {
                draft.retired.push(id.clone());
            }
        }
    }
    draft
}

/// A header cell of either file, as the column it names.
pub(super) fn column(name: &str) -> Option<&'static str> {
    let n = name.trim().trim_start_matches('\u{feff}').to_lowercase();
    Some(match n.as_str() {
        "id" | "code" | "kod" | "kodi" | "код" | "артикул" => "id",
        "name" | "ingredient_name" | "emri" | "назва" | "название" => "name",
        "ingredient" | "supply" | "ingredient name" | "përbërës" | "perberes" | "інгредієнт"
        | "ингредиент" => "ingredient",
        "dish" | "product" | "product_name" | "pjata" | "produkti" | "страва" | "блюдо" => "dish",
        "unit" | "njësia" | "njesia" | "njësi" | "njesi" | "од" | "од." | "одиниця" | "ед" | "единица" => "unit",
        "cost" | "kosto" | "kostoja" | "price" | "çmimi" | "cmimi" | "собівартість" | "ціна" | "себестоимость"
        | "цена" => "cost",
        "per" | "për" | "cost per" | "price per" | "за" => "per",
        "supplier" | "furnitori" | "furnizuesi" | "постачальник" | "поставщик" => "supplier",
        "low_at" | "low at" | "low" | "min" | "minimum" | "minimumi" | "мінімум" | "минимум" => "low_at",
        "currency" | "monedha" | "валюта" => "currency",
        "category" | "kategoria" | "категорія" | "категория" => "category",
        "kind" | "type" | "lloji" | "тип" | "вид" => "kind",
        "kcal" | "kcal/100" | "kalori" | "kalorite" | "калорії" | "ккал" | "калории" => "kcal",
        "protein" | "proteina" | "proteinat" | "білки" | "белки" => "protein",
        "fat" | "yndyra" | "yndyrna" | "жири" | "жиры" => "fat",
        "carbs" | "karbohidrate" | "karbohidratet" | "вуглеводи" | "углеводы" => "carbs",
        "qty" | "quantity" | "sasia" | "кількість" | "количество" => "qty",
        "gross" | "brutto" | "брутто" | "amountin" => "gross",
        "net" | "netto" | "нетто" | "amountmiddle" => "net",
        "yield" | "output" | "вихід" | "выход" | "amountout" => "yield",
        "prepack" | "semi" | "напівфабрикат" | "полуфабрикат" => "prepack",
        "batch" | "assembled" | "assembledamount" | "партія" | "норма" => "batch",
        "from" | "valid_from" | "datefrom" | "від" | "с" => "from",
        "to" | "valid_to" | "dateto" | "до" | "по" => "to",
        _ => return None,
    })
}

/// One CSV file as a header map and numbered rows (row 1 is the header).
pub(super) struct Sheet {
    cols: Vec<Option<&'static str>>,
    pub rows: Vec<(usize, Vec<String>)>,
}

impl Sheet {
    pub fn parse(text: &str) -> Option<Sheet> {
        let mut lines = text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty());
        let (_, header) = lines.next()?;
        let sep = super::detect_separator(header);
        let cols = super::split_csv_line(header, sep).iter().map(|h| column(h)).collect();
        let rows = lines.map(|(i, l)| (i + 1, super::split_csv_line(l, sep))).collect();
        Some(Sheet { cols, rows })
    }
    pub fn has(&self, key: &str) -> bool {
        self.cols.contains(&Some(key))
    }
    pub fn get<'r>(&self, row: &'r [String], key: &str) -> &'r str {
        self.cols
            .iter()
            .position(|c| *c == Some(key))
            .and_then(|i| row.get(i))
            .map(|s| s.trim())
            .unwrap_or("")
    }
}

fn opt_f(v: Option<f64>) -> String {
    v.map_or("null".into(), |x| format!("{x}"))
}
fn opt_i(v: Option<i64>) -> String {
    v.map_or("null".into(), |x| x.to_string())
}
fn strs(v: &[String]) -> String {
    v.iter().map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(",")
}

impl RecipeDraft {
    /// The preview the console draws before anything is written.
    pub fn as_json(&self) -> String {
        let sup: Vec<String> = self
            .supplies
            .iter()
            .map(|s| {
                format!(
                    r#"{{"id":"{}","name":"{}","unit":"{}","kind":{},"category":"{}","costPerBasis":{},"kcalPer100":{},"proteinPer100":{},"fatPer100":{},"carbsPer100":{},"lowAt":{},"supplier":{}}}"#,
                    esc(&s.id),
                    esc(&s.name),
                    s.unit,
                    s.kind.as_ref().map_or("null".into(), |k| format!("\"{}\"", esc(k))),
                    esc(&s.category),
                    opt_i(s.cost_per_basis),
                    opt_f(s.kcal),
                    opt_f(s.protein),
                    opt_f(s.fat),
                    opt_f(s.carbs),
                    opt_i(s.low_at),
                    s.supplier.as_ref().map_or("null".into(), |k| format!("\"{}\"", esc(k)))
                )
            })
            .collect();
        let rec: Vec<String> = self
            .recipes
            .iter()
            .map(|r| {
                let lines: Vec<String> = r
                    .lines
                    .iter()
                    .map(|l| {
                        format!(
                            r#"{{"supply":"{}","qty":{},"net":{},"yield":{}}}"#,
                            esc(&l.supply),
                            l.qty,
                            opt_i(l.net),
                            opt_i(l.yield_)
                        )
                    })
                    .collect();
                format!(
                    r#"{{"productId":"{}","dish":"{}","bom":[{}]}}"#,
                    esc(&r.product_id),
                    esc(&r.dish),
                    lines.join(",")
                )
            })
            .collect();
        format!(
            r#"{{"supplies":[{}],"recipes":[{}],"withoutRecipe":[{}],"retired":[{}],"flattened":[{}],"warnings":[{}]}}"#,
            sup.join(","),
            rec.join(","),
            strs(&self.without_recipe),
            strs(&self.retired),
            strs(&self.flattened),
            strs(&self.warnings)
        )
    }
}
