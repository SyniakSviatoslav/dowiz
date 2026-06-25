# Repo Dossier 3 — MERN + shadcn/ui Food-Ordering (frontend-component reuse target)

> Research date: 2026-06-22. Purpose: mine **frontend component patterns** (shadcn + Tailwind +
> CSS-var theming + react-query + react-hook-form/zod) for DeliveryOS `packages/ui`. The MERN
> backend (Mongo/Express) is a **divergence** from DeliveryOS (Fastify + Postgres + pg-boss) and is
> evaluated only as context, not as reuse.

**Selected repo:** `arnobt78/Restaurant-Food-Ordering-Management-System--React-MERN-FullStack`
URL: https://github.com/arnobt78/Restaurant-Food-Ordering-Management-System--React-MERN-FullStack

**Runner-up (rejected):** `rr3s1/Food-Ordering` — same Chris-Blakely-course lineage, MIT, but only 7
stars, 19 commits, less documented, TanStack/zod/RHF presence unconfirmed in metadata. The selected
repo is the more complete/maintained sibling (adds business-insights, city, auth API layers).

---

## R1 — Identity & License

| Field | Value | Source |
|-------|-------|--------|
| Slug | `arnobt78/Restaurant-Food-Ordering-Management-System--React-MERN-FullStack` | repo URL |
| Stars / Forks | 20 / 15 | GitHub API `stargazers_count`/`forks_count` |
| Created | 2025-08-26 | API `created_at` |
| Last push | 2026-02-21 | API `pushed_at` |
| Open issues / Archived | 0 / No | API |
| Commits | 51 (main) | repo header |
| **License** | **MIT** — `Copyright (c) 2026 Arnob Mahmud` | `/LICENSE` (verbatim opening lines) |

**License verdict: COPYABLE (with attribution).** Note: GitHub's API reports `license.spdx_id =
NOASSERTION` (a detection quirk — likely the long repo name / file placement), but the actual
`/LICENSE` file is standard, unmodified MIT text. The MIT copyright notice must be carried with any
lifted file. **Treat shadcn primitives as patterns-only anyway** — they originate from shadcn/ui
(MIT, Adam Wathan / shadcn) and DeliveryOS already vendors its own copies in `packages/ui`, so reuse
is "compare & align," not "copy file in."

## R2 — Stack & topology

**Frontend (the reuse surface):** React `18.2.0`, TypeScript `5.2`, Vite `7` (`@vitejs/plugin-react-swc`),
Tailwind `3.4`, shadcn/ui (Radix primitives), `react-query@3.39.3` (legacy TanStack v3 — **NOT**
`@tanstack/react-query` v5), `react-hook-form@7.49`, `@hookform/resolvers@3.3` + `zod@3.22`,
`react-router-dom@6.21`, `axios`, `sonner` (toasts), `next-themes` (dark mode), `lucide-react`,
`class-variance-authority` + `clsx` + `tailwind-merge` + `tailwindcss-animate`. **No framer-motion,
no socket.io, no Zustand, no MapLibre.** (source: `food-ordering-frontend/package.json`)

**Backend (DIVERGENCE — do not reuse):** Node + Express + TypeScript, MongoDB/Mongoose, Stripe,
Auth0 middleware, Cloudinary + Multer (image upload), express-validator.

**Topology vs DeliveryOS:**

| Concern | This repo | DeliveryOS |
|---------|-----------|------------|
| Backend | Express + MongoDB | Fastify + Postgres + pg-boss |
| Server state | `react-query@3` | TanStack Query (v5) + Zustand |
| Auth | Auth0 (hosted) | own JWT/RS256 |
| Theming | single static shadcn theme | **per-tenant `var(--brand-*)`** white-label |
| Real-time | react-query polling | own `ws` |
| Maps | none | MapLibre + OSM |
| Repo shape | 2 folders (FE/BE) | pnpm monorepo (`packages/ui`, `apps/*`) |

## R3 — Data model (Mongo, document — divergence)

Mongoose models under `backend/src/models`: `Restaurant` (embeds `menuItems[]`, `cuisines[]`,
`imageUrl`, city, delivery price as **subdocuments**), `User`, `Order` (embeds `cartItems[]` and
`deliveryDetails` inline). This is **document-embedded**: a restaurant owns its menu as a nested
array; an order snapshots cart + delivery as embedded objects.

**Divergence vs DeliveryOS relational:** DeliveryOS normalizes `organizations → locations →
menu/items` with FKs and RLS, single-restaurant-per-storefront (`/s/:slug`). The repo's
restaurant-as-aggregate is closer to NoSQL marketplace modeling. **No data-layer reuse** — the value
is purely the frontend shape of `menuItem { name, price }` and `cartItem` which loosely informs UI
prop types, nothing more.

## R4 — Order state machine vs DeliveryOS 10-state COD

Repo states (verbatim from README/OrderApi): **`placed → paid → inProgress → outForDelivery →
delivered`** (5 states), advanced by the restaurant owner; transition to `paid` is driven by a
**Stripe webhook**. No cancellation / refund / refused / failed branches modeled.

**Vs DeliveryOS:** 10-state **cash-on-delivery** machine (no Stripe `paid` gate; payment is on
delivery). The repo's machine is payment-first and linear; DeliveryOS is cash-first with
auto-cancel sweeps and richer terminal states. **No state-machine reuse.** Only the *UI* of an
order-status stepper/badge is loosely transferable (see R6/R8).

## R5 — Real-time

**Polling, not sockets.** `useGetMyOrders` uses react-query `refetchInterval: 5000` (5 s) to re-pull
order status; the "real-time tracking" claim is client polling. No socket.io / WS anywhere.

**Vs DeliveryOS:** own `ws` push channel. The repo offers **no real-time pattern worth adopting** —
DeliveryOS's WS is strictly better. (If anything, the repo is a cautionary "polling-as-realtime"
anti-pattern, R8.)

## R6 — Component system (DEEP — the main payoff)

**Theming via CSS vars — same mechanism as DeliveryOS, but single-theme.**
`global.css` defines shadcn HSL tokens on `:root` and `.dark`:
```
--background:0 0% 100%; --foreground:222.2 84% 4.9%;
--primary:222.2 47.4% 11.2%; --primary-foreground:210 40% 98%;
--secondary / --muted / --accent / --destructive / --border / --input / --ring; --radius:0.5rem
```
`tailwind.config.js`: `darkMode:["class"]`, colors map `primary:{DEFAULT:"hsl(var(--primary))",
foreground:"hsl(var(--primary-foreground))"}` etc. **`hsl(var(--token))` indirection — identical to
the DeliveryOS approach.** The decisive difference: this repo's tokens are **static, baked into CSS**
(one theme + dark variant). DeliveryOS needs the **same Tailwind→`hsl(var())` wiring but with tokens
swapped per tenant at runtime** (`var(--brand-*)`, derived palette). So the repo **validates the
wiring pattern** DeliveryOS uses, and shows exactly where to inject the per-tenant override (set the
`--brand-*` vars on a tenant root element; Tailwind classes resolve unchanged).

**`cn` helper — identical to DeliveryOS.** `lib/utils.ts`:
```ts
import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"
export function cn(...inputs: ClassValue[]) { return twMerge(clsx(inputs)) }
```
Standard shadcn; confirms `packages/ui` is already aligned. No change needed.

**Component structure:** `components/ui/*` are vanilla shadcn (Radix primitive + `cva` variants +
`cn`), e.g. dialog, dropdown-menu, select, tabs, checkbox, slider, progress, aspect-ratio, separator,
label — all present as `@radix-ui/*` deps. Dark mode via `next-themes` provider toggling the `.dark`
class. Toasts via `sonner`.

**Form pattern — react-hook-form + zod + shadcn `Form` (strong, adoptable):**
`forms/manage-restaurant-form/ManageRestaurantForm.tsx`:
```ts
const formSchema = z.object({
  restaurantName: z.string({ required_error: "..." }),
  cuisines: z.array(z.string()).nonempty({ message: "..." }),
  menuItems: z.array(z.object({ name: z.string().min(1), price: z.coerce.number() })),
}).refine((d) => d.imageUrl || d.imageFile, { message: "..." });
const form = useForm<RestaurantFormData>({ resolver: zodResolver(formSchema), defaultValues:{...} });
// <Form {...form}><form onSubmit={form.handleSubmit(onSubmit)}> ... </form></Form>
```
Notable: `z.coerce.number()` for numeric inputs, `.refine()` for cross-field rules, `defaultValues`
seeding array fields, and `onSubmit` translating validated object → `FormData` for multipart upload.
This is the canonical shadcn form recipe and maps cleanly onto DeliveryOS admin forms (menu editor,
profile).

**react-query usage** (`api/OrderApi.tsx`, `api/MyRestaurantApi.tsx`):
- query: `useQuery("fetchMyOrders", getMyOrdersRequest, { enabled: !!session, refetchInterval:5000 })`
- mutation: `const { mutateAsync, isLoading, error, reset } = useMutation(createCheckoutSessionRequest)`
- error UX: `if (error) { toast.error(...); reset(); }` (sonner)
- thin axios wrappers per resource; **string query keys** (v3 style).
**Caveat for DeliveryOS:** this is react-query **v3** API (string keys, `isLoading`, hook-options
shape). DeliveryOS is on **TanStack Query v5** (array keys, `isPending`, object-arg `useQuery({queryKey,
queryFn})`). The *pattern* (one hook file per resource, mutation+toast+reset) transfers; the *exact
API surface does not* — porting requires the v3→v5 rename.

**Relevance to `packages/ui`:** the shadcn primitives + `cn` + `cva` + `hsl(var())` token wiring are
already what DeliveryOS uses — this repo is a **confirmation/alignment reference**, not a source of
new components. The genuinely liftable idea is the **RHF+zod+shadcn `Form` recipe** and the
**one-file-per-resource query/mutation hook convention** (adapted to v5), neither of which lives in
`packages/ui` proper (they belong in `apps/web`).

## R7 — Checkout & payments

**Stripe-only, payment-first.** `useCreateCheckoutSession` → `POST
/api/order/checkout/create-checkout-session` → redirect to Stripe Checkout; a **Stripe webhook**
flips the order to `paid` and creates/advances the order. No cash path.

**Vs DeliveryOS COD:** DeliveryOS is **cash-on-delivery** — there is no pre-payment gate; the order is
created on placement and money is collected at handoff. **Checkout flow is a hard divergence — do not
reuse.** Only the *front-end cart→review→confirm UI scaffold* is loosely transferable; the
payment-session mechanics are irrelevant to COD.

## R8 — Patterns to adopt (frontend) + anti-patterns

**Adopt (frontend):**
1. **RHF + `zodResolver` + shadcn `Form`** with `z.coerce.number()`, `.refine()` cross-field rules,
   array-field `defaultValues` — canonical; align DeliveryOS admin forms to it. *(MIT; DeliveryOS
   module: `apps/web` forms, types shared via zod.)*
2. **One hook-file per resource** (`OrderApi`, `MyRestaurantApi`…) co-locating query+mutation+toast —
   clean separation; mirror in DeliveryOS data hooks. *(pattern-only.)*
3. **Mutation error UX**: `error → toast.error + reset()` via sonner. *(DeliveryOS already has toasts;
   adopt the `reset()` discipline.)*
4. **`hsl(var(--token))` Tailwind mapping** confirmed correct — keep; extend with `--brand-*`.

**Anti-patterns (avoid):**
- **Polling as "real-time"** (`refetchInterval:5000`) — DeliveryOS WS is correct; don't regress.
- **Static single-theme tokens in CSS** — DeliveryOS must keep tokens runtime-swappable per tenant.
- **react-query v3 string keys** — DeliveryOS is v5; don't copy the v3 surface verbatim.
- **Embedded Mongo aggregates / Stripe-first order machine** — backend, irrelevant.
- **Auth0 hosted auth** — DeliveryOS owns its RS256 JWT; do not introduce.

## R9 — Liftable vs rewrite (frontend focus)

| Item | Decision | License note | DeliveryOS module |
|------|----------|--------------|-------------------|
| `cn` util / `hsl(var())` Tailwind wiring | **already aligned** — verify-only | shadcn MIT | `packages/ui` |
| shadcn `components/ui/*` primitives | **patterns-only** (DeliveryOS vendors own) | shadcn MIT | `packages/ui` |
| RHF+zod+shadcn `Form` recipe | **ADAPT** (good template) | repo MIT (attrib.) | `apps/web` forms |
| Per-resource react-query hook file | **ADAPT** (port v3→v5) | repo MIT | `apps/web` data hooks |
| Cart→review→confirm UI scaffold | **ADAPT** (strip Stripe, wire COD) | repo MIT | `apps/web` checkout |
| Order-status stepper/badge UI | **ADAPT** (remap to 10-state COD) | repo MIT | `apps/web` / `packages/ui` |
| Mongo models / Express routes | **REWRITE / skip** (Fastify+PG) | n/a | n/a |
| Stripe checkout-session + webhook | **skip** (COD, no Stripe) | n/a | n/a |
| Auth0 middleware | **skip** (own JWT) | n/a | n/a |
| Polling "real-time" | **skip** (own WS) | n/a | n/a |

**UX honors:** form-validation & error-toast patterns → **HONOR**. Single-theme tokens →
**MAY-DEVIATE** (DeliveryOS goes per-tenant). Stripe checkout / Auth0 / polling / Mongo → **N/A**.

**Bottom line:** This repo is a **clean reference that confirms DeliveryOS's `packages/ui` foundations
are correct** (shadcn + `cn` + `hsl(var())` + CVA) and offers two genuinely adoptable *app-level*
recipes (RHF+zod `Form`, per-resource query hooks). It does **not** supply novel components and its
backend/realtime/payment layers are full divergences. Net reuse: low-volume but high-confidence
**frontend pattern alignment**, MIT-clean with attribution.
