import Lake
open Lake DSL

package BebopFormal where
  leanOptions := #[⟨`autoImplicit, false⟩]

@[default_target]
lean_lib Bebop where
  roots := #[`Bebop]
