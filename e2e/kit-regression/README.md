# kit-regression — the two gates that would have caught it

```sh
node e2e/kit-regression/run.mjs            # both
ONLY=render  node e2e/kit-regression/run.mjs
ONLY=domains node e2e/kit-regression/run.mjs
HOST=https://sushi-durres.dowiz.org node e2e/kit-regression/run.mjs
```

Exit code 0 when everything passes. No dependency beyond the `playwright`
library already in the tree — adding `@playwright/test` would be a new
supply-chain entry, and this repo requires a decart comparison for one.

## Why these exist

On 2026-09-16 a `style-src 'self'` header was added and every dowiz surface
rendered unstyled in production. Nothing caught it: the HTML was 200, every
stylesheet was 200, the menu API was 200, and a headless DOM render produced
28 KB of correct markup. **jsdom applies no content policy and has no layout
engine**, so the one tool in the loop was structurally incapable of seeing the
defect.

These two gates are what "how would we know" looks like.

## render.mjs — a real browser at 375×812

Chromium, the design's own viewport, an iPhone user agent. Per page it fails on:

| check | the defect it catches |
|---|---|
| CSP violations from **our** origin | the incident above |
| fewer than 50 CSS rules applied | a stylesheet that did not load or was blocked |
| a console error or page error | a screen that throws where jsdom did not |
| `scrollWidth > clientWidth` | anything that makes a phone scroll sideways |
| an inline `style` attribute inside `#kit-sprite` | icons regenerated from Figma without stripping them |
| a `<use href="#…">` pointing at nothing | an icon added to a screen but not to the sprite |

It has already found, in a real browser and nowhere else:

- the icon sprite carrying `style="display:none"` and `mask-type:alpha` as inline
  style attributes — three CSP violations per page, and every masked icon
  rendering wrong;
- `.k-top` used for **two different things** — the screen header and the topping
  tile — so `width:73px` from the tile was squeezing every header in the app;
- the dish plate (`position:absolute; inset:0`) placed directly in a card
  instead of a photo box, covering the name, price and rating: the card rendered
  as a bare gradient;
- an empty green discount chip on every dish with no discount.

### The one violation it does not fail on

Cloudflare injects its own bot-management script at the edge, into a page whose
policy forbids inline scripts. It is reported as `CSP(edge)` and not counted.
That is safe to distinguish because **our HTML ships no inline script at all** —
checked against the source, not assumed:

```sh
grep -c '<script>' workers/api/public/*/index.html      # 0
grep -rhoE '\son[a-z]+="' --include=*.html public/       # 0
```

### Retries

`RETRIES=2` by default. Cloudflare serves a challenge page under rapid load from
one address, and that page's policy blocks our module scripts; the same route
passes on its own every time. A retry that succeeds prints
`(passed on retry N — edge transient)` rather than hiding it.

## domains.mjs — the kernel's laws, over HTTP

25 contracts against a real hub. The Rust unit tests prove the law; these prove
the law is what production actually applies — every one of them asserts
something `dowiz_kernel` decides, never something the Worker re-implements.

Covered: the reservation FSM's refusals and idempotency, pass issue/verify/
tamper/window, wallet conservation and the top-up that is refused without a
payment rail behind it, thread ownership and party validation, and the delivery
estimate moving with distance, cooking time, portions and pickup.

It has already found, against production and nowhere else:

- a guest booking writing `""` into a column that references `users(id)`;
- `i64::into()` binding a JavaScript **BigInt**, which D1 rejects by throwing —
  a bare 500 with no body and nothing saying why;
- a resent message answering "already used by a different message", because the
  idempotency check ran *after* the kernel's and a resend carries a new
  timestamp.

It writes rows under an `e2e-<timestamp>` tag and prints the exact SQL to remove
them. Point it at a venue you are willing to have rows created in.

## What neither gate covers

- **Visual fidelity to the Figma frames.** These check that a page renders,
  applies its styles and fits the viewport; they do not compare pixels to the
  design. Screenshot diffing would, and is the obvious next addition.
- **The light theme.** Both run at the browser's default scheme. The kit ships
  both palettes and `settings` switches `data-theme`; a second pass with
  `colorScheme: 'dark'` would double the coverage cheaply.
- **Anything behind a session.** Neither `render.mjs` nor `domains.mjs` signs
  in; every route those two check is public. `cycle-full.mjs` below is the one
  that does.


## The signed-in gates

These four were written while chasing live defects and then kept, because each
found something no unit test could. They are NOT part of `run.mjs`: they place
real orders on a real venue and need credentials, so they are run deliberately.

```sh
HOST=https://sushi-durres.dowiz.org node e2e/kit-regression/cycle-full.mjs
```

| script | what it walks | what it found |
|---|---|---|
| `cycle-full.mjs` | a whole order through three browsers: storefront → console → courier → tracking sheet | `/api/live` answering 500; a courier signing in to the wrong venue |
| `socket-probe.mjs` | counts the `/api/live` connections a page really opens | that a venue whose sockets all fail looks exactly like a quiet venue |
| `landing-price.mjs` | the price is on the landing page, in each language | — |
| `lang-scrim.mjs` | the language sheet opens and dismisses | — |

`cycle-full.mjs` reads `/root/.dowiz_owner` for the owner, and takes
`QA_COURIER_PHONE` / `QA_COURIER_PASSWORD` for the courier. **The courier must
belong to the venue under test**, and the script checks that before it opens a
browser: a courier of another venue signs in, opens a shift, and then shows the
other venue's pool for ever, which reads as a broken product rather than as
wrong credentials.

### Why these are not named `_*.mjs`

`.gitignore` drops `e2e/kit-regression/_*.mjs` — the one-suspect probes written
while chasing a single defect, of which this directory holds seventy. A gate
that has graduated out of that is committed, and loses the underscore when it
does. Moving a `_`-named script into this directory WITHOUT renaming it does
not commit it: git records the move as a deletion and says nothing, which is
how three of these four were lost the first time and had to be recovered from
`01ed323^`.
