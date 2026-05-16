/-!
# `Oxiz.Math` — Lean4 surface for `oxiz-math`

This file declares the Lean-side wrappers for oxiz-math's Simplex
feasibility checker. Link against `liboxiz_binding_lean4.{a,so}`
built with the `oxiz-math` feature.

Status: v0.3 covers SimplexTableau allocate / free, variable
allocation with optional `i64`-rational bounds, and `check`.
Arbitrary-coefficient constraints (`assert_constraint`) arrive in
v0.4 once the FFI shape is pinned.
-/

namespace Oxiz.Math

opaque SimplexTableauNonempty : NonemptyType

/-- Opaque handle to a Rust-owned `oxiz_math::simplex::SimplexTableau`. -/
def SimplexTableau : Type := SimplexTableauNonempty.type

namespace SimplexTableau

  @[extern "oxiz_lean4_simplex_new"]
  opaque newRaw : Unit → SimplexTableau

  @[extern "oxiz_lean4_simplex_free"]
  opaque free : SimplexTableau → Unit

  /-- Allocate a fresh tableau. -/
  def new : IO SimplexTableau := pure (newRaw ())

  /-- Add a variable with optional rational bounds. `hasLower`/
  `hasUpper` are 0/1 flags; the corresponding `_num`/`_den` pair
  expresses the bound as a signed rational. Returns the new
  variable id (>=0) or -1 on argument fault. -/
  @[extern "oxiz_lean4_simplex_add_var"]
  opaque addVar :
    SimplexTableau →
    (hasLower : Int32) → (lowerNum : Int64) → (lowerDen : Int64) →
    (hasUpper : Int32) → (upperNum : Int64) → (upperDen : Int64) →
    IO Int64

  /-- Run feasibility check. Verdict codes:
  0=Sat, 1=Unsat, 2=Unknown, -1=error. -/
  @[extern "oxiz_lean4_simplex_check"]
  opaque check : SimplexTableau → IO Int32

end SimplexTableau

end Oxiz.Math
