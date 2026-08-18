
#include "ntt32.h"
#include <stdio.h>
#include <time.h>
int main() {
    enum { inner = 65536 };
    uint32_t a[5] = {1,2,3,4,5}, b[3] = {1,1,1}, out[7];
    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int k = 0; k < inner; k++) ntt32_convolve(a,5,b,3,out);
    clock_gettime(CLOCK_MONOTONIC, &t1);
    double ns = (t1.tv_sec-t0.tv_sec)*1e9 + (t1.tv_nsec-t0.tv_nsec);
    printf("ntt32_convolve n=7 sample: %.2f ns/op (inner=%d)
", ns/inner, inner);
    printf("verify: ");
    uint32_t ex[7] = {1,3,6,9,12,9,5};
    for (int i=0;i<7;i++) printf("%u ", out[i]==ex[i]?1:0);
    printf("
");
    return 0;
}
