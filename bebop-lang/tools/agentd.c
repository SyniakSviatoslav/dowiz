/* agentd.c — HOT-PATH agent daemon for internal workflow.
 * Resident process: living-memory graph stays loaded (no cold parse),
 * navigation indexes cached by mtime, gates/probes run in a thread pool,
 * usage counters persist -> evolution data for which commands deserve to
 * exist. Line protocol on stdin/stdout; `par N` fans out N jobs.
 *
 * Build: gcc -O2 -Wall -Wextra -pthread -o native/build/agentd tools/agentd.c
 * Protocol: one command per line; replies prefixed ok:/err:/>> ; quit ends.
 */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>
#include <pthread.h>
#include <sys/stat.h>
#include <unistd.h>
#include <stdarg.h>
#include <sys/wait.h>

#include "../native/src/lmem.h"

static LmGraph G;
static pthread_mutex_t g_mu = PTHREAD_MUTEX_INITIALIZER;
static const char *MEM_PATH = "docs/memory.lmem";
static const char *STAT_PATH = "docs/memory.stats";

/* ── usage counters (evolution) ─────────────────────────────────────── */
typedef struct { char name[48]; unsigned long n; } Stat;
static Stat ST[128]; static int NST;
static pthread_mutex_t st_mu = PTHREAD_MUTEX_INITIALIZER;

static void stat_bump(const char *cmd) {
    pthread_mutex_lock(&st_mu);
    for (int i = 0; i < NST; i++)
        if (!strcmp(ST[i].name, cmd)) { ST[i].n++; goto out; }
    if (NST < 128) { snprintf(ST[NST].name, 48, "%s", cmd); ST[NST].n = 1; NST++; }
out:
    pthread_mutex_unlock(&st_mu);
}
static void stat_load(void) {
    FILE *f = fopen(STAT_PATH, "r");
    if (!f) return;
    char nm[48]; unsigned long v;
    while (NST < 128 && fscanf(f, "%47s %lu", nm, &v) == 2)
        { snprintf(ST[NST].name, 48, "%s", nm); ST[NST].n = v; NST++; }
    fclose(f);
}
static void stat_save(void) {
    FILE *f = fopen(STAT_PATH, "w");
    if (!f) return;
    for (int i = 0; i < NST; i++) fprintf(f, "%s %lu\n", ST[i].name, ST[i].n);
    fclose(f);
}

/* ── hot navigation cache (path,mtime -> fn index) ──────────────────── */
typedef struct { char path[256]; long mtime; int n; char (*names)[64]; } Nav;
static Nav NAV[16]; static int NNAV;
static pthread_mutex_t nav_mu = PTHREAD_MUTEX_INITIALIZER;

/* exact mirror of selfhost collect_fns (lowercase-alpha starts only) */
static int scan_fns(const char *s, long n, char names[][64], int max) {
    long j = 0; int cnt = 0;
    while (j + 2 < n) {
        unsigned char c0 = (unsigned char)s[j], c1 = (unsigned char)s[j+1], c2 = (unsigned char)s[j+2];
        int is_quote = c0 == 34;
        int is_comment = c0 == 47 && c1 == 47;
        unsigned char cafter = j + 3 < n ? (unsigned char)s[j+3] : 0;
        int is_fn = c0==102 && c1==110 && c2==32 && cafter>=97 && cafter<=122;
        if (is_fn) {
            long k = j + 3;
            while (k < n && (((s[k]|32)>='a'&&(s[k]|32)<='z') || (s[k]>='0'&&s[k]<='9') || s[k]=='_')) k++;
            if (cnt < max) {
                size_t L = (size_t)(k-(j+3)); if (L > 63) L = 63;
                memcpy(names[cnt], s+j+3, L); names[cnt][L] = 0;
            }
            cnt++;
            j = k;
            continue;
        }
        if (is_quote) { j++; while (j < n && s[j] != 34) j++; j++; continue; }
        if (is_comment) { while (j < n && s[j] != 10) j++; continue; }
        j++;
    }
    return cnt;
}

static Nav *nav_get(const char *path) {
    struct stat st;
    if (stat(path, &st) != 0) return NULL;
    for (int i = 0; i < NNAV; i++)
        if (!strcmp(NAV[i].path, path)) {
            if (NAV[i].mtime == (long)st.st_mtime) return &NAV[i];
            free(NAV[i].names);           /* stale — reload (hot invalidate) */
            memmove(&NAV[i], &NAV[i+1], sizeof(Nav)*(NNAV-i-1)); NNAV--;
            break;
        }
    if (NNAV >= 16) { free(NAV[0].names); memmove(NAV, NAV+1, sizeof(Nav)*15); NNAV--; }
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END); long sz = ftell(f); fseek(f, 0, SEEK_SET);
    char *src = malloc((size_t)sz + 1);
    if (!src || fread(src, 1, (size_t)sz, f) != (size_t)sz) { fclose(f); free(src); return NULL; }
    src[sz] = 0; fclose(f);
    Nav *nv = &NAV[NNAV++];
    snprintf(nv->path, 256, "%s", path);
    nv->mtime = (long)st.st_mtime;
    nv->names = malloc(512 * 64);
    nv->n = scan_fns(src, sz, nv->names, 512);
    free(src);
    return nv;
}

/* Runs one command line, writing reply lines into out (dynamic buffer). */
typedef struct { char *buf; size_t cap, len; } SB;
static void sb_put(SB *b, const char *fmt, ...) {
    va_list ap; va_start(ap, fmt);
    b->len += vsnprintf(b->buf + b->len, b->cap - b->len > 0 ? b->cap - b->len : 0, fmt, ap);
    va_end(ap);
}

/* ── hydra-borrowed adaptive controller ───────────────────────────────
 * EMA of inconclusive-ratio (entropy proxy, alpha=0.3 like hydra drift),
 * EMA of experiment seconds; PID-ish timebox gain; converged flag =
 * lyapunov-style steady state. Nudges re-enumeration when chaos high. */
typedef struct {
    double ema_inconcl;   /* 0..1 chaos of verdicts */
    double ema_secs;      /* mean experiment cost */
    double kp;            /* timebox gain */
    int    n_exp;
    FILE  *jr;
} Ctl;
static Ctl CTL = { .ema_inconcl = 0.0, .ema_secs = 30.0, .kp = 4.0, .n_exp = 0 };

static void ctl_exp(int verdict, double secs, SB *o) {
    const double A = 0.3;
    double inc = (verdict == 2) ? 1.0 : 0.0;
    CTL.ema_inconcl = A * inc + (1 - A) * CTL.ema_inconcl;
    CTL.ema_secs    = A * secs + (1 - A) * CTL.ema_secs;
    CTL.n_exp++;
    double timebox = 0.5 + CTL.kp * CTL.ema_inconcl + CTL.ema_secs / 30.0;
    if (timebox > 8) timebox = 8;
    sb_put(o, ">> exp#%d v=%d t=%.0fs | entropy=%.2f dur_ema=%.0fs timebox=x%.1f %s\n",
           CTL.n_exp, verdict, secs, CTL.ema_inconcl, CTL.ema_secs, timebox,
           CTL.ema_inconcl > 0.5 ? "!CHAOS: re-enumerate hypotheses, consult T2" :
           CTL.ema_inconcl < 0.2 ? "(converged)" : "");
}

static void cmd_exp(char *rest, SB *o) {
    /* exp <slug> <verdict 0|1|2> <secs> [note...] */
    char *slug = strtok(rest, " ");
    char *vch  = strtok(NULL, " ");
    char *sec  = strtok(NULL, " ");
    char *note = strtok(NULL, "");
    if (!slug || !vch || !sec) { sb_put(o, "err: exp <slug> <0|1|2> <secs> [note]\n"); return; }
    int v = atoi(vch);
    if (v < 0 || v > 2) { sb_put(o, "err: verdict 0=killed 1=confirmed 2=inconclusive\n"); return; }
    if (!CTL.jr) CTL.jr = fopen("docs/exp.journal", "a");
    if (CTL.jr) {
        fprintf(CTL.jr, "%lld v%d %s %s\n", (long long)time(NULL), v, slug, note ? note : "");
        fflush(CTL.jr);
    }
    /* auto-memory: confirmed wins -> pat-ok, kills -> pat-bad */
    if (v == 1 || v == 0) {
        char nm[64], nt[LMEM_NOTE_MAX];
        snprintf(nm, sizeof nm, "%s/%s", v ? "pat-ok" : "pat-bad", slug);
        snprintf(nt, sizeof nt, "%.150s", note ? note : slug);
        pthread_mutex_lock(&g_mu);
        lmem_remember(&G, nm, v ? LEM_PAT_OK : LEM_PAT_BAD, nt, (uint64_t)time(NULL));
        pthread_mutex_unlock(&g_mu);
    }
    ctl_exp(v, atof(sec), o);
}
static void cmd_ctl(char *rest, SB *o) {
    (void)rest;
    double timebox = 0.5 + CTL.kp * CTL.ema_inconcl + CTL.ema_secs / 30.0;
    if (timebox > 8) timebox = 8;
    sb_put(o, ">> ctl n=%d entropy=%.2f dur_ema=%.0fs timebox=x%.1f %s\n",
           CTL.n_exp, CTL.ema_inconcl, CTL.ema_secs, timebox,
           CTL.ema_inconcl > 0.5 ? "CHAOS" : (CTL.ema_inconcl < 0.2 ? "converged" : "searching"));
}


/* ── hot-path: one-line probe pipeline ──────────────────────────────── */
static void cmd_run(char *rest, SB *o) {
    char prog[256], comp[256];
    int k = sscanf(rest, "%255s %255s", prog, comp);
    if (k < 1) { sb_put(o, "err: run <prog.bp> [compiler.bp]\n"); return; }
    if (k == 1) snprintf(comp, sizeof comp, "selfhost/expr_compile.bp");
    char cmd[1200];
    snprintf(cmd, sizeof cmd,
        "./native/build/bebopc compilewords %s %s > /tmp/opencode/run.full 2>/dev/null", comp, prog);
    int rc = system(cmd);
    if (rc != 0) { sb_put(o, "err: compile rc=%d\n", rc >> 8); return; }
    snprintf(cmd, sizeof cmd,
        "./native/build/agent pack /tmp/opencode/run.full /tmp/opencode/run.p.bin 0 >/dev/null 2>&1");
    if (system(cmd)) {}
    snprintf(cmd, sizeof cmd,
        "timeout 300 ./seed/build/seed /tmp/opencode/run.p.bin > /tmp/opencode/run.out 2>&1");
    rc = system(cmd);
    FILE *f = fopen("/tmp/opencode/run.out", "r");
    char buf[400]; size_t rd = f ? fread(buf, 1, sizeof buf - 1, f) : 0;
    if (f) fclose(f);
    buf[rd] = 0;
    if (rd) sb_put(o, ">> %s", buf);
    sb_put(o, "ok: run exit=%d\n", rc < 0 ? 127 : WEXITSTATUS(rc));
}

/* ── hot-path: word -> function-name map (prologue scan) ────────────── */
static unsigned *FM_W; static long FM_N; static long *FM_S; static int FM_NS;
static char (*FM_NM)[64]; static int FM_NN;
static void fm_load(const char *fullpath, const char *bppath, SB *o) {
    FILE *f = fopen(fullpath, "r");
    if (!f) { sb_put(o, "err: open %s\n", fullpath); return; }
    long cnt = 0; if (fscanf(f, "%ld", &cnt) != 1) { fclose(f); return; }
    free(FM_W); FM_W = malloc((size_t)cnt * 4); FM_N = cnt;
    for (long i = 0; i < cnt; i++) { unsigned v; if (fscanf(f, "%u", &v) != 1) { FM_N = i; break; } FM_W[i] = v; }
    fclose(f);
    free(FM_S); FM_S = malloc(sizeof(long) * (size_t)(FM_N + 1)); FM_NS = 0;
    for (long i = 0; i < FM_N; i++) if (FM_W[i] == 2847898621u) FM_S[FM_NS++] = i;
    if (bppath && bppath[0]) {
        FILE *bf = fopen(bppath, "r");
        if (bf) {
            fseek(bf, 0, SEEK_END); long bl = ftell(bf); fseek(bf, 0, SEEK_SET);
            char *t = malloc((size_t)bl + 1); size_t rd = fread(t, 1, (size_t)bl, bf); t[rd] = 0; fclose(bf);
            free(FM_NM); FM_NM = malloc(64 * 2048); FM_NN = 0;
            FM_NM[0][0] = 0; /* slot 0 filled below */
            const char *p = t;
            while ((p = strstr(p, "\nfn ")) != NULL && FM_NN < 2047) {
                p += 4; const char *q = p;
                while ((*q >= 97 && *q <= 122) || (*q >= 65 && *q <= 90) || (*q >= 48 && *q <= 57) || *q == '_') q++;
                size_t L = (size_t)(q - p); if (L > 63) L = 63;
                FM_NN++; memcpy(FM_NM[FM_NN - 1], p, L); FM_NM[FM_NN - 1][L] = 0;
                p = q;
            }
            free(t);
        }
    }
}
static void cmd_fnmap(char *rest, SB *o) {
    char fullp[256] = "", bpp[256] = ""; long want = -1;
    char *save = rest; char *tok = strtok(rest, " ");
    if (tok) snprintf(fullp, sizeof fullp, "%s", tok);
    tok = strtok(NULL, " "); if (tok) snprintf(bpp, sizeof bpp, "%s", tok);
    tok = strtok(NULL, " "); if (tok) want = atol(tok);
    (void)save;
    fm_load(fullp[0] ? fullp : "/tmp/opencode/run.full",
            bpp[0] ? bpp : "/tmp/opencode/selfsrc.bp", o);
    sb_put(o, ">> fns=%d words=%ld\n", FM_NS, FM_N);
    if (want >= 0) {
        long lo = -1;
        for (int i = 0; i < FM_NS; i++) if (FM_S[i] <= want) lo = FM_S[i]; else break;
        int idx = 0;
        for (int i = 0; i < FM_NS; i++) if (FM_S[i] == lo) { idx = i; break; }
        sb_put(o, ">> w%ld -> fn#%d %s (start w%ld)\n", want, idx,
               FM_NN && idx < FM_NN ? FM_NM[idx] : "?", lo);
    }
}


/* ── runx: run WITH declared expectation -> verdict + journal (T0 law) ── */
static void cmd_runx(char *rest, SB *o) {
    char prog[256], expv[64];
    char comp[256];
    int k = sscanf(rest, "%255s %63s %255s", prog, expv, comp);
    if (k < 2) { sb_put(o, "err: runx <prog.bp> <expected> [compiler.bp]  (T0: no probe without expectation)\n"); return; }
    if (k == 2) snprintf(comp, sizeof comp, "selfhost/expr_compile.bp");
    /* reuse run pipeline inline */
    char cmd[1200];
    snprintf(cmd, sizeof cmd,
        "./native/build/bebopc compilewords %s %s > /tmp/opencode/run.full 2>/dev/null", comp, prog);
    int rc = system(cmd);
    if (rc != 0) { sb_put(o, "VERDICT:error COMPILE rc=%d\n", rc >> 8); return; }
    snprintf(cmd, sizeof cmd,
        "./native/build/agent pack /tmp/opencode/run.full /tmp/opencode/run.p.bin 0 >/dev/null 2>&1");
    if (system(cmd)) {}
    snprintf(cmd, sizeof cmd,
        "timeout 300 ./seed/build/seed /tmp/opencode/run.p.bin > /tmp/opencode/run.out 2>&1");
    rc = system(cmd);
    FILE *f = fopen("/tmp/opencode/run.out", "r");
    char buf[400]; size_t rd = f ? fread(buf, 1, sizeof buf - 1, f) : 0;
    if (f) fclose(f);
    buf[rd] = 0;
    long got = strtol(buf, NULL, 10);
    long want = strtol(expv, NULL, 10);
    int crash = (rc >= 128 || WEXITSTATUS(rc) >= 128);
    const char *ver = crash ? (want == 139 || want == 135 ? "confirmed" : "confirmed-crash")
                    : (got == want ? "confirmed" : "KILLED");
    sb_put(o, ">> GOT:%ld WANT:%ld exit=%d\nVERDICT:%s\n", got, want, WEXITSTATUS(rc), ver);
    /* journal line, mechanical */
    FILE *j = fopen("docs/exp.journal", "a");
    if (j) { fprintf(j, "%lld H:probe(%s) DID:runx GOT:%ld WANT:%ld VERDICT:%s\n",
             (long long)time(NULL), prog, got, want, ver); fclose(j); }
}

/* ── lint: allocations inside while bodies are FORBIDDEN (L8) ────────── */
static void cmd_lint(char *rest, SB *o) {
    (void)rest;
    const char *files[] = {"selfhost/expr_compile.bp","selfhost/compile.bp","selfhost/main.bp",NULL};
    int bad = 0;
    for (int fi = 0; files[fi]; fi++) {
        FILE *f = fopen(files[fi], "r");
        if (!f) continue;
        static char line[512];
        int depth = 0; int lstack[64]; int lsp = 0; int ln = 0;
        while (fgets(line, sizeof line, f)) {
            ln++;
            int opens = 0, closes = 0;
            for (char *p = line; *p; p++) { if (*p=='{') opens++; else if (*p=='}') closes++; }
            if (lsp > 0) {
                if (strncmp(line, "let ", 4) == 0) {
                    char *br = strchr(line, '[');
                    char *eq = strchr(line, '=');
                    if (br && eq && br < eq && strncmp(eq+1, "= [", 3) == 0) {}
                }
                /* flag: let NAME = [ ... ]; anywhere in a while body */
                char *lb = strstr(line, "let ");
                if (lb) {
                    char *brk = strchr(lb, '[');
                    char *semi = strchr(lb, ';');
                    char *eqs = strchr(lb, '=');
                    if (brk && eqs && brk > eqs && (!semi || brk < semi))
                        { sb_put(o, "L8 %s:%d: %.*s\n", files[fi], ln, (int)(strchr(line+3,';')?strchr(line+3,';')-line:60), line); bad++; }
                }
            }
            if (strstr(line, "while")) lstack[lsp++] = depth + opens;
            depth += opens - closes;
            while (lsp > 0 && depth <= lstack[lsp-1]) lsp--;
        }
        fclose(f);
    }
    sb_put(o, "ok: lint violations=%d\n", bad);
}

/* ── command execution ──────────────────────────────────────────────── */
static void exec_line(char *line, SB *out);

static void cmd_mem_add(char *rest, SB *o) {
    /* add <name> <kind> <stamp> <note...> */
    char *name = strtok(rest, " ");
    char *kd = strtok(NULL, " ");
    char *stmp = strtok(NULL, " ");
    char *note = strtok(NULL, "");
    if (!name || !kd || !stmp || !note) { sb_put(o, "err: mem add <name> <kind> <stamp> <note...>\n"); return; }
    pthread_mutex_lock(&g_mu);
    int idx = lmem_remember(&G, name, atoi(kd), note, (uint64_t)atoll(stmp));
    pthread_mutex_unlock(&g_mu);
    if (idx < 0) { sb_put(o, "err: graph full\n"); return; }
    sb_put(o, "ok: remembered %s #%d (%d syms)\n", name, idx, G.n_syms);
}
static void cmd_mem_link(char *rest, SB *o) {
    char *a = strtok(rest, " "), *b = strtok(NULL, " ");
    if (!a || !b) { sb_put(o, "err: mem link <a> <b>\n"); return; }
    pthread_mutex_lock(&g_mu);
    int ia = lmem_find(&G, a), ib = lmem_find(&G, b);
    if (ia < 0 || ib < 0) { pthread_mutex_unlock(&g_mu); sb_put(o, "err: unknown symbol(s)\n"); return; }
    lmem_link(&G, ia, ib);
    pthread_mutex_unlock(&g_mu);
    sb_put(o, "ok: linked %s <-> %s\n", a, b);
}
static void cmd_mem_query(char *rest, SB *o) {
    uint64_t qv[16];
    lmem_vec_from_text(rest, qv);
    int hits[8];
    pthread_mutex_lock(&g_mu);
    int nh = lmem_search(&G, qv, 5, hits);
    sb_put(o, ">> q: %s\n", rest);
    for (int i = 0; i < nh; i++) {
        LmSymbol *s = &G.syms[hits[i]];
        sb_put(o, ">> [%d] d=%d %s k=%u t=%llu :: %.120s\n",
               i, lmem_hamming_dist(qv, s->vec, LMEM_VEC_WORDS), s->name,
               s->kind, (unsigned long long)s->stamp, s->note);
        for (int e = 0; e < s->n_edges && e < 8; e++)
            sb_put(o, ">>    -> %s\n", G.syms[s->edges[e]].name);
    }
    pthread_mutex_unlock(&g_mu);
}
static void cmd_nav(char *rest, SB *o) {
    char *mode = strtok(rest, " ");
    char *path = strtok(NULL, " ");
    if (!mode || !path) { sb_put(o, "err: nav fns|find <file> [fn]\n"); return; }
    pthread_mutex_lock(&nav_mu);
    Nav *nv = nav_get(path);
    if (!nv) { pthread_mutex_unlock(&nav_mu); sb_put(o, "err: cannot read %s\n", path); return; }
    if (!strcmp(mode, "fns")) {
        sb_put(o, ">> %s: %d fns\n", path, nv->n);
        for (int i = 0; i < nv->n; i++) sb_put(o, ">> %d %s\n", i, nv->names[i]);
    } else if (!strcmp(mode, "find")) {
        char *fn = strtok(NULL, " ");
        int found = -1;
        for (int i = 0; i < nv->n; i++) if (fn && !strcmp(nv->names[i], fn)) found = i;
        if (found >= 0) sb_put(o, ">> %s idx=%d\n", fn, found);
        else sb_put(o, "err: not found\n");
    } else sb_put(o, "err: unknown nav mode\n");
    pthread_mutex_unlock(&nav_mu);
}

/* gate runner: popen, last meaningful line(s) */
static void run_gate(const char *cmd, SB *o) {
    FILE *p = popen(cmd, "r");
    if (!p) { sb_put(o, "err: popen\n"); return; }
    char last[4][512]; int li = 0;
    char ln[512];
    while (fgets(ln, sizeof ln, p)) {
        snprintf(last[li % 4], 512, "%s", ln);
        li++;
    }
    int rc = pclose(p);
    for (int i = 0; i < 4 && li > 0; i++) {
        int idx = (li - 4 + i + 16) % 4;
        if (i >= 4 - (li < 4 ? li : 4)) sb_put(o, ">> %s", last[idx]);
    }
    sb_put(o, "ok: rc=%d %s\n", rc >> 8, cmd);
}

static void cmd_gate(char *rest, SB *o) {
    if (!strncmp(rest, "parity", 6))
        run_gate("bash bench/vs_rust/parity_driver.sh 2>&1 | tail -1", o);
    else if (!strncmp(rest, "fuzz", 4))
        run_gate("cd native && ./build/fuzz_selfhost 400 2>&1 | tail -1", o);
    else if (!strncmp(rest, "test", 4))
        run_gate("make -C native test 2>&1 | grep total", o);
    else if (!strncmp(rest, "selfcompile", 11))
        run_gate("./native/build/bebopc selfcompile selfhost/expr_compile.bp 2>/dev/null | tail -1", o);
    else sb_put(o, "err: gates: parity|fuzz|test|selfcompile\n");
}

static void cmd_sh(char *rest, SB *o) {
    run_gate(rest, o);
}

/* ── parallel fan-out ───────────────────────────────────────────────── */
typedef struct { char line[1024]; SB out; char buf[4096]; int done; } Job;
static void *job_thread(void *arg) {
    Job *j = (Job *)arg;
    j->out.buf = j->buf; j->out.cap = sizeof j->buf; j->out.len = 0;
    char copy[1024];
    snprintf(copy, sizeof copy, "%s", j->line);
    exec_line(copy, &j->out);
    j->done = 1;
    return NULL;
}
static void cmd_par(char *rest, SB *o) {
    int n = atoi(rest);
    if (n <= 0 || n > 16) { sb_put(o, "err: par <1..16> then N lines\n"); return; }
    Job jobs[16];
    for (int i = 0; i < n; i++) {
        if (!fgets(jobs[i].line, sizeof jobs[i].line, stdin)) { n = i; break; }
        jobs[i].line[strcspn(jobs[i].line, "\r\n")] = 0;
        jobs[i].done = 0;
    }
    pthread_t th[16];
    for (int i = 0; i < n; i++) pthread_create(&th[i], NULL, job_thread, &jobs[i]);
    for (int i = 0; i < n; i++) pthread_join(th[i], NULL);
    for (int i = 0; i < n; i++) {
        sb_put(o, "== job%d: %s\n", i, jobs[i].line);
        sb_put(o, "%s", jobs[i].buf);
    }
}

static void exec_line(char *line, SB *o) {
    while (*line == ' ') line++;
    char *cmd = strtok(line, " ");
    if (!cmd) return;
    stat_bump(cmd);
    char *rest = strtok(NULL, "");
    if (!strcmp(cmd, "mem") && rest) {
        if (!strncmp(rest, "add ", 4)) cmd_mem_add(rest + 4, o);
        else if (!strncmp(rest, "link ", 5)) cmd_mem_link(rest + 5, o);
        else if (!strncmp(rest, "query ", 6)) cmd_mem_query(rest + 6, o);
        else sb_put(o, "err: mem add|link|query\n");
    }
    else if (!strcmp(cmd, "nav")) cmd_nav(rest, o);
    else if (!strcmp(cmd, "exp")) cmd_exp(rest, o);
    else if (!strcmp(cmd, "ctl")) cmd_ctl(rest, o);
    else if (!strcmp(cmd, "run")) cmd_run(rest, o);
    else if (!strcmp(cmd, "fnmap")) cmd_fnmap(rest, o);
    else if (!strcmp(cmd, "runx")) cmd_runx(rest, o);
    else if (!strcmp(cmd, "lint")) cmd_lint(rest, o);
    else if (!strcmp(cmd, "gate")) cmd_gate(rest, o);
    else if (!strcmp(cmd, "sh") && rest) cmd_sh(rest, o);
    else if (!strcmp(cmd, "par")) cmd_par(rest, o);
    else if (!strcmp(cmd, "hot")) {
        stat_save();
        /* top by count: selection over <=128 entries */
        for (int r = 0; r < 5 && r < NST; r++) {
            int bi = r;
            for (int i = r + 1; i < NST; i++) if (ST[i].n > ST[bi].n) bi = i;
            Stat t = ST[r]; ST[r] = ST[bi]; ST[bi] = t;
            sb_put(o, ">> %-12s %lu\n", ST[r].name, ST[r].n);
        }
    }
    else if (!strcmp(cmd, "save")) {
        pthread_mutex_lock(&g_mu);
        lmem_save(&G, MEM_PATH);
        pthread_mutex_unlock(&g_mu);
        stat_save();
        sb_put(o, "ok: saved\n");
    }
    else if (!strcmp(cmd, "quit")) sb_put(o, "ok: bye\n");
    else sb_put(o, "err: ? (mem|nav|exp|ctl|run|fnmap|gate|sh|par|hot|save|quit)\n");
}

int main(void) {
    setvbuf(stdout, NULL, _IOLBF, 0);
    lmem_init(&G);
    if (lmem_load(&G, MEM_PATH) != 0) fprintf(stderr, "# fresh graph\n");
    else fprintf(stderr, "# %d syms loaded\n", G.n_syms);
    stat_load();
    char line[1024];
    SB out; char buf[8192];
    while (fgets(line, sizeof line, stdin)) {
        line[strcspn(line, "\r\n")] = 0;
        if (!*line) continue;
        out.buf = buf; out.cap = sizeof buf; out.len = 0;
        exec_line(line, &out);
        fwrite(buf, 1, out.len < sizeof buf ? out.len : sizeof buf - 1, stdout);
        fflush(stdout);
        if (!strncmp(line, "quit", 4)) break;
    }
    pthread_mutex_lock(&g_mu);
    lmem_save(&G, MEM_PATH);
    pthread_mutex_unlock(&g_mu);
    stat_save();
    return 0;
}
