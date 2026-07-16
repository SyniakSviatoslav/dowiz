# Systems / GPU-ML / Kernel-as-Microcontroller — Research Synthesis (2026-07-16)

> Метод: 8 незалежних Opus-агентів дослідили окремі кластери тем (хмара/IaC, дані/БД,
> розподілені системи, GPU/ML-інфра, "kernel-as-microcontroller" синтез, AI/математика,
> git/CI/CD/frontend, довідкові репо) — кожен читав реальний код `kernel/src/*`,
> `engine/src/*`, `bebop2/*` перед тим як щось рекомендувати. Цей документ — один
> reasoning-прохід поверх усіх 8 звітів: де знахідки різних агентів **незалежно
> підтверджують одна одну** (сильний сигнал), де є напруга між ідеями, і що з цього
> реально впроваджувати, в якому порядку. Сирі звіти агентів не копіювались — кожен
> розділ нижче переписаний і схрещений з іншими.
>
> Два репо в основі: `/root/dowiz` (kernel = 152-файловий event-sourced Rust-core,
> engine = фізичний field-UI рендерер MÜ+ΓU̇+c²LU=S, wasm = тонкий binding) і
> `/root/bebop-repo` (openbebop/bebop2 = сам протокол: mesh-node, ports, proto-cap
> capability-trust, proto-crypto ML-DSA-65⊕Ed25519, proto-wire TLV).

---

## 1. Головна теза користувача — вердикт

Теза: *"хаб-структура/бінарники з усім мають бути мінідигital-мікроконтролером,
event/data-driven microservices, кожен — децентралізований mesh-нод з
адаптерами/мостами"*, плюс *"використати ML/GPU-підходи для Rust/kernel бінарників
щодо процесорів/ядер/пам'яті"*.

**Вердикт (чесний, фальсифіковний — за правилом проєкту "REJECT appeal-to-authority"):**
модель **влучна для одного нода, є розтягуванням для системи в цілому, і почасти
аспіраційна проти сьогоднішнього коду.**

- **Де тримається твердо.** `bebop2/ARCHITECTURE.md` сam формулює це так: "fits 2K
  core RAM per primitive", "a 2.048 MHz machine", "no dense matmul, no allocator
  thrash at hot path" — дисципліна рівня Apollo Guidance Computer, свідомо обрана
  авторами, не нав'язана ззовні. `MeshNode` (`bebop2/mesh-node/src/node.rs`) справді
  є самодостатнім compute+memory+I/O вузлом: власний `Transport`, власний `DodGate`,
  власний injected clock, без спільної mutable пам'яті з іншими нодами — event-driven
  (recv → gate → apply), як переривання на MCU. Детермінізм скрізь (`no_std`-дружній
  `money.rs` без float, фіксовані ітерації в `ppr.rs`/`csr.rs` заради bit-reproducibility)
  — саме те, що цінує embedded-практика.
- **Де це розтягування.** "Мікроконтролер" в однині применшує систему — це **мережа
  багатьох** автономних вузлів із capability-token довірою й підписаними міжвузловими
  фреймами. Чесна модель — **розподілена embedded-система / mesh сенсорів-MCU**, не
  один MCU. Твердження "ціле — це мінідиджитал-мікроконтролер" сплющує розподілену
  систему в метафору одного кристала й губить interconnect-ієрархію (§6.3), яка
  насправді керує продуктивністю.
- **Де це аспірація, а не опис коду сьогодні.** Реальні MCU уникають heap churn;
  числовий шар kernel — навпаки: `kalman.rs` виділяє ~15 `Mat` на крок фільтра,
  `spectral.rs::charpoly` виділяє ~2n `Vec<Vec<f64>>` на власне розкладання,
  `csr.rs::to_adjacency` вибухає розріджену матрицю в щільну O(n²). Власний коментар
  `ARCHITECTURE.md` — "no allocator thrash at hot path / O(n) not O(n²)" — це **ціль,
  якої числові модулі ще не досягають.**
- **"Використати ML/GPU-підходи" сьогодні означає ML-*подібну математику*, не ML і
  не GPU.** GPU в стеку немає (`field_frame.rs`: "wgpu OUT OF SCOPE"), `attention.rs`
  свідомо "reference scalar… no SIMD, no learned weights… kernel stays non-AI".
  Перенесене — це **дизайн-принципи** (fusion, batching, MIG-подібна ізоляція,
  KV-cache, scheduler-interleaving, locality tiers), не субстрат (SIMD-лейни,
  GPU-кернели, навчені ваги).

**Отже:** прийняти модель як **архітектурну північну зірку** (вона коректно передбачає,
де знаходяться виграші — §§3-8 нижче), але не плутати її з описом того, що бінарники
роблять сьогодні.

---

## 2. Наскрізні патерни, що вже присутні (і повторюються в кожному домені)

Найважливіший результат синтезу: один і той самий архітектурний прийом з'являється
незалежно в кожному дослідженому кластері. Це не збіг — це вже де-факто "стиль дому",
просто ніде не назв
аний явно.

### 2.1 Trait-as-Port — усе зовнішнє за трейтом

`BlockStore` (S3-адаптер), `EventStore` (DynamoDB-адаптер, `MemEventStore` дефолт /
`PgEventStore` продакшн), `Transport` (VPC/мережа, `wss_transport` fallback /
`iroh_transport::QuicTransport` основний), `ChannelAdapter`/`InboundPort`/`OutboundPort`
(MCP/A2A/ACP), `SandboxTier` (MIG-подібна ізоляція), `PayloadEnc` (шифрування,
`NoopPayloadEnc` дефолт → ML-KEM-768→XChaCha20 продакшн). Правило "no cloud by
default — only adapters" — це не політика зверху, це буквально те, як вже написаний
код: self-hosted реалізація за замовчуванням, cloud/зовнішнє — один з можливих impl
того ж трейту, ніколи архітектура. **Дія:** коли з'являється нова інтеграція (нова хмара,
новий AI-протокол, нова БД) — перше питання не "як інтегрувати", а "який трейт це вже
розширює".

### 2.2 Content-addressing — універсальний ключ для кешу/дедуплікації/аудиту/шардингу

`sha3_256` в `event_log.rs` (event_id = хеш від prev‖pubkey‖seq‖payload), той самий
хеш-підхід у `backup.rs` (Buzhash CDC-блоки), FNV-1a `snapshot_root` у
`memory_store.rs`. Один примітив обслуговує **чотири** різні патерни одночасно:
audit-trail (ланцюг незмінний і верифіковний), дедуплікація (однаковий контент = один
запис), кеш-інвалідація (хеш змінився → перерахунок; не змінився → взяти з кешу),
consistent-hashing shard-key (той самий sha3_256 як точка на кільці). **Дія:** будь-яка
нова похідна структура (проєкція, індекс, кеш) повинна бути "чистим fold" логу, що
ключується цим самим хешем — інваріантність вже дає безкоштовну інвалідацію.

### 2.3 Log-is-truth (Kappa) + CQRS, pgrust як проєкція, ніколи джерело істини

`event_log.rs` — єдиний append-only лог, `order_machine::fold_transitions` — чиста
проекція-fold, `domain::apply_event` повертає новий `Order` (ніколи мутація на місці).
pgrust — це **читана/compliance проекція**, що будь-коли rebuildable з логу. Це
інвертує класичне "Postgres = істина, кеш = похідне": тут **лог = істина, pgrust =
кешована query-модель.**

### 2.4 Latency-lane vs throughput-lane — CPU-аналог TTFT/TPOT вже частково є

`spool.rs` (fire-and-forget append, backpressure через `is_full`, crash-safe
`reclaim`) — це вже готова ready-queue з правильною семантикою. `engine/src/loop_.rs`
вже реалізує саме той патерн, що continuous-batching scheduler робить на GPU: fixed
DT_STABLE кроки, **MAX_SUBSTEPS = 5** — жорсткий бюджет на важку роботу за тик, після
чого обов'язково `render(alpha)` — тобто "responsive lane ніколи не голодує позаду
throughput lane". Немає лише інтеграції: важкі kernel-операції (спектральний
розклад, backup, recall re-index) сьогодні виконуються **inline**, а не через spool з
таким самим bounded-substep drainer.

---

## 3. Хмара / IaC / DevOps

| Класична concept | Self-hosted еквівалент у dowiz (реальний файл) | Cloud = optional adapter |
|---|---|---|
| EC2 | systemd unit (`deploy/pgrust.service`) + Firecracker/KVM microVM (`isolation/microvm.rs`) + Nanos unikernel (`ports/github/ops.json -t hvt`) | тільки якщо нод живе поза Hetzner |
| Lambda | WASM-компоненти per-event (`ports/telegram`, `wasm32-wasip2`, `bebop2/wasm-host`) | не потрібен |
| S3 | `backup.rs::BlockStore` (sha3-адресований, Buzhash CDC dedup) — **порт уже є** | MinIO → R2 як холодний tier за тим самим трейтом |
| RDS | `pgrust` як native systemd (не контейнер — `check-no-docker.sh` це забороняє) | RDS лише як legacy-міграційний adapter |
| DynamoDB | `event_log.rs` + `spool.rs` append-log | embedded `redb`/`sled` за потреби |
| VPC | Cloudflare Tunnel (без публічного IP) + `internal: true` bridge | WireGuard mesh — правильний наступний крок для node↔node |
| IAM | `proto-cap` capability tokens (`SignedFrame`+`Scope`+`AnchorRoster`+`RevocationSet`) | немає — і не повинно бути (§11) |
| CloudWatch | `BEBOP_TELEMETRY_SINK=local`, systemd journal | опційний sink через WireGuard, ніколи прямий cloud phone-home |

**Головний конкретний розрив: IaC відсутній повністю** (`*.tf` — нуль файлів; сьогодні
— імперативний shell + ручний `systemctl enable`). Рекомендація: **OpenTofu** (MPL-форк
Terraform, без BSL vendor-lock) + `dmacvicar/libvirt` провайдер (`libvirt_domain` для
Firecracker/KVM-гостей — декларативний опис без жодного cloud-акаунту), remote-state
через `backend "pg"` на вже наявний pgrust (не S3+DynamoDB). Перший правий крок:
один `opentofu/` каталог з одним `libvirt_domain`-модулем — декларативно відтворює
один microVM-нод.

**Оркестрація контейнерів: НЕ Kubernetes.** Zero-OCI правило (`check-zero-oci.sh`,
`Dockerfile` фінальний stage `FROM scratch`) архітектурно виключає K8s, це не питання
моди. Драбина: systemd (є вже) → **Nomad** (один Go-бінарник, нативний
Firecracker/`raw_exec` driver, не потребує OCI) коли флот переростає ручний
`systemctl`-фан-аут (орієнтовно >5 нодів).

**Blue-green** = repoint Cloudflare Tunnel hostname (`service: localhost:8081`), не
ALB target-group swap. **Auto-scaling** = додавання підписаних нодів у
`AnchorRoster` (довірча операція, не ASG) — розмір флоту моделюється як IaC `for_each`,
не runtime-автоскейлер, бо призначення вже coordination-free через HRW-хешування
(§5.3).

---

## 4. Дані та бази даних

**12 патернів архітектури даних → найкраще підходить пара Data Mesh + Kappa, і це не
аналогія, а структурний збіг.** Data Mesh вимагає децентралізованого володіння
доменом + федеративне управління — саме це вже робить `MeshNode`: кожен нод володіє
своїм зрізом стану, приймає події через власний `DodGate` (governance-контракт на
рівні домену), reconciliation — peer-pull anti-entropy (`sync_pull`), без центрального
пайплайну. Kappa вимагає одного append-only логу як істини, де batch — лише replay
стріму: це буквально `event_log.rs` + `fold_transitions`. Lakehouse/Warehouse/Lambda-
архітектура — невірний вибір тут: вони передбачають центральний batch-контур, якого
dowiz свідомо не має.

**Нормалізація 1NF→BCNF/5NF майже не важлива на write-шляху, вирішально важлива на
read/fallback-шляху.** Подія в логу — незмінний, самодостатній факт; апдейт-аномалії
(вся суть 2NF/3NF) просто не застосовуються до append-only. Нормалізація повертається
рівно у двох місцях: pgrust-проекціях (read-моделі) і довідкових даних (каталог цін).
Конкретний приклад із домену: `OrderItem.unit_price` навмисно **денормалізований**
(знімок ціни "на момент покупки" — інакше замовлення забуде, що воно реально
коштувало), тоді як pgrust-проекція `products.current_price` має бути нормалізована в
один рядок. Правило: **денормалізуй незмінний історичний факт у логу; нормалізуй
змінну поточну істину в проекції.**

**CQRS/Event Sourcing/Database-per-service** — вже ~80% реалізовано. Command-сторона =
`assert_transition`+`commit_after_decide`; query-сторона = fold + (мала б бути)
персистована проекція. Database-per-service доведено до межі: кожен `MeshNode` володіє
власним `EventStore`, немає спільної БД, узгодження лише через `sync_pull`.

**Saga-патерн — реальний і значущий розрив.** `allowed_next` кодує тільки happy-path
(`InDelivery → [Delivered]`), **немає компенсуючих переходів** (`InDelivery →
CompensatedRefund` не існує), і `money.rs` не має reversal-примітиву (`checked_add`
без парного compensating credit). Замовлення, що охоплює кілька нодів
(merchant→courier→customer), — розподілена транзакція без 2PC і без компенсацій.
**Це стосується грошового/orders домену, який в проєкті red-line — пріоритет P0
(§10).**

**Sharding & consistent hashing:** природний shard-key — вже наявний `sha3_256`
(order_id або geohash з `geo.rs` для регіону) на hash-ring. Ребалансування при
join/leave нода перевикористовує вже наявний `sync_pull` (Merkle-root порівняння
відвантажує саме ті події, яких новому власнику бракує) — нічого не треба вигадувати
заново, лише додати ring-маршрутизацію поверх.

**Optimistic vs pessimistic locking — оптимістична конкурентність правильна, і вона
вже структурно є.** Append-only + content-addressed store детектує конфлікти без
локів: колізія content-id — ідемпотентний `Duplicate`, не пошкодження. Pessimistic
locks не перетинають mesh (немає спільного lock-manager між offline-first нодами).
Money-коректність тримається арифметикою (`checked_add`/`checked_add` currency-guard),
не виключенням. **Єдиний реальний розрив:** `Order` не має `version`/`updated_at` —
рекомендація: використати `(order_id, actor_seq)` як явний concurrency-токен замість
mutable version-поля.

**Малі структурні патерни — статус:** audit-trail (✅ вже сам лог), status-pattern
(✅ `OrderStatus`+`allowed_next`+ `FSM_GOLDEN_SIGNATURE` drift-gate), counter (✅
`actor_seq`, `spool::next_id`), soft-delete (✅ по-своєму — append-only ніколи не
видаляє, "видалення" = новий термінальний event; але `spool.rs::compact_drop` —
справжнє hard-delete, прийнятне лише для transient черги), cache (частково — pgrust-
проекція вже є кеш, а `FSM_GOLDEN_SIGNATURE` — кешований fingerprint), **pagination —
відсутня** (потрібен keyset/seek на `actor_seq`, не OFFSET — природно компонується з
watermark у `sync_pull`), **index — відсутній на pgrust-стороні** (реальні B-tree на
`(order_id, actor_seq)`, `status`).

---

## 5. Розподілені системи та мікросервіс-патерни

**Термінологічний конфлікт, який варто явно розрізняти:** "service mesh" (Istio/Envoy
— sidecar data-plane, централізовано конфігурований control-plane) і "mesh node"
(bebop2 — суверенний P2P peer) — це **не одне й те саме**, і bebop2 свідомо відкидає
модель централізованого control-plane саме тому, що вона повертає ту централізацію,
від якої mesh існує, щоб позбутись. Що зберігається — ідея уніфікованого data-plane
поруч з логікою: `proto-wire` (transport, framing, TLS, rate-limit) — це
**in-process sidecar-бібліотека**, вкомпільована в кожен нод, не позапроцесний Envoy.

**Client-server vs peer/mesh:** `MeshNode<T: Transport>` симетрично надає і
`connect()` (клієнтська роль), і `accept()` (серверна роль) над одним і тим самим
send/recv-каналом — вузол одночасно і клієнт, і сервер, авторитет подорожує з
повідомленням (`SignedFrame`), а не з рівнем.

**Message queue вже замінено:** `bpv7.rs` (RFC-9171 BPv7 store-and-forward) —
durable, at-least-once-delivery-with-idempotent-dedup чергу (`ack`/`mark_delivered`,
expiry за lifetime, dedup за nonce) — це вже те, що дав би Kafka/RabbitMQ між нодами,
без повторної централізації. **Caching — найсильніше підтверджений розрив у всьому
дослідженні** (§9): і меmory сама позначає "caching = only gap", і третій незалежний
кластер (roadmap gap-analysis, §9) те саме підтверджує, і kernel-синтез-кластер (§6.2)
знаходить конкретний кандидат (спектральний розклад Laplacian).

**Circuit breaker / bulkhead / strangler fig — вже реалізовані, по-своєму:**
`DodGate::admit_inbound` = per-event breaker (`Replay`/`Expired` → drop);
`TransportPolicy.max_concurrent_conns`(1024) + `TokenBucket` per-IP = bulkhead;
`iroh_transport` явно документований як "REPLACES the legacy zenoh.rs pub/sub stub" —
strangler fig у дії, як і весь Rust-переписів TS `apps/api`.

**GraphQL — залишити виключно на edge, ніколи не робити mesh-протоколом.** GraphQL
передбачає довірений resolver-шар, що відповідає на довільні клієнтські запити — тобто
client-server + ambient server authority, обидва відкинуті mesh-моделлю.
Capability-scoped binary frame не можна замінити GraphQL-запитом без здачі
deny-by-default моделі довіри.

**Clock skew без NTP — вирішено правильно вже сьогодні.** `event_log.rs` навмисно НЕ
впорядковує за wall-clock: `actor_seq` (per-actor монотонний логічний лічильник) +
content-addressing дають skew-immune причинність і replay. Wall-clock лишається лише
для двох речей: DOD lifetime-перевірка і `bpv7` bundle expiry — обидва мають
допускати **обмежене вікно розсинхрону (±δ)** замість точного порівняння, і жоден не
повинен вирішувати "чий запис виграє" (це робить `actor_seq`).

**MCP/A2A/ACP — вже реалізований референс, не гіпотеза.** `bebop mcp` — hand-rolled
JSON-RPC 2.0 stdio сервер, **нуль нових залежностей**, fail-closed, ~200 рядків. Це
еталон правила "тонкий wire-протокол, ніколи важкий SDK". `proto-cap/port.rs`'s
`InboundPort`/`OutboundPort`/`ChannelAdapter` з одним заявленим `Scope` +
deny-by-default `check_port_scope` — саме той шов, куди вбудовується A2A/ACP-адаптер:
зовнішній агент пред'являє підписану capability (ніколи reputation-score), і атенюація
enforced перевіркою підмножини скоупу, не документацією.

---

## 6. GPU/ML-інфраструктура: механіка → перенесення на CPU/kernel

Ключова причинно-наслідкова модель, яку варто тримати як лінзу для всього іншого:
**arithmetic intensity `I = FLOPs / bytes moved`**, roofline `min(Peak_compute, I ×
Peak_bandwidth)`. Нижче — де kernel-дизайн dowiz реально виграє від цієї лінзи, і де
чесно НЕ виграє.

### 6.1 Kernel fusion → злиття проходів пам'яті на CPU (registers/L1/L2/L3, не VRAM)

Три конкретні, верифіковані проти коду цілі:
- `spectral.rs::charpoly` — Faddeev–LeVerrier виконує `n` матричних множень, і
  обгортка `matmul` конвертує `Mat::from_vecvec` → `matmul_contig` →
  `.into_vecvec()` **на кожній ітерації** — ~2n зайвих алокацій `Vec<Vec<f64>>` на
  n-крокове обчислення. Фікс: тримати акумулятор в одному `Mat` через усі n кроків.
- `spectral.rs::graph_spectrum` розкладає двічі, і `laplacian` будує через
  `Mat`→`Vec<Vec>`→знову flat buf — три копії однієї матриці перед одним FLOP-ом
  власного розкладання.
- `csr.rs::energy` матеріалізує щільну O(n²) матрицю з розрідженої CSR лише заради
  скалярної відповіді.

**Де НЕ треба зливати:** `event_log.rs`→`order_machine.rs`→`money.rs` — це не
bandwidth-bound, а control-flow-bound (десятки байтів, уже в L1). Єдине справжнє
злиття тут — хешувати подію **один раз**, а не двічі (`commit_after_decide` і
`append` зараз обидва рахують `event_id()` на тих самих байтах).

### 6.2 KV-cache → кешування дороговартісних розкладів, ключоване `snapshot_root`

**Найсильніша знахідка всього дослідження**, підтверджена **незалежно двома різними
кластерами з різних напрямків:**
- Kernel-synthesis-кластер знайшов: `field_frame.rs`'s Laplacian `L` (§ проєктне
  рішення — той самий `L` водить recall/decay/layout/motion) обчислюється **заново
  щокроку** (`laplacian()`, matvec), хоча топологія — і отже `L` — не змінюється
  кадр-до-кадру. `bebop2/ARCHITECTURE.md` сам приписує ціль: "Store the operator as
  its **spectrum**… never the dense tensor… propagate(spectrum,t)=exp(-λt)/exp(iωt)".
- AI/математика-кластер незалежно вивів той самий висновок через фізику: власні
  значення `λ_k` Laplaciana відіграють структурно ту саму роль, що `ω²` в
  квантовому гармонічному осциляторі (обидва — власні значення квадратичної форми
  "жорсткості"); розклад по власних модах — те саме, що діагоналізація
  зв'язаних осциляторів на нормальні моди.

Обидва сходяться до одного рецепту: **розклади `L` один раз, кешуй базис за хешем
`snapshot_root` (примітив уже є в `memory_store.rs`), і коли граф незмінний — decay
(`exp(-λt)`), рух (`exp(iωt)`) і layout — усе точкове множення в кешованому
eigenbasis, замість O(n³) перерахунку щокадру.** Другий кандидат: PPR — і щільна
(`retrieval/ppr.rs`), і розріджена (`csr.rs::personalized_pagerank`) реалізації
рестартують з нуля щозапиту, хоча PPR лінійний за seed — кешування PPR з малого
hub-набору й композиція для нових seed'ів як зважена сума усуває це.

### 6.3 Interconnect-ієрархія (NVLink→NVSwitch→InfiniBand) → 3-рівнева locality-модель

```
(a) within-process   ~ns   engine↔kernel↔wasm через zerocopy/bridge — дозволена
                            дрібнозерниста, per-field, per-event балаканина
(b) within-host IPC   ~µs   декілька mesh-node процесів на одній машині —
                            дозволена гранулярність: per-batch (spool-семантика),
                            не per-field
(c) cross-host mesh   ~ms   + крипто-податок (ML-DSA verify на кожен фрейм) —
                            НІКОЛИ per-event round-trip; агрегувати в bundle
                            перед перетином межі
```

Механізм агрегації для рівня (c) уже є: `MeshEventSink::drain()` віддає `Vec<Event>`
батчем, DOD несе `expires_at` як bundle-lifetime (BPv7-стиль). Правило, що варто
закріпити явно: **рівень (c) повинен нести O(n) спектральний підсумок (коефіцієнти
в eigenbasis), ніколи O(n²) щільний тензор** — та сама ідея з §6.2, побачена з
іншого кінця: розклади один раз, відправляй коефіцієнти, відновлюй локально.

### 6.4 MIG → core-pinning + cgroups/CAT + NUMA (точніше, ніж просто "MIG-аналог")

MIG фізично виділяє SM+пам'ять з апаратною ізоляцією (жодного noisy-neighbor).
`isolation/microvm.rs::register_adapter` — це **admission gate, не партиціонер**:
сортує адаптери в `WasmComponent` (завжди прийнятий) чи
`NativeProcessRequiresKvm` (лише якщо `/dev/kvm` доступний), вирішує **чи** тенант
може працювати і **з якою силою ізоляції**, не виділяє ядра/пам'ять. Точний
відповідник MIG — **core-pinning (`sched_setaffinity`) + Intel RDT/CAT
(cache-partitioning) + NUMA-binding**; cgroups (`cpu.max`,`memory.max`) — це слабший
time-shared аналог (ближче до MPS, не MIG), бо тенанти й далі конкурують за спільний
L3/memory bus. Рятівний факт: кожен `MeshNode` вже є окремим процесом без спільної
mutable пам'яті — межа ізоляції вже є на рівні процесу, додати core-pinning/cgroup-
слайси на нод — інкрементальна зміна деплою, не переархітектура.

### 6.5 Continuous-batching scheduler → вже 80% зроблено, бракує інтеграції

`engine/src/loop_.rs::FixedTimestep::frame` — це вже prefill/decode-interleave: фіксовані
кроки фізики, **`MAX_SUBSTEPS=5`** — anti-starvation гарантія, і `render(alpha)`
завжди після. Правильна конструкція для kernel: latency-lane (синхронний inbound-подія
→ decide → append, мікросекунди, ніколи не чекає) + throughput-lane (`spool`-черга
для важких одноразових операцій: спектральний розклад, backup, recall re-index) з
drainer'ом, що переносить `loop_.rs`-ову ідею bounded substep — великий eigen-solve чи
backup ріжеться на шматки, і новоприбулий order-transition перебиває його між
шматками, а не чекає завершення всього перерахунку.

### 6.6 Batching (SIMD) → де переноситься, де ні

Переноситься: N Kalman-фільтрів кур'єрів мають ідентичну структуру `F,H,Q,R` —
struct-of-arrays + `f64x4`-лейни замінюють 50 окремих алокаційних штормів на кілька
векторизованих оновлень; `attention.rs::softmax` — текстбуковий SIMD-reduction;
`money.rs` (i64) — цілочисельний SIMD напряму (без плаваючої точки, зберігаючи
money red-line). **Де чесно НЕ переноситься:** аналог TTFT для delivery — це
time-to-order-accept, і клієнт, що чекає прийняття **єдиного** замовлення, не повинен
сидіти позаду batch-вікна для замовлень #2..#N. Batch — лише для non-critical
throughput-лейна (SIMD по кур'єрах/запитах під час idle-дренажу), ніколи для
одиничного замовлення на критичному шляху клієнта.

---

## 7. AI/ML математика та формальна верифікація

**Attention/Transformer:** `kernel/src/attention.rs` — це вже точна механіка
`softmax(QKᵀ/√d)·V` (з `scale=1/√d`, numerically-stable softmax через відняття row-max),
але **без навчених проєкцій** — свідомо reference scalar path, тренована частина
живе окремо в `micrograd`/`online`, тримаючи kernel non-AI. Оскільки функція
агностична до змісту рядків, той самий Q/K/V-механізм застосовний до
**нетекстових послідовностей**: подати останні події логу чи сусідні mesh-ноди як
q/k/v дає "увага до недавніх подій"/"увага до сусідів" безкоштовно — тести
`row_stochastic`/`equal_keys_is_mean` це підтверджують.

**Reverse-mode autodiff:** `micrograd.rs` — точно та машинерія, потрібна для
обчислення `∇L(θ)` будь-якого лосу (MSE, cross-entropy): DAG скалярних `Value`,
backward у зворотньо-топологічному порядку, кожна операція несе власне правило
похідної (`mul`→product rule, `div`→quotient rule, `pow`→степенева похідна).
Cross-entropy природний для класифікації тому, що `∂L/∂z_k = p_k − y_k` —
"передбачення мінус ціль" — чистий градієнт без насичення, на відміну від MSE поверх
softmax.

**Квантовий гармонічний осцилятор ↔ field_frame.rs — точна структурна відповідність,
не поетична.** Обидва — інстанси `(∂²/∂t² + Γ∂/∂t + c²A)u = s` для лінійного оператора
`A` з дійсними власними значеннями, розв'язувані діагоналізацією в eigenbasis. QHO:
`A` — оператор жорсткості потенціалу, власні значення `ω²`. Field operator:
`A = L` (граф-Лапласіан), власні значення `λ_k`, модальні частоти `ω_k² = (c²/M)λ_k`.
Це не аналогія заради краси — це той самий математичний об'єкт, і саме тому §6.2
(кешування спектрального розкладу `L`) отримало незалежне підтвердження з двох
різних дослідницьких напрямків.

**"Пряме доведення ірраціональності" як шаблон для формальної верифікації
протоколу.** Канонічний доказ (√2 ірраціональне через contradiction/infinite
descent: припусти протилежне в найменших членах → парність конфліктує з
gcd=1) має той самий скелет, що доказ безпеки consensus-протоколів: припусти
заборонений об'єкт існує → застосуй власні обмеження структури (quorum
intersection, монотонні лічильники) → вивідь неможливість. Це вже операціоналізовано
в проєкті через `tools/eqc`: кожна еквація компілюється в f64-шлях, fixed-point-шлях,
і **самоперевіряючу доказову програму** (наприклад `eqc-proofs/lambda_max_of_d.rs`
— `λ_max(L) ≤ 2·d_max`, машинно перевірений spectral-radius bound). Це і є
proof-by-contradiction, скомпільований і виконуваний, а не аргументований на папері.

**Конкретна пропозиція нового інваріанту для bebop2 в цьому ж стилі:** **I-FINAL** —
"два mesh-ноди ніколи не фіналізують конфліктний стан доставки для одного
замовлення." Доказ через quorum-intersection: два підписані кворуми `Q_A, Q_B` за
`n > 3f` перетинаються щонайменше в одному чесному вузлі; той вузол мусив підписати
дві різні фіналізації для одного `(order, epoch)`, що порушує правило "не більше
одного голосу на (order, epoch)" — суперечність. **Дія:** закодувати I-FINAL як
машинно-перевіряний предикат (TLA+/Apalache-інваріант, або eqc-стиль
self-asserting harness над функцією верифікації сертифікатів) — так само, як
`lambda_max_of_d` вже зроблено.

---

## 8. Git / CI-CD / Frontend

**Git-модель: GitHub Flow з жорсткою гейтованою main — не GitFlow, не буквальний
trunk-based.** Це не абстрактний вибір — memory вже фіксує реальний **інцидент
дивергенції** (`rebase --onto` recovery @e275dbce), і поточний стан — ~15
`feat/*`/`recover/*` гілок при відсутній локальній `main`. Діагноз: **довгоживучі
feature-гілки дрейфують від якоря.** Правило, що фіксує саме цей дрейф: короткоживучі
гілки (години-дні, не тижні) + **щоденний `git fetch && rebase origin/main`**. Два
репо (dowiz ↔ bebop-repo) координувати через **версіонований інтерфейсний контракт**
(UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT вже це і є), ніколи через submodules чи
синхронізовані назви гілок — це перетворює дві незалежні історії на одну
lockstep-точку відмови.

**9-й CI/CD-концепт (обраний свідомо, не генерично): secrets management, не feature
flags.** Обґрунтування прив'язане до проєкту, не абстрактне: memory вже фіксує
**реальний, повторюваний** інцидент із секретами ("SECURITY INCIDENT: creds in git
history", "4 unrotated pasted creds incl. CF API token", реальний `.env` з
OPENROUTER/JWT/COURIER_PII на диску). Це підтверджено **тричі незалежно**: тим самим
git/CI кластером, cloud/IaC-кластером (externalized config), і roadmap-gap-analysis
кластером (§9). Дія: gitleaks CI-гейт, секрети виключно через systemd
`EnvironmentFile`/Fly secrets, ніколи в репо чи логах CI.

**Frontend — усі перевірені позиції виявились розривами, не "вже зробленими".**
Класичний React (MenuPage.tsx хотспот з CLAUDE.md) переїхав у `attic/` на
`origin/main`; живий `/root/dowiz/web` — мінімальний WASM/physics shell з порожнім
dependency-set. Нуль debounce/throttle-утиліти, нуль query-cache бібліотеки
(react-query/SWR), нуль `BroadcastChannel`/`storage`-event коду. Для ordering/delivery
UI це конкретно означає: **друга вкладка може показувати застарілий статус
замовлення** (класичний server-state-як-local-state баг) і **stale-tab після logout**.
Рекомендація: server-state шар (stale-while-revalidate, dedup, refetch-on-focus) —
особливо цінний тому, що WS push (`attic/apps-api/src/websocket.ts`) вже існує як
джерело invalidation-подій; `BroadcastChannel` для крос-табової синхронізації статусу
замовлення й logout.

---

## 9. Довідкові репозиторії → конкретні розриви (не просто список)

Найцінніша знахідка з цього кластеру — **потрійна незалежна конвергенція на
"caching = єдиний найбільш підтверджений розрив"**: сама memory каже це
("caching = only gap", ecosystem arc), developer-roadmap-аналіз незалежно підтверджує
(Backend-трек вимагає caching-вузол, у kernel немає LRU/LFU/ARC/TinyLFU-примітиву), і
kernel-synthesis-кластер (§6.2) знаходить конкретного кандидата. Коли три незалежні
шляхи аналізу сходяться на одному висновку — це найсильніший можливий пріоритетний
сигнал у цьому дослідженні.

| Репо | Конкретна дія (не "варто подивитись") |
|---|---|
| **awesome-cheatsheets/tldr** | Генерувати `KERNEL-MAP.md` (модуль → одна фраза + `pub fn` список) і wire-format quick-ref **механічно з grep `pub fn`/`pub struct`** у CI, ніколи вручну (щоб не гнило) — це терсна проекція Knowledge Spine |
| **developer-roadmap** | Використано як gap-analysis: підтвердив caching, secrets-management, distributed-tracing, eval-harness-незавершеність (evals.rs — "9/11 organs stranded", вже в memory) як конвенційно-очікувані-але-тонкі місця |
| **awesome (sindresorhus)** | 4 конкретні sublists для DECART-порівняння: `awesome-rust` (проти mat.rs/householder.rs — чи не перевинаходимо nalgebra/faer невиправдано), `awesome-wasm` (проти wasm.rs/wasm-host), `awesome-distributed-systems` (проти sync_pull anti-entropy — де membership/SWIM бракує), `awesome-post-quantum` (проти ML-DSA-65 KAT-векторів) |
| **TheAlgorithms/Rust** | Grep-підтверджені відсутні алгоритми: **Dijkstra/A\* для маршрутизації кур'єра** (geo.rs лише проєктує позицію на вже задану полілінію, не обчислює маршрут — це продуктовий розрив, не інфраструктурний!), **Union-Find/DSU** (зараз ad-hoc BFS у cgraph.rs), **Bloom filter/Count-Min/HyperLogLog** (для mesh-шляху, де приблизність прийнятна — на відміну від навмисно точного retrieval-індексу), **MST** (Prim/Kruskal — для gossip/overlay spanning tree) |
| **project-based-learning** | Найчистіший невибудований розділ: **storage-engine (WAL→B-tree/LSM→MVCC)**. Сьогодні є лог (WAL-суміжний) і content-addressed blob store (`backup.rs`), але немає on-disk indexed структури, `fsync`/checkpoint протоколу, MVCC. Другий розділ: **mesh membership/DHT** (peer discovery явно "out of scope" у `iroh_transport.rs:23`) |

---

## 10. Пріоритезована дорожня карта

**P0 — робити найближче (кожен пункт підтверджений ≥2 незалежними кластерами або
прив'язаний до вже задокументованого інциденту):**

1. **Кеш спектрального розкладу `L`** (field_frame.rs), ключ = `snapshot_root`.
   Найсильніший сигнал дослідження — §6.2, потрійно/подвійно підтверджений.
2. **Виправити подвійне хешування** в `event_log.rs::commit_after_decide`/`append`
   — дешево, конкретно, знайдено прямо в коді (§6.1).
3. **Secrets management** (Vault/SOPS-еквівалент або мінімум gitleaks CI-гейт +
   systemd EnvironmentFile) — потрійно підтверджено, і вже реальний інцидент у
   memory (§8).
4. **IaC перший крок**: OpenTofu + один `libvirt_domain`-модуль, `backend "pg"`
   (§3) — закриває єдиний повністю відсутній домен.
5. **Git-дисципліна**: короткоживучі гілки + щоденний rebase на origin/main —
   напряму пояснює вже стався інцидент дивергенції (§8).
6. **Saga compensation edges** в `order_machine.rs` + reversal-примітив у
   `money.rs` — коректність у red-line домені (гроші/замовлення) (§4).
7. **I-FINAL інваріант** як eqc-стиль машинно-перевіряний доказ — дешевий перший
   крок (proof sketch), продовжує вже наявну VERIFIED-BY-MATH дисципліну (§7).

**P1 — цінне, середні зусилля:**

8. Core-pinning + cgroups/CAT для ізоляції тенантів (MIG-аналог, §6.4).
9. SIMD-batch некритичного числового лейна (Kalman по кур'єрах, attention-рядки),
   явно захищаючи латентність одиничного замовлення (§6.6).
10. Провести важкі kernel one-shot'и через `spool.rs` з bounded-substep drainer
    за зразком `loop_.rs::MAX_SUBSTEPS` (§6.5).
11. Consistent-hashing ring для order/region-власності поверх вже наявного
    HRW-хешування кур'єрів (§4, §5).
12. Завершити pgrust-проекцію: персистована read-модель, keyset-пагінація на
    `actor_seq`, реальні B-tree індекси, синтезований `updated_at` (§4).
13. Frontend-мінімум для web-застосунку: debounce/throttle, server-state шар
    (stale-while-revalidate), `BroadcastChannel` крос-табова синхронізація,
    коректний CORS+httpOnly cookie auth (§8).
14. Алгоритмічні розриви для маршрутизації і mesh-партиціювання: Dijkstra/A*,
    Union-Find, Bloom/Count-Min/HyperLogLog, MST (§9) — Dijkstra тут це
    **продуктовий**, не інфраструктурний розрив.
15. Distributed tracing — `Envelope.trace` вже є correlation-id, бракує лише sink
    (§5, §9).

**P2 — стратегічне, більший обсяг:**

16. Storage-engine розділ: WAL→LSM/B-tree→MVCC під логом/backup-сховищем (§9, §4).
17. Mesh membership/discovery: SWIM/HyParView gossip + DHT (зараз явно
    "out of scope" у коді) (§5, §9).
18. Формалізувати 3-рівневу locality-модель (§6.3) як явне інженерне правило з
    бюджетами пропускної здатності per-tier.
19. Nomad для мультинодового шедулінгу, коли флот >~5 нодів (§3).
20. DECART-гейтований пошук у awesome-rust/awesome-wasm/awesome-distributed-
    systems/awesome-post-quantum перед будь-яким ручним перевинаходом (§9).

---

## 11. Що свідомо НЕ впроваджувати

- **Managed cloud (AWS/RDS/EKS) як дефолт** — лише як опційний adapter за вже
  наявними трейтами (§2.1, §3).
- **Kubernetes** — zero-OCI правило архітектурно виключає; Nomad/systemd замість.
- **GraphQL як міжвузловий протокол** — лише edge/client-facing (§5).
- **IAM-стиль централізована репутація/ролі, чи будь-яка reputation/blacklist-
  довіра** — capability-токени, це вже раз обговорене й закрите архітектурне
  рішення (`NO-COURIER-SCORING`).
- **Буквальне прийняття GPU/CUDA** — GPU в стеку немає; цінність — у перенесенні
  дизайн-принципів на CPU, не в апаратному рішенні (§1, §6).
- **"Digital microcontroller" як опис поточного коду** — це північна зірка;
  числовий шар сьогодні алокаційно-важкий, що прямо суперечить цій моделі, поки
  P0/P1 §6.1, §6.4 не закриті.

---

## Джерела (агенти-дослідники, Opus, 2026-07-16)

Кожен розділ вище синтезований з grounded-звіту одного з 8 паралельних
дослідницьких агентів (cloud/IaC, data/DB, distributed-systems, GPU/ML-infra,
kernel-as-microcontroller, AI/math, git/CI/frontend, reference-repos), кожен читав
реальний код перед висновками. Повні звіти доступні в історії сесії; цей документ —
їх перехресний синтез, не конкатенація.
