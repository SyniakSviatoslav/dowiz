/* Bebop Morse — ITU codebook (point 12). Identifiers are written in Morse
 * (dot/dash), translatable back to ASCII. Density: variable-length prefix code,
 * ~2-3x shorter than ASCII for typical identifiers. */
#ifndef BEBOP_MORSE_H
#define BEBOP_MORSE_H

#include <stddef.h>

/* Encode ASCII text → Morse ('.'=dot, '-'=dash, letter sep=' ', word sep='/').
 * Returns 0 on success, -1 if a char has no Morse code. */
int bp_morse_encode(const char *text, char *out, size_t cap);

/* Decode Morse → lowercase ASCII ('.'/'-' codes, ' ' letter sep, '/' word sep).
 * Returns 0 on success, -1 on an unknown code. */
int bp_morse_decode(const char *morse, char *out, size_t cap);

#endif /* BEBOP_MORSE_H */
