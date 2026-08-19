/* Bebop glyphs — vector δ-outlines, NOT emoji (implementation).
 *
 * The canonical form of a glyph is its δ-outline (pen moves on a pixel grid).
 * This module holds a small built-in corpus (ordinary lexicon) and renders
 * glyphs to braille text. The UTF-8 placeholder string is terminal-render only.
 */
#include "glyph.h"

#include <string.h>

typedef struct {
    const char *rows[7];
} BpBitmap;

/* ─── built-in corpus: 7×7 bitmaps ('#'=set, '.'=unset) ─────────────── */

static const BpBitmap FN_BM = {{ /* → */
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
static const BpBitmap LET_BM = {{ /* ≡ */
    "#......",
    "#......",
    "#......",
    "#......",
    "#......",
    "#......",
    "#######",
}};
static const BpBitmap IF_BM = {{ /* ? */
    "..###..",
    ".#####.",
    "##...##",
    "##...##",
    "##...##",
    "..###..",
    "...#...",
}};
static const BpBitmap THEN_BM = {{ /* ⇒ */
    "##.....",
    ".##....",
    "..##...",
    "...##..",
    "....##.",
    ".....##",
    "......#",
}};
static const BpBitmap ELSE_BM = {{ /* ∨ */
    ".......",
    ".......",
    ".#...#.",
    ".#...#.",
    ".#...#.",
    ".#####.",
    "...#...",
}};
static const BpBitmap WHILE_BM = {{ /* ∞ */
    ".#####.",
    "#.....#",
    "#..##.#",
    "#.##..#",
    "#..##.#",
    "#.....#",
    ".#####.",
}};
static const BpBitmap MATCH_BM = {{ /* ⊕ */
    ".......",
    "...#...",
    "..###..",
    ".#####.",
    "..###..",
    "...#...",
    ".......",
}};
static const BpBitmap TYPE_BM = {{ /* τ */
    ".......",
    "#######",
    "...#...",
    "...#...",
    "...#...",
    "....#..",
    ".....#.",
}};
static const BpBitmap THEOREM_BM = {{ /* ∎ */
    "#######",
    "#######",
    "#######",
    "#######",
    "#######",
    "#######",
    "#######",
}};
static const BpBitmap PROOF_BM = {{ /* ⊢ */
    "..#....",
    "...#...",
    "....#..",
    ".....##",
    "....#..",
    "...#...",
    "..#....",
}};
static const BpBitmap ENUM_BM = {{ /* ⋃ */
    ".......",
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    ".#####.",
}};
static const BpBitmap I64_BM = {{ /* ℤ */
    ".......",
    "#######",
    "....#..",
    "...#...",
    "..#....",
    ".#.....",
    "#......",
}};
static const BpBitmap U64_BM = {{ /* ℕ */
    ".......",
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    "#######",
}};
static const BpBitmap F64_BM = {{ /* ℝ */
    ".......",
    ".......",
    "#######",
    "#.....#",
    "#.....#",
    "#######",
    ".......",
}};
static const BpBitmap BOOL_BM = {{ /* 𝔹 */
    "#######",
    "#.....#",
    "#.....#",
    "#######",
    "#.....#",
    "#.....#",
    "#######",
}};
static const BpBitmap STR_BM = {{ /* S */
    "..###..",
    ".#...#.",
    ".#.....",
    "..###..",
    ".....#.",
    "#....#.",
    ".###...",
}};
static const BpBitmap NAT_BM = {{ /* N */
    "#.....#",
    "##....#",
    "#.#...#",
    "#..#..#",
    "#...#.#",
    "#....##",
    "#.....#",
}};
static const BpBitmap VEC_BM = {{ /* V */
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    ".#...#.",
    "...#...",
}};
static const BpBitmap SPAWN_BM = {{ /* ≫ */
    "#......",
    ".#.....",
    "..##...",
    "...##..",
    "....##.",
    ".....##",
    "......#",
}};
static const BpBitmap AWAIT_BM = {{ /* ⏸ */
    ".#####.",
    "#.....#",
    "#.....#",
    "#######",
    "#.....#",
    "#.....#",
    ".#####.",
}};
static const BpBitmap ADD_BM = {{ /* + */
    ".......",
    "...#...",
    "...#...",
    "#####..",
    "...#...",
    "...#...",
    ".......",
}};
static const BpBitmap SUB_BM = {{ /* − */
    ".......",
    ".......",
    ".......",
    "#####..",
    ".......",
    ".......",
    ".......",
}};
static const BpBitmap MUL_BM = {{ /* × */
    ".......",
    ".#...#.",
    "..#.#..",
    "...#...",
    "..#.#..",
    ".#...#.",
    ".......",
}};
static const BpBitmap DIV_BM = {{ /* ÷ */
    ".......",
    "...#...",
    ".......",
    "#####..",
    ".......",
    "...#...",
    ".......",
}};
static const BpBitmap EQ_BM = {{ /* = */
    ".......",
    ".......",
    "#####..",
    ".......",
    "#####..",
    ".......",
    ".......",
}};
static const BpBitmap AND_BM = {{ /* ∧ */
    ".......",
    "...#...",
    "...#...",
    "...#...",
    "...#...",
    "...#...",
    ".......",
}};
static const BpBitmap OR_BM = {{ /* ∨ */
    ".......",
    ".......",
    ".......",
    ".......",
    "...#...",
    "...#...",
    ".......",
}};
static const BpBitmap NOT_BM = {{ /* ¬ */
    ".......",
    ".......",
    ".......",
    "#####..",
    ".......",
    ".......",
    ".......",
}};
static const BpBitmap ARROW_BM = {{ /* → */
    "...#...",
    "....#..",
    "#####..",
    "....#..",
    "...#...",
    ".......",
    ".......",
}};
static const BpBitmap ASSIGN_BM = {{ /* ≔ */
    ".......",
    ".......",
    "#####..",
    ".......",
    "..###..",
    ".......",
    ".......",
}};
static const BpBitmap LT_BM = {{ /* < */
    ".......",
    "...#...",
    "..#....",
    ".#.....",
    "..#....",
    "...#...",
    ".......",
}};
static const BpBitmap GT_BM = {{ /* > */
    ".......",
    "...#...",
    "....#..",
    ".....#.",
    "....#..",
    "...#...",
    ".......",
}};
static const BpBitmap LE_BM = {{ /* ≤ */
    ".......",
    "...#...",
    "..#....",
    ".#.....",
    "..#....",
    "...#...",
    "#####..",
}};
static const BpBitmap GE_BM = {{ /* ≥ */
    ".......",
    "...#...",
    "....#..",
    ".....#.",
    "....#..",
    "...#...",
    "#####..",
}};
static const BpBitmap PIPE_BM = {{ /* | */
    "...#...",
    "...#...",
    "...#...",
    "...#...",
    "...#...",
    "...#...",
    "...#...",
}};
static const BpBitmap COLON_BM = {{ /* : */
    ".......",
    "...#...",
    "...#...",
    ".......",
    "...#...",
    "...#...",
    ".......",
}};
static const BpBitmap COMMA_BM = {{ /* , */
    ".......",
    ".......",
    ".......",
    ".......",
    "...#...",
    "..#....",
    ".......",
}};
static const BpBitmap SEMI_BM = {{ /* ; */
    ".......",
    "...#...",
    "...#...",
    ".......",
    "...#...",
    "..#....",
    ".......",
}};
static const BpBitmap UNIT_BM = {{ /* () */
    ".......",
    ".......",
    "..###..",
    "..###..",
    "..###..",
    "..###..",
    ".......",
}};
static const BpBitmap LIST_BM = {{ /* [] */
    ".......",
    "#####..",
    "#.....#",
    "#.....#",
    "#.....#",
    "#####..",
    ".......",
}};
static const BpBitmap PAIR_BM = {{ /* × */
    ".......",
    ".#...#.",
    "..#.#..",
    "...#...",
    "..#.#..",
    ".#...#.",
    ".......",
}};
static const BpBitmap OPTION_BM = {{ /* ?+ */
    "..###..",
    ".#...#.",
    "#.....#",
    "#.....#",
    "#.....#",
    ".#...#.",
    "..###..",
}};
static const BpBitmap RESULT_BM = {{ /* ✓✗ */
    "#.....#",
    ".#...#.",
    "..#..#.",
    "...#...",
    "..#..#.",
    ".#...#.",
    "#.....#",
}};
static const BpBitmap STREAM_BM = {{ /* ~ */
    ".......",
    ".......",
    "#.....#",
    ".#...#.",
    "..#..#.",
    "...#...",
    ".......",
}};
static const BpBitmap FNPTR_BM = {{ /* λ */
    "#......",
    "#......",
    "##.....",
    "#.#....",
    "#..#...",
    "#...#..",
    "#.....#",
}};
static const BpBitmap CHECK_BM = {{ /* ⊢ */
    "#......",
    "#......",
    "#......",
    "#......",
    "#......",
    "#......",
    "#######",
}};
static const BpBitmap INFER_BM = {{ /* ⊣ */
    ".....#.",
    ".....#.",
    ".....#.",
    ".....#.",
    ".....#.",
    ".....#.",
    "#######",
}};
static const BpBitmap EVAL_BM = {{ /* ⇓ */
    "...#...",
    "...#...",
    "...#...",
    "...#...",
    "#######",
    ".####..",
    "..##...",
}};
static const BpBitmap NORM_BM = {{ /* ↓ */
    "...#...",
    "...#...",
    "...#...",
    "...#...",
    "...#...",
    "#####..",
    ".......",
}};
static const BpBitmap REFL_BM = {{ /* ≡r */
    "#######",
    ".......",
    "#######",
    ".......",
    "#######",
    ".......",
    ".......",
}};
static const BpBitmap UNIVERSE_BM = {{ /* U */
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    "#.....#",
    ".#####.",
}};
static const BpBitmap EFFECT_BM = {{ /* ⚡ */
    "...#...",
    "...#...",
    "..###..",
    ".#####.",
    "#######",
    "..###..",
    "...#...",
}};
static const BpBitmap SESSION_BM = {{ /* ⇄ */
    "#.....#",
    "#.....#",
    ".#####.",
    "#######",
    ".#####.",
    "#.....#",
    "#.....#",
}};
static const BpBitmap QUANTITY_BM = {{ /* ω */
    ".......",
    ".......",
    ".#####.",
    "#.....#",
    "#.....#",
    "#.....#",
    ".......",
}};
static const BpBitmap ONE_BM = {{ /* 1 */
    "...#...",
    "..##...",
    "...#...",
    "...#...",
    "...#...",
    "...#...",
    "#####..",
}};
static const BpBitmap MANY_BM = {{ /* ω */
    ".......",
    ".##.##.",
    ".##.##.",
    ".##.##.",
    ".##.##.",
    ".##.##.",
    ".......",
}};
static const BpBitmap PURE_BM = {{ /* ○ */
    "..###..",
    ".#...#.",
    "#.....#",
    "#.....#",
    "#.....#",
    ".#...#.",
    "..###..",
}};
static const BpBitmap IO_BM = {{ /* ⇢ */
    "..#....",
    "...#...",
    "#######",
    "...#...",
    "..#....",
    ".......",
    ".......",
}};
static const BpBitmap STATE_BM = {{ /* ⧉ */
    "#.....#",
    "#.....#",
    "#######",
    "#######",
    "#######",
    "#.....#",
    "#.....#",
}};
static const BpBitmap SEND_BM = {{ /* ⇑ */
    "...#...",
    "..###..",
    ".#####.",
    "#######",
    "...#...",
    "...#...",
    "...#...",
}};
static const BpBitmap RECV_BM = {{ /* ⇓ */
    "...#...",
    "...#...",
    "...#...",
    "#######",
    ".#####.",
    "..###..",
    "...#...",
}};
static const BpBitmap DUAL_BM = {{ /* ⇿ */
    "#.....#",
    ".#...#.",
    "..#.#..",
    "...#...",
    "..#.#..",
    ".#...#.",
    "#.....#",
}};
static const BpBitmap EXTERN_BM = {{ /* ↗ */
    ".....#.",
    "....##.",
    "...##..",
    "..##...",
    ".##....",
    "##.....",
    "#......",
}};
static const BpBitmap COMPTIME_BM = {{ /* ⏱ */
    ".#####.",
    "#.....#",
    "#..##.#",
    "#.#.#.#",
    "#..##.#",
    "#.....#",
    ".#####.",
}};
static const BpBitmap ASM_BM = {{ /* ⌬ */
    "..###..",
    ".#####.",
    "#######",
    "#######",
    "#######",
    ".#####.",
    "..###..",
}};

static const BpBitmap DMA_BM = {{ /* D */
    ".......",
    "..####.",
    ".#...#.",
    "#.....#",
    ".......",
    ".......",
    ".......",
}}};

static const BpBitmap KERNEL_BM = {{ /* K */
    ".......",
    "...#...",
    "...#...",
    ".#####.",
    "...#...",
    ".......",
    ".......",
}}};

static const BpBitmap INVOKE_BM = {{ /* I */
    ".......",
    "...###.",
    "...#...",
    "...#...",
    "...###.",
    ".......",
    ".......",
}}};

static const BpBitmap BUNDLE_BM = {{ /* B */
    ".......",
    "#.....#",
    "..#.#..",
    ".#...#.",
    "..#.#..",
    ".......",
    ".......",
}}};

static const BpBitmap FLOOD_BM = {{ /* F */
    ".......",
    "...#...",
    "..####.",
    ".#...#.",
    ".......",
    ".......",
    ".......",
}}};

static const BpBitmap ROUTE_BM = {{ /* R */
    ".......",
    "#.....#",
    "..#.#..",
    ".#...#.",
    "......#",
    ".......",
    ".......",
}}};

static const BpBitmap KEY_BM = {{ /* K */
    ".......",
    "#.....#",
    "...##..",
    ".#..#..",
    "#.....#",
    ".......",
    ".......",
}}};

static const BpBitmap VERIFY_BM = {{ /* V */
    ".......",
    "#.....#",
    ".#...#.",
    "..#.#..",
    "...#...",
    ".......",
    ".......",
}}};

static const BpBitmap SIGN_BM = {{ /* ~ */
    ".......",
    "..#####",
    "......#",
    "..###..",
    ".#...#.",
    "#.....#",
    ".......",
}}};

static const BpBitmap HASH_BM = {{ /* # */
    ".......",
    "..####.",
    ".....#.",
    "...##..",
    ".##....",
    ".......",
    ".......",
}}};

#define N_GLYPHS 90

static BpDelta FN_OUT[64];
static BpDelta STRUCT_OUT[64];
static BpDelta DATA_OUT[64];
static BpDelta CONTRACT_OUT[64];
static BpDelta VAL_OUT[64];
static BpDelta MOD_OUT[64];
static BpDelta LET_OUT[64];
static BpDelta IF_OUT[64];
static BpDelta THEN_OUT[64];
static BpDelta ELSE_OUT[64];
static BpDelta WHILE_OUT[64];
static BpDelta MATCH_OUT[64];
static BpDelta TYPE_OUT[64];
static BpDelta THEOREM_OUT[64];
static BpDelta PROOF_OUT[64];
static BpDelta ENUM_OUT[64];
static BpDelta I64_OUT[64];
static BpDelta U64_OUT[64];
static BpDelta F64_OUT[64];
static BpDelta BOOL_OUT[64];
static BpDelta STR_OUT[64];
static BpDelta NAT_OUT[64];
static BpDelta VEC_OUT[64];
static BpDelta SPAWN_OUT[64];
static BpDelta AWAIT_OUT[64];
static BpDelta ADD_OUT[64];
static BpDelta SUB_OUT[64];
static BpDelta MUL_OUT[64];
static BpDelta DIV_OUT[64];
static BpDelta EQ_OUT[64];
static BpDelta AND_OUT[64];
static BpDelta OR_OUT[64];
static BpDelta NOT_OUT[64];
static BpDelta ARROW_OUT[64];
static BpDelta ASSIGN_OUT[64];
static BpDelta LT_OUT[64];
static BpDelta GT_OUT[64];
static BpDelta LE_OUT[64];
static BpDelta GE_OUT[64];
static BpDelta PIPE_OUT[64];
static BpDelta COLON_OUT[64];
static BpDelta COMMA_OUT[64];
static BpDelta SEMI_OUT[64];
static BpDelta UNIT_OUT[64];
static BpDelta LIST_OUT[64];
static BpDelta PAIR_OUT[64];
static BpDelta OPTION_OUT[64];
static BpDelta RESULT_OUT[64];
static BpDelta STREAM_OUT[64];
static BpDelta FNPTR_OUT[64];
static BpDelta CHECK_OUT[64];
static BpDelta INFER_OUT[64];
static BpDelta EVAL_OUT[64];
static BpDelta NORM_OUT[64];
static BpDelta REFL_OUT[64];
static BpDelta UNIVERSE_OUT[64];
static BpDelta EFFECT_OUT[64];
static BpDelta SESSION_OUT[64];
static BpDelta QUANTITY_OUT[64];
static BpDelta ONE_OUT[64];
static BpDelta MANY_OUT[64];
static BpDelta PURE_OUT[64];
static BpDelta IO_OUT[64];
static BpDelta STATE_OUT[64];
static BpDelta SEND_OUT[64];
static BpDelta RECV_OUT[64];
static BpDelta DUAL_OUT[64];
static BpDelta EXTERN_OUT[64];
static BpDelta COMPTIME_OUT[64];
static BpDelta ASM_OUT[64];

static BpGlyph GLYPHS[N_GLYPHS];
static int initialized = 0;

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
        &FN_BM,
        &STRUCT_BM,
        &DATA_BM,
        &CONTRACT_BM,
        &VAL_BM,
        &MOD_BM,
        &LET_BM,
        &IF_BM,
        &THEN_BM,
        &ELSE_BM,
        &WHILE_BM,
        &MATCH_BM,
        &TYPE_BM,
        &THEOREM_BM,
        &PROOF_BM,
        &ENUM_BM,
        &I64_BM,
        &U64_BM,
        &F64_BM,
        &BOOL_BM,
        &STR_BM,
        &NAT_BM,
        &VEC_BM,
        &SPAWN_BM,
        &AWAIT_BM,
        &ADD_BM,
        &SUB_BM,
        &MUL_BM,
        &DIV_BM,
        &EQ_BM,
        &AND_BM,
        &OR_BM,
        &NOT_BM,
        &ARROW_BM,
        &ASSIGN_BM,
        &LT_BM,
        &GT_BM,
        &LE_BM,
        &GE_BM,
        &PIPE_BM,
        &COLON_BM,
        &COMMA_BM,
        &SEMI_BM,
        &UNIT_BM,
        &LIST_BM,
        &PAIR_BM,
        &OPTION_BM,
        &RESULT_BM,
        &STREAM_BM,
        &FNPTR_BM,
        &CHECK_BM,
        &INFER_BM,
        &EVAL_BM,
        &NORM_BM,
        &REFL_BM,
        &UNIVERSE_BM,
        &EFFECT_BM,
        &SESSION_BM,
        &QUANTITY_BM,
        &ONE_BM,
        &MANY_BM,
        &PURE_BM,
        &IO_BM,
        &STATE_BM,
        &SEND_BM,
        &RECV_BM,
        &DUAL_BM,
        &EXTERN_BM,
        &COMPTIME_BM,
        &ASM_BM
    };
    static const char *names[N_GLYPHS] = {
        "fn",
        "struct",
        "data",
        "contract",
        "val",
        "mod",
        "let",
        "if",
        "then",
        "else",
        "while",
        "match",
        "type",
        "theorem",
        "proof",
        "enum",
        "i64",
        "u64",
        "f64",
        "bool",
        "str",
        "nat",
        "vec",
        "spawn",
        "await",
        "add",
        "sub",
        "mul",
        "div",
        "eq",
        "and",
        "or",
        "not",
        "arrow",
        "assign",
        "lt",
        "gt",
        "le",
        "ge",
        "pipe",
        "colon",
        "comma",
        "semi",
        "unit",
        "list",
        "pair",
        "option",
        "result",
        "stream",
        "fnptr",
        "check",
        "infer",
        "eval",
        "norm",
        "refl",
        "universe",
        "effect",
        "session",
        "quantity",
        "one",
        "many",
        "pure",
        "io",
        "state",
        "send",
        "recv",
        "dual",
        "extern",
        "comptime",
        "asm"
    };
    static const char *ph[N_GLYPHS] = {
        "→",
        "diamond ◇",
        "triangle △",
        "circle ⊙",
        "filled circle ◉",
        "square ◈",
        "≡",
        "?",
        "⇒",
        "∨",
        "∞",
        "⊕",
        "τ",
        "∎",
        "⊢",
        "⋃",
        "ℤ",
        "ℕ",
        "ℝ",
        "𝔹",
        "S",
        "N",
        "V",
        "≫",
        "⏸",
        "+",
        "−",
        "×",
        "÷",
        "=",
        "∧",
        "∨",
        "¬",
        "→",
        "≔",
        "<",
        ">",
        "≤",
        "≥",
        "|",
        ":",
        ",",
        ";",
        "()",
        "[]",
        "×",
        "?+",
        "✓✗",
        "~",
        "λ",
        "⊢",
        "⊣",
        "⇓",
        "↓",
        "≡r",
        "U",
        "⚡",
        "⇄",
        "ω",
        "1",
        "ω",
        "○",
        "⇢",
        "⧉",
        "⇑",
        "⇓",
        "⇿",
        "↗",
        "⏱",
        "⌬"
    };
    static BpDelta *outs[N_GLYPHS] = {
        FN_OUT,
        STRUCT_OUT,
        DATA_OUT,
        CONTRACT_OUT,
        VAL_OUT,
        MOD_OUT,
    HASH_OUT,
    SIGN_OUT,
    VERIFY_OUT,
    KEY_OUT,
    ROUTE_OUT,
    FLOOD_OUT,
    BUNDLE_OUT,
    INVOKE_OUT,
    KERNEL_OUT,
    DMA_OUT,
        LET_OUT,
        IF_OUT,
        THEN_OUT,
        ELSE_OUT,
        WHILE_OUT,
        MATCH_OUT,
        TYPE_OUT,
        THEOREM_OUT,
        PROOF_OUT,
        ENUM_OUT,
        I64_OUT,
        U64_OUT,
        F64_OUT,
        BOOL_OUT,
        STR_OUT,
        NAT_OUT,
        VEC_OUT,
        SPAWN_OUT,
        AWAIT_OUT,
        ADD_OUT,
        SUB_OUT,
        MUL_OUT,
        DIV_OUT,
        EQ_OUT,
        AND_OUT,
        OR_OUT,
        NOT_OUT,
        ARROW_OUT,
        ASSIGN_OUT,
        LT_OUT,
        GT_OUT,
        LE_OUT,
        GE_OUT,
        PIPE_OUT,
        COLON_OUT,
        COMMA_OUT,
        SEMI_OUT,
        UNIT_OUT,
        LIST_OUT,
        PAIR_OUT,
        OPTION_OUT,
        RESULT_OUT,
        STREAM_OUT,
        FNPTR_OUT,
        CHECK_OUT,
        INFER_OUT,
        EVAL_OUT,
        NORM_OUT,
        REFL_OUT,
        UNIVERSE_OUT,
        EFFECT_OUT,
        SESSION_OUT,
        QUANTITY_OUT,
        ONE_OUT,
        MANY_OUT,
        PURE_OUT,
        IO_OUT,
        STATE_OUT,
        SEND_OUT,
        RECV_OUT,
        DUAL_OUT,
        EXTERN_OUT,
        COMPTIME_OUT,
        ASM_OUT
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
