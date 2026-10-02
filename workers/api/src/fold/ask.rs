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
