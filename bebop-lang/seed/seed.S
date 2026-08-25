/* seed.S v3 — FROZEN Bebop loader. Zero C. File-mmap variant.
 * Usage: seed <prog.bin>   (.bin words..., last 8 bytes = LE64 entry byte off)
 * openat -> lseek size -> mmap(PROT_RWX, MAP_PRIVATE) -> arm arena -> call.
 */
    .arch armv8-a
    .text
    .global _start
_start:
    ldr     x1, [sp, #16]
    cbz     x1, fail

    mov     x0, #-100          /* AT_FDCWD */
    mov     x2, #0             /* O_RDONLY */
    mov     x3, #0
    mov     x8, #56            /* openat */
    svc     #0
    cmp     x0, #0
    b.lt   f_open
    mov     x19, x0            /* fd */

    /* size = lseek(fd, 0, SEEK_END=2) */
    mov     x0, x19
    mov     x1, #0
    mov     x2, #2
    mov     x8, #62
    svc     #0
    cmp     x0, #16
    b.lt   f_read
    mov     x20, x0            /* file size */

    /* code = mmap(0, size, PROT_READ|WRITE=3, MAP_PRIVATE=2, fd, 0) */
    mov     x0, #0
    mov     x1, x20
    mov     x2, #3
    mov     x3, #2
    mov     x4, x19            /* fd */
    mov     x5, #0
    mov     x8, #222
    svc     #0
    cmp     x0, #0
    b.lt   fail
    mov     x21, x0            /* code base */

    /* mprotect(base, round_up(size), RWX=7) */
    mov     x0, x21
    add     x1, x20, #4095
    and     x1, x1, #-4096
    mov     x2, #7
    mov     x8, #226
    svc     #0

    /* entry_off = LE64 at base[size-8] */
    sub     x9, x20, #8
    add     x10, x21, x9
    ldr     x12, [x10]

    /* arena x27/x28 = mmap(64MB, RW, PRIVATE|ANON) */
    mov     x0, #0
    mov     x1, #4096
    movk   x1, #1024, lsl #16  /* 64 MB */
    mov     x2, #3
    mov     x3, #34
    mov     x4, #-1
    mov     x5, #0
    mov     x8, #222
    svc     #0
    cmp     x0, #0
    b.lt   f_mmap
    mov     x27, x0
    add     x28, x0, x1

    add     x15, x21, x12      /* entry */
    blr     x15

    /* print signed decimal of x0 */
    mov     x22, x0
    sub     sp, sp, #48
    add     x5, sp, #48
    mov     x6, #0
    cbnz    x22, 2f
    sub     x5, x5, #1
    mov     w7, #48
    strb    w7, [x5]
    b       3f
2:  cmp     x22, #0
    b.ge    1f
    mov     w6, #1
    neg     x22, x22
1:  mov     x7, #10
    udiv    x8, x22, x7
    msub    x22, x8, x7, x22
    sub     x5, x5, #1
    add     w7, w22, #48
    strb    w7, [x5]
    mov     x22, x8
    cbnz    x22, 1b
    cbz     x6, 3f
    sub     x5, x5, #1
    mov     w7, #45
    strb    w7, [x5]
3:  mov     x0, #1
    mov     x1, x5
    add     x2, sp, #48
    sub     x2, x2, x5
    mov     x8, #64
    svc     #0
    mov     w0, #10
    strb    w0, [sp, #-1]
    mov     x1, sp
    sub     x1, x1, #1
    mov     x0, #1
    mov     x2, #1
    mov     x8, #64
    svc     #0
    mov     x0, #0
    mov     x8, #93
    svc     #0
f_open:
    mov     x0, #90
    b       xexit
f_read:
    mov     x0, #91
    b       xexit
f_mmap:
    mov     x0, #92
xexit:
    mov     x8, #93
    svc     #0
fail:
    mov     x0, #94
    mov     x8, #93
    svc     #0
