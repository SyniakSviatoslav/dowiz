//! THE FIRING'S REQUEST BUDGET (`TURN_BUDGET`, W-EBILLS-POISON): the client
//! counts what it sends -- the login's two included -- and refuses the
//! request past the budget UNSENT, by name. The twin: under the budget every
//! read goes out and nothing is refused.

use super::{Client, TURN_BUDGET};
use crate::ebills::client::Path;
use crate::ebills::judge::Fail;
use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::wire::{Call, Reply};

/// A till that answers everything: the front page's cookie, a login that
/// sets a session, and `{}` for every read.
fn serve() {
    answer_outbound(|c: &Call| {
        let url = c.url()?;
        let mut r = Reply::ok(if url.path() == "/" { "<html></html>" } else { "{}" })?;
        r.headers_mut().set("content-type", if url.path() == "/" { "text/html" } else { "application/json" })?;
        match url.path() {
            "/" => r.headers_mut().append("set-cookie", "XSRF-TOKEN=x1; Path=/")?,
            "/api/authentication" => {
                r.headers_mut().append("set-cookie", "JSESSIONID=s1; Path=/; HttpOnly")?;
                r.headers_mut().set("x-tenant-identifier", "t-1")?;
            }
            _ => {}
        }
        Ok(r)
    });
}

#[test]
fn every_read_under_the_budget_is_sent_and_the_one_past_it_is_refused_unsent() {
    serve();
    let mut c = Client::new("till@x.test", "pw", None);
    // The first read logs in: two requests, then the read itself.
    block_on(c.read(&Path::Items { size: 10 })).expect("the first read");
    assert_eq!((c.sent, sent().len()), (3, 3), "the login's two count");
    for _ in 3..TURN_BUDGET {
        block_on(c.read(&Path::Items { size: 10 })).expect("a read under the budget");
    }
    assert_eq!((c.sent, sent().len() as u32), (TURN_BUDGET, TURN_BUDGET), "the budget is spent to the last request");
    let refused = block_on(c.read(&Path::Items { size: 10 }));
    assert!(matches!(refused, Err(Fail::Budget(n)) if n == TURN_BUDGET), "{refused:?}");
    assert_eq!(sent().len() as u32, TURN_BUDGET, "the refused request never left");
    assert!(!refused.unwrap_err().is_auth(), "a spent budget drops no session");
    let line = Fail::Budget(TURN_BUDGET).line();
    assert!(line.contains("budget is spent") && line.contains("40 sent"), "{line}");
}

#[test]
fn a_client_with_a_session_spends_nothing_on_a_login_and_a_create_counts_too() {
    serve();
    let mut c = Client::new("till@x.test", "pw", None);
    block_on(c.read(&Path::Items { size: 10 })).expect("logs in");
    let live = c.session.clone();
    let mut again = Client::new("till@x.test", "pw", Some(live));
    block_on(again.read(&Path::Items { size: 10 })).expect("reads with the session it was given");
    assert_eq!(again.sent, 1, "no login, one request");
    block_on(again.create("{}")).expect("the fiscal create");
    assert_eq!(again.sent, 2, "a create is a request like any other");
}
