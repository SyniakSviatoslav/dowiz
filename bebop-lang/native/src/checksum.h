/* Bebop checksum — FNV-1a-style folding checksum (port of dowiz checksum.rs). */
#ifndef BEBOP_CHECKSUM_H
#define BEBOP_CHECKSUM_H

#include <stddef.h>
#include <stdint.h>

uint64_t checksum_fold(const uint8_t *data, size_t len);

int checksum_self_test(char *out, size_t cap);

#endif /* BEBOP_CHECKSUM_H */
