#!/usr/bin/env bash
# tools/research-recursive.sh — Recursive parallel research paper extraction.
#
# Uses fan-out parallelism: each iteration queries multiple APIs simultaneously,
# then uses extracted patterns as seeds for the next iteration.
# Growth model: 100 → 500 → 2,500 → 12,500 → 62,500 → 100,000+
#
# Parallelism: 'xargs -P N' fans out N simultaneous curl requests.
# Rate limits: arXiv (1 req/3s), Semantic Scholar (~100 req/s with key)
#
# Usage:
#   ./tools/research-recursive.sh [--max 100000] [--output-dir ./research]

set -euo pipefail

MAX_PAPERS=100000
OUTPUT_DIR="./research"
JOBS=20                   # Parallel API calls per iteration
ARXIV_DELAY=3             # Seconds between arXiv requests
CURRENT=0
ITERATION=0

# Seed keywords for iteration 0
SEEDS=(
    "transformer" "diffusion" "GAN" "large+language+model"
    "reinforcement+learning" "graph+neural+network" "attention+mechanism"
    "self-supervised+learning" "contrastive+learning" "knowledge+distillation"
    "mixture+of+experts" "state+space+model" "federated+learning"
    "meta-learning" "neural+architecture+search" "quantization"
    "pruning" "LoRA" "prompt+engineering" "retrieval-augmented"
)

while [[ $# -gt 0 ]]; do
    case "$1" in
        --max) MAX_PAPERS="$2"; shift 2 ;;
        --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
        --jobs) JOBS="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

mkdir -p "${OUTPUT_DIR}/iterations"
mkdir -p "${OUTPUT_DIR}/patterns"

echo "================================================"
echo "Recursive Research Extraction"
echo "  Target: ${MAX_PAPERS} papers"
echo "  Output: ${OUTPUT_DIR}"
echo "  Parallel: ${JOBS} jobs"
echo "================================================"

# Build the research_ingest binary if needed.
if [ ! -f "target/debug/research_ingest" ]; then
    echo "Building research_ingest..."
    cd kernel && cargo build --bin research_ingest 2>/dev/null && cd ..
fi

RESEARCH_BIN="kernel/target/debug/research_ingest"

# Check for Semantic Scholar API key.
S2_KEY="${SEMANTIC_SCHOLAR_API_KEY:-}"

# ─── arXiv API call ────────────────────────────────────────────────────────
fetch_arxiv() {
    local query="$1"
    local start="$2"
    local max="$3"
    local outfile="$4"
    local url="https://export.arxiv.org/api/query?search_query=all:${query}&start=${start}&max_results=${max}"

    curl -s --max-time 30 -H "User-Agent: dowiz-research/1.0" "${url}" > "/tmp/arxiv_$$.xml" 2>/dev/null || return 1

    python3 -c "
import xml.etree.ElementTree as ET, json, sys
ns = {'atom': 'http://www.w3.org/2005/Atom'}
tree = ET.parse('/tmp/arxiv_$$.xml')
for entry in tree.findall('atom:entry', ns):
    pid = entry.find('atom:id', ns).text.strip() if entry.find('atom:id', ns) is not None else ''
    title = ''.join(entry.find('atom:title', ns).itertext()).strip().replace('\n', ' ') if entry.find('atom:title', ns) is not None else ''
    summary = ''.join(entry.find('atom:summary', ns).itertext()).strip().replace('\n', ' ') if entry.find('atom:summary', ns) is not None else ''
    authors = [a.find('atom:name', ns).text for a in entry.findall('atom:author', ns) if a.find('atom:name', ns) is not None]
    categories = [c.get('term') for c in entry.findall('atom:category', ns)]
    published = entry.find('atom:published', ns).text[:4] if entry.find('atom:published', ns) is not None else '0'
    arxiv_id = pid.split('/abs/')[-1] if '/abs/' in pid else pid
    print(json.dumps({'id':arxiv_id,'title':title,'authors':authors,'abstract':summary[:2000],'categories':categories,'year':int(published) if published.isdigit() else 0,'arxiv_id':arxiv_id}, ensure_ascii=False))
" 2>/dev/null >> "${outfile}"
    rm -f "/tmp/arxiv_$$.xml"
}

# ─── Semantic Scholar API call ─────────────────────────────────────────────
fetch_s2() {
    local query="$1"
    local max="$2"
    local outfile="$3"
    local url="https://api.semanticscholar.org/graph/v1/paper/search?query=${query}&limit=${max}&fields=title,authors,year,citationCount,externalIds"

    local auth=""
    [ -n "${S2_KEY}" ] && auth="-H 'x-api-key: ${S2_KEY}'"

    curl -s --max-time 15 ${auth} "${url}" > "/tmp/s2_$$.json" 2>/dev/null || return 1

    python3 -c "
import json
try:
    data = json.load(open('/tmp/s2_$$.json'))
    for p in data.get('data', []):
        authors = [a.get('name','') for a in p.get('authors',[])]
        cats = ['cs.AI']
        print(json.dumps({'id':p.get('paperId',''),'title':p.get('title',''),'authors':authors,'abstract':'','categories':cats,'year':p.get('year',0),'arxiv_id':p.get('externalIds',{}).get('ArXiv',''),'citation_count':p.get('citationCount',0)}, ensure_ascii=False))
except: pass
" 2>/dev/null >> "${outfile}"
    rm -f "/tmp/s2_$$.json"
}

# ─── OpenAlex API call (fast, 100K req/day free, broadest coverage) ────────
fetch_openalex() {
    local query="$1"
    local max="$2"
    local outfile="$3"
    local url="https://api.openalex.org/works?filter=title.search:${query}&per-page=${max}&sort=cited_by_count:desc"

    curl -s --max-time 15 "${url}" > "/tmp/oa_$$.json" 2>/dev/null || return 1

    python3 -c "
import json
try:
    data = json.load(open('/tmp/oa_$$.json'))
    for r in data.get('results', []):
        authors = [a.get('author',{}).get('display_name','') for a in r.get('authorships',[])]
        cats_raw = r.get('concepts',[])
        cats = [c.get('display_name','cs.AI') for c in cats_raw[:3]]
        print(json.dumps({'id':r.get('id','').split('/')[-1],'title':r.get('title',''),'authors':authors,'abstract':(r.get('abstract_inverted_index') and ' '.join(r['abstract_inverted_index'].keys())[:2000] or ''),'categories':cats,'year':r.get('publication_year',0),'arxiv_id':r.get('id','').split('/')[-1],'citation_count':r.get('cited_by_count',0)}, ensure_ascii=False))
except: pass
" 2>/dev/null >> "${outfile}"
    rm -f "/tmp/oa_$$.json"
}

export -f fetch_arxiv fetch_s2 fetch_openalex 2>/dev/null || true

# ─── Main recursive loop ──────────────────────────────────────────────────
QUERIES=("${SEEDS[@]}")

while [ "${CURRENT}" -lt "${MAX_PAPERS}" ]; do
    echo ""
    echo "=== Iteration ${ITERATION} ==="
    echo "Current: ${CURRENT} / ${MAX_PAPERS} papers"
    echo "Seeds: ${#QUERIES[@]} queries"

    ITER_DIR="${OUTPUT_DIR}/iterations/iter_${ITERATION}"
    mkdir -p "${ITER_DIR}"
    ITER_FILE="${ITER_DIR}/papers.jsonl"
    : > "${ITER_FILE}"

    # Fan-out: split queries into parallel batches.
    # arXiv queries (slower, need delay between each).
    ARXIV_QUERIES=()
    S2_QUERIES=()
    OA_QUERIES=()

    for q in "${QUERIES[@]}"; do
        case $(( ${#ARXIV_QUERIES[@]} % 3 )) in
            0) ARXIV_QUERIES+=("$q") ;;
            1) S2_QUERIES+=("$q") ;;
            2) OA_QUERIES+=("$q") ;;
        esac
    done

    # OpenAlex is fastest — do in parallel with S2.
    echo "  OA: ${#OA_QUERIES[@]} parallel queries..."
    if [ ${#OA_QUERIES[@]} -gt 0 ]; then
        printf '%s\n' "${OA_QUERIES[@]}" | xargs -P "${JOBS}" -I{} bash -c '
            fetch_openalex "{}" 500 "'"${ITER_FILE}"'" 2>/dev/null
        ' 2>/dev/null || true
    fi

    # Semantic Scholar — parallel.
    echo "  S2: ${#S2_QUERIES[@]} parallel queries..."
    if [ ${#S2_QUERIES[@]} -gt 0 ]; then
        printf '%s\n' "${S2_QUERIES[@]}" | xargs -P "${JOBS}" -I{} bash -c '
            fetch_s2 "{}" 500 "'"${ITER_FILE}"'" 2>/dev/null
        ' 2>/dev/null || true
    fi

    # arXiv is slower — do in batches of 5 parallel to speed up.
    echo "  arXiv: ${#ARXIV_QUERIES[@]} queries (batch-5 parallel)..."
    BATCH=5
    for ((j=0; j<${#ARXIV_QUERIES[@]}; j+=BATCH)); do
        BATCH_QS=("${ARXIV_QUERIES[@]:j:BATCH}")
        printf '%s\n' "${BATCH_QS[@]}" | xargs -P 5 -I{} bash -c '
            fetch_arxiv "{}" 0 500 "'"${ITER_FILE}"'" 2>/dev/null
        ' 2>/dev/null || true
        sleep "${ARXIV_DELAY}"
    done

    # Count papers in this iteration.
    ITER_COUNT=$(wc -l < "${ITER_FILE}" 2>/dev/null || echo 0)
    echo "  Fetched: ${ITER_COUNT} papers"

    # Merge with global collection.
    cat "${ITER_FILE}" >> "${OUTPUT_DIR}/all_papers.jsonl"
    CURRENT=$(wc -l < "${OUTPUT_DIR}/all_papers.jsonl" 2>/dev/null || echo 0)
    echo "  Total: ${CURRENT} / ${MAX_PAPERS}"

    # Run pattern extraction.
    if [ "${CURRENT}" -gt 0 ]; then
        echo "  Extracting patterns..."
        PAT_FILE="${OUTPUT_DIR}/patterns/patterns_${ITERATION}.jsonl"
        "${RESEARCH_BIN}" "${OUTPUT_DIR}/all_papers.jsonl" --output "${PAT_FILE}" 2>/dev/null || {
            echo "  WARNING: pattern extraction failed, continuing..."
        }

        # Extract top pattern names from output for next iteration seeds.
        NEXT_QUERIES=()
        if [ -f "${PAT_FILE}" ]; then
            while IFS= read -r line; do
                if echo "${line}" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('pattern',''))" 2>/dev/null | grep -q .; then
                    NAME=$(echo "${line}" | python3 -c "import json,sys; print(json.load(sys.stdin).get('pattern',''))" 2>/dev/null)
                    CONF=$(echo "${line}" | python3 -c "import json,sys; print(json.load(sys.stdin).get('confidence',0))" 2>/dev/null)
                    if [ -n "${NAME}" ]; then
                        NAME_ENCODED=$(echo "${NAME}" | sed 's/ /+/g')
                        NEXT_QUERIES+=("${NAME_ENCODED}")
                    fi
                fi
            done < "${PAT_FILE}"
        fi

        # Combine with previous high-confidence patterns, deduplicate, take top 40.
        ALL_QUERIES=("${NEXT_QUERIES[@]}")
        # Add year-specific diversity queries for high-confidence patterns.
        for q in "${NEXT_QUERIES[@]}"; do
            for year in 2020 2021 2022 2023 2024; do
                ALL_QUERIES+=("${q}+${year}")
            done
            # Add category-specific variants.
            for cat in "cs.AI" "cs.LG" "cs.CL" "cs.CV" "stat.ML"; do
                ALL_QUERIES+=("${q}+cat:${cat}")
            done
        done
        # Add some original seeds to maintain diversity.
        for s in "${SEEDS[@]}"; do
            ALL_QUERIES+=("$s")
        done
        # Deduplicate and truncate to 60 (was 40, increased for diversity).
        QUERIES=($(printf '%s\n' "${ALL_QUERIES[@]}" | sort -u | tail -60))

        # If no new patterns, fall back to seed expansion.
        if [ ${#NEXT_QUERIES[@]} -eq 0 ]; then
            echo "  No new patterns found, expanding seeds..."
            QUERIES=("${SEEDS[@]}")
            for q in "${!QUERIES[@]}"; do
                QUERIES[q]="${QUERIES[q]}+survey"
            done
        fi
    fi

    ITERATION=$((ITERATION + 1))

    # Safety: max 40 iterations.
    if [ "${ITERATION}" -ge 40 ]; then
        echo "Max iterations reached. Stopping."
        break
    fi
done

echo ""
echo "================================================"
echo "Extraction complete!"
echo "  Total papers: ${CURRENT}"
echo "  Iterations:   ${ITERATION}"
echo "  Output:       ${OUTPUT_DIR}/all_papers.jsonl"
echo "================================================"
echo ""
echo "Running final pattern analysis..."
"${RESEARCH_BIN}" "${OUTPUT_DIR}/all_papers.jsonl" --output "${OUTPUT_DIR}/final_patterns.jsonl" 2>/dev/null || true
echo "Final patterns: ${OUTPUT_DIR}/final_patterns.jsonl"
echo ""
echo "Stats:"
wc -l "${OUTPUT_DIR}/all_papers.jsonl" 2>/dev/null || echo "0 papers"
wc -l "${OUTPUT_DIR}/final_patterns.jsonl" 2>/dev/null || echo "0 patterns"

# ─── HFT / Trading / Self-Sovereign — expert seeds ─────────────────────────
HFT_SEEDS=(
    "high+frequency+trading+algorithm"
    "market+microstructure+latency"
    "self-sovereign+infrastructure"
    "decentralized+exchange+arbitrage"
    "intent-based+protocol+solver"
    "MEV+extraction+protection"
    "smart+contract+escrow+atomic"
    "AMM+liquidity+concentrated"
    "flash+loans+arbitrage"
    "cross-chain+atomic+swap"
    "zero-knowledge+proof+trading"
    "private+mempool+validator"
    "order+flow+auction"
    "Rust+trading+engine+HFT"
    "agent+autonomous+trading"
    "P2P+state+channel+payment"
    "self-custody+key+management"
    "decentralized+oracle+price+feed"
    "programmable+escrow+smart+contract"
    "DeFi+agent+framework"
    "blockchain+latency+optimization"
    "direct+validator+relay"
    "trustless+cross+chain+bridge"
    "quantitative+trading+strategy+crypto"
    "order+book+depth+simulation"
)
