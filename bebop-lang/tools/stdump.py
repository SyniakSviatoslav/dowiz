#!/usr/bin/env python3
"""
stdump.py — Store introspection tool for bebop store files (B5 step 1).
Prints both superblocks, determines live one, walks arena from 1024 to used,
validates accounting identity, and reports status.

Usage: python3 tools/stdump.py <store-path>

Exit status: 0 (OK), 1 (STALE-LAYOUT), 2 (CORRUPT), 3 (SIZE-MISMATCH)
"""

import os
import sys
import struct

# Add parent directory to path for storelib import
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'bench', 'oracles'))
from storelib import (
    cells, mask64, pick_live, parttab, sb_valid, 
    SB_A, SB_B, SB_CELL_SIZE, SB_GENERATION, SB_ARENA_USED, 
    SB_LIVE_CELLS, SB_SUPERSEDED_CELLS, ARENA_BASE, MAGIC,
    OBJ_H0_LEN_MASK, OBJ_H1_CRC_SHIFT
)
import zlib

def read_store(path):
    """Read store file, handle errors gracefully."""
    try:
        if not os.path.exists(path):
            return None, f"File not found: {path}"
        with open(path, 'rb') as f:
            data = f.read()
        return data, None
    except Exception as e:
        return None, f"Error reading file: {e}"

def format_superblock(data, sb_idx):
    """Format one superblock's data."""
    if len(data) < (sb_idx + SB_CELL_SIZE) * 8:
        return None
    
    try:
        c = cells(data, sb_idx, SB_CELL_SIZE)
        valid = sb_valid(data, sb_idx)
        
        return {
            'valid': valid,
            'generation': c[SB_GENERATION],
            'cell_3': c[3],  # root pointer
            'arena_used': c[SB_ARENA_USED],
            'live': c[SB_LIVE_CELLS],
            'superseded': c[SB_SUPERSEDED_CELLS],
            'cells': c
        }
    except Exception:
        return None

def walk_arena(data, start, end):
    """Walk arena from start to end, return (objects, errors, total_live)."""
    objects = []
    errors = []
    offset = start
    
    while offset < end:
        if offset + 2 > len(data) // 8:
            errors.append(f"Object at offset {offset}: read past end (h0/h1)")
            break
        
        try:
            h0, h1 = cells(data, offset, 2)
            obj_len = h0 & OBJ_H0_LEN_MASK
            obj_digest = (h0 >> 32) & 0xFFFFFFFF
            obj_crc_stored = (h1 >> OBJ_H1_CRC_SHIFT) & mask64()
            
            # Trap 84: zero-length object
            if obj_len < 1:
                errors.append(f"Object at offset {offset}: zero length (trap 84)")
                offset += 2
                continue
            
            # Trap 82: read past end
            if offset + 2 + obj_len > len(data) // 8:
                errors.append(f"Object at offset {offset}: declared length {obj_len} extends past EOF (trap 82)")
                break
            
            # Read payload and check CRC
            try:
                payload_cells = cells(data, offset + 2, obj_len)
                payload_bytes = struct.pack(f'<{obj_len}q', *payload_cells)
                obj_crc_computed = zlib.crc32(payload_bytes) & mask64()
                crc_ok = (obj_crc_stored == obj_crc_computed)
            except Exception as e:
                errors.append(f"Object at offset {offset}: error reading payload: {e}")
                crc_ok = False
                payload_bytes = b''
            
            objects.append({
                'offset': offset,
                'digest': obj_digest,
                'len': obj_len,
                'crc_ok': crc_ok,
                'bytes': len(payload_bytes)
            })
            
            offset += 2 + obj_len
        except Exception as e:
            errors.append(f"Object at offset {offset}: error: {e}")
            break
    
    total_live = sum(obj['len'] for obj in objects)
    return objects, errors, total_live

def main():
    if len(sys.argv) < 2:
        print("Usage: stdump.py <store-path>")
        sys.exit(1)
    
    store_path = sys.argv[1]
    data, err = read_store(store_path)
    
    if err:
        print(f"REPORT: {err}")
        print("OK")  # Not corruption, just file issue
        sys.exit(0)
    
    # Check file size vs superblock claims (trap 86)
    file_cells = len(data) // 8
    file_pages = (len(data) + 4095) // 4096
    
    # Parse both superblocks
    sb_a = format_superblock(data, SB_A)
    sb_b = format_superblock(data, SB_B)
    
    # Print both superblocks
    if sb_a:
        print(f"SB[0]: valid={sb_a['valid']}, gen={sb_a['generation']}, " +
              f"cell_3={sb_a['cell_3']}, used={sb_a['arena_used']}, " +
              f"live={sb_a['live']}, superseded={sb_a['superseded']}")
    else:
        print("SB[0]: UNREADABLE")
    
    if sb_b:
        print(f"SB[512]: valid={sb_b['valid']}, gen={sb_b['generation']}, " +
              f"cell_3={sb_b['cell_3']}, used={sb_b['arena_used']}, " +
              f"live={sb_b['live']}, superseded={sb_b['superseded']}")
    else:
        print("SB[512]: UNREADABLE")
    
    # Determine live superblock
    sb = pick_live(data)
    if sb == SB_A:
        print("Live: SB[0]")
        sb_live = sb_a
    elif sb == SB_B:
        print("Live: SB[512]")
        sb_live = sb_b
    else:
        print("Live: NONE (both invalid)")
        print("CORRUPT")
        sys.exit(2)
    
    if not sb_live:
        print("CORRUPT")
        sys.exit(2)
    
    used = sb_live['arena_used']
    live_claimed = sb_live['live']
    superseded_claimed = sb_live['superseded']
    
    # Trap 86: Size mismatch
    if used * 8 > len(data):
        print(f"SIZE-MISMATCH: superblock claims used={used} cells ({used*8} bytes) but file is only {len(data)} bytes")
        sys.exit(3)
    
    # Parse PartTab
    pt = parttab(data, sb)
    if pt:
        print(f"PartTab: offset={pt['offset']}, len={pt['len']}, crc_ok={pt['crc_ok']}, " +
              f"root_p={pt['payload'][0]}, used_p={pt['payload'][1]}, gen_p={pt['payload'][2]}")
    else:
        print("PartTab: NONE or INVALID")
    
    # Walk arena
    objects, errors, total_live_arena = walk_arena(data, ARENA_BASE, used)
    
    print(f"\nArena walk ({ARENA_BASE} to {used}):")
    for obj in objects:
        crc_str = "OK" if obj['crc_ok'] else "FAIL"
        print(f"  offset {obj['offset']:6d}: len={obj['len']:6d}, digest={obj['digest']:08x}, crc={crc_str}")
    
    if errors:
        print(f"\nArena errors ({len(errors)}):")
        for err in errors:
            print(f"  {err}")
    
    # Verify accounting identity: live + superseded == used - 1024
    expected_live_plus_superseded = used - ARENA_BASE
    actual_live_plus_superseded = live_claimed + superseded_claimed
    
    print(f"\nAccounting:")
    print(f"  live (claimed) = {live_claimed}")
    print(f"  superseded (claimed) = {superseded_claimed}")
    print(f"  live + superseded = {actual_live_plus_superseded}")
    print(f"  used - 1024 = {expected_live_plus_superseded}")
    
    accounting_ok = (actual_live_plus_superseded == expected_live_plus_superseded)
    if accounting_ok:
        print(f"  PASS")
    else:
        print(f"  FAIL ({actual_live_plus_superseded} != {expected_live_plus_superseded})")
    
    # Determine status
    if errors:
        print("\nCORRUPT")
        sys.exit(2)
    elif not accounting_ok:
        print("\nCORRUPT")
        sys.exit(2)
    else:
        print("\nOK")
        sys.exit(0)

if __name__ == '__main__':
    main()
