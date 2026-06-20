# Verification report · <id> · v<version> · <дата>

Вердикт: **CERTIFIED | REJECTED**
Верифікатор: loop-architect (модель: opus) · cross-opinion (M11): <модель/—>

| # | критерій | PASS/FAIL | доказ |
|---|---|---|---|
| M1 | структурна повнота (4 блоки + DNA) | | |
| M2 | 4-умовний тест | | |
| M3 | верифікація реальна (не вайб) | | |
| M4 | жорсткий вихід (ALL-must-hold) | | |
| M5 | iron principles увімкнені (no-fake-green) | | |
| M6 | skill-driven (нуль фантомних навичок) | | |
| M7 | гейти у високоризикових точках | | |
| M8 | out-of-scope + escalation | | |
| M9 | anti-cheat dry-run (зламане → RED/escalate) | | |
| M10 | пам'ять підключена | | |
| M11 | separate-agent крос-рев'ю | | |

## Якщо REJECTED
Що впало й чому → назад у DESIGN/IMPROVE.

## Anti-cheat dry-run (M9)
Фікстура зламаного стану: <опис>. Очікувано: RED/escalate. Факт: <…>. Артефакти: <шляхи>.
