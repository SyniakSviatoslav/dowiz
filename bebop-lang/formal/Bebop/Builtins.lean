/-  Bebop.Builtins -- 10 executable builtins + dispatch.
    Replaces stub/sorry versions with real implementations where possible.
    Imports only Bebop.Basic (State.zeros/arenaRead live there); the
    evaluator in Bebop.Semantics imports THIS module, never the reverse.
    References:
    - docs/RESEARCH-VERIFICATION-2026-09-09.md section 5.2
    - tools/bpref.py (727 lines, the executable reference semantics)
-/

import Bebop.Basic

namespace Bebop.Builtins


-- ============================================================
-- 1. zeros(n) -- allocate n zeroed i64 cells (already done, keep)
-- ============================================================

/- zeros(n): allocate n zeroed i64 cells from the arena.
    LANGUAGE.md:88: exit 80 when the arena is exhausted (T118/T90).
    Never freed; allocation survives the return of the fn (T126).
    In the formal model, this returns the base offset of the
    newly allocated region. -/
def builtinZeros (n : Val) (s : State) : State × Val :=
  let count := n.toNatClampNeg
  let (s', err) := s.zeros count
  match err with
  | some tc =>
    -- ARENA EXHAUSTED. This used to answer `(s', 0)` and leave "caller should
    -- check" in a comment -- no caller did, so `neg/c37_arenafull.bp`
    -- (`let a = zeros(40000000); a[0]`, EXPECT RUNFAIL:80) evaluated to
    -- `ok 0`: the trap the construct exists to pin was silently a value.
    -- It now RAISES: `State.trapped` stops the evaluator and `evalProgram`
    -- reports `Result.trap` with exit code 80.
    ({ s' with trapped := some tc }, 0)
  | none =>
    let offset := s.arena.cursor
    (s', Int64.ofNat offset)

-- ============================================================
-- 2. str_len(s) -- length of a string literal
-- ============================================================

/- str_len(s): length of a string literal.
    LANGUAGE.md:89: length of a string literal ("..." is only valid as an argument).
    In bpref.py, a string value encodes length in the low 32 bits:
      return s & 0xffffffff
    The high 32 bits hold the offset into the byte buffer (not modelled here).
    str_len returns the length encoded in the low 32 bits. -/
def builtinStrLen (s : Val) : Val :=
  s &&& 0xFFFFFFFF


-- ============================================================
-- 2b. zlib CRC-32, ONE implementation, used by crc32 / crc32x / crc32b
-- ============================================================

/-  THE BUG THIS REPLACES, and why it produced a plausible number.

    `crc32Table` used to be built by shifting LEFT, testing bit 7 and masking
    to 8 bits before XORing the 32-bit polynomial:
        let msb := j >>> 7 ; let j' := (j <<< 1) &&& 0xFF
        let j'' := if msb == 1 then j' ^^^ 0xEDB88320 else j'
    That is the NON-reflected (MSB-first) update written with the REFLECTED
    polynomial and then truncated to 8 bits, so the polynomial's own high bits
    were thrown away every round; and the final `^ 0xFFFFFFFF` was missing
    entirely. It still returned a 32-bit number for every input, which is why
    it survived: `crc32("123456789")` came back 15579374 where zlib gives
    3421780262, and nothing compared the two until the constructs did.

    The correct reflected update, which is what zlib's table builder is:
        c := i ; repeat 8: c := if c &&& 1 == 1 then 0xEDB88320 ^^^ (c >>> 1)
                                else c >>> 1
    and the running CRC starts at 0xFFFFFFFF and is complemented at the end.

    Pinned below by three `#guard`s against values this tree already carries:
    `zlib.crc32(b"") = 0` and `zlib.crc32(b"123456789") = 3421780262` (the
    header of c42_crc32.bp) and `zlib.crc32(b"abc") = 891568578` (the header of
    c68_strval.bp). Those run at elaboration time, so `lake build` fails if the
    polynomial or the final XOR is ever changed back. -/
def crc32Table (i : UInt32) : UInt32 :=
  let rec build : Nat → UInt32 → UInt32
    | 0, c => c
    | k + 1, c => build k (if c &&& 1 == 1 then (0xEDB88320 : UInt32) ^^^ (c >>> 1) else c >>> 1)
  build 8 i

/-- Update a running CRC with one byte (only the low 8 bits are used). -/
def crc32Step (crc : UInt32) (b : UInt8) : UInt32 :=
  crc32Table ((crc ^^^ b.toUInt32) &&& 0xFF) ^^^ (crc >>> 8)

/-- zlib CRC-32 of a byte list: init 0xFFFFFFFF, reflected table, final XOR. -/
def crc32Of (bytes : List UInt8) : UInt32 :=
  (bytes.foldl crc32Step 0xFFFFFFFF) ^^^ 0xFFFFFFFF

private def crcOfString (s : String) : Nat := (crc32Of s.toUTF8.toList).toNat

-- zlib.crc32(b"") = 0
#guard crcOfString "" == 0
-- zlib.crc32(b"123456789") = 3421780262 (0xCBF43926) -- c42_crc32.bp's header
#guard crcOfString "123456789" == 3421780262
-- zlib.crc32(b"abc") = 891568578 -- c68_strval.bp's header
#guard crcOfString "abc" == 891568578

-- ============================================================
-- 3. char(s, i) -- byte at position i of string literal
-- ============================================================

/-- `char(s, i)`: the i-th byte of a string, read from the BYTE ARENA.

    Was: "return the low byte of the handle for i = 0 and 0 after", with a
    comment saying a full model "would read from a separate byte buffer".
    Probed at the time: 3 0 0 for a length-3 handle where bpref reads 97 98 99.
    There is a byte buffer now (`State.bytes`), so this is bpref's rule
    verbatim -- `self.bytes[off + i] if off + i < len(self.bytes) else 0`. Note
    it is bounded by the BUFFER, not by the handle's length: `char(s, 5)` on a
    5-byte string reads the NUL terminator, and that is deliberate on both
    sides. -/
def builtinChar (h i : Val) (s : State) : Val :=
  Int64.ofNat (s.byteAt (strOff h + i.toNatClampNeg)).toNat

/-- `crc32b(s)`: zlib CRC-32 of a STRING's bytes (bpref: `zlib.crc32(
    bytes(self.bytes[off:off+ln]))`). It was absent from `formal/` entirely --
    the one builtin of the 41 modelled nowhere -- and c68_strval checks two of
    its values. -/
def builtinCrc32b (h : Val) (s : State) : Val :=
  let off := strOff h
  let ln := strLen h
  Int64.ofNat (crc32Of ((List.range ln).map (fun k => s.byteAt (off + k)))).toNat

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
  -- clz(x) = 63 - floor(log2 x) for x != 0; Init has UInt64.log2 but no clz.
  (s, if x == 0 then Int64.ofNat 64
       else Int64.ofNat (63 - x.toUInt64.log2.toNat))

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
  let start := cells.toNatClampNeg
  let count := n.toNatClampNeg
  -- bpref: `data = bytes(cells[i] & 0xff for i in range(n))`. A cell outside
  -- the arena reads 0 through `arenaRead`'s `getD`, and reading past the
  -- ALLOCATION stops the loop rather than inventing bytes.
  let rec go (addr k : Nat) (crc : UInt32) (st : State) : UInt32 :=
    match k with
    | 0 => crc
    | k' + 1 =>
      match st.arenaRead addr with
      | some v => go (addr + 1) k' (crc32Step crc (v.toUInt64 &&& 0xFF).toUInt8) st
      | none => crc
  (s, Int64.ofNat ((go start count 0xFFFFFFFF s) ^^^ 0xFFFFFFFF).toNat)

-- ============================================================
-- 7. crc32x(cells, off, n) -- zlib crc32 of raw LE bytes
-- ============================================================

/-- `crc32x(cells, off, n)`: zlib CRC-32 of the raw LITTLE-ENDIAN bytes of n
    cells starting at `cells[off]` (bpref packs each cell with `struct.pack(
    '<q', ...)`).

    IT IS THE SAME POLYNOMIAL AND THE SAME BIT ORDER AS `crc32`, and that was
    worth checking rather than assuming: c45_crc32x's own header says the
    values come from `python zlib.crc32(struct.pack('<%dq'))`, i.e. plain zlib
    over a different BYTE SEQUENCE, not a different CRC. (The ARMv8 `crc32x`
    INSTRUCTION is what the emitter uses -- it consumes 8 bytes per step
    instead of 1 -- but it computes the same reflected CRC-32, which is why one
    table serves both.) So the only difference from `crc32` above is how each
    cell becomes bytes: 8 LE bytes here, one masked byte there. -/
def builtinCrc32x (cells off n : Val) (s : State) : State × Val :=
  let start := (cells + off).toNatClampNeg
  let count := n.toNatClampNeg
  let rec go (addr k : Nat) (crc : UInt32) (st : State) : UInt32 :=
    match k with
    | 0 => crc
    | k' + 1 =>
      match st.arenaRead addr with
      | some v =>
        let w := v.toUInt64
        let crc := [0, 8, 16, 24, 32, 40, 48, 56].foldl
                     (fun c (sh : UInt64) => crc32Step c ((w >>> sh) &&& 0xFF).toUInt8) crc
        go (addr + 1) k' crc st
      | none => crc
  (s, Int64.ofNat ((go start count 0xFFFFFFFF s) ^^^ 0xFFFFFFFF).toNat)

-- ============================================================
-- 8. hvham(a, b, n) -- NEON popcount of a^b over n words
-- ============================================================

/- hvham(a, b, n): NEON popcount of a^b over n words.
    LANGUAGE.md:95: hvham/hvham2.
    In the formal model, we use a scalar popcount (since NEON is not modelled).
    For each i in 0..n-1, compute popcount(a[i] XOR b[i]) and sum. -/
/-- Population count of a 64-bit word (scalar; Init has no popcount). -/
def popCount64 (x : UInt64) : Nat :=
  let rec go (k : Nat) (w : UInt64) (acc : Nat) : Nat :=
    match k with
    | 0 => acc
    | k + 1 => go k (w >>> 1) (acc + (w &&& 1).toNat)
  go 64 x 0

def builtinHvham (a b n : Val) (s : State) : State × Val :=
  let baseA := a.toNatClampNeg
  let baseB := b.toNatClampNeg
  -- LANGUAGE.md:93, read 2026-09-14: the count is "n rounded DOWN to a
  -- multiple of 4 words -- the last `n mod 4` words are SILENTLY IGNORED",
  -- measured that day by triggering it (a[0]=255,b=0 gives 0 at n=1,2,3 and 8
  -- at n>=4). This model used all n. No construct in the 121 calls hvham, so
  -- this changes no score; it is corrected because the doc records a MEASURED
  -- fact that the model contradicted.
  let count := (n.toNatClampNeg / 4) * 4
  let rec loop (i : Nat) (acc : Nat) (st : State) : State × Val :=
    if i >= count then (st, Int64.ofNat acc)
    else
      match st.arenaRead (baseA + i) with
      | some va =>
        match st.arenaRead (baseB + i) with
        | some vb =>
          let xorVal := va ^^^ vb
          let pop := popCount64 xorVal.toUInt64
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

/-- `scan(s, pos, class)`: advance `pos[0]` over bytes of one class and return
    the new position. LANGUAGE.md:100 (read 2026-09-14, verbatim):

      advance `pos[0]` over bytes of one class and return the new pos:
      0 = whitespace, 1 = ident `[0-9A-Za-z_]`, 2 = not-`"`-not-`\`,
      anything else = not-newline. Stops at the `pos[1]` length bound, so it
      never reads past it (A9 step 3, scalar form)

    WHAT WAS HERE, and why it scored 0 rather than failing. The old version
    read `pos` as if the pair were PACKED INTO THE HANDLE
    (`posStart = pos & 0xffffffff`, `posEnd = pos >>> 32`) -- but `pos` is an
    ARRAY of two cells, so those were the low and high halves of an arena
    offset, not a position and a bound. It then declined to read any byte at
    all ("conservative: no byte matching possible") and returned `posStart`
    unchanged, so `c78_scan` evaluated to `ok 0`: the classifier never
    advanced and the program's whole 131-fold collapsed.

    This is bpref.py's loop (`builtin_or_call`, `name == 'scan'`) rule for
    rule: both bounds are checked (`pos[1]` AND the string's own length), the
    write-back to `pos[0]` happens whether or not anything moved, and the
    return value is the new position -- c78's header says it folds over the
    RETURN of every call AND ends with `pos[0]`, precisely so a model that got
    either half right and the other wrong still mismatches. -/
def scanMatches (cls : Val) (b : UInt8) : Bool :=
  let v := b.toNat
  if cls == 0 then v == 32 || v == 9 || v == 10 || v == 13
  else if cls == 1 then
    (48 ≤ v && v ≤ 57) || (65 ≤ v && v ≤ 90) || (97 ≤ v && v ≤ 122) || v == 95
  else if cls == 2 then v != 34 && v != 92
  else v != 10

def builtinScan (h pos cls : Val) (s : State) : State × Val :=
  let off := strOff h
  let len := strLen h
  let base := pos.toNatClampNeg
  let start := (s.arenaRead base).getD 0 |>.toNatClampNeg
  let limit := (s.arenaRead (base + 1)).getD 0 |>.toNatClampNeg
  let stop := min limit len
  let rec go (p : Nat) (steps : Nat) : Nat :=
    match steps with
    | 0 => p
    | steps + 1 =>
      if p ≥ stop then p
      else if scanMatches cls (s.byteAt (off + p)) then go (p + 1) steps
      else p
  let p' := go start (stop + 1)
  let s' := (s.arenaWrite base (Int64.ofNat p')).getD s
  (s', Int64.ofNat p')

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
    match args[0]? with
    | some s' => some (s, builtinStrLen s')
    | none => none
  | "char" =>
    match args[0]?, args[1]? with
    | some c, some i => some (s, builtinChar c i s)
    | _, _ => none
  | "crc32b" =>
    match args[0]? with
    | some h => some (s, builtinCrc32b h s)
    | none => none
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
    | some h, some pos, some cls => some (builtinScan h pos cls s)
    | _, _, _ => none
  | _ => none

end Bebop.Builtins
