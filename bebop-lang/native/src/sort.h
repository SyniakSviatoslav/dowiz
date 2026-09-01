/* Bebop sort — deterministic float-key sort (port of dowiz sort.rs). */
#ifndef BEBOP_SORT_H
#define BEBOP_SORT_H
#include <stddef.h>

void sort_f64_desc(double *items, size_t n);
void sort_f64_asc(double *items, size_t n);
int sort_self_test(char *out, size_t cap);

#endif /* BEBOP_SORT_H */
