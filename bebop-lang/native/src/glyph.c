/* Bebop glyphs — vector δ-outlines, NOT emoji (implementation).
 *
 * The canonical form of a glyph is its δ-outline (pen moves on a pixel grid).
 * This module holds a small built-in corpus (ordinary lexicon) and renders
 * glyphs to braille text. The UTF-8 placeholder string is terminal-render only.
 */
#include "glyph.h"

#include <string.h>

/* ─── built-in corpus: 7×7 bitmaps ('#'=set, '.'=unset) ─────────────── */

typedef struct {
    const char *rows[7];
} BpBitmap;

static const BpBitmap FN_BM = {{
    "...#...",
    "...#...",
    ".####..",
    "#######",
    ".####..",
    "...#...",
    "...#...",
}};

static const BpBitmap STRUCT_BM = {{ /* diamond ◇ */
    "...#...",
    "..###..",
    ".#####.",
    "#######",
    ".#####.",
    "..###..",
    "...#...",
}};

static const BpBitmap DATA_BM = {{ /* triangle △ */
    "...#...",
    "..###..",
    "..###..",
    ".#####.",
    ".#####.",
    "#######",
    "#######",
}};

static const BpBitmap CONTRACT_BM = {{ /* circle ⊙ */
    "..###..",
    ".#####.",
    "##...##",
    "##...##",
    "##...##",
    ".#####.",
    "..###..",
}};

static const BpBitmap VAL_BM = {{ /* filled circle ◉ */
    "..###..",
    ".#####.",
    "#######",
    "#######",
    "#######",
    ".#####.",
    "..###..",
}};

static const BpBitmap MOD_BM = {{ /* square ◈ */
    "#######",
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    "#######",
}};

#define N_GLYPHS 6

static BpDelta FN_OUT[64];
static BpDelta STRUCT_OUT[64];
static BpDelta DATA_OUT[64];
static BpDelta CONTRACT_OUT[64];
static BpDelta VAL_OUT[64];
static BpDelta MOD_OUT[64];

static BpGlyph GLYPHS[N_GLYPHS];
static int initialized = 0;

/* ─── δ-outline derivation (row-major runs) ─────────────────────────── */

size_t bp_bitmap_to_outline(const char *const *rows, unsigned char height,
                            BpDelta *out, size_t cap) {
    size_t n = 0;
    short pen_x = 0, pen_y = 0;
    for (unsigned char y = 0; y < height; y++) {
        const char *row = rows[y];
        int x = 0;
        while (row[x]) {
            if (row[x] == '#') {
                int start = x;
                while (row[x] == '#') {
                    x++;
                }
                int len = x - start;
                if (n + 1 < cap) {
                    out[n].dx = (short)(start - pen_x);
                    out[n].dy = (short)(y - pen_y);
                    n++;
                    out[n].dx = (short)len;
                    out[n].dy = 0;
                    n++;
                }
                pen_x = (short)(start + len);
                pen_y = (short)y;
            } else {
                x++;
            }
        }
    }
    return n;
}

static void init_glyphs(void) {
    if (initialized) {
        return;
    }
    static const BpBitmap *bms[N_GLYPHS] = {
        &FN_BM, &STRUCT_BM, &DATA_BM, &CONTRACT_BM, &VAL_BM, &MOD_BM,
    };
    static const char *names[N_GLYPHS] = {
        "fn", "struct", "data", "contract", "val", "mod",
    };
    static const char *ph[N_GLYPHS] = {
        "→", "◇", "△", "⊙", "◉", "◈",
    };
    static BpDelta *outs[N_GLYPHS] = {
        FN_OUT, STRUCT_OUT, DATA_OUT, CONTRACT_OUT, VAL_OUT, MOD_OUT,
    };
    for (int i = 0; i < N_GLYPHS; i++) {
        size_t n = bp_bitmap_to_outline(bms[i]->rows, 7, outs[i], 64);
        GLYPHS[i].name = names[i];
        GLYPHS[i].placeholder = ph[i];
        GLYPHS[i].outline = outs[i];
        GLYPHS[i].outline_len = n;
        GLYPHS[i].width = 7;
        GLYPHS[i].height = 7;
    }
    initialized = 1;
}

/* ─── rasterize outline → grid ───────────────────────────────────────── */

static void rasterize(const BpGlyph *g, unsigned char *grid) {
    short pen_x = 0, pen_y = 0;
    for (size_t i = 0; i + 1 < g->outline_len; i += 2) {
        short mdx = g->outline[i].dx;
        short mdy = g->outline[i].dy;
        short len = g->outline[i + 1].dx;
        pen_x = (short)(pen_x + mdx);
        pen_y = (short)(pen_y + mdy);
        for (short k = 0; k < len; k++) {
            short cx = (short)(pen_x + k);
            if (pen_y >= 0 && pen_y < g->height && cx >= 0 && cx < g->width) {
                grid[pen_y * g->width + cx] = 1;
            }
        }
        pen_x = (short)(pen_x + len);
    }
}

static int utf8_encode(unsigned int cp, char *out) {
    if (cp < 0x80) {
        out[0] = (char)cp;
        return 1;
    }
    if (cp < 0x800) {
        out[0] = (char)(0xC0 | (cp >> 6));
        out[1] = (char)(0x80 | (cp & 0x3F));
        return 2;
    }
    out[0] = (char)(0xE0 | (cp >> 12));
    out[1] = (char)(0x80 | ((cp >> 6) & 0x3F));
    out[2] = (char)(0x80 | (cp & 0x3F));
    return 3;
}

/* Braille dot bit map: [col][row] → bit. Standard braille dot numbering
 * (dots 1..8), Unicode braille block bit layout. */
static const unsigned char DOT[2][4] = {
    {0x01, 0x02, 0x04, 0x40},
    {0x08, 0x10, 0x20, 0x80},
};

int bp_glyph_render_braille(const BpGlyph *g, char *out, size_t cap) {
    unsigned char grid[64];
    if ((size_t)g->width * g->height > sizeof grid) {
        return -1;
    }
    memset(grid, 0, sizeof grid);
    rasterize(g, grid);

    size_t pos = 0;
    for (int by = 0; by < g->height; by += 4) {
        for (int bx = 0; bx < g->width; bx += 2) {
            int bits = 0;
            for (int r = 0; r < 4; r++) {
                int y = by + r;
                if (y >= g->height) {
                    break;
                }
                if (bx < g->width && grid[y * g->width + bx]) {
                    bits |= DOT[0][r];
                }
                if (bx + 1 < g->width && grid[y * g->width + bx + 1]) {
                    bits |= DOT[1][r];
                }
            }
            char u8[4];
            int ulen = utf8_encode((unsigned int)(0x2800 + bits), u8);
            if (pos + (size_t)ulen + 1 > cap) {
                return -1;
            }
            memcpy(out + pos, u8, (size_t)ulen);
            pos += (size_t)ulen;
        }
        if (pos + 1 < cap) {
            out[pos++] = '\n';
        } else {
            return -1;
        }
    }
    out[pos] = '\0';
    return (int)pos;
}

const BpGlyph *bp_glyph_by_name(const char *name) {
    init_glyphs();
    for (int i = 0; i < N_GLYPHS; i++) {
        if (strcmp(GLYPHS[i].name, name) == 0) {
            return &GLYPHS[i];
        }
    }
    return NULL;
}

size_t bp_glyph_count(void) {
    return N_GLYPHS;
}

const BpGlyph *bp_glyph_at(size_t i) {
    init_glyphs();
    return (i < N_GLYPHS) ? &GLYPHS[i] : NULL;
}
