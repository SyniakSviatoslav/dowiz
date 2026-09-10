/-
  Bebop formal verification: root module
  Imports all submodules of the definitional semantics.

  Build: cd formal && lake build
  This runs OFF-BOX (the box cannot host Lean under 3GB/32-process caps).
  Results are committed as a sha256-bound file, consumed by the chain gate.

  F4 is PARALLEL to F1/F2 -- it does NOT depend on them.
-/

import Bebop.Basic
import Bebop.Semantics
import Bebop.Builtins
import Bebop.Syscalls
import Bebop.Traps
import Bebop.Conformance
