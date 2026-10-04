//! A PLATFORM IN MEMORY for route tests: a `World`, a platform administrator, and venues made
//! the way production makes them -- `POST /api/platform/hubs` (`platform::create_hub`), then the
//! owner signs in through `POST /api/auth/login` (`accounts::owner_login`). Every handler is
//! called through the same `Call`/`Ctx` the router hands it.

use super::mem::{block_on, World};
use super::{Ctx, Env};
use crate::wire::{Call, Reply};
use serde_json::{json, Value};
use std::rc::Rc;
use worker::{Method, Result};

/// The clock every site starts at: 2023-11-14T22:13:20Z.
pub const T0: i64 = 1_700_000_000_000;
pub const ADMIN_ID: &str = "admin-1";
pub const PLATFORM_HOST: &str = "dowiz.org";
pub const STRIPE_WEBHOOK_SECRET: &str = "whsec_test_secret";

pub struct Site {
    pub world: Rc<World>,
    pub now_ms: i64,
}

#[allow(dead_code)]
impl Site {
    pub fn new() -> Self {
        Self::with_secrets(&[])
    }
    /// A site whose Worker also holds these secrets (e.g. a Stripe key).
    pub fn with_secrets(extra: &[(&str, &str)]) -> Self {
        let mut w = World::new(T0);
        for (k, v) in extra {
            w.secrets.insert((*k).into(), (*v).into());
        }
        w.secrets.insert("JWT_SECRET".into(), "0123456789abcdef0123456789abcdef-test".into());
        w.secrets.insert("BOOTSTRAP_SECRET".into(), "bootstrap-secret-bootstrap-secret-0".into());
        w.secrets.insert("STRIPE_WEBHOOK_SECRET".into(), STRIPE_WEBHOOK_SECRET.into());
        w.vars.insert("PLATFORM_HOST".into(), PLATFORM_HOST.into());
        let site = Site { world: Rc::new(w), now_ms: T0 };
        // The administrator: a user record and the `admin` mark, written with the real writer.
        let hash = crate::auth::hash_password("admin-password").expect("hash");
        block_on(crate::identity_store::with_identity(&site.env(), move |t| {
            let rec = json!({"id": ADMIN_ID, "email": "admin@dowiz.org", "password_hash": hash}).to_string();
            t.put(
                crate::identity_store::K_USER,
                ADMIN_ID,
                &rec,
                &[(crate::identity_store::user_by_email("admin@dowiz.org"), ADMIN_ID.to_string())],
                &[],
            )
            .map_err(|e| worker::Error::RustError(format!("{e:?}")))?;
            t.put(crate::identity_store::K_ADMIN, ADMIN_ID, "{}", &[], &[])
                .map_err(|e| worker::Error::RustError(format!("{e:?}")))
        }))
        .expect("seed the administrator");
        site
    }

    pub fn env(&self) -> Env {
        Env::Mem(self.world.clone())
    }

    pub fn ctx(&self, params: &[(&str, &str)]) -> Ctx<crate::Req> {
        Ctx::mem(crate::Req { now_ms: self.now_ms }, self.env(), params)
    }

    /// Run one handler over a request, as the router would.
    pub fn run<F, Fut>(&self, handler: F, call: Call, params: &[(&str, &str)]) -> Reply
    where
        F: FnOnce(Call, Ctx<crate::Req>) -> Fut,
        Fut: std::future::Future<Output = Result<Reply>>,
    {
        block_on(handler(call, self.ctx(params))).expect("the handler answered Err")
    }

    /// Same, keeping an `Err` (a bodyless 500 in production) as the test's to judge.
    pub fn try_run<F, Fut>(&self, handler: F, call: Call, params: &[(&str, &str)]) -> Result<Reply>
    where
        F: FnOnce(Call, Ctx<crate::Req>) -> Fut,
        Fut: std::future::Future<Output = Result<Reply>>,
    {
        block_on(handler(call, self.ctx(params)))
    }

    /// A signed token, minted with the site's own key.
    pub fn sign(&self, claims: &crate::auth::Claims) -> String {
        crate::auth::sign(&self.env(), claims).expect("sign")
    }

    pub fn admin_token(&self) -> String {
        self.sign(&crate::auth::Claims::Owner {
            sub: ADMIN_ID.into(),
            user_id: ADMIN_ID.into(),
            active_location_id: None,
            iat: self.now_ms,
            exp: self.now_ms + 3_600_000,
        })
    }

    /// A venue and its owner, through `create_hub`; then the owner's token through `owner_login`.
    pub fn venue(&self, slug: &str, owner_email: &str) -> String {
        let body = json!({
            "slug": slug, "name": format!("Venue {slug}"), "phone": "+355690000000",
            "owner": {"email": owner_email, "password": "owner-password-1"},
            "dpa": crate::privacy::dpa::VERSION,
        });
        let r = self.run(
            crate::platform::create_hub,
            post(&format!("https://{PLATFORM_HOST}/api/platform/hubs"), &body).bearer(&self.admin_token()),
            &[],
        );
        assert_eq!(r.status_code(), 200, "create_hub {slug}: {}", r.body_str());
        self.login(slug, owner_email)
    }

    /// The owner's access token for `slug`, from the real login.
    pub fn login(&self, slug: &str, owner_email: &str) -> String {
        let r = self.run(
            crate::accounts::owner_login,
            post(
                &format!("https://{slug}.{PLATFORM_HOST}/api/auth/login"),
                &json!({"email": owner_email, "password": "owner-password-1"}),
            )
            .with_header("host", &format!("{slug}.{PLATFORM_HOST}")),
            &[],
        );
        assert_eq!(r.status_code(), 200, "login {owner_email}@{slug}: {}", r.body_str());
        r.body_value()["access_token"].as_str().expect("token").to_string()
    }

    /// A courier of `slug`: invited by its owner, claimed with the code. Returns (jwt, courier id).
    pub fn courier(&self, slug: &str, owner: &str, phone: &str) -> (String, String) {
        let r = self.run(
            crate::services::courier::hiring::invite_courier,
            post(&format!("https://{slug}.{PLATFORM_HOST}/api/owner/couriers/invite"), &json!({"phone": phone, "name": "Rider"}))
                .bearer(owner)
                .on(slug),
            &[],
        );
        assert_eq!(r.status_code(), 200, "invite {phone}: {}", r.body_str());
        let code = r.body_value()["code"].as_str().expect("code").to_string();
        let r = self.run(
            crate::accounts::courier_claim,
            post(
                &format!("https://{slug}.{PLATFORM_HOST}/api/courier/auth/claim"),
                &json!({"phone": phone, "code": code, "password": "courier-password-1"}),
            )
            .on(slug),
            &[],
        );
        assert_eq!(r.status_code(), 200, "claim {phone}: {}", r.body_str());
        let v = r.body_value();
        (v["jwt"].as_str().expect("jwt").to_string(), v["courier"]["id"].as_str().expect("id").to_string())
    }

    /// A member of staff of `slug` with `role` (waiter, kitchen, counter-manager): invited by the
    /// owner, claimed with the code. Returns the staff jwt.
    pub fn staff(&self, slug: &str, owner: &str, email: &str, role: &str) -> String {
        self.staff_full(slug, owner, email, role).0
    }

    /// `staff`, with the person's user id: (jwt, id).
    pub fn staff_full(&self, slug: &str, owner: &str, email: &str, role: &str) -> (String, String) {
        let r = self.run(
            crate::services::identity::staff_admin::invite_staff,
            post(&format!("https://{slug}.{PLATFORM_HOST}/api/owner/staff/invite"), &json!({"email": email, "name": "Staff", "role": role}))
                .bearer(owner)
                .on(slug),
            &[],
        );
        assert_eq!(r.status_code(), 200, "invite {email}: {}", r.body_str());
        let code = r.body_value()["code"].as_str().expect("code").to_string();
        let r = self.run(
            crate::services::identity::staff::staff_claim,
            post(
                &format!("https://{slug}.{PLATFORM_HOST}/api/staff/claim"),
                &json!({"email": email, "code": code, "password": "staff-password-1"}),
            )
            .on(slug),
            &[],
        );
        assert_eq!(r.status_code(), 200, "claim {email}: {}", r.body_str());
        let v = r.body_value();
        (v["jwt"].as_str().expect("jwt").to_string(), v["staff"]["id"].as_str().expect("staff id").to_string())
    }

    /// The venue's own object, for assertions on what was stored.
    pub fn object(&self, venue: &str) -> Rc<crate::hubdo::HubImages> {
        self.world.object(venue)
    }

    /// THE NIGHT AS THE PLATFORM RUNS IT (W-LOOP row 3): the cron's one request, then every
    /// alarm it causes, fired in order up to `at` -- the fan-out's batches and each venue's own
    /// night. The alarms fired, in order.
    pub fn night(&self, at: i64) -> Vec<(String, i64, usize)> {
        block_on(crate::cron::nightly(&self.env(), at));
        // The fan-out's later batches are a second apart: ten minutes covers any test's fleet.
        self.world.fire_alarms(at + 10 * 60_000)
    }

    /// GET a fold route of a venue's object, directly.
    pub fn fold(&self, venue: &str, path: &str) -> Reply {
        block_on(self.object(venue).route(get(&format!("https://hub{path}")))).expect("fold")
    }
}

pub fn get(url: &str) -> Call {
    Call::new(url, Method::Get).expect("url")
}

pub fn post(url: &str, body: &Value) -> Call {
    Call::new(url, Method::Post).expect("url").with_json(body)
}

/// `Authorization: Bearer <token>` and the venue's host, the two things every console call carries.
pub trait As {
    fn bearer(self, token: &str) -> Self;
    fn on(self, slug: &str) -> Self;
}

impl As for Call {
    fn bearer(self, token: &str) -> Self {
        self.with_header("authorization", &format!("Bearer {token}"))
    }
    fn on(self, slug: &str) -> Self {
        self.with_header("host", &format!("{slug}.{PLATFORM_HOST}"))
    }
}
