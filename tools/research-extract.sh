#!/usr/bin/env bash
# tools/research-extract.sh — Extract research papers from arXiv + Semantic Scholar.
# Outputs JSONL (one paper per line) to stdout or a specified file.
#
# Usage:
#   ./tools/research-extract.sh [--category cs.AI] [--max 1000] [--output papers.jsonl]
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
BASE_URL="https://export.arxiv.org/api/query"
S2_BASE="https://api.semanticscholar.org/graph/v1"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --category) CATEGORY="$2"; shift 2 ;;
        --max) MAX_RESULTS="$2"; shift 2 ;;
        --output) OUTPUT_FILE="$2"; shift 2 ;;
        --query) QUERY="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

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

    curl -s -H "User-Agent: dowiz-research/1.0 (mailto:dowiz@local)" \
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

# Now enrich with Semantic Scholar citation counts.
echo "Enriching with Semantic Scholar citations..." >&2
ENRICHED="${TMPDIR}/enriched.jsonl"
: > "${ENRICHED}"

COUNTER=0
while IFS= read -r line; do
    ARXIV_ID=$(echo "$line" | python3 -c "import json,sys; print(json.load(sys.stdin).get('arxiv_id',''))" 2>/dev/null)
    if [ -n "$ARXIV_ID" ]; then
        S2_URL="${S2_BASE}/paper/ArXiv:${ARXIV_ID}?fields=citationCount,title"
        S2_RESP=$(curl -s "${S2_URL}" 2>/dev/null || echo '{}')
        CITATIONS=$(echo "$S2_RESP" | python3 -c "import json,sys; print(json.load(sys.stdin).get('citationCount', 0))" 2>/dev/null || echo 0)
        # Add citation count to the paper
        echo "$line" | python3 -c "
import json,sys
p = json.load(sys.stdin)
p['citation_count'] = ${CITATIONS}
print(json.dumps(p, ensure_ascii=False))
" >> "${ENRICHED}" 2>/dev/null
        COUNTER=$((COUNTER + 1))
        if [ $((COUNTER % 10)) -eq 0 ]; then
            echo "  Enriched ${COUNTER}/${TOTAL}" >&2
        fi
        sleep 0.5  # S2 rate limit
    else
        echo "$line" >> "${ENRICHED}"
    fi
done < "${TMPDIR}/papers.jsonl"

if [ -n "$OUTPUT_FILE" ]; then
    cp "${ENRICHED}" "$OUTPUT_FILE"
    echo "Output written to ${OUTPUT_FILE}" >&2
else
    cat "${ENRICHED}"
fi

echo "Done. ${COUNTER} papers enriched with Semantic Scholar." >&2
