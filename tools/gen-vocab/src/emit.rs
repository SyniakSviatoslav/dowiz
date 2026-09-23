//! Turn the kernel's answers into the JavaScript module the surfaces import.
//!
//! The output is byte-deterministic: no date, no version, no hash map. Two runs
//! on the same kernel produce the same bytes, which is the only reason
//! `tools/gates/vocab.sh` can be a diff.

use crate::read::Vocabulary;

/// Where a wrapped list is allowed to reach before it folds to the next line.
/// Not a style rule for its own sake: the emitted file is READ, in review, next
/// to the kernel enum it came from, and a 200-column line is read by nobody.
const WRAP_COL: usize = 78;

/// `'A', 'B', 'C'` wrapped at [`WRAP_COL`] and indented, or `` for an empty list.
fn wrap(items: &[&str], indent: &str) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut line = String::from(indent);
    for (i, it) in items.iter().enumerate() {
        let piece = format!("'{}',", it);
        if line.len() > indent.len() && line.len() + 1 + piece.len() > WRAP_COL {
            out.push_str(line.trim_end());
            out.push('\n');
            line = String::from(indent);
        }
        if line.len() > indent.len() {
            line.push(' ');
        }
        line.push_str(&piece);
        if i + 1 == items.len() {
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }
    out
}

fn array(name: &str, doc: &str, items: &[&str]) -> String {
    format!("{doc}export const {name} = [\n{}];\n\n", wrap(items, "  "))
}

fn set(name: &str, doc: &str, items: &[&str]) -> String {
    if items.is_empty() {
        return format!("{doc}export const {name} = new Set();\n\n");
    }
    format!(
        "{doc}export const {name} = new Set([\n{}]);\n\n",
        wrap(items, "  ")
    )
}

/// The whole module, header and all.
pub fn render(v: &Vocabulary) -> String {
    let mut s = String::from(HEADER);

    s.push_str(&array(
        "STATUSES",
        "/// Every member of `OrderStatus`, in the order the enum declares them.\n",
        &v.statuses,
    ));
    s.push_str(&set(
        "TERMINAL",
        "/// `OrderStatus::is_terminal` — the order has stopped moving.\n",
        &v.terminal,
    ));
    s.push_str(&set(
        "REFUSED",
        "/// The complement of `OrderStatus::took_money` — the order ended with the\n\
         /// venue holding none of the customer's money. This is the set three\n\
         /// surfaces spelled out by hand as `['REJECTED', 'CANCELLED']`, each of\n\
         /// them short by `COMPENSATED_REFUND`.\n",
        &v.refused,
    ));
    s.push_str(&set(
        "ACTIVE",
        "/// `OrderStatus::is_active` — accepted, not finished. `PENDING` is not\n\
         /// active: it is a decision the venue still owes the customer.\n",
        &v.active,
    ));
    s.push_str(&set(
        "SCAFFOLD",
        "/// States the kernel refuses to enter or leave at all. A surface must not\n\
         /// offer one as a step, and `vocabulary.sh` still wants a word for it\n\
         /// because the FSM can be asked about it.\n",
        &v.scaffold,
    ));

    s.push_str(&format!(
        "/// The transition table, recovered by asking `assert_transition` about every\n\
         /// ordered pair of statuses ({} of them, {} accepted). `NEXT[s]` is every\n\
         /// status the kernel will let `s` become; an empty list is a dead end.\n\
         export const NEXT = {{\n",
        v.statuses.len() * v.statuses.len(),
        v.edges
    ));
    for (from, tos) in &v.next {
        if tos.is_empty() {
            s.push_str(&format!("  {from}: [],\n"));
        } else {
            let joined = tos
                .iter()
                .map(|t| format!("'{t}'"))
                .collect::<Vec<_>>()
                .join(", ");
            s.push_str(&format!("  {from}: [{joined}],\n"));
        }
    }
    s.push_str("};\n\n");

    s.push_str(&array(
        "KINDS",
        "/// Every way an order can reach its customer, from `dowiz-core::fulfilment`.\n",
        &v.fulfilment_kinds,
    ));

    s.push_str(&array(
        "CURRENCIES",
        "/// Every code `money::Currency::from_code` accepts, found by exhaustion\n\
         /// over [A-Z]{1,4} rather than retyped.\n",
        &v.currencies,
    ));

    // Render the DECIMALS object (minor units per currency)
    s.push_str(
        "/// Minor units (decimal places) per currency, from `Currency::minor_units`.\n\
         export const DECIMALS = {\n"
    );
    for (code, units) in &v.minor_units {
        s.push_str(&format!("  {code}: {},\n", units));
    }
    s.push_str("};\n\n");

    s.push_str(&format!(
        "/// `money::MONEY_SCALE_MICRO` — the one scale the rate endpoint, the Worker\n\
         /// and `lib/money.js` all divide by. Parts per million of the target\n\
         /// currency per one unit of the base.\n\
         export const MONEY_SCALE_MICRO = {};\n\n",
        v.money_scale_micro
    ));

    s.push_str(TAIL);
    s
}

const HEADER: &str = "\
// GENERATED FILE — DO NOT EDIT.
//
// Emitted by `tools/gen-vocab`, which LINKS the kernel and calls it:
//
//     cd tools/gen-vocab && cargo run > ../../workers/api/public/lib/vocab.js
//
// `tools/gates/vocab.sh` regenerates into a temporary file and refuses any tree
// where the two differ, so this file cannot drift from the kernel the way a
// hand copy does.
//
// SOURCES, by name, so a reviewer can go and read them:
//   crates/dowiz-core/src/order_machine.rs — OrderStatus, is_terminal,
//     took_money, is_active, assert_transition, FSM_GOLDEN_SIGNATURE
//   crates/dowiz-core/src/money.rs — Currency, MONEY_SCALE_MICRO, minor_units
//   crates/dowiz-core/src/fulfilment.rs — KINDS, ALL
//
// THE DEFECT THIS CLOSES. `OrderStatus` has twelve members. The owner console
// held `DEAD = new Set(['REJECTED', 'CANCELLED'])`, the storefront's sea held
// the same set again under the name `FAILED`, the storefront's tracker held a
// third copy, and the kit spelled the test out inline — four copies of one
// kernel predicate, and every one of them short by `COMPENSATED_REFUND`, the
// terminal state the FSM has been able to reach since the P07 compensation
// edges landed. Nothing emits it yet, so nothing looked broken; the day a
// refund route lands, a refunded order counts as money the venue kept, on
// every screen at once. `tools/gates/vocabulary.sh` catches the missing WORD
// and the missing COLOUR. Only generating the list catches the missing MEMBER.
//
// WHAT IS DELIBERATELY NOT HERE. Translations (`admin/i18n.js`, `store/i18n.js`)
// and the colour per status (three stylesheets) are human choices, not kernel
// facts, and `vocabulary.sh` already refuses a status that is missing either.
// The happy-path progression the consoles draw as a stepper is a presentation
// decision too: the FSM's longest path from PENDING ties between DELIVERED and
// COMPENSATED_REFUND, so there is nothing here to derive it from.

";

const TAIL: &str = "\
/// The predicates, so a surface asks the question instead of holding the set.
export const isTerminal = s => TERMINAL.has(s);
export const isRefused = s => REFUSED.has(s);
export const isActive = s => ACTIVE.has(s);
/// Every status `s` may legally become. An UNKNOWN status answers `[]` rather
/// than `undefined`: a client that has not been redeployed since a member was
/// added must offer no step, not throw while drawing a row.
export const nextOf = s => NEXT[s] || [];
export const canAdvance = (from, to) => nextOf(from).includes(to);
";
