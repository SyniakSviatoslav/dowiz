/* agent.c — compiled helper for the agent workflow (replaces slow,
 * error-prone python one-offs). One binary, subcommands, zero deps.
 * All word handling is little-endian u32, matching AArch64 streams. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

static uint32_t *load_words_text(const char *path, size_t *n) {
    FILE *f = fopen(path, "r");
    if (!f) return NULL;
    static uint32_t buf[1 << 21];
    char line[256];
    size_t k = 0;
    long first = -1;
    while (fgets(line, sizeof line, f)) {
        char *p = line;
        while (*p == ' ' || *p == '\t') p++;
        if (*p == '-' || *p == '+') p++;
        if (*p < '0' || *p > '9') continue;      /* skip OFF/headers/blank */
        int digits = 1;
        for (char *q = p; *q && *q != '\n' && *q != '\r'; q++)
            if (*q == ' ' || *q == '\t') { digits = 0; break; }
        if (!digits) continue;                    /* multi-number line */
        unsigned long v = strtoul(p, NULL, 10);
        if (first < 0) { first = (long)v; continue; } /* count line */
        if (k >= sizeof buf / sizeof buf[0]) break;
        buf[k++] = (uint32_t)(v & 0xFFFFFFFFUL);
    }
    fclose(f);
    (void)first;
    *n = k;
    return buf;
}

static int cmd_pack(int argc, char **argv) {
    if (argc < 4) { fprintf(stderr, "usage: agent pack <in.full> <out.bin> [entry_byte]\n"); return 2; }
    size_t n; uint32_t *w = load_words_text(argv[2], &n);
    if (!w || !n) { fprintf(stderr, "no words\n"); return 1; }
    uint64_t entry = argc > 4 ? strtoull(argv[4], NULL, 10) : 0;
    FILE *o = fopen(argv[3], "wb");
    if (!o) { perror("open out"); return 1; }
    fwrite(w, 4, n, o);
    fwrite(&entry, 8, 1, o);
    fclose(o);
    printf("%zu words + footer -> %s (%zu bytes), entry=%llu\n",
           n, argv[3], n * 4 + 8, (unsigned long long)entry);
    return 0;
}

static int cmd_sum(int argc, char **argv) {
    if (argc < 3) { fprintf(stderr, "usage: agent sum <in.full>\n"); return 2; }
    size_t n; uint32_t *w = load_words_text(argv[2], &n);
    if (!w) { fprintf(stderr, "no words\n"); return 1; }
    /* mirror of .bp sum_words(words, words[0]+1): count word + all words */
    unsigned long long acc = n; /* words[0] == count */
    for (size_t i = 0; i < n; i++) acc += w[i];
    printf("%llu %zu\n", acc, n);
    return 0;
}

static int cmd_prolog(int argc, char **argv) {
    /* verify every OFF offset points at stp x29,x30 prologue */
    if (argc < 3) { fprintf(stderr, "usage: agent prolog <artifact.full>\n"); return 2; }
    FILE *f = fopen(argv[2], "r");
    if (!f) { perror("open"); return 1; }
    static uint32_t w[1 << 21]; size_t n = 0;
    long offs[1024]; int noffs = 0;
    char line[65536];
    while (fgets(line, sizeof line, f)) {
        if (strncmp(line, "OFF", 3) == 0) {
            char *p = line + 3;
            char *tok = strtok(p, " ");
            int first = 1;
            while (tok) {
                if (!first && noffs < 1024) offs[noffs++] = atol(tok);
                first = 0;
                tok = strtok(NULL, " ");
            }
        } else {
            char *end; unsigned long v = strtoul(line, &end, 10);
            if (end != line && n < sizeof w / sizeof w[0]) w[n++] = (uint32_t)v;
        }
    }
    fclose(f);
    int bad = 0;
    const uint32_t PROLOG = 0xa9bf7bfdu; /* stp x29,x30,[sp,#-16]! */
    for (int i = 0; i < noffs; i++) {
        if ((size_t)offs[i] >= n || w[offs[i]] != PROLOG) {
            if (bad < 8) printf("block %d off=%ld not-prolog (word=%08x)\n", i, offs[i], (size_t)offs[i] < n ? w[offs[i]] : 0);
            bad++;
        }
    }
    printf("prolog: %d/%d blocks ok\n", noffs - bad, noffs);
    return bad ? 1 : 0;
}

/* exact mirror of selfhost collect_fns (lowercase-alpha starts only) */
static int scan_fns(const char *s, long n, char names[][64], int max) {
    long j = 0; int cnt = 0;
    while (j + 2 < n) {
        unsigned char c0 = s[j], c1 = s[j+1], c2 = s[j+2];
        int is_quote = c0 == 34;
        int is_comment = c0 == 47 && c1 == 47;
        unsigned char cafter = j + 3 < n ? s[j+3] : 0;
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

static int cmd_entryfind(int argc, char **argv) {
    if (argc < 5) { fprintf(stderr, "usage: agent entryfind <src.bp> <artifact.full> <fn>\n"); return 2; }
    FILE *f = fopen(argv[2], "rb");
    if (!f) { perror("src"); return 1; }
    fseek(f, 0, SEEK_END); long sz = ftell(f); fseek(f, 0, SEEK_SET);
    char *src = malloc((size_t)sz + 1);
    if (!src || fread(src, 1, (size_t)sz, f) != (size_t)sz) { fclose(f); return 1; }
    src[sz] = 0; fclose(f);
    static char names[512][64];
    int nn = scan_fns(src, sz, names, 512);
    FILE *g = fopen(argv[3], "r");
    if (!g) { perror("artifact"); return 1; }
    long offs[512]; int no = -1;
    char line[1 << 16];
    while (fgets(line, sizeof line, g)) {
        if (strncmp(line, "OFF", 3) == 0) {
            no = 0;
            char *p = line + 3, *tok = strtok(p, " ");
            int first = 1;
            while (tok) {
                if (!first && no < 512) offs[no++] = atol(tok);
                first = 0;
                tok = strtok(NULL, " ");
            }
        }
    }
    fclose(g);
    if (no < 0) { fprintf(stderr, "no OFF line\n"); return 1; }
    printf("names=%d offs=%d\n", nn, no);
    if (nn != no) { fprintf(stderr, "PAIRED-COUNT MISMATCH\n"); return 1; }
    for (int i = 0; i < nn; i++) {
        if (strcmp(names[i], argv[4]) == 0) {
            printf("%s: idx=%d word=%ld byte=%ld\n", names[i], i, offs[i], offs[i]*4);
            return 0;
        }
    }
    fprintf(stderr, "fn not found: %s\n", argv[4]);
    return 1;
}

int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "agent: pack|sum|prolog|entryfind\n"); return 2; }
    if (!strcmp(argv[1], "pack")) return cmd_pack(argc, argv);
    if (!strcmp(argv[1], "sum")) return cmd_sum(argc, argv);
    if (!strcmp(argv[1], "prolog")) return cmd_prolog(argc, argv);
    if (!strcmp(argv[1], "entryfind")) return cmd_entryfind(argc, argv);
    fprintf(stderr, "unknown subcommand\n");
    return 2;
}
