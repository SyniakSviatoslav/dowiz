#!/usr/bin/env python3
"""tools/research_cite.py — add Semantic Scholar citation counts to a JSONL corpus.

    tools/research_cite.py papers.jsonl            # rewrites the file in place

One batch request per 500 papers (POST /graph/v1/paper/batch), not one per paper.
Without a key, Semantic Scholar's shared pool answers 429 often; this retries
with growing waits and honours Retry-After. Set S2_API_KEY to use a key.

A paper Semantic Scholar could not answer gets citation_count = null, NEVER 0:
until 2026-10-02 tools/research-extract.sh wrote 0 for every 429, so "uncited"
and "not asked" looked the same. The last line says how many are null.
"""
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request

URL = "https://api.semanticscholar.org/graph/v1/paper/batch?fields=citationCount"
CHUNK = 500
TRIES = 6


def batch(ids):
    body = json.dumps({"ids": ids}).encode()
    headers = {"Content-Type": "application/json"}
    if os.environ.get("S2_API_KEY"):
        headers["x-api-key"] = os.environ["S2_API_KEY"]
    for attempt in range(1, TRIES + 1):
        try:
            req = urllib.request.Request(URL, data=body, headers=headers)
            return json.load(urllib.request.urlopen(req, timeout=90))
        except urllib.error.HTTPError as e:
            wait = int(e.headers.get("Retry-After") or 5 * attempt)
            print(f"  S2 HTTP {e.code}, waiting {wait}s (try {attempt}/{TRIES})", file=sys.stderr)
            time.sleep(wait)
        except (urllib.error.URLError, TimeoutError) as e:
            print(f"  S2 {e} (try {attempt}/{TRIES})", file=sys.stderr)
            time.sleep(5 * attempt)
    return None


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: tools/research_cite.py papers.jsonl")
    path = sys.argv[1]
    papers = [json.loads(line) for line in open(path) if line.strip()]
    for i in range(0, len(papers), CHUNK):
        chunk = papers[i:i + CHUNK]
        ids = ["ArXiv:" + re.sub(r"v\d+$", "", p.get("arxiv_id") or p["id"]) for p in chunk]
        res = batch(ids)
        if res is None:
            print(f"  chunk {i}: Semantic Scholar never answered; {len(chunk)} papers stay null", file=sys.stderr)
        for j, p in enumerate(chunk):
            hit = res[j] if res else None
            p["citation_count"] = hit.get("citationCount") if hit else None
    with open(path, "w") as f:
        for p in papers:
            f.write(json.dumps(p, ensure_ascii=False) + "\n")
    nulls = sum(p["citation_count"] is None for p in papers)
    print(f"research_cite: {len(papers) - nulls} of {len(papers)} with citations; {nulls} null (not answered, not zero)", file=sys.stderr)


if __name__ == "__main__":
    main()
