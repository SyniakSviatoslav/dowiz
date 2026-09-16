"""Server time per endpoint, against the deployment -- and a gate on it.

A NUMBER IS EITHER MEASURED OR IT IS A HYPOTHESIS. This box is a PRoot
container on a phone, and the same endpoint measured three times in a row has
given 104, 190 and 269 ms. Everything below is shaped by that measurement,
not by what a benchmark usually looks like.

How the measurement works
-------------------------
1. ONE KEEP-ALIVE CONNECTION for the whole run. With a fresh connection per
   request (what this file used to do) curl showed connect=142 ms and
   tls=372 ms: 370 ms of every sample was the handshake, and it was the
   noisiest part. Measured on 2026-09-16, 15 samples per endpoint:
       fresh connection : /healthz median 194 ms, endpoint IQRs 80-255 ms,
                          1-6 of 15 samples dropped per endpoint
       keep-alive       : /healthz median  40 ms, endpoint IQRs 18-81 ms
                          (analytics 173), 0 samples dropped
   Keep-alive is two to four times tighter, so that is what the gate uses.
2. ROUNDS, INTERLEAVED. Each round walks every endpoint once per sample, so
   a slow minute on the network hits all endpoints alike instead of landing
   on whichever one happened to be running. Round medians are reported
   separately: they are the independent repetitions the verdict needs.
3. ROBUST STATISTICS. Median and quartiles, never a mean -- a single 600 ms
   sample (they happen: analytics max=591 in a run whose median was 278)
   moves a mean by 20 ms and a median by nothing.
4. SERVER TIME = endpoint median - /healthz median of the SAME run. /healthz
   does no I/O, so it is the network floor for this run; subtracting it
   makes a slow uplink look like a slow uplink instead of a slow endpoint.

The criterion (why the gate is quiet on an unchanged service)
-------------------------------------------------------------
An endpoint is called a REGRESSION only if BOTH hold:
   (a) this run's server-time median  >  baseline Q3 + margin,
       margin = max(60 ms, 25 % of the baseline median);
   (b) a MAJORITY of this run's rounds have a round median above the same
       threshold -- the regression must reproduce, not just average in.
Why Q3 and not the median: the baseline's upper quartile is the level a
healthy run reaches one time in four; anything below it is ordinary.
Why 60 ms: one D1 query from this Worker is roughly 60 ms, so that is the
smallest change worth a build failure, and it is also about the size of a
round-to-round swing (dashboard rounds: 230/200/186). One D1 round trip is
therefore NOT reliably detectable in a single run here; two are. The table
prints, per endpoint, the smallest regression this gate can actually see
("видно від"), so nobody mistakes silence for "no regression".
(a) exceeding without (b) is printed as "підозра" (suspect) and does NOT
fail: it is a hypothesis, and the next run will either reproduce it or not.

Measured on 2026-09-16 against the UNCHANGED service, baseline n=30/endpoint:
   4 measured runs (3 live + 1 replayed), 28 endpoint verdicts: 0 regressions,
   4 "підозра", 24 ✓ -- while single rounds swung as far as +129 ms over the
   baseline median (dashboard rounds 168/266/142). Two further runs during a
   sibling deploy answered 5xx (Cloudflare 1101/1102) and exited 2, not 0.
   Can it fail? Against a copy of the baseline with every value HALVED (a
   synthetic 2x slowdown): analytics ✗ 190 > 159 in 3/3 rounds, customers
   ✗ 158 > 148 in 2/3, three "підозра", and features ✓ at 108 vs 115 --
   a 2x slowdown of a 50 ms endpoint is +58 ms, under the 60 ms floor. So
   the gate sees a +70..100 ms regression on a ~100-180 ms endpoint and
   does NOT see a 50 ms one; the column says so per endpoint.

An endpoint with fewer than two thirds of its samples answered 200 (a 5xx
burst -- seen live on 2026-09-16 as Cloudflare 1101/1102 during a sibling
deploy), or with no baseline row, is "не виміряно" and never counted as a
pass; the codes seen are printed on the first line.

Usage
-----
    python3 tools/live-checks/bench.py                  # measure + compare
    python3 tools/live-checks/bench.py --record         # pool into baseline
    python3 tools/live-checks/bench.py --rounds 3 --per 5 --baseline FILE
Exit: 0 no defensible regression; 1 regression reproduced; 2 could not
measure (login failed, no baseline, too many dropped samples).
"""
import argparse, http.client, json, os, statistics, sys, time
from datetime import datetime, timezone

HOST = "dowiz-api.sviatoslavsyniak.workers.dev"
UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
      "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")   # else Cloudflare 403
H = {"accept": "application/json", "user-agent": UA}
FLOOR = "/healthz"
PATHS = ["/api/public/locations/dubin-sushi/menu", "/api/owner/dashboard",
         "/api/owner/orders", "/api/owner/analytics?days=30", "/api/owner/stock",
         "/api/owner/features", "/api/owner/customers"]
MARGIN_MS, MARGIN_REL = 60.0, 0.25
HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_BASELINE = os.path.join(HERE, "..", "..", "scripts", "perf-baseline.json")


class NotMeasured(Exception):
    """The service could not be measured. Exit 2, never a verdict."""


def quartiles(xs):
    xs = sorted(xs); n = len(xs)
    return statistics.median(xs), xs[n // 4], xs[(3 * n) // 4]


class Probe:
    def __init__(self, tok):
        self.tok, self.conn, self.codes = tok, None, {}

    def get(self, path):
        """Round-trip in ms, or None. A failure drops the sample and the
        connection; it never becomes a number."""
        hh = dict(H)
        if path != FLOOR: hh["authorization"] = "Bearer " + self.tok
        if self.conn is None:
            self.conn = http.client.HTTPSConnection(HOST, timeout=60)
        a = time.perf_counter()
        try:
            self.conn.request("GET", path, headers=hh)
            r = self.conn.getresponse(); r.read()
        except Exception as e:
            self.conn.close(); self.conn = None
            self.codes[type(e).__name__] = self.codes.get(type(e).__name__, 0) + 1
            return None
        dt = (time.perf_counter() - a) * 1000
        if r.status != 200:
            self.codes[r.status] = self.codes.get(r.status, 0) + 1
            return None
        return dt


def login(tries=3, pause=5.0):
    """Bounded retry: a deploy in flight costs seconds, and it should not
    cost the whole run. Three tries, five seconds apart, then NotMeasured
    naming every status seen."""
    seen = []
    for i in range(tries):
        c = http.client.HTTPSConnection(HOST, timeout=60)
        try:
            c.request("POST", "/api/auth/login",
                      body=json.dumps({"email": "ana@dubin.al", "password": "dubin-owner"}),
                      headers={**H, "content-type": "application/json"})
            r = c.getresponse(); body = r.read()
        except Exception as e:
            seen.append(type(e).__name__); body = b""; r = None
        finally:
            c.close()
        if r is not None and r.status == 200:
            return json.loads(body)["access_token"]
        if r is not None: seen.append(f"HTTP {r.status} {body[:120]!r}")
        if i + 1 < tries: time.sleep(pause)
    raise NotMeasured(f"вхід не вдався {tries} рази поспіль: " + " | ".join(seen))


def measure(rounds, per, pause):
    tok = login(); pr = Probe(tok); pr.get(FLOOR)          # warm the connection; discarded
    raw = {p: [[] for _ in range(rounds)] for p in [FLOOR] + PATHS}
    for rd in range(rounds):
        for _ in range(per):
            for p in [FLOOR] + PATHS:
                time.sleep(pause)
                v = pr.get(p)
                if v is not None: raw[p][rd].append(v)
    return raw, pr.codes


def summarize(raw, rounds, per):
    """Per path: server-time median/Q1/Q3, per-round server medians, n."""
    floor_all = [v for rd in raw[FLOOR] for v in rd]
    need = max(3, (2 * rounds * per + 2) // 3)          # two thirds of the samples
    if len(floor_all) < need:
        raise NotMeasured(f"{FLOOR} дав лише {len(floor_all)} з {rounds*per} відповідей")
    fl_med, fl_q1, fl_q3 = quartiles(floor_all)
    out = {FLOOR: {"n": len(floor_all), "med": fl_med, "q1": fl_q1, "q3": fl_q3,
                   "samples": floor_all}}
    for p in PATHS:
        allv = [v for rd in raw[p] for v in rd]
        if len(allv) < need:
            out[p] = {"n": len(allv), "dropped": True}; continue
        m, q1, q3 = quartiles(allv)
        out[p] = {"n": len(allv), "raw_med": m,
                  "med": m - fl_med, "q1": q1 - fl_med, "q3": q3 - fl_med,
                  "rounds": [statistics.median(rd) - fl_med for rd in raw[p] if rd],
                  "samples": [v - fl_med for v in allv]}
    return out


def load_baseline(path):
    try:
        with open(path) as f: return json.load(f)
    except FileNotFoundError:
        return None


def record(path, summ, rounds, per):
    """Pool this run's server-time samples into the baseline. Pooling across
    runs on different days is the point: a baseline from one minute measures
    that minute."""
    dropped = [p for p in PATHS if summ[p].get("dropped")]
    if dropped:
        raise NotMeasured(f"не записую базову лінію: замало відповідей для {dropped}")
    b = load_baseline(path) or {"host": HOST, "runs": [], "paths": {}}
    b["runs"].append({"at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
                      "rounds": rounds, "per": per,
                      "healthz_med_ms": round(summ[FLOOR]["med"], 1)})
    for p in PATHS:
        s = summ[p]
        if s.get("dropped"): continue
        row = b["paths"].setdefault(p, {"samples": []})
        row["samples"] = [round(v, 1) for v in row["samples"] + s["samples"]]
        m, q1, q3 = quartiles(row["samples"])
        row.update({"n": len(row["samples"]), "med": round(m, 1),
                    "q1": round(q1, 1), "q3": round(q3, 1)})
    b["criterion"] = {"margin_ms": MARGIN_MS, "margin_rel": MARGIN_REL,
                      "rule": "regression iff run median > q3+margin AND majority of rounds > q3+margin"}
    with open(path, "w") as f: json.dump(b, f, indent=1, ensure_ascii=False)
    return b


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rounds", type=int, default=3)
    ap.add_argument("--per", type=int, default=5, help="samples per endpoint per round")
    ap.add_argument("--pause", type=float, default=0.25)
    ap.add_argument("--baseline", default=os.path.normpath(DEFAULT_BASELINE))
    ap.add_argument("--record", action="store_true", help="pool this run into the baseline")
    ap.add_argument("--json", help="write this run's summary here")
    a = ap.parse_args()

    t0 = time.time()
    codes = {}
    try:
        raw, codes = measure(a.rounds, a.per, a.pause)
        summ = summarize(raw, a.rounds, a.per)
    except NotMeasured as e:
        print(f"  НЕ ВИМІРЯНО: {e}" + (f"; відкинуто: {codes}" if codes else "")); return 2
    if a.json:
        with open(a.json, "w") as f: json.dump(summ, f, indent=1)
    fl = summ[FLOOR]
    print(f"  {a.rounds} раунди × {a.per} зразків, keep-alive, {time.time()-t0:.0f} с"
          + (f"; відкинуто: {codes}" if codes else ""))
    print(f"  {'база (healthz)':<40} {fl['med']:5.0f} мс   IQR {fl['q1']:.0f}–{fl['q3']:.0f}  n={fl['n']}")

    if a.record:
        try:
            b = record(a.baseline, summ, a.rounds, a.per)
        except NotMeasured as e:
            print(f"  НЕ ВИМІРЯНО: {e}"); return 2
        print(f"  записано в {a.baseline}: {len(b['runs'])} прогонів")
        for p in PATHS:
            r = b["paths"].get(p)
            if r: print(f"  {p:<40} база {r['med']:5.0f} мс  Q1 {r['q1']:.0f}  Q3 {r['q3']:.0f}  n={r['n']}")
        return 0

    base = load_baseline(a.baseline)
    if base is None:
        print(f"  НЕ ВИМІРЯНО: немає базової лінії {a.baseline} (створи через --record)")
        return 2
    print(f"  порівняння з {os.path.relpath(a.baseline)} "
          f"({len(base['runs'])} прогонів, останній {base['runs'][-1]['at']})")
    print(f"  правило: регресія ⇔ медіана > Q3+max({MARGIN_MS:.0f} мс, {MARGIN_REL:.0%}·медіани бази) "
          f"І більшість раундів вище порогу")
    ok = bad = suspect = unmeasured = 0
    for p in PATHS:
        s = summ[p]; r = base["paths"].get(p)
        if s.get("dropped"):
            print(f"  ? {p:<38} НЕ ВИМІРЯНО: лише {s['n']} з {a.rounds*a.per} відповідей"); unmeasured += 1; continue
        if r is None:
            print(f"  ? {p:<38} сервер ~{s['med']:4.0f} мс  НЕ ВИМІРЯНО: немає в базовій лінії"); unmeasured += 1; continue
        thr = r["q3"] + max(MARGIN_MS, MARGIN_REL * r["med"])
        above = sum(1 for x in s["rounds"] if x > thr)
        hit_med, hit_rounds = s["med"] > thr, above * 2 > len(s["rounds"])
        rs = "/".join(f"{x:.0f}" for x in s["rounds"])
        line = (f"{p:<38} {s['raw_med']:4.0f} мс · сервер ~{s['med']:4.0f} мс "
                f"IQR {s['q1']:.0f}–{s['q3']:.0f}  раунди {rs}  "
                f"база {r['med']:.0f} (Q3 {r['q3']:.0f}, n={r['n']})  поріг {thr:.0f}  "
                f"видно від +{thr - r['med']:.0f}")
        if hit_med and hit_rounds:
            bad += 1; print(f"  ✗ {line}  РЕГРЕСІЯ: {s['med']:.0f} > {thr:.0f} у {above}/{len(s['rounds'])} раундах")
        elif hit_med or above:
            suspect += 1; print(f"  ~ {line}  підозра: медіана {'вище' if hit_med else 'нижче'} порогу, "
                                f"раунди вище {above}/{len(s['rounds'])} — не відтворилося, не рахую")
        else:
            ok += 1; print(f"  ✓ {line}")
    print(f"\n{ok} пройшло, {bad} впало, {suspect} підозра, {unmeasured} не виміряно")
    if bad: return 1
    if unmeasured: return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
