/-!
# `Oxiz.Proof` — Lean4 surface for `oxiz-proof`

This file declares the Lean-side wrappers for the DRAT proof
writer in oxiz-proof. Link against `liboxiz_binding_lean4.{a,so}`
built with the `oxiz-proof` feature.

Status: v0.3 covers DratProof allocate / free / add_clause /
delete_clause / len / to_text. Alethe, LFSC, and Coq surfaces
(from `AletheProof`, `LfscProof`, `CoqExporter`) arrive later.
-/

namespace Oxiz.Proof

opaque DratProofNonempty : NonemptyType

/-- Opaque handle to a Rust-owned `oxiz_proof::drat::DratProof`.
The Lean runtime runs `DratProof.free` on collection. -/
def DratProof : Type := DratProofNonempty.type

namespace DratProof

  @[extern "oxiz_lean4_drat_proof_new"]
  opaque newRaw : Unit → DratProof

  @[extern "oxiz_lean4_drat_proof_new_binary"]
  opaque newBinaryRaw : Unit → DratProof

  @[extern "oxiz_lean4_drat_proof_free"]
  opaque free : DratProof → Unit

  /-- Allocate a fresh DRAT proof (text format). -/
  def new : IO DratProof := pure (newRaw ())

  /-- Allocate a fresh DRAT proof (binary format). -/
  def newBinary : IO DratProof := pure (newBinaryRaw ())

  /-- Append an `Add` step. `lits` is a DIMACS-style literal array
  (positive = positive literal on variable `|lit| - 1`, negative
  = negated). Returns 0 on success, -1 on pointer fault. The
  empty clause is accepted and serves as the unsat witness. -/
  @[extern "oxiz_lean4_drat_proof_add_clause"]
  opaque addClause : DratProof → @& Array Int32 → IO Int32

  /-- Append a `Delete` step. Same convention as `addClause`. -/
  @[extern "oxiz_lean4_drat_proof_delete_clause"]
  opaque deleteClause : DratProof → @& Array Int32 → IO Int32

  /-- Number of DRAT steps recorded. Returns -1 on null. -/
  @[extern "oxiz_lean4_drat_proof_len"]
  opaque len : DratProof → IO ISize

  /-- Serialize to DIMACS DRAT text format. Returns the bytes
  written (or required) per the Rust-side contract. -/
  @[extern "oxiz_lean4_drat_proof_to_text"]
  opaque toTextRaw : DratProof → (outCap : USize) → IO (ISize × ByteArray)

end DratProof

end Oxiz.Proof
