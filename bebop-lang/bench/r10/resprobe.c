#include <stdio.h>
#include <time.h>
#include <stdint.h>
static inline uint64_t now(void){ struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts); return (uint64_t)ts.tv_sec*1000000000ull + ts.tv_nsec; }
static inline void nops(int n){ for(int i=0;i<n;i++) __asm__ volatile("nop"); }
static inline uint64_t cntvct(void){ uint64_t v; __asm__ volatile("mrs %0, cntvct_el0" : "=r"(v)); return v; }
int main(void){
    // 1) measure clock_gettime resolution: time 0..10 nops, see granularity
    printf("clock_gettime deltas for 200-nop op (20 samples):\n");
    for(int s=0;s<20;s++){ uint64_t a=now(); nops(200); uint64_t b=now(); printf("  %lld\n", (long long)(b-a)); }
    // 2) cntvct probe: frequency + granularity
    printf("cntvct deltas for 200-nop op (20 samples):\n");
    for(int s=0;s<20;s++){ uint64_t a=cntvct(); nops(200); uint64_t b=cntvct(); printf("  %lld\n", (long long)(b-a)); }
    // 3) cntvct freq estimate via 1 second sleep
    { uint64_t a=cntvct(); struct timespec ts={1,0}; nanosleep(&ts,0); uint64_t b=cntvct(); printf("cntvct freq ~ %.2f MHz\n", (double)(b-a)/1e6); }
    // 4) clock_gettime resolution via 1s
    { uint64_t a=now(); struct timespec ts={1,0}; nanosleep(&ts,0); uint64_t b=now(); printf("1s clock_gettime delta = %lld ns\n", (long long)(b-a)); }
    return 0;
}
