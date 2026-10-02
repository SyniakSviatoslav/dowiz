//! The Worker's side of R2: ask the venue's object for the DERIVED catalogue
//! nodes (`hubdo/menu.rs`) instead of pulling the catalogue image across the
//! hop. Beside `hubstore`'s other `/fold/*` clients in purpose, in its own file
//! so that one does not grow.

use crate::hubstore::Place;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// The storefront's menu, FOLDED IN THE OBJECT (R2, `hubdo/menu.rs`): the
/// body as bytes, to be passed through, or the status the object refused with
/// (404: no such venue at this slug). The catalogue image does not cross.
pub async fn menu_body(place: &Place, slug: &str, locale: Option<&str>, fresh: bool, now_ms: i64)
    -> Result<std::result::Result<String, u16>>
{
    let enc = crate::mcp::enc;
    let mut url = format!("https://hub/fold/menu?slug={}&now={now_ms}", enc(slug));
    if let Some(l) = locale {
        url.push_str(&format!("&locale={}", enc(l)));
    }
    if fresh {
        url.push_str("&fresh=1");
    }
    let mut res = place.stub()?.fetch_with_request(Request::new(&url, Method::Get)?).await?;
    match res.status_code() {
        200 => Ok(Ok(res.text().await?)),
        404 => Ok(Err(404)),
        other => Err(Error::RustError(format!("hub object refused the menu: {other} {}", res.text().await.unwrap_or_default()))),
    }
}

/// The venue record and the products named, as stored (R2): what the live
/// estimate reads, instead of the catalogue image. An id the catalogue does
/// not have is absent from the map.
pub async fn products(place: &Place, ids: &[String])
    -> Result<(Option<serde_json::Value>, std::collections::HashMap<String, serde_json::Value>)>
{
    let q: Vec<String> = ids.iter().map(|id| format!("ids={}", crate::mcp::enc(id))).collect();
    let url = format!("https://hub/fold/products?{}", q.join("&"));
    let mut res = place.stub()?.fetch_with_request(Request::new(&url, Method::Get)?).await?;
    if res.status_code() != 200 {
        return Err(Error::RustError(format!("hub object refused the products: {}", res.status_code())));
    }
    #[derive(serde::Deserialize)]
    struct Out {
        venue: Option<serde_json::Value>,
        products: std::collections::HashMap<String, serde_json::Value>,
    }
    let out: Out = res.json().await?;
    Ok((out.venue, out.products))
}
