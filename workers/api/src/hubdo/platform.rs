//! THE TWO ROUTES THAT NEED THE PLATFORM ITSELF (split out of `hubdo.rs`,
//! W-SPLIT): the runner's minute cron and the socket upgrade. They take a
//! `worker::Request` rather than a `Call`, so they stay out of `route` and out
//! of `cargo test`'s reach; `fetch` asks here first. `edges/tests.rs` reads
//! these arms with the table in `hubdo/route.rs`.

use super::{host, HubImages, TAG_COURIER};
use worker::*;

impl HubImages {
    /// `Some` when the request was one of the two platform routes, answered.
    pub(super) async fn platform_route(&self, req: &worker::Request) -> Result<Option<worker::Response>> {
        let path = req.path();
        let mut seg = path.split('/').filter(|s| !s.is_empty());
        if seg.next() != Some("fold") {
            return Ok(None);
        }
        match (req.method(), seg.next().unwrap_or("")) {
            // ── THE MINUTE CRON, for one venue (`crate::cron`) ──
            //
            // Only a runner object is asked this: the Worker addresses
            // `cron~<venue>`, never a venue's own hub, so the jobs below
            // call the venue's object as a different instance and never
            // wait on themselves.
            (Method::Post, "cron") => {
                let url = req.url()?;
                let Some((venue, now_ms)) = crate::cron::parse_runner_query(
                    url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())),
                ) else {
                    return worker::Response::error("cron needs a venue and a clock", 400).map(Some);
                };
                let Some(env) = self.state.env() else {
                    return worker::Response::error("no runner outside the platform", 500).map(Some);
                };
                crate::cron::run(&crate::edge::Env::Live(env.clone()), &venue, now_ms).await;
                worker::Response::ok("done").map(Some)
            }
            // ── THE SOCKET ──
            //
            // The Worker has already decided WHO this is and which topic
            // they may hear; the tag it sends is that decision. This object
            // is not reachable from the internet, so the tag arriving here
            // is the Worker's word, not a client's.
            (Method::Get, "socket") => {
                let url = req.url()?;
                let tag = url
                    .query_pairs()
                    .find(|(k, _)| k == "tag")
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
                if tag.is_empty() {
                    return worker::Response::error("a socket needs a tag", 400).map(Some);
                }
                let protocol = url
                    .query_pairs()
                    .find(|(k, _)| k == "proto")
                    .map(|(_, v)| v.to_string());
                self.accept(&tag, protocol.as_deref()).map(Some)
            }
            _ => Ok(None),
        }
    }

    /// Accept one socket, tagged by what it is allowed to hear.
    ///
    /// HIBERNATION IS THE POINT. `accept_websocket_with_tags` hands the socket
    /// to the runtime, which keeps it open while the object sleeps; duration
    /// is billed only while the object is actually running. A socket held by
    /// the object itself would keep it awake and turn a free connection into a
    /// billed one.
    fn accept(&self, tag: &str, protocol: Option<&str>) -> Result<worker::Response> {
        let state = match &self.state {
            host::Host::Live { state, .. } => state,
            #[cfg(test)]
            host::Host::Mem(_) => return worker::Response::error("no sockets in memory", 500),
        };
        let pair = WebSocketPair::new()?;
        // A COURIER GETS TWO TAGS: `courier`, which the queue broadcast fans
        // out to, and `courier:<id>`, which says whose socket this is. The
        // second is the only thing the object knows about the client, so it is
        // what a position is attributed to.
        if let Some(id) = tag.strip_prefix("courier:") {
            let both = [TAG_COURIER, tag];
            let _ = id;
            state.accept_websocket_with_tags(&pair.server, &both);
        } else {
            state.accept_websocket_with_tags(&pair.server, &[tag]);
        }
        // ── THE SUBPROTOCOL IS ECHOED WHERE THE RESPONSE IS BUILT ──
        //
        // A client that offers subprotocols requires the server to choose one,
        // or the browser fails the handshake on its own side. The echo was
        // being set on the Worker's side, on the response that came BACK from
        // this object -- and a Response that already wraps a platform one has
        // immutable headers, so that threw and the handshake answered 500.
        // Here the response has not been built yet, so the header is part of
        // its construction.
        let res = worker::Response::from_websocket(pair.client)?;
        match protocol {
            Some(p) => {
                let headers = Headers::new();
                headers.set("Sec-WebSocket-Protocol", p)?;
                Ok(res.with_headers(headers))
            }
            None => Ok(res),
        }
    }
}
