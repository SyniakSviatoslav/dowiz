#!/usr/bin/env python3
"""bench/tq_sqlite/gen_lineitem.py -- B7 prep (docs/blueprints/B7-dsl-planner.md section 6):
writes the 600,000-row deterministic lineitem table (bench/oracles/tpch.py, the SAME
generator the oracle and the sqlite twin both use) as a plain CSV under $OUT, loadable by
sqlite (bench/tq_sqlite/tpch_sqlite.py `load`) and, later, by the store (one row = one
future GbMatrix/columnar object -- out of scope here, python/shell/data only).

usage: gen_lineitem.py <out.csv>   (default: $OUT/lineitem.csv or ./lineitem.csv)
"""
import csv
import os
import sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'oracles'))
from tpch import gen_rows, N_ROWS  # noqa: E402

COLUMNS = ['shipdate', 'discount', 'quantity', 'extendedprice', 'returnflag', 'linestatus', 'tax']


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.environ.get('BEBOP_TMP', '.'), 'lineitem.csv')
    with open(out, 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(COLUMNS)
        n = 0
        for row in gen_rows():
            w.writerow(row)
            n += 1
    print('wrote', out, n, 'rows')
    assert n == N_ROWS, (n, N_ROWS)


if __name__ == '__main__':
    main()
