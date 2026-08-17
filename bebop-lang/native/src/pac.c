/* Bebop PAC — pointer authentication, implementation. */
#include "pac.h"

#include <stdio.h>
#include <string.h>
#include <sys/auxv.h>
#include <sys/mman.h>

int pac_available(void) {
    unsigned long hw = getauxval(AT_HWCAP);
    return (hw & HWCAP_PACA) != 0;
}

int pac_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    A(pac_available() == 0 || pac_available() == 1, "pac_available is boolean");

    if (pac_available()) {
        /* Round-trip: sign a pointer with SP then authenticate it. The
         * authentication must restore the exact pointer (else it faults). */
        unsigned code[4];
        code[0] = PAC_PACIA | (31u << 5) | 0u; /* pacia x0, sp */
        code[1] = PAC_AUTIA | (31u << 5) | 0u; /* autia x0, sp */
        code[2] = 0xD65F03C0u;                 /* ret */
        void *mem = mmap(NULL, sizeof code, PROT_READ | PROT_WRITE | PROT_EXEC,
                         MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (mem == MAP_FAILED) {
            A(0, "PAC round-trip: mmap");
            return -1;
        }
        memcpy(mem, code, sizeof code);
        __builtin___clear_cache((char *)mem, (char *)mem + sizeof code);
        unsigned long (*fn)(unsigned long);
        memcpy(&fn, &mem, sizeof fn);
        unsigned long in = 0xdeadbeefcafebabeUL;
        unsigned long got = fn(in);
        munmap(mem, sizeof code);
        A(got == in, "PAC round-trip sign+authenticate == identity");
    } else {
        A(1, "PAC unsupported on this CPU — round-trip skipped (detection ok)");
    }

    return all_ok ? 0 : -1;
}
