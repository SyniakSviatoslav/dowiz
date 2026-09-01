/* hermes-disasm — AArch64 instruction word decoder for Bebop self-hosting.
   Reads decimal AArch64 words from stdin or a file, prints mnemonic + operands.
   Usage: ./hermes-disasm < words.txt   or   ./hermes-disasm words.txt */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* ── AArch64 instruction decoding (subset used by bebop) ─────────────────── */

static const char *cond_name(unsigned int c) {
    static const char *names[] = {
        "eq","ne","hs","lo","mi","pl","vs","vc",
        "hi","ls","ge","lt","gt","le","al","nv"
    };
    return names[c & 0xF];
}

static unsigned int U(unsigned int w, unsigned int lo, unsigned int hi) {
    return (w >> lo) & ((1u << (hi - lo + 1u)) - 1u);
}

static int Bit(unsigned int w, unsigned int n) {
    return (w >> n) & 1u;
}

/* Decode one AArch64 instruction word. Returns 0 on success, -1 if unhandled. */
static int decode(unsigned int w, char *out, size_t cap) {
    unsigned int op0 = U(w, 28, 30);
    unsigned int op1 = U(w, 4, 7);

    /* Conditional branch (B.cond) */
    if (op0 == 6 && Bit(w, 4) == 0 && op1 == 10) {
        unsigned int cond = U(w, 12, 15);
        long imm = (long)((int)U(w, 0, 25) * 4);
        if (Bit(w, 25)) imm = (long)((int)((unsigned int)U(w, 0, 25) | 0xFC000000u) * 4);
        snprintf(out, cap, "b.%s #%ld", cond_name(cond), imm);
        return 0;
    }

    /* B / BL / RET / BR */
    if (op0 == 6 && U(w, 5, 10) == 60 && op1 == 10) {
        if (Bit(w, 24) == 0 && Bit(w, 23) == 0) {
            long imm = (long)((int)U(w, 0, 25) * 4);
            if (Bit(w, 25)) imm = (long)((int)((unsigned int)U(w, 0, 25) | 0xFC000000u) * 4);
            snprintf(out, cap, "b #%ld", imm);
            return 0;
        }
        if (Bit(w, 24) == 1 && Bit(w, 23) == 0) {
            long imm = (long)((int)U(w, 0, 25) * 4);
            if (Bit(w, 25)) imm = (long)((int)((unsigned int)U(w, 0, 25) | 0xFC000000u) * 4);
            snprintf(out, cap, "bl #%ld", imm);
            return 0;
        }
        if (Bit(w, 24) == 0 && Bit(w, 23) == 1) {
            snprintf(out, cap, "ret x%d", U(w, 5, 10));
            return 0;
        }
        if (Bit(w, 24) == 1 && Bit(w, 23) == 1) {
            snprintf(out, cap, "br x%d", U(w, 5, 10));
            return 0;
        }
    }

    /* STR/LDP/STP (store/load) */
    if (op0 == 4 && op1 == 13) {
        if (Bit(w, 22) == 1 && Bit(w, 21) == 0) {
            snprintf(out, cap, "stp x%d, x%d, [sp, #%u]", U(w,0,4), U(w,5,9), U(w,0,6));
            return 0;
        }
        if (Bit(w, 22) == 0 && Bit(w, 21) == 0 && U(w, 22, 22) == 0) {
            snprintf(out, cap, "ldp x%d, x%d, [sp, #%u]", U(w,0,4), U(w,5,9), U(w,0,6));
            return 0;
        }
    }
    if (op0 == 4 && op1 == 4) {
        if (Bit(w, 22) == 1) {
            snprintf(out, cap, "str x%d, [sp, #%u]", U(w,0,4), U(w,0,8));
            return 0;
        } else {
            snprintf(out, cap, "ldr x%d, [sp, #%u]", U(w,0,4), U(w,0,8));
            return 0;
        }
    }

    /* Data processing — register form (ADD, SUB, AND, etc.) */
    if (op0 == 7 && op1 == 0) {
        unsigned int s = Bit(w, 30);
        unsigned int opc = U(w, 4, 5) | (U(w, 7, 6) << 2);
        const char *mn;
        switch (opc & 0xF) {
            case 0: mn = "and"; break;
            case 1: mn = "eor"; break;
            case 2: mn = "sub"; break;
            case 3: mn = "rsb"; break;
            case 4: mn = "add"; break;
            case 5: mn = "adc"; break;
            case 6: mn = "sbc"; break;
            case 7: mn = "rsc"; break;
            case 8: mn = "tst"; break;
            case 9: mn = "teq"; break;
            case 10: mn = "cmp"; break;
            case 11: mn = "cmn"; break;
            case 12: mn = "orr"; break;
            case 13: mn = "mov"; break;
            case 14: mn = "bic"; break;
            case 15: mn = "mvn"; break;
            default: mn = "?"; break;
        }
        if (mn[0] == 't' || mn[0] == 'c') {
            snprintf(out, cap, "%s x%d, x%d", mn, U(w,5,9), U(w,10,14));
        } else {
            snprintf(out, cap, "%s%s x%d, x%d, x%d",
                     mn, s ? "s" : "", U(w,0,4), U(w,5,9), U(w,10,14));
        }
        return 0;
    }

    /* MADD */
    if (op0 == 7 && op1 == 12) {
        snprintf(out, cap, "madd x%d, x%d, x%d, x%d",
                 U(w,0,4), U(w,5,9), U(w,10,14), U(w,15,19));
        return 0;
    }

    /* MOVZ/MOVN/MOVK */
    if (op0 == 6 && op1 == 2) {
        unsigned int h = U(w, 10, 12);
        unsigned int imm16 = U(w, 5, 10) | (U(w, 13, 14) << 2) | (U(w, 15, 15) << 4);
        const char *mn;
        if (h == 0) mn = "movz";
        else if (h == 1) mn = "movn";
        else mn = "movk";
        snprintf(out, cap, "%s x%d, #%u", mn, U(w,0,4), imm16);
        return 0;
    }

    /* CSEL/CSINC/CSINV/CNEG */
    if (op0 == 5 && op1 == 15) {
        unsigned int m = U(w, 21, 23);
        const char *mn;
        if (m == 0) mn = "csel";
        else if (m == 1) mn = "csinc";
        else if (m == 2) mn = "csinv";
        else if (m == 3) mn = "cneg";
        else mn = "?";
        snprintf(out, cap, "%s x%d, x%d, x%d",
                 mn, U(w,0,4), U(w,5,9), U(w,10,14));
        return 0;
    }

    snprintf(out, cap, "0x%08x (unhandled)", w);
    return -1;
}

/* ── Main ──────────────────────────────────────────────────────────────────── */

int main(int argc, char **argv) {
    FILE *f = stdin;
    if (argc > 1) {
        f = fopen(argv[1], "r");
        if (!f) { perror(argv[1]); return 1; }
    }

    char line[64];
    unsigned int w;
    int i = 0;
    while (fgets(line, sizeof line, f)) {
        if (sscanf(line, "%u", &w) != 1) continue;
        char buf[128];
        decode(w, buf, sizeof buf);
        printf("[%04d] 0x%08x  %s\n", i++, w, buf);
    }

    if (argc > 1) fclose(f);
    printf("\nTotal: %d instructions\n", i);
    return 0;
}
