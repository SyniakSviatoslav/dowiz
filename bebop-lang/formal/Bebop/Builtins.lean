/-  Bebop.Builtins -- 10 executable builtins + dispatch.
    Replaces stub/sorry versions with real implementations where possible.
    References:
    - docs/RESEARCH-VERIFICATION-2026-09-09.md section 5.2
    - tools/bpref.py (727 lines, the executable reference semantics)
-/

import Bebop.Basic
import Bebop.Semantics
import Bebop.Syscalls

namespace Bebop.Builtins

open Basic
open Bebop.Semantics
open Bebop.Syscalls

-- ============================================================
-- 1. zeros(n) -- allocate n zeroed i64 cells (already done, keep)
-- ============================================================

/- zeros(n): allocate n zeroed i64 cells from the arena.
    LANGUAGE.md:88: exit 80 when the arena is exhausted (T118/T90).
    Never freed; allocation survives the return of the fn (T126).
    In the formal model, this returns the base offset of the
    newly allocated region. -/
def builtinZeros (n : Val) (s : State) : State × Val :=
  let count := Nat.abs n.toNat
  let (s', err) := s.zeros count
  match err with
  | some e => (s', 0)  -- trap; caller should check
  | none =>
    let offset := s.arena.cells.size
    (s', Int64.ofNat offset)

-- ============================================================
-- 2. str_len(s) -- length of a string literal
-- ============================================================

/- str_len(s): length of a string literal.
    LANGUAGE.md:89: length of a string literal ("..." is only valid as an argument).
    In the formal model, string literals encode their length in the high 32 bits
    and the string data in subsequent arena cells (A7).
    str_len returns the length encoded in the high 32 bits. -/
def builtinStrLen (s : State) : State × Val :=
  -- Extract length from high 32 bits of the encoded string value.
  -- The string value is passed as the first arg; we need the state to read it.
  -- Actually: str_len takes one arg (the string), returns its length.
  -- In the dispatch, args[0] is the string value.
  (s, 0)  -- stub: real impl reads string table; formal model returns 0

-- ============================================================
-- 3. char(s, i) -- byte at position i of string literal
-- ============================================================

/- char(s, i): byte of a string literal at position i.
    LANGUAGE.md:89: "length / byte of a string literal".
    In the formal model, s is a string value (encoded as length in high 32 bits),
    i is the index. Returns the i-th byte, or 0 if i >= length.
    A full model would decode the string data from arena cells. -/
def builtinChar (s i : Val) (s0 : State) : State × Val :=
  let len := ((s.toU >>> 32) & 0xFFFFFFFF).toNat
  let idx := Nat.abs i.toNat
  if idx < len then
    -- Extract byte at position idx from the string value.
    -- In the formal model, the string data is in the low bits after the length.
    -- For a real string table model, we'd read arena cells.
    -- Simplified: extract byte from the encoded value (works for short strings).
    let byte := ((s.toU >>> (idx * 8)) & 0xFF).toInt
    (s0, byte)
  else
    -- Out of bounds: return 0 (matching bpref.py behavior for OOB)
    (s0, 0)

-- ============================================================
-- 4. clock_ms() -- CLOCK_MONOTONIC in milliseconds
-- ============================================================

/- clock_ms(): CLOCK_MONOTONIC in milliseconds.
    LANGUAGE.md:90.
    Modelled as an oracle input: the state carries clockMs. -/
def builtinClockMs (s : State) : State × Val :=
  (s, s.clockMs)

-- ============================================================
-- 5. clz(x) -- count leading zeros
-- ============================================================

/- clz(x): count leading zeros of the 64-bit word.
    LANGUAGE.md:94: clz(0) = 64 (T105; seeds the Newton isqrt). -/
def builtinClz (x : Val) (s : State) : State × Val :=
  (s, if x == 0 then Int64.ofNat 64
       else Int64.ofNat x.toU.clz)

-- ============================================================
-- 6. crc32(cells, n) -- zlib crc32 of n bytes held one per cell
-- ============================================================

/- crc32(cells, n): zlib crc32 of n bytes held one per cell.
    LANGUAGE.md:96: CRC32B loop (T109).
    In the formal model, we use a pure function over the arena cells.
    The CRC32 polynomial is 0xEDB88320 (reflected).
    We read n cells from the arena starting at address `cells`,
    treat each as a byte (low 8 bits), and compute CRC32. -/
def builtinCrc32 (cells n : Val) (s : State) : State × Val :=
  let start := cells.toNat
  let count := Nat.abs n.toNat
  -- Read count cells from arena starting at start
  let rec readCells (addr : Nat) (k : Nat) (acc : List Val) (st : State)
      : List Val × State :=
    if k == 0 then (acc.reverse, st)
    else
      match st.arenaRead addr with
      | some v => readCells (addr + 1) (k - 1) (v :: acc) st
      | none => (acc.reverse, st)  -- OOB: stop reading
  let (byteVals, s') := readCells start count [] s
  -- Compute CRC32 over the bytes
  let crc := crc32List byteVals s'
  (s', crc)

where
  -- CRC32 computation over a list of bytes (each byte is the low 8 bits of a Val)
  def crc32List (bytes : List Val) (s : State) : Val :=
    let rec go (xs : List Val) (crc : UInt32) : UInt32 :=
      match xs with
      | [] => crc
      | b :: bs =>
        let byte := (b.toU & 0xFF).toUInt32
        let idx := (crc ^^^ byte) & 0xFF
        go bs (crc32Table idx ^^^ (crc >>> 8))
    Int64.ofNat (go bytes 0xFFFFFFFF).toNat

  -- CRC32 lookup table (reflected polynomial 0xEDB88320)
  def crc32Table (i : UInt32) : UInt32 :=
    let rec build (j : UInt32) (k : Nat) : UInt32 :=
      if k == 8 then j
      else
        let msb := j >>> 7
        let j' := (j <<< 1) & 0xFF
        let j'' := if msb == 1 then j' ^^^ 0xEDB88320 else j'
        build j'' (k + 1)
    build i 0

-- ============================================================
-- 7. crc32x(cells, off, n) -- zlib crc32 of raw LE bytes
-- ============================================================

/- crc32x(cells, off, n): zlib crc32 of the raw little-endian bytes
    of n cells from cells[off].
    LANGUAGE.md:96: CRC32X, 8 B per step (T109b).
    Reads n cells from arena starting at cells+off, treats each cell's
    8 bytes (LE order) as input to CRC32. -/
def builtinCrc32x (cells off n : Val) (s : State) : State × Val :=
  let start := (cells + off).toNat
  let count := Nat.abs n.toNat
  let rec readCells (addr : Nat) (k : Nat) (acc : List Val) (st : State)
      : List Val × State :=
    if k == 0 then (acc.reverse, st)
    else
      match st.arenaRead addr with
      | some v => readCells (addr + 1) (k - 1) (v :: acc) st
      | none => (acc.reverse, st)
  let (cellVals, s') := readCells start count [] s
  -- Expand each cell into 8 LE bytes
  let bytes := cellVals.flatMap (fun cv =>
    #[ cv.toU & 0xFF,
       (cv.toU >>> 8) & 0xFF,
       (cv.toU >>> 16) & 0xFF,
       (cv.toU >>> 24) & 0xFF,
       (cv.toU >>> 32) & 0xFF,
       (cv.toU >>> 40) & 0xFF,
       (cv.toU >>> 48) & 0xFF,
       (cv.toU >>> 56) & 0xFF ])
  let crc := crc32xList bytes s'
  (s', crc)

where
  def crc32xList (bytes : List UInt64) (s : State) : Val :=
    let rec go (xs : List UInt64) (crc : UInt32) : UInt32 :=
      match xs with
      | [] => crc
      | b :: bs =>
        -- Process byte b (already masked to 8 bits)
        let byte := (b & 0xFF).toUInt32
        let idx := (crc ^^^ byte) & 0xFF
        go bs (crc32xTable idx ^^^ (crc >>> 8))
    Int64.ofNat (go bytes 0xFFFFFFFF).toNat

  def crc32xTable (i : UInt32) : UInt32 :=
    let rec build (j : UInt32) (k : Nat) : UInt32 :=
      if k == 8 then j
      else
        let msb := j >>> 7
        let j' := (j <<< 1) & 0xFF
        let j'' := if msb == 1 then j' ^^^ 0xEDB88320 else j'
        build j'' (k + 1)
    build i 0

-- ============================================================
-- 8. hvham(a, b, n) -- NEON popcount of a^b over n words
-- ============================================================

/- hvham(a, b, n): NEON popcount of a^b over n words.
    LANGUAGE.md:95: hvham/hvham2.
    In the formal model, we use a scalar popcount (since NEON is not modelled).
    For each i in 0..n-1, compute popcount(a[i] XOR b[i]) and sum. -/
def builtinHvham (a b n : Val) (s : State) : State × Val :=
  let baseA := a.toNat
  let baseB := b.toNat
  let count := Nat.abs n.toNat
  let rec loop (i : Nat) (acc : Nat) (st : State) : State × Val :=
    if i >= count then (st, Int64.ofNat acc)
    else
      match st.arenaRead (baseA + i) with
      | some va =>
        match st.arenaRead (baseB + i) with
        | some vb =>
          let xorVal := va ^^^ vb
          let pop := xorVal.toU.popCount.toNat
          loop (i + 1) (acc + pop) st
        | none => (st, Int64.ofNat acc)  -- OOB: stop
      | none => (st, Int64.ofNat acc)  -- OOB: stop
  loop 0 0 s

-- ============================================================
-- 9. hvham2(a, b, n) -- variant of hvham
-- ============================================================

/- hvham2(a, b, n): variant of hvham.
    LANGUAGE.md:95.
    In the formal model, same as hvham but with a different combining function.
    For conformance, hvham2 = hvham (the difference is in the NEON implementation,
    which is not modelled in the formal semantics). -/
def builtinHvham2 (a b n : Val) (s : State) : State × Val :=
  builtinHvham a b n s  -- stub: same as hvham in formal model

-- ============================================================
-- 10. scan(s, pos, class) -- advance pos over bytes of one class
-- ============================================================

/- scan(s, pos, class): advance pos[0] over bytes of one class and
    return the new pos.
    LANGUAGE.md:99: 0 = whitespace, 1 = ident [0-9A-Za-z_], 2 = not-'"'-not-'\\',
    anything else = not-newline. Stops at the pos[1] length bound.
    In the formal model, s is a string value (encoded as length in high 32 bits,
    and the string data in subsequent arena cells). pos is a 2-cell array
    [start, end] where start is the current position and end is the bound.
    Returns the new pos array with start advanced past bytes of the given class.
    For the formal model, we implement a basic scan over the string data. -/
def builtinScan (s pos class : Val) (s0 : State) : State × Val :=
  let strLen := ((s.toU >>> 32) & 0xFFFFFFFF).toNat
  let posStart := (pos & 0xFFFFFFFF).toNat
  let posEnd := ((pos.toU >>> 32) & 0xFFFFFFFF).toNat
  -- Read string data from arena starting at offset s.toNat
  let rec readByte (addr : Nat) (st : State) : State × Val :=
    match st.arenaRead addr with
    | some v => (st, v)
    | none => (st, 0)
  -- Scan forward from posStart to posEnd
  let rec scanFwd (pos : Nat) (end : Nat) (st : State) : State × Nat :=
    if pos >= end then (st, pos)
    else
      let (st', byte) := readByte (s.toNat + pos) st
      let keep := match class with
        | 0 => byte == 32 || byte == 9 || byte == 10 || byte == 13 || byte == 11 || byte == 12  -- whitespace
        | 1 => (byte >= 48 && byte <= 57) || (byte >= 65 && byte <= 90) || (byte >= 97 && byte <= 122) || byte == 95  -- ident
        | 2 => byte != 34 && byte != 92  -- not '"' and not '\\'
        | _ => byte != 10  -- not newline
      if keep then scanFwd (pos + 1) end st'
      else (st', pos)
  let (s', newPos) := scanFwd posStart posEnd s0
  -- Return new pos: pack start (low 32) and end (high 32)
  (s', (Int64.ofNat newPos) ||| (Int64.ofNat posEnd <<< 32))

-- ============================================================
-- 11. Dispatch table for all 10 executable builtins
-- ============================================================

/- Dispatch a builtin call. Returns none if the name is not a builtin. -/
def dispatchBuiltin (name : Name) (args : Array Val) (s : State)
    : Option (State × Val) :=
  match name with
  | "zeros" =>
    match args[0]? with
    | some n => some (builtinZeros n s)
    | none => none
  | "str_len" =>
    -- str_len takes the string value as arg[0]; we need to extract length.
    -- In the formal model, string value encodes length in high 32 bits.
    match args[0]? with
    | some s' => some (s, ((s'.toU >>> 32) & 0xFFFFFFFF).toInt)
    | none => none
  | "char" =>
    match args[0]?, args[1]? with
    | some c, some i => some (builtinChar c i s)
    | _, _ => none
  | "clock_ms" => some (builtinClockMs s)
  | "clz" =>
    match args[0]? with
    | some x => some (builtinClz x s)
    | none => none
  | "crc32" =>
    match args[0]?, args[1]? with
    | some cells, some n => some (builtinCrc32 cells n s)
    | _, _ => none
  | "crc32x" =>
    match args[0]?, args[1]?, args[2]? with
    | some cells, some off, some n => some (builtinCrc32x cells off n s)
    | _, _, _ => none
  | "hvham" =>
    match args[0]?, args[1]?, args[2]? with
    | some a, some b, some n => some (builtinHvham a b n s)
    | _, _, _ => none
  | "hvham2" =>
    match args[0]?, args[1]?, args[2]? with
    | some a, some b, some n => some (builtinHvham2 a b n s)
    | _, _, _ => none
  | "scan" =>
    match args[0]?, args[1]?, args[2]? with
    | some s', some pos, some cls => some (builtinScan s' pos cls s)
    | _, _, _ => none
  | _ => none

end Bebop.Builtins
