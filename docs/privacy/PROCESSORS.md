# Processor and recipient register

`processors v1 2026-09-24` · P9 of `docs/design/ROADMAP-2026-09-22.md` Wave P.

**The source of truth is code:** `workers/api/src/privacy/registry/outside.rs` (`PROCESSORS`, `HOSTS`).
The privacy notice at `https://<venue>.dowiz.org/privacy` is rendered from those rows and names a
processor only when its switch is on for that venue. `tools/gates/personal-data.sh` fails when an
`https://` host appears in `workers/api/src` or in the storefront's content policy without a `HOSTS`
row. If this page and the code disagree, the code is right and this page is stale.

**Roles.** The venue is the controller for its customers, guests, staff and couriers; dowiz is its
processor (the DPA, `docs/privacy/DPA-v1-2026-09-24.{sq,en,uk}.md`, served at `/dpa`). dowiz is the
controller only for owner/staff platform accounts and the waiting list.

## Parties that receive personal data

| id | Party | Receives | Role | On when | Where | Safeguard / terms |
|---|---|---|---|---|---|---|
| `cloudflare` | Cloudflare, Inc. | everything the platform stores (name, phone, address, map position, orders, notes, messages, bookings, payment amounts, IP) | dowiz's sub-processor | always | US company; worldwide network; each venue's Durable Object in one data centre (location **not yet measured**, see below) | Cloudflare Customer DPA with EU SCCs; Cloudflare states DPF certification |
| `stripe` | Stripe | payment data, IP | independent controller for payments, under its own terms | the platform's Stripe keys are set (`STRIPE_PUBLISHABLE_KEY`) | Ireland / US | Stripe DPA with EU SCCs; Stripe states DPF certification |
| `meta` | Meta (WhatsApp Business, Instagram) | phone, name, messages | the venue's processor, under the venue's contract with Meta | the venue set a WhatsApp or Instagram channel | Ireland / US | WhatsApp Business data processing terms; Meta states DPF certification |
| `telegram` | Telegram (the venue's kitchen chat) | first name + initial, order, address, note; phone **only for a delivery** (`notify::ticket_contact`) | the venue's chosen recipient | the venue set a bot token and chat | UAE company; servers in several countries | **none offered**; minimised ticket; Law 124/2024 Art. 41(3)(b) (necessary to perform the order) |
| `s3` | The venue's S3-compatible bucket | the nightly backup (log, catalogue, settings, posts, stock), sealable with the venue's key | the venue's processor | the venue set `cloud.s3.*` | wherever the venue's bucket is | the venue's contract; copies kept at most 22 days (`cloud::KEEP_WEEKLY_MS` = 21 d + the night) |
| `ai` | The venue's AI endpoint (`ai.endpoint`) | order id, status, total, items, age, kind, courier id; for the courier assistant the address line of that courier's own run; **no customer name or phone** (P11) | the venue's processor | `ai.enabled` and an `https://` endpoint | unknown — venue-chosen | DPA: none — venue-chosen |
| `ebills` | ebills.al | the venue's own POS login; dowiz only READS sales | the venue's processor | the venue linked its till | Albania | the venue's contract; no customer data sent |
| `tax` | e-fiskalizimi (`efiskalizimi-app.tatime.gov.al`) | staff operator codes, amounts | public authority, legal obligation | **never today**: `fiscal::SEND_ENABLED = false` | Albania | Law 124/2024 Art. 7(1)(c) |
| `osm` | OpenStreetMap Foundation (Nominatim) | a map pin's coordinates, IP — from the diner's browser | recipient (browser-direct) | the diner places an address pin | United Kingdom (EU adequacy) | used only when the diner places the pin |
| `openfreemap` | OpenFreeMap tiles | IP and which map area is drawn — from the browser | recipient (browser-direct) | a map is shown | not stated by the provider | no order, name or phone |

## Hosts that receive nothing about a person

| Host | Why it is not a recipient |
|---|---|
| `open.er-api.com` | exchange rates; the request carries a currency code only |
| `rates.dowiz` | a key in the Worker's own cache, never fetched |
| `hub` | the internal URL of a Durable Object stub |
| `{host}`, `{slug}.{platform}`, `{}.{}`, `*.dowiz.org` | the venue's / platform's own addresses |

## Not yet measured

- **Durable Object location of the live venues.** No `jurisdiction` or `locationHint` is passed
  (`grep -rn "jurisdiction\|locationHint" workers/api` → nothing), so Cloudflare places each object near
  the first request that created it. Measuring it needs a production probe (for example a temporary
  owner-only route returning the object's `cf.colo` from inside the object); this lane ran nothing
  against production. Until then the notice says "one Cloudflare data centre" and does not name it.
