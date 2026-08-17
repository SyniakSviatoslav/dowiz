/* Bebop parser — recursive descent → lightweight item AST (Phase 1). */
#ifndef BEBOP_PARSER_H
#define BEBOP_PARSER_H

#include <stddef.h>

#include "typereg.h"

typedef enum {
    AST_ITEM_MODULE,
    AST_ITEM_FN,
    AST_ITEM_STRUCT,
    AST_ITEM_CONST,
    AST_ITEM_USE,
    AST_ITEM_TYPE,
    AST_ITEM_UNKNOWN,
} AstItemKind;

typedef struct {
    AstItemKind kind;
    const char *name; /* into the source buffer */
    size_t name_len;
    const char *text; /* full source span of the item */
    size_t text_len;
} AstItem;

typedef struct {
    AstItem *items;
    size_t len;
    size_t cap;
} AstProgram;

typedef struct {
    unsigned line;
    const char *msg;
} BpParseError;

/* Parse source into items. Returns 0 on success, -1 on error (`err` filled). */
int bp_parse(const char *src, AstProgram *prog, BpParseError *err);

/* Parse a struct declaration from raw source text (e.g. "Point { x: i64, y:
 * i64 }") into the type registry. Returns 0 on success, -1 on error. */
int bp_parse_struct_decl(const char *src, TyRegistry *reg, char *err,
                         size_t cap);

/* Parse a function definition (e.g. "ntt(a: i64) -> i64 { a + 1 }") produces
 * an elaborated core Term (lambda) + its Pi type. Returns 0 on success. */
int bp_parse_fn_decl(const char *src, TyRegistry *reg, Term **out,
                     Ty **out_ty, char *err, size_t cap);

void bp_program_free(AstProgram *prog);

const char *ast_item_kind_name(AstItemKind k);

#endif /* BEBOP_PARSER_H */
