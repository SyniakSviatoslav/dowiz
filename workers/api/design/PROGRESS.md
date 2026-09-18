# dowiz kit — port of "Food Delivery Mobile App UI Design"

Figma file `0CR9FTDXuyuHE6c34EmOR9`. 87 screens, each drawn twice (a dark set at
nodes `1:884`–`1:10620` and a light set at `1:11101`–`1:20894`), plus two
"Color" frames that are the palettes: `1:10794` dark, `1:11079` light.

**All 87 frames are ported. 71 routes, all passing the gate against a live venue
and again with the API unreachable.**

## How this is built

**One module per FAMILY of frames, not per frame.** The file draws a tab change,
a wizard step and a mode switch as separate frames; here they are one screen
with the difference in the route (`#/restaurant-menu?tab=about`,
`#/onboarding?step=2`, `#/cart?mode=pickup`). 87 modules would be 87 copies of
the same layout, and they would drift on the first edit. 34 modules cover 87
frames.

**No inline style anywhere.** `style-src 'self'` in `public/_headers` blocks
`<style>` elements AND `style=` attributes — that is what made every dowiz
surface render unstyled on 2026-09-16. Computed values go through CSSOM
(`el.style.setProperty`), two-state values become two classes. The audit is four
greps and all four must read 0:

```sh
grep -rho '<style>'                  --include=*.html public/   # prose only
grep -rho 'style="'  --include=*.js  --include=*.html public/
grep -rho "createElement('style')"   --include=*.js   public/
grep -rhoE '\son[a-z]+="' --include=*.js --include=*.html public/
```

**Icons are Figma exports.** `public/kit/icon/*.svg` is exactly what Figma
emitted; `design/build-sprite.py` inlines them into `public/kit/icons.js` as one
sprite, rewriting only the neutral fills (`#8A8A8A`, `#B0B0B0`, `#E0E0E0`,
`#A8A8A8`, white, `#010101`, `#2A2A2A`) to `currentColor` so one glyph serves
both themes. Brand colours stay literal. **Re-run it after adding any asset.**

**The gate.** `kitverify.mjs` renders every route in a DOM and fails on: a JS
error, a class used with no rule, a `<use>` pointing at a missing symbol, or the
router falling through to "no such screen". It reports when a screen moves on by
itself (Splash does) instead of quietly measuring the next one. It has caught a
missing `.k-line-card` rule, a module left out of the staging copy, and an
undefined `.k-review-body`.

```sh
node kitverify.mjs "$ROUTES"                              # against a real venue
HOST=https://no-such-host.invalid node kitverify.mjs …    # offline fallback
```

## Reading the design

`design/file.json` is the whole document, pulled once through the REST API with
a personal access token (`/root/.figma_token`, mode 600). 22 MB, 234 top-level
frames, every node's geometry, fills, strokes, effects and typography.

- `python3 design/spec.py <node>` — one frame, nested, hex resolved to palette
  token names, device chrome pruned.
- `python3 design/spec.py <node> --text` — just the copy, in reading order.
- `python3 design/spec.py --list` / `--find <name>` — locate a frame.
- `python3 design/export-image.py 1:5701=qr-sample --scale=2` — render a node to
  `public/kit/img/`, for artwork a sprite cannot carry.

`design/` sits OUTSIDE `public/` on purpose: everything under `public/` is
served, and a 22 MB design dump has no business being fetchable.

**The token is a credential.** It was pasted into a chat transcript, so rotate it
in Figma → Settings → Personal access tokens.

## Live data

`kit/data.js` asks the same endpoint the storefront uses
(`/api/public/locations/<slug>/menu`) with the same host-to-slug rule. **The
frame's own content is the fallback, not a placeholder** — on a host that names
no venue, or when the request fails, every screen still renders the design it is
a port of. Wired: `home`, `restaurant-menu`, `item-details` (by `?id=`),
`popular-dishes`, `my-favourites`.

Money is the server's integer minor units through `Intl.NumberFormat` with the
venue's currency: a dish in Durrës prints `ALL 9.00`, not `$9.00`.

Where the kit asks for something dowiz does not have, the screen leaves it out
rather than inventing it — no per-dish rating, no per-dish distance, no venue
score, because trust in dowiz is a signed capability and never a score
(`DECISIONS.md`). "Vegetarian" is inferred only from an allergen list the venue
actually published.

## What is real, and what is still only drawn

Four of the six screens that used to stop at a backend now have one. The law for
each is in the kernel and the Worker decides nothing; `e2e/kit-regression/
domains.mjs` holds 25 contracts against a live hub, and all 25 pass.

| screen | what is behind it now |
|---|---|
| `book-a-table`, `my-booking`, `cancel-booking` | `dowiz_kernel::reservation` — a booking is fold(events), and an illegal transition is an error rather than a no-op |
| `chat`, `chat-detail` | `dowiz_kernel::thread` — ordering, idempotency and read marks; a message to a thread that does not exist is refused |
| `my-wallet`, `add-money` | `dowiz_kernel::ledger_account` — integer double entry, every transaction netting to zero. **The top-up still refuses:** no payment rail is wired, and money that appears without one is money from nowhere |
| `qr-entry` | `dowiz_kernel::pass` — minted and signed by the venue's hub, checked tag first and window second |

Still drawn only, and each says so plainly at the one action that would need a
backend rather than pretending:

| screen | what is missing |
|---|---|
| `voice-call` | no presence and no signalling anywhere in dowiz; the thread carries text |
| `restaurant-video` | dowiz has no video field, only photographs from the owner console |
| `rate-delivery` | submitting says the score is not kept against the courier — a CI job fails the build if `courier_score`/`rating`/`reputation` appears in the kernel |

## Routes

34 modules, 71 routes, 87 frames:

`home` · `item-details` · `restaurant-menu`/`-about`/`-gallery`/`-review` ·
`onboarding` · `splash` · `welcome` · `signin` · `create-account` ·
`new-password` · `verify-code` · `profile-complete` · `location` ·
`manual-location` · `notification-access` · `cart` · `remove-from-cart` ·
`order-type` · `delivery-type` · `delivery-address` · `manage-address` ·
`pickup-time` · `payment-methods` · `add-card` · `review-summary` ·
`booking-summary` · `payment-successful` · `reservation-confirmed` ·
`top-up-successful` · `e-receipt` · `track-order` · `track-live` ·
`get-direction` · `arrived` · `qr-entry` · `my-orders` · `search` · `filter` ·
`explore` · `category` · `popular-dishes` · `popular-restaurants` ·
`exclusive-offers` · `my-favourites` · `coupon` · `notification` · `gallery` ·
`restaurant-video` · `review` · `leave-review` · `rate-delivery` · `profile` ·
`your-profile` · `add-address` · `settings` · `password-manager` · `my-wallet` ·
`add-money` · `help-faq` · `help-contact` · `privacy-policy` ·
`invite-friends` · `logout` · `book-a-table` · `my-booking` · `cancel-booking` ·
`chat` · `chat-detail` · `voice-call`
