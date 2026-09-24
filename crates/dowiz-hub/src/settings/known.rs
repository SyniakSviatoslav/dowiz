//! The DECLARED keys: the settings key space is closed, and this is it.
//!
//! Split out of `settings.rs` so the list can grow (the `tax.*` keys, blueprint
//! TAX-PRICE-CHANNEL §3.3) without growing the store's own file past the cap.

/// A key the hub actually consults, with what it means and what it defaults to.
///
/// Declared as data so the settings pane can be BUILT from it rather than
/// listing the same keys again in JavaScript — two lists of settings drift, and
/// the one that drifts is the one the owner reads.
pub struct Known {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub default: &'static str,
}

pub const KNOWN: &[Known] = &[
    Known {
        key: "ai.endpoint",
        label: "AI endpoint",
        // THIS DEFAULT WAS A LIE THE WORKER COULD NEVER HONOUR. It said "leave
        // it for a local Ollama; plain http is allowed for an address on this
        // machine" -- which was true of the old self-hosted service and is not
        // true of a Worker, which has no machine and cannot open a plain-http
        // socket at all. `assist.rs` refuses every non-https endpoint with 400,
        // so a venue that took the default saw the assistant reported ON and
        // got "ai.endpoint must be https from a Worker" from every question.
        // Empty is the honest default: not configured, and it says so.
        hint: "An OpenAI-compatible /v1 base URL. It must be https: a Worker \
               cannot reach a plain-http or a loopback address.",
        default: "",
    },
    Known {
        key: "ai.model",
        label: "AI model",
        hint: "The model name as that endpoint spells it.",
        default: "llama3.2",
    },
    Known {
        key: "ai.token",
        label: "AI token",
        hint: "Only needed for a hosted provider. A local model needs none.",
        default: "",
    },
    Known {
        key: "social.telegram.channel",
        label: "Telegram channel",
        hint: "Your public channel, e.g. @dubinsushi. Add your bot as an \
               administrator of it first, or posting will be refused.",
        default: "",
    },
    Known {
        key: "social.enabled",
        label: "Draft social posts",
        hint: "Off by default. When on, dowiz drafts posts about real changes to \
               your menu. Nothing is ever published until you approve it.",
        default: "0",
    },
    Known {
        key: "ai.enabled",
        label: "AI assistant",
        hint: "Off by default. Nothing is sent anywhere until this is on.",
        default: "0",
    },
    // ── owner notifications ──
    //
    // The bot is the VENUE'S, not the platform's: an owner makes one with
    // @BotFather in a minute and pastes its token here, so a hub needs no
    // operator to hand out Telegram. The token's key ends in `token`, so
    // `is_secret` redacts it everywhere it is read back.
    Known {
        key: "notify.telegram.token",
        label: "Telegram bot token",
        hint: "From @BotFather. The bot messages you about every new order; it \
               is also what posts to your channel when no platform bot exists.",
        default: "",
    },
    Known {
        key: "notify.telegram.chat",
        label: "Telegram chat",
        hint: "The chat the bot writes to: your own user id, or a group's id. \
               Send the bot any message first, then use the test button.",
        default: "",
    },
    Known {
        key: "notify.telegram.chat.bar",
        label: "Telegram chat (bar)",
        hint: "The chat the bot writes bar tickets to. Leave empty to send bar lines \
               to the main kitchen chat.",
        default: "",
    },
    Known {
        key: "alerts.exceptions.threshold",
        label: "Exception alert after",
        hint: "How many voids after the kitchen, comps, refunds, pay-outs (each kind counted apart) \
               in one till period before your Telegram chat is told. 0 = off.",
        default: "3",
    },
    Known {
        key: "alerts.exceptions.late_min",
        label: "Late amendment after (minutes)",
        hint: "A change to a round this long after it was placed is listed as an exception.",
        default: "30",
    },
    // ── WhatsApp, through Meta's Cloud API ──
    //
    // The venue's own WhatsApp Business number. A permanent System User token
    // and the number's phone-number id come from the Meta developer console;
    // `to` is where new orders are announced. The same credentials answer
    // customers who write to the number (see `channel_messages`).
    Known {
        key: "notify.whatsapp.token",
        label: "WhatsApp access token",
        hint: "A permanent token from Meta Business (System User, whatsapp_business_messaging).",
        default: "",
    },
    Known {
        key: "notify.whatsapp.phone_id",
        label: "WhatsApp phone number id",
        hint: "The numeric id of your business number in the WhatsApp Manager, not the number itself.",
        default: "",
    },
    Known {
        key: "notify.whatsapp.to",
        label: "WhatsApp number to notify",
        hint: "International format without +, e.g. 355691234567. Message the business number \
               from it once a day, or Meta only allows approved templates.",
        default: "",
    },
    // OFF BY DEFAULT, and this is the only setting in this list whose default
    // is chosen by somebody else's price list. From 2026-10-01 Meta bills
    // service and utility messages per message ($0.004-$0.046 each): two per
    // order at thirty orders a day is $7-$80 per venue per month, up to forty
    // times the whole Cloudflare bill. Telegram is free and carries the same
    // text, so a venue that has not asked for WhatsApp pushes does not pay for
    // them. Answering a customer who wrote first is NOT this setting: that is
    // inside Meta's free 24-hour window and stays on.
    Known {
        key: "notify.whatsapp.status",
        label: "Announce orders on WhatsApp",
        hint: "off by default. Meta bills each of these messages; Telegram carries the same \
               notice for nothing. Set to `on` to turn them on.",
        default: "off",
    },
    Known {
        key: "notify.whatsapp.verify",
        label: "Webhook verify token",
        hint: "Any phrase you choose; paste the same one into the webhook form in Meta's console.",
        default: "",
    },
    Known {
        key: "notify.meta.secret",
        label: "Meta app secret",
        hint: "Required for the inbox: every delivery must carry Meta's signature over it, \
               and an unsigned delivery is acknowledged and dropped.",
        default: "",
    },
    // ── Instagram, through the Graph API ──
    Known {
        key: "social.instagram.token",
        label: "Instagram access token",
        hint: "A long-lived token with instagram_content_publish and instagram_manage_messages.",
        default: "",
    },
    Known {
        key: "social.instagram.user_id",
        label: "Instagram account id",
        hint: "The numeric id of the professional account, from the Graph API or the Meta console.",
        default: "",
    },
    // ── cloud storage: any S3-compatible bucket ──
    //
    // R2, AWS S3, Backblaze B2, Wasabi, MinIO: keys alone connect them, which
    // is why this and not a Drive OAuth dance is the venue's off-site copy.
    Known {
        key: "cloud.s3.endpoint",
        label: "Storage endpoint",
        hint: "https://<account>.r2.cloudflarestorage.com, https://s3.eu-central-1.amazonaws.com, …",
        default: "",
    },
    Known {
        key: "cloud.s3.region",
        label: "Storage region",
        hint: "auto for R2; the region name for AWS and others.",
        default: "auto",
    },
    Known {
        key: "cloud.s3.bucket",
        label: "Bucket",
        hint: "The bucket the backups land in. It must already exist.",
        default: "",
    },
    Known {
        key: "cloud.s3.key",
        label: "Access key id",
        hint: "The key pair of a user allowed to write to that bucket.",
        default: "",
    },
    Known {
        key: "cloud.s3.secret",
        label: "Secret access key",
        hint: "Stored, never shown again.",
        default: "",
    },
    Known {
        key: "cloud.s3.prefix",
        label: "Object prefix",
        hint: "A folder inside the bucket; the venue's id is appended.",
        default: "dowiz",
    },
    // ── tax (BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22 §3.3) ──
    //
    // THE RATE IS AN INTEGER IN PARTS PER MILLION, never a decimal: 20 % is
    // `200000`. `POST /api/owner/settings` refuses `0.20` with a reason
    // (`services::ordering::tax_cfg::validate`). Unset means NOT CONFIGURED:
    // orders carry no `tax` block, and nothing changes for a live venue until
    // its owner sets this one key.
    Known {
        key: "tax.default_ppm",
        label: "VAT rate",
        hint: "Parts per million: 20% is 200000, 6% is 60000. Empty means tax is not configured.",
        default: "",
    },
    Known {
        key: "tax.prices_include",
        label: "Menu prices include VAT",
        hint: "true when the price on the menu is what the customer pays (Albania, the EU); \
               false when tax is added at checkout (the US).",
        default: "true",
    },
    Known {
        key: "tax.delivery_fee_ppm",
        label: "VAT rate on the delivery fee",
        hint: "Parts per million. Empty means the fee takes the VAT rate above.",
        default: "",
    },
    Known {
        key: "tax.schedule",
        label: "Scheduled VAT changes",
        hint: "Future changes only, as [{\"since_ms\":1767225600000,\"ppm\":70000}], at most 8.",
        default: "",
    },
    Known {
        key: "print.kitchen",
        label: "Kitchen printer",
        hint: "A name for the kitchen's printer (e.g. kitchen). When set, every order is also queued as a ticket the printer fetches: set the printer's server URL to /api/print/poll and its user name or password to a venue API key made for it.",
        default: "",
    },
    // ── the stamp card (BLUEPRINT-CRM-CONSENT-LOYALTY §3.5; workers/api/src/services/loyalty) ──
    //
    // OFF BY DEFAULT. The count is a fold over the orders, never a stored
    // number; `services::loyalty::stamps::validate` refuses anything else.
    Known {
        key: "loyalty.stamps.enabled",
        label: "Stamp card",
        hint: "0 or 1. When 1, every delivered or collected order, and every paid table, is a stamp; \
               a full card takes the reward off the next order. Shown only on the customer's own order page.",
        default: "0",
    },
    Known {
        key: "loyalty.stamps.n",
        label: "Stamps for a full card",
        hint: "A whole number from 2 to 20.",
        default: "10",
    },
    Known {
        key: "loyalty.stamps.reward_minor",
        label: "Stamp card reward",
        hint: "Taken off the next order, in the same whole minor units as a dish price. More than 0; \
               empty means no card runs.",
        default: "",
    },
    // ── fiscalisation (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.8; workers/api/src/fiscal/wire.rs) ──
    Known {
        key: "fiscal.since_ms",
        label: "Fiscalisation from",
        hint: "Epoch milliseconds (e.g. 1790000000000). From then on every order that takes money is \
               queued as a fiscal document with a 48 h deadline, shown in the health pane. Nothing is sent \
               to the tax platform yet. Empty means fiscalisation is not configured.",
        default: "",
    },
];
