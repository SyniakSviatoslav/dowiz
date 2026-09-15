//! The operator loop, driven through the real router in-process.
//!
//! WHY IN-PROCESS AND NOT A SPAWNED BINARY. The point is to exercise the routes
//! and their extractors, and a test that shells out to curl proves the shell
//! works. This builds the same `Router` `main` builds and drives it over a real
//! TCP socket, so the auth extractors, the JSON bodies and the status codes are
//! all the production ones.
//!
//! `HUB_PBKDF2_ITERATIONS` is lowered here. At the production 600k a single
//! login is around a second, and a suite with a dozen of them stops being a
//! suite anyone runs. The hub warns when the value is lowered, which is what
//! keeps this from being a quiet way to weaken a real deployment.

use std::sync::Arc;
use std::time::Duration;

use native_spa_server::{api::ApiState, build_router, hub, webhook::WebhookState};
use serde_json::{json, Value};

struct Server {
    base: String,
    dir: std::path::PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

async fn boot(tag: &str) -> Server {
    // SAFETY: set before any hub is opened in this test binary; tests in one
    // binary share a process, and every one of them wants this same value.
    unsafe { std::env::set_var("HUB_PBKDF2_ITERATIONS", "64") };

    let dir = std::env::temp_dir().join(format!("dowiz_hub_ops_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("hub dir");

    // A venue, or /api/menu has nothing to answer with.
    let bundle = dir.join("bundle.json");
    std::fs::write(
        &bundle,
        json!({
            "location": { "id": "venue_1", "name": "Dubin & Sushi", "slug": "dubin",
                          "status": "open", "currency_code": "ALL",
                          "delivery_fee": 200, "free_delivery_threshold": 2000 },
            "categories": [{ "id": "c1", "name": "Rolls", "sortOrder": 0 }],
            "products": [{ "id": "p1", "categoryId": "c1", "name": "Sake Futomaki",
                           "price": 900, "available": true, "sortOrder": 0 }]
        })
        .to_string(),
    )
    .expect("bundle");
    hub::seed_catalog(&dir, &bundle).expect("seed");

    let state = hub::HubState::open(&dir).expect("hub");
    state
        .add_person("ana@dubin.al", dowiz_hub::token::Role::Owner, "Ana", "owner-pw")
        .await
        .expect("owner");
    state
        .add_person("+355691112233", dowiz_hub::token::Role::Courier, "Eni", "courier-pw")
        .await
        .expect("courier");
    // A SECOND courier exists so "this order is not yours" can be tested
    // against a real other person rather than against an absence.
    state
        .add_person("+355694445566", dowiz_hub::token::Role::Courier, "Blerim", "courier-pw-2")
        .await
        .expect("second courier");

    let router = build_router(
        std::path::Path::new("public"),
        ApiState::build_default(),
        Arc::new(WebhookState {
            telegram: Arc::new(intake_adapters::telegram::TelegramAdapter::new("t".into())),
            intake: Arc::new(dowiz_kernel::ports::hub_intake::IntakeService::new(vec![])),
            hub: Some(state.clone()),
        }),
        Some(state),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    // Let the accept loop come up before the first request.
    tokio::time::sleep(Duration::from_millis(50)).await;
    Server { base: format!("http://{addr}"), dir }
}

/// A minimal HTTP client over the standard library, so the test adds no
/// dependency the zero-dep gate would have to admit.
fn request(base: &str, method: &str, path: &str, token: Option<&str>, body: Option<Value>) -> (u16, Value) {
    use std::io::{Read, Write};
    let addr = base.trim_start_matches("http://");
    let mut s = std::net::TcpStream::connect(addr).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(30))).ok();
    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let mut req = format!(
        "{method} {path} HTTP/1.1\r\nHost: h\r\nConnection: close\r\nContent-Length: {}\r\n",
        payload.len()
    );
    if !payload.is_empty() {
        req.push_str("Content-Type: application/json\r\n");
    }
    if let Some(t) = token {
        req.push_str(&format!("Authorization: Bearer {t}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(&payload);
    s.write_all(req.as_bytes()).expect("write");

    let mut raw = Vec::new();
    s.read_to_end(&mut raw).expect("read");
    let text = String::from_utf8_lossy(&raw);
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let code: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    // The response is `Connection: close`, so the body is everything that
    // follows -- no chunk framing to unpick.
    (code, serde_json::from_str(body.trim()).unwrap_or(Value::Null))
}

fn get(b: &str, p: &str, t: Option<&str>) -> (u16, Value) {
    request(b, "GET", p, t, None)
}
fn post(b: &str, p: &str, t: Option<&str>, body: Value) -> (u16, Value) {
    request(b, "POST", p, t, Some(body))
}

/// POST a raw body (not JSON) -- the menu import takes the file itself.
fn post_text(base: &str, path: &str, token: &str, text: &str) -> (u16, Value) {
    use std::io::{Read, Write};
    let addr = base.trim_start_matches("http://");
    let mut s = std::net::TcpStream::connect(addr).expect("connect");
    s.set_read_timeout(Some(Duration::from_secs(30))).ok();
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: h\r\nConnection: close\r\n\
         Authorization: Bearer {token}\r\nContent-Type: text/csv\r\n\
         Content-Length: {}\r\n\r\n{text}",
        text.len()
    );
    s.write_all(req.as_bytes()).expect("write");
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).expect("read");
    let t = String::from_utf8_lossy(&raw);
    let (head, body) = t.split_once("\r\n\r\n").unwrap_or((&t, ""));
    let code = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    (code, serde_json::from_str(body.trim()).unwrap_or(Value::Null))
}

fn login(base: &str, id: &str, pw: &str) -> (u16, Value) {
    post(base, "/api/auth/login", None, json!({ "email": id, "password": pw }))
}

/// Place an order and drive it to READY, which is the state a courier can act on.
fn order_ready_for_a_courier(base: &str, owner: &str) -> String {
    let (code, order) = post(
        base,
        "/api/public/locations/dubin/orders",
        None,
        json!({
            "items": [{ "product_id": "p1", "modifier_ids": [], "quantity": 2 }],
            "contact": { "name": "Customer", "phone": "+355690000000" },
            "fulfilment": { "kind": "delivery", "address": { "line": "Rruga Taulantia 12" } },
            "payment": "cash"
        }),
    );
    assert_eq!(code, 200, "place: {order}");
    let id = order["id"].as_str().expect("order id").to_string();
    for action in ["confirm", "preparing", "ready"] {
        let (c, v) = post(
            base,
            &format!("/api/owner/orders/{id}/action"),
            Some(owner),
            json!({ "action": action, "location_id": "venue_1" }),
        );
        assert_eq!(c, 200, "{action}: {v}");
    }
    id
}

#[tokio::test(flavor = "multi_thread")]
async fn an_owner_route_refuses_everyone_it_should() {
    let s = boot("gate").await;

    // No credential at all.
    assert_eq!(get(&s.base, "/api/owner/orders", None).0, 401);
    // Junk, and a token from another hub's key.
    assert_eq!(get(&s.base, "/api/owner/orders", Some("nonsense")).0, 401);
    assert_eq!(get(&s.base, "/api/owner/orders", Some("Zm9v.YmFy")).0, 401);

    // Wrong password.
    assert_eq!(login(&s.base, "ana@dubin.al", "wrong").0, 401);
    // A person who does not exist.
    assert_eq!(login(&s.base, "nobody@example.com", "owner-pw").0, 401);

    // A COURIER's credentials must not open the owner pane, even though they
    // are perfectly valid credentials.
    assert_eq!(login(&s.base, "+355691112233", "courier-pw").0, 401);

    let (code, body) = login(&s.base, "ana@dubin.al", "owner-pw");
    assert_eq!(code, 200, "{body}");
    let access = body["access_token"].as_str().expect("access").to_string();
    let refresh = body["refresh_token"].as_str().expect("refresh").to_string();
    assert_eq!(body["user"]["role"], "owner");
    assert_eq!(body["user"]["locationId"], "venue_1");

    // A REFRESH token is not an access token. Accepting one here would undo the
    // short access lifetime entirely.
    assert_eq!(get(&s.base, "/api/owner/orders", Some(&refresh)).0, 401);
    assert_eq!(get(&s.base, "/api/owner/orders", Some(&access)).0, 200);

    // An ACCESS token is not a refresh token either.
    assert_eq!(
        post(&s.base, "/api/auth/refresh", None, json!({ "refresh_token": access })).0,
        401
    );
    let (c, r) = post(&s.base, "/api/auth/refresh", None, json!({ "refresh_token": refresh })).into();
    assert_eq!(c, 200, "{r}");
    assert!(r["access_token"].as_str().is_some_and(|t| !t.is_empty()));
}

/// Logging out must take effect NOW, not at token expiry.
#[tokio::test(flavor = "multi_thread")]
async fn logout_kills_the_token_immediately() {
    let s = boot("logout").await;
    let (_, body) = login(&s.base, "ana@dubin.al", "owner-pw");
    let access = body["access_token"].as_str().unwrap().to_string();
    let refresh = body["refresh_token"].as_str().unwrap().to_string();

    assert_eq!(get(&s.base, "/api/owner/orders", Some(&access)).0, 200);
    assert_eq!(post(&s.base, "/api/auth/logout", Some(&access), json!({})).0, 200);
    // The token is still cryptographically valid and still in date. It is the
    // SESSION that is gone, which is the only thing that makes logout mean
    // anything.
    assert_eq!(get(&s.base, "/api/owner/orders", Some(&access)).0, 401);
    // And the refresh token cannot resurrect it.
    assert_eq!(
        post(&s.base, "/api/auth/refresh", None, json!({ "refresh_token": refresh })).0,
        401
    );
}

/// The whole loop: a customer orders, the kitchen works it, a courier delivers.
#[tokio::test(flavor = "multi_thread")]
async fn an_order_travels_from_the_customer_to_the_door() {
    let s = boot("loop").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();
    let (_, c) = post(
        &s.base,
        "/api/courier/auth/login",
        None,
        json!({ "phone": "+355691112233", "password": "courier-pw" }),
    );
    let courier = c["jwt"].as_str().expect("jwt").to_string();

    let id = order_ready_for_a_courier(&s.base, &owner);

    // The order is the venue's money: 2 x 900 = 1800, under the 2000 free
    // threshold, so the 200 fee applies. Computed by the server, never sent.
    let (_, orders) = get(&s.base, "/api/owner/orders", Some(&owner));
    let found = orders["orders"]
        .as_array()
        .expect("orders")
        .iter()
        .find(|o| o["id"] == id.as_str())
        .expect("the order must be in the owner's queue");
    assert_eq!(found["subtotal"], 1800);
    assert_eq!(found["total"], 2000);
    assert_eq!(found["status"], "READY");

    // It is OFFERED to the courier -- unassigned and ready.
    let (_, tasks) = get(&s.base, "/api/courier/tasks", Some(&courier));
    assert!(
        tasks["available"].as_array().unwrap().iter().any(|o| o["id"] == id.as_str()),
        "a READY unassigned delivery must be offered: {tasks}"
    );
    assert!(tasks["tasks"].as_array().unwrap().is_empty(), "nothing is theirs yet");

    // They take it.
    let (code, v) = post(&s.base, &format!("/api/courier/orders/{id}/accept"), Some(&courier), json!({}));
    assert_eq!(code, 200, "{v}");
    let (_, tasks) = get(&s.base, "/api/courier/tasks", Some(&courier));
    assert_eq!(tasks["tasks"].as_array().unwrap().len(), 1, "now it is theirs");
    assert!(tasks["available"].as_array().unwrap().is_empty(), "and no longer offered");

    // Pickup, then delivery.
    let (code, v) = post(&s.base, &format!("/api/courier/orders/{id}/pickup"), Some(&courier), json!({}));
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["status"], "IN_DELIVERY");
    let (code, v) = post(
        &s.base,
        &format!("/api/courier/orders/{id}/deliver"),
        Some(&courier),
        json!({ "cash_collected": 2000 }),
    );
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["status"], "DELIVERED");
    assert_eq!(v["cash_collected"], 2000);
    // The address survived every hand-off. Losing it only shows up at the door.
    assert_eq!(v["fulfilment"]["address"]["line"], "Rruga Taulantia 12");

    // And it leaves the courier's list.
    let (_, tasks) = get(&s.base, "/api/courier/tasks", Some(&courier));
    assert!(tasks["tasks"].as_array().unwrap().is_empty(), "delivered work is not open work");
}

/// A courier must not be able to touch an order that is not theirs.
#[tokio::test(flavor = "multi_thread")]
async fn a_courier_cannot_move_another_couriers_order() {
    let s = boot("claim").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();
    let tok = |phone: &str, pw: &str| {
        let (_, c) = post(
            &s.base,
            "/api/courier/auth/login",
            None,
            json!({ "phone": phone, "password": pw }),
        );
        c["jwt"].as_str().expect("jwt").to_string()
    };
    let eni = tok("+355691112233", "courier-pw");
    let blerim = tok("+355694445566", "courier-pw-2");

    let id = order_ready_for_a_courier(&s.base, &owner);

    // An unknown courier is not assignable: an order handed to nobody would
    // vanish from every app while the customer waits for it.
    let (code, v) = post(
        &s.base,
        &format!("/api/owner/orders/{id}/assign"),
        Some(&owner),
        json!({ "courier_id": "+355699999999" }),
    );
    assert_eq!(code, 400, "an unknown courier must not be assignable: {v}");

    // Eni takes it.
    let (code, _) = post(&s.base, &format!("/api/courier/orders/{id}/accept"), Some(&eni), json!({}));
    assert_eq!(code, 200);

    // NOW the real negative: Blerim is a valid, active, logged-in courier, and
    // must still be refused on every verb for this order.
    let (code, v) = post(&s.base, &format!("/api/courier/orders/{id}/accept"), Some(&blerim), json!({}));
    assert_eq!(code, 409, "a second courier must not be able to steal it: {v}");
    let (code, v) = post(&s.base, &format!("/api/courier/orders/{id}/pickup"), Some(&blerim), json!({}));
    assert_eq!(code, 409, "nor pick it up: {v}");
    let (code, v) = post(&s.base, &format!("/api/courier/orders/{id}/deliver"), Some(&blerim), json!({}));
    assert_eq!(code, 409, "nor deliver it: {v}");

    // It never appeared in Blerim's list in the first place.
    let (_, tasks) = get(&s.base, "/api/courier/tasks", Some(&blerim));
    assert!(tasks["tasks"].as_array().unwrap().is_empty(), "not their work: {tasks}");
    assert!(tasks["available"].as_array().unwrap().is_empty(), "and not on offer: {tasks}");

    // The order is untouched by any of that.
    let (_, v) = get(&s.base, &format!("/api/order/{id}"), None);
    assert_eq!(v["status"], "READY");
    assert_eq!(v["courier_id"], "+355691112233");

    // And the kernel still refuses a delivery with no pickup, even from the
    // courier who does hold the order.
    let (code, v) = post(&s.base, &format!("/api/courier/orders/{id}/deliver"), Some(&eni), json!({}));
    assert_eq!(code, 409, "a delivery without a pickup must be refused: {v}");
}

/// The dish NAME must survive every hand-off.
///
/// The kernel has no menu, so it returns lines carrying a product id and a
/// price and nothing else. Without the hub putting the name back at every
/// transition, the kitchen's ticket and the owner's queue read "2x item-01",
/// which is not something anyone can cook -- and it degrades silently, only at
/// the FIRST status change, which is exactly when nobody is looking at a test.
#[tokio::test(flavor = "multi_thread")]
async fn a_dish_keeps_its_name_all_the_way_to_the_door() {
    let s = boot("names").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();
    let (_, c) = post(
        &s.base,
        "/api/courier/auth/login",
        None,
        json!({ "phone": "+355691112233", "password": "courier-pw" }),
    );
    let courier = c["jwt"].as_str().unwrap().to_string();

    let (_, order) = post(
        &s.base,
        "/api/public/locations/dubin/orders",
        None,
        json!({
            "items": [{ "product_id": "p1", "modifier_ids": [], "quantity": 2 }],
            "contact": { "name": "C", "phone": "+355690000000" },
            "fulfilment": { "kind": "delivery", "address": { "line": "Rruga Taulantia 12" } }
        }),
    );
    let id = order["id"].as_str().unwrap().to_string();
    assert_eq!(order["items"][0]["name"], "Sake Futomaki", "at placement: {order}");

    let named = |v: &Value| v["items"][0]["name"].as_str().unwrap_or("").to_string();

    for action in ["confirm", "preparing", "ready"] {
        let (code, v) = post(
            &s.base,
            &format!("/api/owner/orders/{id}/action"),
            Some(&owner),
            json!({ "action": action }),
        );
        assert_eq!(code, 200, "{action}");
        assert_eq!(named(&v), "Sake Futomaki", "lost the name at {action}: {v}");
    }

    post(&s.base, &format!("/api/courier/orders/{id}/accept"), Some(&courier), json!({}));
    let (_, v) = post(&s.base, &format!("/api/courier/orders/{id}/pickup"), Some(&courier), json!({}));
    assert_eq!(named(&v), "Sake Futomaki", "lost the name at pickup: {v}");
    let (_, v) = post(&s.base, &format!("/api/courier/orders/{id}/deliver"), Some(&courier), json!({}));
    assert_eq!(named(&v), "Sake Futomaki", "lost the name at delivery: {v}");

    // And it is still there when the order is read back from the log.
    let (_, v) = get(&s.base, &format!("/api/order/{id}"), None);
    assert_eq!(named(&v), "Sake Futomaki");

    // Renaming the dish afterwards must NOT rewrite what this order says was
    // bought. The name was recorded as sold.
    let (code, _) = post(
        &s.base,
        "/api/owner/products/p1",
        Some(&owner),
        json!({ "available": true }),
    );
    assert_eq!(code, 200);
    let (_, v) = get(&s.base, &format!("/api/order/{id}"), None);
    assert_eq!(named(&v), "Sake Futomaki", "history must not be rewritten by the menu");
}

/// The owner's actions are the kernel's, including the ones it refuses.
#[tokio::test(flavor = "multi_thread")]
async fn the_kernel_still_decides_what_the_owner_may_do() {
    let s = boot("actions").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    let (_, order) = post(
        &s.base,
        "/api/public/locations/dubin/orders",
        None,
        json!({
            "items": [{ "product_id": "p1", "modifier_ids": [], "quantity": 1 }],
            "contact": { "name": "C", "phone": "+355690000000" },
            "fulfilment": { "kind": "pickup" }
        }),
    );
    let id = order["id"].as_str().unwrap().to_string();

    // An action that does not exist.
    let (code, _) = post(
        &s.base,
        &format!("/api/owner/orders/{id}/action"),
        Some(&owner),
        json!({ "action": "teleport" }),
    );
    assert_eq!(code, 400);

    // A rejection MUST carry a reason -- the customer is shown it.
    let (code, _) = post(
        &s.base,
        &format!("/api/owner/orders/{id}/action"),
        Some(&owner),
        json!({ "action": "reject" }),
    );
    assert_eq!(code, 400);

    // Skipping ahead is refused by the kernel, not by this server.
    let (code, v) = post(
        &s.base,
        &format!("/api/owner/orders/{id}/action"),
        Some(&owner),
        json!({ "action": "ready" }),
    );
    assert_eq!(code, 409, "PENDING -> READY must be refused: {v}");

    // And the refusal did not damage the order.
    let (code, v) = get(&s.base, &format!("/api/order/{id}"), None);
    assert_eq!(code, 200);
    assert_eq!(v["status"], "PENDING");

    // A legal rejection, with a reason.
    let (code, v) = post(
        &s.base,
        &format!("/api/owner/orders/{id}/action"),
        Some(&owner),
        json!({ "action": "reject", "reason": "no salmon today" }),
    );
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["status"], "REJECTED");
    assert_eq!(v["rejection_reason"], "no salmon today");
    assert_eq!(v["last_actor"], "ana@dubin.al", "an action must carry a name");
}

/// The dashboard's numbers must come from the log.
#[tokio::test(flavor = "multi_thread")]
async fn the_dashboard_counts_what_happened() {
    let s = boot("dash").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    let (_, d) = get(&s.base, "/api/owner/dashboard", Some(&owner));
    assert_eq!(d["todayOrders"], 0);
    assert_eq!(d["todayRevenue"], 0);

    let id = order_ready_for_a_courier(&s.base, &owner);
    let (_, d) = get(&s.base, "/api/owner/dashboard", Some(&owner));
    assert_eq!(d["todayOrders"], 1);
    assert_eq!(d["pending"], 0, "it was confirmed");
    assert_eq!(d["active"], 1, "and it is still live");
    assert_eq!(d["todayRevenue"], 2000);

    // A second order, rejected, must count as an order but NOT as revenue --
    // money the venue turned down is not money it took.
    let (_, order2) = post(
        &s.base,
        "/api/public/locations/dubin/orders",
        None,
        json!({
            "items": [{ "product_id": "p1", "modifier_ids": [], "quantity": 1 }],
            "contact": { "name": "C", "phone": "+355690000000" },
            "fulfilment": { "kind": "pickup" }
        }),
    );
    let id2 = order2["id"].as_str().unwrap().to_string();
    post(
        &s.base,
        &format!("/api/owner/orders/{id2}/action"),
        Some(&owner),
        json!({ "action": "reject", "reason": "closing" }),
    );
    let (_, d) = get(&s.base, "/api/owner/dashboard", Some(&owner));
    assert_eq!(d["todayOrders"], 2);
    assert_eq!(d["todayRevenue"], 2000, "the rejected order must not be counted");
    assert_ne!(id, id2);
}

/// Stop-listing a dish must reach the storefront.
#[tokio::test(flavor = "multi_thread")]
async fn the_owner_can_stop_list_a_dish_and_the_menu_shows_it() {
    let s = boot("menu").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    let (code, v) = post(
        &s.base,
        "/api/owner/products/p1",
        Some(&owner),
        json!({ "available": false, "unavailable_note": "no salmon today" }),
    );
    assert_eq!(code, 200, "{v}");

    let (_, menu) = get(&s.base, "/api/menu", None);
    let p = &menu["categories"][0]["products"][0];
    assert_eq!(p["available"], false);
    assert_eq!(p["unavailableNote"], "no salmon today");

    // Putting it back must clear the note, or the customer reads a stale excuse
    // on a dish that is available.
    post(&s.base, "/api/owner/products/p1", Some(&owner), json!({ "available": true }));
    let (_, menu) = get(&s.base, "/api/menu", None);
    assert_eq!(menu["categories"][0]["products"][0]["unavailableNote"], Value::Null);

    // A negative price is a typo, not a discount.
    let (code, _) = post(&s.base, "/api/owner/products/p1", Some(&owner), json!({ "price": -5 }));
    assert_eq!(code, 400);

    // Closing the venue must show on the storefront.
    let (code, _) = post(&s.base, "/api/owner/location", Some(&owner), json!({ "status": "closed" }));
    assert_eq!(code, 200);
    let (_, menu) = get(&s.base, "/api/menu", None);
    assert_eq!(menu["location"]["status"], "closed");
}

/// A spreadsheet becomes a menu, and does not become one by accident.
#[tokio::test(flavor = "multi_thread")]
async fn a_spreadsheet_becomes_a_menu() {
    let s = boot("import").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    let csv = "Category,Name,Description,Price,Available\n\
               Rolls,Sake Futomaki,salmon,900,yes\n\
               Rolls,Ebi Maki,prawn,750,yes\n\
               Rolls,Broken,prawn,9.50,yes\n\
               Drinks,Water,,100,yes\n";

    // A DRY RUN by default: nothing may change until the owner says so.
    let (code, v) = post_text(&s.base, "/api/owner/menu/import", &owner, csv);
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["applied"], false);
    assert_eq!(v["products"], 3, "the fractional-price row is refused: {v}");
    assert_eq!(v["warnings"].as_array().unwrap().len(), 1);
    assert!(
        v["warnings"][0].as_str().unwrap().contains("row 4 (Broken)"),
        "the warning must name the row: {v}"
    );
    // The seeded dish is not in the file, so it is reported as such.
    assert!(
        v["notInFile"].as_array().unwrap().iter().any(|p| p["id"] == "p1"),
        "{v}"
    );
    // And the live menu is untouched.
    let (_, menu) = get(&s.base, "/api/menu", None);
    assert_eq!(menu["categories"][0]["products"][0]["id"], "p1");

    // Now apply.
    let (code, v) = post_text(&s.base, "/api/owner/menu/import?apply=true", &owner, csv);
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["applied"], true);

    let (_, menu) = get(&s.base, "/api/menu", None);
    let all: Vec<&Value> = menu["categories"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|c| c["products"].as_array().unwrap())
        .collect();
    let find = |id: &str| all.iter().find(|p| p["id"] == id).copied();
    assert!(find("rolls-sake-futomaki").is_some(), "{menu}");
    assert_eq!(find("rolls-sake-futomaki").unwrap()["price"], 900);
    assert_eq!(find("drinks-water").unwrap()["price"], 100);
    assert!(find("rolls-broken").is_none(), "a refused row must not appear");
    // The seeded dish survives: an import ADDS, and retiring is opt-in.
    assert!(find("p1").is_some(), "an import must not silently remove a dish");

    // Importing the SAME file again updates rather than duplicating.
    let before = all.len();
    let (_, _) = post_text(&s.base, "/api/owner/menu/import?apply=true", &owner, csv);
    let (_, menu2) = get(&s.base, "/api/menu", None);
    let after: usize = menu2["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["products"].as_array().unwrap().len())
        .sum();
    assert_eq!(after, before, "a second import must not duplicate the menu");

    // Retiring takes the absent dish off the storefront WITHOUT deleting it.
    let (code, v) =
        post_text(&s.base, "/api/owner/menu/import?apply=true&retire_missing=true", &owner, csv);
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["retired"], 1);
    let (_, menu3) = get(&s.base, "/api/menu", None);
    let still: Vec<&Value> = menu3["categories"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|c| c["products"].as_array().unwrap())
        .collect();
    let p1 = still.iter().find(|p| p["id"] == "p1").expect("still present, not deleted");
    assert_eq!(p1["available"], false);
}

/// A file that parses to nothing must NOT be allowed to wipe a working menu.
#[tokio::test(flavor = "multi_thread")]
async fn an_unreadable_file_cannot_erase_the_menu() {
    let s = boot("import_guard").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    // No price column: the parser understands the file, and it means nothing.
    let (code, v) = post_text(
        &s.base,
        "/api/owner/menu/import?apply=true&retire_missing=true",
        &owner,
        "Category,Name\nRolls,Sake\n",
    );
    assert_eq!(code, 400, "{v}");

    // An empty body.
    let (code, _) = post_text(&s.base, "/api/owner/menu/import?apply=true", &owner, "");
    assert_eq!(code, 400);

    // The menu is exactly as it was.
    let (_, menu) = get(&s.base, "/api/menu", None);
    assert_eq!(menu["categories"][0]["products"][0]["id"], "p1");
    assert_eq!(menu["categories"][0]["products"][0]["available"], true);
}

/// Importing is the owner's act, not the public's.
#[tokio::test(flavor = "multi_thread")]
async fn only_an_owner_may_import_a_menu() {
    let s = boot("import_auth").await;
    let (_, c) = post(
        &s.base,
        "/api/courier/auth/login",
        None,
        json!({ "phone": "+355691112233", "password": "courier-pw" }),
    );
    let courier = c["jwt"].as_str().unwrap().to_string();
    let csv = "Category,Name,Price\nRolls,Sake,900\n";

    assert_eq!(post_text(&s.base, "/api/owner/menu/import?apply=true", "", csv).0, 401);
    assert_eq!(
        post_text(&s.base, "/api/owner/menu/import?apply=true", &courier, csv).0,
        403,
        "a courier is authenticated but not permitted"
    );
    let (_, menu) = get(&s.base, "/api/menu", None);
    assert_eq!(menu["categories"][0]["products"][0]["id"], "p1");
}

/// An image becomes a palette, and a palette becomes a checked theme.
#[tokio::test(flavor = "multi_thread")]
async fn a_photo_becomes_a_venue_theme() {
    let s = boot("brand").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    // A "photograph": mostly paper, some ink, a little logo red. The same
    // proportions a menu photo actually has.
    let mut hex = String::new();
    for _ in 0..900 {
        hex.push_str("fcfbf8");
    }
    for _ in 0..80 {
        hex.push_str("121214");
    }
    for _ in 0..20 {
        hex.push_str("e11d48");
    }

    let (code, v) = post(&s.base, "/api/owner/branding/extract", Some(&owner), json!({ "pixels": hex }));
    assert_eq!(code, 200, "{v}");
    let sw = v["swatches"].as_array().expect("swatches");
    assert!(!sw.is_empty(), "the logo colour must be found under the paper: {v}");
    // Paper and ink must not win.
    let top = sw[0]["hex"].as_str().unwrap();
    assert!(top.starts_with("#e") || top.starts_with("#d"), "expected the red, got {top}");
    // Every suggestion arrives with its contrast already measured.
    for pair in sw[0]["theme"]["contrast"].as_array().unwrap() {
        assert_eq!(pair["passes"], true, "a suggested theme must pass: {pair}");
    }

    // Nothing was applied by extracting.
    let (_, menu) = get(&s.base, "/api/menu", None);
    assert_eq!(menu["location"]["theme"], Value::Null, "extraction must not repaint anything");

    // The owner adopts one.
    let (code, v) = post(&s.base, "/api/owner/branding", Some(&owner), json!({ "primary": "#e11d48" }));
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["primary"], "#e11d48");
    assert_eq!(v["primaryAdjustedPct"], 0, "a usable colour must be used as given");

    let (_, menu) = get(&s.base, "/api/menu", None);
    let theme = &menu["location"]["theme"];
    assert_eq!(theme["seed"], "#e11d48");
    let light = theme["light"].as_str().expect("light tokens");
    assert!(light.contains("--brand-primary:#e11d48"), "{light}");
    assert!(light.contains("--brand-text:"), "{light}");
    assert!(theme["dark"].as_str().unwrap().contains("--brand-bg:"), "dark mode is not optional");
}

/// A pastel logo must not produce an unreadable storefront.
#[tokio::test(flavor = "multi_thread")]
async fn a_pastel_brand_is_made_legible_and_says_so() {
    let s = boot("pastel").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    let (code, v) = post(&s.base, "/api/owner/branding", Some(&owner), json!({ "primary": "#ffd9e3" }));
    assert_eq!(code, 200, "{v}");
    // It was adjusted, and the owner is told by how much rather than being
    // handed a different colour silently.
    assert!(v["primaryAdjustedPct"].as_u64().unwrap() > 0, "{v}");
    assert_ne!(v["primary"], "#ffd9e3");
    for pair in v["contrast"].as_array().unwrap() {
        assert_eq!(pair["passes"], true, "{pair}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn branding_refuses_junk_and_strangers() {
    let s = boot("brandguard").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    assert_eq!(post(&s.base, "/api/owner/branding", Some(&owner), json!({ "primary": "red" })).0, 400);
    assert_eq!(post(&s.base, "/api/owner/branding", Some(&owner), json!({ "primary": "" })).0, 400);
    assert_eq!(post(&s.base, "/api/owner/branding", None, json!({ "primary": "#e11d48" })).0, 401);

    // Pixels that are not whole triples, and a payload nobody meant to send.
    assert_eq!(
        post(&s.base, "/api/owner/branding/extract", Some(&owner), json!({ "pixels": "abcd" })).0,
        400
    );
    assert_eq!(
        post(&s.base, "/api/owner/branding/extract", Some(&owner), json!({ "pixels": "zzzzzz" })).0,
        400
    );

    // A black-and-white image yields NOTHING rather than an invented colour.
    let bw: String = std::iter::repeat_n("ffffff", 50).chain(std::iter::repeat_n("000000", 50)).collect();
    let (code, v) = post(&s.base, "/api/owner/branding/extract", Some(&owner), json!({ "pixels": bw }));
    assert_eq!(code, 200, "{v}");
    assert!(v["swatches"].as_array().unwrap().is_empty(), "{v}");
    assert!(v["note"].as_str().unwrap().contains("no strong colours"), "{v}");
}

/// Settings are the owner's, secrets never come back, and the assistant is off
/// until it is switched on.
#[tokio::test(flavor = "multi_thread")]
async fn settings_guard_their_secrets_and_the_assistant_starts_off() {
    let s = boot("settings").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    // The pane is built from the hub's own declarations.
    let (code, v) = get(&s.base, "/api/owner/settings", Some(&owner));
    assert_eq!(code, 200, "{v}");
    let known = v["known"].as_array().expect("known");
    assert!(known.iter().any(|k| k["key"] == "ai.endpoint"), "{v}");
    assert!(
        known.iter().find(|k| k["key"] == "ai.token").unwrap()["secret"] == true,
        "a token must be declared secret"
    );

    // Off by default: asking now is refused rather than silently doing nothing.
    let (code, v) = post(&s.base, "/api/owner/assist", Some(&owner), json!({ "question": "how many orders?" }));
    assert_eq!(code, 409, "{v}");
    assert!(v["error"].as_str().unwrap().contains("switched off"), "{v}");

    // An endpoint is validated when it is SET, not at the first question.
    let bad = post(&s.base, "/api/owner/settings", Some(&owner),
                   json!({ "key": "ai.endpoint", "value": "http://api.example.com/v1" }));
    assert_eq!(bad.0, 400, "plain http to a remote host must be refused: {}", bad.1);
    let bad = post(&s.base, "/api/owner/settings", Some(&owner),
                   json!({ "key": "ai.endpoint", "value": "nonsense" }));
    assert_eq!(bad.0, 400);
    // An undeclared key has nowhere to go.
    assert_eq!(
        post(&s.base, "/api/owner/settings", Some(&owner), json!({ "key": "x.y", "value": "1" })).0,
        400
    );

    // A token goes in and never comes back.
    let (code, _) = post(&s.base, "/api/owner/settings", Some(&owner),
                         json!({ "key": "ai.token", "value": "sk-do-not-leak-me" }));
    assert_eq!(code, 200);
    let (_, v) = get(&s.base, "/api/owner/settings", Some(&owner));
    let shown = v.to_string();
    assert!(!shown.contains("sk-do-not-leak-me"), "the token leaked to the owner pane: {shown}");
    assert!(v["values"]["ai.token"].as_str().unwrap().contains("set"), "{v}");

    // Clearing it works even though it can no longer be read.
    post(&s.base, "/api/owner/settings", Some(&owner), json!({ "key": "ai.token", "value": "" }));
    let (_, v) = get(&s.base, "/api/owner/settings", Some(&owner));
    assert!(v["values"].get("ai.token").is_none(), "{v}");
}

/// Settings and the assistant belong to the owner alone.
#[tokio::test(flavor = "multi_thread")]
async fn a_courier_cannot_read_or_change_settings() {
    let s = boot("settings_auth").await;
    let (_, c) = post(&s.base, "/api/courier/auth/login", None,
                      json!({ "phone": "+355691112233", "password": "courier-pw" }));
    let courier = c["jwt"].as_str().unwrap().to_string();

    assert_eq!(get(&s.base, "/api/owner/settings", Some(&courier)).0, 403);
    assert_eq!(
        post(&s.base, "/api/owner/settings", Some(&courier),
             json!({ "key": "ai.enabled", "value": "1" })).0,
        403
    );
    assert_eq!(get(&s.base, "/api/owner/settings", None).0, 401);

    // The courier's OWN assistant is reachable, and is off like everyone's.
    let (code, v) = post(&s.base, "/api/courier/assist", Some(&courier), json!({ "question": "what is left?" }));
    assert_eq!(code, 409, "{v}");
}

/// With the assistant on and pointed at a model that is not there, the failure
/// must be LOUD and must name what went wrong -- not a blank answer bubble.
#[tokio::test(flavor = "multi_thread")]
async fn an_unreachable_model_fails_loudly() {
    let s = boot("ai_down").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    // Port 1 on loopback: a local address, so the TLS rule permits it, and
    // nothing is listening there.
    post(&s.base, "/api/owner/settings", Some(&owner),
         json!({ "key": "ai.endpoint", "value": "http://127.0.0.1:1/v1" }));
    post(&s.base, "/api/owner/settings", Some(&owner), json!({ "key": "ai.enabled", "value": "1" }));

    let (code, v) = post(&s.base, "/api/owner/assist", Some(&owner), json!({ "question": "how many?" }));
    assert_eq!(code, 503, "{v}");
    let err = v["error"].as_str().unwrap();
    assert!(err.contains("could not reach the model"), "must say what failed: {err}");

    // An empty question never reaches a model at all.
    assert_eq!(post(&s.base, "/api/owner/assist", Some(&owner), json!({ "question": "  " })).0, 400);
}

/// A venue that draws a service area must not take orders outside it.
#[tokio::test(flavor = "multi_thread")]
async fn an_order_outside_the_delivery_area_is_refused() {
    let s = boot("zones").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    const LAT: i64 = 41_323_000; // Durrës
    const LON: i64 = 19_441_000;

    let order_at = |lat: Option<i64>, lon: Option<i64>| {
        let mut addr = json!({ "line": "Rruga Taulantia 12" });
        if let (Some(a), Some(b)) = (lat, lon) {
            addr["lat_udeg"] = json!(a);
            addr["lon_udeg"] = json!(b);
        }
        post(
            &s.base,
            "/api/public/locations/dubin/orders",
            None,
            json!({
                "items": [{ "product_id": "p1", "modifier_ids": [], "quantity": 1 }],
                "contact": { "name": "C", "phone": "+355690000000" },
                "fulfilment": { "kind": "delivery", "address": addr }
            }),
        )
    };

    // Before any zone exists, everywhere is served -- a venue that has not drawn
    // an area has not asked for one to be enforced.
    assert_eq!(order_at(Some(LAT + 900_000), Some(LON)).0, 200);

    // Draw a 3 km circle.
    let (code, v) = post(
        &s.base,
        "/api/owner/zones",
        Some(&owner),
        json!({ "zones": [{ "kind": "circle", "lat": LAT, "lon": LON, "radius_m": 3000 }] }),
    );
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["zones"], 1);

    // Inside: accepted.
    let (code, v) = order_at(Some(LAT + 9_000), Some(LON));
    assert_eq!(code, 200, "1 km away must be served: {v}");
    assert!(v.get("delivery_area_unverified").is_none(), "it WAS verified: {v}");

    // Outside: refused, with how far.
    let (code, v) = order_at(Some(LAT + 900_000), Some(LON));
    assert_eq!(code, 409, "100 km away must be refused: {v}");
    let msg = v["error"].as_str().unwrap();
    assert!(msg.contains("outside the delivery area"), "{msg}");
    assert!(msg.contains(" m"), "the customer must be told how far: {msg}");

    // No coordinates: ACCEPTED and flagged, because there is no geocoder and
    // refusing would refuse everyone who declined the location prompt.
    let (code, v) = order_at(None, None);
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["delivery_area_unverified"], true, "the venue must see it was unchecked: {v}");

    // A PICKUP order is never zone-checked: where the customer lives is not the
    // venue's problem.
    let (code, v) = post(
        &s.base,
        "/api/public/locations/dubin/orders",
        None,
        json!({
            "items": [{ "product_id": "p1", "modifier_ids": [], "quantity": 1 }],
            "contact": { "name": "C", "phone": "+355690000000" },
            "fulfilment": { "kind": "pickup" }
        }),
    );
    assert_eq!(code, 200, "{v}");

    // The storefront learns that a zone exists, so it knows to ask for location.
    let (_, menu) = get(&s.base, "/api/menu", None);
    assert_eq!(menu["location"]["hasDeliveryZones"], true);

    // And a customer can ask BEFORE filling a basket.
    let (code, v) = get(&s.base, &format!("/api/public/reach?lat_udeg={}&lon_udeg={LON}", LAT + 9_000), None);
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["deliverable"], true);
    let (_, v) = get(&s.base, &format!("/api/public/reach?lat_udeg={}&lon_udeg={LON}", LAT + 900_000), None);
    assert_eq!(v["deliverable"], false);
    assert!(v["nearestMetres"].as_i64().unwrap() > 90_000, "{v}");

    // Removing every zone turns the check off again.
    let (code, v) = post(&s.base, "/api/owner/zones", Some(&owner), json!({ "zones": [] }));
    assert_eq!(code, 200, "{v}");
    assert_eq!(order_at(Some(LAT + 900_000), Some(LON)).0, 200);
}

/// A zone the reader cannot understand would silently mean "no restriction".
/// It must be refused at the point it is set, not discovered later.
#[tokio::test(flavor = "multi_thread")]
async fn an_unreadable_zone_is_refused_rather_than_stored() {
    let s = boot("zones_guard").await;
    let (_, o) = login(&s.base, "ana@dubin.al", "owner-pw");
    let owner = o["access_token"].as_str().unwrap().to_string();

    for bad in [
        json!({ "kind": "circle", "lat": 1 }),
        json!({ "kind": "circle", "lat": 1, "lon": 2, "radius_m": 0 }),
        json!({ "kind": "polygon", "points": [[0, 0]] }),
        json!({ "kind": "nonsense" }),
    ] {
        let (code, v) = post(&s.base, "/api/owner/zones", Some(&owner), json!({ "zones": [bad] }));
        assert_eq!(code, 400, "must be refused: {v}");
    }
    // And a courier may not draw the venue's service area.
    let (_, c) = post(&s.base, "/api/courier/auth/login", None,
                      json!({ "phone": "+355691112233", "password": "courier-pw" }));
    let courier = c["jwt"].as_str().unwrap().to_string();
    assert_eq!(
        post(&s.base, "/api/owner/zones", Some(&courier),
             json!({ "zones": [{ "kind": "circle", "lat": 1, "lon": 2, "radius_m": 5 }] })).0,
        403
    );
}
