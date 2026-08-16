# Повний Довідник: AI-інструменти, GitHub-репозиторії, роадмапи та концепції з відкритим кодом

> **Джерело A (Частини I–VII):** об'єднаний батч скріншотів Instagram Reels/Explore — акаунти sebastianhardy_, marc.kaz, simplifyinai, safesynt, datasciencebrain, power.ai, gittrend.io, fork.cast_, 100xengineers, ginacostag, pre_placement_preparations, sambit.ai.tech, ai_vatika, designteamofone, philosophyofphy, techketan.ai, artificialzone, gitscout.io та ін.
>
> **Джерело B (Частини VIII–XV):** 4 раунди обробки скріншотів Instagram Reels — акаунти evolving.qc, safesynt, noborta.ai, gittrend.io, howtowebdev_, agentic.james, nedz_reclassified, fork.cast_, bestapps.ai, artificialzone, aiproductlabs, eeanimation, phdhelp, codewithbrij, hexsecteam, quantscience_, datasciencebrain, marc.kaz, divyannshisharma, mindvergeai, simplifyinai, theartificialintelliges.
>
> **Дата фіксації:** серпень 2026. **Загалом:** 90+ унікальних інструментів, платформ, роадмапів та дослідницьких матеріалів з обох джерел, дублікати між частинами позначені перехресними посиланнями, а не видалені мовчки.
>
> **Методологія анотацій (Частини VIII–XV):** 🟩/📌 — пряма витяжка з джерела · 🟦/⊕ — збагачення понад джерело · ⚠ — епістемічна межа / непідтверджене або сумнівне твердження.

---

## ЗВЕДЕНИЙ ЗМІСТ

### Частина A — GitHub-репозиторії та тематичні добірки (Джерело A)
- **ЧАСТИНА I.** GitHub-репозиторії: комерційні аналоги з відкритим кодом (16 категорій)
- **ЧАСТИНА II.** Освітні роадмапи та інфографіки AI Engineering
- **ЧАСТИНА III.** Додаткові GitHub-інструменти
- **ЧАСТИНА IV.** Математика, фізика та creative coding
- **ЧАСТИНА V.** Кібербезпека — колекції та інструменти
- **ЧАСТИНА VI.** Репозиторії для запозичення та побудови продуктів
- **ЧАСТИНА VII.** Серія "10 GitHub repos competing with products" (artificialzone) + наскрізні концепції + зведені таблиці I–IV

### Частина B — AI-інфраструктура з Instagram Reels, 4 раунди обробки (Джерело B)
- **ЧАСТИНА VIII.** Контекст-менеджмент, LLM Gateway, коагентні фреймворки, self-hosted dev-інфраструктура, дизайн-системи, кібербезпека/red-team, self-hosted SaaS-альтернативи (Round 1, з наскрізними спостереженнями)
- **ЧАСТИНА IX.** Соло-розробники проти мільярдних імперій, інструменти якості AI-виводу, нові окремі інструменти, MCP-архітектури, фізика (Quantum State/Superposition), академічне письмо (Round 2)
- **ЧАСТИНА X.** Офензивний AI-тулінг, AI-редагування відео, self-hosted платформи, пам'ять AI-агентів, гібридні SLM+LLM роутери, ефективність інференсу, екстракція даних, ГІС (Round 3)
- **ЧАСТИНА XI.** Спеціалізовані застосунки та API-інтеграції — VoiceCoach, LibreChat, Camofox, CLI-Anything, Hyperframes, OmniParser та ін. (Round 4)
- **ЧАСТИНА XII.** Наскрізні спостереження за всіма чотирма раундами Джерела B

### Частина C — Перехресні зв'язки між Джерелом A та Джерелом B
- Дублікати, що зустрічаються в обох джерелах (позначені явно)
- Спільні наскрізні тези (router замість grep, solo-dev vs enterprise, приватність за замовчуванням)

---

# ЧАСТИНА A. ДЖЕРЕЛО A — GITHUB-РЕПОЗИТОРІЇ ТА ТЕМАТИЧНІ ДОБІРКИ

---

# ЧАСТИНА I. GitHub-репозиторії: комерційні аналоги з відкритим кодом

---

## 1. AI-аватари та цифрові двійники

### 1.1 Duix-Avatar (HeyGem) — «вбивця HeyGen»
- **Репозиторій:** `duixcom/Duix-Avatar`
- **Зірки:** 14.4k ★ | **Форки:** 2.4k | **Issues:** 405 відкритих | **PR:** 13
- **Опис:** Безкоштовний open-source аналог HeyGen. На вхід — одне відео обличчя користувача, на виході — говорящий аватар (talking avatar), що працює локально, на власному «залізі».
- **Теги:** ai-avatar, ai-avatars, cloning, cloning-tool, digital-human, multimodal-ai, video-generation, video-synthesis
- **Комерційний аналог:** HeyGen (від $29/міс)
- **Концепція:** Об'єднує face-cloning + TTS/voice-cloning + lip-sync рендеринг у єдиний офлайн-пайплайн. Ключова відмінність від SaaS-конкурентів — відсутність облачної обробки біометричних даних, що знімає питання приватності при роботі з обличчям людини.
- **Посилання:** https://github.com/duixcom/Duix-Avatar

---

## 2. CLI-інструменти та трейнінг-фреймворки

### 2.1 Soup
- **Автор:** 100xengineers
- **Зірки:** 678 ★ | **Завантаження:** 83.8k | **Версія:** v0.73.0
- **Ліцензія:** Apache-2.0 | **Python:** 3.10–3.12 | Без вендор-локу (no vendor lock-in)
- **Встановлення:**
  ```
  pip install "soup-cli[train]"
  soup init
  # → створює soup.yaml
  ```
- **Концепція:** CLI-фреймворк для конфігурування та запуску тренувальних пайплайнів (судячи з кроків Install → Configure та файлу soup.yaml) — декларативний підхід через YAML-конфіг замість написання окремих training-скриптів.

---

## 3. MCP (Model Context Protocol) — інфраструктура агентів

### 3.1 Build an MCP Host That Runs Five Servers at Once (туторіал)
- **Автор:** datasciencebrain
- **Верифіковано:** 2026-08-02
- **Стек:** `mcp==2.0.0`, `mcp-types==2.0.0`, `groq==1.6.0`, `streamlit==1.60.0`, Python 3.12, модель Groq `openai/gpt-oss-120b`
- **Архітектура (за діаграмою):**
  - **Streamlit-потік (синхронний):** User prompt → Streamlit page (app.py) — обмін через `run_coroutine_threadsafe` / `future.result()`
  - **Daemon-потік (один постійний asyncio event loop):** MCPHost (host.py) → ClientSessionGroup → 5 незалежних MCP-серверів-процесів
  - **5 серверів:** time (2 tools), fetch (1 tool), git (4 з 12 tools, allowlist), files (3 tools), notes (3 tools)
  - Побічні ефекти на диску: `workspace/*.md`, `notes.db` (переживає рестарт)
  - Три офіційні сервери запускаються через `uvx`, два — власні (in-project)
- **Ключова ідея:** Per-server allowlist скорочує 21 виявлений інструмент до 13, що реально рекламуються моделі — контроль поверхні атаки/шуму для LLM.
- **Зміст туторіалу (18 розділів):** What You're Building → Concepts → Architecture Overview → End-to-End Walkthrough → Prerequisites → Config → First MCP Server → Notes Server → Declaring Five Servers → Launch Parameters → One Namespace From Five Servers → The Host → Proving It Works → Translating MCP Into Groq → Tool-Calling Loop → Rendering → The Page → Run It End-to-End
- **Концепція для розширення:** Це практичний приклад патерну "MCP Host = оркестратор кількох MCP-серверів через один ClientSessionGroup" — актуально для Gortai-подібних мультиагентних систем Свята (аналогія з LangGraph orchestration).

---

## 4. Якість коду та пам'ять агентів

### 4.1 Sentrux
- **Автор:** simplifyinai
- **Версія:** Sentrux Pro v0.5.3
- **Опис:** "Сенсор", що допомагає AI-агентам замикати фідбек-петлю (feedback loop) — рекурсивне самовдосконалення якості коду.
- **Функція:** Дає AI-агентам **постійну пам'ять кодової бази** (permanent memory of codebase), 100% безкоштовно.
- **Концепція:** Вирішує проблему "втрати контексту" в агентних coding-workflow — агент пам'ятає структуру та історію правок кодової бази між сесіями, замість повторного індексування щоразу.

---

## 5. Логування

### 5.1 LogTape
- **Автор:** dahlia
- **Версія:** v2.3.0 | **npm:** v2.3.0 | **bundlephobia:** 429 | **codecov:** 86%
- **Опис:** Ненав'язлива (unobtrusive) логувальна бібліотека з нульовими залежностями для Deno, Node.js, Bun, браузерів та edge-функцій.
- **Демонстровано в консолі:** рівні логування trace/debug/info/warn/error(з exception)/fatal, автоматично налаштований meta-logger (категорія `["logtape", "meta"]`), помилки конфігурації sink виводяться окремо.
- **Посилання:** https://github.com/dahlia/logtape

---

## 6. Витяг структурованих даних (Data Extraction)

### 6.1 OmniParse — GitHub Find #377
- **Зірки:** 7.8k / 6.8k ★ (розбіжність у скрінах) | **Форки:** 541 | **Issues:** 62 відкритих | **PR:** 9 відкритих
- **Слоган:** "Extract Anything" / "Convert anything into structured data"
- **Опис:** Платформа, що приймає й парсить будь-які неструктуровані дані (документи, таблиці, зображення, відео, аудіофайли, вебсторінки) у структуровані, готові до дій дані, оптимізовані для GenAI/LLM-застосунків (RAG, fine-tuning тощо).
- **Вхідні типи (за діаграмою):** зображення, відео, документ (текст), аудіо, веб/структуровані джерела → OmniParse → структурований вихід

### 6.2 LangExtract — GitHub Find #382
- **Зірки:** 38.3k ★ | **PyPI:** v1.1.1 | **CI:** passing | **DOI:** 10.5281/zenodo.17015089
- **Слоган:** "Grounded Output"
- **Опис:** Витягує структуровані дані **з прив'язкою до джерела** (grounded/traceable extraction) — кожен витягнутий факт можна простежити до вихідного тексту.
- **Зміст README:** Introduction → Why LangExtract? → Quick Start → Installation → API Key Setup for Cloud Models → Adding Custom Model Providers → Using OpenAI Models → Using Local LLMs with Ollama → More Examples
- **Концепція:** На відміну від "чорноскринькового" LLM-парсингу, LangExtract підтримує локальні моделі (Ollama) і хмарні — тобто гнучкий вибір провайдера при збереженні grounding-гарантій.

---

## 7. Автоматизація браузера

### 7.1 Lightpanda Browser
- **Автор:** lightpanda-io
- **Зірки:** 32k / 32,125 ★ | **Discord:** 24 online
- **Ліцензія:** AGPL-3.0
- **Слоган:** "Machines deserve a 100x better web browser"
- **Опис:** Headless-браузер, побудований **з нуля** для AI-агентів та автоматизації. Не форк Chromium, не патч WebKit — новий браузер, написаний мовою **Zig**.
- **Ключові принципи:**
  - Built from scratch — не залежить від існуючих браузерів
  - Created with Zig — низькорівнева системна мова для продуктивності й ефективності
  - Focused and opinionated — заточений під headless-режим, без зайвого overhead
- **Заявлені переваги:** нижчий час виконання (execution time), нижчий пік пам'яті (memory peak) порівняно з Chrome (за демо-скріншотом дашборду)
- **Посилання:** https://github.com/lightpanda-io/browser

---

## 8. Фінансові foundation-моделі

### 8.1 Kronos
- **Автори:** Yu Shi та команда, Університет Tsinghua
- **Статус:** прийнято на **AAAI 2026** (не "вихідний GitHub-проєкт вихідного дня")
- **Зірки:** 32–33k ★ | **Форки:** 5.7k | **Ліцензія:** MIT
- **Слоган:** "A Foundation Model for the Language of Financial Markets"
- **Опис:** Перша **відкрита foundation-модель** для фінансових японських свічок (K-lines/candlesticks), натренована на даних з **45 глобальних бірж**.
- **Мультимовна документація:** Deutsch, Español, Français, 日本語, 한국어, Português, Русский, 中文
- **Концепція:** Аналогія до LLM-foundation-моделей, але доменом слугує "мова ринку" — послідовність K-line патернів як токени. Потенційно застосовна для transfer learning у квант-трейдингу без тренування з нуля.

---

## 9. Desktop / Wayland-композитори

### 9.1 Hyprland
- **Репозиторій:** `hyprwm/Hyprland`
- **Зірки:** 266 (у показаному пості) / загалом значно більше на GitHub
- **Ліцензія:** BSD-3-Clause | **Мова:** C++ (95.9%) | **PR:** 133 відкритих | **Issues:** 47 відкритих
- **Опис:** 100% незалежний, динамічний тайлінговий Wayland-компоузер, що не жертвує зовнішнім виглядом.
- **Ключові фічі:**
  - Eyecandy: градієнтні бордери, блюр, анімації, тіні
  - Глибока кастомізація
  - 100% незалежний: без wlroots, без libweston, без kwin, без mutter
  - Кастомні bezier-криві для анімацій
  - Потужна підтримка плагінів + вбудований plugin manager
  - Tearing support для кращого ігрового перформансу
  - Config перезавантажується миттєво при збереженні
- **Посилання:** https://github.com/hyprwm/Hyprland

---

## 10. Білінг для хостинг-бізнесів

### 10.1 Paymenter
- **Версія:** v1.5 Release
- **Слоган:** "Open-Source Billing, Built for Hosting"
- **Опис:** Автоматизація підписок, усунення хаосу білінгу для хостинг-бізнесу — без вендор-локу та прихованих витрат.
- **Позиціонування:** "Next-generation billing platform for modern hosting businesses" — спрощує білінг, транзакції, підтримує кастомізацію.
- **Категорії функцій:** Automation, Responsive, Performant, Personalization, Open-Source

---

## 11. Пошукові / answering-движки

### 11.1 Vane (колишня назва — Perplexica) — «вбивця Perplexity»
- **Репозиторій:** `ItzCrazyKns/Vane`
- **Зірки:** 36.1k ★ | **Форки:** 4.0k | **Issues:** 229 | **PR:** 117 | **Watching:** 199
- **Ліцензія:** MIT
- **Опис:** AI-powered answering engine. Питаєш будь-що — він шукає в живому вебі, читає сторінки і відповідає з прикріпленими джерелами. Ніхто не рахує ваші запити (на противагу платним лімітам SaaS).
- **Теги:** ai-agents, ai-search-engine, answering-engine, artificial-intelligence, llm, machine-learning, open-source-ai-search-engine, perplexica, rag, search-engine, searxng, searxng-copilot, self-hosted-ai, vane
- **Технологічна база:** інтеграція з SearXNG як мета-пошуковим движком
- **Посилання:** https://github.com/ItzCrazyKns/Vane

---

## 12. 3D-сканування / фотометрична стереоскопія

### 12.1 Lumen-PS
- **Слоган:** "Turn a flatbed scanner into a photometric-stereo material scanner"
- **Стек:** Python 3.11+ | Windows | Захоплення через WIA 2.0 | Прискорення CUDA + CPU | Ліцензія MIT
- **Опис:** Відновлює normal maps, albedo, roughness, height та alpha з чотирьох звичайних сканів — без камерної установки (rig), синхронізованого освітлення чи спеціальної оптики.
- **Концепція:** Photometric stereo — техніка реконструкції 3D-поверхні з кількох знімків того самого об'єкта під різними кутами освітлення. Тут "освітлювачем" виступає рухома лампа-каретка звичайного планшетного сканера, що робить дорогу технологію (PBR-текстурування для 3D/ігор) доступною на побутовому обладнанні.

---

## 13. Автономні AI-агенти

### 13.1 agenticSeek — «вбивця Manus»
- **Репозиторій:** `Fosowl/agenticSeek`
- **Зірки:** 26.8k ★ | **Форки:** 3.0k | **Issues:** 27 | **PR:** 6 | **Watching:** 167
- **Ліцензія:** GPL-3.0
- **Слоган:** "Fully Local Manus AI. No APIs, No $200 monthly bills."
- **Опис:** Дай завдання — і агент відкриває справжній браузер, проводить дослідження і копається у файлах самостійно. Все — на вашій машині (кошти витрачаються тільки на електрику).
- **Теги:** agentic-ai, agents, ai, autonomous-agents, deepseek-r1, llm, llm-agents, voice-assistant
- **Компоненти репозиторію:** llm_router, llm_server, searxng (docker+main compose), sources, prompts (з фільтром грубої лексики)
- **Посилання:** https://github.com/Fosowl/agenticSeek

---

## 14. Knowledge Management / RAG для команд

### 14.1 SurfSense — «Knowledge Grounded AI»
- **Зірки:** 15.8k ★ | **Ліцензія:** Apache-2.0 | **Reddit:** r/SurfSense, 165 online
- **Позиціонування:** OSS-альтернатива NotebookLM, Perplexity та Glean
- **Опис:** Підключає будь-яку LLM до внутрішніх джерел знань компанії і дозволяє чатити з ними в реальному часі разом з командою.
- **Підтримувані джерела (дуже широкий список):**
  Search Engines (SearxNG, Tavily, LinkUp), Google Drive, Slack, Microsoft Teams, Linear, Jira, ClickUp, Confluence, BookStack, Gmail, Notion, YouTube, GitHub, Discord, Airtable, Google Calendar, Luma, Circleback, Elasticsearch, Obsidian (і далі планується розширення)
- **Статус:** #1 Repository of the Day (GitHub Trending)
- **Мультимовність:** English, 简体中文
- **Концепція:** Це "конектор-хаб" рівня Glean/Perplexity Enterprise, але self-hosted — критично для компаній, які не можуть віддавати внутрішні дані в чужу хмару.

### 14.2 Semantica — «Open Source Palantir для AI-агентів»
- **Позиціонування:** Graph-Native Infrastructure for Context and Accountable AI Systems
- **Ліцензія:** MIT | **Статус:** #2 Repository of the Day
- **Опис:** Інструмент — безкоштовна відкрита версія технології рівня Palantir для AI-агентів.
- **Демонстровано (Graph Studio / Explorer):** граф на 645 вузлів, 1,081 ребро, 645 активних, "Distance Intelligence" heatmap-режим, приклад запиту "MTOR" (ген/білок) з опціями: Run Link Prediction, Provenance JSON/MD, Trace Path
- **Концепція:** Ключова відмінність від звичайних vector-based RAG — **graph-native** підхід із трасуванням походження (provenance) кожного факту й можливістю прогнозування зв'язків (link prediction). Це напряму резонує з інтересом Свята до "accountable AI" та graph-структур знань — можна розглянути як інфраструктурний шар для його батчевої бази знань.

---

## 15. Мітинг-асистенти

### 15.1 meetily — «вбивця Otter.ai»
- **Репозиторій:** `Zackriya-Solutions/meetily`
- **Зірки:** 29k ★ | **Форки:** 3.1k | **Issues:** 214 | **PR:** 130 | **Watching:** ~
- **Опис:** Приватний AI-асистент для мітингів. Сидить на ваших дзвінках, пише транскрипт і видає нотатки та action items. Аудіо ніколи не покидає ваш ноутбук.
- **Технології:** 4x швидша транскрипція на базі **Parakeet/Whisper** (live), speaker diarization, сумаризація через **Ollama**, 100% локальна обробка, немає потреби в хмарі
- **Сайт:** meetily.ai — позиціонується як "#1 self-hosted, open-source AI meeting note taker для macOS & Windows"
- **Теги:** ai, ai-meeting-assistant, llm, local-ai, mac, meeting-minutes, meeting-notes, offline-first, ollama, parakeet, privacy-focused, privacy-tools, rust, self-hosted, sortformer, speech-to-text, transcription, whisper, whisper-cpp, windows
- **Мова реалізації:** Rust (llama-helper компонент)
- **Посилання:** https://meetily.ai

---

## 16. Перевірка граматики

### 16.1 harper — «вбивця Grammarly»
- **Репозиторій:** `Automattic/harper`
- **Зірки:** 14.4k ★ | **Форки:** 556 | **Issues:** 586 | **PR:** 139 | **Watching:** 37 | **Releases:** 111
- **Ліцензія:** Apache-2.0 | **Мова реалізації:** Rust
- **Опис:** Виправляє граматику й покращує ваш текст всередині будь-якого застосунку, який ви вже використовуєте. Працює офлайн — текст ніколи не завантажується на сервер.
- **Позиціонування на сайті:** "Offline, privacy-first grammar checker. Fast, open-source, Rust-powered."
- **Теги:** chrome-extension, developer-tools, english-language, grammar-checker, nodejs, react, rust, svelte, webassembly
- **Сайт:** writewithharper.com
- **Автор проєкту:** Automattic (компанія-власник WordPress.com)

---

---

# ЧАСТИНА II. Освітні роадмапи та інфографіки AI Engineering

### II.1 AI Engineering Mastery Tree
- **Автор:** ginacostag
- **Опис:** Велике "дерево" з 12 гілок, що покриває повний стек AI-інженерії від основ до production. Кожна гілка — окремий кластер компетенцій.
- **Структура дерева:**
  1. **Foundation:** Git, Python, APIs, Linux, JSON, Cloud Basics
  2. **LLM Fundamentals:** Tokens, Attention, Fine-Tuning, Inference, Structured Outputs, Context Windows, System Prompts, Embeddings, Prompt Chaining
  3. **Prompt Engineering:** Zero-Shot, Few-Shot, Chain of Thought
  4. **RAG:** Chunking, Retrieval, Re-ranking, Citations, Embeddings, Vector Databases
  5. **AI Agents:** Planning, Memory, Tool Calling, Multi-Agent Systems, Reflection, Agent Workflows
  6. **Model Providers:** Meta, Google, Anthropic, OpenAI, Mistral, DeepSeek
  7. **Frameworks:** LangChain, AutoGen, Haystack, Pydantic AI, LlamaIndex, CrewAI
  8. **Databases:** PostgreSQL, Redis, Qdrant, Pinecone, Chroma
  9. **Deployment:** Docker, CI/CD, Monitoring, Kubernetes, Scaling, Serverless
  10. **Evaluation:** Hallucinations, Human Feedback, Rate Limits, Benchmarks, Tracing, Governance, Observability, Guardrails
  11. **Production AI:** Security, Caching, Cost Optimization, Reliability
  12. **Mastery:** AI Copilots, Voice Agents, AI Products, AI Chatbots, Autonomous Agents, AI Search Engines
- **Концепція:** Дерево показує, що "AI Engineering" — не одна дисципліна, а перетин класичного software engineering (Git, Docker, PostgreSQL), ML-специфіки (embeddings, fine-tuning) та нового шару "agent engineering" (planning, memory, tool calling, multi-agent). Корисно як чекліст для самооцінки прогалин.

### II.2 How to Build AI Agents from Scratch (10-крокова методика)
- **Автор:** pre_placement_preparations
- **Опис:** Практичний покроковий гайд побудови агента з нуля.
- **10 кроків:**
  1. **Define the agent's role and goal** — що робить агент, кому допомагає, який вихід генерує (приклад: медичний асистент, що читає рентген-знімки)
  2. **Design structured input & output** — Pydantic AI або JSON Schema; думати про агента як про API, уникати "брудного" тексту
  3. **Tune behavior & add protocol** — role-based system prompts, Prompt/Prefix Tuning, MCP для стандартизації → інструменти: MCP, GPT-4, Claude, Prompt Tuning
  4. **Add reasoning and tool use** — фреймворки ReAct (Reasoning + Action), Chain-of-Thought; доступ до web search, code interpreters, document retrievers → LangChain, OpenAI Tools, ReAct Framework
  5. **Structure multi-agent logic (за потреби)** — оркестрація ролей: Planner, Researcher, Reporter — кожен з власною input/output схемою → CrewAI, LangGraph, OpenAI Swarm
  6. **Add memory and long-term context (RAG)** — conversational memory, summary memory, vector-based memory → Zep, LangChain Memory, ChromaDB, FAISS
  7. **Add voice or vision capabilities (опційно)** — TTS через Coqui/ElevenLabs; image understanding через GPT-4o або LLaMA 3.2 Vision
  8. **Deliver the output** — форматування в Markdown → PDF/JSON, читабельний і парсабельний вивід → Pydantic AI, LangChain Parsers
  9. **Wrap in a UI** — front-end через Gradio, Streamlit або FastAPI ("це те, що перетворює агента на продукт")
  10. **Evaluate and monitor** — тестові промпти й toolchains для перевірки надійності, логи/бенчмарки/фідбек → MCP Logs, OpenAI Evaluation API, Custom Metrics Dashboards
- **Концепція:** Методика підкреслює порядок пріоритетів — спочатку контракт вводу/виводу (крок 2), і лише потім "розум" (reasoning, крок 4). Це дзеркалить принцип API-first дизайну в звичайній інженерії, перенесений на агентів.

### II.3 RAG vs KAG — дві архітектури для розширення знань AI
- **Автор:** pre_placement_preparations
- **RAG (Retrieval-Augmented Generation):** User Query → Vector Search (embedding, similarity) → Document Retrieval → Context Injection → LLM Generation
- **KAG (Knowledge-Augmented Generation):** User Query → Knowledge Graph / Structured Memory → Reasoning Layer (logic engine, decision nodes) → Context Construction (структуровані семантичні блоки) → LLM Generation
- **Порівняльна таблиця:**

| Критерій | RAG | KAG |
|---|---|---|
| Джерело знань | Зовнішні документи | Структурований граф знань |
| Швидкість оновлення | Fast | Moderate |
| Здатність до міркування | Limited | Strong |
| Складність | Lower | Higher |

- **Висновок з інфографіки:** Вибір між RAG і KAG залежить від того, що пріоритетніше — гнучкість (RAG) чи глибина міркування (KAG).
- **Концепція:** KAG вирішує головну слабкість класичного RAG — нездатність робити багатокроковий логічний висновок через розрізнені чанки тексту. Натомість граф знань зберігає explicit-зв'язки (consistency, concurrent relationships), що дозволяє logic engine будувати ланцюжки виводу. Прямий місток до Semantica (Частина I, п. 14.2) — Semantica, по суті, є інфраструктурним шаром саме для KAG-підходу.

### II.4 Vector vs Keyword vs Hybrid Search in RAG — The Complete Guide
- **Автор:** datasciencebrain (instagram.com/datasciencebrain)
- **Три архітектури пошуку:**
  - **Vector only:** query → embedding-model → vector-database → output
  - **Keyword only:** query → text/tokenizer → inverted-index → output
  - **Hybrid:** query → паралельно vector + keyword → fusion → re-ranker → output
- **Теза гайду:** Більшість RAG-туторіалів вчать "заембедити документи і сподіватись". Цей гайд натомість пояснює retrieval-шар так, як його розуміє людина, що дебажила його в проді — включно з тим, як тюнити, як виміряти якість, скільки це коштує і як зібрати частини в один робочий пайплайн.
- **Обіцяний результат:** уміння точно пояснити, чому keyword і vector пошук провалюються в протилежних ситуаціях — і більше ніколи не обирати неправильний варіант.
- **Концепція:** Vector search добре ловить семантичну схожість (синоніми, перефразування), але провалюється на точних збігах (номери деталей, коди помилок, власні назви) — тут виграє keyword/inverted-index. Hybrid із fusion + re-ranker — стандартна production-практика 2025–2026, що компенсує слабкі місця обох підходів.

### II.5 6 AI Engineering Skills (відео-конспект)
- **Автор:** sambit.ai.tech
- **6 навичок:** 1) RAG · 2) AI Agent · 3) LLMOps · 4) Evals · 5) AI Safety · 6) Guardrails
- **Концепція:** Компактна версія "Mastery Tree" (п. II.1) — фокус на 6 стовпах, які найчастіше запитують на технічних співбесідах у 2026: від retrieval-механіки до операційної безпеки (Safety/Guardrails) як обов'язкового, а не опційного, компонента.

### II.6 Python for AI Engineers — 18 тем
- **Автор:** ai_vatika
- **18 тем для освоєння:**
  1. Python Fundamentals (variables, operators, control flow, I/O)
  2. Functions (*args/**kwargs, lambda, closures, decorators)
  3. OOP (класи, наслідування, поліморфізм, інкапсуляція, magic methods)
  4. Data Structures (lists, tuples, dict, sets, collections: deque, Counter)
  5. Iterators & Generators (iter/next, yield, generator expressions)
  6. Functional Programming (map, filter, reduce, zip, enumerate)
  7. File Handling (читання/запис, CSV, JSON, pickle, pathlib, context managers)
  8. Exception Handling (try/except/else/finally, custom exceptions, logging, debugging)
  9. Modules & Packages (import, virtual environments, pip/poetry, __init__.py)
  10. Concurrency (threading, multiprocessing, asyncio, futures/executors)
  11. Memory Management (GIL, garbage collection, reference counting, weakref)
  12. Performance Optimization (timeit, cProfile, functools.lru_cache, Numba, векторизація)
  13. Type Hinting (typing module, generics, protocols, dataclasses/pydantic)
  14. Networking & APIs (requests, FastAPI, WebSockets, HTTP/REST, коди статусів)
  15. Database Programming (SQLite, PostgreSQL, SQLAlchemy ORM, transactions, connection pooling)
  16. Testing & Quality (pytest, unittest, mocking, mypy, black/ruff)
  17. Production Python (logging best practices, env variables, Docker, CI/CD, конфіг-менеджмент)
  18. AI Python Ecosystem (NumPy, Pandas, Matplotlib/Seaborn, Scikit-learn, PyTorch/TensorFlow, Hugging Face, LangChain, FAISS)
- **Концепція:** Список демонструє, що "AI Python" — це звичайний production-grade Python (concurrency, memory management, testing) плюс вузький ML-шар (п. 18), а не окрема мова. Тобто левова частка навичок (1–17) переноситься з будь-якого backend-досвіду.

### II.7 Gen AI Development Tech Stack
- **Автор поста:** datasciencebrain
- **Опис:** Велика мапа сучасного GenAI-стеку за 11 категоріями інструментів.
- **Категорії й представники:**
  - **LLMs:** OpenAI, Claude, Gemini, Llama, Qwen, DeepSeek, Mistral, Grok, Kimi, Gemma
  - **Open LLM Access:** Hugging Face, Ollama, Groq, Together AI, OpenRouter
  - **Frameworks & Agents:** LangChain, LlamaIndex, LangGraph, OpenAI Agents SDK, CrewAI, Pydantic AI, DSPy, Google ADK
  - **Data Extraction:** Docling, Firecrawl, Crawl4AI, Unstructured.io, LlamaParse, Mistral OCR
  - **Text Embeddings:** BGE-M3, Qwen3 Embedding, Jina AI, OpenAI, Voyage AI, Cohere
  - **Vector Databases:** pgvector, Qdrant, Chroma, Pinecone, Weaviate, Milvus, LanceDB, Redis
  - **Memory & Protocols:** MCP, Mem0, Zep, Letta
  - **Inference Serving:** vLLM, SGLang, llama.cpp, LM Studio
  - **Observability:** Langfuse, LangSmith, Arize Phoenix, Opik
  - **Evaluation:** (продовжується поза кадром)
- **Концепція:** Ця мапа — практично одна велика "розгортка" гілки RAG + AI Agents + Model Providers + Deployment з дерева Mastery Tree (Частина II.1), але з конкретними іменами інструментів замість абстрактних категорій компетенцій. Разом з Mastery Tree утворює пару "що вивчати" (II.1) → "чим саме користуватись" (II.7).

### II.8 Graph Engineering for Agentic AI
- **Автор поста:** codewithbrij
- **Слоган:** "From sequential loops to typed, parallel, reliable workflows"
- **Термінологія:** NODE = дія або сутність · EDGE = перехід або зв'язок · STATE = спільний контекст
- **1. Зсув парадигми:** Prompt (сформулювати ввід) → Context (курувати те, що бачить модель) → Loop (план → дія → спостереження → повтор) → **Graph** (розгалуження → паралельний запуск → синтез → маршрутизація). "Цикл (loop) — один прохід; граф розкриває цикл в явні шляхи."
- **2. Loop vs Graph:**
  - Sequential Loop: Plan → Act → Review → Retry (простий, низька конкурентність, менший overhead)
  - Parallel Review Graph: Planner → Worker → паралельно {Security, Logic, Style} → Synthesizer → Pass?/No → назад до Worker; Yes → Output (незалежні перевірки виконуються разом, фідбек повертається лише туди, де потрібно)
- **3. 5-стадійна методика:** Audit (інвентаризувати цикли, позначити ретраї, час очікування, витрати токенів) → Identify (знайти незалежну роботу й конкурентність) → Design (почати з 3–5 вузлів, визначити кожен маршрут) → Implement (асинхронне виконання, базовий час і вартість) → Type (додати семантичні ребра для багатокрокового міркування)
- **4. Коли що використовувати:** матриця "конкурентність × складність задачі" — прості одиночні цикли (low concurrency, simple task) до graph engineering (high concurrency, complex task, 3+ незалежні перевірки)
- **5. Типізовані ребра = знання:** SUPERSEDES (замінює), DEPENDS_ON (вимагає), DECIDED_BY (пояснює обґрунтування), CAUSED (простежує вплив), IMPLEMENTS (виконує контракт), REFERENCES (посилається на контекст) — "тип ребра пояснює агенту, чому дві речі пов'язані"
- **6. Hybrid Retrieval Router:** просте фактичне питання → vector search (швидко й дешево); багатокрокове/темпоральне питання по всьому корпусу → graph traversal (пов'язане міркування); resolution сутностей контролює довіру до багатокрокових висновків
- **7. Temporal Knowledge:** ребра SUPERSEDED_BY / VALID UNTIL, поля created_at/valid_from/valid_to/last_verified — "факти застарівають; зберігайте історію й походження"
- **8. Економіка/надійність/типові пастки:** вимірювати wall-clock час, pass rate, токени, вартість за успішне завершення; трасувати кожен вузол і ребро, checkpoints, timeouts, retry budgets; пастки — over-engineering, погана resolution сутностей, застарілі факти, нескінченні ретраї, "вибух" вартості фідбеку
- **9. Цикл роботи:** Draw → Implement → Run → Measure
- **Концепція:** Це, по суті, прикладна KAG-архітектура (Частина II.3) для agentic-систем — типізовані ребра з семантикою (SUPERSEDES, DEPENDS_ON тощо) є конкретною реалізацією "Knowledge Graph / Structured Memory" з KAG-порівняння. Прямий концептуальний місток до Гідравлічного Контуру Сенсу Свята — граф-based мультиагентна архітектура з явним типуванням переходів і збереженням "стану" (STATE) як спільного контексту, включно з часовою валідністю фактів (temporal knowledge), що резонує з питаннями стабільності й персистентності в holon-структурах.

---

# ЧАСТИНА III. Додаткові GitHub-інструменти

### III.1 WebToApp
- **Репозиторій:** `shiaho777/web-to-app`
- **Зірки:** 4.8k ★ | **Форки:** 695 | **Ліцензія:** Unlicense | **Android:** 23+
- **Автор поста:** gittrend.io
- **Опис:** Збирає Android APK з веб-проєктів прямо на телефоні. Не просто обгортка URL у WebView — повноцінна on-device "APK-майстерня".
- **Що робить нетиповим:**
  - Запускає справжні серверні рантайми на пристрої: Node.js, PHP, Python, Go, WordPress форкаються й виконуються (fork+exec) як нативні бінарники прямо зі сховища застосунку — URL-wrapper інструменти на таке не здатні взагалі
  - Постачає посилений anti-censorship мережевий стек: DNS-over-HTTPS, TLS
  - Підписує білди для публікації в Google Play
  - Запускає MV3 браузерні розширення
  - Все без ПК чи віддаленого build-сервера
- **Мультимовність:** English, 简体中文
- **Розділи README:** What's different · What you can build · Capability overview · Feature map · Module market · Architecture · Build
- **Концепція:** Радикальний зсув парадигми "no-code app builder" — замість хмарного CI/CD пайплайну весь build відбувається локально на смартфоні. Актуально для регіонів з обмеженим доступом до хмарних CI-сервісів або цензурованим інтернетом (звідси й anti-censorship network stack).

### III.2 turbovec
- **Репозиторій:** `RyanCodrai/turbovec`
- **Автор поста:** marc.kaz
- **Ліцензія:** MIT | **PyPI:** v0.7.0 | **crates.io:** v0.8.0 | **Paper:** arXiv
- **Слоган:** "Google's TurboQuant for vector search"
- **Опис:** Rust vector-індекс з Python-байндингами, побудований на алгоритмі Google Research **TurboQuant** — data-oblivious квантизатор, що досягає межі Шеннона (Shannon lower bound) для спотворення (distortion), без тренування кодбуку й окремої тренувальної фази.
- **Ключова цифра:** корпус з 10 мільйонів документів займає 31 ГБ RAM у float32 → turbovec стискає до **4 ГБ** і шукає швидше за FAISS.
- **Функції:**
  - **Online ingest** — додавання векторів індексується одразу, без тренувального кроку, без тюнінгу параметрів, без перебудов при рості корпусу
  - **Швидше за FAISS** — рукописні NEON (ARM) та AVX-512BW (x86) ядра обганяють FAISS на 12–20% на ARM і зрівнюються/перевершують на x86
  - **Фільтрація під час пошуку** — передача id-allowlist або slot-bitmask у search(), завжди повертає k результатів з дозволеної множини — без падіння recall
  - **Pure local** — без managed-сервісу, без витоку даних поза пристрій, підходить для повністю air-gapped RAG-стеку
- **Встановлення:**
  ```python
  pip install turbovec
  from turbovec import TurboQuantIndex
  index = TurboQuantIndex(dim=1536, bit_width=4)
  index.add(vectors)
  index.add(more_vectors)
  ```
- **Концепція:** Пряме практичне рішення проблеми "де зберігати ембеддинги", коли RAG-система росте до мільйонів документів. Data-oblivious квантизація (без окремого тренувального проходу типу product quantization з кодбуками) — ключова технічна відмінність від класичних методів компресії векторів.
- **Посилання:** https://github.com/RyanCodrai/turbovec

### III.3 brain.md
- **Автор поста:** techketan.ai
- **Ліцензія:** Apache-2.0 | **CLI:** zero dependencies
- **Опис:** Персистентний шар пам'яті для кодинг-агентів. Відкритий, agent-agnostic стандарт для фіксації довготривалих знань проєкту у вигляді простого Markdown — читається й записується через один невеликий CLI. Живе прямо в репозиторії й "подорожує" між агентами, машинами та моделями.
- **Підтримувані агенти:** Claude Code, Codex, Cursor, Pi
- **Концепція:** Вирішує ту саму проблему, що й Sentrux (Частина I, п. 4.1) — втрату контексту кодинг-агентом між сесіями — але іншим підходом: не векторна база чи граф, а простий, людиночитний Markdown-файл у репозиторії. Перевага: agent-agnostic (не прив'язаний до конкретного постачальника), git-friendly (diff-able, версіюється разом з кодом), нульові залежності.

### III.4 OpenEvolve
- **Автор поста:** techketan.ai
- **Опис:** Найпросунутіший (за заявою автора) open-source **evolutionary coding agent**. Перетворює ваші LLM на автономні code-оптимізатори, що відкривають проривні алгоритми.
- **Принцип роботи (псевдокод з README):**
  ```python
  def evolve(code):
      while not optimal:
          code = mutate(code)
          evaluate(code)
  ```
- **Концепція:** Класичний генетичний/еволюційний алгоритм (mutate → evaluate → select), але мутаційним оператором виступає LLM замість випадкових бітових змін. Це відрізняє OpenEvolve від "агента, що пише код один раз" — тут ітеративний цикл оптимізації з функцією фітнесу (evaluate), що дозволяє знаходити нетривіальні, контрінтуїтивні рішення (аналогічно AlphaEvolve від DeepMind).

### III.5 The Fable Method (The Fable Workflow)
- **Репозиторій:** `Sahir619/fable-method`
- **Автор поста:** gittrend.io
- **Ліцензія:** MIT | **Claude Code plugin:** v1.4.0 | **Checks:** passing
- **Слоган:** "think · act · prove"
- **Опис:** Задокументована методологія роботи **Claude Fable 5** (модель Anthropic, тимчасово недоступна через експортні обмеження — див. системну примітку про Fable/Mythos), зафіксована до її вилучення з підписки. Автор дистилював підхід моделі до розв'язання задач у набір навичок, які може виконувати будь-яка модель.
- **Чотири навички:**
  - **think** (fable-method) — класифікувати запит перед тим, як щось торкати; визначити "зроблено" через іменовану верифікацію; збирати докази паралельно з першоджерел
  - **act** (fable-loop) — комітитись до однієї рекомендації; змінювати найменшу коректну річ; перевіряти спостереженням
  - **prove** (fable-judge) — звітувати результат першим, з чесними застереженнями
  - **grow** (fable-domain) — генерує нові доменні адаптери так само, як спостерігали, що це робила модель-автор
- **Валідація:** 15 раундів оцінювання, понад 260 запусків агента, сліпі LLM-судді верифікують шляхом diff'ання й виконання коду (а не читання звітів). Кожен кейс — окрема історія в `eval/cases/`, повний лог у `eval/RESULTS.md`.
- **Ключова відмінність від звичайних agent-інструкцій:** "Більшість файлів з інструкціями для агентів кажуть моделі, *що* цінувати ("будь обережним, перевіряй свою роботу"). Цей — каже, *що робити*, так, щоб менш потужна модель могла слідувати буквально."
- **Концепція:** Приклад "дистиляції поведінки" — перетворення емерджентного, важко формалізованого стилю роботи потужної моделі на явний, тестований протокол, придатний для дешевших/менших моделей. Прямий місток до Fable/Mythos-контексту з системних нагадувань Claude.

### III.6 FingerprintJS
- **Репозиторій:** `fingerprintjs/fingerprintjs`
- **Автор поста:** gittrend.io
- **Ліцензія:** MIT
- **Опис:** Open-source client-side бібліотека браузерного фінгерпринтингу. Опитує атрибути браузера й обчислює хешований ідентифікатор відвідувача. На відміну від cookies й local storage, фінгерпринт залишається тим самим в інкогніто/приватному режимі й навіть після очищення даних браузера.
- **Демо:** https://fingerprintjs.github.io/fingerprintjs — показує, що ідентифікатор відвідувача не змінюється при переході в приватний режим.
- **Встановлення:**
  ```javascript
  npm install @fingerprintjs/fingerprintjs
  import FingerprintJS from '@fingerprintjs/fingerprintjs'
  const fpPromise = FingerprintJS.load()
  ;(async () => {
    const fp = await fpPromise
    const result = await fp.get()
    console.log(result.visitorId)
  })()
  ```
- **Концепція:** Технічно цікавий, але етично неоднозначний інструмент — фінгерпринтинг дозволяє відстежувати користувачів способом, що обходить звичайні механізми контролю приватності (видалення cookies, режим інкогніто). Варто розглядати і з точки зору захисту (напр. anti-fraud, запобігання повторній реєстрації), і з точки зору ризиків для приватності користувачів third-party сайтів.

---

# ЧАСТИНА IV. Математика, фізика та creative coding

### IV.1 A Calabi-Yau Slice — візуалізація Хансона (Hanson)
- **Автор поста:** philosophyofphy
- **Опис:** Візуалізація 2D-зрізу Ферма-квартики (Fermat quartic) при n=4: рівняння **z₁⁴ + z₂⁴ = 1**.
- **Технічні уточнення на самій інфографіці (важливо для точності):**
  - Це **2D-зріз у C² ≅ R⁴**, спроєктований для відображення — **не повний многовид вищої розмірності** (Calabi-Yau многовиди в загальному випадку значно вищої комплексної розмірності)
  - Автор явно попереджає: "What the reference actually shows" — тобто пост, ймовірно, коригує поширену помилкову інтерпретацію оригінального референсного зображення
- **Концепція:** Многовиди Калабі-Яу — ключовий об'єкт у теорії струн (compactification зайвих вимірів простору-часу до 6 компактних вимірів). Ферма-квартика — стандартний навчальний приклад через просту алгебраїчну форму рівняння, але саме тому легко неправильно зрозуміти, яку саме розмірність показує конкретна візуалізація. **Наскрізний зв'язок з батчевим тредом Свята:** це чергове звернення до теми "простір як обраний координатний базис/зріз" — тут в буквальному сенсі: те, що ми бачимо на картинці — довільно обраний R⁴-зріз вищовимірного комплексного многовиду, спроєктований у видиму форму, а не сам об'єкт.

### IV.2 TouchDesigner — hand tracking / skeleton detection
- **Автор поста:** designteamofone
- **Середовище:** TouchDesigner 2025.32820, файл `ascii.7.toe`
- **Опис:** Демонстрація трекінгу скелету рук у реальному часі (ASCII-візуалізація за назвою файлу) — на кадрі видно детекцію ключових точок обох долонь (кісточки пальців, зап'ястя) з накладеною bounding-box рамкою.
- **Технічний контекст:** робота супроводжується повідомленням про помилку рушія ("Error renaming node. Specify at least one alphabetic character") — типова робоча деталь ноду-базованого візуального програмування.
- **Саунд-доріжка посту:** alyzea — "aero garden"
- **Концепція:** TouchDesigner — нодовий візуальний movement/interaction-фреймворк, що часто використовує моделі комп'ютерного зору (напр. MediaPipe Hands) для перетворення руху тіла на керуючі сигнали для генеративної графіки/інсталяцій. На відміну від суто ML-репозиторіїв з попередніх частин, це приклад **прикладного** використання CV-моделей у creative coding, а не розробки самої моделі.

### IV.3 Wave Equation vs Wave Function
- **Автор поста:** eeanimation
- **Опис:** Порівняльна інфографіка, що розрізняє два часто плутані фізичні поняття.

| Аспект | Хвильове рівняння | Хвильова функція |
|---|---|---|
| Що це | Диференціальне рівняння, що описує поширення фізичної хвилі в просторі й часі | Комплекснозначна функція ψ(r,t), що описує квантовий стан системи |
| Математична форма | 1D: ∂²y/∂t² = v²∂²y/∂x²; загальна 3D: ∇²ψ − (1/v²)∂²ψ/∂t² = 0 | Часозалежне рівняння Шредінгера: iℏ∂ψ/∂t = [−(ℏ²/2m)∇² + V(r,t)]ψ(r,t) |
| Що описує | Еволюцію класичних хвиль (механічних, електромагнітних, звукових, водних) — швидкість, відбиття, заломлення, інтерференцію, стоячі хвилі | Еволюцію квантового стану, розподіли ймовірності, застосовується до мікроскопічних (квантових) систем |
| Розв'язок являє собою | Фізичну вимірювану величину (зміщення, тиск, поле E/B), що може набувати будь-якого значення | Амплітуду ймовірності; вимірювана величина — |ψ(r,t)|² → густина ймовірності |
| Інтерпретація | Опис поширення збурення; енергія розподілена неперервно; без внутрішнього ймовірнісного сенсу | Кодує всю статистичну інформацію про систему; |ψ|² дає ймовірності, не прямі вимірювані поля; містить амплітуду й фазу |
| Типові системи | Струни, що вібрують, водні хвилі, звукові хвилі, електромагнітні хвилі | Електрони в атомах/молекулах, частинки в потенційних ямах, квантові точки/наносистеми, кубіти |

- **Ключові відмінності одним рядком:** хвильове рівняння каже, *як* хвилі поширюються; хвильова функція каже, *який* квантовий стан і які ймовірності.
- **Концепція:** Плутанина між цими двома поняттями типова через схожу термінологію ("хвиля") і те, що обидва — хвильові за формою рівняння. Але хвильове рівняння — детерміноване і дійснозначне, а рівняння Шредінгера — комплекснозначне і принципово ймовірнісне (з невизначеністю Гейзенберга). Хвильова функція **не задовольняє** класичне хвильове рівняння — вона задовольняє окреме, першого порядку за часом рівняння Шредінгера.

---

### IV.4 Krylov Subspace Methods
- **Автор поста:** eeanimation
- **Опис:** Ітеративні методи для розв'язання великих розріджених лінійних систем Ax=b.
- **Структура інфографіки (10 блоків):**
  1. **Що таке підпростір Крилова:** K_m(A,r₀) = span{r₀, Ar₀, A²r₀, ..., A^(m-1)r₀} — простір, породжений послідовними діями матриці A на залишок (residual); наближені розв'язки будуються саме в цьому низьковимірному підпросторі
  2. **Загальна ідея:** x_m = x₀ + V_m y_m, де V_m — ортонормований базис K_m(A,r₀); метод шукає x_m, що мінімізує норму залишку або задовольняє умови ортогональності
  3. **Генерація базису:** a) процес Арнольді (для загальних матриць) — будує ортонормований базис і верхню Гессенбергову матрицю через AV_m = V_(m+1)H_m; b) процес Ланцоша (для симетричних матриць) — будує ортонормований базис і тридіагональну матрицю через тричленну рекурентність AV_m = V_mT_m
  4. **Основні методи Крилова:** CG (Conjugate Gradient) — для симетричних додатно визначених (SPD) матриць, мінімізує A-норму похибки; GMRES — для загальних несиметричних матриць, мінімізує норму залишку методом найменших квадратів; MINRES — для симетричних (можливо невизначених) матриць; BiCGSTAB — для загальних несиметричних, плавніша збіжність за BiCG; QMR/TFQMR — квазі-мінімальні залишкові методи
  5. **Погляд через проєкцію:** умова Гальоркіна (CG/симетричні методи) — залишок ортогональний до підпростору Крилова; ідея Петрова-Гальоркіна (загальні методи) — залишок ортогональний до іншого тестового простору
  6. **Чому вони потужні:** відмінно працюють з великими розрідженими матрицями; ключова операція — множення матриці на вектор; часто уникають прямої факторизації; добре підходять для PDE, FEM/FDM, обернених задач
  7. **Передобумовлення (preconditioning):** використання M⁻¹Ax = M⁻¹b з M≈A, але легко обертається — швидша збіжність, кластеризовані власні значення
  8. **Розуміння збіжності:** для CG швидка збіжність, коли власні значення A кластеризовані; оцінка похибки через число обумовленості κ; для GMRES зростання пам'яті/роботи з m — застосовується restarted GMRES(m)
  9. **Застосування:** скінченно-елементні/скінченно-різницеві симуляції, обчислювальна гідродинаміка, електромагнетика, будівельна механіка, машинне навчання й оптимізація, наукова та числова лінійна алгебра
  10. **Ключові висновки:** методи Крилова будують наближення зі степенів A, що діють на залишок; Арнольді й Ланцош — базові інструменти генерації підпростору; CG, GMRES, MINRES та BiCGSTAB — ключові практичні методи; передобумовлення часто критичне для ефективності
- **Концепція:** Методи Крилова — стандарт для розв'язання систем розміром у мільйони невідомих (типово для дискретизованих PDE), де пряма факторизація (LU/Cholesky) неможлива через обсяг пам'яті. Ключова ідея — уникнути явного формування чи факторизації матриці A, використовуючи лише операцію матрично-векторного добутку.

### IV.5 Classical Probability Distributions
- **Автор поста:** eeanimation
- **Опис:** Довідник із 12 фундаментальних розподілів ймовірності для дискретних та неперервних випадкових величин.
- **Дискретні розподіли:** Bernoulli(p), Binomial(n,p), Geometric(p), Negative Binomial(r,p), Poisson(λ), Hypergeometric(N,K,n) — кожен з формулою pmf, параметрами, μ (середнє) і σ² (дисперсія)
- **Неперервні розподіли:** Uniform(a,b), Exponential(λ), Normal/Gaussian(μ,σ²), Gamma(α,β), Beta(α,β), Chi-Square(k) — з формулою pdf
- **Де застосовуються:** контроль якості, вимірювання, природні явища; надійність, аналіз виживання (survival analysis), теорія черг; фінанси, страхування, машинне навчання, data science
- **Концепція:** Ці 12 розподілів — "будівельні блоки" статистики та теорії ймовірності, з яких конструюються складніші моделі (напр. суміші розподілів, байєсівські апріорні розподіли). Важлива структурна деталь — Gamma-розподіл узагальнює Exponential, а Chi-Square є частковим випадком Gamma; ці зв'язки корисні для розуміння, коли один розподіл "виникає" з іншого через граничні умови параметрів.

### IV.6 Quantum Circuit
- **Автор поста:** eeanimation
- **Опис:** Квантовий контур — послідовність квантових гейтів (воріт), застосованих до кубітів, що перетворює початковий стан на кінцевий через унітарну еволюцію.
- **Структура інфографіки (6 блоків):**
  1. **Анатомія контуру:** приклад на 3 кубітах (q0,q1,q2) — суперпозиція (Hadamard) → заплутування (CNOT) → однокубітні обертання (R_z, X) → заплутування й фаза → вимірювання → класичні біти
  2. **Однокубітні гейти:** H (Hadamard, створює суперпозицію), X (Pauli-X/NOT, біт-фліп), Y (Pauli-Y, біт+фаза фліп), Z (Pauli-Z, фазовий фліп), R_x/R_y/R_z (обертання навколо осей), T (π/8 фазовий гейт) — з матричними представленнями
  3. **Двокубітні гейти:** CNOT/CX (перевертає цільовий кубіт, якщо контрольний = |1⟩), CZ (застосовує Z до цілі, якщо контроль = |1⟩), CR_z(θ) (застосовує R_z(θ) до цілі, якщо контроль = |1⟩) — з 4×4 матрицями
  4. **Вимірювання:** вимірює кубіт у обчислювальному базисі {|0⟩,|1⟩}, видає класичний біт; ймовірності результатів P(0)=|α|², P(1)=|β|² для стану |ψ⟩=α|0⟩+β|1⟩
  5. **Як працює квантовий контур:** ініціалізація кубітів (|0⟩) → застосування гейтів (унітарні операції) → створення заплутування й інтерференції → вимірювання кубітів → зчитування класичних результатів
  6. **Ключові моменти:** квантові гейти оборотні (унітарні операції); заплутування уможливлює кореляції між кубітами; вимірювання колапсує квантовий стан; квантові контури — будівельні блоки квантових алгоритмів
- **Концепція:** Прямий місток до батчевої теми Свята "простір як обрана координатна система" — вектор стану кубіта на сфері Блоха задається кутами (θ,φ), а квантовий гейт — це обертання цього вектора у 2D комплексному гільбертовому просторі (для одного кубіта) чи в тензорному добутку таких просторів (для багатьох кубітів). "Вимірювання" — це проєкція вектора стану на один із базисних напрямків обраної системи координат (обчислювальний базис {|0⟩,|1⟩}), з ймовірністю результату, що визначається квадратом модуля координати в цьому базисі.

### IV.4 Krylov Subspace Methods
- **Автор поста:** eeanimation
- **Слоган:** "Iterative methods for large sparse linear systems"
- **1. Що таке простір Крилова:** Для системи Ax=b з початковим наближенням x₀ і нев'язкою r₀=b−Ax₀: Kₘ(A,r₀) = span{r₀, Ar₀, A²r₀, ..., Aᵐ⁻¹r₀}. Простір, породжений послідовними діями A на нев'язку; наближені розв'язки будуються в цьому низьковимірному підпросторі, що зростає з m.
- **2. Загальна ідея:** xₘ = x₀ + Vₘyₘ, де Vₘ — ортонормований базис Kₘ(A,r₀); метод шукає xₘ, що мінімізує норму нев'язки або накладає умови ортогональності.
- **3. Генерація базису:**
  - **Процес Арнольді** (загальні матриці): будує ортонормований базис Vₘ і верхню матрицю Хессенберга Hₘ: AVₘ = Vₘ₊₁Hₘ
  - **Процес Ланцоша** (симетричні матриці): будує базис і тридіагональну матрицю Tₘ через трьохчленну рекурсію: AVₘ = VₘTₘ
- **4. Основні методи Крилова:**

| Метод | Найкраще для | Визначальна властивість |
|---|---|---|
| CG (Conjugate Gradient) | Симетричні позитивно визначені (SPD) матриці | Мінімізує A-норму помилки |
| GMRES | Загальні (несиметричні) матриці | Мінімізує норму нев'язки (найменші квадрати) |
| MINRES | Симетричні (можливо невизначені) | Мінімізує норму нев'язки |
| BiCGSTAB | Загальні несиметричні | Плавніша збіжність за BiCG |
| QMR/TFQMR | Загальні несиметричні | Квазі-мінімальний залишок, гарна робастність |

- **5. Погляд через проєкцію:** Умова Гальоркіна (CG/симетричні методи): rₘ ⊥ Kₘ(A,r₀); ідея Петрова-Гальоркіна (загальні методи): rₘ ⊥ Lₘ (можливо інший тестовий простір)
- **6. Чому вони потужні:** відмінні для великих розріджених матриць; матрично-векторні добутки — основна операція; часто уникають прямої факторизації; добре підходять для PDE, FEM/FDM, обернених задач
- **7. Передобумовлення:** M⁻¹Ax = M⁻¹b; мета — швидша збіжність, кластеризовані власні значення, покращена обумовленість
- **8. Розуміння збіжності:** для CG похибка обмежена через κ=λmax/λmin; для GMRES норма нев'язки монотонно незростаюча, але зростає пам'ять/робота з m (~O(mn)) — звідси Restarted GMRES(m)
- **9. Застосування:** скінченні елементи/різниці, обчислювальна гідродинаміка, електромагнетизм, структурна механіка, ML та оптимізація, наукова й чисельна лінійна алгебра
- **10. Ключові висновки:** методи Крилова будують наближення зі степенів A, діючих на нев'язку; ключові для великих розріджених систем; Арнольді й Ланцош — базові інструменти генерації підпростору; передобумовлення часто критичне для ефективності
- **Концепція:** Методи Крилова — стандартний спосіб розв'язувати системи з мільйонами невідомих (типово в PDE-симуляціях), де пряма факторизація (LU/Cholesky) неможлива через розмір матриці. Ключова ідея — уникнути роботи з повною матрицею A, обмежившись лише матрично-векторними добутками, що робить методи природно придатними для розріджених (sparse) систем.

### IV.5 Classical Probability Distributions
- **Автор поста:** eeanimation
- **Слоган:** "Fundamental models for discrete and continuous random variables"
- **Дискретні розподіли (1–6):**
  1. **Bernoulli(p)** — одне випробування, два наслідки; μ=p, σ²=p(1−p)
  2. **Binomial(n,p)** — кількість успіхів у n незалежних випробуваннях Бернуллі; μ=np, σ²=np(1−p)
  3. **Geometric(p)** — кількість спроб до першого успіху; μ=1/p, σ²=(1−p)/p²
  4. **Negative Binomial(r,p)** — кількість спроб до r-го успіху; μ=r/p, σ²=r(1−p)/p²
  5. **Poisson(λ)** — кількість подій за фіксований інтервал часу/простору; μ=λ, σ²=λ
  6. **Hypergeometric(N,K,n)** — кількість успіхів у n витягах без повернення з популяції; μ=nK/N
- **Неперервні розподіли (7–12):**
  7. **Uniform(a,b)** — всі значення на [a,b] рівноймовірні; μ=(a+b)/2
  8. **Exponential(λ)** — час між подіями в процесі Пуассона; μ=1/λ
  9. **Normal/Gaussian(μ,σ²)** — найважливіший статистичний розподіл, моделює час очікування
  10. **Gamma(α,β)** — узагальнює експоненційний розподіл
  11. **Beta(α,β)** — визначений на [0,1], моделює пропорції
  12. **Chi-Square(k)** — сума квадратів k незалежних стандартних нормальних величин
- **Де застосовуються:** контроль якості, вимірювання, природні явища; надійність, аналіз виживання (survival analysis), теорія черг; фінанси, страхування, машинне навчання, data science
- **Концепція:** Розподіли утворюють будівельні блоки статистики й ML — від простого підкидання монети (Bernoulli) до фундаменту статистичного тестування (Chi-Square, Normal). Практично значуще: Poisson і Exponential пов'язані (час між Пуассонівськими подіями — експоненційний), а Gamma — узагальнення обох (сума k експоненційних інтервалів).

### IV.6 Quantum Circuit
- **Автор поста:** eeanimation
- **Означення:** Квантова схема — послідовність квантових вентилів (gates), застосованих до кубітів, що трансформує початковий стан у кінцевий через унітарну еволюцію.
- **1. Анатомія квантової схеми (приклад на 3 кубітах q0,q1,q2):** Superposition (H, Ry(φ)) → Entanglement (CNOT) → Single-qubit Rotations (Rz(θ), X) → Entanglement & Phase (CNOT, T) → Measurement → класичні біти c0,c1,c2
- **2. Поширені одно-кубітні вентилі:**

| Вентиль | Назва | Дія |
|---|---|---|
| H | Hadamard | Створює суперпозицію |
| X | Pauli-X (NOT) | Bit flip |
| Y | Pauli-Y | Bit і phase flip |
| Z | Pauli-Z | Phase flip |
| Rₓ(θ), Rᵧ(θ), R_z(θ) | Rotation | Обертання навколо осі X/Y/Z |
| T | T Gate (π/8) | π/8 фазовий вентиль |

- **3. Поширені дво-кубітні вентилі:** CNOT (CX) — перевертає target-кубіт, якщо control дорівнює |1⟩; CZ — застосовує Z на target, якщо control дорівнює |1⟩; CRz(θ) — застосовує Rz(θ) на target, якщо control активний
- **4. Вимірювання:** вимірює кубіт у обчислювальному базисі {|0⟩,|1⟩}, виводить класичний біт; ймовірності наслідків P(0)=|α|², P(1)=|β|² для стану |ψ⟩=α|0⟩+β|1⟩
- **5. Як працює квантова схема:** Ініціалізувати кубіти (|0⟩) → застосувати квантові вентилі (унітарні операції) → створити заплутаність і інтерференцію → виміряти кубіти → зчитати класичні результати
- **6. Ключові моменти:** квантові вентилі оборотні (унітарні операції); заплутаність уможливлює кореляції між кубітами; вимірювання колапсує квантовий стан; квантові схеми — будівельні блоки квантових алгоритмів
- **Концепція:** Квантова схема — операційна мова квантових обчислень, аналогічна логічній схемі в класичних обчисленнях, але з принциповою відмінністю: вентилі оборотні (унітарні), а не булеві (AND/OR/NOT незворотні). Заплутаність (entanglement), яку створює CNOT, — ресурс, що не має класичного аналога і лежить в основі квантової переваги (quantum advantage) для певних класів задач.

### IV.7 Quantum Superposition
- **Автор поста:** eeanimation
- **Означення:** Квантова система може існувати в кількох можливих станах одночасно до моменту вимірювання.
- **1. Означення:** |ψ⟩ = α|0⟩ + β|1⟩, де |α|² і |β|² — ймовірності вимірювання наслідків |0⟩ та |1⟩; нормалізація: |α|²+|β|²=1
- **2. Класичний біт vs квантова суперпозиція:** класичний біт — детермінований (точно 0 або точно 1); кубіт у суперпозиції — ймовірнісний, перебуває в комбінації |0⟩ та |1⟩ до вимірювання
- **3. Інтуїція сфери Блоха:** будь-який чистий стан кубіта: |ψ⟩=cos(θ/2)|0⟩+e^(iφ)sin(θ/2)|1⟩; північний полюс (θ=0)→|0⟩, південний (θ=π)→|1⟩, екватор (θ=π/2)→рівні суперпозиційні стани
- **4. Вимірювання:** до вимірювання — суперпозиція можливостей; після — стан колапсує до однієї з базисних станів (Result: |0⟩ з ймов. |α|² АБО Result: |1⟩ з ймов. |β|²)
- **5. Фізичні приклади:** спін-1/2 частинка (спін вгору/вниз одночасно), поляризація фотона (горизонтальна/вертикальна одночасно), дворівневий атом (збуджений/основний стани одночасно)
- **6. Чому це важливо:** інтерференція — суперпозиційні стани інтерферують, даючи конструктивні/деструктивні патерни; квантові обчислення — суперпозиція дозволяє представляти багато станів одночасно; квантові алгоритми (Шор, Гровер) використовують суперпозицію й інтерференцію для прискорень над класичними методами
- **7. Ключові висновки (таблиця):** Superposition (кубіт існує в комбінації станів) → Probabilistic Nature (наслідки випадкові, але передбачувані ймовірнісно) → Measurement Collapse (вимірювання змушує стан до однієї базисної) → Power (уможливлює інтерференцію, паралелізм, квантові прискорення)
- **Концепція:** Суперпозиція — не "невідомість" стану (як у класичній теорії ймовірностей, де монета вже впала, просто ми не бачили), а принципова онтологічна властивість: система дійсно перебуває в лінійній комбінації станів, що підтверджується інтерференційними експериментами (напр. подвійна щілина). Ключова відмінність квантової ймовірності від класичної — наявність фази (комплексних амплітуд), що уможливлює інтерференцію.

### IV.8 Quantum State
- **Автор поста:** eeanimation
- **Означення:** Квантовий стан повністю описує фізичний стан квантової системи; містить всю інформацію, потрібну для передбачення наслідків вимірювання.
- **Нотація:** |ψ⟩ — вектор стану (ket); ⟨ψ| — дуальний вектор (bra); ⟨ψ|φ⟩ — внутрішній добуток
- **1. Що таке квантовий стан:** стан представлений нормалізованим вектором |ψ⟩∈H у комплексному гільбертовому просторі H, ⟨ψ|ψ⟩=1. Приклади: спін-1/2 частинка |↑⟩,|↓⟩; кубіт |0⟩,|1⟩; поляризація фотона |H⟩,|V⟩
- **2. Суперпозиція:** |ψ⟩=Σαᵢ|i⟩, де αᵢ∈C — ймовірнісні амплітуди з нормалізацією Σ|αᵢ|²=1; система перебуває в усіх базисних станах одночасно до вимірювання
- **3. Візуалізація на сфері Блоха:** аналогічно IV.7
- **4. Вимірювання:** вимірювання колапсує стан до одного з власних станів вимірюваного спостережуваного; для базису {|i⟩}: P(i)=|⟨i|ψ⟩|²
- **5. Спостережувані й очікуване значення:** спостережувана представлена ермітовим оператором Ô; очікуваний результат ⟨Ô⟩=⟨ψ|Ô|ψ⟩. Для кубіта будь-яку спостережувану можна записати через матриці Паулі: Ô=n⃗·σ⃗=nₓσₓ+nᵧσᵧ+n_zσ_z, власні значення ±1
- **6. Заплутані стани (мультикубітні):** складені системи можуть мати заплутані стани, що не можна записати як добуток станів підсистем. Приклад — стан Белла: |Φ⁺⟩=(|00⟩+|11⟩)/√2 — наслідки вимірювання корельовані незалежно від відстані між кубітами
- **Ключові висновки:** квантовий стан — нормалізований вектор у гільбертовому просторі; підкоряється принципу суперпозиції; вимірювання дає ймовірнісні наслідки й колапсує стан; спостережувані — оператори, очікувані значення — ⟨ψ|Ô|ψ⟩; мультичастинкові стани можуть бути заплутаними; квантовий стан містить *всю* інформацію про систему
- **Спільні фізичні реалізації:** спін-1/2 частинки, два енергетичні рівні (атом), надпровідні кубіти, захоплені іони, поляризація фотона
- **Пов'язані концепції:** Гільбертів простір · Оператори · Унітарність · Матриця густини · Декогеренція
- **Концепція:** Ця інфографіка — концептуальний "батько" для IV.6 (Quantum Circuit) і IV.7 (Quantum Superposition): квантовий стан — фундаментальний об'єкт, суперпозиція — його властивість, а квантова схема — спосіб маніпулювати ним через унітарні оператори. Три інфографіки eeanimation разом утворюють цілісний виклад основ квантових обчислень від математичного об'єкта (стан) через його ключову властивість (суперпозиція) до операційної мови маніпуляції (схема).

---

# ЧАСТИНА V. Кібербезпека — колекції та інструменти

### V.1 Awesome-Hacking
- **Репозиторій:** `hack-with-github/awesome-hacking`
- **Автор поста:** githubradar
- **Зірки:** 177.7k ★
- **Опис:** Підбірка списків корисних ресурсів для хакерів, пентестерів і дослідників безпеки — інструменти, книги, документація, навчальні матеріали по різних напрямках кібербезпеки.
- **Концепція:** "Awesome list" — мета-репозиторій, що не містить коду, а лише курований перелік посилань за темою. Такий формат є де-факто стандартом навігації в GitHub-екосистемі для швидкого пошуку "що взагалі існує" в конкретній ніші.

### V.2 SecLists
- **Репозиторій:** `danielmiessler/seclists`
- **Автор поста:** githubradar
- **Зірки:** 72.7k ★
- **Слоган:** "The Pentester's Companion"
- **Опис:** Колекція словників і списків для тестування безпеки — логіни, паролі, URL, паттерни чутливих даних, пейлоади для фаззингу, веб-шели та багато іншого. Мета — дати тестувальнику безпеки можливість підтягнути цей репозиторій на нову тестову машину й одразу мати весь потрібний матеріал.
- **Використовується разом з:** Gobuster, Hydra, Burp Suite
- **Концепція:** Практична "паливна база" для автоматизованого пентесту — замість написання власних wordlist'ів з нуля, інструменти брутфорсу (директорій, паролів, параметрів) підключають готові, регулярно оновлювані списки з SecLists.

### V.3 HackTricks
- **Репозиторій:** `HackTricks-wiki/hacktricks`
- **Автор поста:** githubradar
- **Зірки:** 12k ★ | **Форки:** 3.2k | **Watch:** 227
- **Опис:** Обширна вікі-енциклопедія з пентестингу — техніки, прийоми й трюки з CTF-змагань, реальних застосунків та досліджень з кібербезпеки. Підтримує багато мов, доступна як сайт (book.hacktricks.xyz) і офлайн через Docker.
- **Концепція:** На відміну від SecLists (сирі дані) чи Awesome-Hacking (список посилань), HackTricks — це структурований **навчальний контент** із поясненнями методології, а не просто набір ресурсів.

### V.4 PayloadsAllTheThings
- **Репозиторій:** `swisskyrepo/PayloadsAllTheThings`
- **Автор поста:** githubradar
- **Зірки:** 79.8k ★
- **Слоган:** "Web Application Security, Pentest and Red Team Cheatsheet"
- **Опис:** Велика колекція корисних пейлоадів і обходів захисту для тестування безпеки веб-застосунків і CTF-змагань. Кожен розділ містить опис вразливості, спосіб експлуатації та приклади для Burp Intruder.
- **Концепція:** На відміну від SecLists (переважно wordlists), PayloadsAllTheThings фокусується на **техніках експлуатації** конкретних класів вразливостей (SQLi, XSS, SSRF, XXE тощо) з готовими до копіювання payload-рядками.

### V.5 Awesome Bug Bounty
- **Репозиторій:** `djadmin/awesome-bug-bounty`
- **Автор поста:** githubradar
- **Зірки:** 5.8k ★
- **Опис:** Список офіційних програм баг-баунті від компаній (легальний пошук вразливостей за винагороду) і розборів знайдених вразливостей від дослідників безпеки.
- **Розділи:** Getting Started · Write Ups & Authors · Platforms · Available Programs
- **Концепція:** Місток між технічними знаннями (HackTricks, PayloadsAllTheThings, SecLists) і легальною монетизацією цих навичок — програми баг-баунті дозволяють застосовувати пентест-техніки без юридичних ризиків, за офіційним дозволом компанії.

### V.6 ESP32-BlueJammer
- **Репозиторій:** `EmenstaNougat/ESP32-BlueJammer`
- **Автор поста:** githubprojects (серія "10 GitHub repositories that went viral", позиція №01)
- **Опис:** Проєкт дослідження бездротової безпеки на базі ESP32 та nRF24 для експериментів з інтерференцією в комунікаціях на частоті 2.4 ГГц.
- **Категорії:** Hardware · Security
- **Концепція:** Апаратний (а не суто програмний) інструмент безпеки — досліджує вразливість Bluetooth/Wi-Fi/2.4ГГц-протоколів до навмисних перешкод (jamming) на фізичному рівні радіохвиль, а не на рівні протоколів/додатків, як решта інструментів цієї частини.

### V.7 Серія "7 GitHub Repositories for Security Researchers" (hexsecteam)

**Загальна концепція серії:** На відміну від Частини V.1–V.5 (курировані списки й довідники), ця серія — семеро **AI-посилених** інструментів безпеки, де LLM вбудовано безпосередньо в робочий процес пентестера/OSINT-дослідника.

- **BruteForceAI** — LLM-Powered Login Testing
  - Опис: Інструмент пентестингу, що використовує LLM для аналізу форм логіну й автоматичного визначення селекторів. Підтримує brute-force і password-spraying тести, багатопоточність, проксі, рандомізовані затримки, логування й нотифікації.
  - Версія: v1.0.0 | Python 3.8+ | Ліцензія: Non-Commercial | AI-Powered
  - Концепція: Вирішує класичну проблему автоматизованого брутфорсу — крихкість селекторів форм при зміні HTML/CSS сайту. LLM аналізує сторінку логіну "по змісту", а не по жорстко заданих CSS-селекторах, що робить інструмент стійкішим до змін фронтенду.

- **User Scanner** — Email & Username OSINT
  - Опис: OSINT-набір для розслідування email та юзернеймів на сотнях платформ. Знаходить пов'язані акаунти й публічні сліди, витягує корисні метадані, підтримує масові перевірки й проксі, експортує результати в JSON або CSV.
  - Версія: 1.4.2.1 | Тестовано на: Termux, Windows, Linux | 127K завантажень

- **GhidraGPT** — AI-Powered Reverse Engineering
  - Репозиторій: `weirdmachine64/GhidraGPT`
  - Опис: Інтегрує LLM безпосередньо в Ghidra (дизасемблер/декомпілятор NSA) для асистованого reverse engineering. Аналізує одну декомпільовану функцію за раз, пояснює її поведінку, пропонує зрозуміліші назви й типи даних, позначає потенційні проблеми безпеки в коді.
  - Концепція: Класична проблема реверс-інжинірингу — декомпільований код з generic-назвами змінних (`uVar1`, `local_38`) важко читати. LLM-асистент відновлює семантичний сенс, ефективно виконуючи роботу, яку раніше робив досвідчений реверсер вручну.

- **PentesterFlow** — Agentic AI for Pentesting
  - Репозиторій: `PentesterFlow/agent`
  - Зірки: 1.2k ★ | Build: passing | Ліцензія: Apache-2.0 | Node: 20+
  - Опис: Agentic AI термінальний асистент для авторизованого пентестингу. Допомагає організувати recon, enumeration і валідацію, запускає реальні security-інструменти, запитує підтвердження для чутливих дій, зберігає задокументовані знахідки.
  - Приклад з демо: агент отримує завдання "протестувати orders API на broken access control", сам завантажує потрібний skill (webvuln), робить HTTP-запити, підтверджує cross-account response через curl з іншим Bearer-токеном, і фіксує підтверджену знахідку (IDOR) у файл findings.
  - Концепція: Прямий приклад "Tool Use" + "Agent Workflows" з Mastery Tree (Частина II.1), застосований до пентестингу — з важливою відмінністю "requests approval for sensitive actions", тобто human-in-the-loop для потенційно шкідливих дій, аналогічно до крокам MCP tune behavior/protocol (Частина II.2, крок 3).

- **Mr.Holmes** — Complete OSINT Investigation Tool
  - Репозиторій: `Lucksi/Mr.Holmes`
  - Опис: OSINT-інструмент для збору публічної інформації про домени, юзернейми й телефонні номери. Поєднує Google dorks, проксі та WHOIS-дані, пропонуючи багато можливостей розслідування через єдиний CLI-інтерфейс.
  - Меню CLI: Social-Account-OSINT, Phone-Number-OSINT, Domain/IP-OSINT, Database (GUI), Port-Scanner, E-Mail, Dorks-Generator, People-OSINT, Encoding/Decoding, PDF-Graph Converter, File-Transfer

- **Android PIN Bruteforce** — USB HID PIN Testing
  - Репозиторій: `TR-TECH-GUIDE/Android-PIN-Bruteforce`
  - Опис: Перетворює сумісний рутований Android- або Kali NetHunter-пристрій на USB HID-клавіатуру для автоматизованого тестування PIN-кодів на іншому Android-пристрої. Підтримує 1–10-значні PIN, кастомні профілі, списки PIN-кодів, безпечні затримки й відстеження прогресу.
  - Апаратна схема: locked phone ← charging cable (Male Micro-B to Male USB-A) ← NetHunter phone → OTG cable (Male Micro-B to Female USB-A)
  - Концепція: Апаратна атака brute-force на локальний PIN-екран — обходить будь-які програмні rate-limit захисти застосунку, оскільки емулює фізичну USB-клавіатуру, вводячи PIN-коди на швидкості, обмеженій лише налаштованою затримкою.

- **Shadowbroker** — Real-Time Geospatial OSINT
  - Репозиторій: `BigBodyCobain/Shadowbroker`
  - Опис: Self-hosted геопросторова OSINT-платформа, що поєднує десятки живих джерел даних в інтерактивній карті. Відстежує літаки, кораблі, супутники, конфлікти й кіберзагрози, надаючи інструменти recon та AI-асистовані кореляції.
  - Демонстровані шари даних: Commercial/Private/Military Flights, Tracked Aircraft, Earthquakes (24h), Satellites, Carriers/MQ/Cargo, Civilian Vessels, Cruise/Passenger суда — з паралельною панеллю "Global Threat Intercept" з AI-згенерованими аналітичними висновками
  - Концепція: Агрегатор OSINT-джерел у реальному часі рівня "ситуаційна обізнаність" (situational awareness), що поєднує традиційний геопросторовий трекінг (ADS-B для літаків, AIS для кораблів) з LLM-шаром для автоматичного зведення розрізнених сигналів у наративні висновки — практичний приклад того, як AI-шар накладається поверх класичних (не-AI) OSINT-джерел даних.

---

# ЧАСТИНА VI. Репозиторії для запозичення та побудови продуктів

### VI.1 Серія "Five Repos To Steal And Build Upon" (buildercult)

**Загальна концепція серії:** Автор buildercult підбирає open-source репозиторії не як готові продукти для використання "як є", а як **фундамент для побудови власного нішевого SaaS** — з конкретними ідеями, на кого націлити продукт.

- **#1 Penpot** (`penpot/penpot`) — Design workflow SaaS
  - Зірки: 58.5k ★ | Мова: Clojure | Ліцензія: — | Теги: design, clojure, ui, clojurescript, prototyping
  - Опис: Open-source дизайн-платформа для продуктових команд, яким потрібна масштабована колаборація.
  - Ідея від автора: "Не конкуруй з Figma. Будуй нішеві портали для дизайн-рев'ю для агенцій, стартап-акселераторів чи команд, яким потрібні аппрувли, handoff і брендована клієнтська співпраця." Penpot підтримує real-time колаборацію, self-hosting, дизайн-системи, API та плагіни.

- **#2 OpenSEO** (`every-app/open-seo`) — Niche SEO intelligence SaaS
  - Зірки: 11.8k ★ | Мова: TypeScript | Теги: mcp, seo, site-audit, seo-tools, keyword-research
  - Опис: Open-source альтернатива Semrush та Ahrefs.
  - Ідея від автора: Зробити SEO простим для Shopify-магазинів, локального бізнесу, афілейт-сайтів, SaaS-засновників чи агенцій. Вже покриває keyword research, rank tracking, competitor insights, backlinks, audits та AI visibility. Пропозиція побудови: використати Blink.new для створення простого й таргетованого SEO/AI-visibility движка для малого бізнесу й Shopify-магазинів.

- **#4 Buzz** (`block/buzz`) — AI-powered team workspace
  - Зірки: 27.2k ★ | Мова: Rust
  - Опис: "Hive mind communication platform" — open-source, self-hostable workspace, де люди й AI-агенти діляться одними й тими самими "кімнатами".
  - Ідея від автора: Перетворити на приватний командний центр для маркетингових агенцій, software-команд, creator-бізнесів, дослідницьких команд чи спільнот, яким потрібні чат, workflows, файли та AI-агенти в одному місці.

*(Позиції #3 та #5 серії не потрапили в наданий батч скріншотів)*

### VI.2 Серія "10 GitHub repositories that went viral last month" (githubprojects)

- **#01 ESP32-BlueJammer** — див. Частину V.6 (кібербезпека/hardware)

- **#02 DSPy** (`stanfordnlp/dspy`)
  - Категорії: AI · Framework
  - Опис: Фреймворк від Stanford NLP для **програмування й оптимізації** AI-систем замість ручного дороблення промптів. Чудово підходить для RAG-пайплайнів, агентів і класифікаторів.
  - Концепція: DSPy зсуває парадигму prompt engineering у бік "declarative programming" — розробник описує *що* система має робити (сигнатури модулів, метрики якості), а фреймворк автоматично оптимізує *як* саме сформульовані промпти для досягнення мети. Прямий концептуальний зв'язок із розділом II.1 (Prompt Engineering) — DSPy автоматизує саме той шар, який там описаний вручну (Zero-Shot/Few-Shot/CoT).

- **#03 auto-editor** (`WyattBlue/auto-editor`)
  - Категорії: Video · Productivity
  - Опис: Командно-рядковий відео- та аудіоредактор, що автоматично виявляє й видаляє тихі або нецікаві секції із записів.
  - Концепція: Типовий інструмент для подкастерів/YouTube-творців — автоматизує рутинну частину монтажу (вирізання пауз), залишаючи творчі рішення людині.

- **#05 Markdoc** (`markdoc/markdoc`)
  - Категорії: Documentation · Markdown
  - Опис: Гнучкий Markdown-based фреймворк для написання багатого документаційного контенту, використовується компанією **Stripe**.
  - Концепція: На відміну від "чистого" Markdown, Markdoc додає можливість вбудовувати інтерактивні/кастомні компоненти прямо в текст (аналогічно MDX), зберігаючи при цьому простоту редагування для нетехнічних авторів документації.

- **#07 Apache Cloudberry** (`apache/cloudberry`)
  - Категорії: Database · Analytics
  - Опис: Open-source **массивно-паралельна процесингова (MPP)** база даних, побудована для аналітики й data warehousing.
  - Концепція: MPP-архітектура розподіляє запит між багатьма вузлами, що обробляють дані паралельно — стандартний підхід для OLAP-навантажень (на відміну від OLTP-баз типу PostgreSQL з Частини II.1). Проєкт входить у парасольку Apache Software Foundation, що додає йому інституційної довіри порівняно з незалежними стартап-форками.

- **#08 code-server** (`coder/code-server`)
  - Категорії: Developer Tools
  - Опис: Запускає VS Code на віддаленій машині й дає доступ до середовища розробки через браузер.
  - Концепція: Класичний патерн "cloud IDE" — корисно для розробки з малопотужних пристроїв (Chromebook, планшет), консистентного середовища в команді, або роботи з важкими обчислювальними ресурсами (GPU-сервер), не покидаючи браузер.

- **#10 pic-smaller** (`joye61/pic-smaller`)
  - Категорії: Productivity
  - Опис: Браузерний пакетний компресор зображень з підтримкою JPEG, PNG, WebP, GIF, SVG, AVIF та HEIC. Зображення ніколи не покидають пристрій користувача.
  - Концепція: Ще один приклад "privacy-by-architecture" — обробка повністю на клієнті (в браузері, ймовірно через WASM-кодеки) замість завантаження на сервер, що усуває питання конфіденційності зображень і витрат на серверну інфраструктуру для розробника.

### VI.3 OpenSandbox
- **Репозиторій:** `opensandbox-group/OpenSandbox`
- **Автор поста:** marc.kaz
- **Зірки:** 13k ★ (позначено як #1 GitHub Trending Repository Of The Day на момент посту)
- **Значки:** OpenSSF, Discord (Join), DingTalk (Join), K8S (passing)
- **Слоган:** "So Alibaba just open-sourced the sandbox every AI agent needed"
- **Опис:** Пісочниця (sandbox) для безпечного виконання коду/дій AI-агентів, відкрита компанією Alibaba.
### VI.4 Інші помітні репозиторії

- **Flash-KMeans**
  - Автор поста: dailydoseofds_
  - Ліцензія: Apache-2.0
  - Опис: IO-aware пакетна (batched) кластеризація K-Means, реалізована з Triton GPU-ядрами. Репозиторій надає офіційну реалізацію K-Means для проєкту Sparse VideoGen2.
  - Заявлена перевага: "найшвидший алгоритм K-Means, спроєктований для GPU" — демо порівнює збіжність Flash-KMeans проти "Fast PyTorch Kmeans" за кількість ітерацій і час.
  - Концепція: "IO-aware" означає, що алгоритм явно оптимізований під патерни доступу до пам'яті GPU (мінімізація трафіку між HBM і SRAM) — той самий принцип, що лежить в основі FlashAttention. Це показує, як GPU-орієнтована переоптимізація класичних ML-алгоритмів (тут — K-Means) дає прискорення на порядки без зміни математики методу.

- **destructive_command_guard (dcg)**
  - Репозиторій: `Dicklesworthstone/destructive_command_guard`
  - Автор поста: fork.cast_
  - Зірки: 3.2k ★ | Форки: 120 | Watch: 10 | Releases: 29 (останній v0.6.5) | Мова: Rust (88.2%), Shell (8.6%), PowerShell (3.0%)
  - Опис: Блокує небезпечні git- і shell-команди від виконання AI-агентами.
  - Теги: git, rust, cli, developer-tools, safety, ai-agents
  - Контрибʼютори: включно з "claude" і "codex" як окремими контрибʼюторами в списку — тобто AI-агенти самі допомагають підтримувати інструмент, що їх же й обмежує
  - Концепція: Прямий практичний приклад "Guardrails" з Mastery Tree (Частина II.1) — на відміну від OpenSandbox (VI.3), що ізолює *все* виконання агента, dcg — вужчий, специфічний фільтр саме для git/shell команд (напр. запобігання `rm -rf`, force-push, видаленню гілок). Показує паралельну еволюцію двох рівнів захисту: повна ізоляція середовища (sandbox) vs. точковий allowlist/denylist конкретних команд (guard).

- **llmfit**
  - Репозиторій: `AlexsJones/llmfit`
  - Автор поста: gittrend.io
  - Версія: crates.io v1.1.6 | Ліцензія: MIT | CI: passing | SignPath: signed
  - Опис: Термінальний інструмент, що "підганяє" LLM-моделі під розміри системи користувача (RAM, CPU, GPU). Виявляє апаратне забезпечення, оцінює кожну модель за критеріями якості, швидкості, відповідності (fit) і контекстного вікна, і каже, які саме моделі реально добре запрацюють на конкретній машині.
  - Режими: інтерактивний TUI (за замовчуванням) і класичний CLI-режим. Підтримує multi-GPU конфігурації, MoE-архітектури, динамічний вибір квантизації, оцінку швидкості й локальних runtime-провайдерів (Ollama, Docker Model Runner, LM Studio).
  - Нова фіча: benchmark & share — завантажити модель, обслужити її, виміряти реальні токени/секунду на своєму залізі, і надіслати результат назад у проєкт як PR прямо з TUI (без `gh` CLI чи стороннього акаунту). Кожен прогін зберігається локально першим; власні виміри замінюють оцінки в таблиці підбору.
  - Споріднені проєкти (sister projects): sympozium (керування агентами в Kubernetes), llmserve (простий TUI для обслуговування локальних LLM), llama-panel (нативний macOS-застосунок для керування llama-server інстансами)
  - Концепція: Вирішує реальну "холодну проблему" локального LLM-хостингу — користувач часто не знає заздалегідь, чи потягне його залізо конкретну модель у конкретному квантуванні. Механізм краудсорсингового бенчмаркінгу (реальні виміри з реального заліза замість теоретичних оцінок) — цікавий патерн community-driven валідації даних, аналогічний до того, як формуються бенчмарки в HuggingFace Open LLM Leaderboard, але на рівні "модель × конкретна конфігурація заліза" замість "модель × стандартний бенчмарк-датасет".

---

# ЧАСТИНА VII. Серія "10 GitHub repos competing with products" (artificialzone)

**Загальна концепція серії:** На відміну від Частини I ("X-killer" одиночні пости), ця серія систематично документує соло-розробників або малі команди, чиї open-source проєкти прямо конкурують із мільярдними компаніями — з акцентом на людську історію (хто, чому побудував) і конкретне зіставлення вартості.

### VII.1 Immich (#1)
- **Розробник:** Alex Tran (соло)
- **Позиціонування:** Self-hosted photo and video management solution, продукт компанії FUTO
- **Наратив:** Alex Tran побудував Immich самотужки, бо Google Photos "тримав у заручниках" його спогади. Замінює підписну імперію Alphabet вартістю $2 трильйони.
- **Концепція:** Класичний приклад "data ownership" мотивації — фото та відео є одними з найособистіших даних користувача, і залежність від пропрієтарного хмарного сервісу (з ризиком підвищення цін, зміни умов чи навіть втрати доступу) стала для розробника достатнім стимулом побудувати повноцінну альтернативу самостійно.

### VII.2 Papermark (#4)
- **Репозиторій:** `mfts/papermark`
- **Розробник:** Marc Seitz (соло)
- **Зірки:** 8k ★ | Форки: 1k | Contributors: 62 | Issues: 93 | Discussions: 13
- **Позиціонування:** Open-source альтернатива DocSend з вбудованою аналітикою й кастомними доменами
- **Наратив:** Marc Seitz побудував як open-source альтернативу DocSend — трекінг документообігу за $0.

### VII.3 tldraw (#6)
- **Розробник:** Steve Ruiz (соло)
- **Позиціонування:** Інфінітний canvas / whiteboard-бібліотека з інструментами малювання, фігур, вбудовуваних елементів (карти, код, зображення)
- **Наратив:** Steve Ruiz побудував самотужки; тепер використовується всередині Vercel, Linear та Microsoft. Прямий конкурент Miro, оціненого в $17 мільярдів.
- **Концепція:** Показовий приклад "інфраструктурного" open-source — tldraw не продається як кінцевий продукт, а вбудовується як компонент у продукти інших компаній (embeddable whiteboard SDK), що пояснює його прийняття великими гравцями (Vercel, Microsoft) без прямої конкуренції з ними.

### VII.4 Documenso (#7)
- **Розробник:** Timur Ercan та невелика команда
- **Позиціонування:** SOC 2 Compliant, Open Source, з підтримкою Organizations, Envelopes, New Editor (типи полів: Signature, Email, Name, Initials, Date, Text, Number, Radio, Checkbox, Dropdown), API V2
- **Версія:** Documenso 2.0.0
- **Наратив:** Timur Ercan і крихітна команда демонтують імперію DocuSign вартістю $14 мільярдів — open-source підписання документів.

### VII.5 Postiz (#8)
- **Розробник:** Nevo David (соло)
- **Позиціонування:** "Everything you need to grow on social" — планування контенту (Calendar, Analytics, Marketplace, Messages) для кількох соцмереж/каналів одночасно
- **Наратив:** Nevo David побудував самотужки як open-source "повстання" проти Buffer та Hootsuite.

### VII.6 CrowdSec (#9)
- **Розробник:** Philippe Humeau
- **Позиціонування:** "Crowdsourced threat intel" — community-driven альтернатива bot-менеджменту Cloudflare
- **Архітектура (за діаграмою):** Log Sources (Syslog Server, Log File, Docker, HTTP Requests з веб-серверів) → CrowdSec Security Engine (детектує небажану поведінку, блокує погані IP) → двонаправлений зв'язок з CrowdSec Central API (надсилає IP порушників, отримує community blocklist) → Online Console (перегляд алертів) → Remediation (WordPress, Cloudflare, Nginx, Firewall)
- **Наратив:** Philippe Humeau побудував community-driven альтернативу bot-менеджменту Cloudflare — crowdsourced threat intelligence.
- **Концепція:** Мережевий ефект як конкурентна перевага open-source security — кожен інстанс CrowdSec, що детектує атаку, ділиться сигнатурою офендера з центральним API, і всі інші інстанси отримують оновлений community blocklist. Це відтворює модель Cloudflare (масштаб трафіку → кращі дані про загрози → кращий захист), але розподілено між незалежними самостійно хостованими інстансами, а не централізовано в одній компанії.

### VII.7 Inbox Zero (#10)
- **Розробник:** Elie Steinbock
- **Позиціонування:** AI email-асистент, що не продає дані користувача
- **Ліцензія:** MIT
- **Наратив:** Elie Steinbock побудував AI email-асистента, що не продає ваші дані. Sanebox бере $25/місяць; Inbox Zero — MIT-ліцензований (безкоштовний і відкритий).

---

## Інші помітні інструменти з цього батчу

### jcode
- **Автор поста:** simplifyinai
- **Версія:** v0.65.0 | Ліцензія: MIT | Платформи: Linux, macOS, Windows | Зірки: 15.3K
- **Слоган:** "The most RAM efficient harness. The most intelligent harness."
- **Заявка:** Цей coding-агент у 245 разів швидший за Claude Code, 100% безкоштовний і відкритий.
- **Демонстровано:** термінальна сесія, де агент автоматично рекол пам'яті, шукає по кодовій базі (agentgrep), читає документацію і код паралельними батчами для розрізнення реалізованого від запланованого функціоналу.
- **Концепція:** "Harness" у контексті coding-агентів — це оркеструючий шар навколо LLM (подібно до Claude Code, Cursor, Aider), що керує контекстом, інструментами й пам'яттю. Заявка про 245x швидкість і водночас найкращу RAM-ефективність передбачає радикально іншу архітектуру керування контекстом порівняно з конкурентами — хоча подібні маркетингові заяви варто сприймати з обережністю без незалежного бенчмарку.

### Cactus Needle (Needle 2)
- **Репозиторій:** `cactus-compute/needle`
- **Автор поста:** gitscout.io
- **Опис:** 14MB foundation-модель для крихітних пристроїв — телефонів, wearables, smart home, роботів.
- **Технічні деталі:** Needle 2 — відкрита 45M-параметрична модель для tool calling, device use і структурованого витягу даних. Весь модель — єдиний 14MB бінарник, що виконує повну сесію приблизно в 28MB RAM. Побудована на власних дослідженнях Simple Attention Network, стиснута до CQ2-біт через Cactus Quants, запечена у власний рушій.
- **Бенчмарки:** обмінюється перемогами з іншими малими моделями (FunctionGemma 270M, LFM2.5 230M, Apple FM), будучи при цьому в 5–70 разів меншою і на 2 біти проти їхнього f16.
- **Ключові властивості:** Self-contained (ваги запечені в єдиний 14MB рушій, інференс без мережі); Simple contract (tool calls повертаються як структуровані дані, JSON-вихід, побайтова граматика обмежує кожен токен); Confidence-gated (кожна відповідь несе калібрований confidence score); Tool retrieval (retrieval-голова рендерить лише топ-5 інструментів за хід); Bounded memory (пам'ять залишається біля 28MB незалежно від довжини розмови)
- **Встановлення:** `pip install cactus-needle`
- **Концепція:** Прямий контрприклад до тренду "більша модель = краще" — Needle 2 оптимізована саме під обмеження едж-пристроїв (телефони, wearables), де 45M параметрів і 14MB — не компроміс, а свідомий вибір архітектури заради локальної, офлайн, приватної роботи на пристроях, які фізично не можуть запустити навіть найменші "звичайні" LLM (зазвичай мільярди параметрів).

### LobeHub
- **Автор поста:** gittrend.io
- **Слоган:** "Your Chief Agent Operator"
- **Опис:** Організовує AI-агентів у режим роботи 7×24. Наймає, планує розклад, звітує про всю вашу AI-команду. "Ви лишаєтесь головним — не залишаючись онлайн."
- **Метрики:** release v2.2.10 | 557 online в Discord | contributors 336 | forks 16k | stars 80k | issues 224 open | ліцензія Apache 2.0 | test coverage 70%
- **Визнання:** #1 Product of the Day (Product Hunt), #1 Repository of the Day (GitHub Trending)
- **Демонстровані ролі агентів:** Data Visualization Expert, Sales Agent, Issue Agent, Frontend Bug Fix Developer, MCP & Marketplace Curator, Feature Analyst, Bug Triage Specialist, Database Expert, Cybersecurity Assistant — з підтримкою каналів Discord, Telegram, Slack, WhatsApp, Lark, WeChat
- **Концепція:** LobeHub позиціонує себе не як окремий агент, а як **мета-рівень оркестрації** над іншими агентними системами (видно інтеграції з Claude Code, Codex, OpenClaw) — по суті, "менеджер команди AI-агентів", що розподіляє завдання, стежить за прогресом і звітує людині. Це на порядок вищий рівень абстракції, ніж окремі coding-агенти чи pentesting-агенти з попередніх частин документа.

### GeoLibre v2.3.0
- **Автор поста:** aiproductlabs
- **Опис:** Open-source геопросторовий інструмент для розробників, що працюють з картами.
- **Демонстровано:** 3D-візуалізація Нью-Йорка з шарами NYC Subway Stations/Lines (MTA), Manhattan Building Heights (з кольоровим кодуванням за десятиліттям будівництва: Pre-1900 до 2015+), World Imagery, підтримка стилів карт, легенди, панель шарів у стилі QGIS/ArcGIS
- **Концепція:** Позиціонується як відкрита альтернатива пропрієтарним ГІС-платформам (ArcGIS, Mapbox Studio) — комбінує функціонал десктопного ГІС-редактора (шари, стилізація, обробка даних) з веб-орієнтованим 3D-рендерингом, судячи з інтерфейсу.

### Sync-in
- **Автор поста:** safesynt (серія "GitHub Find")
- **Зірки:** 1.2k ★ | Ліцензія: AGPL v3.0 | Release: v1.10.0 | Docker Hub Pulls: 28k | NPM Downloads: 2k | Discord: 20 online
- **Слоган:** "Own Your Stack"
- **Опис:** Офіційний репозиторій сервера Sync-in — спроєктований для роботи на власній інфраструктурі, дає повний контроль над даними, пропонуючи сучасний, інтуїтивний інтерфейс для внутрішніх і зовнішніх користувачів.
- **Функції:** collaborative spaces, secure file sharing, granular permission management — підходить від малих команд до великих підприємств, публічних інституцій чи приватних осіб, які дбають про приватність.
- **Концепція:** Self-hosted альтернатива Dropbox/Google Drive/Nextcloud-класу з фокусом на "власність стеку" (own your stack) — той самий мотив приватності й незалежності від хмарного постачальника, що проходить через увесь документ (Duix-Avatar, meetily, harper, тепер Sync-in).

### LandingAI ade-cli
- **Автор поста:** safesynt (серія "GitHub Find")
- **Зірки:** 2.4k ★ | PyPI: v1.2.0 | Python: 3.9+ | Ліцензія: Apache-2.0
- **Слоган:** "CLI First"
- **Опис:** Agentic Document Extraction Python Library — Python-бібліотека для взаємодії з LandingAI Agentic Document Extraction REST API, спроєктована для гнучкості, надійності, ясності й продуктивності. Побудована для Python 3.9+ і згенерована через Stainless (інструмент генерації SDK з OpenAPI-специфікації).
- **Функції:** повністю типізований SDK з Pydantic response-моделями; синхронні й асинхронні клієнти; обробка великих документів через async jobs; вбудовані ретраї з експоненційним backoff; безпечна обробка API-ключів; безшовне завантаження файлів
- **Концепція:** LandingAI (заснована Andrew Ng) — комерційний постачальник computer vision/document AI; ade-cli — офіційний, але open-source клієнтський SDK до їхнього платного API. Це відрізняється від переважної більшості інструментів документа (самостійно хостовані альтернативи): тут відкритий лише клієнт, а сам сервіс екстракції залишається пропрієтарним хмарним API.

### Graph Engineering — "The Karpathy Loop, Improved 1000x by Itself" (документ)
- **Автор поста:** quantscience_ (репост твіту Matt Dancho / Business Science, оригінальний твіт від 6 серпня 2026)
- **Заявка твіту:** Двоє старших AI-інженерів Anthropic зробили "Karpathy Loop" у 1000 разів кращим через "Graph Engineering", потім опублікували безкоштовний 11-сторінковий PDF.
- **Підзаголовок документа:** "Agentic Software Engineering Practice 2026" — незалежно складено в липні 2026, не афілійовано з Andrej Karpathy чи Anthropic, і не схвалено ними
- **Схема з документа (Fig. 1):** Knowledge graph схема для мультиагентних систем — центральний вузол AGENT з'єднується з типізованими сутностями через спрямовані ребра: Title/Description, First Person (name, role, category), Object Above, Tools, Agency/Group, Next Below (contract details), Extra Person Entity, Extra Property/Attribute, Object Below
- **Тези з абстракту:** Карпаті побудував nanoresearch — одноагентний цикл, що провів 700 ML-експериментів за 2 дні й самостійно відкрив 20 оптимізацій тренування. Anthropic незалежно доставили продакшн-версію: Dynamic Workflows, що породжує до 1000 паралельних субагентів з одного промпту, тоді як Knowledge Graph Cookbook замінює тренованих NLP-пайплайнів графом, побудованим одним промптом.
- **Концепція:** Цей документ — прямий концептуальний "первісний матеріал" для Graph Engineering for Agentic AI (Частина II.8) з попереднього доповнення — обидва джерела описують однаковий зсув парадигми (від sequential loop/nanoresearch-стилю Карпаті до графо-orchestrated мультиагентних систем), лише з різних кутів: codewithbrij дає практичний фреймворк (5-стадійна методика, типізовані ребра), а цей документ — теоретичне обґрунтування через призму робіт Карпаті й "Anthropic Playbook". Важливо відзначити явне застереження документа: "не афілійовано з Karpathy чи Anthropic, і не схвалено ними" — тобто це незалежна синтетична робота спільноти, а не офіційний матеріал Anthropic, попри заголовок твіту, що це подає.

---

## Наскрізні концепції та тематичні зв'язки

1. **"X-killer" наратив** — більшість цих проєктів позиціонуються як безкоштовні/self-hosted альтернативи дорогим SaaS: HeyGen ($29/міс) → Duix-Avatar; Perplexity ($20/міс) → Vane; Manus ($20/міс) → agenticSeek; Otter.ai ($17/міс) → meetily; Grammarly ($12/міс) → harper. Спільний патерн: **приватність + локальна обробка + open-source ліцензія** як конкурентна перевага над хмарними підписками.

2. **Локальність (local-first) як наскрізна вимога** — Duix-Avatar, agenticSeek, meetily, harper, Lightpanda (частково) — усі підкреслюють, що дані/аудіо/текст не покидають пристрій користувача. Це відповідає ширшому тренду 2025–2026: витіснення "LLM-as-a-service" моделями, що запускаються локально (Ollama, llama.cpp).

3. **MCP як інфраструктурний шар оркестрації** — туторіал MCP Host (розділ 3) демонструє патерн одного постійного event loop, що керує кількома незалежними MCP-серверами через allowlist. Це напряму застосовне до архітектури "Гідравлічний Контур Сенсу" (Gortai) — де LangGraph/Redis-подібна оркестрація потребує подібного контролю поверхні tool-calls.

4. **Grounded/traceable output як вимога до enterprise AI** — LangExtract (grounded extraction) та Semantica (graph-native provenance) вирішують одну проблему різними шляхами: довіра до LLM-виводу вимагає можливості простежити факт до джерела. Це паралель до епістемічних принципів Свята (незалежна верифікація, чесне маркування помилок).

5. **RAG-конектор-хаби** — SurfSense об'єднує 20+ джерел даних в один чат-інтерфейс; це логічне продовження ідеї "єдиної бази знань", подібно до батчевого проєкту Свята, де Instagram-скріншоти агрегуються в структуровані MD-документи.

6. **Domain-specific foundation models** — Kronos показує, що патерн "pretrain на великому корпусі → fine-tune/zero-shot на задачі" переноситься за межі тексту/зображень у фінансові часові ряди (K-lines). Аналогічно OmniParse/LangExtract переносять LLM-парсинг на неструктуровані документи будь-якого типу.

7. **Персистентна пам'ять для кодинг-агентів — конкуруючі підходи** — Sentrux (Частина I) і brain.md (Частина III) вирішують ідентичну проблему (агент "забуває" контекст кодової бази між сесіями) двома протилежними шляхами: Sentrux — через "сенсорну" рекурсивну систему фідбек-петлі; brain.md — через мінімалістичний, git-friendly Markdown-стандарт. Це ілюструє відсутність консенсусу щодо "правильної" архітектури пам'яті агента в 2026 — від складних векторних/графових рішень до навмисно простих текстових файлів.

8. **Дистиляція поведінки потужної моделі в явний протокол** — The Fable Method (Частина III) формалізує емерджентний стиль роботи Claude Fable 5 у чотири тестовані навички (think/act/prove/grow), щоб слабші моделі могли відтворити якість через явні інструкції, а не через "розмір моделі". Це методологічно перегукується з "How to Build AI Agents from Scratch" (Частина II) — крок 3 (Tune behavior & add protocol) саме про це: явний протокол система-промптів замість сподівання на емерджентну поведінку.

9. **RAG еволюціонує в напрямку структурованих знань** — лінія розвитку простежується через весь документ: Vector-only search (Частина II.4) → Hybrid search з re-ranker (Частина II.4) → RAG з зовнішніми документами (Частина II.3) → KAG з графом знань (Частина II.3) → Semantica як graph-native інфраструктура (Частина I.14.2) → turbovec як практичне рішення для масштабування векторного шару (Частина III.2). Кожен крок додає структуру або ефективність там, де попередній підхід деградує на масштабі чи складних логічних запитах.

10. **Компактне представлення "простору" як наскрізна математична тема** — Calabi-Yau slice (Частина IV.1) продовжує тему "простір як обраний зріз/координатна система" з попередніх батчів документації Свята: те, що бачимо, — це проєкція R⁴-зрізу вищовимірного комплексного многовиду, а не сам об'єкт. Концептуально це паралель до компресії векторного простору в turbovec (31 ГБ → 4 ГБ через "оптимальний" квантований базис) — в обох випадках йдеться про вибір ефективного представлення багатовимірного об'єкта, що втрачає частину інформації, але зберігає потрібну структуру.

11. **"Awesome list" vs структурована вікі vs готовий payload-набір — три рівні кураторства знань з кібербезпеки** — Частина V демонструє градацію форматів: Awesome-Hacking (V.1) — просто список посилань; SecLists (V.2) — сирі дані для інструментів; PayloadsAllTheThings (V.4) — готові до копіювання експлойти з поясненням; HackTricks (V.3) — повноцінна навчальна вікі з методологією. Це той самий спектр "від сирих даних до знань", що і в Частині I–III (LangExtract → OmniParse → SurfSense → Semantica), лише застосований до домену безпеки.

12. **"Repos to steal" — інженерна культура ремікса, а не винаходу з нуля** — Частина VI.1 явно артикулює патерн, неявно присутній у всьому документі: більшість "X-killer" репозиторіїв (Частина I) і продуктових ідей (Частина VI) — це не оригінальні дослідницькі прориви, а **перекомпонування** зрілих open-source будівельних блоків (Penpot, Buzz, OpenSEO) під вузьку нішу. Це нормальна й заохочувана практика в open-source-екосистемі 2026 року, оскільки ліцензії (MIT, Apache-2.0) прямо дозволяють похідні роботи — контрастує з "вбивця X" маркетинговим фреймінгом Частини I, який має на меті продати оригінальність там, де насправді йдеться про репозиціонування.

13. **Sandbox/ізоляція як необхідна інфраструктура для агентної автономії** — OpenSandbox (VI.3) явно називає проблему, яка неявно висіла над усіма агентними інструментами документа (agenticSeek, OpenEvolve, WebToApp з fork+exec серверних рантаймів): дозволити LLM-агенту виконувати довільний код чи команди — це серйозний вектор атаки, якщо не ізольовано. У міру того як агенти отримують дедалі більше автономії (Частина II.2, крок 4 — "Tool Use"), пісочниці стають не опційним, а обов'язковим компонентом продакшн-архітектури — паралель до "Guardrails" і "AI Safety" з Частини II.5.

14. **Graph engineering як "розкриття" agentic-циклу в явний, типізований, паралельний workflow** — Частина II.8 (Graph Engineering for Agentic AI) формалізує перехід від sequential loop до графа з семантично типізованими ребрами (SUPERSEDES, DEPENDS_ON, CAUSED тощо). Це напряму синтезує кілька ліній документа: воно є прикладною реалізацією KAG (Частина II.3) для агентних систем, використовує принципи "Structure multi-agent logic" з 10-кроквого гайду (Частина II.2, крок 5), а Hybrid Retrieval Router у ньому — практичний приклад патерну Vector vs Graph traversal з Частини II.4. Разом ці чотири джерела (II.2, II.3, II.4, II.8) утворюють цілісну картину того, як індустрія в 2026 році переходить від "промпт → один LLM-виклик" до графо-orchestrated мультиагентних систем з явним керуванням часовою валідністю знань (temporal knowledge) — концептуально суголосно з роботою Свята над Гідравлічним Контуром Сенсу.

15. **AI як асистент у вузькоспеціалізованих технічних дисциплінах — реверс-інжиніринг, пентест, OSINT** — Частина V.7 (серія HexSecTeam) демонструє паттерн "LLM за плечима експерта", відмінний від агентної автономії (agenticSeek, OpenEvolve): GhidraGPT пояснює одну декомпільовану функцію за раз, PentesterFlow запитує підтвердження перед чутливими діями, BruteForceAI лише аналізує форму логіну, а не приймає рішення про ціль. Це проміжна точка на спектрі між "LLM як інструмент з людиною за кермом" (ці приклади) і "LLM як автономний агент" (agenticSeek, PentesterFlow у своїй агентній частині) — вибір рівня автономії явно залежить від ризику хибних дій у домені (юридичні наслідки некоректного пентесту значно вищі, ніж некоректного summarize-запиту).

16. **"Соло-розробник проти мільярдної компанії" як повторюваний наратив документа** — Частина VII систематично документує патерн, що вже з'являвся розрізнено раніше (WebToApp, harper, meetily): один розробник або крихітна команда (Alex Tran/Immich, Marc Seitz/Papermark, Steve Ruiz/tldraw, Nevo David/Postiz, Elie Steinbock/Inbox Zero) будує функціональну альтернативу продукту компанії з мільярдною оцінкою (Google Photos, DocSend, Miro $17B, Buffer/Hootsuite, Sanebox). Це не випадковість, а структурна особливість сучасної розробки: доступність потужних фреймворків, хмарної інфраструктури за низьку ціну та LLM-асистованого кодування (сама тема цього документа — jcode, GhidraGPT тощо) різко знижує бар'єр для соло-розробника повторити 80% функціоналу SaaS-продукту, залишаючи компаніям цінність у підтримці, комплаєнсі (SOC 2 у Documenso) та мережевих ефектах.

17. **Спектр "локальності" від едж-пристроїв до self-hosted серверів до хмарних SDK** — новододані інструменти утворюють континуум: Cactus Needle (14MB модель на смартфоні/wearable) → jcode/harper/meetily (локальний застосунок на ноутбуці) → Sync-in/Immich/SurfSense (self-hosted на власному сервері) → LandingAI ade-cli (клієнт до пропрієтарного хмарного API). Кожна точка цього спектра — свідомий компроміс між обчислювальною потужністю (менше локально) і контролем/приватністю (більше локально), і жодна точка не є універсально "правильною" — вибір залежить від задачі (Cactus Needle не замінить великий LLM для складного reasoning, а хмарний ade-cli не підходить для air-gapped середовища).

18. **Метанаратив документа vs власна тема документа** — Graph Engineering PDF (розділ "Інші помітні інструменти") ілюструє важливий епістемічний момент: твіт заявляє, що "два старші AI-інженери Anthropic" створили методику, тоді як сам документ прямо зазначає незалежну, неафілійовану природу роботи. Це паралель до "X-killer" маркетингового фреймінгу з наскрізної концепції №1 — соцмережевий контент систематично applies перебільшену або неточну атрибуцію для підвищення залучення (engagement), і читачеві варто перевіряти першоджерело (тут — сам PDF-документ), а не покладатися на заголовок посту.

---

## Зведена таблиця

| # | Репозиторій | Категорія | Зірки | Ліцензія | "Вбиває" |
|---|---|---|---|---|---|
| 1 | duixcom/Duix-Avatar | AI-аватари | 14.4k | —