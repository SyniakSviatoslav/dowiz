//! The wallet journal — the transport for `dowiz_kernel::ledger_account`.
//!
//! **This module decides nothing.** Whether a set of postings is balanced,
//! whether a refund exactly negates what it names, what an account holds — every
//! one of those is the kernel's answer, and it is asked BEFORE anything is
//! written. SQLite cannot express "these child rows must sum to zero", so a
//! transaction is validated in Rust and only then persisted.
//!
//! # A balance is replayed, never stored
//!
//! There is no `balance` column anywhere in the schema and there must never be
//! one. [`balance`] sums the postings, the same way an order's state is
//! `fold(events)` rather than a column somebody wrote. A stored balance is a
//! number that can disagree with its own history, and when it does, nobody can
//! tell which one is the money.
//!
//! # What is not here
//!
//! Taking money from a card. That is a payment rail, it belongs behind the
//! provider seam the wallet module already defines, and a top-up posted here
//! without one would be money this system invented. [`top_up`] therefore takes
//! a `providerRef` and refuses without it.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use dowiz_kernel::ledger_account::{self, Account, Journal, Posting, Transaction, TxKind};
use dowiz_kernel::money::{Currency, Money};

use crate::owner::now_ms;

#[derive(Deserialize)]
struct TxRow {
    id: String,
    kind: String,
    reverses: Option<String>,
    memo: String,
    at_ms: i64,
}

#[derive(Deserialize)]
struct PostingRow {
    tx_id: String,
    account: String,
    minor: i64,
    currency: String,
}

fn parse_account(s: &str) -> Option<Account> {
    if let Some(rest) = s.strip_prefix("wallet:") {
        return rest.parse().ok().map(Account::Wallet);
    }
    if let Some(rest) = s.strip_prefix("venue:") {
        return rest.parse().ok().map(Account::Venue);
    }
    match s {
        "platform" => Some(Account::Platform),
        "external" => Some(Account::External),
        _ => None,
    }
}

fn parse_kind(s: &str) -> Option<TxKind> {
    match s {
        "TOP_UP" => Some(TxKind::TopUp),
        "SPEND" => Some(TxKind::Spend),
        "REFUND" => Some(TxKind::Refund),
        "PAYOUT" => Some(TxKind::Payout),
        "FEE" => Some(TxKind::Fee),
        _ => None,
    }
}

fn kind_str(k: TxKind) -> &'static str {
    match k {
        TxKind::TopUp => "TOP_UP",
        TxKind::Spend => "SPEND",
        TxKind::Refund => "REFUND",
        TxKind::Payout => "PAYOUT",
        TxKind::Fee => "FEE",
    }
}

fn id64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Bind an integer to D1.
///
/// NOT `i64::into()`. That produces a JavaScript **BigInt**, which the D1 driver
/// rejects — and rejects by throwing, so the Worker returns a bare 500 with no
/// body and nothing in the response says why. Every integer in this file goes
/// through here, as `accounts.rs` already does. Timestamps in milliseconds and
/// slot minutes are far below 2^53, so f64 carries them exactly.
fn num(n: i64) -> worker::wasm_bindgen::JsValue {
    worker::wasm_bindgen::JsValue::from_f64(n as f64)
}


/// Rebuild the kernel's journal from rows.
///
/// A row the kernel refuses stops the load. Reporting a balance computed from a
/// journal that does not replay would be reporting a number nobody can defend.
async fn load_journal(db: &D1Database, location: &str) -> Result<std::result::Result<Journal, String>> {
    let txs: Vec<TxRow> = db
        .prepare(
            "SELECT id, kind, reverses, memo, at_ms FROM ledger_tx \
             WHERE location_id = ?1 ORDER BY at_ms ASC, id ASC",
        )
        .bind(&[location.into()])?
        .all()
        .await?
        .results()?;
    let postings: Vec<PostingRow> = db
        .prepare(
            "SELECT tx_id, account, minor, currency FROM ledger_postings \
             WHERE location_id = ?1 ORDER BY id ASC",
        )
        .bind(&[location.into()])?
        .all()
        .await?
        .results()?;

    let mut journal = Journal::new();
    for t in &txs {
        let Some(kind) = parse_kind(&t.kind) else {
            return Ok(Err(format!("unknown transaction kind {:?}", t.kind)));
        };
        let mut legs = Vec::new();
        for p in postings.iter().filter(|p| p.tx_id == t.id) {
            let Some(account) = parse_account(&p.account) else {
                return Ok(Err(format!("unknown account {:?}", p.account)));
            };
            let Some(currency) = Currency::from_code(&p.currency) else {
                return Ok(Err(format!("unknown currency {:?}", p.currency)));
            };
            legs.push(Posting {
                account,
                amount: Money::new(p.minor, currency),
            });
        }
        let tx = Transaction {
            id: id64(&t.id),
            kind,
            reverses: t.reverses.as_deref().map(id64),
            postings: legs,
            memo: t.memo.clone(),
        };
        match ledger_account::post(journal, tx) {
            Ok(next) => journal = next,
            Err(e) => return Ok(Err(e)),
        }
    }
    Ok(Ok(journal))
}

/// `GET /api/public/locations/:slug/wallet?user=<id>`
pub async fn balance(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let named = req
        .url()?
        .query_pairs()
        .find(|(k, _)| k == "user")
        .map(|(_, v)| v.to_string());

    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, TO THIS VENUE, AND TO THIS WALLET. The first two were the
    // red-team fix; the third is this one. See `wallet_who`.
    let principal =
        match crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, now_ms()).await {
            Ok(p) => p,
            Err(r) => return Ok(r),
        };
    let is_owner = matches!(principal, crate::auth::Principal::Owner { .. });
    let own = own_wallet_key(&principal, &place, &ctx.env).await?;
    let user = match wallet_who(is_owner, named.as_deref(), own.as_deref()) {
        Ok(WalletWho::Named(u)) => u,
        Ok(WalletWho::Own) => own.unwrap_or_default(),
        Err(why) => return Response::error(why, 403),
    };
    let journal = match load_journal(&db, &place.venue).await? {
        Ok(j) => j,
        Err(why) => return Response::error(format!("journal unreadable: {why}"), 500),
    };

    let wallet = Account::Wallet(id64(&user));
    let held = ledger_account::balance_of(&journal, wallet)
        .map_err(|e| Error::RustError(format!("balance: {e}")))?;

    // The conservation probe, reported rather than assumed. Every currency in
    // the journal must net to exactly zero; anything else means money was
    // created or destroyed and the caller gets to see it.
    let sums = ledger_account::journal_sum(&journal)
        .map_err(|e| Error::RustError(format!("journal sum: {e}")))?;
    let conserved = sums.iter().all(|(_, n)| *n == 0);

    Response::from_json(&json!({
        // `null` means the wallet has never been used — which is not the same
        // as zero, and a screen should be able to tell the two apart.
        "balanceMinor": held.map(|m| m.minor),
        "currency": held.map(|m| m.currency.code()),
        "conserved": conserved,
        "transactions": ledger_account::statement(&journal, wallet).len(),
    }))
}

#[derive(Deserialize)]
struct TopUpBody {
    user: String,
    #[serde(rename = "amountMinor")]
    amount_minor: i64,
    currency: String,
    /// The payment rail's own reference. Without one this would be money the
    /// system invented, so it is required and not defaulted.
    #[serde(rename = "providerRef")]
    provider_ref: String,
    #[serde(rename = "requestId")]
    request_id: String,
}

/// `POST /api/public/locations/:slug/wallet/topup`
pub async fn top_up(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    // ── MAY YOU, BEFORE WHAT DID YOU SEND ──
    //
    // THE OWNER, AND ONLY THE OWNER. This route wrote balanced ledger rows --
    // the money this system treats as authoritative -- for any amount any
    // caller asked for, with `providerRef` as its only "proof" and the caller
    // writing that too. Until a payment provider's webhook signs the top-up
    // (as `stripe::webhook` already does for orders), the venue's own console
    // is the only thing allowed to move this ledger.
    //
    // ASKED FIRST, because the order of these checks is a message. A customer
    // whose app posted here learned "missing field `user`" and then, once that
    // was supplied, "providerRef is required" -- two invitations to try harder
    // at a door that was never going to open. It also stops an unauthorised
    // caller costing this Worker a body parse and a currency lookup.
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    match crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, now_ms()).await {
        Ok(crate::auth::Principal::Owner { .. }) => {}
        Ok(_) => return Response::error("only the venue can record a top-up", 403),
        Err(r) => return Ok(r),
    }

    let b: TopUpBody = match req.json().await {
        Ok(v) => v,
        Err(e) => return Response::error(format!("bad request: {e}"), 400),
    };
    if b.provider_ref.trim().is_empty() {
        return Response::error(
            "providerRef is required: a top-up without a payment behind it would be money \
             this system invented",
            422,
        );
    }
    if b.request_id.trim().is_empty() {
        return Response::error("requestId is required", 400);
    }
    let Some(currency) = Currency::from_code(&b.currency) else {
        return Response::error(format!("unknown currency {:?}", b.currency), 400);
    };

    let tx_id = format!("tx_{:016x}", id64(&format!("{}:{}", place.venue, b.request_id)));
    let already: Option<serde_json::Value> = db
        .prepare("SELECT id FROM ledger_tx WHERE id = ?1")
        .bind(&[tx_id.clone().into()])?
        .first(None)
        .await?;
    if already.is_some() {
        return Response::from_json(&json!({ "id": tx_id, "replayed": true }));
    }

    // ── THE DECISION IS THE KERNEL'S ──
    let wallet_id = id64(&b.user);
    let tx = match ledger_account::top_up(
        id64(&tx_id),
        wallet_id,
        Money::new(b.amount_minor, currency),
        &b.provider_ref,
    ) {
        Ok(t) => t,
        Err(e) => return Response::error(e, 422),
    };
    // Belt and braces: the journal's own validator runs before a row is written,
    // so an unbalanced transaction can never reach the database even if the
    // builder above were changed to produce one.
    if let Err(e) = ledger_account::validate(&tx) {
        return Response::error(e, 500);
    }

    let now = now_ms();
    let mut stmts = vec![db
        .prepare(
            "INSERT INTO ledger_tx (id, location_id, kind, reverses, memo, at_ms) \
             VALUES (?1,?2,?3,NULL,?4,?5)",
        )
        .bind(&[
            tx_id.clone().into(),
            place.venue.clone().into(),
            kind_str(tx.kind).into(),
            b.provider_ref.clone().into(),
            num(now),
        ])?];
    for p in &tx.postings {
        stmts.push(
            db.prepare(
                "INSERT INTO ledger_postings (tx_id, location_id, account, minor, currency) \
                 VALUES (?1,?2,?3,?4,?5)",
            )
            .bind(&[
                tx_id.clone().into(),
                place.venue.clone().into(),
                p.account.as_str().into(),
                num(p.amount.minor),
                p.amount.currency.code().into(),
            ])?,
        );
    }
    db.batch(stmts).await?;

    Response::from_json(&json!({
        "id": tx_id,
        "replayed": false,
        "amountMinor": b.amount_minor,
        "currency": currency.code(),
    }))
}



/// The wallet key a principal owns, or `None` for one that owns no wallet.
///
/// A customer token names ONE ORDER, which is the only identity this product
/// mints for a customer. The order carries the phone they gave, and
/// `customer_key` is the venue's own non-reversible handle for it — the same
/// one the customer list and the reveal audit use, so a wallet and a customer
/// row are the same person.
async fn own_wallet_key(
    p: &crate::auth::Principal,
    place: &crate::hubstore::Place,
    env: &Env,
) -> Result<Option<String>> {
    let crate::auth::Principal::Customer { order_id, .. } = p else { return Ok(None) };
    let Some(raw) = crate::hubstore::order(place, order_id).await? else { return Ok(None) };
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
    let phone = v.get("contact").and_then(|c| c.get("phone")).and_then(|x| x.as_str()).unwrap_or("");
    if phone.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(crate::extra::customer_key(&crate::extra::signing_secret(env), phone)))
}

// ── WHOSE WALLET ────────────────────────────────────────────────────────────

/// What a caller is allowed to read.
#[derive(Debug, PartialEq, Eq)]
pub enum WalletWho {
    /// The principal's own wallet; the handler derives the key.
    Own,
    /// A wallet the caller named. Only an owner may.
    Named(String),
}

/// THE IDENTITY COMES FROM THE TOKEN, NOT FROM THE QUERY.
///
/// The red-team pass put these routes behind `principal_at`, which asks only
/// "does this principal belong to this venue". `?user=` stayed exactly as it
/// was: a string the CALLER chooses. So the hole narrowed from "anyone" to
/// "anyone with an order at this venue" — and an order token is minted for
/// every customer who has ever bought a coffee here. One of them could read
/// any other's balance and their whole transaction history by naming them.
///
/// An owner may still name a customer, because an owner's job includes looking
/// one up and they already see the customer list. Everybody else gets their
/// own, and a `?user=` that does not match is a contradiction rather than a
/// preference — refused, not quietly resolved.
pub fn wallet_who(
    is_owner: bool,
    named: Option<&str>,
    own_key: Option<&str>,
) -> std::result::Result<WalletWho, &'static str> {
    match (is_owner, named) {
        (true, Some(u)) => Ok(WalletWho::Named(u.to_string())),
        (true, None) => Err("which customer?"),
        // A principal with no wallet identity of its own — a courier — cannot
        // be given one by asking.
        (false, _) if own_key.is_none() => Err("this token has no wallet"),
        (false, Some(u)) if Some(u) != own_key => Err("that is not your wallet"),
        (false, _) => Ok(WalletWho::Own),
    }
}

#[cfg(test)]
mod who_tests {
    use super::{wallet_who, WalletWho};

    #[test]
    fn an_owner_names_the_customer() {
        assert_eq!(wallet_who(true, Some("abc"), None), Ok(WalletWho::Named("abc".into())));
        assert!(wallet_who(true, None, None).is_err());
    }

    /// THE BUG THIS ENCODES: an authenticated customer naming somebody else.
    #[test]
    fn a_customer_cannot_name_another_wallet() {
        assert!(wallet_who(false, Some("someone-else"), Some("mine")).is_err());
    }

    #[test]
    fn a_customer_gets_their_own_with_or_without_naming_it() {
        assert_eq!(wallet_who(false, None, Some("mine")), Ok(WalletWho::Own));
        assert_eq!(wallet_who(false, Some("mine"), Some("mine")), Ok(WalletWho::Own));
    }

    #[test]
    fn a_principal_with_no_wallet_is_refused_rather_than_invented() {
        assert!(wallet_who(false, None, None).is_err());
        assert!(wallet_who(false, Some("anything"), None).is_err());
    }
}

/// `GET /api/public/locations/:slug/wallet/statement?user=<id>`
pub async fn statement(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let named = req
        .url()?
        .query_pairs()
        .find(|(k, _)| k == "user")
        .map(|(_, v)| v.to_string());

    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    // AUTHENTICATED, TO THIS VENUE, AND TO THIS WALLET. The first two were the
    // red-team fix; the third is this one. See `wallet_who`.
    let principal =
        match crate::auth::principal_at(&req, &ctx.env, &db, &place.venue, now_ms()).await {
            Ok(p) => p,
            Err(r) => return Ok(r),
        };
    let is_owner = matches!(principal, crate::auth::Principal::Owner { .. });
    let own = own_wallet_key(&principal, &place, &ctx.env).await?;
    let user = match wallet_who(is_owner, named.as_deref(), own.as_deref()) {
        Ok(WalletWho::Named(u)) => u,
        Ok(WalletWho::Own) => own.unwrap_or_default(),
        Err(why) => return Response::error(why, 403),
    };
    let journal = match load_journal(&db, &place.venue).await? {
        Ok(j) => j,
        Err(why) => return Response::error(format!("journal unreadable: {why}"), 500),
    };

    let wallet = Account::Wallet(id64(&user));
    let rows: Vec<_> = ledger_account::statement(&journal, wallet)
        .iter()
        .map(|t| {
            // What this movement did to THIS wallet, which is the only figure a
            // statement line should show.
            let net: i64 = t
                .postings
                .iter()
                .filter(|p| p.account == wallet)
                .map(|p| p.amount.minor)
                .sum();
            json!({
                "kind": kind_str(t.kind),
                "memo": t.memo,
                "minor": net,
            })
        })
        .collect();

    Response::from_json(&json!({ "statement": rows }))
}
