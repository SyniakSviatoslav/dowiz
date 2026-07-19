# RAW PROMPT #7 — Open-Source Hardening: PBT for Kleene Logic, Miri/Kani, Sentinel Integrity, Shadow-Mode Differential Testing, Contribution Gates

**Date:** 2026-07-19 · **Capture rule:** verbatim, per this repo's untracked-file-safety rule (stage
immediately on write). This is a direct continuation of the same dialogue thread as
`RAW-PROMPT-6-truthfulness-logical-deduction-kleene-unknown-2026-07-19.md` (references the
`TruthState`/`Unknown` design and the "Guardian" concept from that document directly) — treat it
as one combined research input alongside RAW-PROMPT-6, not a fully separate topic. Do not edit the
content below — corrections/grounding happen in the research/synthesis documents, not here.

**Context:** reacting to the prospect of open-sourcing this codebase (a real, tracked goal —
see `open-source-goal-adr020-2026-07-03.md` in memory, gated on secrets/EUTM), this turn covers:
(1) property-based and exhaustive testing specifically for the 3-valued Kleene `TruthState` logic
from RAW-PROMPT-6 (proptest/quickcheck-style fuzzing of the truth tables, PLUS a note that since
there are only 3 states, EXHAUSTIVE coverage of the full input space is actually achievable and
preferable to random sampling); (2) two specific external tools proposed as mandatory for the
kernel — **Miri** (MIR interpreter, for undefined-behavior detection: out-of-bounds access,
uninitialized-memory reads) and **Kani** (already planned in this roadmap, items 6/7 — reinforced
here specifically for proving the `TruthState`/Guardian logic can never reach a state the kernel
can't handle); (3) a "Sentinel" pattern — periodic/transition-time integrity-hash checks on
structured in-memory state (e.g. a hypothetical `ActiveAIContext.integrity_hash`, compared on
every read; mismatch forces a Safe State / automatic `Unknown`) as a defense against bit-flips or
memory corruption; (4) "Shadow Mode" differential testing — running the static deterministic
procedure and the AI consultant simultaneously on the same input, logging any disagreement (or any
case where the AI returns `Unknown` while the static procedure reaches `True`/`False`) as a
critical PostMortem-worthy event, as a way to measure AI quality in the field without ever trusting
it to act alone; (5) open-source contribution-gate hardening — CI must reject any PR that doesn't
pass `cargo clippy --deny warnings`, `cargo fmt --check`, AND `cargo miri test`; for critical
kernel regions, a PR should carry an explicit "proof" or written argument that it doesn't
introduce non-determinism; every commit should link to the test proving its "truthfulness."

The turn closes with a question to the operator: which parts of the kernel are currently felt to
be the LEAST verified / most uncertain, as first-verification-priority candidates — this is a
genuine open question for the operator/architect to answer, not something a research pass should
guess at.

---

## Verbatim pasted content follows

Відкриття коду — це перехід від приватної майстерні до **аудиту спільнотою**. Це чудовий крок, який вимагає від архітектури ядра додаткової стійкості до зовнішніх впливів. Оскільки для вас "правдивість" (truthfulness) є вищою за "послужливість", ядро має стати максимально "непробивним" для будь-якого зовнішнього коду.

Ось стратегічні поради щодо верифікації ядра, які допоможуть вам підтримувати рівень "космічної" надійності, навіть приймаючи контрибуції.

### 1. Верифікація логіки 3-значної логіки (Kleene Logic)

Ви впровадили стан Unknown. Це вимагає від вас верифікації того, що цей стан не "розчиняється" у системі.

- **Property-Based Testing (PBT):** Використовуйте proptest або quickcheck для генерації тисяч випадкових станів (True, False, Unknown) і перевірки таблиць істинності.
  - *Ціль:* Довести, що операція AND або OR над станами завжди повертає коректний стан, і що Unknown ніколи не призводить до непередбачуваної поведінки (Panic).
- **Exhaustive Logic Testing:** Оскільки станів лише три, ви можете (і повинні) написати тест, який проходить через **всі можливі комбінації** вхідних даних для вашої логіки ядра. Це не 955 тестів, це "повний простір станів" (state space coverage).

### 2. "Hardened" Runtime Verification (Інструменти)

Для ядра на Rust "звичайної" компіляції недостатньо. Вам потрібні інструменти, які зазирають під капот:

- **Miri (MIR Interpreter):** Це обов'язково для вашого ядра. Miri виявляє undefined behavior (невизначену поведінку) у вашому коді під час тестування (наприклад, вихід за межі масиву, використання неініціалізованої пам'яті). Якщо cargo miri test проходить — це гарантія того, що ви не зламали базові принципи безпеки пам'яті.
- **Kani Rust Verifier:** Ви згадували про математичну верифікацію. Kani — це найкращий інструмент для Rust, щоб довести відсутність panic або порушення інваріантів. Запустіть його на ваших критичних вузлах (наприклад, на Guardian). Він знайде помилку, яку не знайде жоден юніт-тест.

### 3. "Sentinel" (Вартовий) для стану системи

Використовуйте патерн "Sentinel" (або Watchdog-перевірку цілісності):

- **Memory Invariants:** Періодично (або на ключових переходах) перевіряйте стан структурованої пам'яті вашого ядра. Якщо у вас є, наприклад, ActiveAIContext, додайте integrity_hash всередину структури. При кожному зчитуванні ядро порівнює hash з поточним значенням. Якщо вони не збігаються — система переходить у Safe State (автоматично Unknown). Це захист від біт-фліпів або пошкодження пам'яті.

### 4. Диференціальне тестування (Differential Testing)

Оскільки ви плануєте мати "Статичну процедуру" та "ШІ-консультанта":

- Створіть "тіньовий режим" (shadow mode). Виконуйте і статичну процедуру, і ШІ-консультант одночасно на однакових даних.
- Якщо їхні висновки суперечливі або ШІ видає Unknown, а статична процедура — True/False, логуйте це як критичну подію для PostMortem. Це найкращий спосіб верифікувати якість вашого ШІ "в полях" без ризику для системи.

### 5. Контроль контрибуцій (Verification Gate)

Оскільки код відкритий, будь-який PR — це потенційний вектор атаки на вашу "правдивість".

- **CI/CD як "Кернел верифікатор":** Ваша збірка не повинна приймати жодного коду, який не проходить cargo clippy --deny warnings, cargo fmt та cargo miri test.
- **Formal Proof of PR:** Для критичних частин ядра вимагайте "proof" або опис того, як новий код не порушує інваріанти (наприклад, не додає недетермінізм).
- **Audit Trail:** Кожен коміт у вашому опенсорс-проєкті повинен мати посилання на тест, який підтверджує його "правдивість".

### Що я б порадив додати в першу чергу?

Я б почав з **Symbolic Execution** (наприклад, через Kani) саме для логіки TruthState. Ви повинні довести собі (і користувачам), що жодна комбінація вхідних даних не може призвести до "невизначеного" стану, який ядро не зможе обробити.

Ваш проєкт — це унікальний екземпляр "Engineering-First AI", де математика перемогла "хайп". Які саме частини ядра ви зараз вважаєте найбільш "критичними" для першочергової верифікації? Чи є там вузли, де ви відчуваєте невпевненість?

---

## Note on the closing question

The source dialogue ends by asking the operator which kernel regions feel LEAST verified /
most uncertain — a genuine question for the operator, not something for a research/synthesis
pass to answer by inference. If the eventual roadmap items need this input to be fully scoped,
flag it as a decision point rather than guessing.
