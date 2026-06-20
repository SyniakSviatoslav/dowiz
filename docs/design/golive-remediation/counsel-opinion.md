# Counsel Opinion — Go/No-Go Pre-Launch Remediation

> Advisory. Aesthetic/strategic notes are non-blocking. ETHICAL-STOPs below are friction, not verdicts: each names a grounded red line, pauses council, and demands a recorded human decision. The conscious human is always final.
> Companion to `docs/design/golive-remediation/proposal.md`. Grounded against: `apps/api/src/lib/anonymizer/index.ts`, `apps/api/src/routes/owner/gdpr.ts`, `apps/api/src/routes/public/fallback-config.ts`, `apps/web/src/pages/client/CheckoutPage.tsx`.

This is a careful proposal. It corrects its own audit before designing (§0), it preserves the human-in-the-loop invariants, and it wins #2 by shortening holds rather than spending the scarce connection budget — restraint over force, which is the aesthetically and ethically right move. The lenses below only flag where the design still, quietly, risks telling a customer something that isn't quite true.

---

## 1. Reasoning per lens (only what's load-bearing)

### Honesty / consent — #5 privacy notice (the heart of the review)

The notice content is well-shaped: what / why / who / retention / removal, warm tone, no legalese, placed at the point of consent without added friction. The "who sees it — this restaurant and its courier, not other restaurants" line is **truthful** to the RLS tenant-isolation model and is the kind of sentence that earns trust. Good.

But "honest" means the displayed words match what the system *actually does*, and two seams need pinning before this notice can be called honest:

1. **Retention number vs. anonymizer reality.** The notice will render `retention_days` as "deri në {{days}} ditë". The anonymizer's retention sweep (`anonymizer/index.ts:247-271`) deletes customers/orders whose `created_at < now() - retention_days`. So the number is *directionally* honest. The subtlety the notice must not overstate: anonymization is **not deletion** — it nulls `name`, randomizes `phone`, nulls `delivery_address`/`client_ip_hash`, purges the avatar, and **keeps the row** (`:133-141`, `:210-217`). That is the correct, defensible posture ("anonymize-not-delete" is an explicit red line). The honesty risk is only if the sq/en copy says "fshijmë të dhënat tuaja" / "we delete your data" when the system anonymizes-in-place. The copy should say *"we remove the data that identifies you"* — true to the code — not "we delete everything." This is a wording constraint on the copywriter, gated by the proof: the E2E should assert the rendered string, and a human should read the final sq string against `anonymizeCustomer`'s actual field list.

2. **Retention requires the sweep to actually run.** Displaying "up to {{days}} days" is a *promise*. It is only honest if the retention sweep is scheduled and firing in prod. If `retention_days=365` is shown but no scheduled job ever calls `anonymize({scope:'retention'})`, the notice promises a deletion that never happens — data lives forever behind an honest-looking number. The proposal does not list "verify the retention sweep is scheduled and runs" as a Go gate. **This is the gap.** (See ETHICAL-STOP-1.)

3. **"Contact X to remove" — is it enough for dignity?** Here the ground truth matters: `routes/owner/gdpr.ts` has **no customer-facing erasure endpoint**. Every route is `requireRole(['owner'])`. The only way a customer's data gets erased on request is: customer asks the owner → owner manually opens admin → owner creates a `gdpr_erasure_request`. There is no self-service path, and nothing obligates the owner to act. For a **pilot**, "ask the restaurant" is an *acceptable, honest* answer *if the notice doesn't imply more than that* — it must not say "click here to delete," because there is no such button for the customer. It must say, plainly, "to remove your data, ask the restaurant" and ideally give the contact (reuse `location.phone`, already surfaced for #4). Dignity at pilot scale is satisfied by *truth about the mechanism*, not by building self-service. The dignity failure would be implying a power the customer doesn't have. (See non-blocking note N1 for the post-pilot direction.)

4. **Albanian cultural context.** Informal cash-delivery norms mean a heavy GDPR-style consent wall would feel alien and distrust-inducing — out of place for a neighborhood sushi order. The proposal's choice (one warm short paragraph, inline, sq-primary, not a modal) reads *right* for that context: it informs without performing legalism. This respects the earlier steel-man. One caution: warmth must not slide into vagueness that hides the retention fact — warm and complete, not warm instead of complete.

### Care / harm — #4 checkout failure fallback

Confirmed dignifying and correctly scoped. On failure the design (a) keeps the existing toast, (b) surfaces "call the restaurant" via `location.phone` from the existing public `fallback-config` route, (c) **preserves the cart** (`clearCart()` only runs on success — verified at `CheckoutPage.tsx:431`), and (d) the fallback fetch itself fail-soft to the generic toast. The customer is never stranded and the failure is never hidden. The dev-mock containment is the *more* important honesty fix here: today `isDevMode()` is a `sessionStorage` flag (`CheckoutPage.tsx:439`) that a real session could carry, so a **failed real order can render fake success and navigate to `o_mock_123`**. That is a customer being told "your order succeeded" when it failed — a dignity/honesty violation. Gating it behind compile-time `import.meta.env.DEV` makes it dead code in prod. Strongly endorse; this is the single most honesty-positive line in the whole remediation. (See ETHICAL-STOP-2 — narrow.)

### Justice / stakeholders — the data task (durres-sushi → staging)

No dignity or consent issue. Menu/location/branding is the vendor's *business* data, structurally PII-free, and the chosen mechanism reads it through the **same public surface customers already see** — so nothing is exposed on staging that isn't already public. The transform even scrubs the real phone to a placeholder (`§10-DATA.2`), which is courtesy beyond the requirement. The verification asserts `customers=0 / orders=0` on the demo tenant. This is clean. One small justice note: durres-sushi is a *real, identifiable* business; their menu and prices sitting on a staging box is low-stakes but non-zero — a courtesy heads-up to the owner ("we're using your live menu to validate staging") costs nothing and respects that it's their livelihood on display. Non-blocking (N2).

### Dignity / autonomy — courier and owner

No new surveillance, no autobahn, no field added (`§1 non-goals`). #1's owner Telegram "auto-cancelled" message is *dignifying to the owner* — closing a notification gap so they aren't left wondering. The courier surface is untouched. Nothing here erodes the courier-completes or human-in-loop lines. Good.

### Long horizon / strategy

This remediation serves the launch trigger (first real paid order placed with honesty), not polish. The restraint choices (no new queue, no pool bump, no schema change, forward-only/revertible) all reduce what we'll regret in a year. The one second-order item to name: shipping an honest-looking retention promise (#5) *creates an obligation* to keep the sweep running. If the sweep silently dies post-launch, the notice becomes a standing false statement to every customer — the regret compounds with every order. Strategy and honesty converge on the same fix: make the sweep observable (heartbeat/audit-log count), the way #1 makes the timeout consumer observable.

### Aesthetics / integrity

Conceptually coherent: "schema rich, runtime minimal," reads-outside-writes, server-authoritative snapshot, transactional outbox reused rather than reinvented. The elegance here is *honest* elegance (fewer moving parts → fewer failure modes → less customer harm), not seductive elegance. The inline-not-modal privacy notice is design-language-consistent with a low-friction checkout. No notes.

### Epistemic — what we're not asking

The unexamined load-bearing assumption: **that "retention_days" displayed at checkout is the same promise the retention sweep enforces.** The number, the sweep schedule, and the copy are three separate things that must agree. The proposal treats the number as a render task; it is actually a *truth-coherence* task across DB config, a scheduled job, and human-written copy.

---

## 2. ETHICAL-STOPs (grounded red lines only)

**ETHICAL-STOP-1 — "server-authoritative / UI-tells-the-truth" + "anonymize-not-delete."**
The #5 notice displays `retention_days` as a retention/removal promise to the customer. This is only honest if the retention anonymization sweep is **actually scheduled and firing in production**. Grounded: `anonymizer/index.ts:247-271` only acts when invoked with `scope:'retention'`; the proposal lists no Go gate verifying that invocation is scheduled and live. Shipping the notice without that verification means the checkout UI states a removal timeline the system does not enforce — the server is not authoritative over its own promise.
*Required recorded human decision before Go:* either (a) verify the retention sweep is scheduled and observable in prod (add it to the §9 operability table with a signal, mirroring how #1 verifies the timeout consumer), **or** (b) soften the copy to describe what is guaranteed ("we keep your data only as long as needed for your orders, and remove identifying details on request") so the displayed number is not a hard promise the runtime can't keep. Friction, not a block — the human chooses (a) or (b) and records it.

**ETHICAL-STOP-2 — "UI tells the truth / soft-confirm not a pretense of success" (narrow, already addressed — recording for the record).**
The current `isDevMode()` path can show a real customer fake order success on a real failure (`CheckoutPage.tsx:439`). The proposal already fixes this via compile-time `import.meta.env.DEV` gating. This STOP is satisfied *by the proposal as written*; it lifts the moment the E2E proof (prod build → forced failure → no `o_mock_123` nav) is green. Named only so the human approval record shows the false-success path was closed deliberately, not incidentally.

No other red line is crossed. #1/#2/#3/#6 and the data task raise no ETHICAL-STOP.

---

## 3. Non-blocking aesthetic / strategic advice

- **N1 (strategy, post-pilot):** "Ask the restaurant to remove your data" is honest for a pilot but leans entirely on owner goodwill, with no obligation and no customer visibility into status. Fine to ship; queue a post-pilot item for a customer-initiated erasure request (even just a "request removal" that creates the `gdpr_erasure_request` the owner already processes) so dignity scales past the pilot's trust-by-acquaintance.
- **N2 (justice, courtesy):** Give durres-sushi's owner a heads-up that their live menu/branding is being mirrored to staging for validation. Costs nothing; respects that it's their business on display.
- **N3 (aesthetics/honesty, copy):** Constrain the #5 sq/en copy to "remove the data that identifies you," matching the anonymizer's null-and-randomize behavior — never "delete everything," which the code does not do.
- **N4 (operability):** Make the retention sweep observable the same way #1 makes the timeout consumer observable (heartbeat or `anonymization_audit_log` count over a window). An honest promise needs a pulse you can watch.

---

## 4. Steel-man of a rejected option

**#1 Option B (cron-sweep reconciliation), rejected/deferred — the strongest case for promoting it *now*:** The whole #5 honesty argument above (ETHICAL-STOP-1) hinges on a periodic, self-healing sweep being a thing this system reliably runs. The retention anonymizer *is itself a periodic reconciliation sweep*. If we are going to build and trust one safety-net poller for data-removal anyway, the marginal cost of also trusting Option B's cancel-reconciliation poller drops sharply — they're the same operational shape (scheduled job, guarded UPDATE, idempotent, observable via audit count). The proposal rejects B as "not required to pass Go," but a system whose *core privacy promise* already depends on a reliable scheduled sweep has arguably already paid the conceptual cost of that pattern; adding B would make the timeout-cancel path self-healing instead of dependent on per-order job delivery (the exact failure the stale `assignments.ts` debt comment fears). Reasonable to still defer B — but the honest reason to defer is "A verified live is enough," not "we don't do scheduled sweeps here," because we manifestly do.

---

## 5. The open question no one asked

The notice tells the customer "this restaurant and its courier" can see their data — true under RLS. But the **courier** is a person who will hold a stranger's name, phone, and home address (and possibly a door photo) on their device, for some window, to do the delivery. Who tells the *courier* what their obligation to that data is — and when does *their* copy of it stop being theirs to see? We've designed honesty toward the customer about the restaurant's holding of data; we have said nothing about the human at the end of the chain who actually arrives at the door. That asymmetry is worth one sentence of thought before a real customer's first address rides in a real courier's pocket.

---

## Re-examine (раунд 2)

> Регресія проти раунду 1. Перечитав поточні `proposal.md` + `resolution.md`. Заземлив три несучі факти проти живого дерева: retention-sweep emit, DEV-gate, courier-deferral. Нон-блокінг по осі; ETHICAL-STOP лишається тертям, не вироком. Людина — фінал.

### Статус ETHICAL-STOP-1 — RESOLVED (з одним уточненням заземлення)

Резолюція #5(c) додала саме той Go-gate, якого бракувало: цільове середовище мусить показати рядок `anonymization_audit_log` зі `scope='retention'` у вікні АБО свіжий heartbeat `anonymizer-retention`, інакше copy пом'якшується (гілка b). Це закриває розрив «число-як-обіцянка vs рантайм-що-не-діє».

Перевірив проти коду, а не лише проти тексту резолюції:
- `anonymizer-retention.ts:22-27` — `boss.work` + `boss.createQueue` + `boss.schedule` з cron-замовчуванням `0 3 * * *`. Розклад реальний.
- `anonymizer-retention.ts:60-65` — викликає `anonymize({ scope: 'retention', ... })` по кожній локації. Інвокація, якої я боявся що нема, **є**.
- `anonymizer/index.ts:285-289` — `insertAuditLog` пише рядок зі `scope`. Тобто Go-gate-сигнал (`scope='retention'` у вікні) **справді записується кожним проходом** — gate спостережний, не вигаданий.

Чи робить це displayed number чесним? **Так, умовно** — за двох важливих застережень, які мушу записати чесно, бо вони ослаблюють силу gate як одноразової перевірки:

1. **Audit-row пишеться лише коли є що анонімізувати.** `findExpiredCustomers/Orders` (`:247-271`) повертають рядки тільки для записів, старших за `retention_days`. На свіжому пілоті (день 1, `retention_days=365`) НЕ буде жодного простроченого запису ще ~рік — отже sweep відпрацює, але `anonymization_audit_log` зі `scope='retention'` **не отримає рядка**, бо анонімізувати нічого. Go-gate у формі «рядок in-window» хибно-червоний у нормальному пілоті. Тому **heartbeat-гілка gate (свіжий `anonymizer-retention` heartbeat) — не альтернатива, а ОСНОВНИЙ сигнал для пілота**; audit-row стає валідним сигналом лише коли вікно retention уже народило прострочені дані. Це не нова діра — це уточнення, який із двох ABO-сигналів несе вагу на старті. Якщо людина вибере перевіряти лише audit-row, вона перевірить порожнечу й подумає що sweep мертвий (хибна тривога) АБО, гірше, послабить gate і вирішить що copy брехлива коли вона чесна. Рекомендація (нон-блокінг, N4-розширення): для пілота gate = «schedule зареєстровано + heartbeat свіжий»; «audit-row in-window» вмикається як сигнал лише після першого вікна retention.

2. **Cron `0 3 * * *` означає, що «firing» спостерігається раз на добу.** Між деплоєм і 03:00 heartbeat може бути ще не «свіжим» у сенсі останнього прогону. Це операційний нюанс перевірки, не етична діра — людина просто має знати, що gate зеленіє після першого нічного вікна, або форснути ручний прогін перед Go. Записую, щоб ніхто не читав «gate червоний» як «sweep зламаний».

Copy-constraint (anonymize-not-delete, sq «heqim të dhënat që ju identifikojnë», «contact the restaurant», без self-service-кнопки) — **достатній і точний**. Звірив із `anonymizer/index.ts:133-141` (phone→`anon_`+uuid, name→NULL, marketing→false, рядок лишається) і `:210-217` (delivery_address→NULL, client_ip_hash→NULL). Слова збігаються з поведінкою. Резолюція додала людську звірку фінального sq-рядка проти списку полів `anonymizeCustomer` перед Go (#5a) — це і є та гарантія, якої я просив. Threat-model #10 («never delete everything») закриває це тестом. Resolved.

### Статус ETHICAL-STOP-2 — RESOLVED (закриває fake-success назавжди, з однією перевіркою-на-merge)

`CheckoutPage.tsx:42` підтверджено: `isDevMode()` = runtime `sessionStorage.getItem('dos_dev')==='1'` — флаг, який реальна сесія може нести. Резолюція #4 ставить `if (import.meta.env.DEV && isDevMode())`. `import.meta.env.DEV` — статично `false` у prod-збірці, Vite dead-strip-ить гілку, тому рядок `o_mock_123` **фізично не існує в prod-бандлі** — це compile-time межа, не runtime. Це закриває fake-success назавжди для цього шляху, бо назавжди = «не компілюється в prod», а не «вимкнено прапором». Сильніше за runtime-перевірку.

Чому «з перевіркою»: «назавжди» тримається рівно настільки, наскільки grep чистий. Threat-model #9 (grep prod-бандла на `o_mock_123` → відсутній) — це і є доказ, що dead-strip спрацював, а не лише що джерело виглядає правильно. Доти, доки цей grep зелений у proof, STOP-2 знятий. Резолюція також зафіксувала єдиність споживача `isDevMode()` (`:439`) і що сусідні `dos_dev`/`?dev=true` гейти (`CourierRoutes.tsx:24`, `AdminRoutes.tsx:57`) — окремі шляхи, вже compile-gated. Жодного нового prod-backdoor цей фікс не вводить. Resolved — лифт на зеленому prod-bundle grep.

### Чи фікси породили НОВУ етичну/естетичну діру? — Ні (одне спостереження)

- #2 batching, #1 sweep, #3 health-down, #6 ESM — жоден не торкається consent/dignity/surveillance-поверхонь; нових червоних ліній не перетинають. #1 owner-Telegram «auto-cancelled» лишається dignifying. #3 прибирає неавтентифікований витік pg-internals — це етично-позитивно (менша recon-поверхня), не діра.
- Єдине нове естетично-етичне спостереження (нон-блокінг): резолюція тепер несе **дві обіцянки, що залежать від reliable scheduled sweep** — retention-anonymize (#5) і timeout-cancel-reconciliation (#1e). Це не діра — це підтвердження мого раунд-1 steel-man (Option B): система вже платить концептуальну ціну патерну «надійний планований sweep», тож #1e як обов'язковий — узгоджено й елегантно. R8 у резолюції чесно іменує standing-obligation retention-промісу й вішає на нього спостережний сигнал. Концептуальна цілісність ціла.

### Відкрите питання N5 (кур'єр тримає чуже ім'я+телефон+адресу) — deferred-post-pilot ПРИЙНЯТНО для пілота, з тертям

Резолюція позначила N5 «Deferred — post-pilot, flagged» (Owner). Чи прийнятно? **Для пілота — так**, бо: (1) це не перетин заземленої червоної лінії — жодна з ліній (anonymize-not-delete, нуль-PII-у-ШІ, claim-check, сервер-авторитетний, GPS-сміття-відкинуто) не говорить про обов'язок кур'єра щодо даних; це етичний *горизонт*, не *стіна*; (2) пілот — trust-by-acquaintance, кур'єр відомий оператору особисто, тож відсутність формальної політики не наражає реальну людину негайно; (3) deferral *записаний і власник його тримає* — це і є «людське рішення зафіксовано», чого вимагає тертя.

Тертя, яке лишаю (нон-блокінг, не ескалюю до STOP): N5 — це асиметрія, не борг. Ми побудували честність кур'єр→немає (кур'єру ніхто не сказав, скільки часу адреса лишається його бачити; чи зникає вона з пристрою після доставки). Перш ніж перша реальна адреса поїде в реальній кишені, варто *одне речення* політики кур'єру — навіть усне на пілоті. Це не блокує Go; це те, про що пошкодуємо за рік, якщо кишеня виявиться постійним архівом. Лишаю власнику як свідомий, записаний відклад — саме так, як належить.

### Фінальний нон-блокінг висновок

Обидва ETHICAL-STOP **RESOLVED** (STOP-2 лифтиться на зеленому prod-bundle grep #9; STOP-1 — на зеленому retention-pulse gate #11, читаючи heartbeat як основний сигнал для пілота, а audit-row — як сигнал що дозріває з першим вікном retention). Жодної нової заземленої червоної лінії фікси не перетнули. N5 коректно відкладено зі записаним рішенням власника. Естетична цілісність зросла, не впала: «надійний планований sweep» тепер єдиний несучий патерн для обох обіцянок.

З осі ДОБРО·КРАСА·МУДРІСТЬ рада може йти на людський фінал. Counsel не блокує. Єдине, що прошу людину свідомо прочитати перед записом рішення: уточнення №1 вище (heartbeat ≠ audit-row на свіжому пілоті) — щоб зелений gate не сплутали з порожнім, а порожній — зі зламаним.
