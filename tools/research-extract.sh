#!/usr/bin/env bash
# tools/research-extract.sh — Extract research papers from arXiv + Semantic Scholar.
# Outputs JSONL (one paper per line) to stdout or a specified file.
#
# Usage:
#   ./tools/research-extract.sh [--category cs.AI] [--max 1000] [--output papers.jsonl] [--no-cite]
#   ./tools/research-extract.sh --query 'abs:%22differential+dataflow%22' --max 150 --output dd.jsonl
#   ./tools/research-extract.sh --oai cs --from 2026-09-01 --until 2026-09-30 --max 5000 --output cs.jsonl
#
# --oai SET harvests a whole arXiv set over OAI-PMH (tools/research_oai.py); the default is
# the search API. Citations come from tools/research_cite.py: ONE batch request per 500
# papers, retried on 429, and a paper Semantic Scholar did not answer gets null, never 0.
# (Until 2026-10-02 this script asked once per paper with a 0.5 s pause, which the unkeyed
# pool answers 429, and wrote every 429 as citation_count 0.) S2_API_KEY uses a key.
#
# Free APIs used:
#   - arXiv API (no key required): https://export.arxiv.org/api/
#   - Semantic Scholar API (no key required for basic): https://api.semanticscholar.org/graph/v1
#
# Rate limits:
#   - arXiv: polite use (~1 req/3s recommended)
#   - Semantic Scholar: ~1 req/s without key, 100 req/s with key

set -euo pipefail

CATEGORY="cs.AI"
MAX_RESULTS=100
OUTPUT_FILE=""
QUERY=""
OAI_SET=""
FROM=""
UNTIL=""
CITE=1
BASE_URL="https://export.arxiv.org/api/query"
S2_BASE="https://api.semanticscholar.org/graph/v1"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --category) CATEGORY="$2"; shift 2 ;;
        --max) MAX_RESULTS="$2"; shift 2 ;;
        --output) OUTPUT_FILE="$2"; shift 2 ;;
        --query) QUERY="$2"; shift 2 ;;
        --oai) OAI_SET="$2"; shift 2 ;;
        --from) FROM="$2"; shift 2 ;;
        --until) UNTIL="$2"; shift 2 ;;
        --no-cite) CITE=0; shift ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

HERE="$(cd "$(dirname "$0")" && pwd)"

if [ -n "$OAI_SET" ]; then
    [ -n "$OUTPUT_FILE" ] || { echo "research-extract: --oai needs --output" >&2; exit 1; }
    python3 "$HERE/research_oai.py" --set "$OAI_SET" ${FROM:+--from "$FROM"} ${UNTIL:+--until "$UNTIL"} \
        --max "$MAX_RESULTS" --output "$OUTPUT_FILE" || exit $?
    [ "$CITE" = 1 ] && { python3 "$HERE/research_cite.py" "$OUTPUT_FILE" || exit $?; }
    exit 0
fi

if [ -z "$QUERY" ]; then
    QUERY="cat:${CATEGORY}"
fi

echo "Extracting up to ${MAX_RESULTS} papers from arXiv (${QUERY})..." >&2

# arXiv API: paginate through results (max 3000 per query, 100 per page).
TOTAL=0
START=0
MAX_PER_PAGE=100
PAGES=$(( (MAX_RESULTS + MAX_PER_PAGE - 1) / MAX_PER_PAGE ))

TMPDIR=$(mktemp -d)
trap "rm -rf ${TMPDIR}" EXIT

for PAGE in $(seq 0 $((PAGES - 1))); do
    REMAIN=$((MAX_RESULTS - TOTAL))
    [ "$REMAIN" -le 0 ] && break
    THIS_PAGE=$(( REMAIN < MAX_PER_PAGE ? REMAIN : MAX_PER_PAGE ))

    URL="${BASE_URL}?search_query=${QUERY}&start=${START}&max_results=${THIS_PAGE}"
    echo "  Page $((PAGE+1)): start=${START}, count=${THIS_PAGE}" >&2

    curl -s -H "User-Agent: dowiz-research/1.0 (+https://dowiz.org; tools/research-extract.sh)" \
        "${URL}" > "${TMPDIR}/page_${PAGE}.xml" 2>/dev/null || {
        echo "  WARNING: arXiv request failed at start=${START}" >&2
        START=$((START + THIS_PAGE))
        continue
    }

    # Parse XML with a simple approach: extract each <entry> block.
    python3 -c "
import xml.etree.ElementTree as ET
import json,sys

ns = {'atom': 'http://www.w3.org/2005/Atom',
      'arxiv': 'http://arxiv.org/schemas/atom'}

tree = ET.parse('${TMPDIR}/page_${PAGE}.xml')
root = tree.getroot()

for entry in root.findall('atom:entry', ns):
    paper_id = entry.find('atom:id', ns).text.strip()
    title = ''.join(entry.find('atom:title', ns).itertext()).strip().replace('\n', ' ')
    summary = ''.join(entry.find('atom:summary', ns).itertext()).strip().replace('\n', ' ')
    authors = [a.find('atom:name', ns).text for a in entry.findall('atom:author', ns)]
    categories = [c.get('term') for c in entry.findall('atom:category', ns)]
    published = entry.find('atom:published', ns).text[:4]

    arxiv_id = paper_id.split('/abs/')[-1] if '/abs/' in paper_id else paper_id

    # Extract DOI
    doi = ''
    for link in entry.findall('atom:link', ns):
        if link.get('title') == 'doi':
            doi = link.get('href', '')

    paper = {
        'id': arxiv_id,
        'title': title,
        'authors': authors,
        'abstract': summary[:2000],
        'categories': categories,
        'year': int(published) if published.isdigit() else 0,
        'arxiv_id': arxiv_id,
        'doi': doi,
    }
    print(json.dumps(paper, ensure_ascii=False))
" 2>/dev/null >> "${TMPDIR}/papers.jsonl"

    # Count lines added.
    NEW=$(wc -l < "${TMPDIR}/papers.jsonl" 2>/dev/null || echo 0)
    TOTAL=$NEW

    START=$((START + THIS_PAGE))
    sleep 3  # Polite rate limiting
done

echo "Extracted ${TOTAL} papers." >&2

# Citations: one batch per 500 papers (tools/research_cite.py); unanswered = null, never 0.
ENRICHED="${TMPDIR}/papers.jsonl"
if [ "$CITE" = 1 ] && [ -s "$ENRICHED" ]; then
    python3 "$HERE/research_cite.py" "$ENRICHED" || exit $?
fi

if [ -n "$OUTPUT_FILE" ]; then
    cp "${ENRICHED}" "$OUTPUT_FILE"
    echo "Output written to ${OUTPUT_FILE}" >&2
else
    cat "${ENRICHED}"
fi

echo "Done. ${TOTAL} papers." >&2
