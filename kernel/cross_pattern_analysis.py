#!/usr/bin/env python3
"""Cross-pattern analysis of the prompt enrichment database (optimized)."""

import json
import math
import collections
import itertools
import re

# ─── Configuration ───────────────────────────────────────────────────────────

STOPWORDS = {
    'the', 'a', 'an', 'is', 'are', 'was', 'were', 'be', 'been', 'being',
    'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'could',
    'should', 'may', 'might', 'can', 'shall', 'to', 'of', 'in', 'for',
    'on', 'with', 'at', 'by', 'from', 'as', 'into', 'through', 'during',
    'before', 'after', 'above', 'below', 'between', 'and', 'or', 'not',
    'but', 'if', 'then', 'else', 'when', 'where', 'why', 'how', 'all',
    'each', 'every', 'both', 'few', 'more', 'most', 'other', 'some',
    'such', 'only', 'own', 'same', 'so', 'than', 'too', 'very', 'just',
    'that', 'this', 'these', 'those', 'it', 'its', 'he', 'she', 'they',
    'them', 'their', 'we', 'you', 'i', 'my', 'me', 'our', 'your',
    'no', 'nor', 'up', 'off', 'out', 'about', 'over', 'under', 'again',
    'further', 'once', 'here', 'there', 'which', 'who', 'whom', 'what',
    'down', 'any', 'll', 've', 're'
}

DB_PATH = '/root/dowiz/kernel/prompt_enrich_db.jsonl'
GAP_THRESHOLD = 0.05
MIN_KEYWORD_LEN = 3
MIN_DOMAIN_ENTRIES = 3       # only consider domains with >= this many entries for pair analysis
TOP_DOMAINS = 400            # max number of domains for pairwise analysis (correlation, bridges)

# ─── Data loading ────────────────────────────────────────────────────────────

def load_entries(path):
    entries = []
    with open(path, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            entries.append(json.loads(line))
    return entries

def extract_domain(entry):
    """Extract domain from entry. Two formats."""
    if 'domain' in entry:
        return entry['domain']
    title = entry.get('title', '')
    if '/' in title:
        return title.split('/')[0]
    return title

def get_text(entry):
    if 'keywords' in entry:
        kw = ' '.join(entry.get('keywords', []))
        pat = ' '.join(entry.get('pattern', []))
        return kw + ' ' + pat
    return entry.get('text', '') + ' ' + entry.get('title', '')

def tokenize(text):
    tokens = re.findall(r'[a-zA-Z][a-zA-Z0-9_\-]*', text.lower())
    return [t for t in tokens if t not in STOPWORDS and len(t) >= MIN_KEYWORD_LEN]

# ─── Matrix helpers ──────────────────────────────────────────────────────────

def build_domain_kind_matrix(entries):
    domain_to_idx = {}
    kind_to_idx = {}
    idx_to_domain = []
    idx_to_kind = []

    for e in entries:
        d = extract_domain(e)
        k = e['kind']
        if d not in domain_to_idx:
            domain_to_idx[d] = len(idx_to_domain)
            idx_to_domain.append(d)
        if k not in kind_to_idx:
            kind_to_idx[k] = len(idx_to_kind)
            idx_to_kind.append(k)

    n_d = len(idx_to_domain)
    n_k = len(idx_to_kind)
    matrix = [[0] * n_k for _ in range(n_d)]
    domain_entry_counts = [0] * n_d

    for e in entries:
        d = extract_domain(e)
        k = e['kind']
        di = domain_to_idx[d]
        ki = kind_to_idx[k]
        matrix[di][ki] += 1
        domain_entry_counts[di] += 1

    return idx_to_domain, idx_to_kind, matrix, domain_entry_counts

def build_keyword_vectors_for_domains(entries, target_domains):
    """Build keyword vectors only for the specified domains.
    Returns: dict domain -> dict keyword_idx -> count
    """
    domain_kw = collections.defaultdict(collections.Counter)
    kw_to_idx = {}
    idx_to_kw = []

    per_entry = []
    for e in entries:
        d = extract_domain(e)
        per_entry.append((d, tokenize(get_text(e))))

    for d, kws in per_entry:
        if d not in target_domains:
            continue
        for w in set(kws):
            if w not in kw_to_idx:
                kw_to_idx[w] = len(idx_to_kw)
                idx_to_kw.append(w)
            domain_kw[d][w] += 1

    return domain_kw, kw_to_idx, idx_to_kw

# ─── Analysis functions ──────────────────────────────────────────────────────

def cosine_similarity_dict(v1, v2, all_keys):
    """Cosine similarity between two Counter vectors."""
    dot = sum(v1.get(k, 0) * v2.get(k, 0) for k in all_keys)
    norm1 = math.sqrt(sum(v * v for v in v1.values()))
    norm2 = math.sqrt(sum(v * v for v in v2.values()))
    if norm1 == 0 or norm2 == 0:
        return 0.0
    return dot / (norm1 * norm2)


def compute_domain_correlation(domains, domain_kw, kw_to_idx):
    """Compute domain correlation for a list of domains."""
    n = len(domains)
    all_kw = set(kw_to_idx.keys())
    corr = {}
    for i in range(n):
        di = domains[i]
        for j in range(i+1, n):
            dj = domains[j]
            sim = cosine_similarity_dict(domain_kw[di], domain_kw[dj], all_kw)
            if sim > 0:
                corr[(di, dj)] = sim
    return corr


def compute_pattern_gap_map(idx_to_domain, idx_to_kind, domain_kind_matrix):
    n_d = len(idx_to_domain)
    n_k = len(idx_to_kind)
    kind_totals = [sum(domain_kind_matrix[d][k] for d in range(n_d)) for k in range(n_k)]
    total_entries = sum(kind_totals)

    gaps = []
    for ki, kind in enumerate(idx_to_kind):
        domains_with = sum(1 for d in range(n_d) if domain_kind_matrix[d][ki] > 0)
        frac_domains = domains_with / n_d if n_d > 0 else 0
        frac_entries = kind_totals[ki] / total_entries if total_entries > 0 else 0

        if frac_domains < GAP_THRESHOLD or frac_entries < GAP_THRESHOLD:
            gaps.append((kind, frac_domains, frac_entries, domains_with, kind_totals[ki]))
    gaps.sort(key=lambda x: x[1])
    return gaps


def compute_keyword_centrality(idx_to_domain, entries):
    """Keyword centrality using streaming approach — track per-domain keyword presence."""
    n_d = len(idx_to_domain)
    domain_set = set(idx_to_domain)
    kw_domains = collections.defaultdict(set)
    kw_total = collections.Counter()

    for e in entries:
        d = extract_domain(e)
        if d not in domain_set:
            continue
        for w in set(tokenize(get_text(e))):
            kw_domains[w].add(d)
            kw_total[w] += 1

    centrality = []
    for kw, doms in kw_domains.items():
        nd = len(doms)
        if nd >= 3:
            centrality.append((kw, nd, kw_total[kw], nd / n_d if n_d > 0 else 0))

    centrality.sort(key=lambda x: (x[1], x[2]), reverse=True)
    return centrality


def compute_density_scores(idx_to_domain, idx_to_kind, domain_kind_matrix, domain_entry_counts):
    n_d = len(idx_to_domain)
    n_k = len(idx_to_kind)

    kind_maxes = [max(domain_kind_matrix[d][k] for d in range(n_d)) for k in range(n_k)]

    scores = []
    for di, domain in enumerate(idx_to_domain):
        vals = []
        for ki in range(n_k):
            mx = kind_maxes[ki]
            vals.append(domain_kind_matrix[di][ki] / mx if mx > 0 else 0)
        avg = sum(vals) / n_k if n_k > 0 else 0
        density = round(avg * 9 + 1, 1)
        non_zero = sum(1 for ki in range(n_k) if domain_kind_matrix[di][ki] > 0)
        scores.append((domain, density, non_zero, n_k, domain_entry_counts[di]))
    scores.sort(key=lambda x: x[1])
    return scores


def compute_bridge_candidates(idx_to_domain, domain_to_idx, kind_vecs, domain_kw, kw_to_idx,
                               target_domains):
    """Bridge candidates among top domains."""
    n = len(target_domains)
    all_kw = set(kw_to_idx.keys())
    candidates = []

    for idx_i in range(n):
        di = target_domains[idx_i]
        i = domain_to_idx[di]
        for idx_j in range(idx_i+1, n):
            dj = target_domains[idx_j]
            j = domain_to_idx[dj]

            # Keyword overlap
            kw_sim = cosine_similarity_dict(domain_kw[di], domain_kw[dj], all_kw)

            # Kind overlap (cosine on binary presence)
            vi = kind_vecs[i]
            vj = kind_vecs[j]
            dot = sum(vi[k] * vj[k] for k in range(len(vi)))
            n1 = math.sqrt(sum(x*x for x in vi))
            n2 = math.sqrt(sum(x*x for x in vj))
            k_sim = dot / (n1 * n2) if n1 > 0 and n2 > 0 else 0

            bridge = kw_sim * (1 - k_sim)
            if bridge > 0.001:
                candidates.append((di, dj, round(kw_sim, 4), round(k_sim, 4), round(bridge, 4)))

    candidates.sort(key=lambda x: x[4], reverse=True)
    return candidates


# ─── Eigen approximation via power iteration ─────────────────────────────────

def power_iteration(matrix, num_components=5, max_iter=200, tol=1e-10):
    n = len(matrix)
    if n == 0:
        return [], []

    eigenvectors = []
    eigenvalues = []
    residual = [row[:] for row in matrix]

    for comp in range(min(num_components, n)):
        b = [1.0 / math.sqrt(n)] * n
        seed = comp * 137 + 73
        for i in range(n):
            seed = (seed * 1103515245 + 12345) & 0x7fffffff
            b[i] += seed / 0x7fffffff * 0.001
        norm = math.sqrt(sum(x * x for x in b))
        b = [x / norm for x in b]

        for _ in range(max_iter):
            Ab = [sum(residual[i][j] * b[j] for j in range(n)) for i in range(n)]
            norm = math.sqrt(sum(x * x for x in Ab))
            if norm < tol:
                break
            b_next = [x / norm for x in Ab]
            diff = max(abs(b_next[i] - b[i]) for i in range(n))
            b = b_next
            if diff < tol:
                break

        eigenvalue = sum(b[i] * sum(residual[i][j] * b[j]
                         for j in range(n)) for i in range(n))
        eigenvalues.append(eigenvalue)
        eigenvectors.append(b[:])

        for i in range(n):
            for j in range(n):
                residual[i][j] -= eigenvalue * b[i] * b[j]

    return eigenvalues, eigenvectors


def compute_kind_pca(idx_to_kind, domain_kind_matrix):
    n_d = len(domain_kind_matrix)
    n_k = len(idx_to_kind)

    kind_matrix = [[0.0] * n_k for _ in range(n_k)]
    for d in range(n_d):
        row = domain_kind_matrix[d]
        present = [ki for ki in range(n_k) if row[ki] > 0]
        for idx_i, ki1 in enumerate(present):
            for ki2 in present[idx_i:]:
                kind_matrix[ki1][ki2] += 1
                if ki1 != ki2:
                    kind_matrix[ki2][ki1] += 1

    for i in range(n_k):
        for j in range(n_k):
            kind_matrix[i][j] /= n_d if n_d > 0 else 1

    eigenvalues, eigenvectors = power_iteration(kind_matrix, num_components=5)

    component_rankings = []
    for comp_idx, (eigval, eigvec) in enumerate(zip(eigenvalues, eigenvectors)):
        ranked = sorted(
            [(idx_to_kind[ki], eigvec[ki], abs(eigvec[ki]))
             for ki in range(n_k)],
            key=lambda x: x[2], reverse=True
        )
        total_abs = sum(abs(e) for e in eigenvalues)
        component_rankings.append({
            'component': comp_idx + 1,
            'eigenvalue': round(eigval, 6),
            'variance_explained': round(
                abs(eigval) / total_abs, 4) if total_abs > 0 else 0,
            'top_kinds': ranked[:10]
        })

    return eigenvalues, component_rankings


# ─── Trinary classification ──────────────────────────────────────────────────

def trinary_classification(idx_to_domain, idx_to_kind, domain_kind_matrix,
                            domain_entry_counts):
    n_d = len(idx_to_domain)
    n_k = len(idx_to_kind)

    metrics = []
    for di in range(n_d):
        row = domain_kind_matrix[di]
        non_zero = sum(1 for ki in range(n_k) if row[ki] > 0)
        cov = non_zero / n_k if n_k > 0 else 0
        metrics.append({
            'domain': idx_to_domain[di],
            'entries': domain_entry_counts[di],
            'kinds_repr': non_zero,
            'total_kinds': n_k,
            'coverage': cov,
        })

    # Z-scores on coverage and entry count
    covs = [m['coverage'] for m in metrics]
    ents = [m['entries'] for m in metrics]

    avg_cov = sum(covs) / len(covs) if covs else 0
    avg_ent = sum(ents) / len(ents) if ents else 0

    var_cov = sum((c - avg_cov)**2 for c in covs) / len(covs) if covs else 0
    var_ent = sum((e - avg_ent)**2 for e in ents) / len(ents) if ents else 0
    std_cov = math.sqrt(var_cov)
    std_ent = math.sqrt(var_ent)

    results = []
    for m in metrics:
        zc = (m['coverage'] - avg_cov) / std_cov if std_cov > 0 else 0
        ze = (m['entries'] - avg_ent) / std_ent if std_ent > 0 else 0
        combined = zc * 0.6 + ze * 0.4

        if combined > 0.5:
            tri = 'True'
        elif combined < -0.5:
            tri = 'False'
        else:
            tri = 'Unknown'

        results.append({**m, 'tri': tri, 'z_combined': round(combined, 3)})

    results.sort(key=lambda x: (x['tri'] != 'False', x['tri'] != 'Unknown',
                                 x['coverage']))
    return results


# ─── Main ────────────────────────────────────────────────────────────────────

def main():
    print("=" * 72)
    print("  CROSS-PATTERN ANALYSIS OF PROMPT ENRICHMENT DATABASE")
    print("=" * 72)

    # ── 1. Load ────────────────────────────────────────────────────────────
    print("\n[1] Loading entries...", flush=True)
    entries = load_entries(DB_PATH)
    print(f"    Loaded {len(entries)} entries", flush=True)

    # ── 2. Build domain×kind matrix ───────────────────────────────────────
    print("\n[2] Building domain × kind matrix...", flush=True)
    idx_to_domain, idx_to_kind, dk_matrix, domain_entry_counts = \
        build_domain_kind_matrix(entries)
    domain_to_idx = {d: i for i, d in enumerate(idx_to_domain)}
    n_d = len(idx_to_domain)
    n_k = len(idx_to_kind)
    print(f"    Domains: {n_d}  |  Kinds: {n_k}", flush=True)

    # Identify top domains for pairwise analysis
    top_domains = sorted(idx_to_domain,
                         key=lambda d: domain_entry_counts[domain_to_idx[d]],
                         reverse=True)[:TOP_DOMAINS]
    top_domains = [d for d in top_domains
                   if domain_entry_counts[domain_to_idx[d]] >= MIN_DOMAIN_ENTRIES]
    print(f"    Top domains for pairwise analysis: {len(top_domains)} "
          f"(≥{MIN_DOMAIN_ENTRIES} entries each)", flush=True)

    # ── 3. Build keyword vectors for top domains ──────────────────────────
    print("\n[3] Building keyword vectors...", flush=True)
    domain_kw, kw_to_idx, idx_to_kw = build_keyword_vectors_for_domains(
        entries, set(top_domains))
    n_kw = len(idx_to_kw)
    print(f"    Keywords in top domains: {n_kw}", flush=True)

    # ── 4. Keyword centrality (all domains) ────────────────────────────────
    print("\n[4] Computing keyword centrality (streaming)...", flush=True)
    centrality = compute_keyword_centrality(idx_to_domain, entries)
    print(f"    Cross-cutting keywords (≥3 domains): {len(centrality)}", flush=True)

    # ── (a) Domain correlation ────────────────────────────────────────────
    print("\n[5] Computing domain correlation matrix...", flush=True)
    domain_corr = compute_domain_correlation(top_domains, domain_kw, kw_to_idx)
    similar_pairs = sorted(domain_corr.items(), key=lambda x: x[1], reverse=True)
    print(f"    Non-zero correlations found: {len(similar_pairs)}", flush=True)

    print("\n" + "─" * 72)
    print("  (a) TOP 10 MOST SIMILAR DOMAINS (keyword overlap)")
    print("─" * 72)
    for (d1, d2), sim in similar_pairs[:10]:
        print(f"  {d1:30s} ←→ {d2:30s}  cos={sim:.4f}")

    # ── (b) Pattern gap map ────────────────────────────────────────────────
    print("\n" + "─" * 72)
    print("  (b) PATTERN GAP MAP — kinds with <5% domain representation")
    print("─" * 72)
    gaps = compute_pattern_gap_map(idx_to_domain, idx_to_kind, dk_matrix)
    if gaps:
        print(f"  {'Kind':20s} {'Frac.Dom':>10s} {'Frac.Entries':>12s} "
              f"{'#Domains':>10s} {'Total':>7s}")
        print(f"  {'─'*20} {'─'*10} {'─'*12} {'─'*10} {'─'*7}")
        for k, fd, fe, nd, tot in gaps:
            print(f"  {str(k):20s} {fd:10.4f}  {fe:12.4f}  {nd:10d}  {tot:7d}")
    else:
        print("  No gap kinds found!")

    # ── (c) Keyword centrality ─────────────────────────────────────────────
    print("\n" + "─" * 72)
    print("  (c) TOP 10 CROSS-CUTTING KEYWORDS (span most domains)")
    print("─" * 72)
    print(f"  {'Keyword':25s} {'#Domains':>10s} {'TotalFreq':>10s} {'Frac.Dom':>10s}")
    print(f"  {'─'*25} {'─'*10} {'─'*10} {'─'*10}")
    for kw, nd, tot, frac in centrality[:10]:
        print(f"  {kw:25s} {nd:10d}  {tot:10d}  {frac:10.4f}")

    # ── (d) Density scores ─────────────────────────────────────────────────
    print("\n" + "─" * 72)
    print("  (d) DENSITY SCORES PER DOMAIN (1-10)")
    print("─" * 72)
    density = compute_density_scores(idx_to_domain, idx_to_kind, dk_matrix,
                                      domain_entry_counts)
    print(f"  {'Domain':32s} {'Density':>7s}  {'Kinds':>6s}  {'Entries':>8s}")
    print(f"  {'─'*32} {'─'*7}  {'─'*6}  {'─'*8}")
    print("  --- LOWEST DENSITY (potential gaps) ---")
    for d, sc, nk, tk, te in density[:10]:
        print(f"  {d:32s} {sc:7.1f}  {nk:6d}/{tk}  {te:8d}")
    print("  --- HIGHEST DENSITY ---")
    for d, sc, nk, tk, te in density[-10:][::-1]:
        print(f"  {d:32s} {sc:7.1f}  {nk:6d}/{tk}  {te:8d}")

    # ── (e) Bridge candidates ──────────────────────────────────────────────
    print("\n[6] Computing bridge candidates...", flush=True)
    kind_vecs = []
    for di in range(n_d):
        vec = [1 if dk_matrix[di][ki] > 0 else 0 for ki in range(n_k)]
        kind_vecs.append(vec)

    bridges = compute_bridge_candidates(idx_to_domain, domain_to_idx, kind_vecs,
                                         domain_kw, kw_to_idx, top_domains)
    print(f"    Bridge candidates found: {len(bridges)}", flush=True)

    print("\n" + "─" * 72)
    print("  (e) TOP 5 BRIDGE CANDIDATES")
    print("      (high keyword overlap × low kind overlap)")
    print("─" * 72)
    print(f"  {'Domain A':28s} {'Domain B':28s} {'KW-Sim':>8s} "
          f"{'Kind-Sim':>9s} {'Bridge':>8s}")
    print(f"  {'─'*28} {'─'*28} {'─'*8} {'─'*9} {'─'*8}")
    for i, (da, db, kw_sim, k_sim, br) in enumerate(bridges[:10]):
        marker = " ◀ TOP-5" if i < 5 else ""
        print(f"  {da:28s} {db:28s} {kw_sim:8.4f} {k_sim:9.4f} {br:8.4f}{marker}")

    # ── Top gap domains ────────────────────────────────────────────────────
    print("\n" + "─" * 72)
    print("  TOP 5 GAP DOMAINS REQUIRING IMMEDIATE ATTENTION")
    print("  (low density + few kinds + non-trivial entries)")
    print("─" * 72)
    # Filter to domains with >1 entry to avoid 1-entry noise
    gap_candidates = []
    for d, sc, nk, tk, te in density:
        if te <= 1:
            continue
        cov = nk / tk if tk > 0 else 0
        gap_score = (10 - sc) * (1 - cov) * max(te, 1)
        gap_candidates.append((d, sc, nk, tk, te, cov, gap_score))
    gap_candidates.sort(key=lambda x: x[6], reverse=True)

    print(f"  {'Domain':32s} {'Density':>7s}  {'Kinds':>6s}  {'Entries':>8s}  {'Cov%':>6s}")
    print(f"  {'─'*32} {'─'*7}  {'─'*6}  {'─'*8}  {'─'*6}")
    for i, (d, sc, nk, tk, te, cov, _) in enumerate(gap_candidates[:10]):
        marker = " ◀ TOP-5" if i < 5 else ""
        print(f"  {d:32s} {sc:7.1f}  {nk:6d}/{tk}  {te:8d}  {cov:6.2%}{marker}")

    # ── (f) Kind PCA ───────────────────────────────────────────────────────
    print("\n" + "─" * 72)
    print("  (f) PRINCIPAL COMPONENT RANKING OF ENRICHMENT DIMENSIONS")
    print("      (eigen decomposition of kind×kind co-occurrence matrix)")
    print("─" * 72)
    eigenvalues, component_rankings = compute_kind_pca(idx_to_kind, dk_matrix)
    for cr in component_rankings:
        print(f"\n  PC{cr['component']}: eigenvalue={cr['eigenvalue']:.4f}, "
              f"var.expl.={cr['variance_explained']:.2%}")
        print(f"  {'Kind':20s} {'Loading':>10s}")
        print(f"  {'─'*20} {'─'*10}")
        for kind, loading, abs_load in cr['top_kinds'][:5]:
            print(f"  {str(kind):20s} {loading:10.4f}")

    # ── (g) Trinary classification ─────────────────────────────────────────
    print("\n" + "─" * 72)
    print("  (g) TRINARY CLASSIFICATION TABLE")
    print("      True=well-covered  False=gap  Unknown=ambiguous")
    print("─" * 72)
    tri_results = trinary_classification(idx_to_domain, idx_to_kind,
                                          dk_matrix, domain_entry_counts)
    tri_counts = collections.Counter(r['tri'] for r in tri_results)
    print(f"\n  Summary: True={tri_counts.get('True', 0)}, "
          f"False={tri_counts.get('False', 0)}, "
          f"Unknown={tri_counts.get('Unknown', 0)}  "
          f"(of {len(tri_results)} domains)")

    print(f"\n  {'Domain':32s} {'Tri':>8s}  {'Coverage':>9s}  "
          f"{'Kinds':>6s}  {'Entries':>8s}  {'Z-comb':>7s}")
    print(f"  {'─'*32} {'─'*8}  {'─'*9}  {'─'*6}  {'─'*8}  {'─'*7}")

    for r in tri_results:
        mk = ""
        if r['tri'] == 'False':
            mk = " ◀ GAP"
        elif r['tri'] == 'Unknown':
            mk = " ◀ AMBIGUOUS"
        print(f"  {r['domain']:32s} {r['tri']:>8s}  {r['coverage']:9.3f}  "
              f"{r['kinds_repr']:6d}/{r['total_kinds']}  {r['entries']:8d}  "
              f"{r['z_combined']:7.3f}{mk}")

    # ── Final summary ──────────────────────────────────────────────────────
    print("\n" + "=" * 72)
    print("  KEY DELIVERABLES SUMMARY")
    print("=" * 72)

    print("\n  TOP 10 CROSS-CUTTING KEYWORDS:")
    for kw, nd, tot, frac in centrality[:10]:
        print(f"    {kw:25s}  spans {nd:4d} domains ({frac:.1%})")

    print("\n  TOP 5 GAP DOMAINS (immediate attention):")
    for d, sc, nk, tk, te, cov, _ in gap_candidates[:5]:
        print(f"    {d:32s}  density={sc:.1f}  kinds={nk}/{tk}  entries={te}")

    print("\n  TOP 5 BRIDGE CANDIDATES:")
    for i, (da, db, kw_sim, k_sim, br) in enumerate(bridges[:5]):
        print(f"    {da:28s} ←→ {db:28s}  bridge={br:.4f}")
        print(f"      keyword-sim={kw_sim:.4f}  kind-sim={k_sim:.4f}")

    print("\n  PRINCIPAL COMPONENT RANKING:")
    for cr in component_rankings:
        top_names = [str(k) for k, _, _ in cr['top_kinds'][:5]]
        print(f"    PC{cr['component']} (λ={cr['eigenvalue']:.4f}, "
              f"var={cr['variance_explained']:.2%}): "
              f"{' + '.join(top_names)}")

    print("\n  TRINARY CLASSIFICATION COUNTS:")
    for label in ['True', 'False', 'Unknown']:
        n_lab = tri_counts.get(label, 0)
        pct = n_lab / len(tri_results) * 100 if tri_results else 0
        print(f"    {label:8s}: {n_lab:5d} ({pct:5.1f}%)")

    print("\n" + "=" * 72)
    print("  ANALYSIS COMPLETE")
    print("=" * 72)


if __name__ == '__main__':
    main()
