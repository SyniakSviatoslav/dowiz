#!/usr/bin/env python3
"""
analyze-bottlenecks.py — повтори + time-bottlenecks по git-історії (нефрагільна версія).

Доповнення 2 з governance-доку. Окрема дія on-demand — НЕ запускати в task-флоу.

Чому Python, а не bash-однорядки:
  - тип рядка визначається ЯВНИМ розділювачем (\x01), а не вгадуванням "це схоже на дату"
    => файл на кшталт 2026-roadmap.md більше не плутається з датою;
  - паузи рахуються зі станом (таймстемпи групуються per-файл) — pipe так не вміє;
  - перейменування детектуються (-M), churn не дробиться навпіл (атрибуція все одно
    наближена — повний rename-follow історії = окрема задача, див. ОБМЕЖЕННЯ);
  - падає ГОЛОСНО (не git-репо / shallow / 0 комітів), а не тихо бреше.

Запуск (у корені репо):
    python3 analyze-bottlenecks.py
    python3 analyze-bottlenecks.py --since "3 months ago" --top 25
    python3 analyze-bottlenecks.py --min-days 3        # поріг "хронічності"

ОБМЕЖЕННЯ (чесно):
  - merge-коміти виключені (--no-merges), щоб не дублювати/не плутати списки файлів;
  - cross-rename атрибуція наближена; для точного сліду одного файлу — `git log --follow <path>`;
  - MemPalace і ~/.claude-логи лише ПЕРЕВІРЯЮТЬСЯ на наявність (їх формат тут не парситься,
    щоб не вгадувати наосліп) — див. STEP-ZERO у виводі.
"""

import argparse
import statistics
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

SENTINEL = "\x01"  # байт, якого практично не буває на початку git-шляху

# Шляхи, що шумлять у churn і не є "роботою" (підлаштуй під репо)
IGNORE_PREFIXES = ("node_modules/", "dist/", "build/", ".next/", "coverage/")
IGNORE_SUFFIXES = ("pnpm-lock.yaml", "package-lock.json", "yarn.lock", ".min.js", ".map")

# Коміти-"переробки": сигнал борсання навколо області
REWORK_WORDS = ("fix", "revert", "refactor", "hotfix", "wip", "retry", "again", "oops")


def die(msg: str) -> None:
    print(f"\nПОМИЛКА: {msg}\n", file=sys.stderr)
    sys.exit(1)


def run_git(args: list[str]) -> str:
    try:
        out = subprocess.run(
            ["git", "-c", "core.quotePath=false", *args],
            check=True, capture_output=True, text=True,
        )
        return out.stdout
    except FileNotFoundError:
        die("git не знайдено в PATH.")
    except subprocess.CalledProcessError as e:
        die(f"git {' '.join(args)} впав:\n{e.stderr.strip()}")
    return ""  # недосяжно


def guard_repo() -> None:
    """STEP-ZERO: переконатись, що історії можна довіряти (інакше аналіз бреше)."""
    inside = run_git(["rev-parse", "--is-inside-work-tree"]).strip()
    if inside != "true":
        die("це не git-робоче дерево.")
    shallow = run_git(["rev-parse", "--is-shallow-repository"]).strip()
    if shallow == "true":
        die("репозиторій SHALLOW — обрізана історія тихо спотворить churn. "
            "Дороби повну історію (`git fetch --unshallow`) і повтори.")


def is_ignored(path: str) -> bool:
    return path.startswith(IGNORE_PREFIXES) or path.endswith(IGNORE_SUFFIXES)


def collect(since: str | None):
    """
    Один прохід git-логу. Заголовок кожного коміта помічений SENTINEL'ом:
        \x01<unix_ts>\x01<subject>
        path/one
        path two/with space.tsx
    Тип рядка визначається наявністю SENTINEL, не вмістом рядка.
    """
    fmt = f"--format={SENTINEL}%ct{SENTINEL}%s"
    args = ["log", "--no-merges", "-M", "--name-only", fmt]
    if since:
        args += [f"--since={since}"]
    raw = run_git(args)

    commit_count = 0
    file_commits: dict[str, int] = defaultdict(int)          # частота (коміти)
    file_days: dict[str, set[str]] = defaultdict(set)        # унікальні дні
    file_times: dict[str, list[int]] = defaultdict(list)     # ts для пауз
    rework_files: dict[str, int] = defaultdict(int)          # коміти-переробки
    theme: dict[str, int] = defaultdict(int)                 # prefix/scope сабджектів

    cur_ts: int | None = None
    cur_rework = False

    for line in raw.split("\n"):
        if line.startswith(SENTINEL):
            commit_count += 1
            _, ts_s, subject = line.split(SENTINEL, 2)
            cur_ts = int(ts_s) if ts_s.isdigit() else None
            subj = subject.strip().lower()
            cur_rework = any(w in subj for w in REWORK_WORDS)
            key = subject.split(":")[0].split("(")[0].strip() or "(no-subject)"
            theme[key] += 1
            continue
        path = line.strip()
        if not path or is_ignored(path):
            continue
        file_commits[path] += 1
        if cur_ts is not None:
            file_times[path].append(cur_ts)
            day = subprocess_day(cur_ts)
            file_days[path].add(day)
        if cur_rework:
            rework_files[path] += 1

    if commit_count == 0:
        die("0 комітів розпарсено (порожній репо або занадто вузьке --since).")

    return commit_count, file_commits, file_days, file_times, rework_files, theme


def subprocess_day(ts: int) -> str:
    """Локальний день з unix-ts без зовнішніх залежностей."""
    import datetime as _dt
    return _dt.datetime.fromtimestamp(ts).strftime("%Y-%m-%d")


def pause_stats(times: list[int]) -> tuple[int, float]:
    """Макс і медіанний gap між сусідніми комітами файлу, у днях."""
    if len(times) < 2:
        return 0, 0.0
    s = sorted(times)
    gaps_days = [(b - a) / 86400.0 for a, b in zip(s, s[1:])]
    return round(max(gaps_days)), round(statistics.median(gaps_days), 1)


def top(d: dict[str, int], n: int) -> list[tuple[str, int]]:
    return sorted(d.items(), key=lambda kv: kv[1], reverse=True)[:n]


def table(title: str, rows: list[tuple], headers: tuple) -> None:
    print(f"\n=== {title} ===")
    if not rows:
        print("  (порожньо)")
        return
    widths = [len(h) for h in headers]
    srows = [tuple(str(c) for c in r) for r in rows]
    for r in srows:
        for i, c in enumerate(r):
            widths[i] = max(widths[i], len(c))
    line = "  " + "  ".join(h.ljust(widths[i]) for i, h in enumerate(headers))
    print(line)
    print("  " + "  ".join("-" * widths[i] for i in range(len(headers))))
    for r in srows:
        print("  " + "  ".join(c.ljust(widths[i]) for i, c in enumerate(r)))


def step_zero_external() -> None:
    print("\n=== STEP-ZERO · зовнішні джерела (лише наявність) ===")
    mp = Path.home() / ".mempalace"
    if mp.exists():
        print(f"  MemPalace: знайдено {mp} — запусти `mempalace status` для статистики палацу.")
    else:
        print("  MemPalace: ~/.mempalace НЕ знайдено — палац не заповнений; "
              "тематичні повтори бери лише з git/логів.")
    cc = Path.home() / ".claude"
    if cc.exists():
        print(f"  Claude Code: знайдено {cc} — там per-message таймстемпи (wall-clock на завдання). "
              "Парсинг формату навмисно не реалізовано тут (формат підтвердь, тоді додамо).")
    else:
        print("  Claude Code: ~/.claude НЕ знайдено — wall-clock на завдання недоступний звідси.")


def main() -> None:
    ap = argparse.ArgumentParser(description="Повтори + time-bottlenecks по git-історії.")
    ap.add_argument("--since", default=None, help="напр. '3 months ago' (за замовч. — уся історія)")
    ap.add_argument("--top", type=int, default=20, help="скільки рядків у кожному списку")
    ap.add_argument("--min-days", type=int, default=2,
                    help="мін. кількість РІЗНИХ днів, щоб файл вважався хронічним/для пауз")
    args = ap.parse_args()

    guard_repo()
    (commit_count, file_commits, file_days, file_times,
     rework_files, theme) = collect(args.since)

    window = args.since or "уся історія"
    print(f"\nРозпарсено комітів (без merge): {commit_count}   |   вікно: {window}")

    # (A) ЧАСТОТА
    table("A · ЧАСТОТА — найбільше комітів на файл (churn)",
          top(file_commits, args.top), ("commits", "file"))

    # (B) ХРОНІЧНІСТЬ — унікальні дні
    days_rank = {f: len(d) for f, d in file_days.items() if len(d) >= args.min_days}
    table(f"B · ХРОНІЧНІСТЬ — на скількох РІЗНИХ днях чіпали (>= {args.min_days})",
          top(days_rank, args.top), ("days", "file"))

    # (C) КОНЦЕНТРАЦІЯ ПЕРЕРОБОК
    table("C · ПЕРЕРОБКИ — файли в комітах fix/revert/refactor/wip/...",
          top(rework_files, args.top), ("rework", "file"))

    # (D) ПАУЗИ — макс gap серед "хронічних"
    pause_rows = []
    for f in days_rank:  # лише ті, що повертались на багатьох днях
        mx, med = pause_stats(file_times[f])
        pause_rows.append((mx, med, len(file_days[f]), f))
    pause_rows.sort(reverse=True)
    table("D · ПАУЗИ — найдовший простій між дотиками (дні); тема 'зависала' і верталась",
          [(mx, med, dys, f) for mx, med, dys, f in pause_rows[:args.top]],
          ("max_gap", "median_gap", "days", "file"))

    # (E) ПЕРЕТИН — і часто, і (хронічно АБО борсається) => систематизувати ПЕРШИМ
    freq_set = {f for f, _ in top(file_commits, args.top)}
    chronic_set = {f for f, _ in top(days_rank, args.top)}
    rework_set = {f for f, _ in top(rework_files, args.top)}
    intersection = freq_set & (chronic_set | rework_set)
    inter_rows = sorted(
        ((file_commits[f], len(file_days.get(f, [])), rework_files.get(f, 0), f)
         for f in intersection),
        reverse=True,
    )
    table("E · ПЕРЕТИН (повтор × час) — КАНДИДАТИ НА СИСТЕМАТИЗАЦІЮ ПЕРШИМИ",
          inter_rows, ("commits", "days", "rework", "file"))

    # Теми
    table("ТЕМИ — частота за prefix/scope сабджектів комітів",
          top(theme, args.top), ("count", "prefix"))

    step_zero_external()

    print("\nДія за результатом: топ-перетин (E) винести з повторення — "
          "у MemPalace L0/L1 pin / doc / skill / hook.\n")


if __name__ == "__main__":
    main()
