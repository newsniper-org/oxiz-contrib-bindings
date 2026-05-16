/-!
# `Oxiz` — Lean4 surface for the OxiZ SAT solver (oxiz-sat)

This file declares the Lean-side wrappers backed by the Rust FFI
in `../src/lib.rs`. Link against `liboxiz_binding_lean4.{a,so}`
(produced by `cargo build --release` in the parent directory).

Status: v0.1 covers solver new / free / new_var / add_clause / solve.
Model extraction and incremental push/pop arrive in v0.2.
-/

namespace Oxiz

opaque SolverNonempty : NonemptyType

/-- Opaque handle to a Rust-owned `oxiz_sat::Solver`. Lean's
runtime registers a finalizer that calls `Solver.free`. -/
def Solver : Type := SolverNonempty.type

namespace Solver

  @[extern "oxiz_lean4_solver_new"]
  opaque newRaw : Unit → Solver

  @[extern "oxiz_lean4_solver_free"]
  opaque free : Solver → Unit

  /-- Allocate a fresh solver in `IO`. -/
  def new : IO Solver := pure (newRaw ())

  /-- Allocate a new SAT variable; returns the (1-based) DIMACS id. -/
  @[extern "oxiz_lean4_solver_new_var"]
  opaque newVar : Solver → IO Int32

  /--
  Add a clause. `lits` follows the DIMACS sign convention: positive
  ints are positive literals on variable `|lit| - 1`. Returns `1` on
  success, `0` if the solver immediately recognized the clause as
  contradictory, `-1` on a pointer fault.
  -/
  @[extern "oxiz_lean4_solver_add_clause"]
  opaque addClause : Solver → @& Array Int32 → IO Int32

  /-- Drive the solver. Verdict codes: 0=Sat, 1=Unsat, 2=Unknown,
  -1=error. -/
  @[extern "oxiz_lean4_solver_solve"]
  opaque solve : Solver → IO Int32

end Solver

/-- Verdict tags matching the FFI verdict codes. -/
inductive Verdict where
  | sat
  | unsat
  | unknown
  | error
  deriving Repr, DecidableEq

def Verdict.ofInt? (i : Int32) : Verdict :=
  match i with
  | 0 => .sat
  | 1 => .unsat
  | 2 => .unknown
  | _ => .error

end Oxiz
