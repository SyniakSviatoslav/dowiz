/* mkimage: wrap a compiled Bebop word-stream into a flat aarch64 kernel
 * image (QEMU -M raspi3b -kernel). Stub: load SP, bl main, park on wfe. */
#include <stdio.h>
#include <stdlib.h>
int main(int argc, char **argv) {
    if (argc < 4) { fprintf(stderr, "usage: mkimage words.txt entry out.img\n"); return 2; }
    FILE *f = fopen(argv[1], "rb");
    if (!f) { perror("open"); return 2; }
    long cnt; if (fscanf(f, "%ld", &cnt) != 1) return 2;
    unsigned int *w = malloc((cnt + 8) * 4);
    for (long i = 0; i < cnt; i++)
        if (fscanf(f, "%ld", (long *)&w[i]) != 1) return 2;
    fclose(f);
    long entry = atol(argv[2]);
    /* stub (6 words): movz x9,#0 ; movk x9,#16,lsl#16 ; mov sp,x9 ;
       bl <entry> ; wfe ; b .-1  -- image base 0x80000, sp top 0x100000 */
    unsigned int img[16];
    long pc = 0;
    #define MOVZ(rd, imm) (3531603968u + (unsigned)(imm)*32u + (unsigned)(rd))
    #define MOVK(rd, imm, hw) (4068474880u + (unsigned)(imm)*32u + (unsigned)(hw)*2097152u + (unsigned)(rd))
    #define MOVSPTMP 0
    img[pc++] = MOVZ(9, 0x8000);
    img[pc++] = MOVK(9, 0x0048, 1);                       /* x9 = 0x48000000 */
    img[pc++] = 2852127712u + 9u*65536u + 31u;            /* mov sp,x9 */
    long off = entry - pc - 1;
    img[pc++] = (unsigned)(2483027968L + off + (off < 0 ? 67108864L : 0)); /* bl main */
    img[pc++] = MOVZ(11, 0xF000);
    img[pc++] = MOVK(11, 0x47FF, 1);                      /* x11 = 0x47FFF000 */
    img[pc++] = 4177527776u + 11u*32u + 0u;               /* str x0,[x11] */
    img[pc++] = MOVZ(8, 0x1000);
    img[pc++] = MOVK(8, 0x0900, 1);                       /* x8 = 0x09000000 UART */
    img[pc++] = 1384120320u + 66u*32u + 10u;              /* movz w10,#'B' */
    img[pc++] = 956301312u + 8u*32u + 10u;                /* strb w10,[x8] */
    img[pc++] = 3573751839u;                              /* wfe */
    img[pc++] = 335544319u;                               /* b . */
FILE *o = fopen(argv[3], "wb");
    if (!o) { perror("out"); return 2; }
    for (long i = 0; i < pc; i++) fwrite(&img[i], 4, 1, o);
    for (long i = 0; i < cnt; i++) { unsigned int v = (unsigned int)w[i]; fwrite(&v, 4, 1, o); }
    fclose(o);
    fprintf(stderr, "image: %ld words + 6-word stub -> %s\n", cnt, argv[3]);
    return 0;
}
