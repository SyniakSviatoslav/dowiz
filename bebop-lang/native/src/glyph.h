/* Bebop glyphs — vector δ-outlines, NOT emoji.
 *
 * The canonical form of every Bebop symbol is a δ-outline on a pixel grid
 * (a sequence of pen moves). The UTF-8 string is only a terminal-render
 * placeholder. This module holds the built-in corpus and renders glyphs to
 * braille text (the terminal tier). Ordinary lexicon.
 */
#ifndef BEBOP_GLYPH_H
#define BEBOP_GLYPH_H

#include <stddef.h>

/* One pen move on the grid. */
typedef struct {
    short dx;
    short dy;
} BpDelta;

/* A glyph: ASCII fallback name, terminal placeholder, and the canonical
 * δ-outline over a width x height grid. */
typedef struct {
    const char *name;           /* ordinary ASCII token (fn, struct, ...) */
    const char *placeholder;    /* UTF-8 render placeholder (not canonical) */
    const BpDelta *outline;     /* canonical δ-outline (vector) */
    size_t outline_len;
    unsigned char width;        /* grid width  */
    unsigned char height;       /* grid height */
} BpGlyph;

/* Render a glyph's δ-outline to braille text. Returns bytes written (excl NUL),
 * or -1 on overflow. */
int bp_glyph_render_braille(const BpGlyph *g, char *out, size_t cap);

/* Look up a glyph by ASCII fallback name. NULL if unknown. */
const BpGlyph *bp_glyph_by_name(const char *name);

/* Corpus size + indexed access. */
size_t bp_glyph_count(void);
const BpGlyph *bp_glyph_at(size_t i);

/* Derive a δ-outline from a width x height bitmap (rows of '#'/'.').
 * Returns number of deltas written (<= cap). This is the canonical form. */
size_t bp_bitmap_to_outline(const char *const *rows, unsigned char height,
                            BpDelta *out, size_t cap);

#endif /* BEBOP_GLYPH_H */
