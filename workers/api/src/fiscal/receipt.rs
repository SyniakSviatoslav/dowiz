//! PURE. The receipt as text (TAX §3.8): what a printer, a screen, a PDF or a
//! message shows for one `Document`.
//!
//! LAW 87/2019 ART. 29 p.1: during an outage the invoice is issued with every
//! element of chapter IV EXCEPT the NIVF, printed on paper — and it must SAY
//! so. Without codes the receipt prints "pa NIVF"; once `Noted{fiscal}` exists
//! the copy carries the codes and the verification link instead.
//!
//! MONEY IS TEXT FROM INTEGERS. `notify::money_text` knows each currency's
//! minor units (1500 ALL is "1500 ALL", never "15.00"); the rate is printed from
//! its ppm with integer arithmetic. No float touches a figure here.

use super::document::{uuid_text, Document, Group, Kind};
use super::sender::Codes;
use crate::notify::money_text;

/// Where a fiscalised invoice is checked (EBILLS §1.5's recorded shape).
const VERIFY: &str = "https://efiskalizimi-app.tatime.gov.al/invoice-check/#/verify";

/// 200000 ppm -> "20", 88750 -> "8.875". Integer only.
fn percent(ppm: i64) -> String {
    let (whole, frac) = (ppm / 10_000, (ppm % 10_000).abs());
    if frac == 0 {
        return whole.to_string();
    }
    let f = format!("{frac:04}");
    format!("{whole}.{}", f.trim_end_matches('0'))
}

fn group_line(label: &str, g: &Group, cur: &str) -> String {
    format!(
        "{label} {} %: base {}, TVSH {}\n",
        percent(g.rate_ppm),
        money_text(g.base, cur),
        money_text(g.tax, cur)
    )
}

/// The receipt. `codes` is `Some` once the document is registered.
pub fn receipt_text(doc: &Document, codes: Option<&Codes>) -> String {
    let cur = doc.currency.as_str();
    let (stamp, _) = crate::cloud::amz_dates(doc.issued_at_ms);
    let mut out = String::new();
    out.push_str(if doc.corrects.is_some() { "FATURE KORRIGJUESE\n" } else { "FATURE\n" });
    out.push_str(&format!("Nr. {}\n", uuid_text(&doc.uuid)));
    if let Some(c) = &doc.corrects {
        out.push_str(&format!("Korrigjon: {}\n", uuid_text(c)));
    }
    out.push_str(&format!("Data: {stamp}\n"));
    out.push_str(&format!("Porosia: {}\n", doc.order_id));
    if let Some(t) = &doc.table {
        out.push_str(&format!("Tavolina: {t}\n"));
    }
    out.push_str(&format!("Kanali: {}\n", doc.channel));
    out.push_str(if doc.inclusive { "Cmimet perfshijne TVSH\n" } else { "Cmimet pa TVSH\n" });
    out.push('\n');

    for l in &doc.lines {
        out.push_str(&format!(
            "{} x {} @ {} = {} (TVSH {} %)\n",
            l.qty,
            l.name,
            money_text(l.unit_as_priced, cur),
            money_text(l.gross, cur),
            percent(l.rate_ppm)
        ));
    }
    out.push('\n');
    for g in &doc.groups {
        out.push_str(&group_line("TVSH", g, cur));
    }
    if let Some(f) = &doc.fee {
        out.push_str(&group_line("Dergesa, TVSH", f, cur));
    }
    out.push_str(&format!("Baza: {}\n", money_text(doc.total_base, cur)));
    out.push_str(&format!("TVSH: {}\n", money_text(doc.total_tax, cur)));
    out.push_str(&format!("TOTALI: {}\n", money_text(doc.total, cur)));
    out.push('\n');

    out.push_str(match doc.kind {
        Kind::Cash => "Pagesa: kesh\n",
        Kind::NonCash => "Pagesa: pa kesh\n",
    });
    for p in &doc.payments {
        let rate = p.rate_ppm.map(|r| format!(" (kursi {} ppm)", r)).unwrap_or_default();
        out.push_str(&format!("  {} {}{}\n", p.method, money_text(p.amount, &p.currency), rate));
    }
    out.push('\n');

    match codes {
        Some(c) => {
            out.push_str(&format!("NSLF (IIC): {}\n", c.iic));
            out.push_str(&format!("NIVF (FIC): {}\n", c.fic));
            out.push_str(&format!("Nr. i fatures: {}\n", c.inv_ord_num));
            out.push_str(&format!("{VERIFY}?iic={}\n", c.iic));
        }
        None => {
            out.push_str("pa NIVF\n");
            out.push_str("Leshuar pa lidhje (neni 29): fiskalizohet brenda 48 oreve.\n");
        }
    }
    out
}

#[cfg(test)]
mod tests;
