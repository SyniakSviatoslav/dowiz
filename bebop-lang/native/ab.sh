#!/bin/bash
# Honest A/B benchmark: interleave runs of both binaries, report best-of-N per line.
# Usage: ab.sh N A_CMD B_CMD   (commands run via bash -c)
set -u
N="$1"; shift
A="$1"; shift
B="$1"; shift

declare -A best_a best_b

# parse a bench output file -> "name|size|ns" triples (size may be empty)
parse() {
    local f="$1" line name s2 ns key
    while IFS= read -r line; do
        name=$(echo "$line" | awk '{print $1}')
        s2=$(echo "$line" | awk '{print $2}')
        ns=$(echo "$line" | awk '{print $3}')
        [ -z "$ns" ] && continue
        if [[ "$ns" =~ ^[0-9.]+$ ]]; then
            key="$name"
            [[ "$s2" == n=* ]] && key="$name $s2"
            printf '%s|%s\n' "$key" "$ns"
        fi
    done < "$f"
}

for round in $(seq 1 "$N"); do
    bash -c "$A" > /tmp/ab_a.txt 2>&1
    bash -c "$B" > /tmp/ab_b.txt 2>&1
    while IFS='|' read -r name ns; do
        cur="${best_a[$name]:-999999999999}"
        ok=$(awk -v a="$ns" -v b="$cur" 'BEGIN{print (a+0 < b+0) ? 1 : 0}')
        [ "$ok" = "1" ] && best_a[$name]="$ns"
    done < <(parse /tmp/ab_a.txt)
    while IFS='|' read -r name ns; do
        cur="${best_b[$name]:-999999999999}"
        ok=$(awk -v a="$ns" -v b="$cur" 'BEGIN{print(a+0 < b+0) ? 1 : 0}')
        [ "$ok" = "1" ] && best_b[$name]="$ns"
    done < <(parse /tmp/ab_b.txt)
done

printf "%-30s %12s %12s %8s\n" "primitive" "Bebop(ns)" "Rust(ns)" "B/R"
for name in $(printf '%s\n' "${!best_a[@]}" "${!best_b[@]}" | sort -u); do
    a="${best_a[$name]:-0}"
    b="${best_b[$name]:-0}"
    if [ "$b" = "0" ]; then
        printf "%-30s %12s %12s %8s\n" "$name" "$a" "-" "-"
        continue
    fi
    ratio=$(awk -v a="$a" -v b="$b" 'BEGIN{printf "%.2f", a/b}')
    printf "%-30s %12s %12s %8s\n" "$name" "$a" "$b" "$ratio"
done
