//! D13 (G6): WHOSE WALLET A PAYMENT MAY SPEND.
//!
//! THE DEFECT. `PayBody.wallet` went straight to `wallet::id64(user)` and the
//! only check was the balance: a waiter could debit any customer's wallet by
//! typing its id -- a balance the read side (`wallet::wallet_who`) refused to
//! show that same waiter.
//!
//! THE RULE is the read side's, one function for both (`wallet_who`): the
//! owner may name a customer's wallet, because the owner already sees the
//! customer list; anyone else spends only the wallet of the customer whose
//! own token was PRESENTED with the payment (`wallet_token`, the customer's
//! order token from their phone), and a typed id that disagrees with it is a
//! contradiction, refused rather than quietly resolved.

use crate::wallet::{wallet_who, WalletWho};
use worker::*;

/// The wallet key this payment spends, or the refusal.
/// `presented` is the wallet key of the customer token shown with it.
pub fn payer(is_owner: bool, named: Option<&str>, presented: Option<&str>) -> std::result::Result<String, (u16, &'static str)> {
    if !is_owner && presented.is_none() {
        return Err((403, "a wallet is spent with the customer's own code, not a typed id"));
    }
    match wallet_who(is_owner, named, presented) {
        Ok(WalletWho::Named(key)) => Ok(key),
        Ok(WalletWho::Own) => presented.map(str::to_string).ok_or((403, "this token has no wallet")),
        Err(why) => Err((403, why)),
    }
}

/// The wallet key of a presented customer token: it must verify, belong to
/// this venue, and name an order with a phone (`wallet::own_wallet_key`).
/// `None` for no token, or one that is not a customer's of this venue.
pub async fn presented(env: &Env, place: &crate::hubstore::Place, token: Option<&str>, now_ms: i64) -> Result<Option<String>> {
    let Some(raw) = token.map(str::trim).filter(|t| !t.is_empty()) else { return Ok(None) };
    let Ok(p) = crate::auth::authenticate_token(raw, env, now_ms).await else { return Ok(None) };
    if !matches!(p, crate::auth::Principal::Customer { .. }) || !crate::auth::belongs_to(&p, &place.venue) {
        return Ok(None);
    }
    crate::wallet::own_wallet_key(&p, place, env).await
}

#[cfg(test)]
mod tests;
