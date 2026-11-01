# R-EINFILE-PRIOR-ART — хто вже робить те, що einfile

Статус: 2026-11-01, лейн W-MG-PRIOR. Вебдослідження; без збірок, без git, без підлеглих агентів. Посилання `[n]` ведуть до §8. Позначка `[п]` біля джерела: бачив лише у видачі пошуку, сторінки не відкривав. **UNVERIFIED** = джерела немає або воно мовчить. **[О]** = моя оцінка, не факт.

Опис einfile взято з чернетки `/root/lanes/w-repo/README.md` і нотаток лейнів станом на 2026-11-01. Самі твердження про код цей лейн не запускав.

## 0. Підсумок

1. **Слова `first`, `unique`, `only`, `nobody else` захистити не можна.** Найсильніші причини: `@nkwib/ledger` [47] створено 2026-07-20, раніше за ціль оператора від 2026-10-30: підписані ланцюги на пристрій, детерміноване злиття, іменовані конфлікти. У einfile немає жодної публічної дати пріоритету: репозиторій не опубліковано.
2. **§6 R-MAINGOAL-A помиляється у своєму висновку.** «Названий конфлікт» є в Git, Dolt, DoltLite, SQLite session, `ledger` [37, 15, 18, 23, 47]. Файл із власним wasm-читачем є в F3 і AnyBlox, докази з власним wasm-верифікатором є в TrustMee [50, 51, 52]. Деталі в §2.
3. **Рядок README `Unlike SQLite, two copies merge` має найближчу загрозу.** DoltLite: форк SQLite, один файл, гілки, злиття, таблиці конфліктів, `file://` remote, бета 0.50.0 від 2026-08-31 [17, 18]. Підпису на прочитаних сторінках немає, але батьківський Dolt підписує коміти GPG з 2024 року [16].
4. **Кожна окрема властивість уже є в когось.** ML-DSA-65 і OpenTimestamps разом в одному файлі-документі: `cdx-core` [49]. ML-DSA-65 на ланцюгу версій: `aion-context` [48]. Підпис, що переживає стирання: Matrix [41]; навантаження окремо від журналу: p2panda і Bamboo [7, 66п]. Сіль проти перебору стертого значення: SD-JWT [62].
5. **Захищається лише комбінація з датою й межею пошуку.** Точне речення в §4. Воно стає правдою лише після трьох умов там само.
6. **Сьогодні сама цікава властивість не виконується для чужої людини.** Чернетка README: читачі рахують печатки, не перевіряють, `checked=0`, `holds` хибне. «Перевіряється скриптом» поки означає цілісність без підпису.
7. У §3 названо, що кожен суперник робить краще; у §5 загрози; у §6 факти для рецензента NLnet. Заявки тут немає: NLnet чекає слів заявника, а його форма вимагає розкривати генеративний ШІ [59].

## 1. Таблиця властивостей

Легенда: **Т** так; **Ч** частково; **Н** джерело каже «ні» або опублікована структура це виключає; **—** прочитані сторінки мовчать, це UNVERIFIED і не означає «ні».

Колонки: **Ф** один файл; **З** злиття без сервера; **К** конфлікт названо, а не вирішено мовчки; **П** підписана історія на пристрій/автора; **Р** підписаний зріз історії; **С** підписане стирання, що не ламає історію; **В** файл несе специфікацію і пісочничний читач; **Д** чужа людина перевіряє скриптом без бібліотеки; **Я** часовий якір через Bitcoin; **М** ML-DSA; **Б** працює без сервера як мета.

| Система | Ф | З | К | П | Р | С | В | Д | Я | М | Б | Джерела і примітка |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **einfile** (чернетка, не опублікована) | Т | Ч | Т | Ч | Ч | Ч | Ч | Ч | Ч | Ч | Ч | README: злиття M2-min; Р/С/П без постачальника підпису не перевіряються; В лише архів; Д рахує печатки; Я: формат є, адаптерів немає; транспорту немає |
| Secure Scuttlebutt | — | Н | — | Т | — | — | — | — | — | Н | — | [1] фід одного ключа, `.sig.ed25519` |
| Hypercore / Pear | — | Ч | — | Т | Ч | — | — | — | — | — | Ч | [2, 3п] одне ядро один письменник; Autobase лінеаризує, кворум індексаторів; `fork` у підписаному корисному навантаженні |
| Willow | Ч | Т | Н | Ч | Ч | Ч | — | — | — | — | Т | [4, 5] LWW за часом, потім дайджест; Drop один байтовий рядок, статус Proposal; токени на запис, ланцюга немає; видалення префіксом |
| Earthstar | — | Т | Ч | Ч | — | Ч | — | — | — | — | Ч | [6] ревізії кількох авторів на шлях; 11.0.0 лише бета [п] |
| p2panda | — | Ч | — | Т | Т | Ч | — | — | — | Н | — | [7] Ed25519, BLAKE3, журнали одного автора; prune-розширення заголовка; корисне навантаження окремо видаляється |
| Iroh (iroh-docs) | Ч | Т | Ч | Ч | — | — | — | — | — | — | Ч | [8, 9п] два ключі на запис, ланцюга немає; 0.101 у 2026; redb лише локальне сховище |
| Automerge | Т | Т | Ч | Н | — | — | Н | — | — | — | Т | [10, 11, 12, 13] `getConflicts` і детермінований переможець; підписів у форматі немає; авторство розпочато 07.2026; Keyhive alpha |
| Yjs | Ч | Т | — | — | — | — | — | — | — | — | Т | [14] оновлення комутативні, `mergeUpdates` без документа |
| Dolt | — | Ч | Т | Ч | — | — | — | — | — | — | Ч | [15, 16] таблиці `dolt_conflicts_*`; GPG-підпис коміта |
| DoltLite | Т | Т | Т | — | — | — | — | — | — | — | Ч | [17, 18] форк SQLite, `file://` remote, коміт блокується до вирішення |
| immudb | — | — | — | Ч | — | — | — | Ч | — | — | Ч | [20] клієнт перевіряє, бібліотека Go або сервіс; BSL 1.1 |
| Datomic | — | — | — | — | — | Ч | — | — | — | — | — | [21, 22] excision лишає постійний запис; про підпис нічого |
| SQLite + session | Т | Ч | Т | Н | Н | — | Н | Ч | — | Н | Т | [23, 24] `xConflict`; журнал/WAL допоміжні; формат описано публічно, але поза файлом; потрібна сесія, що записала зміни |
| cr-sqlite | Т | Т | Н | — | — | — | — | — | — | — | — | [25] LWW; push 2026-08-10, релізів на сторінці немає |
| Sigstore / Rekor v2 | Н | Н | Н | Ч | Н | — | Н | Ч | — | Ч | Н | [26, 27, 28п] центральний журнал; щорічні інстанси; PQ-записів у публічному журналі не планують |
| CT tlog + свідки | Н | Н | Н | Ч | — | — | Н | Ч | — | Ч | Н | [29, 30] `Witnesses SHOULD use ML-DSA-44`, докази офлайн |
| OpenTimestamps | Ч | Н | Н | Н | — | — | Н | Ч | Т | — | Ч | [31] окремий `.ots`; потрібна нода Bitcoin, pruned досить |
| Ditto | — | Т | — | Ч | — | — | — | — | — | — | Ч | [33п, 34п] P-256 на пристрій для mTLS; підпису на документ не знайдено; сертифікати від служби ідентичності |
| PowerSync | Ч | Н | — | Н | — | — | — | — | — | — | Н | [35] протокол через Service; контрольні суми некриптографічні |
| Git | Ч | Т | Т | Ч | — | — | — | Ч | Ч | — | Т | [36, 37, 32] bundle один файл, запис у нього не підтримується; Я через OTS-обгортку над `gpg.program` |
| Fossil | Т | Т | — | Ч | — | Ч | — | — | — | — | Т | [38, 39п, 40] репозиторій один файл; підпис вимкнено за замовчуванням і Fossil його змістовно не перевіряє; shun локальний |
| Matrix | Н | Ч | Н | Ч | — | Ч | — | — | — | — | Н | [41] підпис над редагованою формою, `hashes` лишається; state resolution вирішує сам |
| Nostr | Н | Н | Н | Ч | — | Ч | — | — | Ч | — | Н | [42, 43] NIP-09 лише прохання; NIP-03 позначено unrecommended |
| IPFS | Ч | — | — | — | — | — | — | Ч | — | — | Ч | [44] див. CAR; вміст адресовано хешем; мережу не перевіряв |
| PDF/A | Т | Н | Н | — | — | — | Н | — | — | — | Т | [45] JavaScript і запуск файлів заборонено; вбудовані шрифти обов'язкові |
| CAR | Т | Н | Н | Н | Н | Н | Н | Ч | Н | Н | Т | [44] підписів у специфікації немає; CID на блок |
| Parquet | Т | Н | Н | Ч | Н | Н | Н | — | Н | — | Т | [46] підпис футера лише в режимі шифрування з відкритим футером |
| *Додано:* `@nkwib/ledger` | — | Т | Т | Т | — | — | Н | Ч | — | Н | Т | [47] P-256, TypeScript, 0 зірок; сховище й транспорт на користувачі |
| `aion-context` | Ч | — | — | Т | — | — | — | — | — | Т | — | [48] `.aion`: ланцюг підписів версій; старі навантаження можуть жити зовні; K-of-N |
| `cdx-core` | Т | — | — | Ч | — | — | — | — | Т | Т | — | [49] ZIP `.cdx`; ML-DSA-65 і OTS вмикаються прапорцями; специфікація 0.1 |
| F3 | Т | Н | Н | — | — | — | Ч | — | — | — | Т | [50п] wasm-декодери у файлі даних; прототип |
| AnyBlox | Т | Н | Н | — | — | — | Ч | — | — | — | Т | [51п] `.any` із декодером; посилання на декодер за хешем |
| TrustMee | Ч | Н | Н | — | — | — | Ч | Ч | — | — | Н | [52] докази атестації несуть wasm-верифікатор |
| Radicle | Н | Т | Ч | Т | — | — | — | — | — | — | Ч | [53, 54] підписані refs на вузол, простір імен на вузол, COB як DAG |
| AT Protocol repo | Ч | Н | Н | Ч | Н | Н | — | Ч | — | — | Н | [55] експорт CAR; `prev` майже завжди null; видалення без сліду; ключ із DID |
| Jazz | — | Т | — | Т | — | — | — | — | — | — | Ч | [56п] кожна транзакція підписана Ed25519, журнали сесій; чи обов'язковий сервер, UNVERIFIED |
| Fireproof | Ч | Т | — | — | — | — | — | — | — | — | — | [57п] Merkle-годинник; архів у один CAR; підпис лише в бізнес-плані |
| TrustDB | Ч | — | — | Ч | — | — | — | Ч | Ч | Н | Н | [58] self-hosted; `.sproof` один файл доказу, не база; OTS-приймач; AGPL |

Чого в таблиці немає: жоден рядок не має Т або Ч одночасно в **К** і **В**. Жоден рядок із **Ф=Т** не має **П=Т**. Це спостереження про прочитані сторінки: 217 із 418 клітинок суперників це `—`, і одна така клітинка може його зламати. Найслабше місце: DoltLite, Ф=Т, П=`—`.

## 2. Де R-MAINGOAL-A і B неправі або застаріли

| Де | Твердження | Перевірка |
|---|---|---|
| A §6, кінець | «Те, чого немає ні в кого: названий конфлікт» | **Хибне.** Git тримає три стадії індексу [37]; Dolt і DoltLite таблиці конфліктів, DoltLite блокує коміт [15, 18]; SQLite викликає `xConflict` [23]; `ledger` має типізовані конфлікти і «No silent resolution» [47] |
| A §6, кінець | «...і самочитність» | **Хибне частково.** F3, AnyBlox, TrustMee несуть wasm [50, 51, 52]. Поєднання читача, тексту формату й підписаної історії я не знайшов |
| A §6, Dolt | «без підписів» | **Хибне:** `dolt commit -S`, допис 2024-09-16 [16] |
| A §6, Automerge | конфлікт не названо людині; формат читає лише бібліотека | **Частково хибне:** `getConflicts` повертає всіх, ключі = id операцій [10]; є байтова специфікація, draft [11]. Правильна частина: переможець автоматичний, наступний запис закриває конфлікт |
| A §6, Hypercore | злиття не його задача | Для ядра так, для стеку ні: Autobase [3п] |
| A §5.4 / §6, Fossil | PGP-підпис | Правда, але вимкнено за замовчуванням, і підтримувач пише, що Fossil нічого не може гарантувати за підписом [39п] |
| A §8.5 | Rekor і ML-DSA UNVERIFIED | **Закрито частково:** #425 закрито `not planned` 2026-06-09; в ньому ж: публічний журнал `won't support PQ-signed entries`; замість цього новий тип окремою задачею [27]. Свідчення: новішого за `soon` [26] не знайшов |
| A §8.6 | статус iroh-docs UNVERIFIED | **Закрито:** живий, 0.101 з copyright 2026 [9п], сторінка протоколу на місці [8]. 1.0 не підтверджено |
| A §8.7 | QLDB | Дату 2025-07-31 підтверджує сторінка AWS [64п]; відсутність незмінної історії в Aurora підтверджує блог AWS [65п] |
| A §8.8 | ліцензія immudb | **Закрито:** README: BSL 1.1 [20] |
| A §8.4 | OTS без ноди | **Лишається відкритим:** README вимагає ноду, про заголовки мовчить [31] |
| B §2 | Automerge SHA-256 PARTIALLY VERIFIED | **Закрито:** change hash = SHA256 [11] |
| B §2 | «git: усі три для тексту» | Перебільшено: bundle лише для читання [36], читач у файлі відсутній; підпис потребує GPG |
| B §2 | «Унікальна лише комбінація» | Виживає, але з умовами §4. Формулювання «я не знайшов покупця» лишається чесним |
| Пам'ять `bebop-main-goal` | «nobody has all three» | Для тексту близько до Git/Fossil; для KV-файлу в межах цього пошуку не спростовано; як публічне твердження непридатне |

Перевірено не все: Holochain, Ceramic, TradeLens, ринкові числа B я не перевіряв. Вони поза цією карткою.

## 3. Що близькі суперники роблять краще за einfile

**Git + bundle + OTS.** Підписані коміти, готова звичка, багато реалізацій [О]. OTS вписується в `git commit -S` без змін для клієнтів без OTS [32]. Це відповідь на питання «чому не Git»: відмінність einfile має бути названа в термінах стирання, KV і читача, не злиття.

**Fossil.** Один файл уже сьогодні, зовсім не теоретично; формат артефактів задуманий на довго; shun вирішує повернення стертого при синку [38, 40].

**DoltLite і Dolt.** SQL, 100 % sqllogictest, 99.46 % TCL-тестів SQLite, формат без ламань 57 релізів, GUI, DoltHub [17]. Конфлікти видно як звичайні таблиці [15]. Числа вони опублікували; einfile у чернетці README має лише `TO MEASURE`.

**Automerge.** Автоматичне злиття вкладених структур і тексту, де einfile лише називає конфлікт. Команда, спонсори, десятирічна історія [10, 12]; BeeKEM як препринт [12]. Рівень криптографічних статей, якого в einfile немає.

**`@nkwib/ledger`.** Нуль залежностей, WebCrypto, браузер і Node, той самий ключовий хід: «ланцюг на пристрій + детерміноване злиття + typed conflicts» [47]. За описом, перевірка P-256 працює сьогодні через WebCrypto без окремого постачальника. Використовує неекспортований ключ.

**Radicle.** Працює в мережі зараз, релізи 2026 [п], поріг делегатів і канонічні гілки [53]. Він уже пройшов дефект, який einfile описує як межу: replay старих підписаних refs; виправлено 1.8.0 підписом попереднього елемента або nonce [54]. Це прямий урок для правила Rollback.

**Hypercore.** Sparse-реплікація з перевіреним читанням і кворум підписантів у маніфесті [2].

**Matrix.** Найзріліше відоме мені стирання під підписом: підпис покриває редаговану форму, тому редакція не ламає його [41]. Працює на мільйонах кімнат [О, числа не перевіряв].

**`aion-context`.** Опублікований, 3.0.0, K-of-N кворум, ML-DSA-65, гібридний режим [48]. У einfile k-of-n немає, постачальник лише вибраний.

**`cdx-core`.** Опублікований на crates.io: ML-DSA-65 + OTS + RFC 3161 в одному контейнері [49]. Використання мале: 59 завантажень, 1 зірка [49].

**F3 і AnyBlox.** Рецензовані статті SIGMOD та VLDB, виміряні витрати wasm, відома команда [50п, 51п]. За коментарем у видачі, F3 тримає wasm на групах колонок, AnyBlox на весь файл [п].

**SQLite + session.** Формат стабільний з 2004 [24], три політики конфлікту [23].

**RFC 4998 ERS.** Стандартне оновлення міток часу до того, як алгоритми ослабнуть [63]; чернетка README такого механізму не згадує.

**Sigstore.** Реальні журнали з реальними екосистемами [26]. Те, що файл один, ці системи не цікавить, і для прозорості це перевага: rollback видно ззовні.

## 4. Чи захищаються слова `new`, `first`, `unique`

| Слово | Вердикт | Причина |
|---|---|---|
| `first` | **ні** | Немає публічної дати; `ledger`, DoltLite, `aion-context`, `cdx-core` опубліковані раніше і перекривають частини. Перевірити негативне твердження неможливо |
| `unique`, `only`, `no other` | **ні** | Той самий аргумент; пошук закінчується межею прочитаного. У таблиці 217 із 418 клітинок це `—` |
| `novel` | **ні** | Кожна складова має попередника (§0.4). Новизни механізму я не знайшов |
| `new` | лише буквально | «Новий формат файлу» правдиво і нічого не стверджує |
| `Unlike SQLite, two copies merge` | **ризиковано** | SQLite має зміни з `xConflict`, якщо сесія їх записала [23]; форк SQLite DoltLite зливає [17]. Захищається вужче: «a stock SQLite file carries no merge history». `sqldiff` не перевірено |
| `anyone can check the file was not altered` | **хибно сьогодні** | Чернетка README: читач рахує, `holds` хибне; rollback і зняття хвоста видно лише з якорем або другою копією |
| `Rows only this library has` у README | **переписати** | Правда лише щодо рядків SQLite, LMDB, redb; LMDB і redb я не перевіряв. Безпечніше: «Rows the others in this table lack» |
| `reads itself` | так, з уточненням | README уже каже «archive files» |
| `Designed for erasure ... an erased id confirms no guess` | так, якщо код це тримає | Механізм відомий, солоні зобов'язання [62, 31]; претензії на новизну в рядку немає, і це добре |

**Найсильніше захищене речення (англійською, бо README англійською):**

> We compared 38 systems on 2026-11-01 and found none that keeps, in one file, per-device signed history, offline merge with named conflicts, signed erasure, and its own sandboxed reader; the table and its sources are here, and corrections are welcome.

Умови. Без першої речення хибне:
1. Перевірка ML-DSA у коді, який можна взяти з публічного репозиторію; печатка реально перевіряється (`checked>0`).
2. Посилання веде на цю таблицю з датою, а `—` названо як «не знайдено».
3. Слово `found` не замінювати на `there is`. `only`, `first` і `unique` не додавати. ML-DSA, Bitcoin і «скрипт» у речення не вносити: вони є в інших або ще не виконуються.

Кількість: 38 рядків таблиці без самого einfile. Якщо таблицю змінять, число міняється разом із нею.

## 5. Названі загрози: хто може додати це за рік

Оцінки **[О]**, факти з посиланнями.

| Проєкт | Що вже є | Чого бракує | Загроза |
|---|---|---|---|
| **DoltLite / Dolt** | файл, злиття, конфлікти, `file://`; GPG-підпис у Dolt [16]; 0.50 за 5 місяців [17]; за DoltHub, написано «vibe-coded» [19п], тобто темп схожий на наш | підпис у DoltLite, стирання, вбудований читач | **Висока** для рядка 2 README. Ринок «SQLite, що зливається» вони вже зайняли |
| **Automerge + Keyhive** | злиття, файл, авторство розпочато 07.2026, Keyhive alpha [12, 13] | вбудований читач, стирання, PQ | **Висока** для «підписане злиття»; команда й гранти є. Прямо сказано: «who wrote which commit» [12] |
| **`@nkwib/ledger`** | ядро ідеї [47] | файл, стирання, PQ, користувачі | Середня: мало коду, легко додати; 0 зірок |
| **`aion-context`, `cdx-core`** | ML-DSA, ланцюг, OTS [48, 49] | злиття | Середня: одна людина може додати DAG, попиту не видно |
| **Radicle** | підписані історії, COB, мережа [53] | один файл, стирання | Низька: інша ніша |
| **Sigstore** | журнали, ML-DSA окремим типом [27] | усе щодо файлу | Низька як конкурент, це радше постачальник якорів |
| **Git 3.0** | SHA-256 за замовчуванням планується, дати немає [п: deployhq, phoronix] | PQ-підпис, стирання | Низька |
| **F3, AnyBlox** | wasm у файлі | підписи, злиття | Низька; але це вони визначать, як рецензент сприйматиме wasm у файлі |
| **SQLite** | формат, session | наміру додавати підпис не знайшов | Низька, UNVERIFIED |

Висновок [О]: єдина стійка перевага за рік не в комбінації, а в тому, що опубліковано й перевірено чужими людьми. Комбінацію за рік можуть зібрати DoltLite або Automerge, якщо захочуть.

## 6. Що спитає рецензент під `What is new`

Це факти для заявника, не відповіді. Текст форми: порівняти з іншими зусиллями, наявними й історичними, що нового чи іншого, і **чи думали долучитися до інших** (до 4000 символів) [59].

| Питання, яке очікую | Факт на 2026-11-01 |
|---|---|
| Де код? | Нічого не опубліковано; ім'я `einfile` на crates.io вільне, але не зареєстроване (пам'ять, 11-01; повторно не перевіряв) |
| Чому не Automerge, Git, Fossil, DoltLite, Willow? | Прямо в формі [59]. Внесків у ці проєкти в прочитаних матеріалах немає; UNVERIFIED, чи були. NLnet уже фінансував WillowSync [60п], Automerge-MST [61] і має інтерв'ю з Earthstar [п], отже порівняння вони знають |
| Що вміє підпис? | Чернетка: коду підпису в крейтах немає, постачальник у `w-sig` написаний і за нотатками ще не скомпільований. Головний лейн міг піти далі, цього я не перевіряв |
| Чи перевірить чужа людина? | Python stdlib і Node читають, рахують; не перевіряють підпис (README, Limits). `holds` хибне |
| Що конкретно робить злиття? | M2-min: дві копії однієї genesis; конфлікт названо, вирішує людина; відмови за назвою |
| Які незалежні реалізації? | Кілька читачів, але один автор (B §3). Python не ділить код із бібліотекою. Чужої реалізації немає |
| Яка межа доказу? | Цілісність, не правда. Rollback, зняття хвоста, стрип MERGE видно лише з якорем або другою копією; обмін файлів одного власника проходить (README, Limits) |
| Хто це атакував? | Зовнішнього аудиту немає. Жоден раунд атаки не повернувся порожнім; шість названих випадків не тестовано; Tamarin-модель не розбирали; фазерів немає; Lean про модель, не про Rust (README) |
| Де зроблено ML-DSA, і чи виправдані 3 309 Б? | Постачальник `fips204`, запасний RustCrypto; `dsa.rs` як оракул (пам'ять 11-01). Чернетка README ще описує `Sign_internal`, рішення оператора вже інше: розбіжність виправити до публікації |
| Хто користувачі? | Зовнішніх немає; dowiz сама (B §4). Інтерв'ю §7 B: результатів не бачив |
| Чому Bitcoin у грантовому проєкті? | Адаптер якоря не в репозиторії; кілька видів якоря; ланцюг ніколи не обов'язковий. Позицію NLnet щодо блокчейну не перевіряв |
| Кому це взагалі треба? | B §1: платять за синк з сервером і за вендора; QLDB закрито [64п]. Це сильний контраргумент, і рецензент може його знати |
| Вузьке місце формату? | Unix-only `FileDevice`, один письменник, файл понад 512 МіБ не виміряно; числа `TO MEASURE` (README) |
| Хто це зробив? | Форма питає, чи застосовано ШІ в тексті заявки і чи кодова база має значну частку ШІ-коду; вимагає промпти або логи; правило: `Proposals and projects should be made by humans, not AI` [59]. Кодову базу писали лейни моделей. Цей звіт теж написано ШІ: якщо його вплив є в заявці, це підлягає розкриттю |
| Хто підтримуватиме? | Один розробник (B §1). Форма питає про екосистему і залежності [59] |

Форма: перший грант 5–50 тис. євро [59]. Сторінку читав через інструмент, звірити текст форми в браузері перед поданням.

## 7. Чого я не зміг перевірити

1. Усі клітинки `—`: сторінки мовчать. Особливо Jazz (обов'язковість сервера), Hypercore (PQ), DoltLite (підпис, алгоритм хешу).
2. GitHub-сторінка F3 не відкрилась (socket hang up); F3 і AnyBlox описано з видачі, блогів і API GitHub; PDF-статей не читав. Твердження «wasm на групи колонок» походить від коментаря HN у видачі.
3. README Fossil про PGP 404; відомості з пошуку й форуму.
4. Сторінки AWS QLDB: основна 404, дата підтверджена лише сторінкою Prescriptive Guidance зі видачі.
5. Dolt як каталог, а не файл: не підтверджено джерелом, тому `—`.
6. Як Radicle і Jazz поводяться на файлі; чи можна експортувати Radicle в один файл.
7. SSB: fork як фатальна подія, досі UNVERIFIED; гайд мовчить.
8. Статус Earthstar 11, willow_rs після переїзду на Codeberg, Confidential Sync.
9. `sqldiff` і зміни між двома файлами SQLite.
10. Чи підтверджує Matrix `hashes` перебір: хеш лишається [41], але подія містить інші поля; висновку не роблю.
11. C2PA, BagIt, OCFL, TiddlyWiki (сторінка не дала тексту), Loro, Turso sync: у таблицю не ввійшли.
12. Негативне твердження загалом: пошук обмежений США, недетермінований, і малі проєкти з кінця 2026 могли не потрапити. `ledger` і `aion-context` я знайшов лише випадковими запитами, отже такі можуть бути ще.
13. Твердження про einfile взято з чернетки README і нотаток; нічого не запускав.
14. Дату створення `ledger` видно за API GitHub; коду не читав, заявлених гарантій не перевіряв. Те саме для `aion-context`, `cdx-core`, TrustDB.

## 8. Джерела

1. SSB: https://ssbc.github.io/scuttlebutt-protocol-guide/
2. Hypercore: https://docs.pears.com/building-blocks/hypercore
3. Autobase [п]: https://docs.pears.com/p2p/reference/building-blocks/autobase/
4. Willow data model: https://willowprotocol.org/specs/data-model/index.html
5. Willow Drop: https://willowprotocol.org/specs/sideloading/index.html
6. Earthstar: https://earthstar-project.org/docs/what-is-it ; реліз [п]: https://github.com/earthstar-project/earthstar/releases
7. p2panda: https://docs.rs/p2panda-core/latest/p2panda_core/
8. iroh-docs: https://docs.iroh.computer/protocols/documents
9. iroh-docs [п]: https://docs.rs/iroh-docs/=0.101.0/ , https://github.com/n0-computer/iroh-docs
10. Automerge конфлікти: https://automerge.org/docs/reference/documents/conflicts/
11. Automerge формат: https://automerge.org/automerge-binary-format-spec/
12. Automerge липень 2026: https://automerge.org/blog/2026-july/
13. ARK: https://automerge.org/docs/keyhive/ark-api-guide/
14. Yjs: https://docs.yjs.dev/api/document-updates
15. Dolt merges: https://www.dolthub.com/docs/sql-reference/version-control/merges
16. Dolt підпис [п]: https://www.dolthub.com/blog/2024-09-16-signed-commits/
17. DoltLite бета: https://www.dolthub.com/blog/2026-08-31-doltlite-beta/
18. DoltLite README: https://github.com/timsehn/doltlite ; API: https://api.github.com/repos/dolthub/doltlite (створено 2026-03-13)
19. DoltLite запуск [п]: https://www.dolthub.com/blog/2026-03-25-doltlite/
20. immudb: https://github.com/codenotary/immudb
21. Datomic excision: https://docs.datomic.com/reference/excision.html
22. Datomic модель: https://docs.datomic.com/whatis/data-model.html
23. SQLite session: https://sqlite.org/sessionintro.html
24. SQLite формат: https://www.sqlite.org/fileformat2.html
25. cr-sqlite: https://github.com/vlcn-io/cr-sqlite , https://api.github.com/repos/vlcn-io/cr-sqlite , https://vlcn.io/docs/cr-sqlite/crdts/table-crdts (останнє з A)
26. Rekor v2: https://blog.sigstore.dev/rekor-v2-ga/
27. rekor-tiles#425: https://github.com/sigstore/rekor-tiles/issues/425
28. timestamp-authority [п]: https://github.com/sigstore/timestamp-authority/releases/tag/v2.2.0 ; Trail of Bits [п]: https://blog.trailofbits.com/2026/01/29/building-cryptographic-agility-into-sigstore/
29. RFC 9162: https://www.rfc-editor.org/rfc/rfc9162.html (з A)
30. tlog-witness: https://c2sp.org/tlog-witness
31. OTS README: https://github.com/opentimestamps/opentimestamps-client/blob/master/README.md
32. OTS у Git: https://github.com/opentimestamps/opentimestamps-client/blob/master/doc/git-integration.md
33. Ditto [п]: https://docs.ditto.live/basic/about
34. Ditto сертифікати [п]: https://docs.ditto.live/sdk/v4/auth-and-authorization/certificate-based-security
35. PowerSync: https://docs.powersync.com/architecture/powersync-protocol
36. git-bundle: https://git-scm.com/docs/git-bundle
37. git-merge: https://git-scm.com/docs/git-merge
38. Fossil: https://fossil-scm.org/home/doc/trunk/www/concepts.wiki
39. Fossil підпис [п]: https://fossil-scm.org/home/help?cmd=clearsign , https://fossil-scm.org/forum/forumpost/c39ed9790b
40. Fossil shun (з A): https://fossil-scm.org/home/doc/trunk/www/shunning.wiki
41. Matrix: https://spec.matrix.org/latest/server-server-api/
42. NIP-03: https://github.com/nostr-protocol/nips/blob/master/03.md
43. NIP-09: https://github.com/nostr-protocol/nips/blob/master/09.md
44. CAR: https://ipld.io/specs/transport/car/carv1/
45. PDF/A-1: https://www.loc.gov/preservation/digital/formats/fdd/fdd000125.shtml
46. Parquet: https://parquet.apache.org/docs/file-format/data-pages/encryption/
47. ledger: https://github.com/nkwib/ledger , https://api.github.com/repos/nkwib/ledger
48. aion-context: https://docs.rs/crate/aion-context/latest , https://crates.io/api/v1/crates/aion-context
49. cdx-core: https://docs.rs/crate/cdx-core/latest/source/README.md , https://crates.io/api/v1/crates/cdx-core , https://api.github.com/repos/Entrolution/cdx-core
50. F3 [п]: https://github.com/future-file-format/f3 , https://api.github.com/repos/future-file-format/f3 , DOI 10.1145/3749163 , https://blog-1hf.pages.dev/blog/f3-1/
51. AnyBlox [п]: https://duckdb.org/library/anyblox-self-decoding-datasets/ , https://github.com/AnyBlox/vldb-2025
52. TrustMee: https://arxiv.org/abs/2602.13148
53. Radicle: https://radicle.dev/guides/protocol
54. Radicle Q1 2026: https://community.radworks.org/t/radicle-org-q1-2026-update/3723 ; релізи [п]: https://radicle.dev/2026/03/18/radicle-1.7.0
55. AT Protocol: https://atproto.com/specs/repository
56. Jazz [п]: https://jazz.tools/docs/react/reference/encryption
57. Fireproof [п]: https://use-fireproof.com/docs/architecture , https://fireproof.storage/documentation/how-the-database-engine-works/
58. TrustDB: https://pkg.go.dev/github.com/wowtrust/trustdb/v2
59. NLnet форма: https://nlnet.nl/propose/
60. WillowSync [п]: https://nlnet.nl/project/WillowSync/ ; Earthstar [п]: https://nlnet.nl/project/Earthstar/interview.html
61. Automerge-MST (з B): https://nlnet.nl/project/Automerge-MST/
62. RFC 9901 SD-JWT: https://www.rfc-editor.org/rfc/rfc9901.html
63. RFC 4998 ERS: https://www.rfc-editor.org/rfc/rfc4998.html
64. AWS про QLDB [п]: https://docs.aws.amazon.com/prescriptive-guidance/latest/modern-data-centric-use-cases/data-preparation-cleaning.html
65. AWS про заміну QLDB [п]: https://aws.amazon.com/jp/blogs/news/replace-qldb-with-amazon-aurora-postgresql/
66. Bamboo [п]: https://github.com/AljoschaMeyer/bamboo
