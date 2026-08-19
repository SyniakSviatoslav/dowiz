#include <stdio.h>
#include <time.h>
#include <stdint.h>
static inline uint64_t now(void){ struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts); return (uint64_t)ts.tv_sec*1000000000ull + ts.tv_nsec; }
int main(void){
    // measure empty-loop delta: consecutive clock reads
    uint64_t a=now(); for(int i=0;i<100000;i++){ volatile uint64_t x=now(); (void)x; } uint64_t b=now();
    printf("100k clock reads: %llu ns -> ~%.1f ns/read\n", (unsigned long long)(b-a), (double)(b-a)/100000.0);
    // distribution of single read deltas
    uint64_t prev=now();
    int hist[64]={0};
    for(int i=0;i<20000;i++){ uint64_t c=now(); int64_t d=(int64_t)c-(int64_t)prev; prev=c; if(d>=0&&d<64)hist[d]++; else if(d>=64)hist[63]++; }
    printf("delta histogram (ns):\n");
    for(int i=0;i<64;i++) if(hist[i]) printf("  %d ns: %d\n", i, hist[i]);
    return 0;
}
