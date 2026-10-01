//! THE CSV DIALECT a spreadsheet export speaks: comma or semicolon separated,
//! `"` quoting, `""` for a literal quote -- and which header names which column.
//! Shared with `recipes.rs`, which reads its own sheets the same way.

/// Split one CSV line, honouring double quotes.
///
/// Hand-written because a CSV crate is outside the allowlist, and because the
/// dialect needed here is small and fully specified: comma or semicolon
/// separated, `"` quoting, `""` for a literal quote. A spreadsheet export from
/// Excel or Google Sheets is exactly this.
pub(super) fn split_csv_line(line: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            c if c == sep && !in_quotes => {
                out.push(cur.trim().to_string());
                cur = String::new();
            }
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// The separator a spreadsheet actually used.
///
/// Excel in a locale that uses a decimal comma writes SEMICOLON-separated CSV.
/// Assuming a comma would parse such a file as one giant column per row and
/// import nothing, with no error — the file was valid, it just meant something
/// else. Chosen by counting on the header line, where both characters are
/// separators and neither is data.
pub(super) fn detect_separator(header: &str) -> char {
    if header.matches(';').count() > header.matches(',').count() {
        ';'
    } else {
        ','
    }
}

/// Match a header cell to a known column, tolerantly.
pub(super) fn column_of(name: &str) -> Option<&'static str> {
    let n = name.trim().to_lowercase();
    let n = n.trim_start_matches('\u{feff}'); // Excel writes a BOM
    match n {
        "category" | "categoria" | "kategoria" | "категорія" | "категория" | "розділ" => {
            Some("category")
        }
        "name" | "product" | "item" | "dish" | "emri" | "назва" | "страва" => Some("name"),
        "description" | "desc" | "pershkrimi" | "përshkrimi" | "опис" | "склад" => {
            Some("description")
        }
        "price" | "cmimi" | "çmimi" | "ціна" => Some("price"),
        "available" | "in stock" | "stock" | "наявність" | "є" => Some("available"),
        "id" | "sku" | "code" | "kod" | "артикул" => Some("id"),
        _ => None,
    }
}

pub(super) fn truthy(s: &str) -> bool {
    !matches!(
        s.trim().to_lowercase().as_str(),
        "0" | "no" | "false" | "n" | "jo" | "ні" | "нет" | "немає" | "off"
    )
}
