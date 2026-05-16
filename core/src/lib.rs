//! Lean4 FFI bindings for the OxiZ solver suite.
//!
//! This crate is the **core** binding — it covers oxiz proper
//! (`oxiz-sat`, `oxiz-proof`, `oxiz-math`). Each upstream OxiZ
//! crate is exposed through a Cargo feature so consumers don't pay
//! for surfaces they don't use:
//!
//! - `oxiz-sat` (default) — opaque `Solver` pointer + clause
//!   management + push/pop + model extraction. Lean side in
//!   `lean/Oxiz.lean`.
//! - `oxiz-proof` — opaque `DratProof` pointer + emit / write
//!   helpers. Lean side in `lean/Proof.lean`.
//! - `oxiz-math` — Simplex / polynomial helpers (forthcoming).
//!   Lean side in `lean/Math.lean`.
//!
//! For bindings to our community contribution crates
//! (`oxiz-contrib-abduction`, …), see the sibling
//! `oxiz-binding-lean4-contrib-*` crates in the same workspace.

#[cfg(feature = "oxiz-sat")]
pub mod sat;

#[cfg(feature = "oxiz-proof")]
pub mod proof;

#[cfg(feature = "oxiz-math")]
pub mod math;
