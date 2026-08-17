/* Bebop theorem surface — `theorem <name> : <prop> := <proof>`.
 *
 * Layer 1 (parse): a lightweight recursive-descent over the proof fragment.
 * Layer 2 (elaborate): map the parsed proof into proof-kernel terms (refl /
 *   subst / rec_nat / lam / app) and check `proof : prop` with the kernel.
 * Layer 3 (register): a verified theorem is an invariant the codegen/SMT can
 *   rely on without rechecking.
 */
#ifndef BEBOP_THEOREM_H
#define BEBOP_THEOREM_H

#include <stddef.h>

/* Prove a theorem declaration. `decl` is the source text starting at the
 * `theorem` keyword (e.g. "theorem plus_one : (\\x.x+1) 4 = 5 := refl").
 * Returns 0 on success (out = result type), -1 on error (err filled). */
int theorem_prove(const char *decl, char *out, size_t cap, char *err,
                  size_t cap_err);

#endif /* BEBOP_THEOREM_H */
