# R-MLDSA-PROVENANCE: чи замінювати власний ML-DSA-65

W-MG-PQ, 2026-11-01. Дерево читалось лише на читання: `/root/lanes/batch7`. Нічого не компілювалось і не запускалось: жодне число нижче не є виміром швидкості чи розміру wasm. Вихідники чотирьох крейтів завантажені як дані й прочитані grep-ом, не зібрані.

Позначки: **[дерево]** прочитано в нашому коді, **[джерело]** прочитано в опублікованому вихіднику або на сторінці за URL, **[не перевірено]** без першоджерела.

## 1. Рішення

**Одна зміна термінова, друга ні. Обидві рекомендовані, але незалежні.**

1. **Формат, до першого релізу.** Перейти з `Sign_internal` / `Verify_internal` на зовнішній pure ML-DSA з порожнім контекстом: `M' = 0x00 0x00 ‖ msg`, `ctx = ""`. Ключі, довжини, `alg = 65` і структура SEAL не змінюються, змінюються лише значення підписів. Це єдине незворотне рішення: після першого релізу його не повернути.
2. **Бібліотека: `fips204` 0.5.x** (MIT OR Apache-2.0) у новому малому крейті-адаптері під MIT OR Apache-2.0, який реалізує `Signer` і `Verifier` з `bebop-log`. `dowiz-hub` використовує той самий адаптер. Тоді стороння людина під MIT не приносить своє, а бере те саме, що працює в нас. `dsa.rs` не видаляється: лишається для інших викликів (§6) і як диференційний оракул.

Крок 2 можна відкласти або відкотити без змін формату, бо після кроку 1 підпис перевіряє будь-яка бібліотека зі списку. Крок 1 від кроку 2 не залежить.

Чесна межа рішення: **`fips204` теж без незалежного аудиту.** Заміна не дає «аудитованого» коду, бо такого чистого Rust-варіанта для wasm я не знайшов. Вона дає більше тестового покриття, вимірювання часу, обнулення секретів і спільний код зі сторонніми верифікаторами.

## 2. Чому `Sign_internal` треба прибрати зараз

Наші байти [дерево]: `mu = SHAKE256(tr ‖ msg)` без префікса (`dsa.rs:775`, `:932`). Виклики без контексту:

| Місце | Що підписує |
|---|---|
| `crates/dowiz-hub/src/doc_seal.rs:117`, `:164` | SEAL: `bebop-seal/1 ‖ …` (`bebop-log/src/doc/seal.rs:68`) |
| `crates/dowiz-hub/src/doc_seal/tsa.rs:38` | мітка TSA: `bebop-anchor/1 ‖ …`; перевірка через `MlDsa65` |
| `workers/api/src/hubdo/publish/root.rs:114` | ціновий корінь: `'R' ‖ dowiz-price-root-v1 ‖ …` |

Зовнішній `ML-DSA.Sign` дає `M' = 0x00 ‖ len(ctx) ‖ ctx ‖ M`. Це видно в коді `fips204`: `ml_dsa::mu(&[&self.tr, &[0u8], &[ctx.len()…], ctx, message])` (`src/lib.rs:329`) [джерело]. Тому підпис над `msg` через `Sign_internal` не проходить звичайний `verify(msg)` жодної бібліотеки.

Хто може перевірити наш поточний байт-у-байт формат:

| Бібліотека | Internal у публічному API | Доказ |
|---|---|---|
| RustCrypto `ml-dsa` 0.1.2 | Так, але тимчасово: над `pub fn sign_internal` стоїть `TODO(RLB) Only expose based on a feature. Tests need access, but normal code shouldn't.` | `src/signing.rs:314-315`; `verify_internal` на https://docs.rs/ml-dsa/latest/ml_dsa/struct.VerifyingKey.html |
| `fips204` 0.5.0 | Ні. Лише feature `acvp-internal`, `#[deprecated]`, `Hook for the NIST ACVP internal test groups; do not use elsewhere`. У 0.5.0 звичайні `_internal_sign` / `_internal_verify` видалено | `src/lib.rs:468-534`, `CHANGELOG.md` |
| libcrux-ml-dsa 0.0.11 | Лише за feature `acvp` | `src/ml_dsa_65.rs:89-106` |
| aws-lc-rs 1.19 | Ні. Лише pure, порожній контекст, без external mu і pre-hash | https://docs.rs/aws-lc-rs/latest/aws_lc_rs/signature/index.html |
| mldsa-native (C) | Так: `pre` може бути NULL за `prelen == 0` | https://raw.githubusercontent.com/pq-code-package/mldsa-native/main/mldsa/mldsa_native.h |
| pqcrypto-mldsa 0.1.2 | Ні, у `src/mldsa65.rs` є лише звичайний і `_ctx` варіанти | читання `src/mldsa65.rs` |
| OpenSSL 3.5 | Для підпису `message-encoding=0`, сторінка називає це тестовим режимом; для перевірки не описано | https://docs.openssl.org/3.5/man7/EVP_SIGNATURE-ML-DSA/ |

Висновок: тренд проти нас. Дві Rust-реалізації вже сховали internal, третя залишила TODO сховати. Лишатись на ньому означає прив'язати публічний формат до тестового входу.

Норма. FIPS 204 у вступі до розділу про внутрішні функції каже, що, крім тестування, ці інтерфейси «should not be made available to applications». Це «should not», а не «shall not». Цитату я бачив лише вторинно, у пості Mike Ounsworth на pqc-forum: https://groups.google.com/a/list.nist.gov/g/pqc-forum/c/949fsZRC6Ik. PDF стандарту завантажився (https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.204.pdf), але витягти текст на цій коробці не вдалось. Статус стандарту Final, 2024-08-13: https://csrc.nist.gov/pubs/fips/204/final.

Чому `ctx = ""`, а не `ctx = "bebop-seal/1"`:

- aws-lc-rs уміє лише порожній контекст [джерело вище]; непорожній відрізав би цього верифікатора.
- RFC 9881 для X.509 теж лишає контекст порожнім: https://www.rfc-editor.org/rfc/rfc9881.txt, розділ 3.
- Розділення доменів уже є в самому повідомленні: `bebop-seal/1`, `bebop-anchor/1`, `R…` [дерево]. Перший байт жодного з них не `0x00`, тож перехідний збіг внутрішнього й зовнішнього простору теж неможливий.

Детермінізм (`rnd = 0^32`) у формат не входить: «a verifier cannot tell them apart» (RFC 9881, розділ 9). Його можна змінити будь-коли. mldsa-native застерігає від детермінованого режиму там, де важливі fault- і side-channel атаки (https://github.com/pq-code-package/mldsa-native). Це окреме питання, не для цієї міграції. Коментар у `doc_seal.rs:162` обіцяє «two signings of one message are then one signature», а aws-lc-rs не дає передати `rnd` (`sign(&self, msg, signature)`: https://docs.rs/aws-lc-rs/latest/aws_lc_rs/signature/struct.PqdsaKeyPair.html). Верифікатору це не заважає, а підписанту на aws-lc-rs тоді обіцянка може не діяти (рандомізовано чи ні, сторінка не каже).

## 3. Порівняння

Версії й дати з crates.io API на 2026-11-01. «Рядки» це `.rs` у `src/` опублікованого крейта, без залежностей, тож це не розмір бінарника.

| | `fips204` 0.5.0 | RustCrypto `ml-dsa` 0.1.2 | libcrux-ml-dsa 0.0.11 | aws-lc-rs 1.19 / aws-lc-sys 0.47 |
|---|---|---|---|---|
| Ліцензія | MIT OR Apache-2.0 | Apache-2.0 OR MIT | Apache-2.0 | ISC AND (Apache-2.0 OR ISC); sys-крейт складений, з BSD-3-Clause |
| Реліз | 2026-10-23 | 2026-10-31 | 2026-10-07 | 2026-10-28 |
| FIPS 204 final | заявлено | заявлено | заявлено всі три варіанти | заявлено, FIPS-збірка окремо (нижче) |
| Формальна верифікація | немає | немає | часткова: «field arithmetic, NTT polynomial arithmetic, and serialization» | CBMC на імпортованому C (нижче) |
| Постійний час | заява на рівні вихідника в межах однієї спроби; dudect 1,04 млн зразків, t = 1,59 | у changelog 0.1.2 `Fixed constant time issues`; заяви в README немає | dudect чи аналог не знайдено; є `libcrux-secrets` для valgrind | valgrind у CI за ввімкненої перевірки |
| Аудит | не згадано | «never been independently audited» | не знайдено | окремого на ML-DSA не знайдено |
| Відомі вади | у RUSTSEC сторінки немає (404) | RUSTSEC-2025-0144, CVE-2026-24850 | RUSTSEC-2026-0076, -0077, -0125, -0126 | не шукав окремо |
| `no_std` | так, без `alloc` | так | так | ні, C-бібліотека [не перевірено дослівно] |
| wasm32 | `wasm32-unknown-unknown` у матриці CI | Rust; ціль окремо не перевірена | feature `simd128` є; портабельна збірка не перевірена | у таблиці платформ лише `wasm32-unknown-emscripten` |
| Рядки / `.crate` | 4278 / 577 КБ | 3899 / 57 КБ | 13604 / 129 КБ | sys-крейт 9,7 МБ |
| Сід у ключ | `keygen_from_seed(&[u8;32])` | `SigningKey::from_seed` | у `generate_key_pair` за randomness | `from_seed` |
| Підпис із заданим `rnd` | `try_sign_with_seed(&[0;32], msg, ctx)` | `sign_deterministic(msg, ctx)` | `sign(.., ctx, randomness)` | ні |
| Залежності | `rand_core`, `sha3`, `zeroize` | 6 обов'язкових (`hybrid-array`, `module-lattice`, `shake`, `signature`, `ctutils`, `crypto-common`) | 5 своїх `libcrux-*` | C/C++ компілятор |
| MSRV (у нас 1.96.1) | 1.85 | 1.87 | 1.89 у workspace | 1.71 |
| `unsafe` | `deny(unsafe_code)` | у `src/*.rs` рядків `unsafe` немає | `deny(unsafe_code)`; є `libcrux-intrinsics` | C і FFI |
| Завантаження за 90 днів | 680 918 | 3 208 866 | 155 428 | не брав |

Решта зі списку картки:

| Кандидат | Стан | Висновок |
|---|---|---|
| PQClean | «retired and archived as read-only», архівовано 2026-08-04; відсилає до mldsa-native: https://github.com/PQClean/PQClean | Ні |
| pqcrypto-mldsa 0.1.2 | Обгортка над PQClean, останній реліз 2025-08-05, збірка через `cc`: https://docs.rs/pqcrypto-mldsa/latest/pqcrypto_mldsa/ | Ні: джерело архівоване |
| liboqs | MIT; ML-DSA від mldsa-native; «WE DO NOT CURRENTLY RECOMMEND RELYING ON THIS LIBRARY IN A PRODUCTION ENVIRONMENT»: https://github.com/open-quantum-safe/liboqs | Ні: сам проєкт це забороняє |
| HACL* | README перелічує Curve25519, Ed25519, AES-GCM, Chacha20, Poly1305, SHA-2, SHA-3, HMAC, HKDF; ML-DSA немає: https://github.com/hacl-star/hacl-star | Немає продукту |
| libjade | Dilithium лише як майбутня мета, лише amd64: https://github.com/formosa-crypto/libjade | Немає продукту |
| formosa-mldsa | Jasmin, x86-64 і ARMv7M, але README: експериментальний, не верифікований (знайдено через дзеркало https://gh.r-mj.com/formosa-crypto/formosa-mldsa, оригінал не читав) | Ні |
| mldsa-native | Apache-2.0, ISC або MIT на вибір. CBMC для C, HOL Light для асемблера, valgrind, усі вектори ACVP і Wycheproof: https://github.com/pq-code-package/mldsa-native | Найкраще походження, але C (нижче) |

### Що саме верифіковано

**libcrux: межа вужча за значок.** README обмежує заяву арифметикою поля, NTT і серіалізацією: https://github.com/cryspen/libcrux/blob/main/libcrux-ml-dsa/README.md. У Makefile усе поза `VERIFIED_MODULES` допущене без доведення: https://raw.githubusercontent.com/cryspen/libcrux/main/libcrux-ml-dsa/proofs/fstar/extraction/Makefile. Symbolic Software стверджує, що сюди входять підпис, перевірка й генерація ключів, і показав дві реальні вади саме там: мертву перевірку норми `z` і неповну перевірку лічильника підказок, виправлені PR #1347 і #1348 4 березня 2026: https://symbolic.software/blog/2026-02-17-cryspen-mldsa/. Там само третя знахідка: специфікація `i16_mul_32extended` рахує x² замість x·y, що підриває докази AVX2. Станом на оновлення статті від 6 березня виправлення не згадане; поточний стан **не перевірено**. RUSTSEC 2026 рік: https://rustsec.org/packages/libcrux-ml-dsa.html. Два з чотирьох записів це прийняття недійсного підпису на шляху перевірки (-0077, -0125). Для нас значок не закриває ризик, який нас хвилює: чи `verify` каже правду.

**mldsa-native: CBMC це відсутність UB, не правильність.** Функційна коректність доведена HOL Light для асемблерних бекендів; для C-коду такого твердження в документі немає: https://raw.githubusercontent.com/pq-code-package/mldsa-native/main/SOUNDNESS.md. Поза межами: живлення, EM, мікроархітектура, fault-атаки.

**AWS-LC.** ML-DSA імпортовано з mldsa-native, лише C-варіант, CBMC, valgrind у CI, PCT у FIPS-збірці: https://raw.githubusercontent.com/aws/aws-lc/main/crypto/fipsmodule/ml_dsa/README.md. `aws-lc-fips-sys` 3.x пройшов лабораторне тестування й поданий до NIST: https://github.com/aws/aws-lc-rs. Чи входить ML-DSA у межі саме цієї сертифікації, **не перевірено**. API ML-DSA стабілізовано в 1.18.0: https://docs.rs/aws-lc-rs/latest/aws_lc_rs/unstable/signature/index.html.

**`fips204`.** Формальної верифікації немає. Є ACVP (keyGen, sigGen, sigVer), Wycheproof і CCTV, `tests/negative.rs`, обнулення «best effort» і обмеження 814 спроб: https://raw.githubusercontent.com/integritychain/fips204/main/README.md, `CHANGELOG.md` у крейті. Застереження щодо dudect: заміряно **ML-DSA-44, не 65**, на одному Intel i7-13700K, у режимі однієї спроби, двома фіксованими класами; автори самі кажуть, що t < 5 це не доказ: https://raw.githubusercontent.com/integritychain/fips204/main/dudect/README.md. Постійний час заявлено лише для вихідника; компілятор і CPU довірені. Під wasm не вимірювалось. Проєкт волонтерський («reasonable-effort basis», `SECURITY.md`).

**RustCrypto.** Попередження про відсутність аудиту: https://github.com/RustCrypto/signatures/blob/master/ml-dsa/README.md. Вади: дублікати підказок приймались у версіях від 0.0.4 до 0.1.0-rc.4 (https://nvd.nist.gov/vuln/detail/CVE-2026-24850), таймінг у `decompose` (https://rustsec.org/packages/ml-dsa.html). Wycheproof у `tests/wycheproof.rs` є.

## 4. Наше покриття [дерево]

| Перевірка | Є | Зауваження |
|---|---|---|
| ACVP keyGen ML-DSA-65 | 25 | `pq/kat/acvp/key-gen.json` |
| ACVP sigGen ML-DSA-65 | 20 (10 детерм., 10 рандом.) | `sig-gen.json` |
| ACVP sigVer ML-DSA-65 | 15 | по 3: валідний, змінене повідомлення, змінений підпис, `z too large`, `too many hints` |
| Перевірка кількості | 3 тести | `dsa_acvp_tests.rs:299-316` |
| Зовнішній інтерфейс | **0** | у вендорованих файлах немає поля `signatureInterface` |
| Wycheproof | **0** | grep по `dowiz-core`, `dowiz-hub`, `bebop-log`, `workers/api/src` порожній |
| dudect на `dsa` | **0** | `ct_gate.rs` і `kernel/src/ct_gate.rs` міряють лише компаратори; `mesh` лише порівняння байтів |
| Обнулення секретів | **0** | `zeroize` немає; `DeviceKey.sk` це звичайний `Vec<u8>`; буфер із `key` у `sign_internal_bytes` не стирається |
| Ліміт спроб підпису | **0** | `loop` без межі (FIPS-межа 814 у `fips204`) |

Дві розбіжності:

- **Заголовок тесту неточний.** `dsa_acvp_tests.rs:5` каже «isSample=false» про всі три; у `sig-gen.json` стоїть `isSample: true`. Це вузький снапшот, а не поточний експорт.
- **Наш снапшот застарів.** Поточний експорт NIST ACVP-Server (`usnistgov/ACVP-Server`, `gen-val/json-files/ML-DSA-*-FIPS204/internalProjection.json`, завантажено й пораховано 2026-11-01) для ML-DSA-65: sigGen 120, sigVer 60. Pure-external: **30 sigGen + 15 sigVer**, internal без external mu: 30 + 15, плюс pre-hash і external mu. Ми маємо 35 векторів sigGen і sigVer проти 180 і нуль із групи, яку ця міграція робить робочою.

Тож фраза «ACVP byte-exact» правдива лише для internal.

Порівняння нашого коду з чужими вадами. Це одне читання, не доказ:

| Клас вади | У нас |
|---|---|
| `decompose` з діленням (RUSTSEC-2025-0144) | Ділення немає, множення і зсув (`dsa.rs:261-268`). Grep `/` і `%` по `dsa.rs` дав лише константи й межі циклів |
| Дублікати підказок (CVE-2026-24850) | Строге `<=` відкидає (`dsa.rs:684`) |
| Норма `z` (RUSTSEC-2026-0077) | Межа `GAMMA1 - BETA` (`dsa.rs:925`) |
| Підказки поза межами / паніка (RUSTSEC-2026-0076) | `cnt < k`, `cnt > OMEGA`, хвіст нульовий (`dsa.rs:680`, `:691`) |
| Дірки в перевірці знака норми | `poly_chknorm` без гілки на знаку, ранній вихід лише на порушенні, як у референсі |

Залишок ризику: `make_hint` гілкується з коротким замиканням (`dsa.rs:270-276`), `rej_eta` гілкується на байтах сіда (`:334`). Обидва збігаються з логікою референсу, але **жодного вимірювання часу немає**. Висновок: явної вади я не знайшов, проблема не в знайденому, а в тому, що ніхто не шукав інструментом.

## 5. Чому `fips204`, а не інші

Жорсткі вимоги з дерева:

- `dowiz-hub` підключений у Worker (`workers/api/Cargo.toml:32`), а `pub mod doc_seal` без feature (`dowiz-hub/src/lib.rs:74`). Отже потрібен `wasm32-unknown-unknown` без C. Це прибирає aws-lc-rs, mldsa-native, liboqs і pqcrypto на цьому шляху.
- Сторонній під MIT: libcrux лише Apache-2.0, сумісно, але не те, про що питає картка.
- `deny.toml` дозволяє MIT, Apache-2.0, ISC.

Лишаються `fips204` і RustCrypto. Обидва без аудиту. Вирішальне:

| Критерій | `fips204` | RustCrypto |
|---|---|---|
| Жодного RUSTSEC | так (сторінка 404) | дві вади 2026 року |
| Опубліковане вимірювання часу | є, хоч і на -44 | не знайдено |
| `no_std` без `alloc`, без купи | так | купа, «when available» |
| Опублікована межа відмови | 814 спроб, помилка | помилка з 0.1.2 |
| Спільнота | менша, волонтери | більша (3,2 млн проти 0,68 млн) |
| Свіжість | 0.5.0 дев'ять днів тому, великий реліз | 0.1.2 день тому, з `constant time` виправленням |

Це близький вибір, а не розгром. Запасний варіант: RustCrypto, той самий адаптер, та сама заміна. Відсутність RUSTSEC у `fips204` може означати меншу увагу, а не менше вад. Від бібліотеки залежність одна: крок 1 робить формат нейтральним.

Для аудиторів і нативних верифікаторів варто рекомендувати aws-lc-rs: найкраще походження (mldsa-native) і працює саме з `ctx = ""`.

## 6. Міграція

Часові оцінки не наводжу. Обсяг, за підрахунком місць у дереві:

**Крок 1, формат (без нових залежностей).**

1. `dsa.rs`: дві функції, `sign_pure` і `verify_pure`, що додають `[0, 0]` до повідомлення.
2. Чотири виклики: `doc_seal.rs:117`, `:164`, `tsa.rs:38`, `root.rs:114`. Плюс адаптер у `bebop-log/src/doc/dsa_tests.rs`.
3. Вендорити поточні групи ACVP external/pure для ML-DSA-65 (30 + 15) і додати до `dsa_acvp_tests.rs`; виправити заголовок про `isSample`.
4. Оновити текст: `docs/design/SPEC-PCD-2026-10-27.md:45` (прямо називає `Sign_internal`), коментарі `root.rs`, `doc_seal.rs`.
5. Диференційний дев-тест проти зовнішньої бібліотеки: наш `sign_pure` і `fips204` на одному сіді й `rnd = 0` мають збігтись байт у байт.

Що змінюється у форматі: **тільки значення підпису**. Сід, `pk`, `actor_id`, 3309 байт, `alg = 65`, структура SEAL, байти `seal::message` без змін. `SEAL_VERSION` піднімати не треба, якщо немає образів, які треба читати. Старі внутрішні підписи після цього не пройдуть і будуть полічені як `bad`, а не тихо прийняті. Жодних золотих бінарних образів із ML-DSA-підписами в дереві я не знайшов: тести підписують із сіда під час запуску.

**Крок 2, бібліотека.**

1. Новий крейт (робоча назва `bebop-seal-mldsa`, MIT OR Apache-2.0): `fips204 = { default-features = false, features = ["ml-dsa-65"] }`, `DeviceKey`-подібний підписант і верифікатор. Схема виклику, не компілювалась: `KG::keygen_from_seed(&seed)`, `sk.try_sign_with_seed(&[0u8; 32], msg, b"")`, `PublicKey::try_from_bytes(..)`, `pk.verify(msg, &sig, b"")`.
2. `dowiz-hub` делегує туди `MlDsa65` і `DeviceKey`. `bebop-log` лишається без підписного коду, як обіцяє `doc.rs`.
3. Новий крейт у дереві: `sha3` (у `Cargo.lock` його зараз немає). `rand_core` і `zeroize` у lock уже є, версії не звіряв. `dowiz-core` не чіпати: його `cargo tree -e no-dev` має лишитись порожнім.
4. `dsa.rs` лишається для `mesh`, `envelope`, `root_delegation`, `crypto_signer` і як оракул. `root.rs` на кроці 2 може лишитись на `sign_pure`, якщо заміна б'є по wasm.

**Умови відкату кроку 2 (перевірити, коли коробка вільна):** розмір wasm Worker-а проти ratchet; час підпису й перевірки в wasm проти нашого; стек підпису в wasm; повний прогін `run-all`. Якщо щось червоне, лишається `dsa.rs` + крок 1, а Wycheproof і dudect додаються до нашого коду. Формат при цьому не страждає.

## 7. Що я не зміг перевірити

- **Цінові корені в проді.** `root.rs` підписує лише коли задано `VENUE_SIGN_SK`, ключ створює оператор. Чи заданий він на проді, з дерева не видно, прод я не чіпав. Картка каже про пломби (seals), а цей об'єкт окремий: якщо секрет заданий, опубліковані `{alg, pk, sig, msg}` уже існують і крок 1 для `root.rs` їх не перевірить. Браузер їх не перевіряє (SPEC-PCD §4), але сторонній міг. Це питання до оператора, перш ніж чіпати `root.rs:114`.
- **QA-образи.** Чи є на QA DOC-образи з реальними підписами, які треба зберегти: не перевірено.
- **Текст FIPS 204.** Цитата про `should not` лише вторинна. Алгоритм `M'` підтверджений кодом `fips204`, не PDF-ом.
- **Розмір, час, стек у wasm** для будь-якої бібліотеки: не вимірювалось, компіляція заборонена. Кількість рядків не замінює цього.
- **Збіг байтів** нашого `sign_pure` із `fips204`: виведений з читання коду, не запущений. Сам `sign_pure` ще не існує.
- **Аудити.** «Не згадано» означає відсутність на прочитаних сторінках, а не доведену відсутність. Для aws-lc і mldsa-native окремо звіти не шукав.
- **`fips204` без `default-rng`:** що `try_sign_with_seed` доступний, бачу у `traits.rs`, але збірку не пробував. Чи справді `wasm32-unknown-unknown` у CI запускає тести чи лише збирає: не дочитано.
- **libcrux, третя знахідка** (x² у специфікації): чи виправлена. Також репозиторій у changelog уже `celabshq/libcrux`, а Makefile і README читались із `cryspen/libcrux` main, що могло бути дзеркалом або старішим станом. Значення `rust-version` і `license` взято з цього workspace.
- **Хто стоїть за `fips204`:** лише організація `integritychain`, волонтери. Пошук не підтвердив конкретних імен.
- **OpenSSL:** чи `message-encoding=0` діє на перевірку; liboqs: чи є internal-вхід у його API.
- **Рецензії на наш код:** §4 це одне читання одним агентом. Це не аудит.
