//! ONE GET OF A `/fold/*` ROUTE, as the Worker's side of a move into the
//! object reads it (BN1): the status and the body, so the handler passes a
//! refusal through with the object's own words and status, and a 200 as the
//! JSON it already is. Beside `menu_edge` in purpose; here so that one does
//! not grow a client per route.

use crate::hubstore::Place;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// `GET url` on the venue's object: `(status, body)`. An unreachable object
/// is an error; a refusal is an answer.
pub async fn text(place: &Place, url: &str) -> Result<(u16, String)> {
    let mut res = place.stub()?.fetch_with_request(Request::new(url, Method::Get)?).await?;
    Ok((res.status_code(), res.text().await?))
}

/// `GET url` as the JSON it answers. A refusal IS an error here: these are
/// reads of the venue's own images, and the venue was checked by the caller,
/// so a non-200 is the object failing, not the caller being told no.
pub async fn json(place: &Place, url: &str) -> Result<serde_json::Value> {
    let (status, body) = text(place, url).await?;
    if status != 200 {
        return Err(Error::RustError(format!("hub object refused {url}: {status} {body}")));
    }
    serde_json::from_str(&body).map_err(|e| Error::RustError(format!("hub object answered {url} unreadably: {e}")))
}

/// One derived node of the catalogue (`hubdo/catalogue.rs`, BN1): `query` is
/// the `q=<what>&...` the object dispatches on, already URL-encoded.
pub async fn catalogue(place: &Place, query: &str) -> Result<serde_json::Value> {
    json(place, &format!("https://hub/fold/catalogue?{query}")).await
}
