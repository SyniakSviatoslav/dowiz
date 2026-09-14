/-
  Bebop formal verification: root module
  Imports all submodules of the definitional semantics.

  Build (on-box, measured 2026-09-13 with /root/s30/outC4/lean/bin/lean 4.33.1):
    cd formal && LEAN_PATH=$PWD/.lake/build/lib/lean \
      lean -o .lake/build/lib/lean/Bebop/<M>.olean Bebop/<M>.lean
    leaf-first: Basic, Builtins, Syscalls, Semantics, Traps, Conformance, Theorems,
    then this file. Each module takes 4-9 s and one process. The earlier claim
    that "the box cannot host Lean" was never measured and is false.
  Import DAG: Basic <- {Builtins, Syscalls} <- Semantics <- Traps <- Conformance
    <- Theorems <- Bebop. (Until 2026-09-13 Builtins/Syscalls imported Semantics
    back, and nothing under formal/ had ever elaborated.)

  F4 is PARALLEL to F1/F2 -- it does NOT depend on them.
-/

import Bebop.Basic
import Bebop.Lexer
import Bebop.Parser
import Bebop.Semantics
import Bebop.Reject
import Bebop.Sha256
import Bebop.Builtins
import Bebop.Syscalls
import Bebop.Traps
import Bebop.Conformance
import Bebop.Theorems
