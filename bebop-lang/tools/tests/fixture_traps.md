# Fixture TRAPS table for --texts gate testing

Minimal TRAPS.md documenting only the three codes used in fixture_green_simple.bp.

# Exit codes

| code | who | meaning | where |
|---|---|---|---|
| 95 | bebop.bin | expected `)` (emit_paren) or `in` (let-expression) | bebop.bp |
| 96 | bebop.bin | `++` in an expression: string concatenation is not in the surface | emit_expr |
| 97 | bebop.bin | fn body without a tail expression | compile_fn_at |
