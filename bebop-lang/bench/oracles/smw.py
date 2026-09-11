#!/usr/bin/env python3
"""Oracle for smw (G10): P writer threads, disjoint partitions, cross-partition tx every 100th.
Deterministic fold computed by simulating the same writes sequentially.
Env: BEBOP_TMP (default /tmp/opencode).
Usage: python3 smw.py <P> <N>  -- prints fold as last line."""

import os, sys

M64 = (1 << 64) - 1

def s64(x):
    x &= M64
    return x - (1 << 64) if x >> 63 else x

def fnv64(buf):
    h = 0xcbf29ce484222325 & M64
    for b in buf:
        h = ((h ^ b) * 0x100000001b3) & M64
    return s64(h)


def simulate(P, N):
    """Simulate P writers each doing N commits, cross-partition every 100th.
    Returns the fold (FNV-64 of the final store bytes)."""
    # Arena: 256MB = 33554432 cells. Region per partition = arena / P.
    arena_cells = 33554432
    region_size = arena_cells // P
    # Each partition has its own cursor, root, gen
    cursors = [1024 + p * region_size for p in range(P)]
    roots = [0] * P          # partition roots (counter objects)
    gens = [0] * P
    global_gen = 0
    # Counter objects: each partition has one counter object (4 cells payload)
    # object layout: h0 = (digest << 32) | len, h1 = (crc << 32) | gen, payload[4]
    # Counter starts at 0, incremented by 1 each commit
    counter_vals = [0] * P
    # For cross-partition tx (every 100th): increment ALL partitions
    cross = [0] * P

    # Compute layout digests (low 32 bits of sha256 of type string)
    # "C{i64}" -> digest for counter object
    import hashlib
    def digest(s):
        return int(hashlib.sha256(s.encode()).hexdigest()[:8], 16) & 0xFFFFFFFF

    dc = digest("C{i64}")      # counter digest
    dr = digest("R{arr ref C}")  # root digest

    # We'll build the final store as a byte buffer
    # Store format: page 0 = superblock A (cells 0..15), page 1 = superblock B (cells 512..527)
    # cells 1024.. = arena objects
    # Each cell = 8 bytes LE

    # We need to compute the FINAL state after all commits.
    # Since commits are sequential in the oracle, we just track the final values.

    # After N commits per partition (with cross every 100th):
    # Each partition's counter = N + (N // 100) * (P - 1)  [cross tx increments all P partitions]
    # Actually: every 100th commit is cross-partition, meaning all P partitions increment.
    # So total increments per partition = N (own) + N//100 (cross) 
    # But cross commits happen at multiples of 100, and each cross commit increments ALL partitions.
    # So if writer p does commits 0..N-1, at commit i where i % 100 == 99 (every 100th),
    # it's a cross-partition tx that increments all partitions.
    # Total cross commits = N // 100
    # Each partition gets: N (from its own writer) + N//100 (from cross commits) increments

    total_cross = N // 100
    for p in range(P):
        counter_vals[p] = N + total_cross

    # Now build the store image.
    # We need to create a valid store with:
    # - Two superblocks (A at 0, B at 512), one valid with higher gen
    # - PartTab object pointing to partition roots
    # - Counter objects for each partition

    # Superblock cells (16 cells = 128 bytes):
    # 0: magic (3554557610294396226)
    # 1: version (1)
    # 2: generation
    # 3: root (PartTab offset)
    # 4: arena_used (max cursor)
    # 5: layout_table (0)
    # 6: migration_table (0)
    # 7: live_cells
    # 8: superseded_cells
    # 9-14: zero
    # 15: crc32 of cells 0..14

    # Final global gen = P * N + P * (N // 100) = P * (N + N//100)
    # But actually each commit bumps global gen by 1, so:
    # Total commits = P * N (each writer commits N times, including cross)
    # Wait: cross commits are PART OF the N commits, not additional.
    # Each writer does N commits total. Every 100th is cross-partition.
    # So total commits = P * N
    # Global gen = P * N (starting from 0)

    global_gen = P * N

    # Arena used = max cursor across all partitions
    # Each partition's cursor starts at 1024 + p * region_size
    # Each commit allocates: 1 counter object (4 payload + 2 header = 6 cells) + 1 root object (4 payload + 2 header = 6 cells)
    # Actually for single-partition: allocate counter (6 cells) + new root (6 cells) = 12 cells per commit
    # For cross-partition: allocate counter for EACH partition (6 cells each) + new root for each (6 cells each) = 12*P cells
    # But in our simulation, each "commit" in the .bp program does one st_commit_p which allocates
    # a PartTab (16 + 3*P cells) + counter (6 cells) + root (6 cells) = 28 + 3*P cells per commit

    # Wait, let me re-think. In the .bp program:
    # - st_begin_p: starts tx on partition p
    # - st_alloc: allocates counter object (6 cells: 2 header + 4 payload)
    # - st_alloc: allocates new root object (6 cells)  
    # - st_commit_p: allocates PartTab (16 + 3*P cells), writes superblock
    # Total per commit: 6 + 6 + (16 + 3*P) = 28 + 3*P cells

    # But for cross-partition (every 100th), we do P separate st_commit_p calls?
    # Or one st_commit_2pc? The blueprint says "cross-partition tx every 100th".
    # For step 2, we use st_commit_p for each partition sequentially (since we're testing
    # the basic multi-writer, not 2PC yet). Actually, for cross-partition, we need atomicity.
    # Let's use st_commit_2pc for cross-partition commits.

    # For simplicity in the oracle, let's compute the final arena layout:
    # Each single-partition commit (non-cross): 
    #   - counter: 6 cells
    #   - root: 6 cells  
    #   - PartTab: 16 + 3*P cells
    #   Total: 28 + 3*P cells in partition p's region
    #
    # Each cross-partition commit (every 100th):
    #   - P counters: 6*P cells
    #   - P roots: 6*P cells
    #   - PartTab: 16 + 3*P cells (allocated once)
    #   Total: 16 + 15*P cells (distributed across all partitions)
    #
    # Non-cross commits per partition: N - N//100
    # Cross commits: N//100 (each affects all P partitions)

    non_cross = N - N // 100
    cross_commits = N // 100

    # Arena cells used per partition:
    # From single-partition commits: non_cross * (28 + 3*P)  [counter + root + PartTab]
    # From cross commits: cross_commits * 6 * P  [just counter + root, PartTab allocated in partition 0]
    # Actually for cross, PartTab is allocated once, and it could be in any partition.
    # Let's say PartTab for cross is in partition 0.

    cells_per_single = 28 + 3 * P  # counter(6) + root(6) + PartTab(16+3P)
    cells_per_cross_counter_root = 6  # per partition per cross commit

    # Total arena used (sum across all partitions):
    # Partition 0: non_cross * cells_per_single + cross_commits * (cells_per_single + (P-1)*cells_per_cross_counter_root)
    #   Wait, for cross commits, partition 0 allocates: its own counter(6) + root(6) + PartTab(16+3P) = cells_per_single
    #   Plus it hosts the PartTab for cross commits.
    # Partitions 1..P-1: non_cross * cells_per_single + cross_commits * cells_per_cross_counter_root

    # Actually this is getting complex. Let me simplify: just track final cursor positions.
    # For the fold, we just need the final store bytes. Let's build it directly.

    # Simpler approach: build the store as a sequence of allocations.
    # We'll simulate all allocations in order and produce the final byte image.

    # Start with fresh store
    total_arena = arena_cells
    store = bytearray(total_arena * 8)  # 8 bytes per cell

    def write_cell(off, val):
        """Write i64 LE at cell offset"""
        off_bytes = off * 8
        store[off_bytes:off_bytes+8] = val.to_bytes(8, 'little', signed=True)

    def crc32_cells(off, n):
        """CRC32 of n cells starting at off"""
        import zlib
        data = store[off*8:(off+n)*8]
        return zlib.crc32(data) & 0xFFFFFFFF

    def write_superblock(sb_off, gen, root, used, live, sup):
        """Write a superblock at sb_off (0 or 512)"""
        write_cell(sb_off + 0, 3554557610294396226)  # magic
        write_cell(sb_off + 1, 1)  # version
        write_cell(sb_off + 2, gen)
        write_cell(sb_off + 3, root)
        write_cell(sb_off + 4, used)
        write_cell(sb_off + 5, 0)
        write_cell(sb_off + 6, 0)
        write_cell(sb_off + 7, live)
        write_cell(sb_off + 8, sup)
        for k in range(9, 15):
            write_cell(sb_off + k, 0)
        write_cell(sb_off + 15, crc32_cells(sb_off, 15))

    def write_object(off, digest, length, gen, payload=None):
        """Write an object at off, return off. payload = list of i64."""
        write_cell(off, (digest << 32) | length)
        write_cell(off + 1, gen)
        if payload:
            for i, v in enumerate(payload):
                write_cell(off + 2 + i, v)
        # crc
        crc = crc32_cells(off + 2, length)
        write_cell(off + 1, (crc << 32) | (gen & 0xFFFFFFFF))
        return off

    # Simulate all allocations
    # Cursor per partition
    curs = [1024 + p * region_size for p in range(P)]
    pt_cursor = {}  # partition -> next PartTab offset (for simulation)

    # We'll simulate in writer order: writer 0 does all its commits, then writer 1, etc.
    # (The oracle is sequential; the .bp runs them in parallel but the final state is the same
    #  since partitions are disjoint.)

    # Actually, the order matters for absolute cursor positions but NOT for the final
    # counter values. The fold is based on the final store content. Since each partition
    # writes to its own region, the final state is deterministic regardless of interleaving.

    # For the PartTab and superblock, the LAST commit wins. Since we simulate sequentially,
    # the last writer's final commit determines the final superblock/PartTab.

    # Let's just compute the final state after all commits.

    # Final counter values (already computed above)
    # Final cursors: start + total allocations per partition
    alloc_per_single = 28 + 3 * P  # counter(6) + root(6) + PartTab(16+3P) 
    alloc_per_cross = 6  # just counter + root for this partition

    final_cursors = []
    for p in range(P):
        c = 1024 + p * region_size
        c += non_cross * alloc_per_single
        c += cross_commits * alloc_per_cross
        final_cursors.append(c)

    max_cursor = max(final_cursors)

    # Final global gen
    final_gen = P * N

    # Build final store image
    # First, clear everything
    for i in range(total_arena):
        write_cell(i, 0)

    # Write superblock A (live) at 0
    # PartTab at some offset in partition 0's region
    pt_off = 1024  # PartTab goes at start of partition 0's region
    # Actually, PartTab is allocated during commits. The final PartTab is at the cursor
    # position after all allocations in partition 0. Let's compute:
    # Partition 0 allocations: non_cross * alloc_per_single + cross_commits * alloc_per_cross
    # But PartTab itself is part of those allocations. The LAST PartTab is at the end.
    # For simplicity, let's put the PartTab at the very end of partition 0's used region.
    # Actually, let's just compute where it lands.

    # Partition 0: first allocation is at 1024. After all allocations, cursor = final_cursors[0].
    # The last PartTab was allocated at some point. For the final state, we need the PartTab
    # that the superblock points to. Let's say the last commit's PartTab is at:
    # final_cursors[0] - (16 + 3*P)  [the last allocation in partition 0]

    # But wait, the last commit might be a cross-commit by a different writer. Let's just
    # assume the final PartTab is at a known offset. For the oracle, we'll place it at
    # the end of partition 0's region.

    # Actually, let's do a proper sequential simulation. It's simpler and guarantees correctness.

    # Reset and simulate properly
    for i in range(total_arena):
        write_cell(i, 0)

    curs = [1024 + p * region_size for p in range(P)]
    global_g = 0
    pt = 0  # current PartTab offset, 0 means none yet

    # Root objects per partition (current live root)
    live_roots = [0] * P
    # Counter objects per partition (current live counter)
    live_counters = [0] * P

    # Store the final fold contributions
    # We'll build the store incrementally

    # Helper: allocate in partition p
    def alloc(p, length, digest):
        off = curs[p]
        write_cell(off, (digest << 32) | length)
        write_cell(off + 1, global_g)  # generation
        curs[p] = off + 2 + length
        return off

    # Helper: seal object (compute crc)
    def seal(off, length):
        crc = crc32_cells(off + 2, length)
        cur = int.from_bytes(store[(off+1)*8:(off+2)*8], 'little', signed=True)
        write_cell(off + 1, (crc << 32) | (cur & 0xFFFFFFFF))

    # Helper: commit partition p with new root
    def commit_p(p, new_root, new_counter):
        nonlocal global_g, pt
        global_g += 1
        # Read old PartTab entries
        if pt != 0:
            old_roots = [int.from_bytes(store[(pt + 16 + q*3)*8:(pt + 16 + q*3 + 1)*8], 'little', signed=True) for q in range(P)]
            old_useds = [int.from_bytes(store[(pt + 16 + q*3 + 1)*8:(pt + 16 + q*3 + 2)*8], 'little', signed=True) for q in range(P)]
            old_gens = [int.from_bytes(store[(pt + 16 + q*3 + 2)*8:(pt + 16 + q*3 + 3)*8], 'little', signed=True) for q in range(P)]
        else:
            old_roots = [0] * P
            old_useds = [1024 + q * region_size for q in range(P)]
            old_gens = [0] * P

        # Update partition p
        old_roots[p] = new_root
        old_useds[p] = curs[p]
        old_gens[p] = global_g

        # Allocate new PartTab
        pt = alloc(p, 16 + 3 * P, digest("PartTab"))
        # Write PartTab
        for q in range(16):
            write_cell(pt + q, 0)  # placeholder, will copy from superblock
        # Copy superblock A into PartTab cells 0..15
        for q in range(16):
            write_cell(pt + q, int.from_bytes(store[q*8:(q+1)*8], 'little', signed=True))
        # Write partition entries
        for q in range(P):
            write_cell(pt + 16 + q*3, old_roots[q])
            write_cell(pt + 16 + q*3 + 1, old_useds[q])
            write_cell(pt + 16 + q*3 + 2, old_gens[q])
        # CRC
        write_cell(pt + 15, crc32_cells(pt, 16 + 3*P))

        # Toggle superblock: write to B (512) if A is live, or A (0) if B is live
        # For simplicity, always write to 512 (toggle from 0 to 512)
        # Actually, we alternate. Let's track which superblock is live.
        # For the oracle simulation, we'll just write the final superblock at 0.
        # The live superblock is the one with higher generation.

        # Write superblock B (512) as the new live one
        write_superblock(512, global_g, pt, curs[p], 0, 0)
        # Copy magic/version from A to make it valid
        # Actually, let's just write both properly. The "live" one is the one we just wrote.

        # For the final state, we need the superblock with the highest generation.
        # Let's write superblock A (0) as the final one.
        # We'll handle this at the end.

        return global_g

    # Now simulate all writers sequentially
    # Writer order: 0, 1, ..., P-1 (this is the oracle ordering)
    # Each writer does N commits, every 100th is cross-partition

    # We need to track which objects exist for the fold computation.
    # The fold is computed from the FINAL store state.
    # Let's just build the entire final store.

    # Reset
    for i in range(total_arena):
        write_cell(i, 0)

    curs = [1024 + p * region_size for p in range(P)]
    global_g = 0
    pt = 0
    live_roots = [0] * P
    live_counters = [0] * P

    # We'll record the sequence of (partition, counter_value) for the fold
    # The fold needs to capture the final state deterministically.

    # Actually, the simplest correct oracle: build the final store exactly as the .bp would,
    # then compute FNV-64 over the live superblock's entire mapping.

    # Let's do a proper sequential simulation where writer p does all its commits,
    # then writer p+1, etc. Within each writer, commits 0..N-1, with cross at every 100th.

    # Cross-partition: when writer p is at commit i and i % 100 == 99 (every 100th),
    # it does a cross-partition tx: increment all partitions' counters.
    # This is done via st_commit_2pc in the .bp, but in the oracle we just simulate it.

    # For cross-partition, all P partitions get a new counter and root.
    # The PartTab is updated once.

    # Let's simulate:

    for p in range(P):
        for i in range(N):
            is_cross = (i % 100 == 99)
            if is_cross:
                # Cross-partition: increment ALL partitions
                # Allocate new counter + root for each partition
                new_roots = [0] * P
                new_counters = [0] * P
                for q in range(P):
                    # Allocate counter
                    cnt_off = alloc(q, 4, digest("C{i64}"))
                    write_cell(cnt_off + 2, live_counters[q] + 1)  # new value
                    seal(cnt_off, 4)
                    new_counters[q] = cnt_off
                    # Allocate root
                    root_off = alloc(q, 5, digest("R{arr ref C}"))
                    write_cell(root_off + 2, 0)  # will link counters
                    # Link: root[1..4] = refs to counters
                    # For simplicity, just store the counter offset as a ref
                    write_cell(root_off + 2 + 1 + q, new_counters[q] - root_off)  # ref to counter
                    # Actually, root has 4 ref fields for 4 counters. For P partitions,
                    # each partition has its own root with 1 counter. Let's simplify:
                    # root object has 5 cells payload: [count, ref C0, ref C1, ref C2, ref C3]
                    # For P partitions, each root points to its own partition's counter.
                    # But we're using a single root per partition with 4 counter refs.
                    # Let's just do: root[1] = ref to counter, rest = 0
                    write_cell(root_off + 2, 1)  # count = 1
                    write_cell(root_off + 3, new_counters[q] - root_off)  # ref to counter
                    for j in range(4, 5):
                        write_cell(root_off + 2 + j, 0)
                    seal(root_off, 5)
                    new_roots[q] = root_off
                    live_counters[q] = new_counters[q]
                    live_roots[q] = new_roots[q]

                # Now commit via 2PC: update PartTab with all partitions
                global_g += 1
                if pt != 0:
                    old_roots = [int.from_bytes(store[(pt + 16 + q*3)*8:(pt + 16 + q*3 + 1)*8], 'little', signed=True) for q in range(P)]
                    old_useds = [int.from_bytes(store[(pt + 16 + q*3 + 1)*8:(pt + 16 + q*3 + 2)*8], 'little', signed=True) for q in range(P)]
                    old_gens = [int.from_bytes(store[(pt + 16 + q*3 + 2)*8:(pt + 16 + q*3 + 3)*8], 'little', signed=True) for q in range(P)]
                else:
                    old_roots = [0] * P
                    old_useds = [1024 + q * region_size for q in range(P)]
                    old_gens = [0] * P

                for q in range(P):
                    old_roots[q] = new_roots[q]
                    old_useds[q] = curs[q]
                    old_gens[q] = global_g

                pt = alloc(0, 16 + 3 * P, digest("PartTab"))
                for q in range(16):
                    write_cell(pt + q, int.from_bytes(store[q*8:(q+1)*8], 'little', signed=True))
                for q in range(P):
                    write_cell(pt + 16 + q*3, old_roots[q])
                    write_cell(pt + 16 + q*3 + 1, old_useds[q])
                    write_cell(pt + 16 + q*3 + 2, old_gens[q])
                write_cell(pt + 15, crc32_cells(pt, 16 + 3*P))

                # Toggle superblock
                write_superblock(512, global_g, pt, max(curs), 0, 0)
            else:
                # Single-partition commit: just increment partition p
                global_g += 1
                # Allocate counter
                cnt_off = alloc(p, 4, digest("C{i64}"))
                write_cell(cnt_off + 2, live_counters[p] + 1)
                seal(cnt_off, 4)
                new_counter = cnt_off
                # Allocate root
                root_off = alloc(p, 5, digest("R{arr ref C}"))
                write_cell(root_off + 2, 1)
                write_cell(root_off + 3, new_counter - root_off)
                seal(root_off, 5)
                new_root = root_off

                live_counters[p] = new_counter
                live_roots[p] = new_root

                # Commit
                if pt != 0:
                    old_roots = [int.from_bytes(store[(pt + 16 + q*3)*8:(pt + 16 + q*3 + 1)*8], 'little', signed=True) for q in range(P)]
                    old_useds = [int.from_bytes(store[(pt + 16 + q*3 + 1)*8:(pt + 16 + q*3 + 2)*8], 'little', signed=True) for q in range(P)]
                    old_gens = [int.from_bytes(store[(pt + 16 + q*3 + 2)*8:(pt + 16 + q*3 + 3)*8], 'little', signed=True) for q in range(P)]
                else:
                    old_roots = [0] * P
                    old_useds = [1024 + q * region_size for q in range(P)]
                    old_gens = [0] * P

                old_roots[p] = new_root
                old_useds[p] = curs[p]
                old_gens[p] = global_g

                pt = alloc(p, 16 + 3 * P, digest("PartTab"))
                for q in range(16):
                    write_cell(pt + q, int.from_bytes(store[q*8:(q+1)*8], 'little', signed=True))
                for q in range(P):
                    write_cell(pt + 16 + q*3, old_roots[q])
                    write_cell(pt + 16 + q*3 + 1, old_useds[q])
                    write_cell(pt + 16 + q*3 + 2, old_gens[q])
                write_cell(pt + 15, crc32_cells(pt, 16 + 3*P))

                write_superblock(512, global_g, pt, curs[p], 0, 0)

    # Now write the final superblock at 0 (the live one)
    # The live superblock is the one with the higher generation
    write_superblock(0, global_g, pt, max(curs), 0, 0)
    # Zero out superblock B
    for q in range(16):
        write_cell(512 + q, 0)

    # Compute fold: FNV-64 over the entire mapped region (arena_size cells)
    # The .bp program reads the whole mapping via st_cells base.
    # Fold = FNV-64 of the raw bytes of the mapping.
    # But the .bp's fold function is specific. Let's check what fold the .bp computes.

    # Actually, I need to write the .bp program first to know what fold it computes.
    # For now, let's return the store bytes for FNV computation.

    return store[:arena_cells * 8], global_g, pt, max(curs)


def main():
    if len(sys.argv) < 3:
        print("Usage: smw.py <P> <N>", file=sys.stderr)
        sys.exit(2)
    P = int(sys.argv[1])
    N = int(sys.argv[2])
    store_bytes, gen, pt, used = simulate(P, N)
    fold = fnv64(store_bytes)
    print(f"gen={gen} pt={pt} used={used} fold={fold}")
    print(fold)

if __name__ == "__main__":
    main()
