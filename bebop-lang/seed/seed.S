/* seed.S v2 — FROZEN Bebop loader, zero C, zero statics.
 * All buffers runtime-mmap'd. Usage: seed <prog.bin>
 * .bin contract: words..., last 8 bytes = LE64 entry byte offset.
 */
    .arch armv8-a
    .text
    .global _start
_start:
    ldr     x1, [sp, #16]      /* argv[1] */
    cbz     x1, fail

    /* fd = openat(AT_FDCWD, path, O_RDONLY, 0) */
    mov     x0, #-100          /* AT_FDCWD */
    mov     x2, #0             /* O_RDONLY */
    mov     x3, #0
    mov     x8, #56            /* openat */
    svc     #0
    cmp     x0, #0
    b.lt   f_open
    mov     x19, x0            /* fd */

    /* code = mmap(0, 65536, RW, PRIVATE|ANON, -1, 0) */
    mov     x0, #0
    mov     x1, #65536
    mov     x2, #3
    mov     x3, #34
    mov     x4, #-1
    mov     x5, #0
    mov     x8, #222
    svc     #0
    cmp     x0, #0
    b.lt   fail
    mov     x20, x0            /* code base */

    /* n = read(fd, code, 65536) */
    mov     x0, x19
    mov     x1, x20
    mov     x2, #65536
    mov     x8, #63
    svc     #0
    cmp     x0, #16
    b.lt   f_read
    mov     x21, x0            /* total bytes */

    /* entry_off = LE64 at code[n-8] */
    sub     x9, x21, #8
    add     x10, x20, x9
    ldr     x12, [x10]

    /* mprotect(code, round_up(n), RWX=7) */
    mov     x0, x20
    add     x1, x21, #4095
    and     x1, x1, #-4096
    mov     x2, #7
    mov     x8, #226
    svc     #0

    /* arena x27/x28 = mmap(64MB, RW, ANON) */
    mov     x0, #0
    mov     x1, #4096
    movk   x1, #1024, lsl #16
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

    /* call code + entry_off ; result -> x0 */
    add     x15, x20, x12
    blr     x15

    /* save result, print signed decimal */
    mov     x22, x0
    sub     sp, sp, #48
    add     x5, sp, #48        /* one past end */
    mov     x6, #0             /* neg */
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
3:  mov     x0, #1             /* write(1, x5, sp+48-x5) */
    mov     x1, x5
    add     x2, sp, #48
    sub     x2, x2, x5
    mov     x8, #64
    svc     #0
    mov     w0, #10            /* newline */
    sub     x1, sp, #1         /* below our frame is fine for 1 byte? use red zone */
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
