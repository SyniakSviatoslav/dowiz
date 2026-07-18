# OPUS-R5 — Multi-Vendor Ecosystem & Self-Host Ops Research

> **Scope:** Wave-0 mechanism research for six operator-settled decisions in
> `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.9 / §16.15 / §16.17 /
> §16.27 / §16.46 / §16.51 / §16.54 / §16.57–§16.59.
> **Status:** research + concrete Wave-0 recommendations. Not a blueprint, not implementation.
> **Author:** Opus research pass, 2026-07-18. **Ground rule:** every §16 decision below is
> operator-binding and **not** re-litigated here — this document only answers *how*, with
> real, current prior art (cited URLs), never *whether*.

---

## 0. What this pass locked (the four required Wave-0 recommendations, up front)

| Area | Wave-0 recommendation (one line) | Confidence |
|------|----------------------------------|------------|
| **Encrypted backup** (§16.27) | `age` (Rust lib, X25519 streaming) for the client-side crypto envelope, wrapped by the already-live `rclone` sync to `hetzner:dowiz` or vendor S3 — **not** rclone-crypt as the primary cipher. | High |
| **Auto-update + rollback** (§16.27) | A/B slot + atomic `current` symlink flip + health-check-before-promote, driven by a small Rust supervisor. `self_update` crate for *fetch/verify only* — it has **no rollback**, so the slot machinery is the real answer. | High |
| **Food-court data model** (§16.15/§16.46) | Single hub DB, `vendor_id` scoping column on catalog/menu tables (row-level, not schema-per-vendor); ONE shared `order` + `order_item.vendor_id` fan-out; ONE courier pool table hub-wide; split settlement via provider-side Connect-style transfers, computed client-side per §16.49. | High |
| **Brand preview draft/live** (§16.9/§16.58) | Two 5-token records (`published` + `draft`) selected by one preview flag feeding the **same** wgpu render pipeline via a swapped uniform buffer. No second pipeline, no second renderer. Publish = atomic copy draft→published. | High |

Two secondary topics (content moderation §16.51, AGPL+TM+DCO packaging §16.54/§16.57) are
covered in §5–§6; the repo **already has** the packaging file set (§6 is refinement, not build).

---

## 1. Encrypted backup to S3-compatible storage (§16.27)

### 1.1 The requirement, precisely
Self-hosted hubs get built-in encrypted auto-backup to `hetzner:dowiz` **or** the vendor's own
S3-compatible target. The hard invariant: **dowiz (and the storage provider) never see
plaintext** — this is client-side / zero-knowledge encryption, encrypt-before-upload. The
`hetzner:dowiz` rclone remote is already confirmed live (`BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md`).

### 1.2 The two real options, and why they differ

**Option A — rclone `crypt` remote (zero-knowledge wrapper over the S3 remote).**
rclone's `crypt` remote wraps another remote and encrypts *before* upload / decrypts *after*
download, on the local system, leaving ciphertext at rest in the wrapped backend — genuine
client-side encryption, the provider never sees plaintext or filenames.
([rclone crypt docs](https://rclone.org/crypt/))
- **Cipher:** file content = NaCl **SecretBox (XSalsa20 + Poly1305)**, 64 KiB chunks, each with
  a 16-byte Poly1305 tag; filenames = **EME over AES-256**, then modified-base32.
- **Key derivation:** **scrypt (N=16384, r=8, p=1)** → 80 bytes, from the password + optional
  salt (`password2`). ([rclone crypt docs](https://rclone.org/crypt/))
- **Hard caveat:** *"It is not possible to change the password/key of already encrypted
  content."* A password change means re-encrypting and re-uploading everything. The password in
  `rclone.conf` is only *lightly obscured* (AES-CTR, static key) — the config file itself must be
  protected. ([rclone crypt docs](https://rclone.org/crypt/),
  [RcloneView zero-knowledge guide](https://rcloneview.com/support/blog/encrypt-cloud-backups-crypt-remote-guide-rcloneview))

**Option B — `age` (Rust library) as the crypto envelope, plain rclone as the transport.**
`age` is a modern file-encryption format with a first-class Rust crate. The library exposes
`Encryptor`/`Decryptor` with **streaming** and async I/O, so an arbitrarily large backup archive
is encrypted without loading it into memory; recipients can be **X25519 public keys**, **scrypt
passphrases**, SSH keys, or a **post-quantum `tagpq::Recipient`**. Output is *binary and
non-malleable*. ([age crate docs](https://docs.rs/age/latest/age/),
[rage / str4d](https://github.com/str4d/rage), [age on lib.rs](https://lib.rs/crates/age))
- The Rust wrapper `age-crypto` gives an idiomatic high-level API over X25519 + scrypt with
  binary or PEM-armored output. ([age-crypto crate](https://crates.io/crates/age-crypto))
- **Version note:** `age` is pre-1.0 (0.12.x) — *"all crate versions prior to 1.0 are beta
  releases for testing purposes only"* per its own docs. The **format** is stable and widely
  deployed; the Rust crate API may still shift. ([age crate docs](https://docs.rs/age/latest/age/))

### 1.3 Recommendation — **age envelope + rclone transport**, not rclone-crypt

**Reasoning tied to dowiz invariants (Rust-native, self-custody, PQ-aware):**

1. **Public-key recipients beat a shared password.** rclone-crypt is *symmetric* — the same
   password that decrypts is the password that must live on the hub to encrypt. `age` with an
   **X25519 recipient** lets the hub hold only the *public* key; the private identity lives only
   in the vendor's data-wallet / offline. This is the same self-custody framing §16.47 already
   chose for the customer wallet and §16.48 for owner certs — apply it consistently to backups.
2. **Rust-native, in-process.** §16.51's *"без тяжких бібліотек"* / lean-kernel directive and the
   repo's Rust-native default (`rust-native-bare-metal-decision-2026-07-14`) favor an in-process
   crate over shelling out to the rclone-crypt binary for the *crypto*. rclone stays as the
   **transport** (`rclone copy`/`sync` of the already-encrypted `.age` blobs to `hetzner:dowiz`
   or vendor S3) — that half is already proven live.
3. **Post-quantum path exists.** `age`'s `tagpq::Recipient` gives a forward path consistent with
   the whole bebop2 ML-DSA-65 PQ posture; rclone-crypt has no PQ story.
4. **Key rotation.** Because age re-encrypts per-archive to a recipient set, adding/rotating a
   recipient is a new backup with a new recipient list — no "can never change the password"
   trap that rclone-crypt carries.

**Concrete Wave-0 shape:**
- Hub tars its state (event log + pgrust/PgStore snapshot per W13) → streams through
  `age::Encryptor::with_recipients([vendor_x25519_pubkey, dowiz_break_glass_pubkey?])` →
  writes `snapshot-<ts>.age`.
- A tiny scheduler (systemd timer or in-kernel tick) runs `rclone copy snapshot-*.age
  <remote>:dowiz/<hub-id>/` where `<remote>` is `hetzner` (default) or the vendor's configured
  S3 remote. rclone handles S3 auth, retry, and 3-2-1 fan-out to multiple remotes.
  ([rclone S3 guide](https://danubedata.ro/blog/rclone-s3-compatible-storage-complete-guide-2026))
- **Restore** = `rclone copy` back + `age -d` with the vendor's identity. dowiz-side operator
  cannot decrypt (no private key) — invariant satisfied by construction, not by policy.
- Keep rclone-crypt available as a **fallback** transport for vendors who want filename
  encryption too, but do not make it the primary cipher.

**Open sub-decision (flag for blueprint):** whether a `dowiz_break_glass_pubkey` is *ever* in the
recipient set. Including it lets dowiz assist recovery but weakens "dowiz never sees plaintext"
to "dowiz *could* if it used the break-glass key." Default recommendation: **no dowiz recipient**
— pure vendor self-custody, matching §16.47's "loss is the user's own responsibility."

---

## 2. Auto-update with rollback for self-hosted Rust software (§16.27)

### 2.1 The requirement
Auto-update **by default** (keeps the mesh from fragmenting into stale protocol versions), with
an **explicit owner-triggered rollback** to a prior version. Safety-critical: a bad auto-update
must not brick a self-hosted hub.

### 2.2 What the obvious crate does *not* give you
`self_update` (jaemk) is the standard Rust self-update crate. It fetches releases from **GitHub /
GitLab / Gitea / S3-compatible** backends, does in-place binary replacement via `self_replace`,
and offers transactional multi-file `MoveAll` (all-or-nothing), plus **signature verification**
(`signatures` feature, zipsign over `.zip`/`.tar.gz`) and **SHA-256/512 checksums**.
([self_update on GitHub](https://github.com/jaemk/self_update))
**But:** it is **forward-only** — the docs contain *no rollback mechanism* and *no built-in
version pinning*. It compares current vs. latest and updates upward; that's it. Its `rename`-based
replacement also *cannot cross filesystems* (staging dir and destination must share a filesystem).

So `self_update` solves *fetch + verify + atomic single-binary swap*, but **not** the operator's
rollback requirement. Rollback has to be built around it.

### 2.3 The real pattern — A/B slots + atomic symlink flip + health gate

This is the mature, boring, correct pattern from blue-green / zero-downtime deploy practice,
scaled down to a single self-hosted box:

1. **Two release slots** (`releases/<version>/`), `current` is a **symlink** to the live one.
   A symlink switch is a single `rename()` syscall — genuinely atomic, no request ever sees a
   half-deployed state; rollback is re-pointing the symlink at the previous slot, equally atomic.
   ([blue-green / symlink flip summary](https://www.deployhq.com/blog/zero-downtime-deployments-keeping-your-application-running-smoothly),
   [single-EC2 blue-green](https://saadh393.github.io/projects/blue-green-deployment-zero-downtime))
2. **Stage into the idle slot**, verify signature + checksum (via `self_update`'s verify path)
   **before** touching `current`.
3. **Health-check before promote.** Start the new version on a scratch port / in a probe mode,
   hit its health endpoint; **promote only on healthy**. Critical nuance from the ops literature:
   a health check confirming the process *responds* is **not** proof the app *works* — the probe
   must exercise a real code path (e.g. open the event-log DB, replay-verify last snapshot), and
   should return **503 during warm-up, 200 only when truly ready**.
   ([K8s automated-rollback-on-health-failure](https://oneuptime.com/blog/post/2026-02-09-automated-rollback-health-failures/view))
4. **Auto-rollback on failed health**, and **keep the previous slot** so the owner-triggered
   rollback (§16.27's explicit requirement) is just "flip `current` back + restart" — seconds,
   no re-download. Old slot stays around exactly to make rollback cheap.
5. **Version pinning** = an owner-set "pinned version" that the auto-updater refuses to move past
   (build it via `self_update`'s `ReleaseSource` trait, since pinning isn't built in).
6. **Config/state hot-reload** (not full restart) for config-only changes via `arc-swap`
   lock-free atomic swap, validating the new config **before** the swap and keeping the old one
   active on validation failure. ([hot config reload in Rust](https://oneuptime.com/blog/post/2026-01-25-hot-configuration-reloading-rust/view))

**Rust supervisor sketch (Wave-0):**
- A small `dowiz-hub-supervisor` process owns the slot dir, the `current` symlink, and the
  auto-update timer. It runs `self_update` for *fetch + verify only* (`.no_confirm(true)` so it
  never blocks on a TTY — the crate blocks on interactive confirm by default, fatal for a
  daemon), stages into the idle slot, runs the health probe, flips the symlink, and supervises
  restart. On health failure it never flips; on post-flip crash-loop it flips back.
- **Migration safety** is the sharp edge: forward-only DB migrations (per the repo's
  `build-stage` skill discipline) mean rollback of *code* can outrun rollback of *schema*. The
  supervisor must snapshot the event-log/pgrust state (reuse the §1 `age` snapshot) **before**
  promote, so an owner rollback restores code *and* a compatible state, not code against a
  migrated-forward schema.

### 2.4 Recommendation
Build the **A/B slot + symlink-flip + health-gate supervisor** as the mechanism; use
`self_update` as a *component* for signed/checksummed fetch, never as the whole answer. Auto-update
default-on with a pinned-version override and a one-command owner rollback that re-points the
symlink and restores the pre-promote state snapshot. This is directly modeled on established
blue-green practice, degraded gracefully to one host.

---

## 3. Food-court multi-vendor data model (§16.15 / §16.46 / §16.17 / §16.49)

### 3.1 The requirement
ONE hub can host **multiple vendors sharing one delivery/courier pool** (food-court model).
Each vendor has its **own free-form menu/catalog** (§16.17, no fixed dowiz taxonomy). A customer
gets **one unified cart across vendors, one delivery, one checkout** (§16.46), which forces
**split payment/settlement** across vendors within the hub. Card data flows client→provider
directly; the hub sees only a token (§16.49).

### 3.2 Real prior art (food-hall / multi-vendor POS)
This exact shape is a solved product category. Food-hall / food-court POS systems let a guest
"order from multiple stalls in a single transaction, route each item to the correct vendor's
kitchen, and handle split payments and vendor settlement automatically."
([Tabski food-court POS](https://tabski.com/food-court-pos/),
[GoTab food-hall](https://gotab.com/business-type/food-hall-pos))
Key modeled patterns to copy:
- **Shared browse, single checkout:** all vendor menus behind one QR/entry; add items from many
  stalls; one checkout. ([Chowbus multi-vendor](https://www.chowbus.com/blog/best-food-court-pos-system))
- **Per-item routing:** each line item routes to *its* vendor's kitchen/KDS/printer.
  ([Tabski](https://tabski.com/food-court-pos/))
- **Tenant data isolation within the hall:** *"Tenants see only their own orders, products, and
  KDS tickets — never another vendor's data,"* while the hall keeps hall-wide visibility.
  ([GoTab multi-vendor deep dive](https://gotab.com/latest/everything-food-hall-operators-need-to-know-about-gotabs-multi-vendor-pos))
- **Auto-split settlement:** one guest transaction split and deposited per-vendor (their true
  sales, taxes, tips, fees). ([GoTab](https://gotab.com/business-type/food-hall-pos))

### 3.3 The split-payment mechanism (marketplace prior art)
Stripe Connect's **separate charges and transfers** is the canonical "one payment, N sellers"
primitive, and Stripe's own docs name a **restaurant delivery platform (DoorDash)** as the model
use case: charge once on the platform account, then `transfer` funds to each connected (vendor)
account, amounts you decide, associated via a `transfer_group`; `checkout.session.completed`
fires on payment. Transfer and charge amounts need not match, so one charge splits across many
vendors. ([Stripe separate charges & transfers](https://docs.stripe.com/connect/separate-charges-and-transfers),
[Stripe marketplace](https://docs.stripe.com/connect/marketplace))
This lives in the **provider's** API (§16.49: split logic lives in the provider's Connect-style
split, not in hub code), and is exposed through dowiz's §16.13 multi-provider payment-adapter
port — Stripe Connect is one adapter, not a hardcoded dependency.

### 3.4 Recommendation — row-scoped single-DB tenancy, shared fulfillment

**Do NOT** use schema-per-vendor or DB-per-vendor inside a hub. A food-court hub is *one* trust
and *one* fulfillment domain; the vendors are catalog partitions, not isolated tenants. Use a
**single hub database with a `vendor_id` scoping column** on catalog-side tables:

```
hub                      (the isolated per-venue unit; §16.6)
 └─ vendor               (0..N per hub; 1 for the common single-vendor case, N for food-court)
      └─ catalog_node    (vendor-authored free-form tree — categories/modifiers/variants;
                          §16.17: NO fixed dowiz taxonomy. Adjacency-list/closure-table,
                          vendor_id-scoped. A "menu item" is just a leaf node with a price.)
 └─ courier_pool         (ONE per hub, shared across all vendors; §16.15)
 └─ order                (ONE per checkout, hub-scoped, NOT vendor-scoped; §16.46 unified cart)
      └─ order_item      (each carries vendor_id → per-vendor kitchen routing + settlement split)
 └─ settlement_split     (derived per order: amount owed to each vendor_id; feeds the
                          provider-side transfer, §16.49; empty/trivial for single-vendor hubs)
```

**Why row-scoping, not schema-per-vendor:**
- The unified cart (§16.46) is a **cross-vendor** aggregate — an `order` spanning `order_item`s
  with different `vendor_id`s. Schema-per-vendor would make the single most important query
  (the cart) a cross-schema join, fighting the model instead of expressing it.
- The courier pool is explicitly **shared** (§16.15) — it belongs to the hub, not any vendor.
- Free-form catalog (§16.17) is naturally a `vendor_id`-scoped tree; no shared taxonomy to
  reconcile. This matches the food-hall reality that *"each vendor creates their own menus and
  products to align with their brand."* ([Chowbus](https://www.chowbus.com/blog/best-food-court-pos-system))
- Data-isolation ("vendor sees only their own orders/products") becomes an **RLS/`vendor_id`
  filter**, not a physical boundary — the repo already has RLS-FORCE discipline; reuse it. The
  single-vendor hub is just the degenerate `N=1` case with zero split logic (matches §16.16's
  "no split needed" *for the common case*; §16.46 reopens it *only* for food-court).

**Kitchen routing / KDS:** `order_item.vendor_id` is the routing key — the same per-item routing
the POS prior art uses. No central "order broker"; the hub fans line items to each vendor's
view by `vendor_id`.

**Settlement:** compute `settlement_split` per order (sum of `order_item` totals per `vendor_id`,
plus each vendor's own tax rate from §16.49's free-schema vendor-set VAT), hand the split to the
payment adapter, which executes it via the provider's transfers (Stripe Connect
`transfer_group`). dowiz still touches **no money** beyond orchestrating the adapter call — the
provider moves the funds, consistent with §16.16/§16.24.

---

## 4. Brand preview: draft vs. live over a real render engine (§16.9 / §16.58)

### 4.1 The requirement
Brand customization is the **5-token Sheet** (accent / ink / paper / type / radius) over the
fixed dowiz **Sea** ambient layer (§16.9). §16.58 adds: the vendor must see a **live draft /
staging preview** — their 5-token changes rendered in the **real field engine** — before
customers see them, so they can't publish-then-regret. The whole UI is **wgpu** (§16.30), so
there is no DOM/CSS fallback path to lean on.

### 4.2 Prior art — how theme editors do draft/live without a second pipeline

- **Shopify** keeps *one* live theme and N drafts; preview is the **same storefront rendered
  against real data** but reached via an isolated preview URL (`<token>-<shop_id>.shopifypreview.com`,
  short-TTL), so live customers never hit draft code; **publish promotes a draft to live**, and
  crucially *store data (products/menus) is unchanged by the theme switch* — theme is a
  presentation layer swapped over stable content.
  ([Shopify add/preview/publish themes](https://help.shopify.com/en/manual/online-store/themes/adding-themes))
- **WordPress** uses the **Customizer**: an unpublished set of changes rendered live in a preview
  frame, saved as a **draft**, schedulable, and shareable via a preview link, then published
  atomically. ([WPBeginner preview-before-live](https://www.wpbeginner.com/beginners-guide/how-to-preview-your-wordpress-website-before-going-live/))
- **Design-token systems** are explicitly built to hold **multiple token sets** and switch
  between them dynamically so every visual element updates consistently from one source — you
  don't fork the renderer, you swap the token set feeding it.
  ([Advanced theming with design tokens](https://david-supik.medium.com/advanced-theming-techniques-with-design-tokens-bd147fe7236e),
  [Design Tokens Format Module](https://www.designtokens.org/tr/drafts/format/))

The universal shape: **content and render pipeline stay single; only the token set has a
draft/live pair, selected by a flag.** Nobody duplicates the renderer.

### 4.3 Recommendation — one pipeline, two token buffers, a preview flag

Because the 5-token Sheet is *tiny* (5 scalar/color values + a type id + a radius), the entire
draft/live problem collapses to a **uniform-buffer swap** on the existing wgpu pipeline:

1. Store **two token records** per hub/vendor: `sheet_published` and `sheet_draft` (each 5
   tokens). This is a couple hundred bytes — cheap to hold both.
2. The wgpu field engine already reads Sheet tokens from a **uniform buffer**. Draft preview =
   bind the `sheet_draft` buffer instead of `sheet_published`. **Same pipeline, same shaders,
   same Sea layer** — only the uniform source changes. This is the token-set-swap pattern above,
   expressed in GPU terms; there is *no* second render path to build or keep in sync (which also
   sidesteps §16.30's a11y-mirror duplication cost — one pipeline means one a11y mirror).
3. **Preview is owner-only:** the draft buffer is bound only in the owner/admin surface (behind
   the owner capability-cert), exactly as Shopify gates preview behind authenticated admin
   access + short-TTL token. Customers' clients only ever fetch `sheet_published`.
4. **Publish = atomic copy** `sheet_draft → sheet_published` (one event in the event log; the
   Sea/content is untouched, mirroring Shopify's "data unchanged by theme switch"). Because it's
   one small record, publish is a single atomic write — no staged rollout needed, and a "revert
   brand" is just re-publishing the prior token record (keep the last published as history).
5. **Live-updating preview:** as the vendor drags a color/radius slider, write straight into the
   `sheet_draft` uniform each frame — the field engine re-renders with the physics already
   running, giving true "real field-engine" WYSIWYG (§16.58) with zero extra pipeline.

**Why this is the whole answer:** the token count is fixed and tiny (§16.9 deliberately capped it
at 5), so draft/live never needs a second renderer, a preview subdomain, or a staging deploy —
just a second small buffer and a bind-time flag. The heavy machinery real theme editors need
(preview URLs, isolated hosting) exists because their "theme" is arbitrary code; dowiz's Sheet is
5 numbers, so the same *semantics* (draft, preview, publish, revert) cost almost nothing.

---

## 5. Post-hoc content moderation — report/blocklist, no pre-review (§16.51)

### 5.1 The requirement
**Full vendor trust, no pre-publication review** for Wave-0 (§16.51). A post-hoc report/blocklist
mechanism is *implied as necessary but not designed* — and dowiz has **no vendor quality bar of
any kind** (§16.59), so moderation must be abuse-only, never a quality gate.

### 5.2 Prior art (low-moderation-overhead, decentralized)
- **Reactive (post-hoc) moderation** reviews only content users flag — cheap, but risks unflagged
  harm sitting live; the tradeoff is explicit in the moderation literature.
  ([GetStream content moderation](https://getstream.io/blog/content-moderation/))
- **Community-level blocklists** are *the* core strategy in decentralized/self-hosted systems
  (Mastodon-style): admins self-host, can block others entirely, and **optionally share their
  blocklists**, which others subscribe to. This is directly analogous to dowiz's isolated-hub
  mesh — each hub is sovereign, and a *shareable* blocklist is the natural cross-hub signal
  without a central authority.
  ([Understanding community-level blocklists (arXiv 2506.05522)](https://arxiv.org/html/2506.05522v1),
  [Jhaver et al., blocklists study (TOCHI'18)](http://eegilbert.org/papers/tochi18-jhaver-blocklists.pdf),
  [Hachyderm blocklists doc](https://community.hachyderm.io/docs/moderation/blocklists/))
- Keyword/allow-block filters are the standard mechanical layer under the human report flow.
  ([Moderation API allow/blocklist glossary](https://moderationapi.com/glossary/allowlist-blocklist))

### 5.3 Recommendation (Wave-0, minimal)
Given §16.59's *no quality bar*, keep this abuse-only and decentralized:
- **Report primitive:** a `report` event any customer can raise against a vendor/menu-item
  (reason enum + free text), stored in that hub's own event log. No central dowiz moderation
  queue (would violate §16.14 zero-central-state).
- **Blocklist as a signed, shareable artifact:** a hub/operator can maintain a blocklist
  (vendor-id / content-hash); publish it as an **optional subscribable list** other hubs *may*
  honor — never a mandatory central ban. This mirrors the Mastodon federated model and fits the
  mesh's "signed capability, never central reputation" stance (`MEMORY` /
  `sovereign-event-exchange` — trust = signed capability, **not** reputation/blacklist as an echo
  chamber). Distinguish carefully: an **abuse blocklist** (illegal/abusive content) is
  legitimate; a **quality/reputation blocklist** is exactly what §16.59 and the repo's
  no-scoring red-line forbid. Wave-0 should ship only the *abuse* report + *optional* shareable
  blocklist, and explicitly **not** a rating/ranking system.
- **dowiz-org role:** at most a legal-takedown endpoint for content dowiz is *legally* compelled
  to act on (heartbeat/liveness exception aside, §16.53) — not a proactive review.

This is deliberately thin; §16.51 itself defers the full design to a Tier-3/moderation blueprint.
The one durable design decision to lock now: **report + optional-subscribe blocklist, never a
central mandatory ban and never a quality score.**

---

## 6. AGPLv3 + Trademark + DCO packaging (§16.54 / §16.57 / §16.59)

### 6.1 The requirement
Hub software + client/courier Tauri apps = **AGPLv3 + trademark + DCO** (§16.57/§16.59).
`dowiz.org`'s own claim/tenant-isolation infra stays **closed** (§16.54). Model the packaging on
comparable real self-hostable AGPL projects.

### 6.2 Prior art — how Mastodon & Nextcloud actually structure it

**Mastodon** (self-hosted, AGPLv3):
- `LICENSE` = AGPLv3 at repo root. ([mastodon/LICENSE](https://github.com/mastodon/mastodon/blob/main/LICENSE))
- Separate, published **Trademark Policy**: the name/logos are trademarks of Mastodon gGmbH; the
  FOSS copyright license *does not* grant trademark rights; use the marks only to accurately
  identify software built on Mastodon; **do not** register them as part of your own
  trademark/domain/company/product name. ([Mastodon trademark](https://joinmastodon.org/trademark),
  [new trademark policy discussion](https://github.com/mastodon/mastodon/discussions/22785))
- `CONTRIBUTING` guide covering the dev process; the project deliberately uses **DCO, not a CLA**
  (like the Linux kernel). ([SovereignCloudStack DCO/licenses guide](https://github.com/SovereignCloudStack/contributor-guide/blob/main/source/dco-and-licenses.rst))

**Nextcloud** (self-hosted, AGPLv3):
- AGPLv3 (`COPYING-AGPL`), with the explicit framing that *"the GNU AGPLv3 is a copyright license
  and does not affect any trademarks."* ([Nextcloud news-android COPYING-AGPL](https://github.com/nextcloud/news-android/blob/master/COPYING-AGPL.md),
  [Nextcloud AGPL clarifications](https://help.nextcloud.com/t/clarifications-on-agpl-license/15025))
- Separate **trademark guidelines** (registered marks: "Nextcloud", the logo, "Nextcloud
  Enterprise/Files/Talk"…) aimed at encouraging community use while preventing confusion.
  ([Nextcloud trademarks](https://nextcloud.com/trademarks/))
- Contribution guidelines mandate **DCO** (as an additional safeguard), license headers,
  conventional commits. ([Nextcloud license-uncertainty blog](https://nextcloud.com/blog/how-nextcloud-protects-your-business-from-license-uncertainty/))

The consistent 4-part structure across both: **(1) `LICENSE`=AGPLv3 · (2) separate published
trademark policy · (3) `CONTRIBUTING` requiring DCO sign-off · (4) per-commit `Signed-off-by`
enforced in CI.**

### 6.3 What dowiz ALREADY has (verified in-repo, 2026-07-18)
dowiz's root **already matches this structure** — this is refinement, not a build:
- `LICENSE` (AGPLv3 full text, 33 KB), `NOTICE` (copyright + trademark notice + a recorded
  MIT carve-out for two non-derivative tool crates), `TRADEMARK.md`, `CONTRIBUTING.md`
  (mandates `git commit -s` DCO, states *"CI rejects commits without a valid Signed-off-by"*),
  `DCO` (the 1.1 text), `CODE_OF_CONDUCT.md`. Reference: `open-source-goal-adr020-2026-07-03`
  in memory (AGPLv3+TM+DCO, gated on secrets scrub + EUTM).

### 6.4 Recommendation — refine, don't rebuild; solve the open/closed split
The packaging files exist and are well-formed. The **real unsolved piece** is §16.54's
**open/closed boundary**: hub software (kernel/protocol/UI-render) is open AGPLv3; `dowiz.org`'s
claim mechanic + CF-tenant-isolation + landing site stay closed. Two concrete options:
- **(A) Split repos:** a public `dowiz-hub` repo (AGPLv3, everything a self-hoster installs) and a
  private `dowiz-infra` repo (claim/tenant-isolation, closed). Cleanest license boundary, matches
  Mastodon/Nextcloud's "the thing you self-host is the public repo." **Recommended.**
- **(B) Mono-repo with per-directory license:** keep one tree, mark `infra/` closed via a
  directory-scoped license note (extending the existing `NOTICE` per-crate carve-out pattern).
  Riskier — AGPL's network-copyleft + accidental linkage makes "closed subdir in an AGPL repo" a
  standing audit burden.

Given the repo's existing **`NOTICE` carve-out precedent** is for *non-derivative* MIT helpers,
and the claim/tenant-isolation infra is genuinely *not* something a self-hoster runs, **Option A
(split repos)** is the honest boundary. Concrete additions to model on the prior art: a published
**trademark policy page** on `dowiz.org` (Mastodon-style, beyond the in-repo `TRADEMARK.md`), and
DCO CI enforcement confirmed live on the **public** hub repo specifically.

---

## 7. Riskiest open unknowns for a Tier-3 blueprint

Ranked by blast radius:

1. **Migration/state rollback vs. code rollback (§16.27).** Auto-update rollback of *code* is
   easy (symlink flip); rollback of *forward-only schema migrations* is not. A bad update that
   migrated the event-log/pgrust schema forward, then gets rolled back, leaves new code's schema
   under old code. **Needs:** a hard rule that every auto-update snapshots state (§1 age blob)
   *before* promote, and that migrations are rollback-tolerant or the rollback restores the
   snapshot. This is the single most dangerous gap — it can brick a self-hosted hub's data.

2. **Backup key custody & the break-glass question (§16.27/§16.47).** If backups use a pure
   vendor-held X25519 identity (recommended), a vendor who loses the key loses every backup —
   "loss is the user's responsibility" (§16.47) is philosophically consistent but operationally
   brutal for a small vendor. **Needs:** an explicit ruling on whether a break-glass recipient
   exists, and if so where its private half lives (that key *is* the "dowiz can see plaintext"
   backdoor the invariant forbids). No safe middle ground is free.

3. **Split-payment settlement correctness & liability (§16.46/§16.49).** dowiz claims to "touch
   no money," but a food-court split it *computes* (even if the provider *moves* it) is a money
   action with real failure modes: partial-transfer failure, a vendor's connected account
   frozen, refunds against a multi-vendor order, tax split when each vendor sets its own rate
   (§16.49). **Needs:** the payment-adapter blueprint (already named as a gap in §16.13) to own
   split failure/refund semantics precisely — this is where "protocol not intermediary" is most
   likely to leak into de-facto financial liability.

4. **Free-form catalog schema vs. cross-vendor cart integrity (§16.17/§16.46).** A fully
   vendor-defined catalog tree (no fixed taxonomy) makes the *unified* cart harder: pricing,
   modifiers, and availability are vendor-authored with no shared contract, yet the cart and the
   settlement split must reason over all of them uniformly. **Needs:** a minimal invariant every
   catalog leaf must satisfy (a resolvable price + currency + vendor_id) even while the *tree*
   above it is free-form — the boundary between "free schema" and "enough structure to check
   out" is undesigned.

5. **wgpu draft-preview a11y parity (§16.30/§16.58).** The token-buffer-swap preview is cheap for
   *rendering*, but §16.30's a11y-mirror must also reflect the draft state in the owner preview
   — the accessible tree can't silently show published tokens while the canvas shows draft. Low
   blast radius on the money side, but a real correctness requirement the one-pipeline design
   must carry into the (undesigned) a11y mirror.

6. **Open/closed repo split enforcement (§16.54).** If Option A (split repos) is chosen, the
   *discipline* that keeps claim/tenant-isolation code out of the public hub repo is a standing
   leak risk (a convenience import of an infra helper into hub code silently AGPLs it or, worse,
   ships closed infra publicly). **Needs:** a CI guard on the public repo's dependency graph
   (analogous to the repo's existing kernel-fence guards) that fails if hub code imports
   `dowiz-infra`.

---

## Sources

**Encrypted backup:** [rclone crypt docs](https://rclone.org/crypt/) ·
[RcloneView zero-knowledge guide](https://rcloneview.com/support/blog/encrypt-cloud-backups-crypt-remote-guide-rcloneview) ·
[rclone S3 complete guide 2026](https://danubedata.ro/blog/rclone-s3-compatible-storage-complete-guide-2026) ·
[age crate docs](https://docs.rs/age/latest/age/) · [rage / str4d](https://github.com/str4d/rage) ·
[age on lib.rs](https://lib.rs/crates/age) · [age-crypto crate](https://crates.io/crates/age-crypto)

**Auto-update / rollback:** [self_update crate](https://github.com/jaemk/self_update) ·
[zero-downtime deploy strategies](https://www.deployhq.com/blog/zero-downtime-deployments-keeping-your-application-running-smoothly) ·
[single-EC2 blue-green symlink](https://saadh393.github.io/projects/blue-green-deployment-zero-downtime) ·
[automated rollback on health failure](https://oneuptime.com/blog/post/2026-02-09-automated-rollback-health-failures/view) ·
[hot config reload in Rust (arc-swap)](https://oneuptime.com/blog/post/2026-01-25-hot-configuration-reloading-rust/view)

**Food-court data model & split pay:** [Tabski food-court POS](https://tabski.com/food-court-pos/) ·
[GoTab food-hall](https://gotab.com/business-type/food-hall-pos) ·
[GoTab multi-vendor deep dive](https://gotab.com/latest/everything-food-hall-operators-need-to-know-about-gotabs-multi-vendor-pos) ·
[Chowbus multi-vendor POS](https://www.chowbus.com/blog/best-food-court-pos-system) ·
[Stripe separate charges & transfers](https://docs.stripe.com/connect/separate-charges-and-transfers) ·
[Stripe marketplace](https://docs.stripe.com/connect/marketplace)

**Brand draft/live preview:** [Shopify add/preview/publish themes](https://help.shopify.com/en/manual/online-store/themes/adding-themes) ·
[WPBeginner preview-before-live](https://www.wpbeginner.com/beginners-guide/how-to-preview-your-wordpress-website-before-going-live/) ·
[advanced theming with design tokens](https://david-supik.medium.com/advanced-theming-techniques-with-design-tokens-bd147fe7236e) ·
[Design Tokens Format Module](https://www.designtokens.org/tr/drafts/format/)

**Content moderation:** [community-level blocklists (arXiv 2506.05522)](https://arxiv.org/html/2506.05522v1) ·
[Jhaver et al. blocklists (TOCHI'18)](http://eegilbert.org/papers/tochi18-jhaver-blocklists.pdf) ·
[Hachyderm blocklists](https://community.hachyderm.io/docs/moderation/blocklists/) ·
[GetStream content moderation](https://getstream.io/blog/content-moderation/) ·
[Moderation API allow/blocklist](https://moderationapi.com/glossary/allowlist-blocklist)

**AGPL + TM + DCO packaging:** [mastodon/LICENSE](https://github.com/mastodon/mastodon/blob/main/LICENSE) ·
[Mastodon trademark](https://joinmastodon.org/trademark) ·
[Mastodon new-trademark-policy discussion](https://github.com/mastodon/mastodon/discussions/22785) ·
[Nextcloud trademarks](https://nextcloud.com/trademarks/) ·
[Nextcloud COPYING-AGPL](https://github.com/nextcloud/news-android/blob/master/COPYING-AGPL.md) ·
[Nextcloud license-uncertainty](https://nextcloud.com/blog/how-nextcloud-protects-your-business-from-license-uncertainty/) ·
[SovereignCloudStack DCO/licenses guide](https://github.com/SovereignCloudStack/contributor-guide/blob/main/source/dco-and-licenses.rst)
