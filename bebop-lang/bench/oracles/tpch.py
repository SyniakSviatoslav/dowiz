#!/usr/bin/env python3
"""bench/oracles/tpch.py -- B7 prep oracle (pending: no bebop tpch_q6/tpch_q1 gate exists
yet, docs/blueprints/B7-dsl-planner.md section 6, see bench/vs_rust/PREP-b1-b3-b7.md).
Deterministic SF-0.1-like lineitem generator (600,000 rows, stdlib LCG, integers only --
no floats anywhere so the fold is bit-exact between this oracle, the sqlite twin, and
later the store) + the Q6/Q1 fold definitions the sqlite twin (bench/tq_sqlite/
tpch_sqlite.py) and the CSV writer (bench/tq_sqlite/gen_lineitem.py) both reuse.

Columns (the ones the blueprint names for Q6/Q1), all plain i64:
  shipdate       proleptic Gregorian ordinal day number (date.toordinal()), range
                 1992-01-01 .. 1998-12-31 (2557 days) -- matches real TPC-H's date span.
  discount       integer PERCENTAGE POINTS 0..10 (TPC-H itself generates discount as
                 randint(0,10)/100 -- storing the raw integer is exact, no float).
  quantity       integer 1..50 (TPC-H range).
  extendedprice  integer CENTS = quantity * unit_price_cents (unit_price_cents in
                 [90000, 210100), a "TPC-H-like" not exact-TPC-H price range).
  returnflag     0='N' 1='A' 2='R' (TPC-H enum, coded as a small int).
  linestatus     0='O' 1='F' (TPC-H enum, coded as a small int).
  tax            integer PERCENTAGE POINTS 0..8 (TPC-H range, same reasoning as discount).

Fold units are documented, not TPC-H dollars: Q6 fold = SUM(extendedprice_cents *
discount_points); Q1 fold folds every (returnflag,linestatus) group's
(count, sum_qty, sum_extendedprice, sum_disc_price=sum(extendedprice*(100-discount)))
into one combined value via lag_common.combine, group order = ascending (returnflag,
linestatus) for determinism.
"""
import datetime
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lag_common import combine, s64, M  # noqa: E402

A, C, LCGM = 6364136223846793005, 1442695040888963407, (1 << 64) - 1
N_ROWS = 600_000
SEED = 20260906
EPOCH = datetime.date(1992, 1, 1).toordinal()
SPAN_DAYS = (datetime.date(1998, 12, 31) - datetime.date(1992, 1, 1)).days + 1  # 2557

Q6_LO = datetime.date(1994, 1, 1).toordinal()
Q6_HI = datetime.date(1995, 1, 1).toordinal()  # exclusive
Q1_CUTOFF = (datetime.date(1998, 12, 1) - datetime.timedelta(days=90)).toordinal()


def gen_rows(n=N_ROWS, seed=SEED):
    """Yields (shipdate, discount, quantity, extendedprice, returnflag, linestatus, tax)
    -- one LCG stream, deterministic, no floats."""
    x = seed
    def nxt():
        nonlocal x
        x = (x * A + C) & LCGM
        return x
    for _ in range(n):
        shipdate = EPOCH + nxt() % SPAN_DAYS
        discount = nxt() % 11          # 0..10
        quantity = 1 + nxt() % 50      # 1..50
        unit_price_cents = 90000 + nxt() % 120100
        extendedprice = quantity * unit_price_cents
        returnflag = nxt() % 3         # 0 N, 1 A, 2 R
        linestatus = nxt() % 2         # 0 O, 1 F
        tax = nxt() % 9                # 0..8
        yield (shipdate, discount, quantity, extendedprice, returnflag, linestatus, tax)


def q6_fold(rows):
    total = 0
    for shipdate, discount, quantity, extendedprice, returnflag, linestatus, tax in rows:
        if Q6_LO <= shipdate < Q6_HI and 5 <= discount <= 7 and quantity < 24:
            total += extendedprice * discount
    return s64(total & M)


def q1_fold(rows):
    # group key (returnflag, linestatus) -> [count, sum_qty, sum_extendedprice, sum_disc_price]
    groups = {}
    for shipdate, discount, quantity, extendedprice, returnflag, linestatus, tax in rows:
        if shipdate > Q1_CUTOFF:
            continue
        g = groups.setdefault((returnflag, linestatus), [0, 0, 0, 0])
        g[0] += 1
        g[1] += quantity
        g[2] += extendedprice
        g[3] += extendedprice * (100 - discount)
    vals = []
    for key in sorted(groups):
        vals.extend(groups[key])
    return combine(vals)


if __name__ == '__main__':
    rows = list(gen_rows())
    print('rows', len(rows))
    print('q6_fold', q6_fold(rows))
    print('q1_fold', q1_fold(rows))
