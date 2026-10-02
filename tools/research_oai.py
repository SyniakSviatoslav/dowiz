#!/usr/bin/env python3
"""tools/research_oai.py — harvest arXiv metadata over OAI-PMH, openly.

Replaces what .github/workflows/academia-*.yml tried to do (2026-10-02, operator):
those bots spoofed a browser User-Agent, sent "chaff" requests, kept only an
8-byte hash of each title and "signed" the result with a random key. This one
says who it is, honours arXiv's flow control (HTTP 503 + Retry-After), waits
between requests, and keeps the full record: id, title, authors, abstract,
categories, dates, doi — the same JSONL schema tools/research-extract.sh writes.

    tools/research_oai.py --set cs --from 2026-09-01 --until 2026-09-30 \
        --max 5000 --output papers.jsonl

FAILURES ARE LOUD: a request that keeps failing exits 2 and names the URL; an
OAI error element (badArgument, noRecordsMatch, ...) is printed with its code.
noRecordsMatch alone is an empty result, exit 0, said out loud.
"""
import argparse
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET

BASE = "https://oaipmh.arxiv.org/oai"
UA = "dowiz-research/1.0 (+https://dowiz.org; tools/research_oai.py)"
NS = {"oai": "http://www.openarchives.org/OAI/2.0/", "ax": "http://arxiv.org/OAI/arXiv/"}
PAUSE_S = 3  # arXiv's stated pace for automated access: one request every three seconds


def fetch(url, tries=5):
    for attempt in range(1, tries + 1):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": UA})
            return urllib.request.urlopen(req, timeout=60).read()
        except urllib.error.HTTPError as e:
            if e.code == 503:  # OAI-PMH flow control: the server says when to come back
                wait = int(e.headers.get("Retry-After") or 30)
                print(f"  503 flow control, waiting {wait}s (try {attempt}/{tries})", file=sys.stderr)
                time.sleep(wait)
                continue
            print(f"  HTTP {e.code} on {url} (try {attempt}/{tries})", file=sys.stderr)
        except (urllib.error.URLError, TimeoutError) as e:
            print(f"  {e} on {url} (try {attempt}/{tries})", file=sys.stderr)
        time.sleep(PAUSE_S * attempt)
    print(f"research_oai: GAVE UP after {tries} tries: {url}", file=sys.stderr)
    sys.exit(2)


def text(el, path):
    t = el.find(path, NS)
    return " ".join("".join(t.itertext()).split()) if t is not None else ""


def year_of(aid, created):
    """The year of FIRST submission. `created` is the datestamp of the version OAI
    serves (1812.02621 says 2026-09-28), so a new-style id's YYMM wins: 1812.x -> 2018."""
    m = re.match(r"(\d{2})(\d{2})\.\d{4,5}$", aid)
    if m:
        return 2000 + int(m.group(1))
    m = re.search(r"/(\d{2})\d{5}$", aid)  # old style, e.g. cs/9901001
    if m:
        yy = int(m.group(1))
        return (1900 if yy >= 91 else 2000) + yy
    return int(created[:4]) if created[:4].isdigit() else 0


def record(meta):
    a = meta.find("ax:arXiv", NS)
    authors = []
    for au in a.findall("ax:authors/ax:author", NS):
        name = " ".join(x for x in (text(au, "ax:forenames"), text(au, "ax:keyname")) if x)
        if name:
            authors.append(name)
    created = text(a, "ax:created")
    aid = text(a, "ax:id")
    return {
        "id": aid,
        "title": text(a, "ax:title"),
        "authors": authors,
        "abstract": text(a, "ax:abstract"),
        "categories": text(a, "ax:categories").split(),
        "year": year_of(aid, created),
        "arxiv_id": aid,
        "doi": text(a, "ax:doi"),
        "created": created,
        "updated": text(a, "ax:updated"),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--set", default="cs", help="OAI set: cs, math, stat, physics, q-bio, eess, econ, q-fin")
    ap.add_argument("--from", dest="frm", help="YYYY-MM-DD (datestamp, inclusive)")
    ap.add_argument("--until", help="YYYY-MM-DD (datestamp, inclusive)")
    ap.add_argument("--max", type=int, default=1000)
    ap.add_argument("--output", help="JSONL file (default stdout)")
    o = ap.parse_args()

    q = {"verb": "ListRecords", "metadataPrefix": "arXiv", "set": o.set}
    if o.frm:
        q["from"] = o.frm
    if o.until:
        q["until"] = o.until
    url = BASE + "?" + urllib.parse.urlencode(q)
    out = open(o.output, "w") if o.output else sys.stdout
    n = pages = 0
    while url and n < o.max:
        root = ET.fromstring(fetch(url))
        pages += 1
        err = root.find("oai:error", NS)
        if err is not None:
            code = err.get("code")
            if code == "noRecordsMatch":
                print("research_oai: 0 records match (noRecordsMatch)", file=sys.stderr)
                break
            print(f"research_oai: OAI error {code}: {(err.text or '').strip()}", file=sys.stderr)
            sys.exit(3)
        for rec in root.findall(".//oai:record", NS):
            if rec.find("oai:header", NS).get("status") == "deleted":
                continue
            meta = rec.find("oai:metadata", NS)
            if meta is None:
                continue
            out.write(json.dumps(record(meta), ensure_ascii=False) + "\n")
            n += 1
            if n >= o.max:
                break
        tok = root.find(".//oai:resumptionToken", NS)
        token = (tok.text or "").strip() if tok is not None else ""
        url = BASE + "?" + urllib.parse.urlencode({"verb": "ListRecords", "resumptionToken": token}) if token else None
        print(f"  page {pages}: {n} records", file=sys.stderr)
        if url:
            time.sleep(PAUSE_S)
    if o.output:
        out.close()
    print(f"research_oai: {n} records from set {o.set} in {pages} page(s)", file=sys.stderr)


if __name__ == "__main__":
    main()
