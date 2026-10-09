//! Everything that leaves the Worker: the parties that receive personal data
//! (`PROCESSORS`), every host this crate or the storefront's CSP names
//! (`HOSTS`), and every key the front ends keep in a browser (`BROWSER`).
//! `tools/gates/personal-data.sh` reads the last two; the privacy notice and
//! `docs/privacy/PROCESSORS.md` read the first.

use super::Data::*;
use super::Recipient::{NotARecipient, Processor as To};
use super::Role::*;
use super::{DeviceKey, Host, Processor, Switch};

/// Every party outside this Worker that receives something about a person.
/// A row is shown to a venue's customers only when its `switch` is on there.
pub const PROCESSORS: &[Processor] = &[
    Processor { id: "cloudflare", name: "Cloudflare, Inc.",
        receives: &[Name, Phone, Address, Coordinates, OrderContent, Note, Messages, Booking, Payment, Ip],
        role: SubProcessor, switch: Switch::Always,
        location: "United States (company); data is served from Cloudflare's worldwide network and each venue's database lives in one Cloudflare data centre",
        safeguard: "Cloudflare's Customer Data Processing Addendum, which includes the EU Standard Contractual Clauses; Cloudflare states that it is certified under the EU-US Data Privacy Framework",
        terms: "https://www.cloudflare.com/cloudflare-customer-dpa/" },
    Processor { id: "meta", name: "Meta Platforms (WhatsApp Business, Instagram)",
        receives: &[Phone, Name, Messages],
        role: VenueProcessor, switch: Switch::Meta,
        location: "Ireland and the United States",
        safeguard: "the WhatsApp Business data processing terms, which the venue accepts with Meta; Meta states that it is certified under the EU-US Data Privacy Framework",
        terms: "https://www.whatsapp.com/legal/business-data-processing-terms" },
    Processor { id: "telegram", name: "Telegram (the venue's kitchen chat or a staff group the venue chooses)",
        receives: &[Name, Phone, Address, OrderContent, Note],
        role: VenueProcessor, switch: Switch::Telegram,
        location: "United Arab Emirates (company); servers in several countries",
        safeguard: "none offered by Telegram; the ticket is cut to what the kitchen needs (first name and initial, the order, the address, and a phone only for a delivery) and is sent because it is needed to fulfil your order (Law 124/2024 Art. 41(3)(b))",
        terms: "https://telegram.org/privacy" },
    Processor { id: "s3", name: "The venue's own backup storage (S3-compatible)",
        receives: &[Name, Phone, Address, Coordinates, OrderContent, Note],
        role: VenueProcessor, switch: Switch::Cloud,
        location: "wherever the venue's bucket is; the venue chooses it",
        safeguard: "the venue's own contract with its storage provider; copies can be sealed with the venue's key",
        terms: "chosen by the venue" },
    Processor { id: "stripe", name: "Stripe (card, Apple Pay, Google Pay)",
        receives: &[Payment, Ip],
        role: IndependentController, switch: Switch::Stripe,
        location: "Ireland and the United States",
        safeguard: "Stripe's Data Processing Agreement, which includes the EU Standard Contractual Clauses; Stripe states that it is certified under the EU-US Data Privacy Framework",
        terms: "https://stripe.com/legal/dpa" },
    // P11: the assistant's facts carry no customer name or phone.
    Processor { id: "ai", name: "The venue's AI assistant endpoint (ai.endpoint, set by the venue)",
        receives: &[OrderContent, Address],
        role: VenueProcessor, switch: Switch::Ai,
        location: "unknown: the venue chooses the endpoint",
        safeguard: "none from dowiz: the venue chooses the endpoint and its terms. It receives order id, status, total, items, age, kind and courier id, and for the courier assistant the address line of that courier's own delivery; never a customer's name or phone",
        terms: "chosen by the venue" },
    Processor { id: "ebills", name: "ebills.al (the venue's point of sale)",
        receives: &[StaffId, Password],
        role: VenueProcessor, switch: Switch::EbillsLink,
        location: "Albania",
        safeguard: "no transfer abroad. dowiz only READS the venue's sales from it with the venue's own login; no customer data is sent to it",
        terms: "the venue's contract with ebills.al" },
    Processor { id: "tax", name: "Tax authority e-fiscalisation (efiskalizimi-app.tatime.gov.al)",
        receives: &[StaffId, Payment],
        role: Authority, switch: Switch::FiscalSend,
        location: "Albania",
        safeguard: "a legal obligation (Law 124/2024 Art. 7(1)(c)). Sending is switched OFF in this build (`fiscal::SEND_ENABLED = false`), so nothing reaches it today",
        terms: "Albanian fiscalisation law" },
    // W-PUSH: the Worker posts a sealed message to the push service the
    // subscriber's own browser chose (notify/push/rail.rs); the endpoint
    // allowlist is notify/push/subs.rs `push_host`.
    Processor { id: "webpush", name: "Your browser's push service (Google FCM, Mozilla autopush, Apple Push, Microsoft WNS)",
        receives: &[Device],
        role: IndependentController, switch: Switch::Always,
        location: "the United States and the EU, depending on the browser's maker",
        safeguard: "only when you turn notifications on; the message is end-to-end encrypted to your device (RFC 8291), so the push service sees the device's address and the time, never the text",
        terms: "the browser maker's own terms (Google, Mozilla, Apple, Microsoft)" },
    // W-SMS: order-status texts, only to a customer who ticked the SMS box.
    // The default is the venue's OWN Android phone through SMSGate's cloud
    // relay; textbee or Twilio only with the venue's own account.
    Processor { id: "sms", name: "The venue's SMS gateway: its own Android phone through SMSGate (sms-gate.app), or textbee or Twilio with the venue's own account, and the mobile network that carries the text",
        receives: &[Phone],
        role: VenueProcessor, switch: Switch::Sms,
        location: "SMSGate's relay: not stated by the provider; Twilio: the United States; the SIM's mobile network: the venue's country",
        safeguard: "only when you tick the SMS box at checkout; the text carries the order number and the venue's name, never your name, address or order, and you can stop it by telling the venue STOP",
        terms: "the gateway the venue chooses (sms-gate.app, textbee.dev or twilio.com)" },
    // Contacted by the diner's own browser, not by the Worker.
    Processor { id: "osm", name: "OpenStreetMap Foundation (Nominatim address lookup)",
        receives: &[Coordinates, Ip],
        role: BrowserDirect, switch: Switch::Map,
        location: "United Kingdom (an EU adequacy decision covers it)",
        safeguard: "your browser asks it which street a map pin is on, only when you place the pin yourself",
        terms: "https://osmfoundation.org/wiki/Privacy_Policy" },
    Processor { id: "openfreemap", name: "OpenFreeMap (map tiles)",
        receives: &[Ip],
        role: BrowserDirect, switch: Switch::Map,
        location: "not stated by the provider",
        safeguard: "it receives only your device's address and which part of the map is drawn; no order, name or phone",
        terms: "https://openfreemap.org" },
];

/// Every `https://` (or `wss://`) host this crate names outside its tests, and
/// every host in `public/_headers`' content policy. The gate matches these
/// strings exactly as they are written in the source.
#[cfg_attr(not(test), allow(dead_code))]
pub const HOSTS: &[Host] = &[
    Host { host: "api.stripe.com", recipient: To("stripe") },
    Host { host: "js.stripe.com", recipient: To("stripe") },
    Host { host: "*.js.stripe.com", recipient: To("stripe") },
    Host { host: "hooks.stripe.com", recipient: To("stripe") },
    Host { host: "api.telegram.org", recipient: To("telegram") },
    // W-SMS (notify/sms/config.rs): the three gateways a venue can choose.
    Host { host: "api.sms-gate.app", recipient: To("sms") },
    Host { host: "api.textbee.dev", recipient: To("sms") },
    Host { host: "api.twilio.com", recipient: To("sms") },
    // W-TG: the `startgroup` deep link the owner's console shows; the hub never calls it.
    Host { host: "t.me", recipient: NotARecipient("a link the owner opens to pick a group; the hub never calls it and sends it nothing") },
    Host { host: "graph.facebook.com", recipient: To("meta") },
    Host { host: "www.ebills.al", recipient: To("ebills") },
    Host { host: "efiskalizimi-app.tatime.gov.al", recipient: To("tax") },
    Host { host: "nominatim.openstreetmap.org", recipient: To("osm") },
    Host { host: "tiles.openfreemap.org", recipient: To("openfreemap") },
    // cloud.rs: `https://{endpoint}` of the venue's own bucket.
    Host { host: "{}", recipient: To("s3") },
    Host { host: "open.er-api.com", recipient: NotARecipient("exchange rates: the request carries a currency code and nothing else") },
    Host { host: "rates.dowiz", recipient: NotARecipient("a cache key in this Worker's own cache; never fetched") },
    Host { host: "hub", recipient: NotARecipient("the internal URL of a Durable Object stub; it never leaves Cloudflare") },
    // BN2 (hubdo/publish.rs, store/shell.js): the platform's own R2 bucket behind a custom domain.
    Host { host: "dowiz.org", recipient: NotARecipient("the contact (`sub`) named inside a push token (notify/push/vapid.rs SUBJECT); never fetched") },
    Host { host: "cdn.dowiz.org", recipient: NotARecipient("the venue's published menu and photos, read by every customer's browser; a read carries no identity and nothing about a person is written there") },
    Host { host: "cron", recipient: NotARecipient("the internal URL of the minute cron's runner object (cron.rs); it never leaves Cloudflare") },
    Host { host: "{host}", recipient: NotARecipient("the venue's own storefront address, printed on a table's QR code") },
    Host { host: "{slug}.{platform}", recipient: NotARecipient("the venue's own addresses on this platform") },
    Host { host: "{}.{}", recipient: NotARecipient("the venue's own addresses on this platform") },
    Host { host: "*.dowiz.org", recipient: NotARecipient("this platform's own live-update sockets") },
    // W-SENSE (services/venue/context.rs): the weather AT THE VENUE, asked by the Worker at most hourly.
    Host { host: "api.open-meteo.com", recipient: NotARecipient("the weather at the venue: the request carries the venue's own coordinates (two decimals) and nothing about a guest; the Worker asks, never the guest's browser") },
    Host { host: "weather.dowiz", recipient: NotARecipient("a cache key in this Worker's own cache; never fetched") },
    Host { host: "context.dowiz", recipient: NotARecipient("a cache key in this Worker's own cache; never fetched") },
];

const fn key(key: &'static str, holds: &'static [super::Data], surface: &'static str) -> DeviceKey {
    DeviceKey { key, prefix: false, holds, surface }
}
const fn prefix(key: &'static str, holds: &'static [super::Data], surface: &'static str) -> DeviceKey {
    DeviceKey { key, prefix: true, holds, surface }
}

/// Every `dw_*` / `dowiz.*` key the front ends keep in `localStorage`,
/// `sessionStorage` or IndexedDB. They stay on the person's device.
pub const BROWSER: &[DeviceKey] = &[
    // The storefront (store/).
    key("dw_name", &[Name], "store"),
    key("dw_phone", &[Phone], "store"),
    key("dw_addr", &[Address], "store"),
    key("dw_addrs", &[Address], "store"),
    key("dw_addr_parts", &[Address], "store"),
    key("dw_orders", &[OrderContent], "store"),
    key("dw_last_order", &[OrderContent], "store"),
    key("dw_bookings", &[Booking], "store"),
    key("dw_avoid", &[Note], "store"),
    // W-MR7: the guest's taste and menu behaviour, scored on the phone, ON by default, off in one tap
    // (operator ruling 2026-10-04; store/taste.js). IndexedDB.
    key("dowiz.taste.v1", &[OrderContent, Taste, Device], "store"),
    // The switch itself, '0' (off) or absent/'1' (on): nothing personal.
    key("dw_taste_on", &[], "store"),
    // The guest's objection to the venue's profile, '1' or absent: nothing personal (W-MR0 MR8).
    key("dw_taste_objected", &[], "store"),
    prefix("dw_cart_", &[OrderContent], "store"),
    prefix("dowiz.rev.", &[Coordinates, Address], "store"),
    key("dw_lang", &[], "store"),
    key("dw_table", &[], "store"),
    key("dw_src", &[], "store"), // W-QR: `{src:"bag", c}` until checkout, sessionStorage only
    key("dw_install_asked", &[], "store"),
    key("dw_install_hide", &[], "store"),
    prefix("dw_boot_", &[], "store"),
    key("dowiz.currency", &[], "store"),
    key("dowiz.blocks.v1", &[], "store"),
    // The customer kit (kit/).
    key("dowiz.kit.contact", &[Name, Phone], "kit"),
    key("dowiz.kit.address", &[Address], "kit"),
    key("dowiz.kit.addresses", &[Address], "kit"),
    key("dowiz.kit.orders", &[OrderContent], "kit"),
    key("dowiz.basket.v1", &[OrderContent], "kit"),
    key("dowiz.basket.seeded", &[], "kit"),
    key("dowiz.favourites", &[], "kit"),
    key("dowiz.compare.v1", &[], "kit"),
    key("dowiz.search.recent", &[], "kit"),
    key("dowiz.install.snoozed", &[], "kit"),
    key("dw_kit_theme", &[], "kit"),
    key("dw_kit_avatar", &[Photo], "kit"),
    key("dw_kit_coupon", &[], "kit"),
    key("dw_kit_addr", &[Address], "kit"),
    key("dw_kit_geo", &[Coordinates], "kit"),
    // The owner console (admin/).
    key("dw_at", &[Session], "admin"),
    key("dw_rt", &[Session], "admin"),
    key("dw_loc", &[], "admin"),
    key("dw_admin_lang", &[], "admin"),
    key("dw_admin_cur", &[], "admin"),
    // The console's light/dark choice (admin/theme.js): a word, nothing personal.
    key("dw_theme", &[], "admin"),
    key("dowiz.replica.v1", &[Name, Phone, Address, OrderContent], "admin"),
    prefix("dw_guide_", &[], "admin"),
    // Lesson progress (lib/learn.js), one index per app: lesson ids and done/paused, nothing personal.
    prefix("dw_learn_", &[], "admin"),
    // W-STORE: the storage this device sends deliveries to (admin/stock-storages-logic.js), a storage id, nothing personal.
    key("dowiz.recvStore", &[], "admin"),
    // The courier app (courier/).
    key("dw_c_jwt", &[Session], "courier"),
    key("dw_c_last", &[Address, OrderContent], "courier"),
    key("dw_c_theme", &[], "courier"),
    key("dw_c_lang", &[], "courier"),
    key("dowiz.outbox", &[OrderContent], "courier"),
    // The waiter's room (room/).
    key("dw_room_session", &[Session], "room"),
    key("dw_room_last", &[OrderContent], "room"),
    key("dw_room_till", &[], "room"),
    key("dw_room_lang", &[], "room"),
    key("dw_room_theme", &[], "room"),
    // W-CV8a: the board's new-ticket chime on/off for this device ('0' = off), nothing personal.
    key("dw_room_sound", &[], "room"),
    key("dw_room_currency", &[], "room"),
    key("dowiz.room.outbox", &[OrderContent], "room"),
    // W-OFFSALE: the tablet's journal of its offline cash sales (lines, totals, no
    // customer), and the cached public menu they are priced from.
    key("dowiz.room.sales", &[OrderContent], "room"),
    key("dw_room_menu", &[], "room"),
    // The wiki (wiki/): the language the reader last chose, nothing personal.
    key("dw_wiki_lang", &[], "wiki"),
];
