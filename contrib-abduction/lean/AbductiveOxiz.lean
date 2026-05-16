/-!
# `AbductiveOxiz` — Lean4 surface for `oxiz-contrib-abduction`

Companion to the `Oxiz` namespace in `oxiz-binding-lean4`. This file
declares the Lean-side wrappers for the abductive-reasoning crate;
link against `liboxiz_binding_lean4_contrib_abduction.{a,so}`.

Status: v0.2 — covers backend new / free, add_clause, `abduce`
search, `check_with` (single-step verdict + unsat-core probe),
and per-abducible `explanation` strings.
-/

namespace AbductiveOxiz

opaque BackendNonempty : NonemptyType

/-- Opaque handle to a Rust-owned `OxizSatBackend`. The Lean runtime
runs the registered finalizer (`Backend.free`) on collection. -/
def Backend : Type := BackendNonempty.type

namespace Backend

  @[extern "oxiz_lean4_abduction_backend_new"]
  opaque newRaw : @& Array Int32 → Backend

  @[extern "oxiz_lean4_abduction_backend_free"]
  opaque free : Backend → Unit

  /-- Build a backend whose abducibles are the DIMACS literals in
  `abducibles`. -/
  def new (abducibles : Array Int32) : IO Backend := pure (newRaw abducibles)

  /-- Add a clause to the underlying solver. Returns 1 on success,
  0 if the clause is immediately contradictory, -1 on pointer fault.
  -/
  @[extern "oxiz_lean4_abduction_add_clause"]
  opaque addClause : Backend → @& Array Int32 → IO Int32

  /-- Run the abductive search. Returns the list of solutions, each
  represented as an array of the abducible DIMACS ids the solver
  picked. The list is bounded by `maxSolutions`; each solution is
  bounded by `maxSize`. -/
  @[extern "oxiz_lean4_abduce"]
  opaque abduceRaw :
    Backend →
    (maxSize : USize) →
    (maxSolutions : USize) →
    (maxIndices : USize) →
    IO (Int32 × Array USize × Array Int32)

  /-- Single-step probe: check whether the formula remains
  satisfiable under the chosen abducible subset (indices into
  the abducible list configured at backend creation). Returns the
  verdict code plus, when the verdict is Unsat, the unsat-core
  literals (DIMACS-style) written into a caller-allocated buffer.
  Verdict codes: 0=Sat, 1=Unsat, 2=Unknown, -1=error. -/
  @[extern "oxiz_lean4_abduction_check_with"]
  opaque checkWith :
    Backend →
    (indices : @& Array USize) →
    (maxCore : USize) →
    IO (Int32 × USize × Array Int32)

  /-- Read the optional `explanation` field for the abducible at
  the given index. Returns the bytes written into the caller-
  allocated buffer; `0` if the explanation is unset; `-1` on
  out-of-range index. -/
  @[extern "oxiz_lean4_abduction_abducible_explanation"]
  opaque abducibleExplanation :
    Backend → (index : USize) → (outCap : USize) → IO (Int32 × String)

end Backend

end AbductiveOxiz
