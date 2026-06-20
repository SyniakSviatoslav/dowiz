---
description: Тонкий вхід до loop-architect — build нової / improve наявної / verify петлі. Делегує суб-агенту; повертає вердикт CERTIFIED/REJECTED + шлях до report.
argument-hint: <build|improve|verify> <деталі: ціль/проблема/навички або loop_id+failure>
allowed-tools: Task, Read, Write
---

Розбери «$ARGUMENTS» на режим (build|improve|verify) і деталі.
Use the loop-architect subagent on: «Режим: <режим>. Деталі: <деталі>. Виконай свою петлю DESIGN→SELF-VERIFY→DRY-RUN→CERTIFY→REGISTER за рубрикою M1–M11. НЕ випускай несертифіковану петлю.»
Покажи людині: вердикт (CERTIFIED/REJECTED), що саме впало (якщо REJECTED), шляхи до loops/<id>.yaml і loops/reports/<id>-<ver>.md. Опц.: запропонуй CROSS-OPINION незалежною моделлю через OpenRouter-міст (M11).
