/* rssrun -- run a child command, report its peak RSS (ru_maxrss) + wall ns. */
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
#include <time.h>
int main(int argc, char **argv) {
    if (argc < 2) return 2;
    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    pid_t p = fork();
    if (p == 0) { execvp(argv[1], &argv[1]); _exit(127); }
    int st; waitpid(p, &st, 0);
    clock_gettime(CLOCK_MONOTONIC, &t1);
    struct rusage ru; getrusage(RUSAGE_CHILDREN, &ru);
    double ms = (t1.tv_sec - t0.tv_sec) * 1e3 + (t1.tv_nsec - t0.tv_nsec) / 1e6;
    fprintf(stderr, "RSS_KB=%ld WALL_MS=%.1f STATUS=%d\n",
            ru.ru_maxrss, ms, WIFEXITED(st) ? WEXITSTATUS(st) : -1);
    return 0;
}
