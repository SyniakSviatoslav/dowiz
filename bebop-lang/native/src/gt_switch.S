/* Bebop green threads — AArch64 context switch (hand-written asm).
 * gt_switch(from, to): save callee-saved regs to *from, restore from *to,
 * return on *to's saved link register. Layout matches GtContext (gt.h):
 *   x19..x28 @ 0..64, fp @ 80, lr @ 88, sp @ 96. */

    .text
    .global gt_switch
    .type   gt_switch, %function
gt_switch:
    stp x19, x20, [x0, #0]
    stp x21, x22, [x0, #16]
    stp x23, x24, [x0, #32]
    stp x25, x26, [x0, #48]
    stp x27, x28, [x0, #64]
    stp x29, x30, [x0, #80]
    mov x9, sp
    str x9, [x0, #96]
    ldp x19, x20, [x1, #0]
    ldp x21, x22, [x1, #16]
    ldp x23, x24, [x1, #32]
    ldp x25, x26, [x1, #48]
    ldp x27, x28, [x1, #64]
    ldp x29, x30, [x1, #80]
    ldr x9, [x1, #96]
    mov sp, x9
    ret
    .size   gt_switch, .-gt_switch

/* trampoline: x19 = fn, x20 = arg (callee-saved across the first switch). */
    .global gt_trampoline
    .type   gt_trampoline, %function
gt_trampoline:
    mov x0, x20
    blr x19
    b   gt_finish
    .size   gt_trampoline, .-gt_trampoline
