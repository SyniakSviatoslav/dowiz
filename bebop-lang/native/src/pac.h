/* Bebop PAC — pointer authentication (16B / #51).
 *
 * ARMv8.3-A Pointer Authentication: the backend signs the return address on
 * entry (paciasp) and authenticates before ret (autiasp), which turns a
 * classic ROP/JOP gadget chain into a fault — any corrupted return address
 * fails the authentication and traps. Feature-detected at runtime via
 * HWCAP_PACA; on CPUs without PAC the instructions are simply not emitted
 * (graceful fallback, still ROP-mitigated by W^X + fixed frames).
 */
#ifndef BEBOP_PAC_H
#define BEBOP_PAC_H

#include <stddef.h>
#include <stdint.h>

/* PAC instruction encodings (assembled once from ARMv8.3-A; the backend
 * emits these bytes directly — no assembler, no compiler at runtime). */
#define PAC_PACIASP 0xD503233Fu /* paciasp — sign LR with SP */
#define PAC_AUTIASP 0xD50323BFu /* autiasp — authenticate LR */
#define PAC_PACIA   0xDAC10000u /* pacia xd, xn  — sign a pointer (+(Rn<<5)+Rd) */
#define PAC_AUTIA   0xDAC11000u /* autia xd, xn  — authenticate (+(Rn<<5)+Rd) */
#define PAC_XPACLRI 0xD50320FFu /* xpaclri — strip the PAC */

/* 1 if the CPU supports address authentication (HWCAP_PACA). */
int pac_available(void);

/* Run the PAC self-test. On CPUs without PAC the runtime round-trip is
 * skipped (the detection + encoding table are still verified). */
int pac_self_test(char *out, size_t cap);

#endif /* BEBOP_PAC_H */
