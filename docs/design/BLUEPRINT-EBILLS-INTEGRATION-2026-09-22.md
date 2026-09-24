# ebills.al → dowiz: what the fiscal platform exposes, read-only, and what it would take to bring the venue's dine-in orders into one hub with one inventory

**Date:** 2026-09-22. **HEAD read:** `b949528e` ("dine-in: an order placed at a table", 2026-09-22).
**Account used:** the operator's own `ebills.al` login, from `/root/.ebills_account` (sourced; the password never
left the shell's environment and appears in no command line, file or log). **Tenant:** one business, one point of
sale (`id: 1`), 18 tables, Durrës — the venue dowiz knows as `dubin-durres` / `dubin-sushi.dowiz.org`.

**Method.** STRICTLY READ-ONLY. Every request was a `GET` except the login `POST` itself. Every "the platform
does X" below names the request that showed it and quotes what came back, with personal and fiscal identifiers
replaced by `<redacted>` or by their shape (`<str:36>` = a 36-character string). `measured` means the command was
run from this box on 2026-09-22 between 20:56Z and 21:15Z and its output is quoted; `hypothesis` means it was
not. Nothing was taken from the SPA's UI text when the wire was readable; the SPA's JavaScript (228 chunks,
9.0 MB, fetched and grepped) was used only to learn WHICH requests the UI makes, never as evidence of what the
server returns.

**One thing went wrong and is reported in full in §2:** a path that looked like a catalogue (`/api/close-shift`)
is, in this platform, a `GET` that *performs* the shift close. It was called once, it refused
(`{"errorKey":"unpaidSales","success":false}`), and §2 shows the before/after reads that establish nothing changed.
It is the single most important operational fact in this document: **on ebills.al, the HTTP verb does not tell
you whether a request writes.**

> **UPDATED 2026-09-23 — READ §8 FIRST.** The integration is wired (§8.1). A read-only run against the
> live platform on 2026-09-23 contradicted this document in five places; §8.2 lists each one with the
> evidence, and the code follows the evidence, not the text below. The sections below are kept as the
> 2026-09-22 record and are marked where §8.2 overrides them.

---

## 0. Verdict, in six lines

1. **There is a real JSON API** (JHipster / Spring Boot behind Azure Front Door), same-origin under
   `https://www.ebills.al/api/...`, 131 distinct paths in the SPA bundle. No scraping is needed. The OpenAPI
   document exists at `/v3/api-docs` but is `403 Access Denied` even to this `ROLE_ADMIN` account (measured).
2. **Auth is a server session**: form-urlencoded `POST /api/authentication` guarded by a CSRF cookie/header pair,
   answering with `JSESSIONID` (HttpOnly, session-scoped), `remember-me` (31 days) and an `X-TENANT-IDENTIFIER`
   response header that must be echoed as a request header on every call. No API key, no bearer token.
3. **Tables and open orders are readable**: `GET /api/sale-units-tables?pointOfSaleId=1` returns every table with
   `status: ACTIVE | OCCUPIED`, the unpaid `orderTotal`, `guests`, and the server's name. That is the live floor.
   The LINE ITEMS of an open table are behind `POST /api/sales/get-sale-unit-orders` — a read by meaning, a write
   by verb — and were **not** fetched (§5).
4. **A sale is two-level.** Every course ordered at a table is its own fiscalised sale (`summaryInvoice:false`,
   `saleUnitOrder:{status:"COMPLETED"}`) and the list endpoint HIDES those; closing the table issues a second
   fiscalised sale (`summaryInvoice:true`) whose lines are the union of the courses (verified on one table:
   260 + 4600 = 4860, 2 + 2 = 4 lines). Importing both as orders would double revenue and double stock draw.
5. **dowiz can hold this today** with four additions (§3.2): an external-identity block on the order (ebills
   `uuid`/`fic`/`iic`/`invOrdNum`), a catalogue crosswalk (ebills `itemCode` → dowiz `product_id`), a VAT line
   field, and a `Paid` event that names the fiscal bill. Nothing in the FSM needs to change.
6. **The pure half is written**: `workers/api/src/ebills/mod.rs` (+ `tests.rs`) parses the three payloads and
   maps a sale to the hub's order envelope, refusing floats that are not whole lek, quantities that are not
   whole, unfinished fiscal states and unknown payment methods. It is not wired (§7) and makes no network call.

---

## 1. The platform, as measured

### 1.1 What it is

`curl https://www.ebills.al/` with curl's default User-Agent → `HTTP 403`, an Azure Front Door block page
(`x-azure-ref` header, `x-cache: CONFIG_NOCACHE`). With a browser User-Agent → `200`, an Angular SPA titled
"eBills Cloud — All in One Cloud Based POS System" (`main-ORPPFB6U.js` → `chunk-IWIZNCDX.js`, 1,122,870 bytes,
plus 227 lazy chunks). Error bodies are RFC 7807 `application/problem+json` with `"type":
"https://fiskalizim.al/problem-with-message"` and `"message":"error.http.401"` — the JHipster shape.
`GET /management/health` → `{"status":"UP","groups":["liveness","readiness"]}` (Spring Boot Actuator, open).
Response headers carry `x-powered-by: ASP.NET` (the Azure App Service front) and cookies `ARRAffinity*` scoped to
`*.westeurope-01.azurewebsites.net` — a cookie a client for `www.ebills.al` will never send back, so session
stickiness is NOT something the client can rely on (**hypothesis**: the session store is shared, since every GET
after login succeeded without those cookies — measured 40+ calls, 0 failures).

**A poller must send a browser-like `User-Agent`**, or the WAF answers 403 before the application sees it
(measured, §1.1 first line).

### 1.2 Authentication, step by step (measured)

```
1. GET  /                                   → Set-Cookie: XSRF-TOKEN=<redacted>; path=/; secure
2. POST /api/authentication                 Content-Type: application/x-www-form-urlencoded
        X-XSRF-TOKEN: <value of the cookie>
        body: username=…&password=…&remember-me=true&submit=Login   (optionally &tenant=…)
   → 200, empty body, and:
        Set-Cookie: JSESSIONID=<redacted>; path=/; secure; HttpOnly; SameSite=Lax        (no Max-Age: session)
        Set-Cookie: XSRF-TOKEN=<redacted>  (rotated)
        Set-Cookie: remember-me=<redacted>; Max-Age=2678400 (31 days); HttpOnly
        x-tenant-identifier: <64 hex chars>
3. GET  /api/account   with Cookie + X-TENANT-IDENTIFIER: <same 64 hex>   → 200 (the user)
```

Without step 1's cookie the POST is `403 "Could not verify the provided CSRF token because no token was found
to compare."` (measured). The SPA stores the tenant header in `localStorage` and an interceptor adds it to every
request (`chunk-IWIZNCDX.js`, class `Gm`: `t.headers.set("X-TENANT-IDENTIFIER", r)`). The login component also
handles two branches this account did not take (read from `chunk-2U2CKYSY.js`, not exercised):
`X-TENANT-IDENTIFIER-NEEDED` (one username, several tenants → the client must resend with `tenant=`) and
`401 {mfaRequired, activeMethod, availableMethods}` → `POST /api/mfa/verify`. This account has
`"mfaEnabled": false, "mfaDefaultMethod": "TOTP"` (measured, in the `extraUser.user` block of
`GET /api/warehouses`). Out-of-shift logins answer with an `X-Shift-Error` header.

`GET /api/account` (measured, redacted):

```json
{"id":1,"login":"<str:19>","firstName":"<str:6>","lastName":"<str:5>","email":"<str:19>","activated":true,
 "langKey":"<str:2>","createdDate":"2026-05-04T14:51:26Z","authorities":["ROLE_ADMIN"],
 "extraUser":{"id":1,"validTo":null,"validFrom":null,"ratedAt":"2026-03-28","operatorCode":"<str:10>"}}
```

`operatorCode` is the fiscal operator code the tax authority knows this person by. Roles the bundle knows
(`chunk-DNTWM2S7.js`): `ROLE_ADMIN, ROLE_SALE, ROLE_BUYER, ROLE_REPORT, ROLE_MANAGEMENT, ROLE_MANAGEMENT_GENERAL,
ROLE_CONFIGURATION, ROLE_MODIFY_INVOICE, ROLE_POS_FULL, ROLE_POS_DELTA, ROLE_POS_BASIC, …, ROLE_USER`. A poller
should NOT run as `ROLE_ADMIN` (§6.9); the platform has narrower roles and a `POST /api/extra-users` to create
them — a write, so not tested, and to be done by the operator in the UI.

**Session lifetime:** the session cookie was still valid 13 minutes after login (measured: `GET /api/account` →
200 at 21:14:20Z, login at 21:01:13Z). The idle timeout was NOT measured (it needs waiting); the `remember-me`
cookie (31 days) re-authenticates a lapsed session on the next request (**hypothesis** from the cookie's name and
Spring Security's `remember-me` contract; not exercised). The bundle has no client-side idle timer constant
(grepped `idle|inactiv|timeout` with a 4+ digit literal: no hit). GETs need no CSRF header (measured: none was sent).

### 1.3 The API surface

131 distinct `"api/…"` strings in the bundle. The ones that matter to this goal, with the verb the SPA uses and
the answer this account got:

| Path | Verb | Answer (measured) | What it is |
|---|---|---|---|
| `/api/sales?…&currentPosId=1` | GET | 200, `{"sales":[…],"total":32850.0}` + `X-Total-Count`, `Link` (first/next/last) | the fiscal sales LIST — **hides table courses** (§1.6) |
| `/api/sales/{id}?currentPosId=1` | GET | 200, a wrapper `{"sale":{…},"saleModified":[…],"reason":null,"saleType":null,"withoutfiscalization":null,"includeClosingTheTable":false,…}` | one sale with its `saleRecords` (line items) |
| `/api/sale-units?…` | GET | 200, 18 rows | the table catalogue (`status`, `activeUser`, `posX/posY`) |
| `/api/sale-units-tables?pointOfSaleId=1` | GET | 200, 18 rows | **the live floor**: `status`, `orderTotal`, `guests`, `server` |
| `/api/sales/get-sale-unit-orders` | **POST** `{saleUnit, pointOfSale}` | not called | the open orders (lines) of one table |
| `/api/sales/sale-unit-orders-unpaidAmount` | **POST** | not called | unpaid total of a list of orders |
| `/api/sales/close-sale-unit`, `/close-sale-unit-order` | POST | not called | closes a table / one order — **never** |
| `/api/sales-cancel/{id}`, `/api/sales/refiscalize`, `/api/sales/removeItems`, `/api/sales/{id}` DELETE | PUT/POST/DELETE | not called | fiscal amendments — **never** |
| `/api/sales/summary-invoice` | **POST** | not called | the courses a summary bill covers (§1.6) |
| `/api/point-of-sales`, `/api/valid-pos` | GET | 200, 1 row | the POS: `id:1`, `posName`, `tcrOrderNo`, `bussinesUnit`, `registerTcr{…}`, `business{…}`, `warehouse{…}` |
| `/api/current-pos` | GET | 403 | not for this role |
| `/api/item-in-sales` | GET | 200, `X-Total-Count: 200` | the menu as ebills knows it (`itemCode`, `item`, `price`, `vat`, `unit`, `itemCategory`, `isService`) |
| `/api/item-categories` | GET | 200, 8 rows | `kafeteria`, `freskuese`, … |
| `/api/sale-records` | GET | 200, `X-Total-Count: 18661` | every line of every sale, flat; **no sale filter** (`?saleId=`, `?sale=`, `?sale.id=` all ignored, measured) |
| `/api/sale-registers` | GET | 200, 39 rows (today) | a flat report row per line: `nr, invOrdNum, price, amount, totalValue, vat, category, itemName, invoiceDate, fic, warehouse, user, client` |
| `/api/saled-items` | GET | 200 | per-item totals `{code,name,amount,valueWithoutVAT,valueVAT,valueWithVat,price}` |
| `/api/tcr-cash-balances` | GET | 200, 1 row today | the cash drawer: `cashBalanceType: INITIALIZE|WITHDRAW|DEPOSIT`, `suggestedBalance` |
| `/api/payment-method-configurations` | GET | 200, 6 rows, all `active:false` | BKT_BANK, EREJA, PAYSERA… none enabled |
| `/api/invoice-types` | GET | 200, `X-Total-Count: 727` | the UN/CEFACT invoice-type code list; every sale here is `invoiceTypeId: 388` (tax invoice) |
| `/api/deliveries`, `/api/sections`, `/api/sector-pos`, `/api/transporters`, `/api/sale-einvoice-infos`, `/api/item-in-warehouses`, `/api/service-items` | GET | 200, `[]` | unused by this venue |
| `/api/sale-pays` | GET | 200, **6,544,131 bytes, 5,854 rows, `size=2` ignored** | every payment ever, each embedding its whole sale — never poll this |
| `/api/items` | GET | 403 | |
| `/api/close-shift` | **GET** | `{"errorKey":"unpaidSales","success":false,"sales":null}` | **closes the shift** — see §2 |
| `/v3/api-docs`, `/v3/api-docs/swagger-config` | GET | 403 `Access Denied` (401 unauthenticated) | OpenAPI exists, denied |
| `/swagger-ui/index.html` | GET | 200 (the static page; its initializer is `500 No static resource`) | |

Pagination is JHipster's: `page` is 0-based on the wire (the SPA's URL shows `page=1` and sends `page-1`,
`chunk-YQFNNNTR.js`, `loadPage`), `size`, repeated `sort=field,dir`, `X-Total-Count` and a `Link` header.
`size=500` was honoured on `/api/sales` (452 rows, 1,529,447 bytes) — and ignored on `/api/sale-pays`.

### 1.4 The query model — what the operator's URL means

The SPA route `/sale?page=1&size=20&sort=id,asc&beginDate=…&endDate=…&client=-1&pointOfSale=-1&extraUser=-1
&sale=-1&invOrdNum=-1&isSelfIssue=false&isReverseCharge=false&isExport=false` is translated by `loadPage` into
`GET /api/sales` with the same names plus two the URL does not show, and the API refuses without one of them:

| Parameter | Meaning (from the SPA's form binding) | Measured |
|---|---|---|
| `beginDate`, `endDate` | `YYYY-MM-DD`, inclusive; default = today | one day → 24 rows; 30 days → 452; since 2026-05-01 → 3,025 |
| `client` | client id, `-1` = any | this venue has one client, the `defaultClient` (`id:1`, walk-in) |
| `pointOfSale` | POS id, `-1` = any | one POS |
| `extraUser` | operator id, `-1` = any | one operator |
| `sale` | **a sale id, exact** (the form's `invoice` picker) | `sale=8601` → exactly `[8601]` |
| `invOrdNum` | the fiscal ordinal, exact | `invOrdNum=8598` → `[]` although sale 8602 has that ordinal — the list filter hides it (§1.6) |
| `isSelfIssue`, `isReverseCharge`, `isExport` | booleans | all `false` on every sale here |
| `status`, `paymentMethod` | optional filters the form has and the URL omits (`null` = any) | the vocabulary is server-side; only `CLOSED` / `CASH` were ever observed |
| **`currentPosId`** | **required**; the SPA takes it from `localStorage["pos"]` | omitted → `400 "Required request parameter 'currentPosId' for method parameter type Long is not present"` |

**Time zone.** Timestamps are ISO-8601 UTC (`"2026-09-21T09:01:19.404634Z"`). The day filter for 2026-09-21
returned sales from 09:01Z to 21:01Z (11:01–23:01 Tirana). Whether `beginDate`/`endDate` cut at UTC midnight or
at Tirana midnight was NOT determined (no sale straddled it); a poller must overlap its windows by one day
(§4) rather than assume. Cf. memory "Venue timezone was a summer constant".

### 1.5 A SALE, field by field (measured; `GET /api/sales/8602?currentPosId=1`, the wrapper's `sale`)

A course ordered at table 12, one line, paid cash, fiscalised:

```json
{"id":8602,"invOrdNum":8598,"uuid":"<str:36>","fic":"<str:36>","timestamp":"2026-09-21T09:01:25.857Z",
 "status":"CLOSED","fiscalSatus":"FINISHED","draft":0,"changedStatus":null,"modified":null,
 "summaryInvoice":false,"exchange":false,"buying":false,"selfIssue":"SELFISSUE_0",
 "isReverseCharge":false,"isExport":false,"invoiceTypeId":388,"invoiceType":null,
 "paymentMethod":"CASH","salePaymentMethods":[],"salePays":[],"saleBanks":[],"vouchers":[],"fees":[],
 "totalValue":500.0,"totalVatAmount":83.3333333333,"currencyRate":1.0,
 "currency":{"id":1,"currency":"Albania Lek","currencyCode":"ALL","sellRate":1.0,"buyRate":1.0,"isBaseCurrency":false},
 "notes":null,"contract":null,"projectReference":null,"importedInvoiceNumber":null,
 "periodStartFisc":null,"periodEndFisc":null,"document":null,"documentContentType":null,"filename":null,
 "delivery":null,"clientExtraAddress":null,
 "client":{"id":1,"name":"<str:14>","defaultClient":true,"status":"ACTIVE","nipt":"","identType":"ID_NUMBER",
           "address":"","city":"","country":"ALB","contact":null,"email":null},
 "extraUser":{"id":1,"operatorCode":"<str:10>","operatorID":"","simpleUser":false},
 "pointOfSale":{"id":1,"posName":"<str:4>","tcrOrderNo":1,"bussinesUnit":"<str:10>","license":"<str:23>",
                "printing":"THERMAL","printerSize":"SIZE_80","address":"<str:70>","city":"Durres","isActive":true},
 "saleUnit":{"id":12,"identifier":"12","type":"TABLE","status":"ACTIVE","posX":5.0,"posY":3.0,
             "note":null,"def":null,"size":null,"hasGroup":null,"saleUnitCategory":null,"saleUnitReservations":null},
 "saleUnitOrder":{"id":5614,"status":"COMPLETED"},
 "saleRecords":[
   {"id":18490,"saleNo":1,"itemName":"<str:5>","amount":2.0,"price":250.0,"totalValue":500.0,
    "vat":"VAT_20","totalVatAmount":83.3333333333,"discount":0.0,"discountReduceBase":false,
    "refundType":null,"expireDate":null,"notes":null,"serialNumber":null,
    "itemInSale":{"id":74,"itemCode":"56","item":"<str:5>","price":250.0,"promoPrice":null,"vat":"VAT_20",
                  "isService":true,"isMixProduct":false,"statusi":"ACTIVE","barcode":null,
                  "itemCategory":{"id":1,"category":"kafeteria","status":"ACTIVE"},
                  "unit":{"id":1839,"unit":"Copë","unitCode":"XPP","isActive":true},
                  "itemRemboursement":{"id":74,"partialRemboursement":0.0,"fullRemboursement":0.0},
                  "linkedItemInWarehouseId":null}}],
 "logCis":[{"id":8602,"iic":"<str:32>","iicReference":"<str:32>","fic":"<str:36>","status":"SUCCESS",
            "sentTime":"2026-09-21T09:01:25Z","invoiceGeneratedAt":"2026-09-21T09:01:25Z",
            "faultString":null,"faultStringMsg":null,"eic":null,"faultStringEic":null,
            "sale":{ "…the same sale again, recursively, one level…" }}],
 "qrCode":"https://efiskalizimi-app.tatime.gov.al/invoice-check/#/verify?iic=<redacted>&tin=<redacted>&crtd=…&ord=8598&bu=<redacted>&cr=<redacted>&sw=<redacted>&prc=500.0"}
```

What each block IS:

- **Identity.** `id` (row id), `invOrdNum` (the fiscal ordinal printed on the receipt; increments
  `BY_POINT_OF_SALE`), `uuid` (the client-generated invoice UUID), `fic` (Fiscal Invoice Code — the tax
  authority's receipt for this invoice), `logCis[0].iic` (Issuer Invoice Code — the hash the issuer computed
  before sending). `fic` is present on all 452 sales of the last 30 days (measured: 0 missing). **The
  idempotency key is `uuid`**; `iic`/`fic` are the proof it was fiscalised.
- **State.** `status: "CLOSED"`, `fiscalSatus: "FINISHED"`, `draft: 0`, `logCis[].status: "SUCCESS"` on all
  452/452 (measured). The sale-status and fiscal-status vocabularies are server-side (not in the bundle); what
  the bundle does define is the PURCHASE status `OPENED|CLOSED|CANCELLED` (`chunk-XFJ6XVAE.js`), the sale-unit
  status `ACTIVE|INACTIVE|AVAILABLE|OCCUPIED|RESERVED|DELETED` and type `TABLE|ROOM|OTHER`
  (`chunk-3KWBGLKO.js`), VAT `VAT_0|VAT_6|VAT_10|VAT_20` (`chunk-AG7FWIUT.js`), self-issue
  `SELFISSUE_0..5`, ident type `NONE|NIPT|ID_NUMBER|PASSPORT`, cash-balance `INITIALIZE|WITHDRAW|DEPOSIT`, and
  payment methods `CASH, BANK, BKT_BANK, CARD, CARD_ON_POS, POK_CARD, PAYSERA, EREJA, OTHER, VOUCHER, WAIVER,
  COMPENSATION, CHECK, KIND, MULTIPLE` (`chunk-GX4POSSC.js`, enum `p6`). `changedStatus` and `modified` were
  `null` on all 452 — what a cancelled or amended sale looks like on the wire was NOT observed (§6.6).
- **Money — floats.** `totalValue: 500.0`, `price: 250.0`, `amount: 2.0` are JSON doubles; VAT amounts are
  repeating decimals (`83.3333333333`). Lek has no minor unit and dowiz stores ALL amounts as whole lek
  (`workers/api/public/kit/data.js:60`, `notify.rs::lek_has_no_minor_unit_and_euro_has_two`), so the mapping is
  `f64 → i64` **only when the value is whole**, else a refusal (§7). `totalVatAmount` is derived and is not
  mapped; the line's `vat` rate is.
- **Where.** `saleUnit` = the table (`identifier` is the number painted on it; `posX/posY` a floor grid),
  `pointOfSale`, `extraUser` = who rang it up. `delivery` was `null` on all 452: **this venue takes no
  deliveries through ebills**; deliveries are dowiz's.
- **Who.** `client` is the walk-in default on all 452. A named client would carry `name`, `nipt`, `address`
  — personal data that dowiz's `Revealed` audit exists for and that a poller must not copy into the order log.

### 1.6 The most important question: tables and open orders — YES, with one door closed

**Tables exist as first-class records.** `GET /api/sale-units-tables?pointOfSaleId=1` (measured, 21:0xZ, the
venue's last hour):

```json
[{"id":17,"identifier":"17","type":"TABLE","status":"OCCUPIED","pointOfSaleId":1,"posX":2.0,"posY":3.0,
  "guests":"0","orderTotal":5600.0,"server":"<redacted, a person's name>","activeUser":{"id":1,"user":{…}},
  "size":null,"def":null,"note":null,"saleUnitCategoryId":null},
 {"id":18,"identifier":"18","status":"OCCUPIED","orderTotal":160.0, …},
 {"id":1,"identifier":"1","status":"ACTIVE","orderTotal":null,"activeUser":null, …}, … 18 rows]
```

`status: OCCUPIED` + `orderTotal` IS the open, unpaid state of a table, at table granularity. The same 18 rows
from `GET /api/sale-units` carry `status` and `activeUser` but not `orderTotal`. Eleven minutes later the same
call returned 18 × `ACTIVE` with `orderTotal: null` after the staff closed both tables (§2) — so the field is
live, not a cached report.

**The open orders' LINES are behind a POST.** The SPA's "close table" flow (`chunk-3KWBGLKO.js`, `closeTable`)
calls `saleService.getOpenOrdersForSaleUnit(saleUnit, pointOfSale)` = `POST /api/sales/get-sale-unit-orders`
with body `{saleUnit, pointOfSale}`, then `POST /api/sales/sale-unit-orders-unpaidAmount`. By their names and
their use in the UI they are reads; by verb they are writes, and after §2 this document does not assume a
name is honest. **They were not called.** Whether they are idempotent reads is the first question for the
operator to answer, or for a lane with an explicit write permission to test on a table that is not in service.

**A table's life on the wire, reconstructed from reads only** (table 12, 2026-09-22, measured):

| Time (UTC) | Sale id | `summaryInvoice` | `saleUnitOrder` | `totalValue` | lines | in the list? |
|---|---|---|---|---|---|---|
| 19:23:42 | 8689 | false | `{5xxx, COMPLETED}` | 260 | 2 | **no** |
| 19:23:56 | 8690 | false | `{…, COMPLETED}` | 4600 | 2 | **no** |
| 20:52:33 | 8691 | **true** | null | **4860** | **4** | yes |

Each course is fiscalised the moment it is rung up (its own `fic`, `invOrdNum`, `logCis SUCCESS`); the bill at
the end is a *second* fiscalised sale — a summary invoice, which Albanian fiscalisation treats as referencing
the earlier ones (the `iicReference` field; the tax authority does not double count). The list endpoint returns
only the bills: 452 of the 1,456 ids in the 30-day id span; the hidden ones I opened were all courses
(`summaryInvoice:false` + `saleUnitOrder`), and one listed row was a `summaryInvoice:false` sale with NO
`saleUnitOrder` — a counter sale. The rule the server applies is therefore **hypothesis: "hide sales that
belong to a table order"**, consistent with every observation.

**The link from a bill to its courses is not in any GET.** The bill's `saleUnitOrder` is `null`; its
`saleRecords` are a copy of the courses' lines; `POST /api/sales/summary-invoice` (`getInvoicesOfSummaryInvoice`)
is what the UI uses. From reads alone, the join is *same table, courses since that table was last billed* —
which is exactly what §4's poller records, because it sees the table go `OCCUPIED` and back.

### 1.7 What else this account can read, and should not have to (measured)

- `GET /api/businesses` and every `pointOfSale.business` block return `"certificate": "<str:4136>"`,
  `"keystorePass": "<str:8>"`, `"keystoreAlias": "<str:1>"` — **the fiscal signing certificate and its
  password**, to any `ROLE_ADMIN` API client, inside routine list responses. A poller that logs response bodies
  would log them. §6.9.
- `GET /api/sale-pays` returns 6.5 MB and ignores paging.
- `GET /api/close-shift` closes the shift (§2).
- The `qrCode` URL embeds the business's TIN; `server` on the floor map is a person's name; `client` may be.

---

## 2. Incident: `GET /api/close-shift`, called once, refused, verified inert

**What happened.** While reading every catalogue path the bundle names, `/api/close-shift` was fetched with
`GET …?page=0&size=2&currentPosId=1&posId=1` at **21:06:34Z**. The bundle later showed
(`chunk-IWIZNCDX.js`, class `Wr`): `close(){return this.http.get(this.resourceUrl,{observe:"response"})}` —
the UI's "close shift" button IS this GET. The response:

```json
{"errorKey":"unpaidSales","success":false,"sales":null,"extraUser":{…}}
```

**Why it refused.** Two tables were `OCCUPIED` with unpaid orders (`orderTotal` 5600 and 160, measured at
21:0xZ before the call). The shift cannot close over unpaid sales; the server said so and returned no sales.

**What was checked afterwards (all reads):**

- `GET /api/tcr-cash-balances?sort=id,desc` at 21:09Z → still one record for the day, `id:140,
  cashBalanceType:"INITIALIZE", timestamp:"2026-09-22T09:19:59Z"`. A closed shift would have written a
  balance record; none exists.
- `GET /api/sales?…beginDate=endDate=2026-09-22` → 14 rows, newest `id:8693` at 21:07:29Z, `CLOSED`, cash.
- `GET /api/sales/8692` and `/8693`: two **summary** bills at 21:07:26Z and 21:07:29Z, for **table 18 = 160**
  and **table 17 = 5600** — exactly the two unpaid totals — 52 and 55 seconds after the refused call, with
  `saleRecords` of 3 and 1 lines, `paymentMethod: CASH`, `fic` present, `logCis SUCCESS`.
- The floor map then showed 18 × `ACTIVE`.

**Reading.** A refused shift close returns `sales: null` and creates nothing; it cannot issue a summary
invoice, take a cash payment or fiscalise a bill — those are `POST /api/sales/close-sale-unit` with a payment
method, which was never sent. The two bills are the staff closing the last two tables at 23:07 local, the
venue's usual closing minute (the previous day's last sale was 21:01Z = 23:01 local). I cannot prove the
staff did not see a message on the terminal at 21:06Z; I can prove the call wrote nothing and that the bills
carry a human's payment. The operator should be told, in these words, so they can ask the staff.

**The lesson this document exists to carry:** on this platform `GET` is not a promise of a read. `close-shift`
is a GET that writes; `get-sale-unit-orders` is a POST that (by its name) reads. A poller may call ONLY the
paths in §4's allow-list, and the allow-list is by path, not by verb.

---

## 3. ebills ↔ dowiz, field by field

### 3.1 The mapping

dowiz's order is the kernel's `Order` (`crates/dowiz-core/src/domain.rs:104`: `id, customer_id, status, items[
product_id, modifier_ids, quantity, unit_price], subtotal, total, created_at_ms, channel, cash_pay_with,
price_trusted, ledger, fulfilment, contact, scheduled_for_ms, courier_id`) inside the envelope
`storefront::place` writes (`workers/api/src/storefront.rs:1000-1050`: `delivery_fee, tip, total, location_id,
contact{name,phone}, fulfilment{kind,note,table,address,fee}, payment, crypto?`), appended to the hub log as
`EventKind::Placed` and moved by `Advanced` / `Paid` / `Noted` (`crates/dowiz-hub/src/lib.rs:63-101`).

| ebills (a COURSE sale, `summaryInvoice:false`) | dowiz | Note |
|---|---|---|
| `uuid` | `id` = `"ebills:" + uuid` | the idempotency key; a second arrival is the same `id`. **The hub does not refuse a second `Placed` for a known id** (`Hub::holds`, `lib.rs:711`, is about record content ids; `history(order_id)` would simply show two) — the guard is the poller's, and it must run INSIDE the object as a `command` so that "is it held? then append" is one turn (§6.1) |
| `id`, `invOrdNum`, `fic`, `logCis[0].iic`, `pointOfSale.id` | **missing** → `external{source:"ebills", sale_id, inv_ord_num, fic, iic, pos_id}` | §3.2 (1) |
| `timestamp` (UTC ISO) | `created_at_ms` | parsed without a clock; the poller's own time never enters the record |
| `status:CLOSED` ∧ `fiscalSatus:FINISHED` ∧ `draft:0` ∧ `logCis[].status:SUCCESS` (**§8.2 (1): also `OPENED` for a course at an open table**) | `status: "PICKED_UP"` | the only terminal that `took_money()` and has no courier leg (`order_machine.rs:14-31,96-103`); a course is placed, made and served before ebills ever shows it — dowiz sees history, not a live ticket (§6.3). Anything else → **refused**, not `PENDING` |
| `saleUnit.identifier` | `fulfilment.kind:"dine_in"`, `fulfilment.table` | `fulfilment::needs("dine_in") == Needs::Table` (`services/ordering/fulfilment.rs:59`) |
| `saleUnitOrder.id` | `external.sale_unit_order_id` | ~~groups the courses of one sitting~~ **not shown to group a sitting (§8.2 (2))** |
| `paymentMethod` | `payment` | `CASH→"cash"`, `CARD|CARD_ON_POS|POK_CARD→"card"`; the other twelve → **refused** (`PAYMENT_KINDS` = `cash, card, apple_pay, google_pay, crypto`, `storefront.rs:649`) |
| `totalValue` (f64) | `total`, `subtotal` | whole lek or refused; `delivery_fee:0`, `tip:0` |
| `currency.currencyCode` | the location's `currency` | must be `ALL` or refused; `currencyRate` must be `1.0` |
| `saleRecords[].itemInSale.itemCode` | `items[].product_id` = `"ebills:" + itemCode` until a crosswalk exists | §3.2 (2) |
| `saleRecords[].itemName` | `items[].name` | the snapshot dowiz already keeps (`storefront.rs`, "THE DISH'S NAME TRAVELS WITH THE LINE") |
| `saleRecords[].amount` (f64) | `items[].quantity` | whole or refused (ebills sells by weight elsewhere; this venue does not) |
| `saleRecords[].price` (f64) | `items[].unit_price` | whole lek or refused; `price * amount − discount == totalValue` checked per line, `Σ == totalValue` checked per sale |
| `saleRecords[].vat` (`VAT_20`) | **missing** → `items[].vat_rate_pct: 20` | §3.2 (3) |
| `saleRecords[].discount`, `discountReduceBase` | `items[].discount_pct` — **a PERCENT, already inside `price` (§8.2 (3))** | §3.2 (3) |
| `modifier_ids` | `[]` | ebills has no modifiers; `isMixProduct` is a different thing (a bundle) |
| `client` (walk-in) | `contact{name:"",phone:""}`, `customer_id:null` | never copy a named client's `name/nipt/address` (§6.8) |
| `extraUser.id` | **missing** → `external.operator_id` (the id, never the code) | who rang it up |
| `channel` | `"ebills"` | the kernel's existing free-text field |
| `price_trusted` | `false` | the prices did not come from dowiz's catalogue; the kernel's rule: downstream must not charge an untrusted order — correct, nothing is charged |
| `ledger` | `[]` | money moved outside dowiz; §3.2 (4) |

| ebills (a BILL, `summaryInvoice:true`) | dowiz | Note |
|---|---|---|
| the bill | **not an order** | its lines duplicate the courses'; importing it as an order doubles revenue and stock draw |
| `uuid, fic, iic, invOrdNum, totalValue, paymentMethod, timestamp` | a `Paid` event on each course of that sitting, payload naming the bill | §3.2 (4); the join is by table + window (§1.6) |

| dowiz has, ebills cannot express | |
|---|---|
| `PENDING → CONFIRMED → PREPARING → READY` | ebills has no kitchen states; a course is `COMPLETED` when rung up |
| `Rejected`, `Refunding`, `CompensatedRefund` | ebills has `sales-cancel` with a reason and `refundType` on a line; the wire shape of a cancelled sale was not observed |
| delivery, courier, address, ETA, `scheduled_for_ms` | ebills `deliveries` is empty for this venue |
| modifiers / option groups | none |
| `contact.phone`, the `Revealed` audit | ebills has `client.contact` on named clients only |
| stock reservation / the automated 86 (`stock.rs`) | ebills has warehouses and `linkedItemInWarehouseId` (all `null` here); `showInventory:false` on the business |
| crypto payment, wallets | none |

### 3.2 What dowiz is missing to hold an ebills sale faithfully

1. **An external-identity block on the order envelope** — `external{source, sale_id, inv_ord_num, uuid, fic,
   iic, pos_id, sale_unit_order_id, operator_id}`. Today the envelope has nowhere to keep a foreign id except
   `channel` (a word) and `id` (which this design overloads with `ebills:<uuid>`). The fiscal codes matter: an
   owner reconciling the drawer against dowiz needs `invOrdNum`, and a tax query needs `fic`.
2. **A catalogue crosswalk** ebills `itemCode` ↔ dowiz `product_id`. ebills knows 200 items by `itemCode`
   (`"1"`, `"3"`, `"56"`…) and name; dowiz's catalogue for this venue has its own ids (165 dishes, memory
   "Sushi Durrës menu source"). Without the crosswalk the order can be stored (`product_id:"ebills:56"`) but
   **cannot draw stock**, because `stock::bom_of(product_json)` needs the dowiz product. The crosswalk is a
   small owner-editable table in the hub image (`table.rs`) keyed by `itemCode`; unmatched codes are a
   visible owner task, not a silent drop.
3. **Per-line VAT rate and discount.** `OrderItem` has `unit_price` only. Albanian receipts are VAT-inclusive
   and the rate per line (`VAT_20`, `VAT_6`…) is a fact the owner's export must carry; `discount` and
   `discountReduceBase` likewise. Two optional fields on the line JSON, ignored by the kernel's money law (which
   sums `unit_price × quantity`).
4. **A `Paid` event that names the fiscal bill**, and the rule "an `ebills` order is born `PICKED_UP` with an
   empty ledger". `EventKind::Paid` exists (`lib.rs:74-79`, "an order can be paid while still PENDING"); its
   payload today is the order JSON. The bill's `uuid/fic/invOrdNum/paymentMethod` belong in that payload so the
   sitting can be reconciled.
5. **A table-state surface that is not a booking.** `tables.rs` computes occupancy from bookings for a minute
   asked for ("`occ` IS NOT A PROPERTY OF A TABLE"). ebills says a table is `OCCUPIED` *now* with `orderTotal`
   unpaid. That is a different fact — live floor state — and the console should show it beside the booking
   view without pretending it is one. Fifty bytes per table, rewritten each poll, in a `floor` image; not in
   the order log.
6. **Stock, shared.** The goal is one inventory. With (2), a course can raise `StockEvent::Consumed` (not
   `Reserved`: it was already served) per BOM line — `stock::settle(ledger, order_id, consume=true)` exists for
   this. What is NOT there: a rule for a course that arrives after a dowiz delivery already reserved the last
   portion. §6.4.

### 3.3 What the FSM does not need

Nothing. A served course is `PICKED_UP`; a cancellation seen later (if the wire ever shows one) is
`Refunding → CompensatedRefund` on an order whose ledger is empty, which nets to zero trivially. No new status,
no new edge, the golden signature stands.

---

## 4. The poll: cadence, allow-list, cost

**Allow-list — the only paths a poller may ever request, all `GET`:**
`/api/sales` (list, with `currentPosId`), `/api/sales/{id}`, `/api/sale-units-tables`, `/api/item-in-sales`,
`/api/account` (liveness), `/` (CSRF cookie), plus the one `POST /api/authentication`. Nothing else. Not
`close-shift`, not `sale-pays`, not any `/api/sales/*` sub-path.

**Two loops, because two facts move at two speeds:**

1. **Floor** — `GET /api/sale-units-tables?pointOfSaleId=1` (4,148 bytes for 18 tables, measured) every
   **60 s while the venue is open** (memory "Owner console": venue hours are in the hub, `hours.rs`), skipped
   when closed. ≈ 14 h × 60 = **840 requests/day, 3.5 MB/day**. Gives `OCCUPIED`/`orderTotal` per table and the
   moment a table is billed (its `orderTotal` returns to `null`) — the join key for (2).
2. **Sales** — `GET /api/sales?beginDate=<yesterday>&endDate=<today>&size=500&sort=id,desc&currentPosId=1`
   every **5 min while open** (168/day), 3,386 bytes per row (measured; `logCis` embeds the sale twice), ≈ 25
   rows/day live → ≈ 85 KB per call, **14 MB/day**. The list gives bills and counter sales and the **id
   watermark**. Courses are not in the list (§1.6): for every id between the last watermark and the newest
   listed id that is not itself listed, `GET /api/sales/{id}` (≈ 10 KB, 12 KB with 2+ lines). 55 ids/day were
   observed on 2026-09-21 (24 bills + 31 courses) → **≈ 31 detail requests/day**; a `404` on a gap id is
   recorded, not retried (a deleted draft).
   Overlapping `beginDate` by one day covers the unknown midnight (§1.4); the watermark makes the overlap free.

**Total ≈ 1,040 requests/day, ≈ 18 MB/day inbound, ≈ 31k requests/month** against ebills. On the dowiz side the
poller is a Worker cron (`wrangler.toml` `[triggers] crons`; 1-minute granularity is Cloudflare's floor) or a
Durable-Object alarm on the venue's `HubImages` object; each firing is one billable request. At the public
price list (Workers Paid: 10 M requests included, then $0.30/M; DO requests $0.15/M after 1 M; **not measured
here** — memory "What one hub actually costs" measured $5.93/mo all-in for a hub and found request COUNTS and
the whole-image log rewrite are what bind), 1,440 cron firings/day = 43k/month is inside every included
allowance; **the write that costs is the per-course `Placed` append**, ≈ 31/day, each a whole-image rewrite of
the order log — the same cost as 31 storefront orders, i.e. the venue's dine-in volume is roughly its delivery
volume and doubles the hub's log writes. Per memory, that is the number to watch, not the fetches.

**Login cost:** once per 31 days on the happy path (remember-me), plus once per session lapse; the CSRF
two-step is two requests. MFA, if the operator enables it on the polling user, ends unattended polling — the
poller must fail loudly, not retry.

**Latency to the hub:** a course appears in dowiz ≤ 5 min after it is rung up; a table's occupancy ≤ 60 s.
Neither is a live kitchen ticket (§6.3).

---

## 5. Impossible without a write, stated plainly

1. **The line items of an OPEN table** — `POST /api/sales/get-sale-unit-orders`. Until someone with a write
   permission establishes that this POST is an idempotent read (on a table not in service), dowiz can show a
   table as occupied with a total, and cannot show what is on it.
2. **The bill → courses join** — `POST /api/sales/summary-invoice`. Reconstructed by table + window instead
   (§1.6); exact only if no two sittings at one table are billed inside one poll interval.
3. **The vocabulary of `status` / `fiscalSatus` / `changedStatus`** on a cancelled or amended sale — only
   observable by cancelling one (`PUT /api/sales-cancel/{id}`), which is a fiscal act. Until observed, the
   mapper refuses anything but `CLOSED/FINISHED/0/SUCCESS`, and the poller must re-read recent ids
   periodically so a later cancellation is at least SEEN (§6.6).
4. **Whether `beginDate` cuts at UTC or Tirana midnight** — needs a sale in the two-hour gap; a read, but not
   one this box can cause. The overlap in §4 makes it moot.
5. **A narrower role for the poller** — `POST /api/extra-users`. The operator creates it in the UI.
6. **Pushing dowiz's delivery orders INTO ebills so they are fiscalised** — `POST /api/sales`. That is the
   other half of "one place manages both", it is a fiscal act on every order, and it is out of scope of this
   read-only lane by the operator's instruction. It is named here so nobody mistakes the import for it.

---

## 6. Risks

1. **The same sale arrives twice** — every poll overlaps the previous one by design (§4). The order `id` is
   `ebills:<uuid>`; the hub does NOT refuse a second `Placed` for an id it already has (verified: `Hub::holds`
   is a content-id check, `lib.rs:711`; nothing in `command/`, `hubdo.rs` or `fold.rs` names a duplicate order
   id — grepped `holds(|AlreadyPlaced|Duplicate`, 0 relevant hits), so the poller's check `history(id).is_empty()`
   must be a `command` the object executes, check-and-append in one turn, never a Worker-side read followed by a
   write across the hop (the shape `command/mod.rs`'s header names as the one that "hoped"). Two
   different ebills sales can never share a `uuid`; two arrivals of one sale always do. A sale that arrives,
   is later AMENDED in ebills (`saleModified` grows) and arrives again with the same `uuid` is the case that
   must be a `Noted` event, not a second `Placed` and not silence: the poller compares `logCis` length and
   `totalValue` with what it holds.
2. **Nothing is ever pushed back.** The import is one-way. No dowiz surface may call `close-sale-unit`,
   `sales-cancel`, `refiscalize`, `close-shift`, `removeItems`, `csv-upload` or `POST /api/sales`. This is the
   invariant to enforce in code: the ebills client type exposes `get_*` for the six allow-listed paths and has
   no method that sends a body except `login`. A route that "just checks" via a POST is how §2 happens.
3. **It is history, not a ticket.** A course is fiscalised when the waiter rings it up, i.e. when it is
   ordered, and ebills shows it as `COMPLETED` at once. dowiz will see it as a served `PICKED_UP` order up to
   5 min later. The kitchen console must not show ebills courses in the `Pending` queue: they are already on
   the pass. Showing them as `PICKED_UP` history with the table number is honest; showing them as new orders
   would ring the bell twice for every course.
4. **Stock double-draw and the last portion.** A course draws stock at import (`Consumed`), a dowiz delivery
   draws at `place` (`Reserved`). Both against one ledger — good, that is the goal — but a course can arrive
   for an ingredient dowiz already refused a delivery on (`OutOfStock` is a refusal, `stock.rs:14-18`). The
   right answer is a NEGATIVE-not-refused consume for imported courses, flagged in the owner's stock view: the
   dish was served, the count is what it is, and refusing the record would make the ledger lie. Needs a
   `StockEvent` variant or a policy flag on `Consumed`; not present today.
5. **Floats.** Every amount is a JSON double; `26.6666666666` is a VAT figure, `250.0` a price. The mapper
   accepts a value only if it is whole and below 2⁵³, else refuses the whole sale with the field named. A
   venue that later sells by weight (`amount: 0.35`) will see refusals, not rounding — the kernel's rule
   ("no float, ever") applied at the border.
6. **Cancellations are invisible until proven otherwise.** Nothing in 452 sales showed one. A poller that only
   walks forward will never see a sale cancelled after import. Re-read the last 7 days' ids once a day; any
   change in `status`/`changedStatus`/`saleModified` becomes a `Noted` event and an owner notification, never an
   automatic `Refunding` (the FSM edge exists, the evidence does not yet).
7. **The session, the WAF, the tenant header.** Three ways to be 401/403'd silently: a lapsed session with a
   consumed remember-me, a User-Agent the WAF dislikes, a missing `X-TENANT-IDENTIFIER`. Each answers
   `problem+json`; the poller must surface them to `worker_errors` (memory: it has never fired) with the
   `errorKey`, and must never fall back to "no sales today" (memory "A silent fallback hid a broken call").
8. **Personal data.** `server` (a name) on every floor read; `client.name/nipt/address` on named-client sales;
   the `qrCode` carries the TIN; `extraUser.operatorCode` is a regulated identifier. Store `operator_id`, never
   the code; never store `server`; store `client` only as `customer_id: null` unless the owner enables it.
9. **The account.** `ROLE_ADMIN` reads the signing certificate and keystore password in list responses
   (§1.7), can close the shift with a GET, and can cancel invoices. The poller must run as a user with
   `ROLE_REPORT` (or the narrowest role that can `GET /api/sales`), and the Worker secret must be that user's,
   not the operator's. Until then, do not deploy a poller with `/root/.ebills_account`.
10. **The endpoint is a moving target.** No OpenAPI, no versioning in the path, chunk hashes that change on
    every deploy. Every parser refuses unknown shapes loudly (missing `uuid`, missing `saleRecords`, a status
    word not in the observed set) rather than defaulting; a refusal is an owner-visible count, not a log line.

---

## 7. What was written, and what is handed back as text

**Written (owned by this lane), four files under `workers/api/src/ebills/`:**

- `wire.rs` (140 lines) — the measured shapes as serde types: `Sale`, `SaleRecord`, `LogCis`, `SaleUnit`,
  `SaleUnitOrder`, `SaleList`, `Detail`, `TableState`. Undeclared keys are ignored, missing declared keys are
  parse errors; `client.name`, `operatorCode`, `server`, `activeUser`, `certificate` have no field at all.
- `time.rs` (40 lines) — `epoch_ms`: ISO-8601 UTC to Unix ms with no clock and no crate, fraction truncated.
- `mod.rs` (285 lines) — `parse_list` / `parse_detail` / `parse_tables`; `classify` (course / bill / counter
  sale, and the refused fourth combination); `whole` / `lek` (whole numbers below 2⁵³ or a refusal naming the
  field); `payment`; `finished`; `to_order` (a course or counter sale → the hub's envelope, born `PICKED_UP`,
  per-line and per-sale conservation checks); `to_paid` (a bill → the `Paid` payload).
- `tests.rs` (476 lines, 16 tests) — the wire fixtures of §1.5/§1.6 with synthetic identifiers, and one
  refusal test per rule.

No route, no fetch, no clock. Every function is private (`fn`, not `pub`), deliberately:
`tools/gates/unreached.py` counts `pub`/`pub(crate)` fns that no shipping code calls, and a module with no
route has, by definition, no caller yet. Making them `pub(crate)` is the wiring change's job, in the same
commit as the route that calls them. There is no `allow(dead_code)`.

**Verified by:** a scratch crate under the scratchpad that `#[path]`-includes `mod.rs` and runs its tests
against the same `serde`/`serde_json` the Worker uses (`cd workers/api && cargo test ebills` cannot see a module
`lib.rs` does not declare) — `16 passed; 0 failed`, exit 0; `tools/gates/file-size.sh` → `40 file(s) over 300
lines (baseline 40)`, exit 0; `tools/gates/unreached.py` → exit 0 with no `ebills` row; `rustfmt --check` on
all four files → exit 0. The commands and their output are in the lane's report.

**Handed back as text (files this lane does not own):**

- `workers/api/src/lib.rs`: one line, `mod ebills;`, beside `mod integrations;` — NOT before the route that
  calls it exists, or the module is dead code by the gate's definition.
- `wrangler.toml`: `[triggers] crons = ["* * * * *"]` is the floor; the sales loop should skip four of five
  firings and the floor loop should skip all firings outside `hours.rs`'s open window.
- Secrets: `EBILLS_USER`, `EBILLS_PASSWORD` for a `ROLE_REPORT` user the operator creates, `EBILLS_POS_ID=1`,
  and the tenant identifier learned at login stored in the venue's `settings` image, not as a secret.
- `.github/workflows/ci.yml`: nothing until wired; the module's tests run under the existing `workers/api`
  cargo-test job once `mod ebills;` lands.

---

## 8. 2026-09-23: wired, and what the live platform corrected

### 8.1 Status (2026-09-23)

**Wired, not yet deployed at the time of writing.** Everything below is in `workers/api/src/ebills/`
unless another path is given; every claim has a native test beside it.

| Piece | Where | What it does |
|---|---|---|
| The allow-list as a type | `client.rs`, `judge.rs`, `fetch.rs` | `Path` names the six reads of §4 and nothing else; `Wire` has two constructors (`get`, the one `login` POST); `allowed()` re-checks every URL and `fetch.rs` sends nothing it rejects. Every refusal shape (401/403/problem+json/redirect/HTML/MFA/tenants/shift) is a named failure, never "no sales". |
| The mapper | `map.rs`, `mod.rs`, `wire.rs` | §3.1, corrected by §8.2. Law 3 holds by construction (checked with `e2e/gates/conservation.mjs` over 35 real imported orders: `CONSERVATION HOLDS`). |
| The import, one object turn | `import.rs` (+ `import/join.rs`, `import/bill.rs`), `glue.rs`, `hubdo/ebills.rs` | §6.1: unknown uuid → `Placed` (PICKED_UP); changed total/logCis → `Noted` (money untouched); unchanged → nothing (bytes equal). A bill → `Paid` on its courses (a `bill{}` block, never `payments[]`, so law 10 is untouched); bills wait up to 2 days for their courses, then are refused by name. |
| Stock (§6.4) | `crates/dowiz-hub/src/stock.rs` | `StockEvent::Served` (never refused for the shelf; may go negative, `stock::short` names it) and `Unserved` (a void puts back exactly what its sale served, never twice). Old logs fold unchanged (tested with old-format bytes). |
| The crosswalk | `state.rs`, `status.rs`, `routes.rs`, `public/admin/ebills.js` | ebills `itemCode` → dowiz `product_id`, set only by the owner (`POST /api/owner/ebills/map`); suggestions by normalised name + price agreement. Unmatched lines stay `ebills:<code>` and draw no stock. |
| The poller | `poll.rs`, `walk.rs`, `lib.rs` `scheduled` | On the existing minute cron. Floor every 60 s while open (own `floor` image, rewritten only on change); sales every 5 min open / 30 closed / at once on backlog; ≤ 20 details per firing plus 5 probes past the newest id; the daily 7-day re-read; the re-check pass over closed courses (10 ids a firing); a first run reads 40 ids back as LEADS. Failures: venue error log, `last_error`, backoff 1→60 min, a halt on MFA / several tenants. |
| Credentials | the venue's own `ebills` image | Per venue, not a Worker secret: one Worker serves every venue, and an `EBILLS_PASSWORD` secret would tie the platform to one till. Written by the owner's route, answered nowhere. |
| Owner surface | `GET /api/owner/ebills`, the console's "Till (ebills)" pane, `/api/owner/health` → `ebills` | Link state and last error, unmatched codes with suggestions, mappings, the floor, and the settings. |

**Live, read-only proof (2026-09-23):** login + 43 allow-listed GETs. 32 listed rows (22 bills), 35
course details, 1 gap (404); 35 placed, 24 courses paid, 18 of 22 bills joined; the same day imported
twice placed and paid nothing and left the bytes equal. The till's menu: 200 items; against the dowiz
catalogue 150 match exactly by name and price, 10 fuzzily, 40 not at all.

### 8.2 Five corrections from the live platform

1. **A course at an OPEN table is listed and is `OPENED`, not hidden and not `CLOSED`.** §1.6 said the
   list hides courses; on 2026-09-23 the list returned 10 rows `summaryInvoice:false, status:"OPENED"`
   with a `saleUnitOrder`, all fiscalised (`FINISHED`, `logCis SUCCESS`). They turn `CLOSED` once the
   bill is issued (their details did). The mapper accepts `OPENED` for a course only; a counter sale
   that says `OPENED` is still refused.
2. **`saleUnitOrder` is `{id}` in the list (no `status`), and it is NOT shown to group a sitting.**
   The details inspected each carried a distinct id (8690 → 5668; at table 10, the course 8717 → 5685
   and its void 8718 → 5686), and the list omits `status`, which parsing had required. The bill→courses
   join therefore does not rely on it: all unbilled courses at the table when they sum to the bill,
   else the newest run that does, else the oldest (`import/join.rs`).
3. **A line's `discount` is a PERCENT and `price` is already discounted.** Every discounted line on the
   day was `price 0.0, discount 100.0, totalValue 0.0` (a dish given free, `discountReduceBase:true`).
   The first mapper subtracted it as lek and refused all of them. Now the percent is kept as
   `discount_pct`, `price × amount == totalValue` is checked, and a partial discount whose `price` is a
   list price does not add up and is refused, never guessed.
4. **A cancellation was observed (§5.3, §6.6).** Sale 8718: `changedStatus:"CANCELLED"`, every line
   negative (`amount -1`, `totalValue -4600`), `modified` = the sale it reverses (8717, with its
   `uuid`). Table 10's bill for that sitting was 0 (+4600 −4600). It is imported as its own negative
   order carrying `void_of`; the voided order gets a `Noted` `ebills_voided_by`; the shelf gets back
   exactly what the voided sale served (`Unserved`); the bill of 0 pays both.
5. **The catalogue escapes `&`** (`J&amp;B` in the public menu, `J&B` on the till): name matching decodes
   it before comparing.

Also measured: a sale id that does not exist answers `404 application/problem+json` (so a gap is
recorded, not retried); the first run's 4 unjoined bills were exactly those whose courses were rung up
before the first window — which is why a first run now reads 40 ids back as leads (placed only when a
window bill claims them, never otherwise).

### 8.3 Still open

- Courses before the first window that no window bill claims are not imported (by design: they belong
  to days before the link existed).
- The `Paid` join is a heuristic by table and time (§5.2); two sittings at one table whose totals are
  equal and billed in one poll could be paid in the wrong order of sitting — the totals are right either
  way.
- The poller's user should hold the narrowest role that reads `/api/sales`; per the SPA bundle the sale
  list needs `ROLE_ADMIN` or `ROLE_SALE` (§6.9), so `ROLE_REPORT` alone may be refused — test it with the
  allow-listed reads first.

