"""
Centralized store introspection library for bebop oracles (B5 step 1).
Each constant and function is tagged with its source in selfhost/prelude/store.bp.
"""

import struct
import zlib

# === Constants ===

# st_magic() line 51 of selfhost/prelude/store.bp
MAGIC = 3554557610294396226

# Superblock constants (line 11-15 of store.bp)
SB_A = 0
SB_B = 512
SB_CELL_SIZE = 16  # Superblock is 16 cells

# Superblock field offsets within a 16-cell superblock (line 11-15 of store.bp)
SB_MAGIC = 0        # "BEBOPST1"
SB_VERSION = 1
SB_GENERATION = 2
SB_ROOT = 3         # Points to PartTab (B5 step 1)
SB_ARENA_USED = 4
SB_LAYOUT_TABLE = 5
SB_MIGRATION_TABLE = 6
SB_LIVE_CELLS = 7
SB_SUPERSEDED_CELLS = 8
SB_RESERVED = 9
SB_COMMIT_OBJ = 10  # C3
SB_HEADS_TABLE = 11  # C3
SB_CRC = 15

# PartTab constants (line 18-35 of store.bp)
PARTTAB_SIZE = 19  # 16-cell SB copy + 3-cell partition entry
PARTTAB_HEADER_SIZE = 16  # SB copy
PARTTAB_PAYLOAD_SIZE = 3  # [root_p, used_p, gen_p]
PARTTAB_ROOT_P_OFFSET = 16  # Cell index in PartTab
PARTTAB_USED_P_OFFSET = 17
PARTTAB_GEN_P_OFFSET = 18

# Arena layout (line 9 of store.bp)
ARENA_BASE = 1024

# Object header constants (line 7-8 of store.bp)
OBJ_HEADER_SIZE = 2  # h0, h1
OBJ_H0_LEN_MASK = 0xFFFFFFFF
OBJ_H1_CRC_SHIFT = 32


def cells(data, offset, count):
    """Extract little-endian i64 values from data starting at byte offset."""
    byte_offset = offset * 8
    byte_count = count * 8
    if byte_offset + byte_count > len(data):
        raise ValueError(f"Read past end: offset={offset}, count={count}, len={len(data) // 8}")
    return list(struct.unpack(f'<{count}q', data[byte_offset:byte_offset + byte_count]))


def mask64():
    """64-bit wrapping mask for unsigned operations."""
    return (1 << 64) - 1


def sb_valid(data, sb):
    """Check if superblock at sb (0 or 512) is valid.
    Source: st_sb_valid line 86-87 of store.bp"""
    M = mask64()
    try:
        c = cells(data, sb, SB_CELL_SIZE)
        if c[SB_MAGIC] != MAGIC:
            return False
        # CRC covers cells 0..14 (15 cells), not including the crc itself (cell 15)
        expected_crc = zlib.crc32(struct.pack('<15q', *c[:15])) & M
        return (c[SB_CRC] & M) == expected_crc
    except (ValueError, struct.error):
        return False


def pick_live(data):
    """Determine the live superblock index (0, 512) or -1 if neither valid.
    Implements st_pick logic from line 89-91 of store.bp:
    - Both valid: pick higher generation
    - One valid: pick that one
    - None valid: return -1
    """
    va = sb_valid(data, SB_A)
    vb = sb_valid(data, SB_B)
    
    if va and vb:
        # Both valid: pick the one with higher generation
        gen_a = cells(data, SB_A, SB_CELL_SIZE)[SB_GENERATION]
        gen_b = cells(data, SB_B, SB_CELL_SIZE)[SB_GENERATION]
        return SB_B if gen_b > gen_a else SB_A
    elif va:
        return SB_A
    elif vb:
        return SB_B
    else:
        return -1


def parttab(data, sb):
    """Read the PartTab object at superblock sb's root pointer.
    Returns: {'offset': parttab_offset, 'len': declared_length, 'crc_ok': bool, 'payload': [3 cells]}
    or None if root is null or PartTab is malformed.
    Source: PartTab format lines 18-35 of store.bp"""
    c = cells(data, sb, SB_CELL_SIZE)
    parttab_off = c[SB_ROOT]
    
    if parttab_off == 0:
        return None
    
    try:
        # Read PartTab header (2 cells)
        pt_header = cells(data, parttab_off, 2)
        pt_h0, pt_h1 = pt_header
        
        pt_len = pt_h0 & OBJ_H0_LEN_MASK
        pt_crc_stored = (pt_h1 >> OBJ_H1_CRC_SHIFT) & mask64()
        
        # Verify it's the expected PartTab (19 cells for P=1)
        if pt_len != PARTTAB_SIZE:
            return None
        
        # The PAYLOAD starts at +2: cells 0 and 1 are the object header that st_alloc writes.
        # Reading from parttab_off instead put every payload index two cells early -- root_p
        # came back as the superblock copy's cell 14 (always 0) and the tool reported a healthy
        # store as corrupt. That is the same off-by-two the store itself suffered, now in the
        # instrument: st_get/st_ref/st_len/st_seal all apply the +2 skip and so must this.
        pt_payload = cells(data, parttab_off + 2, PARTTAB_SIZE)

        # st_seal computes the object crc over the WHOLE payload -- st_crc(base, off+2,
        # st_len(base, off)) -- not over the 16-cell superblock copy, and stores it in the
        # high half of h1.
        pt_crc_computed = zlib.crc32(struct.pack('<%dq' % PARTTAB_SIZE, *pt_payload)) & 0xFFFFFFFF
        crc_ok = ((pt_crc_stored & 0xFFFFFFFF) == pt_crc_computed)
        
        return {
            'offset': parttab_off,
            'len': pt_len,
            'crc_ok': crc_ok,
            'payload': [pt_payload[PARTTAB_ROOT_P_OFFSET],
                       pt_payload[PARTTAB_USED_P_OFFSET],
                       pt_payload[PARTTAB_GEN_P_OFFSET]]
        }
    except (ValueError, struct.error, IndexError):
        return None


def root(data):
    """Return the root object offset by dereferencing the live superblock's root chain.
    Implements the PartTab dereference logic (B5 step 1).
    Source: Inspired by scrash.py lines 36-45 and st_open logic"""
    sb = pick_live(data)
    if sb < 0:
        return None
    
    c = cells(data, sb, SB_CELL_SIZE)
    root_off = c[SB_ROOT]
    
    if root_off == 0:
        return None
    
    # B5 step 1: root points to a PartTab, not directly to the root chain
    pt = parttab(data, sb)
    if pt is None:
        return None
    
    # Extract root_p from PartTab payload
    return pt['payload'][0]  # root_p is at offset 16 in PartTab


def obj(data, offset):
    """Read object at arena offset.
    Returns: {'digest': digest_lo32, 'len': length_in_cells, 'gen': generation,
              'crc_ok': bool, 'payload': raw_payload_bytes}
    or None if read fails.
    Source: Object format from lines 7-8 of store.bp"""
    try:
        h0, h1 = cells(data, offset, 2)
        
        obj_len = h0 & OBJ_H0_LEN_MASK
        obj_digest = (h0 >> 32) & 0xFFFFFFFF
        obj_crc_stored = (h1 >> OBJ_H1_CRC_SHIFT) & mask64()
        obj_gen = h1 & 0xFFFFFFFF
        
        if obj_len < 1:
            return None  # Trap 84: zero-length object
        
        # Read payload
        payload_cells = cells(data, offset + 2, obj_len)
        
        # CRC covers payload only (cells offset+2 through offset+2+len-1)
        payload_bytes = struct.pack(f'<{obj_len}q', *payload_cells)
        obj_crc_computed = zlib.crc32(payload_bytes) & mask64()
        crc_ok = (obj_crc_stored == obj_crc_computed)
        
        return {
            'digest': obj_digest,
            'len': obj_len,
            'gen': obj_gen,
            'crc_ok': crc_ok,
            'payload': payload_bytes
        }
    except (ValueError, struct.error, IndexError):
        return None
