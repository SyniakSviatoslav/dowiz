/-
  Bebop.Lexer -- tokenizer for the `.bp` surface syntax.

  Grammar source: docs/LANGUAGE.md lines 13-83. Where the doc and the compiler
  disagree, the disagreement is recorded here and NOT silently resolved; see
  `Bebop/Parser.lean` §0 for the precedence table and the measurements behind it.

  Design rules this file follows, because the alternative is the failure mode
  this tree keeps paying for:
    * Every unrecognised byte is an ERROR with a position. There is no
      "skip what you do not understand" path.
    * `LexError` distinguishes a byte the lexer cannot classify from a token it
      classifies but the PARSER does not support yet. The second kind is not
      the lexer's business and is never reported here.
    * Longest-match is explicit and ordered: `>>>` before `>>` before `>=`
      before `>`. Getting that order wrong would silently retokenise every
      shift in the corpus, so the order is asserted by #guard at the bottom.
-/

import Bebop.Basic

namespace Bebop.Lexer

/-- A lexical token. Keywords are NOT a separate case: they arrive as `ident`
    and the parser compares strings, which keeps the token type small and makes
    a misspelled keyword a parse error at a useful position rather than a lex
    error at a useless one. -/
inductive Tok where
  | ident (s : String)
  | num   (v : Int)
  | str   (s : String)
  | punct (s : String)
  | eof
  deriving BEq, Inhabited, Repr

def Tok.render : Tok → String
  | .ident s => s!"ident({s})"
  | .num v   => s!"num({v})"
  | .str s   => s!"str(\"{s}\")"
  | .punct s => s!"'{s}'"
  | .eof     => "<eof>"

/-- A token with the source position of its first character (1-based line,
    0-based column, matching `Bebop.Position`). -/
structure Token where
  tok  : Tok
  line : Nat
  col  : Nat
  deriving Inhabited, Repr

inductive LexError where
  /-- A character that cannot begin any token. -/
  | badChar (c : Char) (line col : Nat)
  /-- A string literal with no closing quote. -/
  | unterminatedString (line col : Nat)
  /-- `0x` with no hex digits after it. -/
  | badHex (line col : Nat)
  deriving Inhabited

def LexError.render : LexError → String
  | .badChar c l c' => s!"{l}:{c'}: cannot start a token: '{c}'"
  | .unterminatedString l c => s!"{l}:{c}: unterminated string literal"
  | .badHex l c => s!"{l}:{c}: `0x` with no hex digits"

/-- The multi-character operators, LONGEST FIRST. This order is the whole
    correctness argument for `>>` vs `>>>`: LANGUAGE.md:64 says `>>` is the
    LOGICAL shift (lsrv) and `>>>` the ARITHMETIC one (asrv) -- the opposite of
    most languages -- and MEASURED 2026-09-14 on the promoted `bebop.bin`
    (sha256 3af3250...):
      `fn main() -> i64 { (0 - 16) >> 4 }`  -> 1152921504606846975
      `fn main() -> i64 { (0 - 16) >>> 4 }` -> -1
    If `>>` were matched before `>>>`, `a >>> b` would lex as `a >> (> b)` and
    fail, or worse, as two shifts. Hence: longest first, and the #guard below. -/
def multiOps : List String :=
  [">>>", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "->", "=>",
   "+=", "-=", "*=", "/=", "%="]

def singleOps : List Char :=
  ['(', ')', '{', '}', '[', ']', ',', ';', ':', '=', '<', '>',
   '|', '^', '&', '+', '-', '*', '/', '%', '!', '.']

private def isIdentStart (c : Char) : Bool := c.isAlpha || c == '_'
private def isIdentCont  (c : Char) : Bool := c.isAlphanum || c == '_'

private def hexVal (c : Char) : Option Nat :=
  if c.isDigit then some (c.toNat - '0'.toNat)
  else if 'a' ≤ c && c ≤ 'f' then some (c.toNat - 'a'.toNat + 10)
  else if 'A' ≤ c && c ≤ 'F' then some (c.toNat - 'A'.toNat + 10)
  else none

/-- Does `src` have `pat` starting at index `i`? -/
private def matchesAt (src : Array Char) (i : Nat) (pat : String) : Bool :=
  let pc := pat.toList
  let rec go (k : Nat) (cs : List Char) : Bool :=
    match cs with
    | [] => true
    | c :: rest =>
      if h : i + k < src.size then
        if src[i + k]'h == c then go (k + 1) rest else false
      else false
  go 0 pc

/-- The tokenizer. `partial` is deliberate and safe here: every branch either
    returns or recurses with a STRICTLY larger index (each branch consumes at
    least one character), so it terminates on every finite input; Lean just
    cannot see that through the `Array Char` indexing without a proof that
    would not earn its keep. The parser, which is the part a malformed file
    could actually drive in circles, takes explicit fuel instead. -/
partial def tokenize (source : String) : Except LexError (Array Token) :=
  let src := source.toList.toArray
  let rec go (i line col : Nat) (acc : Array Token) : Except LexError (Array Token) :=
    if h : i < src.size then
      let c := src[i]'h
      if c == '\n' then go (i + 1) (line + 1) 0 acc
      else if c == ' ' || c == '\t' || c == '\r' then go (i + 1) line (col + 1) acc
      -- line comment
      else if matchesAt src i "//" then
        let rec skipLine (k : Nat) : Nat :=
          if h2 : k < src.size then
            if src[k]'h2 == '\n' then k else skipLine (k + 1)
          else k
        let k := skipLine i
        go k line (col + (k - i)) acc
      -- string literal
      else if c == '"' then
        let rec scanStr (k : Nat) (out : String) : Except LexError (String × Nat) :=
          if h2 : k < src.size then
            let ch := src[k]'h2
            if ch == '"' then .ok (out, k + 1)
            else if ch == '\\' then
              if h3 : k + 1 < src.size then
                let e := src[k + 1]'h3
                let d := if e == 'n' then '\n' else if e == 't' then '\t'
                         else if e == '0' then Char.ofNat 0 else e
                scanStr (k + 2) (out.push d)
              else .error (.unterminatedString line col)
            else scanStr (k + 1) (out.push ch)
          else .error (.unterminatedString line col)
        match scanStr (i + 1) "" with
        | .error e => .error e
        | .ok (s, k) => go k line (col + (k - i)) (acc.push ⟨.str s, line, col⟩)
      -- hex literal
      else if matchesAt src i "0x" || matchesAt src i "0X" then
        let rec scanHex (k : Nat) (v : Nat) (n : Nat) : (Nat × Nat × Nat) :=
          if h2 : k < src.size then
            match hexVal (src[k]'h2) with
            | some d => scanHex (k + 1) (v * 16 + d) (n + 1)
            | none => (v, n, k)
          else (v, n, k)
        let (v, n, k) := scanHex (i + 2) 0 0
        if n == 0 then .error (.badHex line col)
        else go k line (col + (k - i)) (acc.push ⟨.num (Int.ofNat v), line, col⟩)
      -- decimal literal
      else if c.isDigit then
        let rec scanDec (k : Nat) (v : Nat) : (Nat × Nat) :=
          if h2 : k < src.size then
            let ch := src[k]'h2
            if ch.isDigit then scanDec (k + 1) (v * 10 + (ch.toNat - '0'.toNat))
            else (v, k)
          else (v, k)
        let (v, k) := scanDec i 0
        go k line (col + (k - i)) (acc.push ⟨.num (Int.ofNat v), line, col⟩)
      -- identifier / keyword
      else if isIdentStart c then
        let rec scanId (k : Nat) (out : String) : (String × Nat) :=
          if h2 : k < src.size then
            let ch := src[k]'h2
            if isIdentCont ch then scanId (k + 1) (out.push ch) else (out, k)
          else (out, k)
        let (s, k) := scanId i ""
        go k line (col + (k - i)) (acc.push ⟨.ident s, line, col⟩)
      else
        -- multi-character operators, LONGEST FIRST (see `multiOps`)
        match multiOps.find? (fun p => matchesAt src i p) with
        | some p => go (i + p.length) line (col + p.length) (acc.push ⟨.punct p, line, col⟩)
        | none =>
          if singleOps.contains c then
            go (i + 1) line (col + 1) (acc.push ⟨.punct (String.singleton c), line, col⟩)
          else .error (.badChar c line col)
    else .ok (acc.push ⟨.eof, line, col⟩)
  go 0 1 0 #[]

-- ============================================================
-- Self-checks. These run at elaboration time, so `lake build` FAILS if the
-- longest-match order regresses. Cheap, and they pin the one ordering mistake
-- that would silently change the meaning of every shift in the corpus.
-- ============================================================

private def toks (s : String) : List Tok :=
  match tokenize s with
  | .ok ts => (ts.toList.map (·.tok)).dropLast   -- drop the trailing eof
  | .error _ => [.ident "LEX-ERROR"]

/- `>>>` lexes as ONE token, not `>>` followed by `>` -/
#guard toks "a >>> b" == [.ident "a", .punct ">>>", .ident "b"]
/- `>>` still lexes as one token -/
#guard toks "a >> b" == [.ident "a", .punct ">>", .ident "b"]
/- `>=` is not `>` then `=`; `>` alone still works -/
#guard toks "a >= b > c" == [.ident "a", .punct ">=", .ident "b", .punct ">", .ident "c"]
/- `&&` / `||` are single tokens and distinct from `&` / `|` -/
#guard toks "a && b & c || d | e" ==
  [.ident "a", .punct "&&", .ident "b", .punct "&", .ident "c",
   .punct "||", .ident "d", .punct "|", .ident "e"]
/- `==` vs `=`, `!=` vs `!`, `->` vs `-`, `+=` vs `+` -/
#guard toks "a == b = !c != -d -> e += f" ==
  [.ident "a", .punct "==", .ident "b", .punct "=", .punct "!", .ident "c",
   .punct "!=", .punct "-", .ident "d", .punct "->", .ident "e",
   .punct "+=", .ident "f"]
/- decimal and hex literals; `0x1f` = 31 -/
#guard toks "42 0x1f 0X10" == [.num 42, .num 31, .num 16]
/- comments vanish, and do not eat the following line -/
#guard toks "1 // comment 2 3\n4" == [.num 1, .num 4]
/- `=>` (match arms) is one token and is not `=` then `>` -/
#guard toks "a => b" == [.ident "a", .punct "=>", .ident "b"]
/- a string literal is one token and keeps its bytes -/
#guard toks "str_len(\"hi\")" ==
  [.ident "str_len", .punct "(", .str "hi", .punct ")"]

end Bebop.Lexer
