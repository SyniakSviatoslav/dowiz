# ebills.al — the write path: what `POST /api/sales` is, as the SPA builds it, from evidence

**Date:** 2026-09-24. **Lane:** research, read-only. **Parent:** `BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md`
(§1 platform, §2 the `close-shift` incident, §5 impossible-without-a-write, §6 risks, §8 the 2026-09-23
corrections). This document exists so that a later build lane can implement a sender without guessing a
single field name. It does NOT authorise a send: every item in §7.2 is something only the first real send —
a fiscal act at the Albanian tax authority — can settle.

**Method, and exactly what was sent.** Two sources, kept apart as the blueprint keeps them:

1. **The SPA bundle**, the same 228 chunks (9.0 MB) the 2026-09-22 lane fetched, still on disk under that
   lane's scratchpad (`chunk-IWIZNCDX.js` + `chunks/`). Read with grep and a brace-balanced method extractor;
   nothing was fetched again. Every claim below that says *the SPA does X* names the chunk and quotes the
   code. The bundle shows what the UI SENDS and how it READS an answer; it is not evidence of what the server
   requires, defaults or rejects — those are marked **server-side, UNKNOWN** wherever the wire did not show them.
2. **The live platform, read-only**: three runs of one script, each `GET /` → `POST /api/authentication` →
   `GET /api/sales?…` (list), and twice `GET /api/sales/7783?currentPosId=1`. Total: 3 logins, 3 list reads,
   2 detail reads, **all on the blueprint §4 allow-list; nothing else was sent**. The first run's 3-day
   window (2026-09-22..24) held 41 rows — 4 courses, 37 bills, **0 counter sales** — so the window was
   widened to 30 days (456 rows: 4 courses, 451 bills, **1 counter sale, id 7783**), which is the real sale
   §1.6 is built from. Credentials came from `/root/.ebills_account` (sourced; the password, cookies and
   tenant header were held in shell variables and files that the script deletes on exit; none reached
   stdout). Identifiers are redacted as the blueprint redacts them: `<str:N>` is an N-character string.

**Verdict, in six lines.**

1. `POST /api/sales` takes the SAME wrapper the detail GET returns — `{sale:{…}, saleType, includeClosingTheTable,
   …}` — with the whole `sale` entity inside; the `sale` carries `saleRecords[]` with per-line VAT the client
   computed. No `currentPosId` query parameter on create (the POS travels inside `sale.pointOfSale`); `PUT
   /api/sales/{id}/{currentPosId}` (update) carries it in the path. (§1)
2. Headers: the session cookies, `X-TENANT-IDENTIFIER`, and the CSRF pair `XSRF-TOKEN` cookie ↔ `X-XSRF-TOKEN`
   header — Angular's built-in XSRF interceptor adds it to every non-GET; there is no app interceptor for it. (§1.2)
3. Fiscalisation is synchronous inside the POST: the response body is the created `sale` with `logCis[0]`
   already `SUCCESS` or `ERROR`; sale 7783 was fiscalised in the same second it was created. (§2)
4. `uuid`, `invOrdNum`, `fic`, `timestamp`, `status`, `fiscalSatus`, `extraUser` are NOT sent by the SPA; the
   server sets them. Whether the server honours or de-duplicates a client-supplied `uuid` is **UNKNOWN** —
   the SPA's only guard against a double send is a 700 ms click timer. (§3)
5. Cancellation is `PUT /api/sales-cancel/{id}` with `{reason, sale, withoutfiscalization, saleType}` after
   `POST /api/sales/can-cancel`; the answer is the negative sale (observed as 8718 on 2026-09-23). (§4)
6. The sending user needs `ROLE_SALE` (or `ROLE_ADMIN`) on the SPA routes, a valid fiscal `operatorCode`
   (`extraUser`), the day's cash-balance `INITIALIZE` on the POS, and an open shift. (§5)

---

## 1. The create request

### 1.1 Method, path, query

`chunks/chunk-GX4POSSC.js`, the sale service (`resourceUrl = getEndpointFor("api/sales")`):

```js
create(e){let c=this.convertDateFromClient(e.sale),i=K(V({},e),{sale:c});
  return this.http.post(this.resourceUrl,i,{observe:"response"}).pipe(J(s=>this.convertDateFromServer(s)))}
update(e,c){let i=this.convertDateFromClient(e);
  return this.http.put(`${this.resourceUrl}/${B0(e)}/${c}`,i,{observe:"response"})…}   // c = currentPos.id
convertDateFromClient(e){return Object.assign({},e,{
  timestamp:e.timestamp?.isValid()?e.timestamp.toJSON():void 0,
  periodStartFisc:e.periodStartFisc?.isValid()?e.periodStartFisc.format(b3):void 0,
  periodEndFisc:e.periodEndFisc?.isValid()?e.periodEndFisc.format(b3):void 0,
  delivery:e.delivery&&typeof e.delivery.deliveryDate!="string"?K(V({},e.delivery),{deliveryDate:…}):null})}
```

| | |
|---|---|
| Method / path | **`POST https://www.ebills.al/api/sales`** — no query string. (`downloadSale`, `printOrder`, `exportSalesPDF`, `update` add `currentPosId`; `create` does not.) |
| Body | JSON, the **wrapper** of §1.3 with the **sale** of §1.4 inside |
| `Content-Type` | `application/json` (Angular's default for an object body) |
| Answer | `200`, body = the created **`sale` entity** (not the wrapper; `subscribeToSaveResponse` reads `l.body.logCis[0]`, `l.body.totalValue`, `l.body.saleUnitOrder`), plus an optional `ALPHA-WEB` response header (§2.1) |

### 1.2 Headers

From `chunk-IWIZNCDX.js`, the app's interceptor list `Ym` — six, in order: `Zm` (loading bar), `Bm`
(session-expired → `/login`), `Um` (error broadcast), `Wm` (`*-app-alert` headers), **`Gm`**, `$m`:

```js
var Gm=…intercept(t,o){let r=this.localStorageService.retrieve("X-TENANT-IDENTIFIER");
  if(r){let a=t.clone({headers:t.headers.set("X-TENANT-IDENTIFIER",r)});return o.handle(a)}…}
var $m=…intercept(t,o){let r="en-US, q=1";this.translateService.currentLang==="al"&&(r="al, q=1");…
  let a=t.clone({headers:t.headers.set("Accept-Language",r)});return o.handle(a)}
```

**There is no CSRF interceptor in the app.** The `X-XSRF-TOKEN` header comes from Angular's own
`HttpClientXsrf` (`chunks/chunk-JXC7IKTP.js`): cookie name `"XSRF-TOKEN"`, header name `"X-XSRF-TOKEN"`, and the
rule that decides when it is added:

```js
function JA(e,t){let n=e.url.toLowerCase();
  if(!g(TI)||e.method==="GET"||e.method==="HEAD"||n.startsWith("http://")||n.startsWith("https://"))return t(e);
  let r=g(is).getToken(),o=g(QA);return r!=null&&!e.headers.has(o)&&(e=e.clone({headers:e.headers.set(o,r)})),t(e)}
```

So the SPA (which sends relative `api/sales`) gets the header on every POST/PUT/DELETE/PATCH. A sender that
uses absolute URLs must add it itself — exactly as the existing `Wire::login` already does for the one POST it
sends (`workers/api/src/ebills/client.rs:181-197`: `x-xsrf-token: <XSRF-TOKEN cookie>`). The full header set
for a create, therefore:

```
POST /api/sales HTTP/1.1
Host: www.ebills.al
User-Agent: <a browser UA — the WAF 403s curl's default, blueprint §1.1>
Accept: application/json, text/plain, */*
Content-Type: application/json
Cookie: JSESSIONID=<…>; XSRF-TOKEN=<…>; remember-me=<…>
X-XSRF-TOKEN: <the current XSRF-TOKEN cookie value — it ROTATES at login, client.rs::absorb keeps the latest>
X-TENANT-IDENTIFIER: <64 hex, from the login response header>
Accept-Language: en-US, q=1        (optional; picks the language of faultStringMsg / errors)
```

Measured on the login POST (blueprint §1.2): without the CSRF pair the server answers `403 "Could not verify
the provided CSRF token…"`. **Hypothesis, not measured:** the same filter guards `/api/sales`; Spring Security's
CSRF filter is global, and the SPA never sends a POST without the header.

### 1.3 The wrapper, as the SPA builds it

`chunks/chunk-YQFNNNTR.js` (1,055,604 bytes; components `jhi-sale-update`, `jhi-new-sale-update`,
`jhi-exchange-sale-update`, `jhi-sale`), method `save(t,n,i)` — the seven `saleService.create(` sites are all
this method or its POS twin, and they build the wrapper identically:

```js
// c = this.createFromForm()  (the sale, §1.4); after the pre-checks of §1.5:
let I;
if(serialNumber!==null && serialDate!==null)                      // manual/corrective by serial — not our path
  I={sale:T({},c),serialNumber:…,serialDate:O.toJSON(),serialNumberEic:…,correctiveBySerialNumber:!0};
else I={sale:T({},c),serialNumber:null,serialDate:null,serialNumberEic:null,correctiveBySerialNumber:!1};
if(this.setting.haveRooms||this.setting.haveTables)
     I=V(T({},I),{saleUnit:this.currentSaleUnit,saleType:ct.ORDER});
else I=V(T({},I),{saleType:ct.NORMAL});
t ? (n===!1 ? I=V(T({},I),{includeClosingTheTable:!1,saleType:ct.ORDER})
             : I=V(T({},I),{includeClosingTheTable:!0,saleType:ct.NORMAL}))
  : I=V(T({},I),{includeClosingTheTable:!1});
if(!i){let O=yield this.openPaymentModal(c);                       // §1.5: a modal ONLY for external methods
  if(O.action==="cancelled"){this.isSaving=!1;return}
  O.action==="confirmed"&&(I=V(T({},I),{externalReferenceCode:O.data.externalReferenceCode,sale:this.toCardIfOnPos(I.sale)}))}
c.paymentMethod===R.MULTIPLE
  ? (I=V(T({},I),{saleType:ct.ORDERANDSUMMARY}),this.subscribeToSaveResponse(this.saleService.create(I),!0,n))
  :  this.subscribeToSaveResponse(this.saleService.create(I),!1,n)
```

The `saleType` vocabulary (`chunks/chunk-RBIYWZGD.js`, `var ue`): `ORDER | ORDERANDSUMMARY | SUMMARY | NORMAL`.
What the three flags mean, from their callers:

| Caller | `t` | `n` | Result |
|---|---|---|---|
| "Order" button at a table (`orderAndCollect`, open orders exist) | `false` | `false` | `saleType: ORDER`, `saleUnit`, `includeClosingTheTable:false` → **a course** (blueprint §1.6) |
| "Order & collect" at a table with NO open orders | `true` | `true` | `saleType: NORMAL`, `saleUnit`, `includeClosingTheTable:true` → **a counter sale rung at a table, closed at once** — this is sale 7783's shape |
| a POS without rooms/tables | – | – | `saleType: NORMAL`, no `saleUnit`, `includeClosingTheTable:false` → **a plain counter sale** |
| `paymentMethod: MULTIPLE` | | | `saleType: ORDERANDSUMMARY` and the `pay` call follows (§2.1) |

**Wrapper fields the server READS BACK** (measured: the top-level keys of `GET /api/sales/7783`) are every
`sale` field flattened (all `null`/`[]`) plus **`reason, sale, withoutfiscalization, saleModified, saleType,
includeClosingTheTable, periodStart, periodEnd, qrCode`**. `serialNumber`, `serialDate`, `serialNumberEic`,
`correctiveBySerialNumber`, `externalReferenceCode` are sent by the SPA and **not** echoed — write-only fields
or ignored (JHipster's Jackson does not fail on unknown properties by default; **server-side, UNKNOWN**).

### 1.4 The `sale`, field by field

`createFromForm()` (`chunk-YQFNNNTR.js` @337614) sends exactly these keys; the form group (`this.editForm =
this.fb.group({…})`, same chunk, in the `jhi-sale-update` constructor) gives each one's default on a NEW sale:

| Key | Type | Value on a new sale (SPA default) | Evidence / note |
|---|---|---|---|
| `id` | number\|null | `null` | `fb.control(null)`; the server assigns |
| `totalValue` | double | Σ `saleRecords[].totalValue`, **required** | `fb.control(null,K.required)`; 7783: `200.0` |
| `totalVatAmount` | double | Σ `saleRecords[].totalVatAmount`, **required** | 7783: `33.3333333333` |
| `currencyRate` | double | `1` | `fb.control(1,K.required)`; `onCurrencyChange`: `currencyCode==="ALL" ? 1 : sellRate` |
| `currency` | object | `{id:1,currency:"Albania Lek",currencyCode:"ALL",isActive:true}` then overwritten by `business.currency` | `fb.control({…ALL…})`; `loadRelationshipsOptions`: `setValue(this.business.currency)` |
| `paymentMethod` | enum | `"CASH"` | `setDefaultPaymentMethod` / `setValue(R.CASH)`; enum `p6` in `chunk-GX4POSSC.js`: `CASH, BANK, BKT_BANK, CARD, CARD_ON_POS, POK_CARD, PAYSERA, EREJA, OTHER, VOUCHER, WAIVER, COMPENSATION, CHECK, KIND, MULTIPLE` |
| `salePaymentMethods` | array | `[]` unless `MULTIPLE` | `getSalePaymentMethods()`: `[{paymentMethod:"CASH",amount:n},{…CARD…},{…CHECK…},{…VOUCHER…}]` |
| `selfIssue` | enum | `"SELFISSUE_0"` | `Me` enum `SELFISSUE_0..5`; i18n: 1 Agreement, 2 Domestic, 3 Abroad, 4 Other, 5 Self |
| `isReverseCharge` | bool | `false` | |
| `isExport` | bool | `false` | required |
| `invOrdNum`, `fic` | | `null` | form controls, never set on a new sale — **the server assigns** |
| `timestamp` | ISO string | **omitted** (`void 0`) | `timestamp:… ? D(…) : void 0` and `convertDateFromClient` → the server stamps it; 7783: `"2026-09-04T12:49:36.981Z"` |
| `importedInvoiceNumber`, `contract`, `projectReference`, `notes` | string\|null | `null` | `notes` max 500, `contract` max 255 — free text the server keeps (see §3 for a use) |
| `periodStartFisc`, `periodEndFisc` | `YYYY-MM-DD`\|omitted | omitted | periodic invoices only |
| `document`, `documentContentType`, `filename` | | `null` | attachment |
| `invoiceTypeId` | number\|null | `null` | every sale read back has `388` (tax invoice) — **server default, hypothesis** |
| `invoiceType` | object\|null | `null` | |
| `draft`, `status`, `fiscalSatus`, `changedStatus`, `modified` | | `null` | read back as `0 / CLOSED / FINISHED / null / null` on 7783 — **server sets all five** |
| `client` | object | the venue's `defaultClient` row | `clientsSharedCollection.filter(l=>l.defaultClient)` → `clientSelected(i[0])`; the SPA sends the WHOLE client object it listed (7783's read-back block: `{id:1, defaultClient:true, status:"ACTIVE", nipt:"", identType:"ID_NUMBER", country:"ALB", …}`) |
| `extraUser` | object\|null | **`null`** | `resetPanel`: `editForm.get(["extraUser"])?.setValue(null)`; only an EXISTING sale patches it (`updateForm`: `extraUser:t.extraUser`). 7783 read back `extraUser.id:1` = the login's `extraUser` → **the server takes the operator from the session** |
| `pointOfSale` | object | the current POS object | `editForm.get(["pointOfSale"]).setValue(this.currentPos)`; `currentPos = JSON.parse(atob(localStorage["pos"]))`, the row `GET /api/valid-pos` returned (blueprint §1.3) |
| `delivery` | object\|null | `null` | the tax-law TRANSPORT block, not food delivery: `{address, city, countyCode:"ALB", transporterName, transporterNIPT, licensePlate, deliveryDate}` (`setDelivery`) — leave `null` |
| `saleRecords` | array | the lines, §1.4.1 | |
| `saleEinvoiceInfos`, `saleBanks`, `fees`, `vouchers` | arrays | `[]` | e-invoice/bank/fee/voucher extras |
| `clientExtraAddress` | | `null` | |

Not sent by `createFromForm` and therefore server-owned: **`uuid`**, `summaryInvoice`, `exchange`, `buying`,
`saleUnit` (it travels at WRAPPER level, §1.3), `saleUnitOrder`, `salePays`, `logCis`, `qrCode`,
`valueWithoutVat`.

#### 1.4.1 A line (`saleRecords[]`)

The POS item-panel builder (`chunk-YQFNNNTR.js`, three copies; `t` is the `/api/item-in-sales` row, `S`/`i`
the quantity, `z`/`w` the discount %, `c`/`h` = `getVAT(vat)`):

```js
getVAT(t){return t===ce.VAT_0?1:t===ce.VAT_6?1.06:t===ce.VAT_10?1.1:1.2}
… (z!==0)&&(w=w*(1-z/100));                                   // price AFTER the percent discount
let A={itemInSale:t, itemName:t.item, saleNo:this.getSaleNo(),  // 1-based, max(saleNo)+1
  refundType:this.isPharmacy?ft.NORMAL:null, notes:this.setting?.addItemNote?t.notes:null,
  serialNumber:this.setting?.pharmacy&&!t.isService?t.serialNumber:null,
  price:w, amount:S, expireDate:I, discount:z,
  priceWithoutVat:Number((w-w/c*(c-1)).toFixed(10)),
  vat:this.isExport||this.isNoVAT?ce.VAT_0:t.vat,
  totalValue:Number((S*w).toFixed(10)),
  totalVatAmount:Number((S*w/c*(c-1)).toFixed(10))}
```

| Key | Type | Rule |
|---|---|---|
| `itemInSale` | object | **the whole `/api/item-in-sales` row** (`id, itemCode, item, price, vat, isService, unit{…}, itemCategory{…}, …`). Whether `{id}` alone is accepted is **UNKNOWN**. The existing `wire::ItemInSale` has NO `id` field (`item_code, item, price` only) — the build lane must add `id` (and pass the row through untouched; nothing personal is in it) |
| `itemName` | string | the item's name at sale time (`t.item`) |
| `saleNo` | int | 1, 2, 3… per sale |
| `amount` | double | quantity (`1.0`); this venue sells whole units only |
| `price` | double | unit price **VAT-inclusive, after discount** — confirms blueprint §8.2 (3) from the sending side |
| `discount` | double | percent, `0` |
| `discountReduceBase` | bool | not sent; read back `false` |
| `vat` | enum | `VAT_0 | VAT_6 | VAT_10 | VAT_20` (`NO_VAT` exists in i18n only); divisor 1 / 1.06 / 1.1 / 1.2 |
| `priceWithoutVat` | double | `price − price/c×(c−1)` = `price / c`, 10 dp |
| `totalValue` | double | `amount × price` |
| `totalVatAmount` | double | `amount × price / c × (c−1)`; for 200 lek at 20%: `33.3333333333` (matches 7783) |
| `refundType`, `notes`, `serialNumber`, `expireDate` | | `null` unless pharmacy / item-note settings |

The kernel's money law is whole-lek `i64`; the sender computes these doubles at the border and never stores
them: `totalVatAmount` is `total − total×10/12` etc., and the sale-level sums must equal the line sums to the
last digit the SPA would produce (`toFixed(10)`), or the server may refuse — **UNKNOWN** whether it recomputes.

### 1.5 What the SPA checks BEFORE it sends (all client-side; the server's own checks are UNKNOWN)

`save(t,n,i)` in order (`chunk-YQFNNNTR.js` @295232):

1. `Date.now()-this.lastClickTime<700 || this.isSaving` → return (the ONLY double-send guard; §3).
2. `cashBalanceRouteAccessService.doesNotRequireCashInitFromLocalStorage(currentPos)` — a POS with
   `registerTcr` needs a cash-balance INITIALIZE **today** for this `posId` (`chunk-D5JVOWHV.js`: compares a
   stored `{posId,timestamp}` with today's date), else the alert "cash balance required" and a redirect to
   `/tcr-cash-balance/register`. The blueprint measured that record (`tcr-cash-balances`, `INITIALIZE` at
   09:19Z on 2026-09-22). Whether the SERVER refuses a sale without it: **UNKNOWN** (the check reads localStorage).
3. `totalExceedNonB2BLimit(total, rate)` — a warning modal above `setting.nonBToBInvoiceLimitWarn`.
4. `client.nipt === business.nipt && !isReverseCharge && selfIssue==="SELFISSUE_0"` → refused ("sameBusiness").
5. `paymentMethod===CASH && !validateCashAmount(total, c)`: **cash > 500,000 lek** to an individual
   (`nipt===""` or `identType ID_NUMBER|PASSPORT`) or **> 100,000 lek** to a `NIPT` client is refused (the
   legal cash limits, i18n `totalExceeded` / `totalExceededForDefaultClient`).
6. `totalValue<0` / `totalVatAmount<0` on a new sale without a serial → refused.
7. `saleConfirmation(c)` — a confirm modal only if `setting.addInvoiceConfirmation`.
8. `checkforClientVerification(client)` — NIPT clients only (walk-in default: no call).
9. **`checkOperatore()`** — `GET api/extra-users/check` at most every 30 min per operator, answering
   `{status, errMessage}`; `status:false` → modal with `errMessage` and NO send; a non-200 → warning
   "Could not verify Operator validity from self care. invoice might not be fiscalized if the operator is
   not valid" and the send proceeds. Also, at panel load, **`GET api/sale/pre-check?posId=`** →
   `{status, errMessageCode, errMessage}` (comma-separated codes; `Unit`/`server` classes), and the
   business's `electonicCert` expiry is compared with today ("certificateExpired").
10. `openPaymentModal(c)` returns `{action:"noPaysera"}` at once unless `paymentMethod ∈ {PAYSERA, EREJA,
    BKT_BANK, POK_CARD, CARD_ON_POS}` — **for `CASH` and `CARD` no payment API is touched before or after the
    create** (`subscribeToSaveResponse` calls `pay` only when `n` (MULTIPLE) or `withPaymentModal && !saleUnitOrder`,
    and `withPaymentModal` is set only for those external methods / `CARD_ON_POS`). `toCardIfOnPos` rewrites
    `CARD_ON_POS` → `CARD` in the sent sale.

Steps 9 and the `pre-check` are GETs the existing client cannot name (`client.rs::Path` has no variant). They
are reads by their use; adding them to the allow-list is a decision for the build lane and the operator, not
a fact this document can supply — after §2 of the blueprint, a name is not evidence.

### 1.6 A redacted example body, built from a REAL sale

Sale **7783** (`GET /api/sales/7783?currentPosId=1`, measured 2026-09-24; the one counter sale in 30 days:
`summaryInvoice:false`, `saleUnitOrder:null`, `status:CLOSED`, `fiscalSatus:FINISHED`, `draft:0`,
`paymentMethod:CASH`, `totalValue:200.0`, `salePays:null`, `delivery:null`, `logCis[0].status:SUCCESS`,
`timestamp 2026-09-04T12:49:36.981Z`, `logCis[0].sentTime 2026-09-04T12:49:36Z`). It was rung at table 7
with no open orders (the `t=true,n=true` row of §1.3), so its `saleUnit` is present. Mapped BACK into the
create shape — server-owned fields removed, defaults from §1.4 restored, one line:

```json
{
  "sale": {
    "id": null,
    "totalValue": 200.0,
    "totalVatAmount": 33.3333333333,
    "currencyRate": 1,
    "currency": {"id": 1, "currency": "Albania Lek", "currencyCode": "ALL", "isActive": true,
                 "isBaseCurrency": false, "sellRate": 1.0, "buyRate": 1.0},
    "paymentMethod": "CASH",
    "salePaymentMethods": [],
    "selfIssue": "SELFISSUE_0",
    "isReverseCharge": false,
    "isExport": false,
    "invOrdNum": null, "fic": null,
    "importedInvoiceNumber": null, "contract": null, "projectReference": null, "notes": null,
    "document": null, "documentContentType": null, "filename": null,
    "invoiceTypeId": null, "invoiceType": null,
    "draft": null, "status": null, "fiscalSatus": null, "changedStatus": null, "modified": null,
    "client": {"id": 1, "name": "<str:14>", "contact": null, "email": null, "notes": null,
               "defaultClient": true, "status": "ACTIVE", "nipt": "", "identType": "ID_NUMBER",
               "address": "", "city": "", "country": "ALB", "spendingLimit": null, "barcode": null},
    "extraUser": null,
    "pointOfSale": {"id": 1, "license": "<str:23>", "posName": "<str:4>", "printing": "THERMAL",
                    "printerSize": "SIZE_80", "address": "<str:70>", "city": "<str:6>", "tcrOrderNo": 1,
                    "bussinesUnit": "<str:10>", "onlyCashInv": false, "isActive": true,
                    "printerBrand": "NORMAL", "printingType": "DIRECT", "connectorPort": 9090, "useSmallDim": true},
    "delivery": null,
    "saleRecords": [
      {"itemInSale": {"id": 23, "itemCode": "16", "item": "<str:15>", "price": 200.0, "promoPrice": null,
                      "vat": "VAT_20", "isMixProduct": false, "noTVSHType": null, "barcode": null,
                      "statusi": "ACTIVE", "isService": true, "linkedItemInWarehouseId": null,
                      "itemCategory": {"id": 2, "category": "freskuese", "status": "ACTIVE"},
                      "unit": {"id": 1839, "unit": "Copë", "unitCode": "XPP", "isActive": true},
                      "itemRemboursement": {"id": 23, "partialRemboursement": 0.0, "fullRemboursement": 0.0}},
       "itemName": "<str:15>", "saleNo": 1,
       "refundType": null, "notes": null, "serialNumber": null, "expireDate": null,
       "price": 200.0, "amount": 1.0, "discount": 0,
       "priceWithoutVat": 166.6666666667, "vat": "VAT_20",
       "totalValue": 200.0, "totalVatAmount": 33.3333333333}
    ],
    "saleEinvoiceInfos": [], "saleBanks": [], "fees": [], "vouchers": [],
    "clientExtraAddress": null
  },
  "serialNumber": null, "serialDate": null, "serialNumberEic": null, "correctiveBySerialNumber": false,
  "saleUnit": {"id": 7, "identifier": "7", "type": "TABLE", "status": "ACTIVE", "posX": 2.0, "posY": 2.0},
  "saleType": "NORMAL",
  "includeClosingTheTable": true
}
```

**For a dowiz delivery / pickup order paid in full**, the shape a POS *without* tables sends is the natural one:
drop `saleUnit`, `saleType:"NORMAL"`, `includeClosingTheTable:false`, `paymentMethod` `"CASH"` or `"CARD"`,
`client` = the default client, `delivery: null`, `notes` = the dowiz order id (§3). At THIS venue the SPA
never sends that combination (`setting.haveTables` is true, so `saleUnit` is always attached) — whether the
server accepts a table-less `NORMAL` sale on a POS configured with tables is first-send unknown **#1** (§7.2).
The fallback that copies 7783 exactly is a dedicated sale unit the owner creates in the UI (e.g. a TABLE
named `DELIVERY`) sent with `includeClosingTheTable:true`.

The redacted full read-back of 7783 (428 lines) is in this lane's scratchpad as `sale-7783-redacted.json`;
it is not committed.

---

## 2. Success, failure, and what the SPA does about a fiscalisation failure

### 2.1 The response, and the sale afterwards

`subscribeToSaveResponse(t,n,i)` (`chunk-YQFNNNTR.js` @324455):

```js
next:l=>{let c=l.body,p=l.headers.get("ALPHA-WEB");
  if(p&&…modal(String(h.alphaErr)+String(p))…, n||this.withPaymentModal&&!c.saleUnitOrder){
    let h={id:void 0,value:c.totalValue/c.currencyRate,method:c.paymentMethod,exchangeRate:c.currencyRate,extraUser:c.extraUser,currency:c.currency},
        S={sale:V(T({},c),{totalValue:…/c.currencyRate,totalVatAmount:…/c.currencyRate}),payment:T({},h)};
    this.saleService.pay(S).subscribe(()=>{ if(c.logCis[0].status==="ERROR"){…modal(c.logCis[0].faultStringMsg)…}
                                            else …; this.downloadSale(c)})}
  else if(c.logCis[0].status==="ERROR"){let h=modal(Be); h.componentInstance.message=c.logCis[0].faultStringMsg;
       h.closed.subscribe(S=>{S==="confirmed"&&(i&&this.onSaveSuccess(c),this.downloadSale(c))})}
  else i?this.onSaveSuccess(c):this.closeSaleUnit(),this.downloadSale(c)},
error:l=>this.onSaveError(l)
```

- **`200` = the sale EXISTS, fiscalised or not.** The body is the `sale` entity with `id`, `uuid`, `invOrdNum`,
  `timestamp`, `status`, `fiscalSatus` and `logCis[]` filled. Success is `logCis[0].status==="SUCCESS"` with
  `iic`, `fic`, `iicReference`, `sentTime`, `invoiceGeneratedAt` (7783 had all; `eic` null — e-invoices are
  for NIPT clients). The later `GET /api/sales/{id}` shows `fiscalSatus:"FINISHED"`, `status:"CLOSED"`,
  `draft:0`; the list shows the same row (a counter sale is listed, blueprint §1.6/§8.2).
- **Fiscalisation failed but the sale was created:** `logCis[0].status==="ERROR"` with `faultString` /
  `faultStringMsg` (the tax authority's fault; `Accept-Language` picks the language). The SPA shows the message
  and **still downloads/prints the receipt** (`downloadSale(c)`). Read back, such a sale is
  `fiscalSatus:"WEBSERVICEERROR"` — the i18n `FiscalStatus` vocabulary (`chunk-6YHOXXRR.js`):
  `{WEBSERVICEERROR:"ERROR", PENDING:"PENDING", FINISHED:"SUCCESS"}`; and `SaleStatus`:
  `{OPENED:"NOT PAID", CLOSED:"PAID", CANCELLED, MODIFIED, UNFISCALIZED, PARTIALLY_PAID}`; `ChangedStatus`:
  `{CANCELLED:"CORRECTED", MODIFIED:"CORRECTED", DISCARDED}`. The list paints `WEBSERVICEERROR|PENDING` rows
  `bg-fiscalization-error` (`getSaleClass`). The sale HAS an `invOrdNum` in that state (**hypothesis** — the
  ordinal is `BY_POINT_OF_SALE` and the receipt printed; not observed, 0 of 456 sales were in error).
- **HTTP error = no sale** (`onSaveError`): `500` → "Server Error : could not create invoice. please check all
  invoice fields and try again!"; `problem+json` with `errorKey` containing `negInventory` → the `title` carries
  `"<message>##<itemId>_<warehouseId>_<n>"` (stock would go negative; `showInventory` is off at this venue,
  blueprint §3.1). A `401`/`403` is the session/CSRF/tenant, as for reads (`judge.rs` already names them).
- The `ALPHA-WEB` response header lists sale ids that failed to reach "AlphaWeb" (an accounting export,
  i18n `alphaErr: "Failed to send invoice to AlphaWeb. ids : "`) — informational.
- After a create, `PUT /api/sales-pay` is called ONLY for `MULTIPLE` or the external methods (§1.5 step 10);
  sale 7783 (`CASH`, `NORMAL`) is `CLOSED` with `salePays:null` and no pay call: **the server closes a NORMAL
  sale on create** (measured for CASH; **hypothesis** for CARD, whose path in the SPA is identical).

### 2.2 Retry of a failed fiscalisation — three mechanisms, none client-side

1. **Per POS, on demand:** the sale list's button `refiscalize()` → **`POST /api/sales/refiscalize`** with body =
   the current POS object; answer: header `x-message` and `body.errors` = `{ "<saleId>": "<fault>", … }`
   (i18n `refiscalizeError: "Sale with Id = {{param}} refiscalized with error : {{param1}}"`). The server
   re-sends every `WEBSERVICEERROR` sale of that POS.
2. **At login, for the whole tenant:** `triggerRefiscalizationCheck()` (`chunk-IWIZNCDX.js`) →
   `GET /api/refiscalization/analyze` → `204` + `X-Cooldown-Until` (nothing to do; the SPA caches the cooldown
   in `sessionStorage`) or a body `{refiscalize:bool, registerEinvoices:bool, missingEInvoiceCount, …}`;
   the "Non-Fiscalized Invoices Detected" modal (`jhi-refiscalization-dialog`) returns that summary with its two
   checkboxes edited, and the SPA sends it as **`POST /api/refiscalization/process`** ("Processing in
   background…"). i18n: "You can submit them now or cancel and handle them manually from the Sale Management
   pages whenever you're ready."
3. **Discard instead of retry:** for a `WEBSERVICEERROR` sale the list shows a different Cancel button
   (`chunk-YQFNNNTR.js` templates `ns`/`os`, guarded by `fiscalSatus==="WEBSERVICEERROR"&&status!=="CANCELLED"
   &&changedStatus!=="DISCARDED"`) that calls `cancel(sale, true)` → `PUT /api/sales-cancel/{id}` with
   **`withoutfiscalization:true`** (§4). i18n `cancelInvoiceWarning`: "This Invoice is not fiscalized. By
   cancelling it, the inventory of the items is returned, but you will not be able fiscalize it later!"
   The result is `changedStatus:"DISCARDED"` (**hypothesis** from the enum and the guard; not observed).

**Offline / the 48-hour "subsequent delivery" rule:** the bundle has **no** such flag and no such constant —
`subseq`, `SUBSEQUENT`, `isSubseqDeliv`, `subsDeliv`, `OFFLINE` have zero hits in 228 chunks; `offline` occurs
only in the Firebase messaging chunk (`chunk-2USTFXG5.js`, "app-offline"). The SPA never sends
`isSubseqDeliv`/`subseqDelivType`/`issueDateTime`/`tcrCode`/`businUnitCode` (the tax authority's own schema
names; `businUnitCode`/`tcrCode`/`issueDateTime` appear only inside the e-invoice/QR display code). **The whole
fiscal envelope — IIC computation with the business certificate (blueprint §1.7), operator/BU/TCR codes,
the subsequent-delivery marking of a late send — is the SERVER's.** How long a `WEBSERVICEERROR` sale may wait
before `refiscalize` is refused, and whether the server marks it as subsequent delivery: **UNKNOWN**.

---

## 3. Idempotency

- **`uuid` is server-generated.** The strings `uuid`/`UUID`/`randomUUID` occur in NO code chunk of the sale
  screens or service (`chunk-YQFNNNTR.js`, `chunk-GX4POSSC.js`: 0 hits; the only `uuid` hits are the three
  i18n files' field labels and the model's field name). `createFromForm` has no `uuid` key. The read-back
  wrapper has `uuid:null` at wrapper level and the real `uuid` inside `sale`.
- **The SPA's only guard against a double send is client-side:** `Date.now()-this.lastClickTime<700 ||
  this.isSaving` (§1.5 step 1). Nothing in the bundle suggests a server-side de-duplication key, and
  `H_`/`ri` ("no changes made") compares field by field for UPDATES only.
- **Whether the server accepts a client-supplied `uuid`, and whether it rejects a duplicate: UNKNOWN.** The
  blueprint's "the idempotency key is `uuid`" (§1.5) is true for the READ side (two arrivals of one sale share
  it); it is not evidence that a POST with a repeated `uuid` is refused.
- **What the sender can do without that knowledge:** (a) write the intent (order id → attempt, before the
  send) into the venue's `ebills` image in the same object turn that sends, so a Worker that dies mid-send
  leaves a visible "sent, answer unknown" row rather than a silent retry; (b) put the dowiz order id in the
  sale's `notes` (free text, ≤ 500, kept and read back — it may print on the receipt) or `projectReference`,
  so an unanswered send is reconciled by the next allow-listed `GET /api/sales?…` list read (the sale
  would carry `notes:"dowiz:<order-id>"`; `totalValue` and `timestamp` as the tiebreak); (c) never retry a
  timed-out POST until that reconciliation has run — a retry is a second fiscal invoice.

---

## 4. Corrective invoice and cancellation — shape only

### 4.1 Cancel (the refund path): `PUT /api/sales-cancel/{id}`

Gate first: `POST /api/sales/can-cancel` with the sale as body; a truthy body allows the modal, else the
message `saleCancelation` (`chunk-YQFNNNTR.js` list `cancel(t,n)`). `checkShowPeriod(sale)` (service) shows
the period fields when the sale is from the previous month and today is on or before the 10th. Then
(`chunk-RBIYWZGD.js`, cancel modal `confirmCancellation`, service `cancel`):

```js
// modal
let l={sale: this.sale?.delivery ? {...this.sale, delivery:{id:this.sale.delivery.id}} : {...this.sale},
       withoutfiscalization:this.withoutfiscalization,          // true ONLY for a WEBSERVICEERROR sale (§2.2.3)
       reason:this.cancellationForm.get(["reason"]).value,      // required; the form's value
       saleType:ue.NORMAL};
if(t.saleUnit&&(t.summaryInvoice&&(l={...l,saleType:ue.SUMMARY}), !t.summaryInvoice&&t.saleUnitOrder&&(l={...l,saleType:ue.ORDER})));
if(confirmAddPeriod&&!periodDatesError) l={...l,periodStart:X(r).format(xe),periodEnd:X(p).format(xe)};
let r=yield Ee(this.saleService.cancel(l));
if(l.sale.status===Ce.CLOSED){ /* book the refund payment */
  let C={sale:{...r.body,totalValue:…/currencyRate,totalVatAmount:…/currencyRate},
         payment:{value:r.body.totalValue/r.body.currencyRate,method:r.body.paymentMethod,exchangeRate:r.body.currencyRate,extraUser:r.body.extraUser,currency:r.body.currency}};
  yield Ee(this.saleService.pay(C))}                            // PUT /api/sales-pay
// service
cancel(e){let c={reason:e.reason,sale:this.convertDateFromClient(e.sale),withoutfiscalization:e.withoutfiscalization,
  saleType:e.saleType,periodStart:e.periodStart,periodEnd:e.periodEnd};
  return this.http.put(`${this.resourceUrl}-cancel/${B0(e.sale)}`,c,{observe:"response"})}
```

| | |
|---|---|
| Path | `PUT /api/sales-cancel/{saleId}` (no `currentPosId`) |
| Body | `{reason:string, sale:{the full sale as read}, withoutfiscalization:bool, saleType:"NORMAL"|"ORDER"|"SUMMARY", periodStart?, periodEnd?}` |
| Answer | `200`, body = **the new negative sale** — exactly what blueprint §8.2 (4) observed on 2026-09-23: sale 8718 `changedStatus:"CANCELLED"`, every line `amount:-1`, `totalValue:-4600`, `modified:{id:8717,uuid}`, its own `invOrdNum`/`fic`/`logCis SUCCESS`; the original stays and is referenced |
| Then | if the original was `CLOSED` (paid), the SPA books the refund payment with `PUT /api/sales-pay` `{sale, payment{value, method, exchangeRate, extraUser, currency}}` on the negative sale. Whether this second call is REQUIRED for the negative sale to be `CLOSED`: **UNKNOWN** (8718's `salePays` was not recorded) |
| Roles | the button sits on the `sale` route (`ROLE_ADMIN|ROLE_SALE`); `ROLE_MODIFY_INVOICE` names the amend right on other routes (§5) |
| Not for dowiz | `withoutfiscalization:true` (discard) — only for a sale the tax authority never received |

The `reason` values the form offers were not extracted (the modal's select is data-driven); the control is
`fb.control(null, required)`.

### 4.2 Corrective (amend a sale, keeping it): `PUT /api/sales/{id}/{currentPosId}`

`update(e,c)` sends the edited **sale** (not the wrapper) to `/api/sales/{sale.id}/{currentPos.id}` after
`POST /api/sales/can-edit` (`checkIfCanEdit`). i18n `cantEdit`: "Correcting the invoice with this total … will
make all related invoices go to minus… fiscalization error…". The result is, by the enum, a sale with
`changedStatus:"MODIFIED"` (i18n "CORRECTED") and `modified` → the original (**hypothesis**: only `CANCELLED`
was ever observed on the wire; the detail wrapper's `saleModified[]` lists the chain — for 7783 it holds the
sale itself). `POST /api/sales/removeItems` `{sale, records, pointOfSale}` removes lines. The
`correctiveBySerialNumber:true` branch of the create (§1.3) issues a corrective **for an invoice that lives
outside ebills**, by its serial number and date — not dowiz's case.

**For a dowiz refund the right primitive is 4.1**, whole-sale, mapped to the FSM's `Refunding →
CompensatedRefund` — a partial refund (some lines) has no observed wire shape.

---

## 5. Which user

- **SPA routes** (`chunk-YQFNNNTR.js` route table): `sale` (list), `sale/new`, `sale/draft`, `sale/sale-view`,
  `sale/e-invoices`, `sale/self-care-invoices` — all `authorities:[Ie.ADMIN, Ie.SALE]`. The roles enum
  (`chunk-DNTWM2S7.js`): `ROLE_ADMIN, ROLE_SALE, ROLE_BUYER, ROLE_REPORT, ROLE_MANAGEMENT,
  ROLE_MANAGEMENT_GENERAL, ROLE_CONFIGURATION, ROLE_MODIFY_INVOICE, ROLE_POS_FULL, ROLE_POS_DELTA,
  ROLE_POS_BASIC, ROLE_TRANSFER_NOTE, ROLE_TRANSPORTER_INVOICE, ROLE_CLIENT, ROLE_USER, ROLE_SUPPLIER,
  ROLE_TRANSPORTER, ROLE_ITEM_IN_SALE, ROLE_ITEM_IN_WAREHOUSE, ROLE_WAREHOUSE, ROLE_CASH_REGISTER, …`.
  `ROLE_POS_FULL|DELTA|BASIC` choose the POS panel variant (stored as `posType`, compared literally:
  `n.postype==="ROLE_POS_FULL"`); they are not what the sale route asks for. **`ROLE_SALE` is the narrowest
  role the SPA needs to create a sale**; the SERVER's `@PreAuthorize` on `POST /api/sales` is not in the bundle
  (**UNKNOWN**, to be tested with the sender's own user — a `403 problem+json` is the answer, and it creates nothing).
- **A fiscal operator:** the user must have an `extraUser` with an `operatorCode` the tax authority knows
  (blueprint §1.2: the operator's own account has one). The SPA's `checkOperatore` verifies it via
  `GET api/extra-users/check`; the created sale's `extraUser` is the session's (§1.4). A user without one is
  what the i18n `operatorNotFound` warning is about — and the server would then fiscalise under… **UNKNOWN**.
- **The POS and the day:** `pointOfSale.id` in the body must be a POS the user may sell on (`GET /api/valid-pos`
  lists them; `GET /api/current-pos` was `403` to `ROLE_ADMIN`, blueprint §1.3); the day's cash-balance
  `INITIALIZE` (§1.5 step 2, client-side check); and the shift must be open — the login itself answers with an
  `X-Shift-Error` header out of shift (`chunk-2U2CKYSY.js`: `t.headers.has("X-Shift-Error")?this.outOfShiftError=!0`),
  which `judge.rs` already surfaces as a named refusal.
- The operator creates that user in the UI (`POST /api/extra-users` is a write; blueprint §5.5) with
  `ROLE_SALE` and the same operator code, and the sender's credentials are that user's, in the venue's
  `ebills` image (blueprint §8.1) — never `/root/.ebills_account`.

---

## 6. Where the existing client stands, and what the write path changes

- `client.rs::Path` has no variant for `/api/sales` POST and `Wire` has one POST constructor (`login`);
  `allowed(url, post=true)` admits only `/api/authentication`; `fetch.rs::send` refuses anything
  `sendable()` rejects. **That is the invariant the blueprint §6.2 asked for, and it is exactly what a sender
  must extend deliberately**: one new constructor, `Wire::create_sale(body, &Session)`, one new arm in
  `allowed` for `POST /api/sales` (exact path, no sub-path), and NOTHING for `sales-cancel` / `refiscalize` /
  `sales-pay` until §4 is wanted and separately approved. The CSRF header is the login's code path
  (`x-xsrf-token` from the jar).
- `wire::Sale` has no `notes`, `qrCode`; `wire::ItemInSale` has no `id` — the send needs the item's `id` (and
  the whole row, §1.4.1). `wire::Sale.client`/`extraUser` deliberately drop personal fields; the sender must
  pass the default-client object through WITHOUT keeping it (read it from `/api/sales/{id}` of a previous
  sale or from the create's own answer; the block holds a `name`).
- `map.rs::to_order` is the READ direction; the WRITE direction is its inverse and the same money law:
  whole lek in, doubles out, `Σ lines == totalValue`, VAT derived per line with the four divisors.
- The response is a `Sale` the existing `parse_detail`-style parser can read once wrapped — but the CREATE
  answer is the bare `sale`, not the `{sale:…}` wrapper (§1.1). `judge.rs` must accept a `200` with a body
  whose `logCis[0].status` is `ERROR` as "created, not fiscalised" — a state the read side refuses
  (`finished()`), and the write side must record.

---

## 7. Risks, and what only the first real send can tell

### 7.1 Risks

1. **Every send is an irreversible fiscal act.** There is no sandbox in evidence: one tenant, one POS, the live
   business; the `fic` is the tax authority's receipt. A wrong amount is corrected only by another fiscal act (§4).
2. **A timeout is an unknown invoice.** The POST fiscalises synchronously; a Worker that times out after the
   server committed has issued an invoice it does not know about. §3 (c): reconcile before any retry.
3. **No server de-duplication is in evidence** (§3). The 700 ms guard is the SPA's whole defence.
4. **Money is doubles on the wire** with 10-digit VAT fractions; the sender must produce exactly the SPA's
   arithmetic or learn whether the server recomputes (§7.2 #5).
5. **Cash limits are law:** > 500,000 lek cash to an individual or > 100,000 to a NIPT client is refused
   client-side; the server's answer is unknown. dowiz orders are far below, but the check belongs in the sender.
6. **The table flag.** At this venue the SPA always attaches `saleUnit`; a table-less `NORMAL` sale is unproven
   on this POS (§1.6, #1).
7. **The operator.** The session user becomes the fiscal operator of every sale; a shared or wrong user
   misattributes every invoice.
8. **Daily preconditions** (cash-balance INITIALIZE, open shift, valid certificate — the business's
   `electonicCert` expiry is checked at panel load) turn a working sender into a `WEBSERVICEERROR` stream on
   a morning nobody opened the till. The sender must read the floor/shift state first and refuse loudly, and
   must never call `refiscalize`/`close-shift` to "fix" it.
9. **`WEBSERVICEERROR` is a created sale**, listed and numbered; dowiz must show it as "fiscalisation pending"
   with the fault text, never as "not sent", and must not send it again.
10. **The receipt.** ebills prints/downloads after create; a dowiz sale has no printer. Whether an unprinted
    fiscal invoice is a compliance problem for the venue (the customer must be able to get the receipt/QR) is
    an operator question, not a wire one: the `qrCode` URL comes back in the sale and can be shown on the
    order page.
11. **Personal data on the write path:** the `client` block and `pointOfSale.address` travel in the body; the
    `logCis` in the answer embeds the sale; the sender logs nothing of either (the read side's rule, `fetch.rs`).
12. **`Accept-Language` and `notes`:** a `notes` marker prints on the receipt (**hypothesis**); the operator
    should see one before it is chosen.

### 7.2 Only the first real send can tell

1. Whether `saleType:"NORMAL"` **without `saleUnit`** is accepted on a POS configured with tables, or whether a
   dedicated `DELIVERY` sale unit + `includeClosingTheTable:true` (7783's shape) is required.
2. Whether the server **honours a client `uuid`** and **refuses a duplicate**, or overwrites it.
3. Whether `itemInSale:{id}` suffices or the whole row must be echoed; whether `client:{id:1}` suffices.
4. Whether `extraUser:null` is accepted for a `ROLE_SALE` user (the operator from the session) and which
   `operatorCode` the sale then carries.
5. Whether the server **recomputes** `totalValue`/`totalVatAmount`/`priceWithoutVat` or **validates** them
   (and to how many digits), i.e. whether whole-lek inputs with `toFixed(10)` VAT are enough.
6. Whether `invoiceTypeId:null` becomes `388`, and `timestamp` omitted becomes server time (UTC) as on 7783.
7. Whether a `CARD` `NORMAL` sale is `CLOSED` on create without `PUT /api/sales-pay` (measured only for `CASH`).
8. The exact `problem+json`/`errorKey` vocabulary of a refused create (`negInventory` is the one the SPA names).
9. What `logCis[0].faultString` looks like on a real fault, whether `invOrdNum` is assigned in that state, and
   how long `refiscalize` remains possible (the 48-hour rule, if the server applies it).
10. Whether `POST /api/sales` needs `ROLE_SALE` only, and whether `ROLE_SALE` alone can also `GET /api/sales`
    (the poller's open question, blueprint §8.3).
11. Whether the CSRF filter guards `/api/sales` as it guards `/api/authentication` (assumed yes; a `403` with
    the CSRF message creates nothing).
12. Whether the cancel's follow-up `PUT /api/sales-pay` is required for the negative sale to net the drawer.

The right first send is **one 1-line, 1-lek-scale real item** (the venue's cheapest `isService` item), CASH,
at a time the operator is at the till to void it if wrong — and the answer, the later GET and the list row
are recorded in this document's successor before a second send is written.
