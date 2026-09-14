/-
  Bebop.Sha256 -- SHA-256, because the `use "cas://sha256:<hex>"` form cannot be
  resolved without it.

  WHY IT IS HERE RATHER THAN SKIPPED. `bench/parity_constructs/c50_cas.bp` and
  `neg/c51_casbad.bp` are the SAME shape with different digests, and the whole
  point of the pair is that one verifies and the other does not:

      .bcas/632cc10e...117.bp   sha256 = 632cc10e...117   -> c50 includes it
      .bcas/543df89f...36c.bp   sha256 = 632cc10e...117   -> c51 is exit 88

  Measured on this tree, 2026-09-14:
      $ sha256sum .bcas/*
      632cc10e...117  .bcas/543df89fec85b1c280e5be7bc6a33e31203503cd6edb3084312de4db5a9b436c.bp
      632cc10e...117  .bcas/632cc10e760076356e446c2f667b9634c7758c7f2e845f7b230c728b34f4e117.bp
  -- the "bad" file's CONTENT is byte-identical to the good one; only its NAME
  lies. So a loader that resolved `cas://` by filename and skipped the digest
  would include exactly the right text for c51 and score it as a value, and a
  loader that refused every `cas://` would score c51 as a refusal for the wrong
  reason. Neither tells the two apart. Only the digest does, which is why this
  file exists: `bebop.bp:8007` (`cas_verify` ... "exit 88 -- a module is named
  by what it IS") and `tools/bpref.py:813` both do the same check.

  Pinned at elaboration time against the two NIST vectors below, so `lake build`
  fails if this implementation is ever broken.
-/

import Bebop.Basic

namespace Bebop.Sha256

/-- The 64 round constants: the first 32 bits of the fractional parts of the
    cube roots of the first 64 primes (FIPS 180-4 §4.2.2). -/
def K : Array UInt32 := #[
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
  0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
  0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
  0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
  0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
  0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
  0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
  0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
  0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2 ]

#guard K.size == 64

/-- The eight initial hash words: fractional parts of the square roots of the
    first eight primes (FIPS 180-4 §5.3.3). -/
def H0 : Array UInt32 := #[
  0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
  0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19 ]

def rotr (x : UInt32) (n : UInt32) : UInt32 := (x >>> n) ||| (x <<< (32 - n))

/-- Pad the message: `0x80`, then zeros until the length is 56 mod 64, then the
    bit length as a 64-bit big-endian integer. -/
def pad (msg : Array UInt8) : Array UInt8 :=
  let bitLen := msg.size * 8
  let withOne := msg.push 0x80
  let zeros := (56 + 64 - withOne.size % 64) % 64
  let padded := withOne ++ Array.replicate zeros (0 : UInt8)
  padded ++ ((List.range 8).map (fun i =>
    UInt8.ofNat ((bitLen >>> ((7 - i) * 8)) % 256))).toArray

/-- Compress one 64-byte block into the running hash. -/
def block (h : Array UInt32) (b : Array UInt8) (off : Nat) : Array UInt32 :=
  let w0 : Array UInt32 := (Array.range 16).map (fun i =>
    (UInt32.ofNat (b.getD (off + 4*i) 0).toNat <<< 24)
      ||| (UInt32.ofNat (b.getD (off + 4*i+1) 0).toNat <<< 16)
      ||| (UInt32.ofNat (b.getD (off + 4*i+2) 0).toNat <<< 8)
      ||| (UInt32.ofNat (b.getD (off + 4*i+3) 0).toNat))
  let w : Array UInt32 := (List.range 48).foldl (fun w j =>
    let i := j + 16
    let s0 := rotr (w.getD (i-15) 0) 7 ^^^ rotr (w.getD (i-15) 0) 18 ^^^ (w.getD (i-15) 0 >>> 3)
    let s1 := rotr (w.getD (i-2) 0) 17 ^^^ rotr (w.getD (i-2) 0) 19 ^^^ (w.getD (i-2) 0 >>> 10)
    w.push (w.getD (i-16) 0 + s0 + w.getD (i-7) 0 + s1)) w0
  let fin := (List.range 64).foldl (fun (v : Array UInt32) i =>
    let a := v.getD 0 0; let bb := v.getD 1 0; let c := v.getD 2 0; let d := v.getD 3 0
    let e := v.getD 4 0; let f := v.getD 5 0; let g := v.getD 6 0; let hh := v.getD 7 0
    let S1 := rotr e 6 ^^^ rotr e 11 ^^^ rotr e 25
    let ch := (e &&& f) ^^^ ((~~~e) &&& g)
    let t1 := hh + S1 + ch + K.getD i 0 + w.getD i 0
    let S0 := rotr a 2 ^^^ rotr a 13 ^^^ rotr a 22
    let maj := (a &&& bb) ^^^ (a &&& c) ^^^ (bb &&& c)
    let t2 := S0 + maj
    #[t1 + t2, a, bb, c, d + t1, e, f, g]) h
  (Array.range 8).map (fun i => h.getD i 0 + fin.getD i 0)

def hexDigit (n : Nat) : Char :=
  if n < 10 then Char.ofNat (48 + n) else Char.ofNat (87 + n)

/-- SHA-256 of a byte array, as 64 lowercase hex characters. -/
def hex (msg : Array UInt8) : String :=
  let p := pad msg
  let blocks := p.size / 64
  let h := (List.range blocks).foldl (fun h i => block h p (i * 64)) H0
  h.foldl (fun acc (w : UInt32) =>
    ((List.range 8).foldl (fun s j =>
      s.push (hexDigit ((w.toNat >>> ((7 - j) * 4)) % 16))) acc)) ""

private def hexOf (s : String) : String := hex s.toUTF8.toList.toArray

-- FIPS 180-4 / NIST test vectors.
#guard hexOf "" == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
#guard hexOf "abc" == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
#guard hexOf "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
  == "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"

end Bebop.Sha256
