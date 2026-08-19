/* Bebop CoreMark — a faithful reimplementation of the EEMBC CoreMark 1.0
 * algorithm suite (list processing, matrix operations, state machine, CRC-16)
 * for external comparability (checklist §12).
 *
 * This is an independent reimplementation of the published CoreMark
 * algorithms, NOT the official EEMBC tarball — so scores are comparable in
 * methodology but not bit-identical to an official CoreMark certificate run.
 * Score reported as iterations/sec and iterations/MHz (CoreMark/MHz).
 */
#ifndef BEBOP_COREMARK_H
#define BEBOP_COREMARK_H
#include <stddef.h>

int coremark_run(void);
int coremark_self_test(char *out, size_t cap);
#endif
