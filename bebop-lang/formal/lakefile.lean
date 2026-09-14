import Lake
open Lake DSL

package BebopFormal where
  leanOptions := #[⟨`autoImplicit, false⟩]

@[default_target]
lean_lib Bebop where
  roots := #[`Bebop]

-- The conformance driver. It is an EXECUTABLE, not a `#guard`, because it does
-- IO: it reads bench/parity_constructs/*.bp from disk. Keeping it out of
-- elaboration also means a parser bug can never hang `lake build`.
lean_exe parityrun where
  root := `ParityRun

-- An INSTRUMENT: times one program at one fuel. Used to measure the arena's
-- scaling exponent rather than infer it from reading the code.
lean_exe timeone where
  root := `TimeOne
