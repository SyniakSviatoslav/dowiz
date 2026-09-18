//! Account-scoped double-entry journal — where a wallet balance actually lives.
//!
//! [`crate::money`] already carries the reversal Law: an `Earn` leg and its
//! `Reversal` net to exactly zero. What it does not carry is an ACCOUNT
//! dimension, and a wallet balance is meaningless without one — "how much money
//! is there" is not a question until you say whose.
//!
//! So this module adds the second dimension and nothing else. It does not invent
//! a new money type, a new currency, or a new arithmetic: every amount is
//! [`Money`], every addition is `checked_add`, and there is not one float in the
//! file. A wallet top-up is a two-posting transaction, a spend is a two-posting
//! transaction, and a refund is the same transaction negated.
//!
//! # The invariant
//!
//! **Every transaction sums to exactly zero.** That is the whole design. Money
//! is never created or destroyed by a posting — it moves between accounts, and
//! [`Account::External`] is where it crosses the boundary of the system, so the
//! journal as a whole also sums to zero. [`journal_sum`] is the falsifier and a
//! test asserts it after every operation this module offers.
//!
//! A balance is therefore never stored. It is [`balance_of`] — a replay, the
//! same way an order's state is `fold(events)` rather than a column somebody
//! wrote. A stored balance is a number that can disagree with its own history.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::money::{Currency, Money};

// ─────────────────────────────────────────────────────────────────────────────
// Accounts
// ─────────────────────────────────────────────────────────────────────────────

/// Who holds money.
///
/// Deliberately NOT `Ord`: accounts are named, not ranked, and sorting them
/// would invite "the biggest account" to mean something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Account {
    /// A customer's wallet inside one venue's hub.
    Wallet(u64),
    /// A venue's own account — what it has earned and not yet been paid out.
    Venue(u64),
    /// dowiz's account, for a platform fee a venue has agreed to.
    Platform,
    /// The world outside the ledger: a card rail, cash in a courier's hand, a
    /// bank transfer. Money enters and leaves the system through here, which is
    /// why the journal can sum to zero while a wallet holds a positive balance.
    External,
}

impl Account {
    pub fn as_str(&self) -> String {
        match self {
            Self::Wallet(id) => format!("wallet:{id}"),
            Self::Venue(id) => format!("venue:{id}"),
            Self::Platform => String::from("platform"),
            Self::External => String::from("external"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Postings and transactions
// ─────────────────────────────────────────────────────────────────────────────

/// One leg: an amount moving into (positive) or out of (negative) an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posting {
    pub account: Account,
    pub amount: Money,
}

/// Why money moved. Carried for the venue's own records; it changes no
/// arithmetic, which is why an unknown reason cannot make a transaction unbalance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxKind {
    /// Money entering a wallet from outside (a card, cash at the counter).
    TopUp,
    /// A wallet paying a venue for an order.
    Spend,
    /// The exact compensation of an earlier transaction.
    Refund,
    /// A venue being paid out of the system.
    Payout,
    /// A fee the venue has agreed to, moving to the platform.
    Fee,
}

/// A balanced set of postings. `id` is caller-assigned and is the idempotency
/// key: posting the same id twice is refused, so a replayed request is a no-op
/// at the caller rather than a second movement of money.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub id: u64,
    pub kind: TxKind,
    /// For a `Refund`: the id of the transaction being compensated.
    pub reverses: Option<u64>,
    pub postings: Vec<Posting>,
    /// Free reference to whatever caused this — an order id, a reservation id.
    /// Never interpreted here.
    pub memo: String,
}

/// The journal: every transaction, in the order it was accepted.
pub type Journal = Vec<Transaction>;

// ─────────────────────────────────────────────────────────────────────────────
// The Law
// ─────────────────────────────────────────────────────────────────────────────

/// Check that a transaction is internally sound, before it touches the journal.
///
/// Fail-closed on: an empty posting list, mixed currencies, arithmetic overflow,
/// and — the one that matters — a set of postings that does not sum to zero.
pub fn validate(tx: &Transaction) -> Result<(), String> {
    if tx.postings.is_empty() {
        return Err(format!("transaction {}: no postings", tx.id));
    }
    let currency = tx.postings[0].amount.currency;
    let mut total = Money::new(0, currency);
    for p in &tx.postings {
        if p.amount.currency != currency {
            return Err(format!(
                "transaction {}: mixes {} and {}",
                tx.id,
                currency.code(),
                p.amount.currency.code()
            ));
        }
        total = total.checked_add(p.amount)?;
    }
    if total.minor != 0 {
        return Err(format!(
            "transaction {}: postings sum to {}, not 0 — money would be created",
            tx.id, total.minor
        ));
    }
    Ok(())
}

/// Append a transaction to the journal.
///
/// Refuses a duplicate id, an unbalanced transaction, and a `Refund` that does
/// not exactly negate the transaction it names. A transaction may be refunded at
/// most once — a second distinct refund of the same movement is refused rather
/// than quietly paying twice.
pub fn post(mut journal: Journal, tx: Transaction) -> Result<Journal, String> {
    validate(&tx)?;
    if journal.iter().any(|t| t.id == tx.id) {
        return Err(format!("journal: duplicate transaction id {}", tx.id));
    }
    if tx.kind == TxKind::Refund {
        let target = tx
            .reverses
            .ok_or_else(|| format!("transaction {}: a refund must name what it reverses", tx.id))?;
        let original = journal
            .iter()
            .find(|t| t.id == target)
            .ok_or_else(|| format!("transaction {}: reverses unknown id {target}", tx.id))?;
        let expected = negate(original)?;
        if !same_postings(&expected, &tx.postings) {
            return Err(format!(
                "transaction {}: refund is not the exact negation of {target}",
                tx.id
            ));
        }
        if journal.iter().any(|t| t.reverses == Some(target)) {
            return Err(format!("transaction {target} is already refunded"));
        }
    }
    journal.push(tx);
    Ok(journal)
}

/// Build the exact compensating postings of a transaction.
fn negate(tx: &Transaction) -> Result<Vec<Posting>, String> {
    tx.postings
        .iter()
        .map(|p| {
            Ok(Posting {
                account: p.account,
                amount: p.amount.checked_neg()?,
            })
        })
        .collect()
}

/// Two posting lists are the same movement if each account's net is the same.
/// Order does not matter; a split leg does.
fn same_postings(a: &[Posting], b: &[Posting]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut used = alloc::vec![false; b.len()];
    for x in a {
        let mut found = false;
        for (i, y) in b.iter().enumerate() {
            if !used[i] && x.account == y.account && x.amount == y.amount {
                used[i] = true;
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }
    true
}

/// Build the refund of a transaction already in the journal.
pub fn refund_of(journal: &Journal, target_id: u64, refund_id: u64) -> Result<Transaction, String> {
    let original = journal
        .iter()
        .find(|t| t.id == target_id)
        .ok_or_else(|| format!("no transaction {target_id} to refund"))?;
    if original.kind == TxKind::Refund {
        return Err(format!("transaction {target_id} is itself a refund"));
    }
    Ok(Transaction {
        id: refund_id,
        kind: TxKind::Refund,
        reverses: Some(target_id),
        postings: negate(original)?,
        memo: original.memo.clone(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Reading the journal
// ─────────────────────────────────────────────────────────────────────────────

/// What one account holds, replayed from the journal.
///
/// Returns `Ok(None)` when the account has no postings at all — which is not the
/// same as a zero balance, and a wallet that has never been used should not
/// claim a currency it was never given.
pub fn balance_of(journal: &Journal, account: Account) -> Result<Option<Money>, String> {
    let mut total: Option<Money> = None;
    for tx in journal {
        for p in &tx.postings {
            if p.account != account {
                continue;
            }
            total = Some(match total {
                None => p.amount,
                Some(t) => t.checked_add(p.amount)?,
            });
        }
    }
    Ok(total)
}

/// The whole journal's net. **This must be exactly zero**, always: money only
/// moves between accounts, and anything else is a bug that created or destroyed
/// some. Cross-currency journals are summed per currency, and any currency that
/// does not net to zero is reported.
pub fn journal_sum(journal: &Journal) -> Result<Vec<(Currency, i64)>, String> {
    let mut out: Vec<(Currency, i64)> = Vec::new();
    for tx in journal {
        for p in &tx.postings {
            match out.iter_mut().find(|(c, _)| *c == p.amount.currency) {
                Some((_, n)) => {
                    *n = n
                        .checked_add(p.amount.minor)
                        .ok_or("journal sum overflow")?;
                }
                None => out.push((p.amount.currency, p.amount.minor)),
            }
        }
    }
    Ok(out)
}

/// Every transaction touching an account, newest last — a wallet's statement.
pub fn statement(journal: &Journal, account: Account) -> Vec<&Transaction> {
    journal
        .iter()
        .filter(|t| t.postings.iter().any(|p| p.account == account))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// The movements the product actually makes
// ─────────────────────────────────────────────────────────────────────────────

/// Money entering a wallet from outside. The customer's wallet gains it and
/// [`Account::External`] loses it, so the journal still nets to zero.
pub fn top_up(id: u64, wallet: u64, amount: Money, memo: &str) -> Result<Transaction, String> {
    if amount.minor <= 0 {
        return Err(format!("top-up must be positive, got {}", amount.minor));
    }
    Ok(Transaction {
        id,
        kind: TxKind::TopUp,
        reverses: None,
        postings: alloc::vec![
            Posting { account: Account::Wallet(wallet), amount },
            Posting { account: Account::External, amount: amount.checked_neg()? },
        ],
        memo: String::from(memo),
    })
}

/// A wallet paying a venue, with an optional platform fee taken out of the
/// venue's side — never out of the customer's, who agreed to one number.
pub fn spend(
    id: u64,
    wallet: u64,
    venue: u64,
    amount: Money,
    fee: Option<Money>,
    memo: &str,
) -> Result<Transaction, String> {
    if amount.minor <= 0 {
        return Err(format!("a spend must be positive, got {}", amount.minor));
    }
    let mut postings = alloc::vec![Posting {
        account: Account::Wallet(wallet),
        amount: amount.checked_neg()?,
    }];
    match fee {
        Some(f) if f.minor > 0 => {
            if f.minor > amount.minor {
                return Err(format!(
                    "fee {} exceeds the amount {} — the venue would owe money on a sale",
                    f.minor, amount.minor
                ));
            }
            postings.push(Posting {
                account: Account::Venue(venue),
                amount: amount.checked_sub(f)?,
            });
            postings.push(Posting {
                account: Account::Platform,
                amount: f,
            });
        }
        _ => postings.push(Posting {
            account: Account::Venue(venue),
            amount,
        }),
    }
    Ok(Transaction {
        id,
        kind: TxKind::Spend,
        reverses: None,
        postings,
        memo: String::from(memo),
    })
}

/// Whether a wallet can cover an amount. Answers `false` for an untouched wallet
/// and for the wrong currency, rather than guessing either.
pub fn can_cover(journal: &Journal, wallet: u64, amount: Money) -> Result<bool, String> {
    match balance_of(journal, Account::Wallet(wallet))? {
        None => Ok(false),
        Some(b) if b.currency != amount.currency => Ok(false),
        Some(b) => Ok(b.minor >= amount.minor),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: Currency = Currency::All;
    fn m(n: i64) -> Money {
        Money::new(n, ALL)
    }

    fn zeroed(journal: &Journal) {
        for (c, n) in journal_sum(journal).unwrap() {
            assert_eq!(n, 0, "journal does not net to zero in {}", c.code());
        }
    }

    #[test]
    fn a_top_up_moves_money_and_conserves_it() {
        let j = post(Journal::new(), top_up(1, 7, m(50_000), "card").unwrap()).unwrap();
        assert_eq!(balance_of(&j, Account::Wallet(7)).unwrap(), Some(m(50_000)));
        assert_eq!(
            balance_of(&j, Account::External).unwrap(),
            Some(m(-50_000))
        );
        zeroed(&j);
    }

    #[test]
    fn an_untouched_wallet_has_no_balance_at_all() {
        let j = Journal::new();
        assert_eq!(balance_of(&j, Account::Wallet(9)).unwrap(), None);
        // Which is NOT the same as zero: the wallet has no currency yet.
        assert!(!can_cover(&j, 9, m(1)).unwrap());
    }

    #[test]
    fn spending_moves_the_money_to_the_venue() {
        let mut j = post(Journal::new(), top_up(1, 7, m(50_000), "card").unwrap()).unwrap();
        j = post(j, spend(2, 7, 3, m(12_000), None, "order#1").unwrap()).unwrap();
        assert_eq!(balance_of(&j, Account::Wallet(7)).unwrap(), Some(m(38_000)));
        assert_eq!(balance_of(&j, Account::Venue(3)).unwrap(), Some(m(12_000)));
        zeroed(&j);
    }

    #[test]
    fn a_fee_comes_out_of_the_venues_side() {
        let mut j = post(Journal::new(), top_up(1, 7, m(50_000), "card").unwrap()).unwrap();
        j = post(j, spend(2, 7, 3, m(10_000), Some(m(500)), "order#2").unwrap()).unwrap();
        // The customer paid exactly what they agreed to.
        assert_eq!(balance_of(&j, Account::Wallet(7)).unwrap(), Some(m(40_000)));
        assert_eq!(balance_of(&j, Account::Venue(3)).unwrap(), Some(m(9_500)));
        assert_eq!(balance_of(&j, Account::Platform).unwrap(), Some(m(500)));
        zeroed(&j);
    }

    #[test]
    fn a_fee_larger_than_the_sale_is_refused() {
        assert!(spend(1, 7, 3, m(100), Some(m(101)), "x").is_err());
    }

    #[test]
    fn an_unbalanced_transaction_is_refused() {
        let bad = Transaction {
            id: 1,
            kind: TxKind::TopUp,
            reverses: None,
            postings: alloc::vec![Posting {
                account: Account::Wallet(1),
                amount: m(1000)
            }],
            memo: String::new(),
        };
        let err = post(Journal::new(), bad).unwrap_err();
        assert!(err.contains("money would be created"), "{err}");
    }

    #[test]
    fn mixed_currencies_in_one_transaction_are_refused() {
        let bad = Transaction {
            id: 1,
            kind: TxKind::TopUp,
            reverses: None,
            postings: alloc::vec![
                Posting { account: Account::Wallet(1), amount: Money::new(100, Currency::All) },
                Posting { account: Account::External, amount: Money::new(-100, Currency::Eur) },
            ],
            memo: String::new(),
        };
        assert!(post(Journal::new(), bad).unwrap_err().contains("mixes"));
    }

    #[test]
    fn a_replayed_transaction_is_refused_not_repeated() {
        let tx = top_up(1, 7, m(1_000), "card").unwrap();
        let j = post(Journal::new(), tx.clone()).unwrap();
        assert!(post(j.clone(), tx).unwrap_err().contains("duplicate"));
        // The one accepted movement still stands.
        assert_eq!(balance_of(&j, Account::Wallet(7)).unwrap(), Some(m(1_000)));
    }

    #[test]
    fn a_refund_nets_the_original_to_zero() {
        let mut j = post(Journal::new(), top_up(1, 7, m(50_000), "card").unwrap()).unwrap();
        j = post(j, spend(2, 7, 3, m(12_000), None, "order#1").unwrap()).unwrap();
        let r = refund_of(&j, 2, 3).unwrap();
        j = post(j, r).unwrap();
        assert_eq!(balance_of(&j, Account::Wallet(7)).unwrap(), Some(m(50_000)));
        assert_eq!(balance_of(&j, Account::Venue(3)).unwrap(), Some(m(0)));
        zeroed(&j);
    }

    #[test]
    fn a_second_refund_of_the_same_movement_is_refused() {
        let mut j = post(Journal::new(), top_up(1, 7, m(9_000), "card").unwrap()).unwrap();
        j = post(j, spend(2, 7, 3, m(4_000), None, "order").unwrap()).unwrap();
        // The refund is built from the journal BEFORE the journal is moved into
        // `post`; argument evaluation order would otherwise borrow a moved value.
        let first = refund_of(&j, 2, 3).unwrap();
        j = post(j, first).unwrap();
        let second = refund_of(&j, 2, 4).unwrap();
        assert!(post(j, second).unwrap_err().contains("already refunded"));
    }

    #[test]
    fn a_refund_that_is_not_the_exact_negation_is_refused() {
        let mut j = post(Journal::new(), top_up(1, 7, m(9_000), "card").unwrap()).unwrap();
        j = post(j, spend(2, 7, 3, m(4_000), None, "order").unwrap()).unwrap();
        let mut sneaky = refund_of(&j, 2, 3).unwrap();
        // Still balanced — this is the dangerous shape. An unbalanced forgery is
        // caught earlier by `validate`; this one sums to zero and is only wrong
        // because it is not what the original movement was.
        sneaky.postings[0].amount = m(5_000);
        sneaky.postings[1].amount = m(-5_000);
        assert_eq!(validate(&sneaky), Ok(()), "the forgery must be balanced");
        assert!(post(j, sneaky)
            .unwrap_err()
            .contains("not the exact negation"));
    }

    #[test]
    fn a_refund_must_name_something_that_exists() {
        let j = Journal::new();
        let orphan = Transaction {
            id: 1,
            kind: TxKind::Refund,
            reverses: Some(99),
            postings: alloc::vec![
                Posting { account: Account::Wallet(1), amount: m(1) },
                Posting { account: Account::External, amount: m(-1) },
            ],
            memo: String::new(),
        };
        assert!(post(j, orphan).unwrap_err().contains("unknown id 99"));
    }

    #[test]
    fn can_cover_answers_from_the_replay_not_a_stored_number() {
        let mut j = post(Journal::new(), top_up(1, 7, m(1_000), "card").unwrap()).unwrap();
        assert!(can_cover(&j, 7, m(1_000)).unwrap());
        assert!(!can_cover(&j, 7, m(1_001)).unwrap());
        j = post(j, spend(2, 7, 3, m(600), None, "order").unwrap()).unwrap();
        assert!(can_cover(&j, 7, m(400)).unwrap());
        assert!(!can_cover(&j, 7, m(401)).unwrap());
    }

    #[test]
    fn a_wallet_in_another_currency_cannot_cover() {
        let j = post(Journal::new(), top_up(1, 7, m(10_000), "card").unwrap()).unwrap();
        assert!(!can_cover(&j, 7, Money::new(1, Currency::Eur)).unwrap());
    }

    #[test]
    fn a_statement_lists_only_what_touched_the_account() {
        let mut j = post(Journal::new(), top_up(1, 7, m(5_000), "card").unwrap()).unwrap();
        j = post(j, top_up(2, 8, m(5_000), "card").unwrap()).unwrap();
        j = post(j, spend(3, 7, 3, m(1_000), None, "order").unwrap()).unwrap();
        assert_eq!(statement(&j, Account::Wallet(7)).len(), 2);
        assert_eq!(statement(&j, Account::Wallet(8)).len(), 1);
        assert_eq!(statement(&j, Account::Venue(3)).len(), 1);
    }

    #[test]
    fn a_non_positive_top_up_is_refused() {
        assert!(top_up(1, 7, m(0), "x").is_err());
        assert!(top_up(1, 7, m(-100), "x").is_err());
    }

    #[test]
    fn there_is_no_float_in_this_module() {
        // Money is integer minor units, full stop (`DECISIONS.md`, money.rs).
        // The tokens are split so this check cannot match itself.
        let src = include_str!("ledger_account.rs");
        for token in [concat!("f", "64"), concat!("f", "32"), concat!("as ", "f", "64")] {
            assert!(
                !src.contains(token),
                "{token} appears — money arithmetic must stay integral"
            );
        }
    }

    #[test]
    fn overflow_fails_closed_rather_than_wrapping() {
        let j = post(
            Journal::new(),
            top_up(1, 7, Money::new(i64::MAX, ALL), "huge").unwrap(),
        )
        .unwrap();
        let second = top_up(2, 7, Money::new(i64::MAX, ALL), "huge again").unwrap();
        let j2 = post(j, second).unwrap();
        // The postings are individually fine; reading the balance is where the
        // sum would overflow, and it refuses instead of wrapping negative.
        assert!(balance_of(&j2, Account::Wallet(7)).is_err());
    }
}
